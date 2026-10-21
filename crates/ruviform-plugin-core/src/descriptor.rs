//! Standard-neutral plugin description: the [`PluginDescriptor`] value
//! type.
//!
//! A descriptor is static data a plugin ships about itself; it carries
//! no runtime state and no standard-specific identifiers. Adapters map
//! these fields onto each standard's own identification scheme.

/// Standard-neutral plugin description.
///
/// All fields are `&'static` — a descriptor is compile-time data baked
/// into the plugin, not host-allocated state. `id` is the stable,
/// standard-neutral identifier (reverse-DNS style, e.g.
/// `"org.example.gain"`); adapters translate it to whatever each plugin
/// standard uses natively.
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_plugin_core::PluginDescriptor;
///
/// const GAIN: PluginDescriptor = PluginDescriptor {
///     id: "org.example.gain",
///     name: "Gain",
///     vendor: "Ruviform",
///     version: "1.0.0",
///     features: &["audio-effect", "utility"],
/// };
/// assert_eq!(GAIN.id, "org.example.gain");
/// assert_eq!(GAIN.features.len(), 2);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PluginDescriptor {
    /// Stable standard-neutral identifier (reverse-DNS style).
    pub id: &'static str,
    /// Human-readable plugin name.
    pub name: &'static str,
    /// Vendor / author name.
    pub vendor: &'static str,
    /// Version string in a standard-neutral form (e.g. `"1.0.0"`).
    pub version: &'static str,
    /// Feature descriptors (category tags such as `"audio-effect"`);
    /// may be empty. Adapters map them to standard-specific categories.
    pub features: &'static [&'static str],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_holds_literal_fields() {
        const D: PluginDescriptor = PluginDescriptor {
            id: "org.example.d",
            name: "D",
            vendor: "V",
            version: "0.0.1",
            features: &[],
        };
        let no_features: &[&'static str] = &[];
        assert_eq!(D.features, no_features);
        assert_eq!(D, D);
    }
}
