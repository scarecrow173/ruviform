/* ofx_bridge.c   host/catalog/instance lifecycle, admission, ImageEffectV1.
 * All SDK-facing handles are generation tokens. Foreign calls run unlocked:
 * prepare under g_mu -> release -> call -> reacquire -> commit. */
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <stdio.h>
#include <limits.h>
#include "ofx_internal.h"

int32_t ofx_bridge_image_effect_api_version(void) {
    return kOfxImageEffectPluginApiVersion;
}

const void *rv_fetch_suite(void *self, const char *name, int ver) {
    (void)self;
    if (!name || ver != 1) return NULL;
    if (!strcmp(name, kOfxPropertySuite)) return rv_prop_suite_ptr();
    if (!strcmp(name, kOfxParameterSuite)) return rv_param_suite_ptr();
    if (!strcmp(name, kOfxImageEffectSuite)) return rv_fx_suite_ptr();
    if (!strcmp(name, kOfxMemorySuite)) return rv_mem_suite_ptr();
    return NULL;
}

/* the OfxHost handed to foreign code (OfxSetHost / plugin setHost) lives
 * in static storage: a pointer a plugin may retain past a call window
 * must not point into the reclaimable RvHost block. fetchSuite stays
 * valid for the process lifetime; host names a dedicated property set
 * created once and bound once under g_mu, so the field is read-only
 * after first publish     never rebound, never poisoned, and a token a
 * plugin retains can never alias a later session's state (Issue #131) */
static OfxHost g_api = { NULL, rv_fetch_suite };
static RvPropsObj *g_host_props;

static void img_free(RvImg *im);
static void stage_free(void *p);

/* ---------- host ---------- */

void *rv_ofx_host_new(void) {
    RvHost *h;
    uintptr_t tok;
    if (rv_lock()) return NULL;
    if (g_active) { rv_unlock(); return NULL; }
    h = (RvHost *)calloc(1, sizeof(RvHost));
    if (!h) { rv_unlock(); return NULL; }
    /* the caller-visible handle is a generation token, not the block:
     * a stale value resolves to NULL instead of aliasing recycled
     * storage */
    h->tok = rv_tok_new(H_HOST, h, 0);
    if (!h->tok) { free(h); rv_unlock(); return NULL; }
    if (!g_host_props) {
        g_host_props = rv_po_new(0);
        if (!g_host_props) { rv_tok_revoke(h->tok); free(h); rv_unlock(); return NULL; }
        g_api.host = (OfxPropertySetHandle)(uintptr_t)g_host_props->tok;
    }
    h->alive = 1;
    g_active = h;
    tok = h->tok;
    rv_unlock();
    return (void *)tok;
}

static void effect_free(RvEffect *e) {
    RvClip *c;
    if (!e) return;
    rv_tok_revoke(e->tok);
    rv_tok_revoke(e->props_tok);
    /* any image handles still handed out die with the effect */
    while (e->images) {
        RvImg *im = e->images;
        e->images = im->next;
        rv_tok_revoke(im->tok);
        if (im->props) rv_tok_revoke(im->props->tok);
        rv_po_free(im->props);
        free(im);
    }
    /* staging pointers are call-scoped; free defensively if a teardown
     * path ever runs with a render bracket still open. Allocated by
     * stage_alloc (_aligned_malloc on Windows)   matched free required */
    stage_free(e->stage_src);
    stage_free(e->stage_dst);
    rv_params_free(e->params, e->nparams, g_active);
    rv_po_free(e->props);
    c = e->clips;
    while (c) {
        RvClip *n = c->next;
        rv_tok_revoke(c->tok);
        rv_tok_revoke(c->props_tok);
        rv_po_free(c->props);
        free(c);
        c = n;
    }
    free(e);
}

/* caller holds g_mu; h is already poisoned   g_active cleared, alive=0,
 * catalogs detached, every session token revoked. reclaim the
 * host-owned pools and the block itself */
static void host_reclaim(RvHost *h) {
    RvStrNode *s = h->strings;
    while (s) { RvStrNode *n = s->next; free(s->s); free(s); s = n; }
    /* rv_mem_free revokes the H_MEMORY token, unlinks, frees buffer+node */
    while (h->mem) (void)rv_mem_free(h, h->mem->p);
    free(h);
}

/* caller holds g_mu. drops one foreign-call lease; a host poisoned while
 * a window was open is reclaimed when the last lease drops. callers must
 * not dereference h after this returns */
static void host_unlease(RvHost *h) {
    if (h && h->foreign && --h->foreign == 0 && !h->alive) host_reclaim(h);
}

/* caller holds g_mu. lease the session host this attempt was created
 * under. NULL when the owning session is gone: the pool its foreign
 * actions would touch is already reclaimed, so no foreign call may run */
static RvHost *eff_host_lease(const RvEffect *i) {
    if (g_active && g_active->tok == i->host_tok) {
        g_active->foreign++;
        return g_active;
    }
    return NULL;
}

void rv_ofx_host_free(void *vh) {
    RvHost *h;
    if (rv_lock()) return;
    /* vh is the session token host_new returned; a stale or replayed
     * value resolves to NULL (or a wrong object) rather than a raw
     * pointer into possibly-reclaimed storage */
    h = (RvHost *)rv_tok_get((uintptr_t)vh, H_HOST);
    if (!h || h != g_active || !h->alive) { rv_unlock(); return; }
    g_active = NULL;
    h->alive = 0;
    while (h->catalogs) {
        /* catalogs must be closed first; detach anyway and poison the
         * back-pointer so a late close degrades to no-op */
        RvCatalog *d = h->catalogs;
        h->catalogs = d->next;
        d->host = NULL;
    }
    /* every token the session published dies with the host: a suite call
     * or release that lands after reclaim resolves nothing, so it cannot
     * reach objects whose pool strings are gone. the dedicated api.host
     * set is process-lifetime and survives the sweep */
    rv_tok_sweep(g_host_props ? g_host_props->tok : 0);
    /* an open foreign-call window still dereferences h after re-locking;
     * poison above is immediate, reclaim waits for the last lease drop */
    if (!h->foreign) host_reclaim(h);
    rv_unlock();
}

/* ---------- admission ---------- */

typedef int (*FnGetCount)(void);
typedef OfxPlugin *(*FnGetPlugin)(int);
typedef OfxStatus (*FnSetHost)(OfxHost *);

static int adv_has_str(RvPropsObj *o, const char *name, const char *want) {
    int dim = 0, i;
    char *s = NULL;
    if (rv_po_dim(o, name, &dim) != kOfxStatOK || dim <= 0) return 0;
    for (i = 0; i < dim; i++) {
        s = NULL;
        if (rv_po_get(o, name, RV_PT_STRING, i, NULL, NULL, &s, NULL) == kOfxStatOK &&
            s && !strcmp(s, want)) return 1;
    }
    return 0;
}

static RvEffect *effect_new(int is_inst, uintptr_t parent) {
    RvEffect *e = (RvEffect *)calloc(1, sizeof(RvEffect));
    if (!e) return NULL;
    e->is_instance = is_inst;
    /* bind the attempt to the session that creates it   window leases
     * later require this token to still name the live host */
    e->host_tok = g_active ? g_active->tok : 0;
    e->props = rv_po_new(0);
    if (!e->props) { free(e); return NULL; }
    e->tok = rv_tok_new(H_EFFECT, e, parent);
    e->props_tok = e->props->tok;
    if (!e->tok) { rv_tok_revoke(e->props->tok); rv_po_free(e->props); free(e); return NULL; }
    return e;
}

/* foreign call unlocked: prepare under lock, release, call, reacquire */
static OfxStatus call_action(OfxPlugin *raw, const char *action,
                             void *handle, void *inargs, void *outargs) {
    return raw->mainEntry(action, handle, inargs, outargs);
}

static int entry_admit(RvHost *h, RvCatalog *c, RvEffect *e) {
    OfxPlugin *raw = e->raw;
    OfxPropertySetHandle in;
    OfxStatus st;
    char *ctx = NULL;
    size_t i;
    int has_src = 0, has_out = 0, has_string = 0;
    RvClip *cl;
    (void)c;

    /* seed descriptor props the plugin will fill */
    rv_seed_s(e->props, kOfxPropLabel, 1, e->identifier);
    rv_seed_i(e->props, kOfxImageEffectPluginPropSingleInstance, 1, 0);
    e->props_tok = e->props->tok;

    /* unlock around foreign calls; the caller holds a lease on h so the
     * block stays valid across the window   alive may still drop */
    rv_unlock();
    raw->setHost(&g_api);
    st = call_action(raw, kOfxActionLoad, NULL, NULL, NULL);
    rv_lock();
    /* a Load that actually succeeded stays owed an Unload even when the
     * host died inside the window   the rejection/close path balances it */
    if (st == kOfxStatOK) e->loaded = 1;
    if (!h->alive) return kOfxStatErrBadHandle;
    if (st != kOfxStatOK) return st;

    rv_unlock();
    st = call_action(raw, kOfxActionDescribe, e->tok ? (void *)(uintptr_t)e->tok : NULL, NULL, NULL);
    rv_lock();
    if (!h->alive) return kOfxStatErrBadHandle;
    if (st != kOfxStatOK) return st;

    /* Filter required across whole advertised context list */
    if (!adv_has_str(e->props, kOfxImageEffectPropSupportedContexts,
                     kOfxImageEffectContextFilter))
        return kOfxStatErrUnsupported;

    {
        RvPropsObj *o = rv_po_new(e->tok);
        if (!o) return kOfxStatErrMemory;
        rv_po_set(o, kOfxImageEffectPropContext, RV_PT_STRING, 0, 0, 0,
                  kOfxImageEffectContextFilter, NULL);
        in = (OfxPropertySetHandle)(uintptr_t)o->tok;
        rv_unlock();
        st = call_action(raw, kOfxImageEffectActionDescribeInContext,
                         (void *)(uintptr_t)e->tok, (void *)in, NULL);
        rv_lock();
        rv_tok_revoke(o->tok);
        rv_po_free(o);
    }
    if (!h->alive) return kOfxStatErrBadHandle;
    if (st != kOfxStatOK) return st;

    /* validate clips: Source + Output present and RGBA */
    for (cl = e->clips; cl; cl = cl->next) {
        if (!cl->name || !cl->props) continue;
        if (adv_has_str(cl->props, kOfxImageEffectPropSupportedComponents,
                        kOfxImageComponentRGBA)) {
            if (!strcmp(cl->name, "Source")) has_src = 1;
            if (!strcmp(cl->name, "Output")) has_out = 1;
        }
    }
    if (!has_src || !has_out) return kOfxStatErrUnsupported;

    /* read back param metadata; unsupported kinds flag rejection */
    for (i = 0; i < e->nparams; i++) {
        RvParam *pr = e->params[i];
        if (rv_params_read_meta(h, pr) != kOfxStatOK)
            return kOfxStatFailed;
        if (pr->kind == PK_STRING) has_string = 1;
        else if (!rv_param_domain_ok(pr, pr->kind,
                                     pr->kind == PK_DOUBLE ? pr->ddef : 0,
                                     pr->kind == PK_DOUBLE ? 0 : pr->idef, 0))
            return kOfxStatErrValue;
    }
    if (has_string) return kOfxStatErrUnsupported;

    /* descriptor fields */
    {
        char *sv = NULL;
        int si = 0;
        char buf[32];
        rv_po_get(e->props, kOfxPropLabel, RV_PT_STRING, 0, NULL, NULL, &sv, NULL);
        e->label = rv_str_own(h, sv ? sv : e->identifier);
        rv_po_get(e->props, kOfxImageEffectPluginPropSingleInstance, RV_PT_INT, 0,
                  &si, NULL, NULL, NULL);
        e->single_instance = si;
        snprintf(buf, sizeof(buf), "%u.%u",
                 raw->pluginVersionMajor, raw->pluginVersionMinor);
        e->version = rv_str_own(h, buf);
        e->vendor = rv_str_own(h, "");
    }
    (void)ctx;
    return kOfxStatOK;
}

int rv_ofx_catalog_open(void *vh, const void *get_count,
                        const void *get_plugin, const void *set_host,
                        void **catalog) {
    RvHost *h;
    RvCatalog *c = NULL;
    FnGetCount gc = (FnGetCount)get_count;
    FnGetPlugin gp = (FnGetPlugin)get_plugin;
    FnSetHost sh = (FnSetHost)set_host;
    int n, i, rc = kOfxStatOK;

    if (!vh || !gc || !gp || !catalog) return kOfxStatErrBadHandle;
    *catalog = NULL;
    if (rv_lock()) return kOfxStatFailed;
    /* the handle is a session token: a stale h is rejected without a
     * deref     it can never alias a recycled RvHost block */
    h = (RvHost *)rv_tok_get((uintptr_t)vh, H_HOST);
    if (!h || h != g_active || !h->alive) { rv_unlock(); return kOfxStatErrBadHandle; }
    /* lease: h stays allocated across every foreign window below even if
     * teardown poisons it mid-call; reclaim waits for the lease drop */
    h->foreign++;

    rv_unlock();
    if (sh) {
        OfxStatus s = sh(&g_api);
        if (s != kOfxStatOK) {
            rv_lock();
            host_unlease(h);
            rv_unlock();
            return (int)s;
        }
    }
    n = gc();
    rv_lock();
    /* the unlocked foreign window above may have raced host teardown;
     * the lease keeps h readable so alive is a defined check */
    if (!h->alive) { host_unlease(h); rv_unlock(); return kOfxStatErrBadHandle; }
    if (n < 0) { host_unlease(h); rv_unlock(); return kOfxStatErrValue; }

    c = (RvCatalog *)calloc(1, sizeof(RvCatalog));
    if (!c) { host_unlease(h); rv_unlock(); return kOfxStatErrMemory; }
    c->host = h;
    c->next = h->catalogs;
    h->catalogs = c;

    for (i = 0; i < n; i++) {
        RvEffect *e;
        OfxPlugin *raw;
        rv_unlock();
        raw = gp(i);
        rv_lock();
        if (!h->alive) { rc = kOfxStatErrBadHandle; break; }
        e = effect_new(0, 0);
        if (!e) { rc = kOfxStatErrMemory; break; }
        e->native_index = i;
        e->raw = raw;
        e->next_quar = NULL;
        if (!raw) {
            e->rejected = 1;
            e->rejection_status = kOfxStatErrBadHandle;
        } else {
            e->identifier = rv_str_own(h, raw->pluginIdentifier ? raw->pluginIdentifier : "");
            if (!raw->pluginApi || strcmp(raw->pluginApi, kOfxImageEffectPluginApi) ||
                raw->apiVersion != kOfxImageEffectPluginApiVersion)
                e->rejection_status = kOfxStatErrUnsupported;
            else if (!e->identifier || !e->identifier[0])
                e->rejection_status = kOfxStatErrValue;
            else if (!raw->setHost || !raw->mainEntry)
                e->rejection_status = kOfxStatErrBadHandle;
            else e->rejection_status = entry_admit(h, c, e);
            if (e->rejection_status != kOfxStatOK) {
                e->rejected = 1;
                /* a rejected entry that reached Load unloads now;
                   admission keeps no claim on its code */
                if (e->loaded && e->raw->mainEntry) {
                    rv_unlock();
                    (void)call_action(e->raw, kOfxActionUnload, NULL, NULL, NULL);
                    rv_lock();
                    e->loaded = 0;
                }
            }
        }
        e->cat = c;
        /* append at tail to preserve scan order */
        {
            RvEffect **p = &c->entries;
            while (*p) p = &(*p)->next;
            *p = e;
        }
    }
    {
        /* teardown ran inside a window: entries hold host-pool strings
         * that die with reclaim, so a partial catalog is torn down in
         * place instead of escaping to the caller. the catalog was
         * already detached by host poison, so close is a defined path.
         * the recursive call runs with no lock held     a foreign Unload
         * inside close may join a thread calling back into host suites,
         * and must not find g_mu owned by this frame (Issue #131) */
        int dead = !h->alive;
        *catalog = dead ? NULL : c;
        if (dead) rc = kOfxStatErrBadHandle;
        host_unlease(h);
        rv_unlock();
        if (dead && c) rv_ofx_catalog_close(c);
        return rc;
    }
}

/* live attempts minus the ones parked on this catalog's quarantine list
 *   the count of non-quarantined attempts still dereferencing `e` as
 * their descriptor (quarantined attempts also hold desc->live) */
static int entry_open_live(const RvCatalog *c, const RvEffect *e) {
    int n = e->live;
    const RvEffect *q;
    for (q = c->quar; q; q = q->next_quar)
        if (q->desc == e) n--;
    return n;
}

void rv_ofx_catalog_close(void *vc) {
    RvCatalog *c = (RvCatalog *)vc;
    RvHost *host;
    RvEffect *e;
    if (!c) return;
    if (rv_lock()) return;
    /* a re-entrant close from inside a foreign Unload window must not
     * re-walk or re-free the catalog this frame is tearing down */
    if (c->closing) { rv_unlock(); return; }
    c->closing = 1;
    /* lease the owning host across the Unload windows so post-window
     * bookkeeping stays defined when teardown lands inside one. NULL
     * once poison detached it: with no session, no foreign call may run
     *     the pool the plugin's callback would touch is already gone */
    host = c->host;
    if (host) host->foreign++;
    /* detach from host   NULL when host already tore down */
    if (c->host) {
        RvCatalog **p = &c->host->catalogs;
        while (*p && *p != c) p = &(*p)->next;
        if (*p) *p = c->next;
        c->host = NULL;
    }
    /* Unload each entry that had a successful Load   once, and only when
     * no non-quarantined attempt still holds the descriptor: Unloading
     * under a live attempt is an action-serialization violation */
    for (e = c->entries; e; e = e->next) {
        if (host && e->loaded && e->raw && e->raw->mainEntry &&
            entry_open_live(c, e) == 0) {
            rv_unlock();
            call_action(e->raw, kOfxActionUnload, NULL, NULL, NULL);
            rv_lock();
            e->loaded = 0;
        }
    }
    while (c->entries) {
        e = c->entries;
        c->entries = e->next;
        if (entry_open_live(c, e) > 0) {
            /* a live non-quarantined attempt still dereferences i->desc:
             * detach the descriptor (deliberate leak, not UAF) so its
             * fields stay valid; destroy/release NULL-check i->desc->cat */
            e->cat = NULL;
            continue;
        }
        effect_free(e);
    }
    while (c->quar) {
        e = c->quar;
        c->quar = e->next_quar;
        if (e->render_active) continue; /* bracket open   leak, not UAF */
        effect_free(e);
    }
    free(c);
    host_unlease(host);
    rv_unlock();
}

size_t rv_ofx_catalog_count(void *vc) {
    RvCatalog *c = (RvCatalog *)vc;
    size_t n = 0;
    RvEffect *e;
    if (rv_lock()) return 0;
    if (c) for (e = c->entries; e; e = e->next) n++;
    rv_unlock();
    return n;
}

static RvEffect *entry_at(RvCatalog *c, size_t idx) {
    RvEffect *e;
    if (!c) return NULL;
    for (e = c->entries; e; e = e->next)
        if ((size_t)e->native_index == idx) return e;
    return NULL;
}

void *rv_ofx_catalog_descriptor(void *vc, size_t idx) {
    RvEffect *e;
    if (rv_lock()) return NULL;
    e = entry_at((RvCatalog *)vc, idx);
    {
        uintptr_t tok = (e && !e->rejected) ? e->tok : 0;
        rv_unlock();
        return (void *)tok;
    }
}
const char *rv_ofx_catalog_identifier(void *vc, size_t idx) {
    RvEffect *e;
    const char *r = NULL;
    if (rv_lock()) return NULL;
    e = entry_at((RvCatalog *)vc, idx);
    if (e) r = e->identifier;
    rv_unlock();
    return r;
}
int rv_ofx_catalog_rejection(void *vc, size_t idx) {
    RvEffect *e;
    int r;
    if (rv_lock()) return kOfxStatFailed;
    e = entry_at((RvCatalog *)vc, idx);
    r = e ? e->rejection_status : kOfxStatErrBadHandle;
    rv_unlock();
    return r;
}

const char *rv_ofx_descriptor_string(void *vd, int field) {
    RvEffect *e;
    const char *r = NULL;
    if (rv_lock()) return NULL;
    e = (RvEffect *)rv_tok_get((uintptr_t)vd, H_EFFECT);
    if (e) switch (field) {
        case 0: r = e->identifier; break;
        case 1: r = e->label; break;
        case 2: r = e->vendor; break;
        case 3: r = e->version; break;
        default: break;
    }
    rv_unlock();
    return r;
}

/* ---------- params (descriptor OR attempt) ---------- */

static RvEffect *eff_of(void *v) {
    return (RvEffect *)rv_tok_get((uintptr_t)v, H_EFFECT);
}

size_t rv_ofx_param_count(void *eff) {
    RvEffect *e;
    size_t n = 0;
    if (rv_lock()) return 0;
    e = eff_of(eff);
    if (e) n = e->nparams;
    rv_unlock();
    return n;
}

int rv_ofx_param_info(void *eff, size_t idx, int *kind,
                      const char **name, const char **unit,
                      double *mn, double *mx, double *dm, double *dx,
                      double *dd, int *di, unsigned int *flags, size_t *cc) {
    RvEffect *e;
    RvParam *pr;
    int rc = kOfxStatOK;
    if (rv_lock()) return kOfxStatFailed;
    e = eff_of(eff);
    if (!e || idx >= e->nparams) { rv_unlock(); return kOfxStatErrBadHandle; }
    pr = e->params[idx];
    if (kind) *kind = pr->kind;
    if (name) *name = pr->name;
    if (unit) *unit = "";
    if (mn) *mn = pr->mn;
    if (mx) *mx = pr->mx;
    if (dm) *dm = pr->dmn;
    if (dx) *dx = pr->dmx;
    if (dd) *dd = pr->ddef;
    if (di) *di = pr->idef;
    if (flags) *flags = (pr->enabled ? 0u : (1u << 4)) |
                        (pr->kind == PK_CHOICE ? (1u << 2) : 0u) |
                        pr->flags;
    if (cc) *cc = (size_t)pr->nchoices;
    rv_unlock();
    return rc;
}

const char *rv_ofx_param_choice(void *eff, size_t idx, size_t c) {
    RvEffect *e;
    const char *r = NULL;
    if (rv_lock()) return NULL;
    e = eff_of(eff);
    if (e && idx < e->nparams && c < (size_t)e->params[idx]->nchoices)
        r = e->params[idx]->choices[c];
    rv_unlock();
    return r;
}

int rv_ofx_param_get(void *eff, size_t idx, double *vd, int *vi) {
    RvEffect *e;
    int rc;
    if (rv_lock()) return kOfxStatFailed;
    e = eff_of(eff);
    if (!e || !e->is_instance || idx >= e->nparams) { rv_unlock(); return kOfxStatErrBadHandle; }
    {
        RvParam *pr = e->params[idx];
        if (pr->kind == PK_DOUBLE) { if (!vd) rc = kOfxStatErrBadHandle; else { *vd = pr->cur_d; rc = kOfxStatOK; } }
        else { if (!vi) rc = kOfxStatErrBadHandle; else { *vi = pr->cur_i; rc = kOfxStatOK; } }
    }
    rv_unlock();
    return rc;
}

int rv_ofx_param_validate(void *eff, size_t idx, int kind,
                          double vd, int vi, int hw) {
    RvEffect *e;
    int rc;
    if (rv_lock()) return kOfxStatFailed;
    e = eff_of(eff);
    if (!e || !e->is_instance || idx >= e->nparams) { rv_unlock(); return kOfxStatErrBadHandle; }
    rc = rv_param_domain_ok(e->params[idx], kind, vd, vi, hw)
         ? kOfxStatOK : kOfxStatErrValue;
    rv_unlock();
    return rc;
}

int rv_ofx_param_set(void *eff, size_t idx, int kind,
                     double vd, int vi, int hw) {
    RvEffect *e;
    int rc;
    if (rv_lock()) return kOfxStatFailed;
    e = eff_of(eff);
    if (!e || !e->is_instance || idx >= e->nparams) { rv_unlock(); return kOfxStatErrBadHandle; }
    if (!rv_param_domain_ok(e->params[idx], kind, vd, vi, hw))
        rc = kOfxStatErrValue;
    else {
        if (kind == PK_DOUBLE) e->params[idx]->cur_d = vd;
        else e->params[idx]->cur_i = vi;
        rc = kOfxStatOK;
    }
    rv_unlock();
    return rc;
}

/* ---------- instance ---------- */

int rv_ofx_instance_new(void *vd, void **out) {
    RvEffect *d, *i;
    if (!out) return kOfxStatErrBadHandle;
    *out = NULL;
    if (rv_lock()) return kOfxStatFailed;
    d = (RvEffect *)rv_tok_get((uintptr_t)vd, H_EFFECT);
    if (!d || d->is_instance) { rv_unlock(); return kOfxStatErrBadHandle; }
    i = effect_new(1, d->tok);
    if (!i) { rv_unlock(); return kOfxStatErrMemory; }
    i->desc = d;
    i->params = rv_params_clone(d->params, d->nparams, g_active, i->tok);
    if (d->nparams && !i->params) { effect_free(i); rv_unlock(); return kOfxStatErrMemory; }
    i->nparams = d->nparams;
    /* clone clips   the instance clip property set mirrors the
     * descriptor-declared records so instance actions see the contract
     * the plugin wrote during DescribeInContext (Issue #124) */
    {
        RvClip *c;
        for (c = d->clips; c; c = c->next) {
            RvClip *q = (RvClip *)calloc(1, sizeof(RvClip));
            if (!q) { effect_free(i); rv_unlock(); return kOfxStatErrMemory; }
            q->name = rv_str_own(g_active, c->name ? c->name : "");
            q->props = rv_po_new(i->tok);
            q->tok = rv_tok_new(H_CLIP, q, i->tok);
            if (q->props) q->props_tok = q->props->tok;
            if (q->props && c->props &&
                rv_po_clone(q->props, c->props) != kOfxStatOK) {
                rv_tok_revoke(q->tok);
                rv_tok_revoke(q->props->tok);
                rv_po_free(q->props);
                free(q);
                effect_free(i); rv_unlock(); return kOfxStatErrMemory;
            }
            q->eff = i;
            q->next = i->clips;
            i->clips = q;
        }
    }
    /* staged values ride on cloned params */
    i->pset_tok = rv_tok_new(H_PSET, i, i->tok);
    *out = (void *)(uintptr_t)i->tok;
    rv_unlock();
    return kOfxStatOK;
}

int rv_ofx_instance_configure(void *eff, uint32_t w, uint32_t h, double fps) {
    RvEffect *i;
    if (rv_lock()) return kOfxStatFailed;
    i = eff_of(eff);
    if (!i || !i->is_instance) { rv_unlock(); return kOfxStatErrBadHandle; }
    /* h itself must fit int (RenderWindow/image bounds); the allocation
     * bound uses the padded stride the render path allocates, not w*4 */
    if (!w || !h || !isfinite(fps) || fps <= 0.0 ||
        (uint64_t)h > (uint64_t)INT32_MAX ||
        (uint64_t)w * 4u + 15u > (uint64_t)INT32_MAX ||
        (((uint64_t)w * 4u + 15u) & ~(uint64_t)15u) * (uint64_t)h >
            (uint64_t)PTRDIFF_MAX) {
        rv_unlock(); return kOfxStatErrValue; }
    i->w = w; i->h = h; i->fps = fps;
    {
        double sz[2] = {(double)w, (double)h};
        double off[2] = {0.0, 0.0};
        rv_po_setn(i->props, "OfxImageEffectPropProjectSize", RV_PT_DOUBLE, 2, NULL, sz, NULL, NULL);
        rv_po_setn(i->props, "OfxImageEffectPropProjectOffset", RV_PT_DOUBLE, 2, NULL, off, NULL, NULL);
        rv_po_setn(i->props, "OfxImageEffectPropProjectExtent", RV_PT_DOUBLE, 2, NULL, sz, NULL, NULL);
        rv_po_set(i->props, kOfxImageEffectPropFrameRate, RV_PT_DOUBLE, 0, 0, fps, NULL, NULL);
        rv_po_set(i->props, kOfxImagePropPixelAspectRatio, RV_PT_DOUBLE, 0, 0, 1.0, NULL, NULL);
        rv_po_set(i->props, kOfxImageEffectPropFieldToRender, RV_PT_STRING, 0, 0, 0,
                  kOfxImageFieldNone, NULL);
        rv_po_set(i->props, kOfxImageEffectPropPreMultiplication, RV_PT_STRING, 0, 0, 0,
                  kOfxImageUnPreMultiplied, NULL);
    }
    i->configured = 1;
    rv_unlock();
    return kOfxStatOK;
}

int rv_ofx_instance_create(void *eff) {
    RvEffect *i;
    RvHost *host;
    OfxPlugin *raw;
    uintptr_t itok;
    OfxStatus st;
    if (rv_lock()) return kOfxStatFailed;
    i = eff_of(eff);
    if (!i || !i->is_instance || !i->desc || !i->desc->raw) { rv_unlock(); return kOfxStatErrBadHandle; }
    if (i->created || i->destroyed) { rv_unlock(); return kOfxStatFailed; }
    if (i->desc->single_instance && i->desc->live > 0) {
        rv_unlock();
        return kOfxStatErrExists;
    }
    /* lease the owning session across the Create window: a re-entrant
     * rv_ofx_host_free inside the action keeps this block (and its pool)
     * allocated until the lease drops below */
    host = eff_host_lease(i);
    if (!host) { rv_unlock(); return kOfxStatErrBadHandle; }
    i->create_attempted++;
    raw = i->desc->raw;
    itok = i->tok;
    rv_unlock();
    st = call_action(raw, kOfxActionCreateInstance,
                     (void *)itok, NULL, NULL);
    rv_lock();
    /* the attempt may have been released     or its token revoked by host
     * poison     during the foreign window */
    i = eff_of(eff);
    if (!i) { host_unlease(host); rv_unlock(); return kOfxStatErrBadHandle; }
    if (st != kOfxStatOK) { host_unlease(host); rv_unlock(); return (int)st; }
    i->created = 1;
    i->desc->live++;
    host_unlease(host);
    rv_unlock();
    return kOfxStatOK;
}

int rv_ofx_instance_changed(void *eff, size_t idx) {
    RvEffect *i;
    RvHost *host;
    OfxPlugin *raw;
    uintptr_t itok;
    OfxStatus st = kOfxStatOK;
    OfxStatus st2;
    RvPropsObj *o;
    uintptr_t in;
    if (rv_lock()) return kOfxStatFailed;
    i = eff_of(eff);
    if (!i || !i->is_instance || !i->created) { rv_unlock(); return kOfxStatErrBadHandle; }
    if (idx >= i->nparams) { rv_unlock(); return kOfxStatErrBadIndex; }
    host = eff_host_lease(i);
    if (!host) { rv_unlock(); return kOfxStatErrBadHandle; }
    o = rv_po_new(i->tok);
    if (!o) { host_unlease(host); rv_unlock(); return kOfxStatErrMemory; }
    rv_po_set(o, kOfxPropName, RV_PT_STRING, 0, 0, 0, i->params[idx]->name, NULL);
    rv_po_set(o, kOfxPropType, RV_PT_STRING, 0, 0, 0, kOfxTypeParameter, NULL);
    rv_po_set(o, kOfxPropChangeReason, RV_PT_STRING, 0, 0, 0, kOfxChangeUserEdited, NULL);
    rv_po_set(o, kOfxPropTime, RV_PT_DOUBLE, 0, 0, 0.0, NULL, NULL);
    {
        double rs[2] = {1.0, 1.0};
        rv_po_setn(o, kOfxImageEffectPropRenderScale, RV_PT_DOUBLE, 2, NULL, rs, NULL, NULL);
    }
    in = o->tok;
    raw = i->desc->raw;
    itok = i->tok;
    rv_unlock();
    st = call_action(raw, kOfxActionBeginInstanceChanged,
                     (void *)itok, (void *)in, NULL);
    if (st == kOfxStatOK || st == kOfxStatReplyDefault) {
        st = call_action(raw, kOfxActionInstanceChanged,
                         (void *)itok, (void *)in, NULL);
    }
    st2 = call_action(raw, kOfxActionEndInstanceChanged,
                      (void *)itok, (void *)in, NULL);
    if (st == kOfxStatOK) st = st2;
    /* ReplyDefault means the action was ignored   not a failure */
    if (st == kOfxStatReplyDefault) st = kOfxStatOK;
    rv_lock();
    /* teardown inside the window makes the action result untrusted;
     * the lease kept the reads above and the pool reads below defined */
    if (!host->alive) st = kOfxStatErrBadHandle;
    rv_tok_revoke(in);
    rv_po_free(o);
    host_unlease(host);
    rv_unlock();
    return (int)st;
}

/* Seed the offered clip contract, run GetClipPreferences unlocked, and
 * verify the plugin left the fixed RGBA8/byte values intact. Called with
 * the lock held and a lease on the owning session host (non-NULL);
 * returns with the lock held. `i` is not dereferenced after the foreign
 * window. */
static OfxStatus rv_clip_prefs_validate(RvEffect *i, OfxPlugin *raw,
                                        uintptr_t itok, const RvHost *host) {
    OfxStatus st;
    RvPropsObj *o;
    uintptr_t out;
    o = rv_po_new(i->tok);
    if (!o) return kOfxStatErrMemory;
    rv_seed_s(o, "OfxImageClipPropComponents_Source", 1, kOfxImageComponentRGBA);
    rv_seed_s(o, "OfxImageClipPropComponents_Output", 1, kOfxImageComponentRGBA);
    rv_seed_s(o, "OfxImageClipPropDepth_Source", 1, kOfxBitDepthByte);
    rv_seed_s(o, "OfxImageClipPropDepth_Output", 1, kOfxBitDepthByte);
    rv_seed_d(o, "OfxImageClipPropPAR_Source", 1, 1.0);
    rv_seed_d(o, "OfxImageClipPropPAR_Output", 1, 1.0);
    rv_seed_s(o, kOfxImageClipPropFieldOrder, 1, kOfxImageFieldNone);
    rv_seed_d(o, kOfxImageEffectPropFrameRate, 1, i->fps);
    rv_seed_s(o, kOfxImageEffectPropPreMultiplication, 1, kOfxImageUnPreMultiplied);
    rv_seed_i(o, "OfxImageClipPropContinuousSamples", 1, 0);
    rv_seed_i(o, "OfxImageClipPropFrameVarying", 1, 0);
    out = o->tok;
    rv_unlock();
    st = call_action(raw, kOfxImageEffectActionGetClipPreferences,
                     (void *)itok, NULL, (void *)out);
    rv_lock();
    /* the lease keeps the `o` reads below defined when teardown landed
     * inside the window, but a dead session's result is untrusted */
    if (!host->alive) st = kOfxStatErrBadHandle;
    /* validate unchanged supported values   the fixed RGBA8/byte contract
     * the adapter can actually consume */
    if (st == kOfxStatOK || st == kOfxStatReplyDefault) {
        static const char *const want_s[][2] = {
            {"OfxImageClipPropComponents_Source", kOfxImageComponentRGBA},
            {"OfxImageClipPropComponents_Output", kOfxImageComponentRGBA},
            {"OfxImageClipPropDepth_Source", kOfxBitDepthByte},
            {"OfxImageClipPropDepth_Output", kOfxBitDepthByte},
            {kOfxImageClipPropFieldOrder, kOfxImageFieldNone},
            {kOfxImageEffectPropPreMultiplication, kOfxImageUnPreMultiplied},
        };
        static const char *const want_d[] = {
            "OfxImageClipPropPAR_Source", "OfxImageClipPropPAR_Output",
        };
        size_t k;
        for (k = 0; k < sizeof(want_s) / sizeof(want_s[0]); k++) {
            char *s = NULL;
            if (rv_po_get(o, want_s[k][0], RV_PT_STRING, 0,
                          NULL, NULL, &s, NULL) == kOfxStatOK &&
                s && strcmp(s, want_s[k][1]))
                { st = kOfxStatErrUnsupported; break; }
        }
        for (k = 0; st != kOfxStatErrUnsupported &&
                    k < sizeof(want_d) / sizeof(want_d[0]); k++) {
            double v = 0.0;
            if (rv_po_get(o, want_d[k], RV_PT_DOUBLE, 0,
                          NULL, &v, NULL, NULL) == kOfxStatOK &&
                v != 1.0)
                st = kOfxStatErrUnsupported;
        }
    }
    rv_tok_revoke(out);
    rv_po_free(o);
    return (st == kOfxStatOK || st == kOfxStatReplyDefault) ? kOfxStatOK : st;
}

int rv_ofx_instance_preferences(void *eff) {
    RvEffect *i;
    RvHost *host;
    OfxStatus st;
    if (rv_lock()) return kOfxStatFailed;
    i = eff_of(eff);
    if (!i || !i->is_instance || !i->created) { rv_unlock(); return kOfxStatErrBadHandle; }
    host = eff_host_lease(i);
    if (!host) { rv_unlock(); return kOfxStatErrBadHandle; }
    st = rv_clip_prefs_validate(i, i->desc->raw, i->tok, host);
    host_unlease(host);
    rv_unlock();
    return (int)st;
}

/* 16-byte aligned staging allocated per render call   independent of the
 * host memory pool so a render never depends on catalog/host liveness. */
static void *stage_alloc(size_t n) {
#if defined(_WIN32)
    return _aligned_malloc(n, 16);
#else
    void *p = NULL;
    return posix_memalign(&p, 16, n) ? NULL : p;
#endif
}
static void stage_free(void *p) {
#if defined(_WIN32)
    _aligned_free(p);
#else
    free(p);
#endif
}

int rv_ofx_instance_render(void *eff, const unsigned char *src,
                           size_t src_stride, unsigned char *dst,
                           size_t dst_stride, double time) {
    RvEffect *i;
    RvHost *host;
    OfxPlugin *raw;
    uintptr_t itok;
    OfxStatus st;
    RvPropsObj *inb = NULL, *inr = NULL;
    size_t stride, row_bytes, bytes, y;
    double fr[2], rs[2];
    int rw[4];
    if (rv_lock()) return (int)kOfxStatFailed;
    i = eff_of(eff);
    if (!i || !i->is_instance || !i->created || i->destroyed || i->quarantined ||
        !i->configured || i->render_active) {
        rv_unlock(); return (int)kOfxStatErrBadHandle;
    }
    if (!src || !dst || !isfinite(time)) {
        rv_unlock(); return (int)kOfxStatErrValue;
    }
    /* lease the owning session across the validate + render-bracket
     * windows: teardown inside an action keeps the block and pool
     * readable until the bracket closes */
    host = eff_host_lease(i);
    if (!host) { rv_unlock(); return (int)kOfxStatErrBadHandle; }
    row_bytes = (size_t)i->w * 4u;
    if (src_stride < row_bytes || dst_stride < row_bytes) {
        host_unlease(host); rv_unlock(); return (int)kOfxStatErrValue;
    }
    stride = (row_bytes + 15u) & ~(size_t)15u;
    bytes = stride * (size_t)i->h;
    i->stage_src = (unsigned char *)stage_alloc(bytes);
    i->stage_dst = (unsigned char *)stage_alloc(bytes);
    if (!i->stage_src || !i->stage_dst) {
        stage_free(i->stage_src); stage_free(i->stage_dst);
        i->stage_src = i->stage_dst = NULL;
        host_unlease(host); rv_unlock(); return (int)kOfxStatErrMemory;
    }
    i->stage_stride = stride;
    /* zero both buffers: deterministic row padding on src, and a defined
     * dst for a plugin that reports OK without covering the bounds */
    memset(i->stage_src, 0, bytes);
    memset(i->stage_dst, 0, bytes);
    for (y = 0; y < i->h; y++)
        memcpy(i->stage_src + y * stride, src + y * src_stride, row_bytes);
    i->render_active = 1;
    raw = i->desc->raw;
    itok = i->tok;

    /* unconditional clip-prefs revalidation before the bracket opens */
    st = rv_clip_prefs_validate(i, raw, itok, host);
    if (st == kOfxStatOK) {
        inb = rv_po_new(itok);
        inr = rv_po_new(itok);
        if (!inb || !inr) st = kOfxStatErrMemory;
    }
    if (st == kOfxStatOK) {
        fr[0] = time; fr[1] = time; rs[0] = 1.0; rs[1] = 1.0;
        rw[0] = 0; rw[1] = 0; rw[2] = (int)i->w; rw[3] = (int)i->h;
        rv_po_setn(inb, kOfxImageEffectPropFrameRange, RV_PT_DOUBLE, 2,
                   NULL, fr, NULL, NULL);
        rv_po_setn(inb, kOfxImageEffectPropRenderScale, RV_PT_DOUBLE, 2,
                   NULL, rs, NULL, NULL);
        rv_po_set(inb, kOfxPropIsInteractive, RV_PT_INT, 0, 0, 0, NULL, NULL);
        rv_po_set(inr, kOfxPropTime, RV_PT_DOUBLE, 0, 0, time, NULL, NULL);
        rv_po_setn(inr, kOfxImageEffectPropRenderWindow, RV_PT_INT, 4,
                   rw, NULL, NULL, NULL);
        rv_po_setn(inr, kOfxImageEffectPropRenderScale, RV_PT_DOUBLE, 2,
                   NULL, rs, NULL, NULL);
        rv_po_set(inr, kOfxImageEffectPropFieldToRender, RV_PT_STRING, 0,
                  0, 0, kOfxImageFieldNone, NULL);
        rv_unlock();
        st = call_action(raw, kOfxImageEffectActionBeginSequenceRender,
                         (void *)itok, (void *)(uintptr_t)inb->tok, NULL);
        /* ReplyDefault is a legal success for the optional sequence
         * bracket actions ("not trapped, but all is well anyway"); Render
         * itself is mandatory and stays strict */
        if (st == kOfxStatOK || st == kOfxStatReplyDefault) {
            OfxStatus st_end;
            st = call_action(raw, kOfxImageEffectActionRender,
                             (void *)itok, (void *)(uintptr_t)inr->tok, NULL);
            /* a failed Render still closes a successful Begin; the primary
             * error is preserved */
            st_end = call_action(raw, kOfxImageEffectActionEndSequenceRender,
                                 (void *)itok, (void *)(uintptr_t)inb->tok, NULL);
            if (st_end == kOfxStatReplyDefault) st_end = kOfxStatOK;
            if (st == kOfxStatOK) st = st_end;
        }
        rv_lock();
    }
    if (inb) { rv_tok_revoke(inb->tok); rv_po_free(inb); }
    if (inr) { rv_tok_revoke(inr->tok); rv_po_free(inr); }
    /* a session torn down inside a window leaves staged pixels the dead
     * host's plugin produced     untrusted, so the commit is skipped */
    if (st == kOfxStatOK && !host->alive) st = kOfxStatErrBadHandle;
    /* retire images the plugin failed to release inside the bracket */
    while (i->images) {
        RvImg *im = i->images;
        i->images = im->next;
        img_free(im);
    }
    i->render_active = 0;
    /* commit only on a fully successful bracket   output pixels and
     * row padding stay untouched on every failure path */
    if (st == kOfxStatOK)
        for (y = 0; y < i->h; y++)
            memcpy(dst + y * dst_stride, i->stage_dst + y * stride, row_bytes);
    stage_free(i->stage_src);
    stage_free(i->stage_dst);
    i->stage_src = i->stage_dst = NULL;
    i->stage_stride = 0;
    host_unlease(host);
    rv_unlock();
    return (int)st;
}

int rv_ofx_instance_destroy(void *eff) {
    RvEffect *i;
    RvHost *host;
    OfxPlugin *raw;
    uintptr_t itok;
    OfxStatus st;
    if (rv_lock()) return kOfxStatFailed;
    i = eff_of(eff);
    if (!i || !i->is_instance) { rv_unlock(); return kOfxStatErrBadHandle; }
    if (i->destroyed) { rv_unlock(); return kOfxStatFailed; }
    i->destroyed = 1;
    if (!i->created) { rv_unlock(); return kOfxStatOK; }
    if (i->render_active) {
        /* a destroy action concurrent with an open render bracket is a
         * contract violation; treat as uncertain   quarantine for close */
        i->quarantined = 1;
        if (i->desc->cat) {
            i->next_quar = i->desc->cat->quar;
            i->desc->cat->quar = i;
        }
        rv_unlock();
        return (int)kOfxStatFailed;
    }
    /* lease the owning session across the Destroy window   when the
     * owning session is already gone, running native Destroy would let
     * the plugin touch a reclaimed pool; park it as uncertain instead */
    host = eff_host_lease(i);
    if (!host) {
        i->quarantined = 1;
        if (i->desc->cat) {
            i->next_quar = i->desc->cat->quar;
            i->desc->cat->quar = i;
        }
        rv_unlock();
        return (int)kOfxStatErrBadHandle;
    }
    raw = i->desc->raw;
    itok = i->tok;
    rv_unlock();
    st = call_action(raw, kOfxActionDestroyInstance,
                     (void *)itok, NULL, NULL);
    rv_lock();
    i = eff_of(eff);
    if (!i) {
        host_unlease(host); rv_unlock();
        return (int)(st == kOfxStatOK ? kOfxStatErrBadHandle : st);
    }
    if (st != kOfxStatOK) {
        i->quarantined = 1;
        /* uncertain instance keeps the SingleInstance reservation and is
         * reclaimed only when its catalog runs the Unload teardown */
        if (i->desc->cat) {
            i->next_quar = i->desc->cat->quar;
            i->desc->cat->quar = i;
        }
        host_unlease(host);
        rv_unlock();
        return (int)st;
    }
    i->desc->live--;
    host_unlease(host);
    rv_unlock();
    return kOfxStatOK;
}

void rv_ofx_instance_release(void *eff) {
    RvEffect *i;
    RvHost *host;
    if (rv_lock()) return;
    i = eff_of(eff);
    if (!i || !i->is_instance) { rv_unlock(); return; }
    if (i->render_active) {
        /* freeing the attempt mid-bracket would orphan live image handles
         * and staging; park it on the quarantine list for catalog close */
        i->destroyed = 1;
        if (!i->quarantined && i->desc && i->desc->cat) {
            i->quarantined = 1;
            i->next_quar = i->desc->cat->quar;
            i->desc->cat->quar = i;
        }
        rv_unlock();
        return;
    }
    host = eff_host_lease(i);
    if (i->created && !i->destroyed) {
        OfxStatus st;
        OfxPlugin *raw = i->desc->raw;
        uintptr_t itok = i->tok;
        i->destroyed = 1;
        if (host) {
            rv_unlock();
            st = call_action(raw, kOfxActionDestroyInstance,
                             (void *)itok, NULL, NULL);
            rv_lock();
            i = eff_of(eff);
            if (!i) { host_unlease(host); rv_unlock(); return; }
            if (st == kOfxStatOK) {
                i->desc->live--;
            } else {
                i->quarantined = 1;
                if (i->desc->cat) {
                    i->next_quar = i->desc->cat->quar;
                    i->desc->cat->quar = i;
                }
            }
        } else {
            /* owning session is gone   the foreign Destroy would touch a
             * reclaimed pool, so the attempt is parked as uncertain
             * instead of running the action (Issue #131) */
            i->quarantined = 1;
            if (i->desc->cat) {
                i->next_quar = i->desc->cat->quar;
                i->desc->cat->quar = i;
            }
        }
    }
    if (!i->quarantined) {
        effect_free(i);
    }
    /* quarantined instances are freed at catalog close */
    host_unlease(host);
    rv_unlock();
}

/* ---------- ImageEffectV1 ---------- */

static RvEffect *eff_of_handle(OfxImageEffectHandle h) {
    return (RvEffect *)rv_tok_get((uintptr_t)h, H_EFFECT);
}

static OfxStatus fx_props(OfxImageEffectHandle h, OfxPropertySetHandle *props) {
    RvEffect *i;
    if (rv_lock()) return kOfxStatFailed;
    i = eff_of_handle(h);
    if (!i || !props) { rv_unlock(); return kOfxStatErrBadHandle; }
    *props = (OfxPropertySetHandle)(uintptr_t)i->props_tok;
    rv_unlock();
    return kOfxStatOK;
}

static OfxStatus fx_paramset(OfxImageEffectHandle h, OfxParamSetHandle *pset) {
    RvEffect *i;
    if (rv_lock()) return kOfxStatFailed;
    i = eff_of_handle(h);
    if (!i || !pset) { rv_unlock(); return kOfxStatErrBadHandle; }
    if (!i->pset_tok)
        i->pset_tok = rv_tok_new(H_PSET, i, i->tok);
    *pset = (OfxParamSetHandle)(uintptr_t)i->pset_tok;
    rv_unlock();
    return *pset ? kOfxStatOK : kOfxStatErrMemory;
}

static OfxStatus fx_clipDefine(OfxImageEffectHandle h, const char *name,
                               OfxPropertySetHandle *props) {
    RvEffect *i;
    RvClip *cl;
    if (rv_lock()) return kOfxStatFailed;
    i = eff_of_handle(h);
    if (!i || !name || !props) { rv_unlock(); return kOfxStatErrBadHandle; }
    /* duplicate clip name -> exists */
    for (cl = i->clips; cl; cl = cl->next)
        if (cl->name && !strcmp(cl->name, name)) {
            *props = (OfxPropertySetHandle)(uintptr_t)cl->props_tok;
            rv_unlock(); return kOfxStatOK;
        }
    cl = (RvClip *)calloc(1, sizeof(RvClip));
    if (!cl) { rv_unlock(); return kOfxStatErrMemory; }
    cl->name = rv_str_own(g_active, name);
    cl->props = rv_po_new(i->tok);
    cl->tok = rv_tok_new(H_CLIP, cl, i->tok);
    if (cl->props) cl->props_tok = cl->props->tok;
    cl->eff = i;
    if (!cl->name || !cl->props || !cl->tok) {
        rv_tok_revoke(cl->tok);
        if (cl->props) rv_tok_revoke(cl->props->tok);
        rv_po_free(cl->props); free(cl); rv_unlock(); return kOfxStatErrMemory; }
    cl->next = i->clips;
    i->clips = cl;
    *props = (OfxPropertySetHandle)(uintptr_t)cl->props_tok;
    rv_unlock();
    return kOfxStatOK;
}

static OfxStatus fx_clipGetHandle(OfxImageEffectHandle h, const char *name,
                                  OfxImageClipHandle *clip,
                                  OfxPropertySetHandle *props) {
    RvEffect *i;
    RvClip *cl;
    if (rv_lock()) return kOfxStatFailed;
    i = eff_of_handle(h);
    if (!i || !name || !clip) { rv_unlock(); return kOfxStatErrBadHandle; }
    for (cl = i->clips; cl; cl = cl->next)
        if (cl->name && !strcmp(cl->name, name)) {
            *clip = (OfxImageClipHandle)(uintptr_t)cl->tok;
            if (props) *props = (OfxPropertySetHandle)(uintptr_t)cl->props_tok;
            rv_unlock(); return kOfxStatOK;
        }
    *clip = NULL;
    rv_unlock();
    return kOfxStatErrUnknown;
}

static OfxStatus fx_clipProps(OfxImageClipHandle c, OfxPropertySetHandle *props) {
    RvClip *cl;
    if (rv_lock()) return kOfxStatFailed;
    cl = (RvClip *)rv_tok_get((uintptr_t)c, H_CLIP);
    if (!cl || !props) { rv_unlock(); return kOfxStatErrBadHandle; }
    *props = (OfxPropertySetHandle)(uintptr_t)cl->props_tok;
    rv_unlock();
    return kOfxStatOK;
}

static void img_free(RvImg *im) {
    if (!im) return;
    rv_tok_revoke(im->tok);
    if (im->props) rv_tok_revoke(im->props->tok);
    rv_po_free(im->props);
    free(im);
}

static OfxStatus fx_clipGetImage(OfxImageClipHandle c, OfxTime t,
                                 const OfxRectD *r, OfxPropertySetHandle *img) {
    RvClip *cl;
    RvEffect *i;
    RvImg *im;
    unsigned char *base;
    int rect[4];
    double rs[2];
    (void)t; (void)r;
    if (!img) return kOfxStatErrBadHandle;
    *img = NULL;
    if (rv_lock()) return kOfxStatFailed;
    cl = (RvClip *)rv_tok_get((uintptr_t)c, H_CLIP);
    if (!cl || !cl->eff) { rv_unlock(); return kOfxStatErrBadHandle; }
    i = cl->eff;
    /* images exist only while a render bracket is open on this instance */
    if (!i->render_active || !i->stage_src || !i->stage_dst) {
        rv_unlock(); return kOfxStatFailed;
    }
    /* only Source and Output route to staging; an auxiliary input clip is
     * outside the fixed contract this adapter exposes */
    if (cl->name && !strcmp(cl->name, kOfxImageEffectSimpleSourceClipName))
        base = i->stage_src;
    else if (cl->name && !strcmp(cl->name, kOfxImageEffectOutputClipName))
        base = i->stage_dst;
    else { rv_unlock(); return kOfxStatFailed; }
    im = (RvImg *)calloc(1, sizeof(RvImg));
    if (!im) { rv_unlock(); return kOfxStatErrMemory; }
    im->clip = cl;
    im->props = rv_po_new(cl->tok);
    if (!im->props) { free(im); rv_unlock(); return kOfxStatErrMemory; }
    im->tok = rv_tok_new(H_IMAGE, im, cl->tok);
    if (!im->tok) { img_free(im); rv_unlock(); return kOfxStatErrMemory; }
    /* bottom-left exposure: data pointer on the last physical row and a
     * negative rowBytes so image y=0 is the frame's bottom row */
    base += (size_t)(i->h - 1) * i->stage_stride;
    rect[0] = 0; rect[1] = 0;
    rect[2] = (int)i->w; rect[3] = (int)i->h;
    rs[0] = 1.0; rs[1] = 1.0;
    rv_po_set(im->props, kOfxImagePropData, RV_PT_POINTER, 0, 0, 0, NULL, base);
    rv_po_set(im->props, kOfxImagePropRowBytes, RV_PT_INT, 0,
              -(int)i->stage_stride, 0, NULL, NULL);
    rv_po_setn(im->props, kOfxImagePropBounds, RV_PT_INT, 4, rect, NULL, NULL, NULL);
    rv_po_setn(im->props, kOfxImagePropRegionOfDefinition, RV_PT_INT, 4,
               rect, NULL, NULL, NULL);
    rv_po_set(im->props, kOfxImageEffectPropPixelDepth, RV_PT_STRING, 0,
              0, 0, kOfxBitDepthByte, NULL);
    rv_po_set(im->props, kOfxImageEffectPropComponents, RV_PT_STRING, 0,
              0, 0, kOfxImageComponentRGBA, NULL);
    rv_po_setn(im->props, kOfxImageEffectPropRenderScale, RV_PT_DOUBLE, 2,
               NULL, rs, NULL, NULL);
    rv_po_set(im->props, kOfxImagePropField, RV_PT_STRING, 0, 0, 0,
              kOfxImageFieldNone, NULL);
    rv_po_set(im->props, kOfxImagePropPixelAspectRatio, RV_PT_DOUBLE, 0,
              0, 1.0, NULL, NULL);
    rv_po_set(im->props, kOfxImageEffectPropPreMultiplication, RV_PT_STRING,
              0, 0, 0, kOfxImageUnPreMultiplied, NULL);
    rv_po_set(im->props, kOfxPropType, RV_PT_STRING, 0, 0, 0,
              kOfxTypeImage, NULL);
    rv_po_set(im->props, kOfxImagePropUniqueIdentifier, RV_PT_STRING, 0,
              0, 0, "ruviform-staging", NULL);
    im->next = i->images;
    i->images = im;
    *img = (OfxPropertySetHandle)(uintptr_t)im->tok;
    rv_unlock();
    return kOfxStatOK;
}
static OfxStatus fx_clipRelease(OfxPropertySetHandle img) {
    RvImg *im;
    RvImg **link;
    RvEffect *i;
    if (rv_lock()) return kOfxStatFailed;
    im = (RvImg *)rv_tok_get((uintptr_t)img, H_IMAGE);
    if (!im) { rv_unlock(); return kOfxStatErrBadHandle; }
    i = im->clip ? im->clip->eff : NULL;
    if (i)
        for (link = &i->images; *link; link = &(*link)->next)
            if (*link == im) { *link = im->next; break; }
    img_free(im);
    rv_unlock();
    return kOfxStatOK;
}
static OfxStatus fx_clipRoD(OfxImageClipHandle c, OfxTime t, OfxRectD *r) {
    RvClip *cl;
    (void)t;
    if (rv_lock()) return kOfxStatFailed;
    cl = (RvClip *)rv_tok_get((uintptr_t)c, H_CLIP);
    if (!cl || !cl->eff || !cl->eff->configured || !r) {
        rv_unlock(); return kOfxStatErrBadHandle;
    }
    r->x1 = 0.0; r->y1 = 0.0;
    r->x2 = (double)cl->eff->w; r->y2 = (double)cl->eff->h;
    rv_unlock();
    return kOfxStatOK;
}
static int fx_abort(OfxImageEffectHandle h) { (void)h; return 0; }

static OfxStatus fx_imgAlloc(OfxImageEffectHandle h, size_t n, OfxImageMemoryHandle *mem) {
    void *p = NULL;
    uintptr_t t;
    if (!mem) return kOfxStatErrBadHandle;
    *mem = NULL;
    if (rv_lock()) return kOfxStatFailed;
    if (h && !eff_of_handle(h)) { rv_unlock(); return kOfxStatErrBadHandle; }
    if (rv_mem_alloc(g_active, n, &p) != kOfxStatOK) { rv_unlock(); return kOfxStatErrMemory; }
    t = rv_tok_new(H_MEMORY, p, 0);
    if (!t) { rv_mem_free(g_active, p); rv_unlock(); return kOfxStatErrMemory; }
    *mem = (OfxImageMemoryHandle)(uintptr_t)t;
    rv_unlock();
    return kOfxStatOK;
}
static OfxStatus fx_imgLock(OfxImageMemoryHandle m, void **ptr) {
    void *p;
    if (!ptr) return kOfxStatErrBadHandle;
    *ptr = NULL;
    if (rv_lock()) return kOfxStatFailed;
    p = rv_tok_get((uintptr_t)m, H_MEMORY);
    if (!p) { rv_unlock(); return kOfxStatErrBadHandle; }
    *ptr = p;
    rv_unlock();
    return kOfxStatOK;
}
static OfxStatus fx_imgUnlock(OfxImageMemoryHandle m) {
    if (rv_lock()) return kOfxStatFailed;
    if (!rv_tok_get((uintptr_t)m, H_MEMORY)) { rv_unlock(); return kOfxStatErrBadHandle; }
    rv_unlock();
    return kOfxStatOK;
}
static OfxStatus fx_imgFree(OfxImageMemoryHandle m) {
    void *p;
    if (rv_lock()) return kOfxStatFailed;
    p = rv_tok_get((uintptr_t)m, H_MEMORY);
    if (!p) { rv_unlock(); return kOfxStatErrBadHandle; }
    rv_tok_revoke((uintptr_t)m);
    rv_mem_free(g_active, p);
    rv_unlock();
    return kOfxStatOK;
}

static OfxImageEffectSuiteV1 g_fx_suite = {
    .getPropertySet = fx_props,
    .getParamSet = fx_paramset,
    .clipDefine = fx_clipDefine,
    .clipGetHandle = fx_clipGetHandle,
    .clipGetPropertySet = fx_clipProps,
    .clipGetImage = fx_clipGetImage,
    .clipReleaseImage = fx_clipRelease,
    .clipGetRegionOfDefinition = fx_clipRoD,
    .abort = fx_abort,
    .imageMemoryAlloc = fx_imgAlloc,
    .imageMemoryLock = fx_imgLock,
    .imageMemoryUnlock = fx_imgUnlock,
    .imageMemoryFree = fx_imgFree,
};
const void *rv_fx_suite_ptr(void) { return &g_fx_suite; }
