/*
 * ofx_bridge.h   private C bridge ABI for ruviform-plugin-ofx (Issue #98).
 *
 * The C side owns OpenFX SDK layouts, suite tables, variadic host
 * callbacks, OfxHost, and typed native-action wrappers. Rust sees only
 * fixed-arity primitive/pointer helpers declared here   never SDK types.
 *
 * The bridge implements the real host/session lifecycle: catalog admission,
 * descriptor/instance attempts, suite callbacks, and teardown; video render
 * lands in a later task.
 */
#ifndef RUVIFORM_OFX_BRIDGE_H
#define RUVIFORM_OFX_BRIDGE_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Image Effect plugin API version the bridge was built against
 * (kOfxImageEffectPluginApiVersion from ofxImageEffect.h). */
int32_t ofx_bridge_image_effect_api_version(void);

/* Private fixed-arity ABI. No SDK layout crosses into Rust. All opaque
 * handles except the catalog pointer are generation tokens, never
 * dereferenced; the host handle is the session token host_new returns.
 * Strings are owned by the host until its teardown. */
void *rv_ofx_host_new(void);
void rv_ofx_host_free(void *host);

/* The loader supplies addresses obtained from the trusted native library.
 * Admission invokes optional DLL SetHost before enumeration. The catalog
 * owns successful Loads and closes them once, before the OS DLL unloads. */
int rv_ofx_catalog_open(void *host, const void *get_count,
                       const void *get_plugin, const void *set_host,
                       void **catalog);
void rv_ofx_catalog_close(void *catalog);
size_t rv_ofx_catalog_count(void *catalog);
void *rv_ofx_catalog_descriptor(void *catalog, size_t native_index);
const char *rv_ofx_catalog_identifier(void *catalog, size_t native_index);
int rv_ofx_catalog_rejection(void *catalog, size_t native_index);

/* Descriptor fields: 0=id, 1=label, 2=vendor, 3=version.
 * Parameter kinds: 0=Double, 1=Integer, 2=Boolean, 3=Choice.
 * Flags map to the neutral core's STEPPED/HIDDEN/READONLY bits. */
const char *rv_ofx_descriptor_string(void *descriptor, int field);
size_t rv_ofx_param_count(void *effect);
int rv_ofx_param_info(void *effect, size_t index, int *kind,
                      const char **name, const char **unit,
                      double *min_value, double *max_value,
                      double *display_min, double *display_max,
                      double *default_double, int *default_int,
                      unsigned int *flags, size_t *choice_count);
const char *rv_ofx_param_choice(void *effect, size_t index, size_t choice);
int rv_ofx_param_get(void *effect, size_t index, double *value_double,
                     int *value_int);
int rv_ofx_param_validate(void *effect, size_t index, int kind,
                          double value_double, int value_int, int host_write);
/* host_write=1 enforces the UI readonly restriction; native ParamSuite
 * callbacks use the same domain validator without that UI restriction. */
int rv_ofx_param_set(void *effect, size_t index, int kind,
                     double value_double, int value_int, int host_write);

/* Allocates a host-owned native attempt, not a plugin instance. Values are
 * staged into this attempt before NativeCreate. A failed create may be
 * discarded and retried without claiming foreign-owned cleanup. */
int rv_ofx_instance_new(void *descriptor, void **effect);
int rv_ofx_instance_configure(void *effect, uint32_t width, uint32_t height,
                              double frame_rate);
int rv_ofx_instance_create(void *effect);
int rv_ofx_instance_changed(void *effect, size_t param_index);
int rv_ofx_instance_preferences(void *effect);
/* Per-frame Begin/Render/End bracket over adapter-owned staging buffers.
 * `src`/`dst` are top-row-first RGBA8 frames of the configured size with
 * strides >= width*4; the plugin sees bottom-left data (negative rowBytes).
 * The clip-preference contract is revalidated before every bracket, and
 * output rows/padding are committed only after a fully successful bracket. */
int rv_ofx_instance_render(void *effect, const unsigned char *src,
                           size_t src_stride, unsigned char *dst,
                           size_t dst_stride, double time);
int rv_ofx_instance_destroy(void *effect);
/* Never issues Destroy twice. Failed-Destroy resources remain quarantined
 * until catalog Unload and continue holding SingleInstance admission. */
void rv_ofx_instance_release(void *effect);

#ifdef __cplusplus
}
#endif

#endif /* RUVIFORM_OFX_BRIDGE_H */
