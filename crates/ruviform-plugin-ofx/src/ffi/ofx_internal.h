/* ofx_internal.h   private shared state between bridge translation units.
 * All SDK-facing handles are generation tokens (uintptr >= 0x10000) cast to
 * the opaque pointer types   never dereferenced as SDK structs. Registry
 * nodes carry owner + parent for cascade revocation. A single recursive
 * global mutex protects all registry/property/param/memory state; it is
 * NEVER held across a foreign call (setHost/mainEntry). */
#ifndef RUVIFORM_OFX_INTERNAL_H
#define RUVIFORM_OFX_INTERNAL_H

#include "ofx_bridge.h"
#include "ofxCore.h"
#include "ofxImageEffect.h"
#include "ofxParam.h"
#include "ofxProperty.h"
#include "ofxMemory.h"

#if defined(_WIN32)
#  include <windows.h>
#else
#  include <pthread.h>
#endif

int rv_lock(void);      /* init-once + lock; 0 ok, nonzero init failure */
void rv_unlock(void);

enum { H_EFFECT, H_PSET, H_PARAM, H_CLIP, H_EPROPS, H_PPROPS, H_CPROPS,
       H_INARGS, H_OUTARGS, H_ENTRY, H_MEMORY, H_IMAGE };

uintptr_t rv_tok_new(int kind, void *payload, uintptr_t parent);
void *rv_tok_get(uintptr_t tok, int kind); /* NULL on stale/wrong kind */
void rv_tok_revoke(uintptr_t tok);         /* cascades to children */

/* ---------- property storage ---------- */
enum { RV_PT_INT = 1, RV_PT_DOUBLE, RV_PT_STRING, RV_PT_POINTER };
typedef struct RvProp {
    char *name;
    int type, dim, cap;
    int seeded;              /* has default snapshot for reset */
    int *vi; double *vd; char **vs; void **vp;
    int *di; double *dd; char **ds; void **dp;
    int ddim;
    struct RvProp *next;
} RvProp;
typedef struct RvPropsObj { RvProp *head; uintptr_t tok; } RvPropsObj;

RvPropsObj *rv_po_new(uintptr_t parent);
void rv_po_free(RvPropsObj *o);
OfxStatus rv_po_set(RvPropsObj *o, const char *n, int t, int i,
                    int vi, double vd, const char *vs, void *vp);
OfxStatus rv_po_setn(RvPropsObj *o, const char *n, int t, int c,
                     const int *vi, const double *vd,
                     const char *const *vs, void *const *vp);
OfxStatus rv_po_get(RvPropsObj *o, const char *n, int t, int i,
                    int *vi, double *vd, char **vs, void **vp);
OfxStatus rv_po_getn(RvPropsObj *o, const char *n, int t, int c,
                     int *vi, double *vd, char **vs, void **vp);
OfxStatus rv_po_dim(RvPropsObj *o, const char *n, int *dim);
OfxStatus rv_po_reset(RvPropsObj *o, const char *n);
/* seed helpers   explicit schema, no autocreated defaults */
void rv_seed_i(RvPropsObj *o, const char *n, int dim, int v);
void rv_seed_d(RvPropsObj *o, const char *n, int dim, double v);
void rv_seed_s(RvPropsObj *o, const char *n, int dim, const char *v);
void rv_seed_p(RvPropsObj *o, const char *n, int dim, void *v);

/* ---------- host ---------- */
typedef struct RvStrNode { char *s; struct RvStrNode *next; } RvStrNode;
typedef struct RvMemNode { void *p; struct RvMemNode *next; } RvMemNode;

typedef struct RvHost {
    OfxHost api;
    int alive;
    struct RvCatalog *catalogs;
    RvPropsObj *props;
    RvStrNode *strings;
    RvMemNode *mem;
} RvHost;

extern RvHost *g_active;

char *rv_str_own(RvHost *h, const char *s);      /* pool-owned stable copy */
OfxStatus rv_mem_alloc(RvHost *h, size_t n, void **out);
OfxStatus rv_mem_free(RvHost *h, void *ptr);

const void *rv_fetch_suite(void *self, const char *name, int ver);
const void *rv_prop_suite_ptr(void);
const void *rv_param_suite_ptr(void);
const void *rv_mem_suite_ptr(void);
const void *rv_fx_suite_ptr(void);

/* ---------- params / clips / effects ---------- */
enum { PK_DOUBLE, PK_INT, PK_BOOL, PK_CHOICE, PK_STRING /*unsupported*/ };
typedef struct RvParam {
    int kind;
    char *name;
    int enabled;
    unsigned int flags;
    double mn, mx, dmn, dmx, ddef, cur_d;
    int idef, cur_i;
    char **choices;
    int nchoices;
    RvPropsObj *props;
    uintptr_t tok, props_tok;
} RvParam;

struct RvEffect;
typedef struct RvClip {
    char *name;
    RvPropsObj *props;
    uintptr_t tok, props_tok;
    struct RvEffect *eff;         /* owning effect (descriptor or attempt) */
    struct RvClip *next;
} RvClip;

/* one live image handle handed to the plugin during a render bracket;
 * image props are an ordinary RvPropsObj resolved through the token */
typedef struct RvImg {
    RvClip *clip;
    RvPropsObj *props;
    uintptr_t tok;
    struct RvImg *next;
} RvImg;

/* one type serves the frozen admission descriptor AND live attempts */
typedef struct RvEffect {
    int is_instance;             /* 0 descriptor, 1 attempt */
    uintptr_t tok, props_tok, pset_tok;
    RvPropsObj *props;           /* effect property set */
    RvParam **params;            /* heap nodes   stable for tokens */
    size_t nparams, cap_params;
    RvClip *clips;
    /* admission entry fields */
    int native_index;
    char *identifier, *label, *vendor, *version;
    int single_instance;
    OfxPlugin *raw;
    int loaded;                  /* successful native Load owed an Unload */
    int rejected, rejection_status;
    int live;                    /* live or uncertain attempts holding admission */
    struct RvCatalog *cat;       /* owning catalog (descriptors only) */
    /* attempt fields */
    struct RvEffect *next;       /* catalog entry list link */
    struct RvEffect *desc;       /* descriptor this attempt cloned from */
    int created, create_attempted, destroyed, quarantined, configured;
    uint32_t w, h;
    double fps;
    /* per-render staging: adapter-owned 16B aligned buffers exposed
     * bottom-left to the plugin for the bracket duration only */
    unsigned char *stage_src, *stage_dst;
    size_t stage_stride;
    int render_active;
    RvImg *images;               /* images handed out, still unreleased */
    struct RvEffect *next_quar;  /* quarantine list on catalog */
} RvEffect;

typedef struct RvCatalog {
    RvHost *host;
    RvEffect *entries;           /* linked list, scan order */
    RvEffect *quar;              /* failed-destroy instances until Unload */
    struct RvCatalog *next;
} RvCatalog;

/* params.c */
int rv_param_domain_ok(const RvParam *pr, int kind, double vd, int vi, int hw);
RvParam *rv_param_find(RvEffect *e, const char *name);
void rv_params_free(RvParam **pp, size_t n, RvHost *h);
RvParam **rv_params_clone(RvParam **src, size_t n, RvHost *h, uintptr_t parent);
OfxStatus rv_params_read_meta(RvHost *h, RvParam *pr); /* props -> fields */

#endif
