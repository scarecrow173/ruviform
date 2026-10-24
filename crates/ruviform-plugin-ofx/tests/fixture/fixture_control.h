#ifndef RUVIFORM_OFX_FIXTURE_CONTROL_H
#define RUVIFORM_OFX_FIXTURE_CONTROL_H

#if defined(_WIN32)
#define FIXTURE_EXPORT __declspec(dllexport)
#else
#define FIXTURE_EXPORT __attribute__((visibility("default")))
#endif

enum FixtureEvent {
    FX_DLL_SET_HOST,
    FX_GET_COUNT,
    FX_GET_PLUGIN,
    FX_SET_HOST,
    FX_LOAD,
    FX_UNLOAD,
    FX_DESCRIBE,
    FX_DESCRIBE_CONTEXT,
    FX_CREATE_ENTER,
    FX_CREATE_OK,
    FX_DESTROY,
    FX_BEGIN_CHANGED,
    FX_CHANGED,
    FX_END_CHANGED,
    FX_CLIP_PREFS,
    FX_EVENT_COUNT
};

enum FixtureMode {
    FX_REFUSE_SET_HOST = 1 << 0,
    FX_FAIL_CREATE = 1 << 1,
    FX_FAIL_DESTROY = 1 << 2,
    FX_FAIL_CLIP_PREFS = 1 << 3,
    FX_INVALID_API = 1 << 4,
    FX_EMPTY_ID = 1 << 5,
    FX_NULL_SET_HOST = 1 << 6,
    FX_NULL_MAIN = 1 << 7,
    FX_NO_FILTER = 1 << 8,
    FX_BAD_COUNT = 1 << 9,
    FX_SINGLE_INSTANCE = 1 << 10,
    FX_WIDE_BOUNDS = 1 << 11,
    FX_NULL_ENTRY = 1 << 12,
    FX_STRING_PARAM = 1 << 13,
    FX_WORKER_ON_CREATE = 1 << 14
};

enum FixtureProbe {
    FX_PROBE_PROPERTY,
    FX_PROBE_PARAM,
    FX_PROBE_MEMORY,
    FX_PROBE_WORKER,
    FX_PROBE_READONLY
};

typedef void (*FixtureObserver)(int event, int native_index);

FIXTURE_EXPORT void ofx_fixture_configure(int mode, FixtureObserver observer);
FIXTURE_EXPORT void ofx_fixture_set_mode(int mode);
FIXTURE_EXPORT int ofx_fixture_probe(int which);
FIXTURE_EXPORT int ofx_fixture_read_gain(double *current, double *at_time);
FIXTURE_EXPORT int ofx_fixture_created_gain(double *value);

#endif
