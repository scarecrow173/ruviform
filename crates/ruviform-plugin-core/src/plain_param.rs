//! Typed plain parameter channel for plugin control (Issue #98).
//!
//! This is a separate, NONRT control API — distinct from the normalized
//! [`crate::ParamValue`] wire path used by audio plugins. Values carry their
//! concrete scalar type; no normalized mapping is applied anywhere on this
//! channel.
//!
//! [`PlainParamValue::Double`] may be constructed with a non-finite value —
//! the enum is a raw type. Finiteness is enforced by setters/boundary
//! validation, not by the constructor.

use crate::{ParamFlags, ParamId};

/// Plain typed parameter value for the video/native control channel.
///
/// The variant itself encodes the parameter kind — a separate `kind`
/// discriminant would duplicate knowledge.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PlainParamValue {
    /// 64-bit float parameter.
    Double(f64),
    /// 32-bit signed integer parameter.
    Integer(i32),
    /// Boolean parameter.
    Boolean(bool),
    /// Index into a choice list.
    Choice(u32),
}

/// Borrowed parameter metadata view projected by an adapter.
///
/// All string/list fields borrow from adapter-owned storage — the view never
/// outlives its owner and never leaks memory to satisfy `'static`.
///
/// Contract (adapter responsibility — the view is a raw projection):
/// `min_value`/`max_value` are finite, ordered hard endpoints;
/// `default_value` lies within them. A double domain whose `max - min`
/// overflows `f64` is **not** invalid by itself. `display_*` are UI hints
/// only — they never narrow the hard domain and imply no implicit clamp.
/// For `Choice` parameters `choices` is non-empty and the default/current
/// index is in bounds and fits `i32`; for every other kind `choices` is
/// empty.
#[derive(Copy, Clone, Debug)]
pub struct PlainParamInfo<'a> {
    /// Parameter identifier (define-order stable).
    pub id: ParamId,
    /// Parameter name.
    pub name: &'a str,
    /// Unit label (may be empty).
    pub unit: &'a str,
    /// Hard minimum (finite; logical domain).
    pub min_value: f64,
    /// Hard maximum (finite; logical domain).
    pub max_value: f64,
    /// Display minimum (UI hint only; does not narrow the hard domain).
    pub display_min_value: f64,
    /// Display maximum (UI hint only; does not narrow the hard domain).
    pub display_max_value: f64,
    /// Declared default value.
    pub default_value: PlainParamValue,
    /// Parameter flags.
    pub flags: ParamFlags,
    /// Choice option labels — empty unless `default_value` is `Choice`.
    pub choices: &'a [String],
}

/// Owned parameter metadata — the owned counterpart of
/// [`PlainParamInfo`].
///
/// A borrowed view cannot cross a process boundary: the L1 wire codec
/// ([`crate::wire`]) carries parameter metadata as owned strings and
/// lists, and the video host surface returns them to callers in this
/// owned form. The `PlainParamInfo` contract applies unchanged:
/// `min_value`/`max_value` are finite, ordered hard endpoints with
/// `default_value` inside them; `display_*` are UI hints only (no
/// finiteness requirement — they never narrow the hard domain);
/// `choices` is non-empty exactly when `default_value` is
/// [`PlainParamValue::Choice`], with the default/current index in
/// bounds.
///
/// Conversions live here only: [`From`] builds the owned copy from a
/// view, [`as_view`](Self::as_view) projects back to the borrowed form.
///
/// **RT contract:** non-RT only — owns `String`/`Vec` storage.
#[derive(Clone, Debug, PartialEq)]
pub struct PlainParamInfoOwned {
    /// Parameter identifier (define-order stable).
    pub id: ParamId,
    /// Parameter name.
    pub name: String,
    /// Unit label (may be empty).
    pub unit: String,
    /// Hard minimum (finite; logical domain).
    pub min_value: f64,
    /// Hard maximum (finite; logical domain).
    pub max_value: f64,
    /// Display minimum (UI hint only; does not narrow the hard domain).
    pub display_min_value: f64,
    /// Display maximum (UI hint only; does not narrow the hard domain).
    pub display_max_value: f64,
    /// Declared default value.
    pub default_value: PlainParamValue,
    /// Parameter flags.
    pub flags: ParamFlags,
    /// Choice option labels — empty unless `default_value` is `Choice`.
    pub choices: Vec<String>,
}

impl PlainParamInfoOwned {
    /// Projects back to the borrowed [`PlainParamInfo`] view — the
    /// inverse of [`From`].
    ///
    /// **RT contract:** non-RT only — borrows `String`/`Vec` storage.
    ///
    /// # Examples
    ///
    /// ```
    /// use ruviform_plugin_core::{ParamFlags, ParamId, PlainParamInfoOwned, PlainParamValue};
    ///
    /// let info = PlainParamInfoOwned {
    ///     id: ParamId(0),
    ///     name: "Threshold".to_owned(),
    ///     unit: "dB".to_owned(),
    ///     min_value: -60.0,
    ///     max_value: 12.0,
    ///     display_min_value: -60.0,
    ///     display_max_value: 12.0,
    ///     default_value: PlainParamValue::Double(0.0),
    ///     flags: ParamFlags::AUTOMATABLE,
    ///     choices: Vec::new(),
    /// };
    /// let view = info.as_view();
    /// assert_eq!(view.name, "Threshold");
    /// assert_eq!(PlainParamInfoOwned::from(&view).name, info.name);
    /// ```
    #[must_use]
    pub fn as_view(&self) -> PlainParamInfo<'_> {
        PlainParamInfo {
            id: self.id,
            name: &self.name,
            unit: &self.unit,
            min_value: self.min_value,
            max_value: self.max_value,
            display_min_value: self.display_min_value,
            display_max_value: self.display_max_value,
            default_value: self.default_value,
            flags: self.flags,
            choices: self.choices.as_slice(),
        }
    }
}

impl From<&PlainParamInfo<'_>> for PlainParamInfoOwned {
    fn from(info: &PlainParamInfo<'_>) -> Self {
        Self {
            id: info.id,
            name: info.name.to_owned(),
            unit: info.unit.to_owned(),
            min_value: info.min_value,
            max_value: info.max_value,
            display_min_value: info.display_min_value,
            display_max_value: info.display_max_value,
            default_value: info.default_value,
            flags: info.flags,
            choices: info.choices.to_vec(),
        }
    }
}

impl From<PlainParamInfo<'_>> for PlainParamInfoOwned {
    fn from(info: PlainParamInfo<'_>) -> Self {
        Self::from(&info)
    }
}
