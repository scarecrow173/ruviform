//! The plugin-side contracts the host drives: [`Plugin`] (one
//! instance), [`PluginFactory`] (one registered plugin type) and the
//! erased I/O views [`HostInput`]/[`HostOutput`] the dyn dispatch
//! uses.
//!
//! The contracts are owned by `ruviform-plugin-core` — they moved
//! there so the out-of-process (L1) plugin runner can name them
//! without a host↔runner dependency — and are re-exported here
//! unchanged: `ruviform_plugin_host::Plugin` and friends keep
//! resolving on their existing paths.
//!
//! Standard adapters (`ruviform-plugin-clap` / `-vst3`) implement
//! these traits over their wrapped plugin objects and register a
//! factory with the host; this crate never names a plugin standard.

pub use ruviform_plugin_core::{
    ErasedPlugin, HostInput, HostOutput, Plugin, PluginFactory, erase, erase_sendable,
};
