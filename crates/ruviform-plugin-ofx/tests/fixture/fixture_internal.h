/* fixture_internal.h  - private shared declarations between the fixture
 * translation units. NOT part of the exported fixture_control.h surface. */
#ifndef RUVIFORM_OFX_FIXTURE_INTERNAL_H
#define RUVIFORM_OFX_FIXTURE_INTERNAL_H

#include "ofxCore.h"
#include "ofxImageEffect.h"
#include "ofxParam.h"
#include "ofxMemory.h"
#include "fixture_control.h"

typedef struct FixtureInstance {
    struct FixtureInstance *next;
    OfxImageEffectHandle effect;
    double created_gain;
} FixtureInstance;

/* Internal accessors implemented by ofx_test_fixture.c, used by
 * ofx_fixture_probes.c  - pass explicit handles, no duplicate registry. */
OfxHost *fx_current_host(void);
const OfxPropertySuiteV1 *fx_props(void);
const OfxParameterSuiteV1 *fx_params(void);
const OfxImageEffectSuiteV1 *fx_effects(void);
const OfxMemorySuiteV1 *fx_memory(void);
FixtureInstance *fx_last_instance(void);
FixtureInstance *fx_find_instance(OfxImageEffectHandle effect);
int fx_live_count(void);
double fx_created_gain_value(void);

#endif
