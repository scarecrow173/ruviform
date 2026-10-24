/* ofx_params.c   ParameterV1 suite + domain validation + param mgmt. */
#include <stdlib.h>
#include <string.h>
#include <stdarg.h>
#include <math.h>
#include <float.h>
#include <limits.h>
#include "ofx_internal.h"

int rv_param_domain_ok(const RvParam *pr, int kind, double vd, int vi, int hw) {
    if (!pr || kind != pr->kind) return 0;
    if (hw && !pr->enabled) return 0;
    switch (kind) {
    case PK_DOUBLE:
        return isfinite(vd) && vd >= pr->mn && vd <= pr->mx;
    case PK_INT:
        return vi >= (int)pr->mn && vi <= (int)pr->mx;
    case PK_BOOL:
        return vi >= 0 && vi <= 1;
    case PK_CHOICE:
        return pr->nchoices > 0 && vi >= 0 && vi < pr->nchoices;
    default:
        return 0;
    }
}

RvParam *rv_param_find(RvEffect *e, const char *name) {
    size_t i;
    if (!e || !name) return NULL;
    for (i = 0; i < e->nparams; i++)
        if (e->params[i]->name && !strcmp(e->params[i]->name, name))
            return e->params[i];
    return NULL;
}

static void param_free(RvParam *p) {
    if (!p) return;
    rv_tok_revoke(p->tok);
    rv_tok_revoke(p->props_tok);
    /* props_tok may not be mirrored yet on early failure paths */
    if (p->props) rv_tok_revoke(p->props->tok);
    rv_po_free(p->props);
    free(p->choices); /* elements are pool strings */
    free(p);
}

void rv_params_free(RvParam **pp, size_t n, RvHost *h) {
    size_t i;
    (void)h;
    if (!pp) return;
    for (i = 0; i < n; i++) param_free(pp[i]);
    free(pp);
}

static const char *param_ptype_name(int kind) {
    switch (kind) {
    case PK_DOUBLE: return kOfxParamTypeDouble;
    case PK_INT:    return kOfxParamTypeInteger;
    case PK_BOOL:   return kOfxParamTypeBoolean;
    case PK_CHOICE: return kOfxParamTypeChoice;
    default:        return kOfxParamTypeString;
    }
}

RvParam **rv_params_clone(RvParam **src, size_t n, RvHost *h, uintptr_t parent) {
    RvParam **d;
    size_t i;
    if (!n) return NULL;
    d = (RvParam **)calloc(n, sizeof(RvParam *));
    if (!d) return NULL;
    for (i = 0; i < n; i++) {
        RvParam *s = src[i], *q;
        int c;
        q = (RvParam *)calloc(1, sizeof(RvParam));
        if (!q) { rv_params_free(d, i, h); return NULL; }
        *q = *s;
        q->name = rv_str_own(h, s->name ? s->name : "");
        q->tok = q->props_tok = 0;
        /* drop the aliased source pointer before any failure path can
         * param_free the clone */
        q->props = NULL;
        q->cur_d = s->ddef;
        q->cur_i = s->idef;
        q->choices = NULL;
        if (s->nchoices > 0) {
            q->choices = (char **)calloc((size_t)s->nchoices, sizeof(char *));
            if (!q->choices) { param_free(q); rv_params_free(d, i, h); return NULL; }
            for (c = 0; c < s->nchoices; c++)
                q->choices[c] = rv_str_own(h, s->choices[c] ? s->choices[c] : "");
        }
        /* an in-effect param always owns a props obj   rebuild it from the
         * described domain fields so CreateInstance-time suite calls see
         * the same schema/bounds the descriptor carried */
        q->props = rv_po_new(parent);
        if (!q->props) { param_free(q); rv_params_free(d, i, h); return NULL; }
        q->props_tok = q->props->tok;
        rv_po_set(q->props, kOfxParamPropType, RV_PT_STRING, 0, 0, 0,
                  param_ptype_name(q->kind), NULL);
        rv_seed_s(q->props, kOfxPropLabel, 1, q->name ? q->name : "");
        rv_seed_i(q->props, kOfxParamPropEnabled, 1, q->enabled);
        if (q->kind == PK_DOUBLE) {
            rv_seed_d(q->props, kOfxParamPropMin, 1, q->mn);
            rv_seed_d(q->props, kOfxParamPropMax, 1, q->mx);
            rv_seed_d(q->props, kOfxParamPropDisplayMin, 1, q->dmn);
            rv_seed_d(q->props, kOfxParamPropDisplayMax, 1, q->dmx);
            rv_seed_d(q->props, kOfxParamPropDefault, 1, q->ddef);
        } else if (q->kind != PK_STRING) {
            rv_seed_i(q->props, kOfxParamPropMin, 1, (int)q->mn);
            rv_seed_i(q->props, kOfxParamPropMax, 1, (int)q->mx);
            rv_seed_i(q->props, kOfxParamPropDefault, 1, q->idef);
        }
        for (c = 0; c < q->nchoices; c++)
            rv_po_set(q->props, kOfxParamPropChoiceOption, RV_PT_STRING, c,
                      0, 0, q->choices[c], NULL);
        d[i] = q;
    }
    return d;
}

/* pull SDK-written props into RvParam domain metadata after describe */
OfxStatus rv_params_read_meta(RvHost *h, RvParam *pr) {
    RvPropsObj *o;
    char *s = NULL;
    int iv = 0;
    if (!h || !pr) return kOfxStatErrBadHandle;
    o = pr->props;
    if (!o) return kOfxStatErrBadHandle;
    /* enabled */
    if (rv_po_get(o, kOfxParamPropEnabled, RV_PT_INT, 0, &iv, NULL, NULL, NULL) == kOfxStatOK)
        pr->enabled = iv;
    /* label -> flags */
    if (rv_po_get(o, kOfxPropLabel, RV_PT_STRING, 0, NULL, NULL, &s, NULL) == kOfxStatOK && s)
        (void)s;
    /* secret -> hidden flag bit */
    {
        int sv = 0;
        if (rv_po_get(o, kOfxParamPropSecret, RV_PT_INT, 0, &sv, NULL, NULL, NULL) == kOfxStatOK &&
            sv)
            pr->flags |= (1u << 3);
    }
    /* bounds per kind   unset bounds keep the OFX-wide domain */
    if (pr->kind == PK_DOUBLE) {
        double a = -DBL_MAX, b = DBL_MAX, c = -DBL_MAX, d = DBL_MAX;
        rv_po_get(o, kOfxParamPropMin, RV_PT_DOUBLE, 0, NULL, &a, NULL, NULL);
        rv_po_get(o, kOfxParamPropMax, RV_PT_DOUBLE, 0, NULL, &b, NULL, NULL);
        rv_po_get(o, kOfxParamPropDisplayMin, RV_PT_DOUBLE, 0, NULL, &c, NULL, NULL);
        rv_po_get(o, kOfxParamPropDisplayMax, RV_PT_DOUBLE, 0, NULL, &d, NULL, NULL);
        pr->mn = a; pr->mx = b; pr->dmn = c; pr->dmx = d;
        if (rv_po_get(o, kOfxParamPropDefault, RV_PT_DOUBLE, 0, NULL, &a, NULL, NULL) == kOfxStatOK)
            { pr->ddef = a; pr->cur_d = a; }
    } else if (pr->kind == PK_STRING) {
        char *sv = NULL;
        if (rv_po_get(o, kOfxParamPropDefault, RV_PT_STRING, 0, NULL, NULL, &sv, NULL) == kOfxStatOK)
            (void)sv;
    } else {
        int a = INT_MIN, b = INT_MAX, d = 0;
        rv_po_get(o, kOfxParamPropMin, RV_PT_INT, 0, &a, NULL, NULL, NULL);
        rv_po_get(o, kOfxParamPropMax, RV_PT_INT, 0, &b, NULL, NULL, NULL);
        pr->mn = a; pr->mx = b;
        if (rv_po_get(o, kOfxParamPropDefault, RV_PT_INT, 0, &d, NULL, NULL, NULL) == kOfxStatOK)
            { pr->idef = d; pr->cur_i = d; }
        if (pr->kind == PK_CHOICE) {
            char *sv;
            int dim = 0, k;
            if (rv_po_dim(o, kOfxParamPropChoiceOption, &dim) == kOfxStatOK && dim > 0) {
                pr->choices = (char **)calloc((size_t)dim, sizeof(char *));
                if (!pr->choices) return kOfxStatErrMemory;
                pr->nchoices = dim;
                for (k = 0; k < dim; k++) {
                    sv = NULL;
                    rv_po_get(o, kOfxParamPropChoiceOption, RV_PT_STRING, k, NULL, NULL, &sv, NULL);
                    pr->choices[k] = rv_str_own(h, sv ? sv : "");
                }
            }
        }
    }
    return kOfxStatOK;
}

/* ---------- ParamV1 ---------- */

static RvEffect *eff_of_pset(OfxParamSetHandle h) {
    return (RvEffect *)rv_tok_get((uintptr_t)h, H_PSET);
}
static RvParam *par_of(OfxParamHandle h) {
    return (RvParam *)rv_tok_get((uintptr_t)h, H_PARAM);
}

static OfxStatus p_define(OfxParamSetHandle pset, const char *ptype,
                          const char *name, OfxPropertySetHandle *props) {
    RvEffect *e;
    RvParam *pr;
    int kind;
    if (rv_lock()) return kOfxStatFailed;
    e = eff_of_pset(pset);
    if (!e || !ptype || !name) { rv_unlock(); return kOfxStatErrBadHandle; }
    if (rv_param_find(e, name)) { rv_unlock(); return kOfxStatErrExists; }
    if (!strcmp(ptype, kOfxParamTypeDouble)) kind = PK_DOUBLE;
    else if (!strcmp(ptype, kOfxParamTypeInteger)) kind = PK_INT;
    else if (!strcmp(ptype, kOfxParamTypeBoolean)) kind = PK_BOOL;
    else if (!strcmp(ptype, kOfxParamTypeChoice)) kind = PK_CHOICE;
    else if (!strcmp(ptype, kOfxParamTypeString)) kind = PK_STRING;
    else { rv_unlock(); return kOfxStatErrUnknown; }
    pr = (RvParam *)calloc(1, sizeof(RvParam));
    if (!pr) { rv_unlock(); return kOfxStatErrMemory; }
    pr->kind = kind;
    pr->enabled = 1;
    pr->name = rv_str_own(g_active, name);
    pr->props = rv_po_new(e->tok);
    if (!pr->name || !pr->props) { param_free(pr); rv_unlock(); return kOfxStatErrMemory; }
    /* seed schema: type string, label, enabled default */
    rv_po_set(pr->props, kOfxParamPropType, RV_PT_STRING, 0, 0, 0, ptype, NULL);
    rv_seed_s(pr->props, kOfxPropLabel, 1, name);
    rv_seed_i(pr->props, kOfxParamPropEnabled, 1, 1);
    pr->tok = rv_tok_new(H_PARAM, pr, e->tok);
    pr->props_tok = pr->props->tok;
    {
        RvParam **np = (RvParam **)realloc(e->params,
            sizeof(RvParam *) * (e->nparams + 1));
        if (!np) { param_free(pr); rv_unlock(); return kOfxStatErrMemory; }
        e->params = np;
        e->params[e->nparams++] = pr;
    }
    if (props) *props = (OfxPropertySetHandle)(uintptr_t)pr->props_tok;
    rv_unlock();
    return kOfxStatOK;
}

static OfxStatus p_getHandle(OfxParamSetHandle pset, const char *name,
                             OfxParamHandle *ph, OfxPropertySetHandle *props) {
    RvEffect *e;
    RvParam *pr;
    if (rv_lock()) return kOfxStatFailed;
    e = eff_of_pset(pset);
    if (!e || !name || !ph) { rv_unlock(); return kOfxStatErrBadHandle; }
    pr = rv_param_find(e, name);
    if (!pr) { rv_unlock(); return kOfxStatErrUnknown; }
    if (!pr->tok) pr->tok = rv_tok_new(H_PARAM, pr, e->tok);
    if (!pr->props_tok) pr->props_tok = pr->props->tok;
    *ph = (OfxParamHandle)(uintptr_t)pr->tok;
    if (props) *props = (OfxPropertySetHandle)(uintptr_t)pr->props_tok;
    rv_unlock();
    return kOfxStatOK;
}

static OfxStatus p_setProps(OfxParamSetHandle pset, OfxPropertySetHandle *props) {
    RvEffect *e;
    if (rv_lock()) return kOfxStatFailed;
    e = eff_of_pset(pset);
    if (!e || !props) { rv_unlock(); return kOfxStatErrBadHandle; }
    /* param-set props IS the effect props */
    *props = (OfxPropertySetHandle)(uintptr_t)e->props_tok;
    rv_unlock();
    return kOfxStatOK;
}

static OfxStatus p_getProps(OfxParamHandle param, OfxPropertySetHandle *props) {
    RvParam *pr;
    if (rv_lock()) return kOfxStatFailed;
    pr = par_of(param);
    if (!pr || !props) { rv_unlock(); return kOfxStatErrBadHandle; }
    if (!pr->props_tok) pr->props_tok = pr->props->tok;
    *props = (OfxPropertySetHandle)(uintptr_t)pr->props_tok;
    rv_unlock();
    return kOfxStatOK;
}

static OfxStatus p_getValue(OfxParamHandle param, ...) {
    RvParam *pr;
    va_list ap;
    OfxStatus st = kOfxStatOK;
    if (rv_lock()) return kOfxStatFailed;
    pr = par_of(param);
    if (!pr) { rv_unlock(); return kOfxStatErrBadHandle; }
    if (pr->kind == PK_STRING) { rv_unlock(); return kOfxStatErrUnsupported; }
    va_start(ap, param);
    if (pr->kind == PK_DOUBLE) { double *o = va_arg(ap, double *); if (o) *o = pr->cur_d; else st = kOfxStatErrBadHandle; }
    else { int *o = va_arg(ap, int *); if (o) *o = pr->cur_i; else st = kOfxStatErrBadHandle; }
    va_end(ap);
    rv_unlock();
    return st;
}

static OfxStatus p_getValueAtTime(OfxParamHandle param, OfxTime t, ...) {
    RvParam *pr;
    va_list ap;
    OfxStatus st = kOfxStatOK;
    (void)t;
    if (rv_lock()) return kOfxStatFailed;
    pr = par_of(param);
    if (!pr) { rv_unlock(); return kOfxStatErrBadHandle; }
    if (pr->kind == PK_STRING) { rv_unlock(); return kOfxStatErrUnsupported; }
    va_start(ap, t);
    if (pr->kind == PK_DOUBLE) { double *o = va_arg(ap, double *); if (o) *o = pr->cur_d; else st = kOfxStatErrBadHandle; }
    else { int *o = va_arg(ap, int *); if (o) *o = pr->cur_i; else st = kOfxStatErrBadHandle; }
    va_end(ap);
    rv_unlock();
    return st;
}

static OfxStatus p_getDerivative(OfxParamHandle p, OfxTime t, ...) {
    va_list ap;
    (void)t;
    if (rv_lock()) return kOfxStatFailed;
    if (!par_of(p)) { rv_unlock(); return kOfxStatErrBadHandle; }
    va_start(ap, t); va_end(ap);
    rv_unlock();
    return kOfxStatErrUnsupported;
}
static OfxStatus p_getIntegral(OfxParamHandle p, OfxTime a, OfxTime b, ...) {
    va_list ap;
    (void)a; (void)b;
    if (rv_lock()) return kOfxStatFailed;
    if (!par_of(p)) { rv_unlock(); return kOfxStatErrBadHandle; }
    va_start(ap, b); va_end(ap);
    rv_unlock();
    return kOfxStatErrUnsupported;
}

static OfxStatus p_setValue(OfxParamHandle param, ...) {
    RvParam *pr;
    va_list ap;
    OfxStatus st = kOfxStatOK;
    if (rv_lock()) return kOfxStatFailed;
    pr = par_of(param);
    if (!pr) { rv_unlock(); return kOfxStatErrBadHandle; }
    if (pr->kind == PK_STRING) { rv_unlock(); return kOfxStatErrUnsupported; }
    va_start(ap, param);
    if (pr->kind == PK_DOUBLE) {
        double v = va_arg(ap, double);
        if (!rv_param_domain_ok(pr, PK_DOUBLE, v, 0, 0)) st = kOfxStatErrValue;
        else pr->cur_d = v;
    } else {
        int v = va_arg(ap, int);
        if (!rv_param_domain_ok(pr, pr->kind, 0, v, 0)) st = kOfxStatErrValue;
        else pr->cur_i = v;
    }
    va_end(ap);
    rv_unlock();
    return st;
}

static OfxStatus p_setValueAtTime(OfxParamHandle p, OfxTime t, ...) {
    va_list ap;
    (void)t;
    if (rv_lock()) return kOfxStatFailed;
    if (!par_of(p)) { rv_unlock(); return kOfxStatErrBadHandle; }
    va_start(ap, t); va_end(ap);
    rv_unlock();
    return kOfxStatErrUnsupported;
}

static OfxStatus p_getNumKeys(OfxParamHandle p, unsigned int *n) {
    if (rv_lock()) return kOfxStatFailed;
    if (!par_of(p) || !n) { rv_unlock(); return kOfxStatErrBadHandle; }
    *n = 0;
    rv_unlock();
    return kOfxStatOK;
}
static OfxStatus p_getKeyTime(OfxParamHandle p, unsigned int k, double *t) {
    (void)k; (void)t;
    if (rv_lock()) return kOfxStatFailed;
    if (!par_of(p)) { rv_unlock(); return kOfxStatErrBadHandle; }
    rv_unlock();
    return kOfxStatErrBadIndex;
}
static OfxStatus p_getKeyIndex(OfxParamHandle p, OfxTime t, int dir, int *idx) {
    (void)t; (void)dir;
    if (rv_lock()) return kOfxStatFailed;
    if (!par_of(p) || !idx) { rv_unlock(); return kOfxStatErrBadHandle; }
    *idx = -1;
    rv_unlock();
    return kOfxStatFailed;
}
static OfxStatus p_deleteKey(OfxParamHandle p, OfxTime t) {
    (void)t;
    if (rv_lock()) return kOfxStatFailed;
    if (!par_of(p)) { rv_unlock(); return kOfxStatErrBadHandle; }
    rv_unlock();
    return kOfxStatErrBadIndex;
}
static OfxStatus p_deleteAllKeys(OfxParamHandle p) {
    if (rv_lock()) return kOfxStatFailed;
    if (!par_of(p)) { rv_unlock(); return kOfxStatErrBadHandle; }
    rv_unlock();
    return kOfxStatOK;
}
static OfxStatus p_copy(OfxParamHandle to, OfxParamHandle from,
                        OfxTime off, const OfxRangeD *r) {
    (void)off; (void)r;
    if (rv_lock()) return kOfxStatFailed;
    if (!par_of(to) || !par_of(from)) { rv_unlock(); return kOfxStatErrBadHandle; }
    rv_unlock();
    return kOfxStatErrUnsupported;
}
static OfxStatus p_editBegin(OfxParamSetHandle ps, const char *n) {
    (void)n;
    if (rv_lock()) return kOfxStatFailed;
    if (!eff_of_pset(ps)) { rv_unlock(); return kOfxStatErrBadHandle; }
    rv_unlock();
    return kOfxStatErrUnsupported;
}
static OfxStatus p_editEnd(OfxParamSetHandle ps) {
    if (rv_lock()) return kOfxStatFailed;
    if (!eff_of_pset(ps)) { rv_unlock(); return kOfxStatErrBadHandle; }
    rv_unlock();
    return kOfxStatErrUnsupported;
}

static OfxParameterSuiteV1 g_param_suite = {
    .paramDefine = p_define,
    .paramGetHandle = p_getHandle,
    .paramSetGetPropertySet = p_setProps,
    .paramGetPropertySet = p_getProps,
    .paramGetValue = p_getValue,
    .paramGetValueAtTime = p_getValueAtTime,
    .paramGetDerivative = p_getDerivative,
    .paramGetIntegral = p_getIntegral,
    .paramSetValue = p_setValue,
    .paramSetValueAtTime = p_setValueAtTime,
    .paramGetNumKeys = p_getNumKeys,
    .paramGetKeyTime = p_getKeyTime,
    .paramGetKeyIndex = p_getKeyIndex,
    .paramDeleteKey = p_deleteKey,
    .paramDeleteAllKeys = p_deleteAllKeys,
    .paramCopy = p_copy,
    .paramEditBegin = p_editBegin,
    .paramEditEnd = p_editEnd,
};
const void *rv_param_suite_ptr(void) { return &g_param_suite; }
