/* ofx_fixture_probes.c  - suite oracle probes for ruviform-plugin-ofx.
 * Fixture-side C probes calling REAL host suites via the stored OfxHost and
 * a live effect handle. 0 only when every check succeeds; nonzero = failing
 * line id. Honest failure  - no all-OK stubs. */
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#if defined(_WIN32)
#  include <windows.h>
#else
#  include <pthread.h>
#endif
#include "fixture_internal.h"

static const void *fetch(OfxHost *h, const char *name, int ver) {
    if (!h || !h->fetchSuite) return NULL;
    return h->fetchSuite(h->host, name, ver);
}

/* ---------- PROPERTY probe ---------- */

static int probe_property_run(OfxHost *host, OfxImageEffectHandle effect) {
    const OfxPropertySuiteV1 *p = (const OfxPropertySuiteV1 *)fetch(host, kOfxPropertySuite, 1);
    const OfxParameterSuiteV1 *pp = (const OfxParameterSuiteV1 *)fetch(host, kOfxParameterSuite, 1);
    const OfxImageEffectSuiteV1 *fx = fx_effects();
    OfxPropertySetHandle eps = NULL, pps = NULL, mps = NULL;
    OfxParamSetHandle pset = NULL;
    OfxParamHandle gain = NULL, mode = NULL;
    void *save_ptr = NULL, *tmp_ptr = NULL, *ptr_n[1] = {NULL};
    char *strv[2] = {NULL, NULL};
    const char *str_in[1];
    char *label_save = NULL;
    double dmin_save = 0.0, dmin2 = 0.0, dvals[1] = {0.0};
    int en_save = 0, en2 = 0, ivals[1] = {0}, dim = 0;

    if (!p || !pp || !fx || !effect) return 1001;
    /* unknown suite / wrong version -> NULL */
    if (fetch(host, "OfxNoSuchSuite", 1) != NULL) return 1002;
    if (fetch(host, kOfxPropertySuite, 999) != NULL) return 1003;

    if (fx->getPropertySet(effect, &eps) != kOfxStatOK || !eps) return 1004;
    if (fx->getParamSet(effect, &pset) != kOfxStatOK || !pset) return 1005;
    if (pp->paramGetHandle(pset, "gain", &gain, &pps) != kOfxStatOK || !gain || !pps) return 1006;
    if (pp->paramGetHandle(pset, "mode", &mode, &mps) != kOfxStatOK || !mode || !mps) return 1007;

    /* POINTER: InstanceData scalar + N count-1 roundtrip, restore */
    if (p->propGetPointer(eps, kOfxPropInstanceData, 0, &save_ptr) != kOfxStatOK) return 1010;
    tmp_ptr = (void *)(uintptr_t)0xABCD;
    if (p->propSetPointer(eps, kOfxPropInstanceData, 0, tmp_ptr) != kOfxStatOK) return 1011;
    ptr_n[0] = NULL;
    if (p->propGetPointer(eps, kOfxPropInstanceData, 0, ptr_n) != kOfxStatOK ||
        ptr_n[0] != tmp_ptr) return 1012;
    ptr_n[0] = save_ptr;
    if (p->propSetPointerN(eps, kOfxPropInstanceData, 1, ptr_n) != kOfxStatOK) return 1013;
    ptr_n[0] = NULL;
    if (p->propGetPointerN(eps, kOfxPropInstanceData, 1, ptr_n) != kOfxStatOK ||
        ptr_n[0] != save_ptr) return 1014;
    ptr_n[0] = NULL;
    if (p->propGetPointer(eps, kOfxPropInstanceData, 0, ptr_n) != kOfxStatOK ||
        ptr_n[0] != save_ptr) return 1015;

    /* STRING: param label save/SetN/GetN/scalar/restore */
    if (p->propGetString(pps, kOfxPropLabel, 0, strv) != kOfxStatOK || !strv[0]) return 1020;
    label_save = (char *)malloc(strlen(strv[0]) + 1);
    if (!label_save) return 1021;
    strcpy(label_save, strv[0]);
    str_in[0] = "fixture-label";
    if (p->propSetStringN(pps, kOfxPropLabel, 1, str_in) != kOfxStatOK) return 1022;
    strv[0] = NULL;
    if (p->propGetStringN(pps, kOfxPropLabel, 1, strv) != kOfxStatOK ||
        !strv[0] || strcmp(strv[0], "fixture-label") != 0) return 1023;
    strv[0] = NULL;
    if (p->propGetString(pps, kOfxPropLabel, 0, strv) != kOfxStatOK ||
        !strv[0] || strcmp(strv[0], "fixture-label") != 0) return 1024;
    str_in[0] = label_save;
    if (p->propSetStringN(pps, kOfxPropLabel, 1, str_in) != kOfxStatOK) return 1025;
    free(label_save);

    /* DOUBLE: gain DisplayMin save/SetN/GetN/scalar/restore */
    if (p->propGetDouble(pps, kOfxParamPropDisplayMin, 0, &dmin_save) != kOfxStatOK) return 1030;
    dvals[0] = dmin_save + 0.5;
    if (p->propSetDoubleN(pps, kOfxParamPropDisplayMin, 1, dvals) != kOfxStatOK) return 1031;
    dvals[0] = 0.0;
    if (p->propGetDoubleN(pps, kOfxParamPropDisplayMin, 1, dvals) != kOfxStatOK ||
        dvals[0] != dmin_save + 0.5) return 1032;
    if (p->propGetDouble(pps, kOfxParamPropDisplayMin, 0, &dmin2) != kOfxStatOK ||
        dmin2 != dmin_save + 0.5) return 1033;
    if (p->propSetDouble(pps, kOfxParamPropDisplayMin, 0, dmin_save) != kOfxStatOK) return 1034;

    /* INT: gain Enabled save -> SetIntN(0) -> scalar check -> reset -> GetN==1 -> restore */
    if (p->propGetInt(pps, kOfxParamPropEnabled, 0, &en_save) != kOfxStatOK) return 1040;
    ivals[0] = 0;
    if (p->propSetIntN(pps, kOfxParamPropEnabled, 1, ivals) != kOfxStatOK) return 1041;
    if (p->propGetInt(pps, kOfxParamPropEnabled, 0, &en2) != kOfxStatOK || en2 != 0) return 1042;
    if (p->propReset(pps, kOfxParamPropEnabled) != kOfxStatOK) return 1043;
    ivals[0] = -7;
    if (p->propGetIntN(pps, kOfxParamPropEnabled, 1, ivals) != kOfxStatOK || ivals[0] != 1) return 1044;
    if (p->propSetInt(pps, kOfxParamPropEnabled, 0, en_save) != kOfxStatOK) return 1045;

    /* DIMENSION: mode ChoiceOption dim 2, readN labels a/b */
    if (p->propGetDimension(mps, kOfxParamPropChoiceOption, &dim) != kOfxStatOK || dim != 2) return 1050;
    strv[0] = strv[1] = NULL;
    if (p->propGetStringN(mps, kOfxParamPropChoiceOption, 2, strv) != kOfxStatOK ||
        !strv[0] || !strv[1] || strcmp(strv[0], "a") != 0 || strcmp(strv[1], "b") != 0) return 1051;

    /* error oracles: unknown prop ErrUnknown, bad index, wrong type, bad handle */
    strv[0] = NULL;
    if (p->propGetString(pps, "ruviform.no.such.prop", 0, strv) != kOfxStatErrUnknown) return 1060;
    if (p->propGetInt(pps, kOfxParamPropEnabled, -1, &en2) != kOfxStatErrBadIndex) return 1061;
    if (p->propGetInt(pps, kOfxPropLabel, 0, &en2) == kOfxStatOK) return 1062;
    if (p->propGetInt((OfxPropertySetHandle)(uintptr_t)1, kOfxParamPropEnabled, 0, &en2)
        != kOfxStatErrBadHandle) return 1063;
    return 0;
}

/* ---------- PARAM probe ---------- */

static int probe_param_run(OfxHost *host, OfxImageEffectHandle effect) {
    const OfxParameterSuiteV1 *pp = (const OfxParameterSuiteV1 *)fetch(host, kOfxParameterSuite, 1);
    const OfxPropertySuiteV1 *p = (const OfxPropertySuiteV1 *)fetch(host, kOfxPropertySuite, 1);
    const OfxImageEffectSuiteV1 *fx = fx_effects();
    const OfxParamHandle bad = (OfxParamHandle)(uintptr_t)1;
    const OfxParamSetHandle bad_ps = (OfxParamSetHandle)(uintptr_t)1;
    OfxParamSetHandle pset = NULL;
    OfxPropertySetHandle eps = NULL, pset_ps = NULL, gps = NULL;
    OfxParamHandle gain = NULL, count = NULL, enabled = NULL, mode = NULL;
    double d = 0.0, d_at = 0.0, d_save = 0.0;
    int i = 0, i_save = 0, key_index = -2;
    unsigned int num_keys = ~0u;
    double key_time = 0.0;
    char *ptype = NULL;
    OfxRangeD range = {0.0, 0.0};

    if (!pp || !p || !fx || !effect) return 2001;

    /* all 18 V1 slots non-null */
    if (!pp->paramDefine || !pp->paramGetHandle || !pp->paramSetGetPropertySet ||
        !pp->paramGetPropertySet || !pp->paramGetValue || !pp->paramGetValueAtTime ||
        !pp->paramGetDerivative || !pp->paramGetIntegral || !pp->paramSetValue ||
        !pp->paramSetValueAtTime || !pp->paramGetNumKeys || !pp->paramGetKeyTime ||
        !pp->paramGetKeyIndex || !pp->paramDeleteKey || !pp->paramDeleteAllKeys ||
        !pp->paramCopy || !pp->paramEditBegin || !pp->paramEditEnd) return 2002;

    if (fx->getParamSet(effect, &pset) != kOfxStatOK || !pset) return 2003;
    /* paramSet propset must equal the effect propset */
    if (fx->getPropertySet(effect, &eps) != kOfxStatOK || !eps) return 2004;
    if (pp->paramSetGetPropertySet(pset, &pset_ps) != kOfxStatOK || pset_ps != eps) return 2005;

    if (pp->paramGetHandle(pset, "gain", &gain, &gps) != kOfxStatOK || !gain || !gps) return 2006;
    if (pp->paramGetHandle(pset, "count", &count, NULL) != kOfxStatOK || !count) return 2007;
    if (pp->paramGetHandle(pset, "enabled", &enabled, NULL) != kOfxStatOK || !enabled) return 2008;
    if (pp->paramGetHandle(pset, "mode", &mode, NULL) != kOfxStatOK || !mode) return 2009;

    /* kind check via param property set type */
    if (pp->paramGetPropertySet(gain, &gps) != kOfxStatOK || !gps) return 2010;
    if (p->propGetString(gps, kOfxParamPropType, 0, &ptype) != kOfxStatOK ||
        !ptype || strcmp(ptype, kOfxParamTypeDouble) != 0) return 2011;

    /* Double: get + getAtTime same; set/get/restore */
    if (pp->paramGetValue(gain, &d_save) != kOfxStatOK) return 2020;
    if (pp->paramGetValueAtTime(gain, 0.0, &d_at) != kOfxStatOK || d_at != d_save) return 2021;
    if (pp->paramSetValue(gain, 1.5) != kOfxStatOK) return 2022;
    if (pp->paramGetValue(gain, &d) != kOfxStatOK || d != 1.5) return 2023;
    if (pp->paramSetValue(gain, d_save) != kOfxStatOK) return 2024;

    /* Integer: get/atTime/set/get/restore */
    if (pp->paramGetValue(count, &i_save) != kOfxStatOK) return 2030;
    if (pp->paramGetValueAtTime(count, 0.0, &i) != kOfxStatOK || i != i_save) return 2031;
    if (pp->paramSetValue(count, 7) != kOfxStatOK) return 2032;
    if (pp->paramGetValue(count, &i) != kOfxStatOK || i != 7) return 2033;
    if (pp->paramSetValue(count, i_save) != kOfxStatOK) return 2034;

    /* Bool: get/atTime/set/get/restore */
    if (pp->paramGetValue(enabled, &i_save) != kOfxStatOK) return 2040;
    if (pp->paramGetValueAtTime(enabled, 0.0, &i) != kOfxStatOK || i != i_save) return 2041;
    if (pp->paramSetValue(enabled, 0) != kOfxStatOK) return 2042;
    if (pp->paramGetValue(enabled, &i) != kOfxStatOK || i != 0) return 2043;
    if (pp->paramSetValue(enabled, i_save) != kOfxStatOK) return 2044;

    /* Choice: int* varargs  - set 0, get 0, atTime 0, restore saved */
    if (pp->paramGetValue(mode, &i_save) != kOfxStatOK) return 2050;
    if (pp->paramSetValue(mode, 0) != kOfxStatOK) return 2051;
    if (pp->paramGetValue(mode, &i) != kOfxStatOK || i != 0) return 2052;
    if (pp->paramGetValueAtTime(mode, 0.0, &i) != kOfxStatOK || i != 0) return 2053;
    if (pp->paramSetValue(mode, i_save) != kOfxStatOK) return 2054;

    /* key oracles */
    if (pp->paramGetNumKeys(gain, &num_keys) != kOfxStatOK || num_keys != 0) return 2060;
    if (pp->paramGetKeyTime(gain, 0, &key_time) != kOfxStatErrBadIndex) return 2061;
    if (pp->paramDeleteKey(gain, 0) != kOfxStatErrBadIndex) return 2062;
    if (pp->paramGetKeyIndex(gain, 0.0, 0, &key_index) != kOfxStatFailed || key_index != -1) return 2063;
    if (pp->paramDeleteAllKeys(gain) != kOfxStatOK) return 2064;

    /* unsupported slots: exact kOfxStatErrUnsupported + gain unchanged each time */
    if (pp->paramSetValueAtTime(gain, 0.0, 1.0) != kOfxStatErrUnsupported) return 2070;
    if (pp->paramGetValue(gain, &d) != kOfxStatOK || d != d_save) return 2071;
    if (pp->paramGetDerivative(gain, 0.0, &d) != kOfxStatErrUnsupported) return 2072;
    if (pp->paramGetValue(gain, &d) != kOfxStatOK || d != d_save) return 2073;
    if (pp->paramGetIntegral(gain, 0.0, 1.0, &d) != kOfxStatErrUnsupported) return 2074;
    if (pp->paramGetValue(gain, &d) != kOfxStatOK || d != d_save) return 2075;
    if (pp->paramCopy(gain, gain, 0.0, &range) != kOfxStatErrUnsupported) return 2076;
    if (pp->paramGetValue(gain, &d) != kOfxStatOK || d != d_save) return 2077;
    if (pp->paramEditBegin(pset, "gain") != kOfxStatErrUnsupported) return 2078;
    if (pp->paramGetValue(gain, &d) != kOfxStatOK || d != d_save) return 2079;
    if (pp->paramEditEnd(pset) != kOfxStatErrUnsupported) return 2080;
    if (pp->paramGetValue(gain, &d) != kOfxStatOK || d != d_save) return 2081;

    /* BadHandle-first: unknown token AND wrong-kind on the unsupported slots */
    if (pp->paramSetValueAtTime(bad, 0.0, 1.0) != kOfxStatErrBadHandle) return 2090;
    if (pp->paramGetDerivative(bad, 0.0, &d) != kOfxStatErrBadHandle) return 2091;
    if (pp->paramGetIntegral(bad, 0.0, 1.0, &d) != kOfxStatErrBadHandle) return 2092;
    if (pp->paramCopy(bad, gain, 0.0, &range) != kOfxStatErrBadHandle) return 2093;
    if (pp->paramCopy(gain, bad, 0.0, &range) != kOfxStatErrBadHandle) return 2094;
    if (pp->paramEditBegin(bad_ps, "gain") != kOfxStatErrBadHandle) return 2095;
    if (pp->paramEditEnd(bad_ps) != kOfxStatErrBadHandle) return 2096;
    if (pp->paramSetValueAtTime((OfxParamHandle)effect, 0.0, 1.0) != kOfxStatErrBadHandle) return 2097;
    if (pp->paramEditBegin((OfxParamSetHandle)gain, "x") != kOfxStatErrBadHandle) return 2098;
    /* plain get on bad handle also checked */
    if (pp->paramGetValue(bad, &d) != kOfxStatErrBadHandle) return 2099;
    return 0;
}

/* ---------- MEMORY probe ---------- */

static int probe_memory_run(OfxHost *host, OfxImageEffectHandle effect) {
    const OfxMemorySuiteV1 *m = (const OfxMemorySuiteV1 *)fetch(host, kOfxMemorySuite, 1);
    const OfxImageEffectSuiteV1 *fx = fx_effects();
    void *ptr = NULL, *huge = NULL, *img_locked = NULL;
    OfxImageMemoryHandle img = NULL;

    if (!m || !fx || !effect) return 3001;
    if (!m->memoryAlloc || !m->memoryFree) return 3002;
    if (!fx->imageMemoryAlloc || !fx->imageMemoryLock ||
        !fx->imageMemoryUnlock || !fx->imageMemoryFree) return 3003;

    if (m->memoryAlloc(NULL, 64, &ptr) != kOfxStatOK || !ptr) return 3010;
    if (((uintptr_t)ptr & 15) != 0) return 3011;
    memset(ptr, 0xAB, 64);
    if (m->memoryFree(ptr) != kOfxStatOK) return 3012;

    if (m->memoryAlloc(NULL, SIZE_MAX, &huge) != kOfxStatErrMemory || huge != NULL) return 3013;
    if (m->memoryFree((void *)(uintptr_t)1) != kOfxStatErrBadHandle) return 3014;

    if (fx->imageMemoryAlloc(effect, 64, &img) != kOfxStatOK || !img) return 3020;
    if (fx->imageMemoryLock(img, &img_locked) != kOfxStatOK || !img_locked) return 3021;
    if (((uintptr_t)img_locked & 15) != 0) return 3022;
    memset(img_locked, 0xCD, 64);
    if (fx->imageMemoryUnlock(img) != kOfxStatOK) return 3023;
    if (fx->imageMemoryFree(img) != kOfxStatOK) return 3024;

    /* stale handle after free -> BadHandle, no deref of opaque */
    img_locked = (void *)(uintptr_t)0xDEAD;
    if (fx->imageMemoryLock(img, &img_locked) != kOfxStatErrBadHandle || img_locked != NULL) return 3025;
    if (fx->imageMemoryUnlock((OfxImageMemoryHandle)(uintptr_t)1) != kOfxStatErrBadHandle) return 3026;
    img = NULL;
    if (fx->imageMemoryAlloc(effect, (size_t)-1, &img) != kOfxStatErrMemory || img != NULL) return 3027;
    return 0;
}

/* ---------- READONLY probe (native set legal; host UI set rejected in Rust) ---------- */

static int probe_readonly_run(OfxHost *host, OfxImageEffectHandle effect) {
    const OfxParameterSuiteV1 *pp = (const OfxParameterSuiteV1 *)fetch(host, kOfxParameterSuite, 1);
    const OfxImageEffectSuiteV1 *fx = fx_effects();
    OfxParamSetHandle pset = NULL;
    OfxParamHandle outv = NULL;
    double d = 0.0;

    if (!pp || !fx || !effect) return 4001;
    if (fx->getParamSet(effect, &pset) != kOfxStatOK || !pset) return 4002;
    if (pp->paramGetHandle(pset, "outv", &outv, NULL) != kOfxStatOK || !outv) return 4003;
    /* native C SetValue on readonly param is legal  - intentionally leaves 4 */
    if (pp->paramSetValue(outv, 4.0) != kOfxStatOK) return 4004;
    if (pp->paramGetValue(outv, &d) != kOfxStatOK || d != 4.0) return 4005;
    return 0;
}

/* ---------- WORKER probe: same-effect suite calls on a joined OS thread ---------- */

typedef struct {
    OfxHost *host;
    OfxImageEffectHandle effect;
    int result;
} WorkerCtx;

static void worker_body(WorkerCtx *c) {
    c->result = probe_property_run(c->host, c->effect);
    if (c->result == 0) c->result = probe_param_run(c->host, c->effect);
    if (c->result == 0) c->result = probe_memory_run(c->host, c->effect);
}

#if defined(_WIN32)
static DWORD WINAPI worker_fn(LPVOID arg) { worker_body((WorkerCtx *)arg); return 0; }
static int run_worker(OfxHost *host, OfxImageEffectHandle effect) {
    WorkerCtx ctx = { host, effect, 0 };
    DWORD rc;
    HANDLE th = CreateThread(NULL, 0, worker_fn, &ctx, 0, NULL);
    if (!th) return 5001;
    /* must join before returning  - ctx must not outlive this frame */
    rc = WaitForSingleObject(th, INFINITE);
    CloseHandle(th);
    if (rc != WAIT_OBJECT_0) abort(); /* cannot return with live stack ctx */
    return ctx.result;
}
#else
static void *worker_fn(void *arg) { worker_body((WorkerCtx *)arg); return NULL; }
static int run_worker(OfxHost *host, OfxImageEffectHandle effect) {
    WorkerCtx ctx = { host, effect, 0 };
    pthread_t th;
    if (pthread_create(&th, NULL, worker_fn, &ctx) != 0) return 5001;
    if (pthread_join(th, NULL) != 0) abort(); /* cannot return with live stack ctx */
    return ctx.result;
}
#endif

/* ---------- CLIP PROPS probe: instance clip set mirrors descriptor ---------- */

static int probe_clip_props_run(OfxHost *host, OfxImageEffectHandle effect) {
    const OfxPropertySuiteV1 *p = (const OfxPropertySuiteV1 *)fetch(host, kOfxPropertySuite, 1);
    const OfxImageEffectSuiteV1 *fx = fx_effects();
    OfxImageClipHandle src_clip = NULL, dst_clip = NULL;
    OfxPropertySetHandle cps = NULL;
    char *s = NULL;
    if (!p || !fx || !effect) return 6001;
    /* the instance clip property set must mirror the descriptor-declared
     * contract the fixture wrote during DescribeInContext (Issue #124) */
    if (fx->clipGetHandle(effect, "Source", &src_clip, &cps) != kOfxStatOK ||
        !src_clip || !cps) return 6002;
    if (p->propGetString(cps, kOfxImageEffectPropSupportedComponents, 0, &s) != kOfxStatOK ||
        !s || strcmp(s, kOfxImageComponentRGBA) != 0) return 6003;
    s = NULL;
    cps = NULL;
    if (fx->clipGetHandle(effect, "Output", &dst_clip, &cps) != kOfxStatOK ||
        !dst_clip || !cps) return 6004;
    if (p->propGetString(cps, kOfxImageEffectPropSupportedComponents, 0, &s) != kOfxStatOK ||
        !s || strcmp(s, kOfxImageComponentRGBA) != 0) return 6005;
    /* clipGetPropertySet resolves the same mirrored set on the clip handle */
    s = NULL;
    cps = NULL;
    if (fx->clipGetPropertySet(src_clip, &cps) != kOfxStatOK || !cps) return 6006;
    if (p->propGetString(cps, kOfxImageEffectPropSupportedComponents, 0, &s) != kOfxStatOK ||
        !s || strcmp(s, kOfxImageComponentRGBA) != 0) return 6007;
    return 0;
}

/* ---------- dispatch ---------- */

int ofx_fixture_probe(int which) {
    OfxHost *host = fx_current_host();
    FixtureInstance *inst = fx_last_instance();
    OfxImageEffectHandle effect = inst ? inst->effect : NULL;
    switch (which) {
    case FX_PROBE_PROPERTY: return probe_property_run(host, effect);
    case FX_PROBE_PARAM:    return probe_param_run(host, effect);
    case FX_PROBE_MEMORY:   return probe_memory_run(host, effect);
    case FX_PROBE_WORKER:   return run_worker(host, effect);
    case FX_PROBE_READONLY: return probe_readonly_run(host, effect);
    case FX_PROBE_CLIP_PROPS: return probe_clip_props_run(host, effect);
    default: return 9000;
    }
}
