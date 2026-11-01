/* ofx_test_fixture.c  - real C test plugin, ruviform-plugin-ofx (Issue #98).
 * Plugin side only; host stays Unsupported until A7. Implements the exact
 * fixture_control.h interface: observer events, mode bits, real SDK
 * actions with checked statuses, per-instance data via kOfxPropInstanceData. */
#include <stdlib.h>
#include <string.h>
#include <stddef.h>
#include <float.h>
#include "fixture_internal.h"

static OfxHost *g_host = NULL;
static int g_mode = 0;
static FixtureObserver g_observer = NULL;
static FixtureInstance *g_instances = NULL;
static double g_created_gain = 0.0;
static double g_render_time = 0.0;
static int g_src_rowbytes = 0, g_dst_rowbytes = 0;
static unsigned char g_src_corners[16];

static void emit(int event, int native_index) {
    if (g_observer) g_observer(event, native_index);
}

/* ---- control API ---- */

void ofx_fixture_configure(int mode, FixtureObserver observer) {
    /* fresh-DLL reset: only legal before host load with no live instances */
    if (fx_live_count() != 0) return;
    g_mode = mode;
    g_observer = observer;
    g_created_gain = 0.0;
    g_host = NULL;
}

void ofx_fixture_set_mode(int mode) { g_mode = mode; }

/* ---- internal accessors (fixture_internal.h) ---- */

OfxHost *fx_current_host(void) { return g_host; }

const OfxPropertySuiteV1 *fx_props(void) {
    if (!g_host || !g_host->fetchSuite) return NULL;
    return (const OfxPropertySuiteV1 *)g_host->fetchSuite(g_host->host, kOfxPropertySuite, 1);
}
const OfxParameterSuiteV1 *fx_params(void) {
    if (!g_host || !g_host->fetchSuite) return NULL;
    return (const OfxParameterSuiteV1 *)g_host->fetchSuite(g_host->host, kOfxParameterSuite, 1);
}
const OfxImageEffectSuiteV1 *fx_effects(void) {
    if (!g_host || !g_host->fetchSuite) return NULL;
    return (const OfxImageEffectSuiteV1 *)g_host->fetchSuite(g_host->host, kOfxImageEffectSuite, 1);
}
const OfxMemorySuiteV1 *fx_memory(void) {
    if (!g_host || !g_host->fetchSuite) return NULL;
    return (const OfxMemorySuiteV1 *)g_host->fetchSuite(g_host->host, kOfxMemorySuite, 1);
}
FixtureInstance *fx_last_instance(void) { return g_instances; }
FixtureInstance *fx_find_instance(OfxImageEffectHandle effect) {
    FixtureInstance *i;
    for (i = g_instances; i; i = i->next)
        if (i->effect == effect) return i;
    return NULL;
}
int fx_live_count(void) {
    int n = 0; FixtureInstance *i;
    for (i = g_instances; i; i = i->next) n++;
    return n;
}
double fx_created_gain_value(void) { return g_created_gain; }

/* ---- actions ---- */

static OfxStatus describe(OfxImageEffectHandle effect, const char *ctx, int idx) {
    const OfxPropertySuiteV1 *p = fx_props();
    const OfxImageEffectSuiteV1 *fx = fx_effects();
    OfxPropertySetHandle set = NULL;
    if (!p || !fx) return kOfxStatErrMissingHostFeature;
    if (fx->getPropertySet(effect, &set) != kOfxStatOK || !set) return kOfxStatFailed;
    emit(FX_DESCRIBE, idx);
    if (p->propSetString(set, kOfxPropLabel, 0, "Fixture Gain") != kOfxStatOK) return kOfxStatFailed;
    if (p->propSetString(set, kOfxImageEffectPluginPropGrouping, 0, "RuviformTests") != kOfxStatOK) return kOfxStatFailed;
    if (p->propSetString(set, kOfxImageEffectPropSupportedContexts, 0, ctx) != kOfxStatOK) return kOfxStatFailed;
    if (p->propSetString(set, kOfxImageEffectPropSupportedPixelDepths, 0, kOfxBitDepthByte) != kOfxStatOK) return kOfxStatFailed;
    if (p->propSetInt(set, kOfxImageEffectPluginPropSingleInstance, 0,
                      (g_mode & FX_SINGLE_INSTANCE) ? 1 : 0) != kOfxStatOK) return kOfxStatFailed;
    return kOfxStatOK;
}

static OfxStatus describe_in_context(OfxImageEffectHandle effect,
                                     OfxPropertySetHandle in_args,
                                     const char *want_ctx, int idx) {
    const OfxPropertySuiteV1 *p = fx_props();
    const OfxParameterSuiteV1 *pp = fx_params();
    const OfxImageEffectSuiteV1 *fx = fx_effects();
    OfxParamSetHandle pset = NULL;
    OfxPropertySetHandle clip = NULL, prm = NULL;
    char *ctx = NULL;
    if (!p || !pp || !fx) return kOfxStatErrMissingHostFeature;
    emit(FX_DESCRIBE_CONTEXT, idx);
    if (!in_args ||
        p->propGetString(in_args, kOfxImageEffectPropContext, 0, &ctx) != kOfxStatOK ||
        !ctx || strcmp(ctx, want_ctx) != 0)
        return kOfxStatFailed;

    if (fx->clipDefine(effect, "Source", &clip) != kOfxStatOK || !clip) return kOfxStatFailed;
    if (p->propSetString(clip, kOfxImageEffectPropSupportedComponents, 0,
                         kOfxImageComponentRGBA) != kOfxStatOK) return kOfxStatFailed;
    if (fx->clipDefine(effect, "Output", &clip) != kOfxStatOK || !clip) return kOfxStatFailed;
    if (p->propSetString(clip, kOfxImageEffectPropSupportedComponents, 0,
                         kOfxImageComponentRGBA) != kOfxStatOK) return kOfxStatFailed;
    if (fx->getParamSet(effect, &pset) != kOfxStatOK || !pset) return kOfxStatFailed;

    /* gain  - Double default 1, display [0,2], hard +-1e20 (or +-DBL_MAX wide) */
    if (pp->paramDefine(pset, kOfxParamTypeDouble, "gain", &prm) != kOfxStatOK) return kOfxStatFailed;
    if (p->propSetDouble(prm, kOfxParamPropDefault, 0, 1.0) != kOfxStatOK) return kOfxStatFailed;
    if (g_mode & FX_WIDE_BOUNDS) {
        if (p->propSetDouble(prm, kOfxParamPropMin, 0, -DBL_MAX) != kOfxStatOK) return kOfxStatFailed;
        if (p->propSetDouble(prm, kOfxParamPropMax, 0, DBL_MAX) != kOfxStatOK) return kOfxStatFailed;
    } else {
        if (p->propSetDouble(prm, kOfxParamPropMin, 0, -1e20) != kOfxStatOK) return kOfxStatFailed;
        if (p->propSetDouble(prm, kOfxParamPropMax, 0, 1e20) != kOfxStatOK) return kOfxStatFailed;
    }
    if (p->propSetDouble(prm, kOfxParamPropDisplayMin, 0, 0.0) != kOfxStatOK) return kOfxStatFailed;
    if (p->propSetDouble(prm, kOfxParamPropDisplayMax, 0, 2.0) != kOfxStatOK) return kOfxStatFailed;

    /* mode  - Choice[a,b] default 1 */
    if (pp->paramDefine(pset, kOfxParamTypeChoice, "mode", &prm) != kOfxStatOK) return kOfxStatFailed;
    if (p->propSetString(prm, kOfxParamPropChoiceOption, 0, "a") != kOfxStatOK) return kOfxStatFailed;
    if (p->propSetString(prm, kOfxParamPropChoiceOption, 1, "b") != kOfxStatOK) return kOfxStatFailed;
    if (p->propSetInt(prm, kOfxParamPropDefault, 0, 1) != kOfxStatOK) return kOfxStatFailed;
    if (p->propSetInt(prm, kOfxParamPropMin, 0, 0) != kOfxStatOK) return kOfxStatFailed;
    if (p->propSetInt(prm, kOfxParamPropMax, 0, 1) != kOfxStatOK) return kOfxStatFailed;

    /* count  - Integer 0..10 default 0; enabled  - Bool default 1 */
    if (pp->paramDefine(pset, kOfxParamTypeInteger, "count", &prm) != kOfxStatOK) return kOfxStatFailed;
    if (p->propSetInt(prm, kOfxParamPropMin, 0, 0) != kOfxStatOK) return kOfxStatFailed;
    if (p->propSetInt(prm, kOfxParamPropMax, 0, 10) != kOfxStatOK) return kOfxStatFailed;
    if (p->propSetInt(prm, kOfxParamPropDefault, 0, 0) != kOfxStatOK) return kOfxStatFailed;
    if (pp->paramDefine(pset, kOfxParamTypeBoolean, "enabled", &prm) != kOfxStatOK) return kOfxStatFailed;
    if (p->propSetInt(prm, kOfxParamPropDefault, 0, 1) != kOfxStatOK) return kOfxStatFailed;
    if (p->propSetInt(prm, kOfxParamPropMin, 0, 0) != kOfxStatOK) return kOfxStatFailed;
    if (p->propSetInt(prm, kOfxParamPropMax, 0, 1) != kOfxStatOK) return kOfxStatFailed;

    /* outv  - readonly (Enabled=0) Double, bounds [0,10] default 2 */
    if (pp->paramDefine(pset, kOfxParamTypeDouble, "outv", &prm) != kOfxStatOK) return kOfxStatFailed;
    if (p->propSetInt(prm, kOfxParamPropEnabled, 0, 0) != kOfxStatOK) return kOfxStatFailed;
    if (p->propSetDouble(prm, kOfxParamPropMin, 0, 0.0) != kOfxStatOK) return kOfxStatFailed;
    if (p->propSetDouble(prm, kOfxParamPropMax, 0, 10.0) != kOfxStatOK) return kOfxStatFailed;
    if (p->propSetDouble(prm, kOfxParamPropDefault, 0, 2.0) != kOfxStatOK) return kOfxStatFailed;

    /* strval  - valid String descriptor (unsupported kind for host admission) */
    if (g_mode & FX_STRING_PARAM) {
        if (pp->paramDefine(pset, kOfxParamTypeString, "strval", &prm) != kOfxStatOK) return kOfxStatFailed;
        if (p->propSetString(prm, kOfxParamPropDefault, 0, "s") != kOfxStatOK) return kOfxStatFailed;
    }
    return kOfxStatOK;
}

static OfxStatus create_inst(OfxImageEffectHandle effect, int idx) {
    FixtureInstance *inst;
    const OfxImageEffectSuiteV1 *fx = fx_effects();
    const OfxParameterSuiteV1 *pp = fx_params();
    const OfxPropertySuiteV1 *p = fx_props();
    OfxParamSetHandle pset = NULL;
    OfxParamHandle gain = NULL;
    OfxPropertySetHandle set = NULL;
    double staged = 0.0;

    emit(FX_CREATE_ENTER, idx);
    if (g_mode & FX_FAIL_CREATE) return kOfxStatFailed;
    if (!fx || !pp || !p) return kOfxStatErrMissingHostFeature;
    if (fx->getParamSet(effect, &pset) != kOfxStatOK || !pset) return kOfxStatFailed;
    if (pp->paramGetHandle(pset, "gain", &gain, NULL) != kOfxStatOK || !gain) return kOfxStatFailed;
    if (pp->paramGetValue(gain, &staged) != kOfxStatOK) return kOfxStatFailed;
    if (fx->getPropertySet(effect, &set) != kOfxStatOK || !set) return kOfxStatFailed;

    inst = (FixtureInstance *)calloc(1, sizeof(FixtureInstance));
    if (!inst) return kOfxStatErrMemory;
    inst->effect = effect;
    inst->created_gain = staged;
    if (p->propSetPointer(set, kOfxPropInstanceData, 0, inst) != kOfxStatOK) {
        free(inst);
        return kOfxStatFailed;
    }
    inst->next = g_instances; g_instances = inst;
    g_created_gain = staged;
    if (g_mode & FX_WORKER_ON_CREATE) {
        /* exercise the host callback path from a foreign OS thread while we
         * are inside the native Create action; failure unwinds this create. */
        if (ofx_fixture_probe(FX_PROBE_WORKER) != 0) {
            p->propSetPointer(set, kOfxPropInstanceData, 0, NULL);
            g_instances = inst->next;
            free(inst);
            return kOfxStatFailed;
        }
    }
    emit(FX_CREATE_OK, idx);
    return kOfxStatOK;
}

static OfxStatus destroy_inst(OfxImageEffectHandle effect, int idx) {
    FixtureInstance **link, *victim = NULL;
    const OfxPropertySuiteV1 *p = fx_props();
    const OfxImageEffectSuiteV1 *fx = fx_effects();
    OfxPropertySetHandle set = NULL;

    emit(FX_DESTROY, idx);
    if (g_mode & FX_FAIL_DESTROY) return kOfxStatFailed;

    for (link = &g_instances; *link; link = &(*link)->next)
        if ((*link)->effect == effect) { victim = *link; break; }
    if (!victim) return kOfxStatErrBadHandle;
    if (!p || !fx) return kOfxStatErrMissingHostFeature;
    if (fx->getPropertySet(effect, &set) != kOfxStatOK || !set) return kOfxStatFailed;
    if (p->propSetPointer(set, kOfxPropInstanceData, 0, NULL) != kOfxStatOK)
        return kOfxStatFailed; /* leave resource alive  - unload cleans up */
    *link = victim->next;
    free(victim);
    return kOfxStatOK;
}

/* pointwise gain over the full image bounds. Uses only the image
 * property contract (data pointer + signed rowBytes + bounds), so the
 * host's bottom-left/negative-rowBytes exposure is what makes the SDK
 * corner assertions meaningful. */
static OfxStatus render_inst(OfxImageEffectHandle effect,
                             OfxPropertySetHandle in_args, int idx) {
    FixtureInstance *inst;
    const OfxPropertySuiteV1 *p;
    const OfxParameterSuiteV1 *pp;
    const OfxImageEffectSuiteV1 *fx;
    OfxParamSetHandle pset = NULL;
    OfxParamHandle gain = NULL;
    OfxImageClipHandle src_clip = NULL, dst_clip = NULL;
    OfxPropertySetHandle src_img = NULL, dst_img = NULL;
    void *src_data = NULL, *dst_data = NULL;
    int src_rb = 0, dst_rb = 0, bounds[4] = {0, 0, 0, 0};
    double t = 0.0, g = 1.0;
    int x, y, x1, y1, x2, y2;
    OfxStatus st = kOfxStatFailed, r1, r2;

    emit(FX_RENDER, idx);
    if (g_mode & FX_FAIL_RENDER) return kOfxStatFailed;
    inst = fx_find_instance(effect);
    p = fx_props(); pp = fx_params(); fx = fx_effects();
    if (!inst || !p || !pp || !fx) return kOfxStatErrBadHandle;
    if (!in_args ||
        p->propGetDouble(in_args, kOfxPropTime, 0, &t) != kOfxStatOK)
        return kOfxStatFailed;
    if (fx->getParamSet(effect, &pset) != kOfxStatOK || !pset) return kOfxStatFailed;
    if (pp->paramGetHandle(pset, "gain", &gain, NULL) != kOfxStatOK || !gain)
        return kOfxStatFailed;
    if (pp->paramGetValueAtTime(gain, t, &g) != kOfxStatOK) return kOfxStatFailed;
    if (fx->clipGetHandle(effect, "Source", &src_clip, NULL) != kOfxStatOK || !src_clip)
        return kOfxStatFailed;
    if (fx->clipGetHandle(effect, "Output", &dst_clip, NULL) != kOfxStatOK || !dst_clip)
        return kOfxStatFailed;
    if (fx->clipGetImage(src_clip, t, NULL, &src_img) != kOfxStatOK || !src_img)
        return kOfxStatFailed;
    if (fx->clipGetImage(dst_clip, t, NULL, &dst_img) != kOfxStatOK || !dst_img) {
        (void)fx->clipReleaseImage(src_img);
        return kOfxStatFailed;
    }
    if (p->propGetPointer(src_img, kOfxImagePropData, 0, &src_data) != kOfxStatOK ||
        p->propGetPointer(dst_img, kOfxImagePropData, 0, &dst_data) != kOfxStatOK ||
        p->propGetInt(src_img, kOfxImagePropRowBytes, 0, &src_rb) != kOfxStatOK ||
        p->propGetInt(dst_img, kOfxImagePropRowBytes, 0, &dst_rb) != kOfxStatOK ||
        p->propGetIntN(dst_img, kOfxImagePropBounds, 4, bounds) != kOfxStatOK)
        goto done;
    g_src_rowbytes = src_rb;
    g_dst_rowbytes = dst_rb;
    g_render_time = t;
    x1 = bounds[0]; y1 = bounds[1]; x2 = bounds[2]; y2 = bounds[3];
    /* record the Source pixels the host exposed at image coords
     * (x1,y1)..(x1+1,y1+1)   under the bottom-left contract these are the
     * bottom-left 2x2 of the frame */
    for (y = 0; y < 2; y++)
        for (x = 0; x < 2; x++)
            memcpy(g_src_corners + (y * 2 + x) * 4,
                   (const unsigned char *)src_data +
                       (ptrdiff_t)(y1 + y) * src_rb + (ptrdiff_t)(x1 + x) * 4,
                   4);
    for (y = y1; y < y2; y++) {
        const unsigned char *sr =
            (const unsigned char *)src_data + (ptrdiff_t)y * src_rb;
        unsigned char *dr = (unsigned char *)dst_data + (ptrdiff_t)y * dst_rb;
        for (x = x1; x < x2; x++) {
            const unsigned char *sp = sr + (ptrdiff_t)x * 4;
            unsigned char *dp = dr + (ptrdiff_t)x * 4;
            dp[0] = (unsigned char)(sp[0] * g + 0.5);
            dp[1] = (unsigned char)(sp[1] * g + 0.5);
            dp[2] = (unsigned char)(sp[2] * g + 0.5);
            dp[3] = sp[3];
        }
    }
    st = kOfxStatOK;
done:
    r1 = fx->clipReleaseImage(src_img);
    r2 = fx->clipReleaseImage(dst_img);
    if (st == kOfxStatOK && (r1 != kOfxStatOK || r2 != kOfxStatOK))
        st = kOfxStatFailed;
    return st;
}

/* GetClipPreferences: slave params steer the offered output contract.
 * enabled == 0 offers RGB output (outside the host RGBA contract);
 * mode choice 0 offers float depth (outside the byte contract). Both
 * are legal param writes whose effect only the next render rejects. */
static OfxStatus clip_prefs(OfxImageEffectHandle effect,
                            OfxPropertySetHandle out_args, int idx) {
    FixtureInstance *inst;
    const OfxPropertySuiteV1 *p;
    const OfxParameterSuiteV1 *pp;
    const OfxImageEffectSuiteV1 *fx;
    OfxParamSetHandle pset = NULL;
    OfxParamHandle prm = NULL;
    int v = 0;
    emit(FX_CLIP_PREFS, idx);
    if (g_mode & FX_FAIL_CLIP_PREFS) return kOfxStatFailed;
    inst = fx_find_instance(effect);
    p = fx_props(); pp = fx_params(); fx = fx_effects();
    if (!inst || !p || !pp || !fx || !out_args) return kOfxStatOK;
    if (fx->getParamSet(effect, &pset) != kOfxStatOK || !pset) return kOfxStatOK;
    if (pp->paramGetHandle(pset, "enabled", &prm, NULL) == kOfxStatOK && prm &&
        pp->paramGetValue(prm, &v) == kOfxStatOK && v == 0)
        (void)p->propSetString(out_args, "OfxImageClipPropComponents_Output", 0,
                               kOfxImageComponentRGB);
    if (pp->paramGetHandle(pset, "mode", &prm, NULL) == kOfxStatOK && prm &&
        pp->paramGetValue(prm, &v) == kOfxStatOK && v == 0)
        (void)p->propSetString(out_args, "OfxImageClipPropDepth_Output", 0,
                               kOfxBitDepthFloat);
    return kOfxStatOK;
}

static void unload_free_instances(void) {
    /* free fixture-owned memory only  - NO foreign calls on stale handles */
    FixtureInstance *i = g_instances;
    while (i) { FixtureInstance *n = i->next; free(i); i = n; }
    g_instances = NULL;
}

static OfxStatus main_entry(int idx, const char *action, const void *handle,
                            OfxPropertySetHandle in_args,
                            OfxPropertySetHandle out_args) {
    (void)out_args;
    if (!action) return kOfxStatFailed;
    if (!strcmp(action, kOfxActionLoad))   { emit(FX_LOAD, idx);   return kOfxStatOK; }
    if (!strcmp(action, kOfxActionUnload)) { emit(FX_UNLOAD, idx); if (idx == 0) unload_free_instances(); return kOfxStatOK; }
    if (!strcmp(action, kOfxActionDescribe))
        return describe((OfxImageEffectHandle)handle,
                        (g_mode & FX_NO_FILTER) ? kOfxImageEffectContextGenerator
                                                : kOfxImageEffectContextFilter, idx);
    if (!strcmp(action, kOfxImageEffectActionDescribeInContext))
        return describe_in_context((OfxImageEffectHandle)handle, in_args,
                                   (g_mode & FX_NO_FILTER) ? kOfxImageEffectContextGenerator
                                                           : kOfxImageEffectContextFilter, idx);
    if (!strcmp(action, kOfxActionCreateInstance))  return create_inst((OfxImageEffectHandle)handle, idx);
    if (!strcmp(action, kOfxActionDestroyInstance)) return destroy_inst((OfxImageEffectHandle)handle, idx);
    if (!strcmp(action, kOfxActionBeginInstanceChanged)) { emit(FX_BEGIN_CHANGED, idx); return kOfxStatOK; }
    if (!strcmp(action, kOfxActionEndInstanceChanged))   { emit(FX_END_CHANGED, idx);   return kOfxStatOK; }
    if (!strcmp(action, kOfxActionInstanceChanged))      { emit(FX_CHANGED, idx);       return kOfxStatOK; }
    if (!strcmp(action, kOfxImageEffectActionGetClipPreferences))
        return clip_prefs((OfxImageEffectHandle)handle, out_args, idx);
    if (!strcmp(action, kOfxImageEffectActionBeginSequenceRender)) {
        emit(FX_BEGIN_RENDER, idx);
        if (g_mode & FX_FAIL_BEGIN_RENDER) return kOfxStatFailed;
        /* legal non-answer: "not trapped, but all is well anyway" */
        return (g_mode & FX_REPLY_DEFAULT_BRACKET) ? kOfxStatReplyDefault
                                                 : kOfxStatOK;
    }
    if (!strcmp(action, kOfxImageEffectActionRender))
        return render_inst((OfxImageEffectHandle)handle, in_args, idx);
    if (!strcmp(action, kOfxImageEffectActionEndSequenceRender)) {
        emit(FX_END_RENDER, idx);
        if (g_mode & FX_FAIL_END_RENDER) return kOfxStatFailed;
        return (g_mode & FX_REPLY_DEFAULT_BRACKET) ? kOfxStatReplyDefault
                                                 : kOfxStatOK;
    }
    return kOfxStatReplyDefault;
}

static OfxStatus gain_main(const char *a, const void *h, OfxPropertySetHandle i, OfxPropertySetHandle o) {
    return main_entry(0, a, h, i, o);
}
static OfxStatus gen_main(const char *a, const void *h, OfxPropertySetHandle i, OfxPropertySetHandle o) {
    if (a && !strcmp(a, kOfxActionDescribe))
        return describe((OfxImageEffectHandle)h, kOfxImageEffectContextGenerator, 1);
    if (a && !strcmp(a, kOfxImageEffectActionDescribeInContext))
        return describe_in_context((OfxImageEffectHandle)h, i, kOfxImageEffectContextGenerator, 1);
    return main_entry(1, a, h, i, o);
}

static void gain_set_host(OfxHost *host) { g_host = host; emit(FX_SET_HOST, 0); }
static void gen_set_host(OfxHost *host)  { g_host = host; emit(FX_SET_HOST, 1); }

/* descriptor rebuilt fresh per OfxGetPlugin call, then modes applied */
static OfxPlugin g_gain = { kOfxImageEffectPluginApi, 1, "org.ruviform.gain",
                            0, 1, gain_set_host, gain_main };
static OfxPlugin g_gen  = { kOfxImageEffectPluginApi, 1, "org.ruviform.generator",
                            0, 1, gen_set_host, gen_main };

OfxExport OfxStatus OfxSetHost(const OfxHost *host) {
    emit(FX_DLL_SET_HOST, -1);
    if (g_mode & FX_REFUSE_SET_HOST) return kOfxStatFailed;
    g_host = (OfxHost *)host;
    return kOfxStatOK;
}

int OfxGetNumberOfPlugins(void) {
    emit(FX_GET_COUNT, -1);
    return (g_mode & FX_BAD_COUNT) ? -1 : 2;
}
OfxPlugin *OfxGetPlugin(int nth) {
    emit(FX_GET_PLUGIN, nth);
    /* rebuild defaults each call, then apply mode mutations (gain only) */
    g_gain.pluginApi = kOfxImageEffectPluginApi;
    g_gain.pluginIdentifier = "org.ruviform.gain";
    g_gain.setHost = gain_set_host;
    g_gain.mainEntry = gain_main;
    if (nth == 0) {
        if (g_mode & FX_NULL_ENTRY)    return NULL;
        if (g_mode & FX_INVALID_API)   g_gain.pluginApi = "OfxBadApi";
        if (g_mode & FX_EMPTY_ID)      g_gain.pluginIdentifier = "";
        if (g_mode & FX_NULL_SET_HOST) g_gain.setHost = NULL;
        if (g_mode & FX_NULL_MAIN)     g_gain.mainEntry = NULL;
        return &g_gain;
    }
    if (nth == 1) return &g_gen;
    return NULL;
}

/* ---- read_gain / created_gain (real Param suite on last live instance) ---- */

int ofx_fixture_read_gain(double *current, double *at_time) {
    FixtureInstance *inst = fx_last_instance();
    const OfxImageEffectSuiteV1 *fx;
    const OfxParameterSuiteV1 *pp;
    OfxParamSetHandle pset = NULL;
    OfxParamHandle gain = NULL;
    if (!inst || !current || !at_time) return 1;
    fx = fx_effects(); pp = fx_params();
    if (!fx || !pp) return 2;
    if (fx->getParamSet(inst->effect, &pset) != kOfxStatOK || !pset) return 3;
    if (pp->paramGetHandle(pset, "gain", &gain, NULL) != kOfxStatOK || !gain) return 4;
    if (pp->paramGetValue(gain, current) != kOfxStatOK) return 5;
    if (pp->paramGetValueAtTime(gain, 0.0, at_time) != kOfxStatOK) return 6;
    return 0;
}

int ofx_fixture_created_gain(double *value) {
    if (!value || fx_live_count() == 0) return 1;
    *value = fx_created_gain_value();
    return 0;
}

int ofx_fixture_last_render_time(double *time) {
    if (!time) return 1;
    *time = g_render_time;
    return 0;
}

int ofx_fixture_last_rowbytes(int *src, int *dst) {
    if (!src || !dst) return 1;
    *src = g_src_rowbytes;
    *dst = g_dst_rowbytes;
    return 0;
}

int ofx_fixture_src_corners(unsigned char rgba16[16]) {
    if (!rgba16) return 1;
    memcpy(rgba16, g_src_corners, 16);
    return 0;
}
