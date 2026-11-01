/* ofx_registry.c   global recursive mutex, token registry, host-owned
 * strings/aligned memory, property storage, PropertyV1 + MemoryV1. */
#include <stdlib.h>
#include <string.h>
#include <limits.h>
#include "ofx_internal.h"

RvHost *g_active = NULL;

/* ---------- recursive global mutex, initialized exactly once ---------- */
#if defined(_WIN32)
static CRITICAL_SECTION g_mu;
static INIT_ONCE g_once = INIT_ONCE_STATIC_INIT;
static BOOL CALLBACK once_fn(PINIT_ONCE i, PVOID p, PVOID *c) {
    (void)i; (void)p; (void)c;
    InitializeCriticalSection(&g_mu);
    return TRUE;
}
int rv_lock(void) {
    if (!InitOnceExecuteOnce(&g_once, once_fn, NULL, NULL)) return 1;
    EnterCriticalSection(&g_mu);
    return 0;
}
void rv_unlock(void) { LeaveCriticalSection(&g_mu); }
#else
static pthread_mutex_t g_mu;
static pthread_once_t g_once = PTHREAD_ONCE_INIT;
static void once_fn(void) {
    pthread_mutexattr_t a;
    pthread_mutexattr_init(&a);
    pthread_mutexattr_settype(&a, PTHREAD_MUTEX_RECURSIVE);
    pthread_mutex_init(&g_mu, &a);
    pthread_mutexattr_destroy(&a);
}
int rv_lock(void) {
    if (pthread_once(&g_once, once_fn) != 0) return 1;
    pthread_mutex_lock(&g_mu);
    return 0;
}
void rv_unlock(void) { pthread_mutex_unlock(&g_mu); }
#endif

/* ---------- token registry (owner + parent cascade) ---------- */
typedef struct RvTok {
    uintptr_t tok;
    int kind;
    void *payload;
    uintptr_t parent;
    struct RvTok *next;
} RvTok;
static RvTok *g_toks = NULL;
static uintptr_t g_next = 0x10000; /* never reset across sessions */

/* caller holds g_mu */
uintptr_t rv_tok_new(int kind, void *payload, uintptr_t parent) {
    RvTok *n;
    if (g_next == (uintptr_t)-1) return 0;
    n = (RvTok *)calloc(1, sizeof(*n));
    if (!n) return 0;
    n->tok = g_next++;
    n->kind = kind;
    n->payload = payload;
    n->parent = parent;
    n->next = g_toks;
    g_toks = n;
    return n->tok;
}

void *rv_tok_get(uintptr_t tok, int kind) {
    RvTok *n;
    if (tok < 0x10000) return NULL;
    for (n = g_toks; n; n = n->next)
        if (n->tok == tok) return (kind < 0 || n->kind == kind) ? n->payload : NULL;
    return NULL;
}

void rv_tok_sweep(uintptr_t keep) {
    for (;;) {
        RvTok *n;
        uintptr_t z = 0;
        for (n = g_toks; n; n = n->next)
            if (n->tok != keep) { z = n->tok; break; }
        if (!z) return;
        rv_tok_revoke(z);
    }
}

void rv_tok_revoke(uintptr_t tok) {
    RvTok **n;
    for (n = &g_toks; *n; n = &(*n)->next)
        if ((*n)->tok == tok) {
            RvTok *d = *n;
            *n = d->next;
            free(d);
            /* cascade: revoke children. revoking a child rewrites the
             * list, so rescan from the head until no orphan remains */
            for (;;) {
                RvTok *c;
                for (c = g_toks; c; c = c->next)
                    if (c->parent == tok) break;
                if (!c) return;
                { uintptr_t ct = c->tok; c->parent = 0; rv_tok_revoke(ct); }
            }
        }
}

/* ---------- host-owned strings / aligned memory ---------- */
char *rv_str_own(RvHost *h, const char *s) {
    RvStrNode *n;
    size_t len;
    if (!h || !s) return NULL;
    len = strlen(s) + 1;
    n = (RvStrNode *)malloc(sizeof(*n));
    if (!n) return NULL;
    n->s = (char *)malloc(len);
    if (!n->s) { free(n); return NULL; }
    memcpy(n->s, s, len);
    n->next = h->strings;
    h->strings = n;
    return n->s;
}

OfxStatus rv_mem_alloc(RvHost *h, size_t n, void **out) {
    RvMemNode *m;
    if (!h || !out) return kOfxStatErrBadHandle;
    *out = NULL;
    if (n > (size_t)PTRDIFF_MAX - 16) return kOfxStatErrMemory;
    m = (RvMemNode *)malloc(sizeof(*m));
    if (!m) return kOfxStatErrMemory;
#if defined(_WIN32)
    m->p = _aligned_malloc(n ? n : 1, 16);
#else
    m->p = NULL;
    if (posix_memalign(&m->p, 16, n ? n : 1) != 0) m->p = NULL;
#endif
    if (!m->p) { free(m); return kOfxStatErrMemory; }
    m->next = h->mem;
    h->mem = m;
    *out = m->p;
    return kOfxStatOK;
}

OfxStatus rv_mem_free(RvHost *h, void *ptr) {
    RvMemNode **n;
    RvTok *t;
    if (!h || !ptr) return kOfxStatErrBadHandle;
    /* retire the H_MEMORY token first so a stale handle cannot resolve
     * the buffer while it is being freed */
    for (t = g_toks; t; t = t->next)
        if (t->kind == H_MEMORY && t->payload == ptr) {
            rv_tok_revoke(t->tok);
            break;
        }
    for (n = &h->mem; *n; n = &(*n)->next)
        if ((*n)->p == ptr) {
            RvMemNode *d = *n;
            *n = d->next;
#if defined(_WIN32)
            _aligned_free(d->p);
#else
            free(d->p);
#endif
            free(d);
            return kOfxStatOK;
        }
    return kOfxStatErrBadHandle;
}

/* ---------- property storage ---------- */
/* callers hold g_mu */

RvPropsObj *rv_po_new(uintptr_t parent) {
    RvPropsObj *o = (RvPropsObj *)calloc(1, sizeof(*o));
    if (!o) return NULL;
    o->tok = rv_tok_new(H_EPROPS, o, parent);
    if (!o->tok) { free(o); return NULL; }
    return o;
}

static RvProp *pfind(RvPropsObj *o, const char *name) {
    RvProp *p;
    for (p = o->head; p; p = p->next)
        if (!strcmp(p->name, name)) return p;
    return NULL;
}

static RvProp *pensure(RvPropsObj *o, const char *name, int type) {
    RvProp *p = pfind(o, name);
    if (p) return p;
    p = (RvProp *)calloc(1, sizeof(*p));
    if (!p) return NULL;
    p->name = rv_str_own(g_active, name);
    if (!p->name) { free(p); return NULL; }
    p->type = type;
    p->next = o->head;
    o->head = p;
    return p;
}

static int grow(RvProp *p, int type, int need) {
    if (need <= p->cap) return 1;
    {
        int nc = need + 4, k;
        size_t sz = (type == RV_PT_INT || type == RV_PT_DOUBLE) ? sizeof(double) : sizeof(char *);
        void **base = (type == RV_PT_INT) ? (void **)&p->vi
                    : (type == RV_PT_DOUBLE) ? (void **)&p->vd
                    : (type == RV_PT_STRING) ? (void **)&p->vs : (void **)&p->vp;
        void *np = realloc(*base, sz * (size_t)nc);
        if (!np) return 0;
        *base = np;
        for (k = p->cap; k < nc; k++) {
            if (type == RV_PT_INT) p->vi[k] = 0;
            else if (type == RV_PT_DOUBLE) p->vd[k] = 0.0;
            else if (type == RV_PT_STRING) p->vs[k] = NULL;
            else p->vp[k] = NULL;
        }
        p->cap = nc;
    }
    return 1;
}

OfxStatus rv_po_set(RvPropsObj *o, const char *n, int t, int i,
                    int vi, double vd, const char *vs, void *vp) {
    RvProp *p;
    if (!o) return kOfxStatErrBadHandle;
    /* i+1 and grow's need+4 must stay inside int range */
    if (!n || i < 0 || i > INT_MAX - 8) return kOfxStatErrBadIndex;
    if (t != RV_PT_INT && t != RV_PT_DOUBLE && t != RV_PT_STRING && t != RV_PT_POINTER)
        return kOfxStatErrUnknown;
    p = pfind(o, n);
    if (p && p->type != t) return kOfxStatErrUnknown;
    if (!p) {
        p = pensure(o, n, t);
        if (!p) return kOfxStatErrMemory;
    }
    if (!grow(p, t, i + 1)) return kOfxStatErrMemory;
    if (t == RV_PT_INT) p->vi[i] = vi;
    else if (t == RV_PT_DOUBLE) p->vd[i] = vd;
    else if (t == RV_PT_STRING) {
        /* strings live in the host pool   stable pointers, retired not freed */
        char *cp = rv_str_own(g_active, vs ? vs : "");
        if (!cp) return kOfxStatErrMemory;
        p->vs[i] = cp;
    } else p->vp[i] = vp;
    if (i >= p->dim) p->dim = i + 1;
    return kOfxStatOK;
}

OfxStatus rv_po_setn(RvPropsObj *o, const char *n, int t, int c,
                     const int *vi, const double *vd,
                     const char *const *vs, void *const *vp) {
    int k;
    OfxStatus st;
    if (!o || !n) return kOfxStatErrBadHandle;
    if (c < 0 || (c > 0 && !vi && !vd && !vs && !vp)) return kOfxStatErrValue;
    for (k = 0; k < c; k++) {
        st = rv_po_set(o, n, t, k,
                       vi ? vi[k] : 0, vd ? vd[k] : 0.0,
                       vs ? vs[k] : NULL, vp ? vp[k] : NULL);
        if (st != kOfxStatOK) return st;
    }
    return kOfxStatOK;
}

OfxStatus rv_po_getn(RvPropsObj *o, const char *n, int t, int c,
                     int *vi, double *vd, char **vs, void **vp) {
    RvProp *p;
    int k;
    if (!o) return kOfxStatErrBadHandle;
    if (!n || c < 0) return kOfxStatErrBadIndex;
    p = pfind(o, n);
    if (!p) return kOfxStatErrUnknown;
    if (p->type != t) return kOfxStatErrUnknown;
    if (c > p->dim) return kOfxStatErrBadIndex;
    for (k = 0; k < c; k++) {
        if (t == RV_PT_INT) vi[k] = p->vi[k];
        else if (t == RV_PT_DOUBLE) vd[k] = p->vd[k];
        else if (t == RV_PT_STRING) vs[k] = p->vs[k];
        else vp[k] = p->vp[k];
    }
    return kOfxStatOK;
}

OfxStatus rv_po_get(RvPropsObj *o, const char *n, int t, int i,
                    int *vi, double *vd, char **vs, void **vp) {
    RvProp *p;
    if (!o) return kOfxStatErrBadHandle;
    if (!n || i < 0) return kOfxStatErrBadIndex;
    p = pfind(o, n);
    if (!p) return kOfxStatErrUnknown;
    if (p->type != t) return kOfxStatErrUnknown;
    if (i >= p->dim) return kOfxStatErrBadIndex;
    if (t == RV_PT_INT) *vi = p->vi[i];
    else if (t == RV_PT_DOUBLE) *vd = p->vd[i];
    else if (t == RV_PT_STRING) *vs = p->vs[i];
    else *vp = p->vp[i];
    return kOfxStatOK;
}

OfxStatus rv_po_dim(RvPropsObj *o, const char *n, int *dim) {
    RvProp *p;
    if (!o || !n || !dim) return kOfxStatErrBadHandle;
    p = pfind(o, n);
    if (!p) return kOfxStatErrUnknown;
    *dim = p->dim;
    return kOfxStatOK;
}

OfxStatus rv_po_reset(RvPropsObj *o, const char *n) {
    RvProp *p;
    int k;
    if (!o || !n) return kOfxStatErrBadHandle;
    p = pfind(o, n);
    if (!p) return kOfxStatErrUnknown;
    if (!p->seeded) return kOfxStatOK;
    if (!grow(p, p->type, p->ddim)) return kOfxStatErrMemory;
    p->dim = p->ddim;
    for (k = 0; k < p->ddim; k++) {
        if (p->type == RV_PT_INT) p->vi[k] = p->di[k];
        else if (p->type == RV_PT_DOUBLE) p->vd[k] = p->dd[k];
        else if (p->type == RV_PT_STRING) p->vs[k] = p->ds[k];
        else p->vp[k] = p->dp[k];
    }
    return kOfxStatOK;
}

static void seed(RvPropsObj *o, const char *n, int t, int dim,
                 int vi, double vd, const char *vs, void *vp) {
    RvProp *p;
    int k;
    rv_po_set(o, n, t, 0, vi, vd, vs, vp);
    p = pfind(o, n);
    if (!p || p->seeded) return;
    /* mark seeded only after the snapshot array exists   a failed calloc
     * must not leave rv_po_reset dereferencing a NULL snapshot array
     * (Issue #131) */
    {
        int ddim = dim > 0 ? dim : 1;
        int ok = 0;
        if (p->type == RV_PT_INT) {
            p->di = (int *)calloc((size_t)ddim, sizeof(int));
            if (p->di) { for (k = 0; k < ddim; k++) p->di[k] = vi; ok = 1; }
        } else if (p->type == RV_PT_DOUBLE) {
            p->dd = (double *)calloc((size_t)ddim, sizeof(double));
            if (p->dd) { for (k = 0; k < ddim; k++) p->dd[k] = vd; ok = 1; }
        } else if (p->type == RV_PT_STRING) {
            p->ds = (char **)calloc((size_t)ddim, sizeof(char *));
            /* a NULL element would reach propGetString as a NULL result
             * on reset   mark seeded only when every snapshot string
             * actually pooled (Issue #131) */
            if (p->ds) {
                ok = 1;
                for (k = 0; k < ddim; k++) {
                    p->ds[k] = rv_str_own(g_active, vs ? vs : "");
                    if (!p->ds[k]) { ok = 0; break; }
                }
            }
        } else {
            p->dp = (void **)calloc((size_t)ddim, sizeof(void *));
            if (p->dp) { for (k = 0; k < ddim; k++) p->dp[k] = vp; ok = 1; }
        }
        if (!ok) return;
        p->ddim = ddim;
        p->seeded = 1;
    }
}
void rv_seed_i(RvPropsObj *o, const char *n, int d, int v) { seed(o, n, RV_PT_INT, d, v, 0, NULL, NULL); }
void rv_seed_d(RvPropsObj *o, const char *n, int d, double v) { seed(o, n, RV_PT_DOUBLE, d, 0, v, NULL, NULL); }
void rv_seed_s(RvPropsObj *o, const char *n, int d, const char *v) { seed(o, n, RV_PT_STRING, d, 0, 0, v, NULL); }
void rv_seed_p(RvPropsObj *o, const char *n, int d, void *v) { seed(o, n, RV_PT_POINTER, d, 0, 0, NULL, v); }

/* frees one prop record's arrays; name and string payloads are
 * host-pool owned   retired with the host, never freed here */
static void prop_free(RvProp *p) {
    if (!p) return;
    free(p->vi); free(p->vd); free(p->vp);
    free(p->ds);
    free(p->vs);
    free(p->di); free(p->dd); free(p->dp);
    free(p);
}

/* drop every record but keep the propset object and its token: the
 * record structs and their value/snapshot arrays are heap-owned, while
 * the name/string payloads they reference belong to the session host's
 * string pool. a process-lifetime propset (the dedicated api.host set)
 * must retire its records when the session dies so it can never reach
 * into a reclaimed pool; the next session's writes rebuild fresh
 * records on its own pool (Issue #131) */
void rv_po_clear(RvPropsObj *o) {
    RvProp *p;
    if (!o) return;
    p = o->head;
    o->head = NULL;
    while (p) {
        RvProp *n = p->next;
        prop_free(p);
        p = n;
    }
}

void rv_po_free(RvPropsObj *o) {
    if (!o) return;
    rv_po_clear(o);
    free(o);
}

/* clone the value array of one prop, mirroring grow()'s element sizing
 * (int/double share double-width slots, string/pointer share ptr slots) */
static int vals_clone(RvProp *q, const RvProp *p) {
    size_t esz = (p->type == RV_PT_INT || p->type == RV_PT_DOUBLE)
               ? sizeof(double) : sizeof(char *);
    const void *sv;
    void *dv;
    if (p->cap <= 0) return 1;
    sv = (p->type == RV_PT_INT) ? (const void *)p->vi
       : (p->type == RV_PT_DOUBLE) ? (const void *)p->vd
       : (p->type == RV_PT_STRING) ? (const void *)p->vs : (const void *)p->vp;
    if (!sv) return 1;
    dv = malloc(esz * (size_t)p->cap);
    if (!dv) return 0;
    memcpy(dv, sv, esz * (size_t)p->cap);
    if (p->type == RV_PT_INT) q->vi = (int *)dv;
    else if (p->type == RV_PT_DOUBLE) q->vd = (double *)dv;
    else if (p->type == RV_PT_STRING) q->vs = (char **)dv;
    else q->vp = (void **)dv;
    return 1;
}

/* clone the reset snapshot array (ddim elements of the matching type);
 * an unseeded prop or a prop whose snapshot was never allocated mirrors
 * as absent */
static int seed_clone(RvProp *q, const RvProp *p) {
    size_t n = (size_t)p->ddim;
    if (!p->seeded || p->ddim <= 0) return 1;
    if (p->type == RV_PT_INT) {
        if (!p->di) return 1;
        q->di = (int *)malloc(sizeof(int) * n);
        if (!q->di) return 0;
        memcpy(q->di, p->di, sizeof(int) * n);
    } else if (p->type == RV_PT_DOUBLE) {
        if (!p->dd) return 1;
        q->dd = (double *)malloc(sizeof(double) * n);
        if (!q->dd) return 0;
        memcpy(q->dd, p->dd, sizeof(double) * n);
    } else if (p->type == RV_PT_STRING) {
        if (!p->ds) return 1;
        q->ds = (char **)malloc(sizeof(char *) * n);
        if (!q->ds) return 0;
        memcpy(q->ds, p->ds, sizeof(char *) * n);
    } else {
        if (!p->dp) return 1;
        q->dp = (void **)malloc(sizeof(void *) * n);
        if (!q->dp) return 0;
        memcpy(q->dp, p->dp, sizeof(void *) * n);
    }
    return 1;
}

OfxStatus rv_po_clone(RvPropsObj *dst, const RvPropsObj *src) {
    RvProp *p;
    if (!dst || !src) return kOfxStatErrBadHandle;
    for (p = src->head; p; p = p->next) {
        RvProp *q = (RvProp *)calloc(1, sizeof(*q));
        if (!q) return kOfxStatErrMemory;
        q->name = p->name; /* host-pool string   shared, retires with host */
        q->type = p->type;
        q->dim = p->dim;
        q->cap = p->cap;
        q->seeded = p->seeded;
        q->ddim = p->ddim;
        if (!vals_clone(q, p) || !seed_clone(q, p)) {
            prop_free(q);
            return kOfxStatErrMemory;
        }
        q->next = dst->head;
        dst->head = q;
    }
    return kOfxStatOK;
}

/* ---------- PropertyV1 (designated initializers, SDK order) ---------- */

/* only property-set-token kinds are legal   resolve then check payload */
static RvPropsObj *po_of_props(OfxPropertySetHandle h) {
    uintptr_t t = (uintptr_t)h;
    if (t >= 0x10000) {
        RvPropsObj *o = (RvPropsObj *)rv_tok_get(t, H_EPROPS);
        if (o) return o;
        o = (RvPropsObj *)rv_tok_get(t, H_PPROPS); if (o) return o;
        o = (RvPropsObj *)rv_tok_get(t, H_CPROPS); if (o) return o;
        o = (RvPropsObj *)rv_tok_get(t, H_INARGS); if (o) return o;
        o = (RvPropsObj *)rv_tok_get(t, H_OUTARGS); if (o) return o;
        {
            /* an image handle resolves to its own property set */
            RvImg *im = (RvImg *)rv_tok_get(t, H_IMAGE);
            if (im) return im->props;
        }
    }
    return NULL;
}

#define PO(h) po_of_props(h)

static OfxStatus ps_ptr(OfxPropertySetHandle h, const char *n, int i, void *v) {
    if (rv_lock()) return kOfxStatFailed;
    { OfxStatus s = rv_po_set(PO(h), n, RV_PT_POINTER, i, 0, 0, NULL, v); rv_unlock(); return s; }
}
static OfxStatus ps_dbl(OfxPropertySetHandle h, const char *n, int i, double v) {
    if (rv_lock()) return kOfxStatFailed;
    { OfxStatus s = rv_po_set(PO(h), n, RV_PT_DOUBLE, i, 0, v, NULL, NULL); rv_unlock(); return s; }
}
static OfxStatus ps_str(OfxPropertySetHandle h, const char *n, int i, const char *v) {
    if (rv_lock()) return kOfxStatFailed;
    { OfxStatus s = rv_po_set(PO(h), n, RV_PT_STRING, i, 0, 0, v, NULL); rv_unlock(); return s; }
}
static OfxStatus ps_int(OfxPropertySetHandle h, const char *n, int i, int v) {
    if (rv_lock()) return kOfxStatFailed;
    { OfxStatus s = rv_po_set(PO(h), n, RV_PT_INT, i, v, 0, NULL, NULL); rv_unlock(); return s; }
}

static OfxStatus ps_ptrn(OfxPropertySetHandle h, const char *n, int c, void *const *v) {
    if (rv_lock()) return kOfxStatFailed;
    { OfxStatus s = rv_po_setn(PO(h), n, RV_PT_POINTER, c, NULL, NULL, NULL, v); rv_unlock(); return s; }
}
static OfxStatus ps_dbln(OfxPropertySetHandle h, const char *n, int c, const double *v) {
    if (rv_lock()) return kOfxStatFailed;
    { OfxStatus s = rv_po_setn(PO(h), n, RV_PT_DOUBLE, c, NULL, v, NULL, NULL); rv_unlock(); return s; }
}
static OfxStatus ps_strn(OfxPropertySetHandle h, const char *n, int c, const char *const *v) {
    if (rv_lock()) return kOfxStatFailed;
    { OfxStatus s = rv_po_setn(PO(h), n, RV_PT_STRING, c, NULL, NULL, v, NULL); rv_unlock(); return s; }
}
static OfxStatus ps_intn(OfxPropertySetHandle h, const char *n, int c, const int *v) {
    if (rv_lock()) return kOfxStatFailed;
    { OfxStatus s = rv_po_setn(PO(h), n, RV_PT_INT, c, v, NULL, NULL, NULL); rv_unlock(); return s; }
}

static OfxStatus pg_ptr(OfxPropertySetHandle h, const char *n, int i, void **v) {
    if (!v) return kOfxStatErrBadHandle;
    if (rv_lock()) return kOfxStatFailed;
    { OfxStatus s = rv_po_get(PO(h), n, RV_PT_POINTER, i, NULL, NULL, NULL, v); rv_unlock(); return s; }
}
static OfxStatus pg_dbl(OfxPropertySetHandle h, const char *n, int i, double *v) {
    if (!v) return kOfxStatErrBadHandle;
    if (rv_lock()) return kOfxStatFailed;
    { OfxStatus s = rv_po_get(PO(h), n, RV_PT_DOUBLE, i, NULL, v, NULL, NULL); rv_unlock(); return s; }
}
static OfxStatus pg_str(OfxPropertySetHandle h, const char *n, int i, char **v) {
    if (!v) return kOfxStatErrBadHandle;
    if (rv_lock()) return kOfxStatFailed;
    { OfxStatus s = rv_po_get(PO(h), n, RV_PT_STRING, i, NULL, NULL, v, NULL); rv_unlock(); return s; }
}
static OfxStatus pg_int(OfxPropertySetHandle h, const char *n, int i, int *v) {
    if (!v) return kOfxStatErrBadHandle;
    if (rv_lock()) return kOfxStatFailed;
    { OfxStatus s = rv_po_get(PO(h), n, RV_PT_INT, i, v, NULL, NULL, NULL); rv_unlock(); return s; }
}

static OfxStatus pg_ptrn(OfxPropertySetHandle h, const char *n, int c, void **v) {
    if (!v || c < 0) return kOfxStatErrBadHandle;
    if (rv_lock()) return kOfxStatFailed;
    { OfxStatus s = rv_po_getn(PO(h), n, RV_PT_POINTER, c, NULL, NULL, NULL, v); rv_unlock(); return s; }
}
static OfxStatus pg_dbln(OfxPropertySetHandle h, const char *n, int c, double *v) {
    if (!v || c < 0) return kOfxStatErrBadHandle;
    if (rv_lock()) return kOfxStatFailed;
    { OfxStatus s = rv_po_getn(PO(h), n, RV_PT_DOUBLE, c, NULL, v, NULL, NULL); rv_unlock(); return s; }
}
static OfxStatus pg_strn(OfxPropertySetHandle h, const char *n, int c, char **v) {
    if (!v || c < 0) return kOfxStatErrBadHandle;
    if (rv_lock()) return kOfxStatFailed;
    { OfxStatus s = rv_po_getn(PO(h), n, RV_PT_STRING, c, NULL, NULL, v, NULL); rv_unlock(); return s; }
}
static OfxStatus pg_intn(OfxPropertySetHandle h, const char *n, int c, int *v) {
    if (!v || c < 0) return kOfxStatErrBadHandle;
    if (rv_lock()) return kOfxStatFailed;
    { OfxStatus s = rv_po_getn(PO(h), n, RV_PT_INT, c, v, NULL, NULL, NULL); rv_unlock(); return s; }
}

static OfxStatus p_reset(OfxPropertySetHandle h, const char *n) {
    if (rv_lock()) return kOfxStatFailed;
    { OfxStatus s = rv_po_reset(PO(h), n); rv_unlock(); return s; }
}
static OfxStatus p_dim(OfxPropertySetHandle h, const char *n, int *c) {
    if (rv_lock()) return kOfxStatFailed;
    { OfxStatus s = rv_po_dim(PO(h), n, c); rv_unlock(); return s; }
}

static OfxPropertySuiteV1 g_prop_suite = {
    .propSetPointer = ps_ptr,
    .propGetPointer = pg_ptr,
    .propSetPointerN = ps_ptrn,
    .propGetPointerN = pg_ptrn,
    .propSetString = ps_str,
    .propGetString = pg_str,
    .propSetStringN = ps_strn,
    .propGetStringN = pg_strn,
    .propSetDouble = ps_dbl,
    .propGetDouble = pg_dbl,
    .propSetDoubleN = ps_dbln,
    .propGetDoubleN = pg_dbln,
    .propSetInt = ps_int,
    .propGetInt = pg_int,
    .propSetIntN = ps_intn,
    .propGetIntN = pg_intn,
    .propReset = p_reset,
    .propGetDimension = p_dim,
};
const void *rv_prop_suite_ptr(void) { return &g_prop_suite; }

/* ---------- MemoryV1 ---------- */
static OfxStatus mem_alloc(void *handle, size_t n, void **out) {
    (void)handle;
    if (!g_active) return kOfxStatErrMissingHostFeature;
    if (rv_lock()) return kOfxStatFailed;
    { OfxStatus s = rv_mem_alloc(g_active, n, out); rv_unlock(); return s; }
}
static OfxStatus mem_free(void *p) {
    if (!g_active) return kOfxStatErrMissingHostFeature;
    if (rv_lock()) return kOfxStatFailed;
    { OfxStatus s = rv_mem_free(g_active, p); rv_unlock(); return s; }
}
static OfxMemorySuiteV1 g_mem_suite = {
    .memoryAlloc = mem_alloc,
    .memoryFree = mem_free,
};
const void *rv_mem_suite_ptr(void) { return &g_mem_suite; }
