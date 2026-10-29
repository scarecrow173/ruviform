//! Standard-neutral parameter description and value types:
//! [`ParamId`], [`ParamFlags`], [`ParamInfo`] and the normalized
//! [`ParamValue`].
//!
//! ## Value domain and NaN policy
//!
//! [`ParamValue`] is the wire-canonical parameter value: a normalized
//! `f64` restricted to `[0.0, 1.0]`. The stored invariant — finite and
//! in range — is enforced by construction:
//!
//! - `NaN` and `±inf` are **rejected** at the boundary
//!   ([`ParamValue::new`] returns `Err`) — non-finite values are never
//!   stored.
//! - Finite out-of-range inputs are **clamped** into `[0.0, 1.0]`.
//!
//! [`ParamInfo`] maps the normalized domain linearly onto the
//! parameter's plain range `[min_value, max_value]`
//! ([`ParamInfo::normalize`] / [`ParamInfo::denormalize`]). Plain
//! (non-normalized) `NaN`/`±inf` inputs are likewise rejected by
//! `normalize`. Descriptor range fields are plugin-author data and must
//! carry finite values with `min_value <= max_value`; a degenerate
//! `min_value == max_value` range normalizes to `0.0` and denormalizes
//! to `min_value`.

use ruviform_media_core::{Error, Result};

/// Identifier of a parameter within a plugin.
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_plugin_core::ParamId;
///
/// let id = ParamId(3);
/// assert_eq!(id.0, 3);
/// ```
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ParamId(pub u32);

/// Boolean capability flags of a parameter, as a bitmask.
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_plugin_core::ParamFlags;
///
/// let flags = ParamFlags::AUTOMATABLE.union(ParamFlags::STEPPED);
/// assert!(flags.contains(ParamFlags::AUTOMATABLE));
/// assert!(!flags.contains(ParamFlags::HIDDEN));
/// ```
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ParamFlags(u32);

impl ParamFlags {
    /// No flags set.
    pub const NONE: Self = Self(0);
    /// The host may automate this parameter over time.
    pub const AUTOMATABLE: Self = Self(1 << 0);
    /// The host may modulate this parameter per-note/per-voice.
    pub const MODULATABLE: Self = Self(1 << 1);
    /// The parameter takes stepped (discrete) values rather than a
    /// continuous range.
    pub const STEPPED: Self = Self(1 << 2);
    /// The parameter is hidden from generic user interfaces.
    pub const HIDDEN: Self = Self(1 << 3);
    /// The parameter is read-only: the plugin may change it, the host
    /// may not write it.
    pub const READONLY: Self = Self(1 << 4);
    /// The parameter is the plugin's bypass control.
    pub const BYPASS: Self = Self(1 << 5);

    /// The empty flag set — same as [`ParamFlags::NONE`].
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn empty() -> Self {
        Self::NONE
    }

    /// The raw bitmask.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// The union of two flag sets.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether `self` contains every flag in `other`.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Every bit this revision defines — the union of all named flags.
    const KNOWN_BITS: u32 = Self::AUTOMATABLE.0
        | Self::MODULATABLE.0
        | Self::STEPPED.0
        | Self::HIDDEN.0
        | Self::READONLY.0
        | Self::BYPASS.0;

    /// Rebuilds a flag set from its raw bitmask — `None` when `bits`
    /// sets a flag this revision does not define. The wire codec uses
    /// this strict reading so an unknown bit is rejected like any other
    /// reserved field rather than silently carried.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_plugin_core::ParamFlags;
    ///
    /// assert_eq!(
    ///     ParamFlags::from_bits(ParamFlags::AUTOMATABLE.bits()),
    ///     Some(ParamFlags::AUTOMATABLE),
    /// );
    /// assert_eq!(ParamFlags::from_bits(1 << 31), None);
    /// ```
    #[must_use]
    pub const fn from_bits(bits: u32) -> Option<Self> {
        if bits & !Self::KNOWN_BITS == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }
}

/// Normalized parameter value in `[0.0, 1.0]` — the wire-canonical
/// parameter representation.
///
/// NaN policy: `NaN`/`±inf` are rejected at construction and finite
/// out-of-range inputs are clamped, so a `ParamValue` is always finite
/// and in range. See the module docs for the full domain contract.
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_plugin_core::ParamValue;
///
/// let v = ParamValue::new(0.5).unwrap();
/// assert_eq!(v.get(), 0.5);
/// // Finite out-of-range inputs clamp; non-finite inputs are rejected.
/// assert_eq!(ParamValue::new(2.0).unwrap().get(), 1.0);
/// assert!(ParamValue::new(f64::NAN).is_err());
/// ```
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct ParamValue(f64);

impl ParamValue {
    /// The normalized `0.0` value.
    pub const ZERO: Self = Self(0.0);
    /// The normalized `1.0` value.
    pub const ONE: Self = Self(1.0);

    /// Creates a normalized value: clamps finite inputs into
    /// `[0.0, 1.0]` and rejects `NaN`/`±inf`.
    ///
    /// **RT contract:** RT-suitable — pure math, allocation-free.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `value` is `NaN` or `±inf`.
    #[inline]
    pub fn new(value: f64) -> Result<Self> {
        if value.is_finite() {
            Ok(Self(value.clamp(0.0, 1.0)))
        } else {
            Err(Error::InvalidValue("param_value"))
        }
    }

    /// The normalized `f64` — always finite and in `[0.0, 1.0]`.
    ///
    /// **RT contract:** RT-suitable — `const`, pure.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Standard-neutral parameter descriptor: id, name, unit, plain range,
/// plain default and capability flags.
///
/// `default_value` is expressed in the plain domain
/// `[min_value, max_value]`; [`ParamInfo::default_normalized`] converts
/// it to the normalized domain for standards that expect it. Range
/// fields must be finite with `min_value <= max_value` (see module
/// docs).
///
/// **RT contract:** RT-suitable — `Copy` value type.
///
/// # Examples
///
/// ```
/// use ruviform_plugin_core::{ParamFlags, ParamId, ParamInfo};
///
/// const GAIN_DB: ParamInfo = ParamInfo {
///     id: ParamId(0),
///     name: "Gain",
///     unit: "dB",
///     min_value: -60.0,
///     max_value: 12.0,
///     default_value: 0.0,
///     flags: ParamFlags::AUTOMATABLE,
/// };
/// // 0.0 dB sits at 60/72 of the normalized range.
/// let n = GAIN_DB.normalize(0.0).unwrap();
/// assert_eq!(n.get(), 60.0 / 72.0);
/// assert_eq!(GAIN_DB.denormalize(n), 0.0);
/// ```
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ParamInfo {
    /// Parameter identifier.
    pub id: ParamId,
    /// Human-readable parameter name.
    pub name: &'static str,
    /// Display unit (`"dB"`, `"Hz"`, …); `""` when unit-less.
    pub unit: &'static str,
    /// Lower bound of the plain value range.
    pub min_value: f64,
    /// Upper bound of the plain value range.
    pub max_value: f64,
    /// Default value in the plain domain.
    pub default_value: f64,
    /// Capability flags.
    pub flags: ParamFlags,
}

impl ParamInfo {
    /// Maps a plain-domain value to the normalized domain
    /// `[0.0, 1.0]` (linear, clamped; a degenerate `min == max` range
    /// normalizes to `0.0`).
    ///
    /// **RT contract:** RT-suitable — pure math, allocation-free.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `plain` is `NaN` or `±inf` — the
    /// same NaN policy as [`ParamValue::new`].
    pub fn normalize(&self, plain: f64) -> Result<ParamValue> {
        if !plain.is_finite() {
            return Err(Error::InvalidValue("param_value"));
        }
        let range = self.max_value - self.min_value;
        let normalized = if range > 0.0 {
            (plain - self.min_value) / range
        } else {
            0.0
        };
        // `normalized` is finite: `plain` is finite and a finite range
        // only yields infinities for degenerate ranges, which are
        // handled by the `range > 0.0` guard. `new` still clamps.
        ParamValue::new(normalized)
    }

    /// Maps a normalized value back to the plain domain
    /// (`min + n * (max - min)`; a degenerate range maps to `min`).
    ///
    /// **RT contract:** RT-suitable — pure math, allocation-free.
    #[must_use]
    #[inline]
    pub fn denormalize(&self, normalized: ParamValue) -> f64 {
        self.min_value + normalized.get() * (self.max_value - self.min_value)
    }

    /// The default value in the normalized domain.
    ///
    /// **RT contract:** RT-suitable — pure math, allocation-free.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] when `default_value` is `NaN` or `±inf`.
    pub fn default_normalized(&self) -> Result<ParamValue> {
        self.normalize(self.default_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PARAM: ParamInfo = ParamInfo {
        id: ParamId(1),
        name: "Level",
        unit: "",
        min_value: 0.0,
        max_value: 2.0,
        default_value: 1.0,
        flags: ParamFlags::NONE,
    };

    #[test]
    fn normalize_denormalize_round_trip() {
        for plain in [0.0, 0.5, 1.0, 2.0] {
            let n = PARAM.normalize(plain).unwrap();
            assert_eq!(PARAM.denormalize(n), plain);
        }
        assert_eq!(PARAM.normalize(0.0).unwrap(), ParamValue::ZERO);
        assert_eq!(PARAM.normalize(2.0).unwrap(), ParamValue::ONE);
    }

    #[test]
    fn out_of_range_clamps_and_non_finite_rejected() {
        assert_eq!(PARAM.normalize(100.0).unwrap(), ParamValue::ONE);
        assert_eq!(PARAM.normalize(-5.0).unwrap(), ParamValue::ZERO);
        assert!(ParamValue::new(f64::NAN).is_err());
        assert!(ParamValue::new(f64::INFINITY).is_err());
        assert!(ParamValue::new(f64::NEG_INFINITY).is_err());
        assert!(PARAM.normalize(f64::NAN).is_err());
    }

    #[test]
    fn degenerate_range_normalizes_to_zero() {
        const FLAT: ParamInfo = ParamInfo {
            id: ParamId(2),
            name: "flat",
            unit: "",
            min_value: 7.0,
            max_value: 7.0,
            default_value: 7.0,
            flags: ParamFlags::NONE,
        };
        assert_eq!(FLAT.normalize(7.0).unwrap(), ParamValue::ZERO);
        assert_eq!(FLAT.denormalize(ParamValue::new(0.9).unwrap()), 7.0);
    }

    #[test]
    fn flags_bitmask_ops() {
        let flags = ParamFlags::AUTOMATABLE
            .union(ParamFlags::MODULATABLE)
            .union(ParamFlags::READONLY);
        assert!(flags.contains(ParamFlags::MODULATABLE));
        assert!(flags.contains(ParamFlags::AUTOMATABLE.union(ParamFlags::READONLY)));
        assert!(!flags.contains(ParamFlags::BYPASS));
        assert!(!ParamFlags::empty().contains(ParamFlags::HIDDEN));
        assert_eq!(ParamFlags::AUTOMATABLE.bits(), 1);
    }

    #[test]
    fn default_normalized_uses_plain_default() {
        assert_eq!(PARAM.default_normalized().unwrap().get(), 0.5);
    }
}
