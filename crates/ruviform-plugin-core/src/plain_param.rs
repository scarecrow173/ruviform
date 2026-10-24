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
