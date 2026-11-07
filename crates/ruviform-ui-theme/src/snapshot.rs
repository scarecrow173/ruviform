//! Resolved, typed, immutable theme snapshot.
//!
//! `ResolvedTheme` is the single backend-neutral output of the pipeline.
//! It carries schema version, mode, scale context and token source
//! identity. It intentionally contains no GPUI/toolkit/DOM/wry types;
//! driver mappers consume it one-way into private style types.

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use crate::doc::DocumentLayer;

/// Color space of a resolved color. v1 supports sRGB only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    /// sRGB, components normalized to `0.0..=1.0`.
    Srgb,
}

/// A resolved color: color space + 3 components + alpha.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedColor {
    /// Color space.
    pub space: ColorSpace,
    /// `[r, g, b]` components, each `0.0..=1.0`.
    pub components: [f64; 3],
    /// Alpha `0.0..=1.0`.
    pub alpha: f64,
}

/// Length unit of a resolved length. Rem→px conversion is a mapper-side
/// concern (see `ScaleContext`); the theme keeps the declared unit.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LengthUnit {
    /// Absolute pixels.
    Px,
    /// Root-relative em.
    Rem,
}

/// Rounding hint for a resolved length, applied by the driver mapper
/// when converting to device units.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rounding {
    /// No rounding requested.
    None,
    /// Round to nearest integer.
    Nearest,
    /// Round down.
    Floor,
    /// Round up.
    Ceil,
}

/// A resolved length: magnitude + unit + rounding hint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedLength {
    /// Numeric magnitude.
    pub value: f64,
    /// Declared unit.
    pub unit: LengthUnit,
    /// Rounding hint for the mapper.
    pub rounding: Rounding,
}

/// Time unit of a resolved duration.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurationUnit {
    /// Milliseconds.
    Milliseconds,
    /// Seconds.
    Seconds,
}

/// A resolved duration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedDuration {
    /// Numeric magnitude.
    pub value: f64,
    /// Declared unit.
    pub unit: DurationUnit,
}

/// DTCG `strokeStyle` keyword values.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrokeStyle {
    /// `solid`
    Solid,
    /// `dashed`
    Dashed,
    /// `dotted`
    Dotted,
    /// `double`
    Double,
    /// `groove`
    Groove,
    /// `ridge`
    Ridge,
    /// `outset`
    Outset,
    /// `inset`
    Inset,
}

/// A typed resolved token value.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedValue {
    /// `color`
    Color(ResolvedColor),
    /// `dimension`
    Length(ResolvedLength),
    /// `fontFamily` (family list, primary first, fallbacks after)
    FontFamily(Vec<String>),
    /// `fontWeight` as numeric weight `1..=1000`
    FontWeight(u16),
    /// `number`
    Number(f64),
    /// `duration`
    Duration(ResolvedDuration),
    /// `cubicBezier` `[x1, y1, x2, y2]`
    CubicBezier([f64; 4]),
    /// `strokeStyle`
    StrokeStyle(StrokeStyle),
}

/// Scale context carried by the snapshot. Token values are unit-faithful;
/// the mapper applies `scale_factor` and `rem_base_px` when converting to
/// device pixels. Defaults: `scale_factor = 1.0`, `rem_base_px = 16.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScaleContext {
    /// UI scale factor (e.g. OS DPI scaling).
    pub scale_factor: f64,
    /// Pixel size used when a mapper converts `rem` lengths.
    pub rem_base_px: f64,
}

impl Default for ScaleContext {
    fn default() -> Self {
        Self {
            scale_factor: 1.0,
            rem_base_px: 16.0,
        }
    }
}

/// Identity of one input document that contributed to the snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenSource {
    /// Document layer.
    pub layer: DocumentLayer,
    /// Document `id` from its profile block.
    pub id: String,
    /// FNV-1a 64-bit fingerprint of the document's normalized token map.
    pub fingerprint: u64,
}

/// Immutable typed theme snapshot produced by `ThemeResolver::resolve`.
///
/// Values are stored in a `BTreeMap` keyed by dot-separated token path,
/// so iteration and `to_normalized_json` are deterministic.
#[derive(Debug, Clone)]
pub struct ResolvedTheme {
    schema_version: String,
    mode: String,
    scale: ScaleContext,
    sources: Vec<TokenSource>,
    tokens: BTreeMap<String, ResolvedValue>,
}

impl ResolvedTheme {
    pub(crate) fn new(
        mode: String,
        sources: Vec<TokenSource>,
        tokens: BTreeMap<String, ResolvedValue>,
    ) -> Self {
        Self {
            schema_version: crate::SCHEMA_VERSION.to_owned(),
            mode,
            scale: ScaleContext::default(),
            sources,
            tokens,
        }
    }

    /// Profile schema version this snapshot was resolved under.
    #[must_use]
    pub fn schema_version(&self) -> &str {
        &self.schema_version
    }

    /// Mode this snapshot was resolved for.
    #[must_use]
    pub fn mode(&self) -> &str {
        &self.mode
    }

    /// Scale context (mapper-side conversion hints).
    #[must_use]
    pub fn scale(&self) -> ScaleContext {
        self.scale
    }

    /// Copy of this snapshot with a different scale context.
    ///
    /// Token values are unchanged; only mapper-side conversion hints differ.
    #[must_use]
    pub fn with_scale(&self, scale: ScaleContext) -> Self {
        Self {
            schema_version: self.schema_version.clone(),
            mode: self.mode.clone(),
            scale,
            sources: self.sources.clone(),
            tokens: self.tokens.clone(),
        }
    }

    /// Input documents that contributed, in precedence order.
    #[must_use]
    pub fn sources(&self) -> &[TokenSource] {
        &self.sources
    }

    /// Iterate over `(token path, resolved value)` in path-sorted order.
    pub fn tokens(&self) -> impl Iterator<Item = (&str, &ResolvedValue)> {
        self.tokens.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Number of resolved tokens.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    /// Whether the snapshot has no tokens.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Look up a resolved token by path.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&ResolvedValue> {
        self.tokens.get(key)
    }

    /// Resolved color at `key`, if present and a color.
    #[must_use]
    pub fn color(&self, key: &str) -> Option<ResolvedColor> {
        match self.tokens.get(key) {
            Some(ResolvedValue::Color(c)) => Some(*c),
            _ => None,
        }
    }

    /// Resolved length at `key`, if present and a dimension.
    #[must_use]
    pub fn length(&self, key: &str) -> Option<ResolvedLength> {
        match self.tokens.get(key) {
            Some(ResolvedValue::Length(l)) => Some(*l),
            _ => None,
        }
    }

    /// Resolved font-family list at `key`, if present.
    #[must_use]
    pub fn font_family(&self, key: &str) -> Option<&[String]> {
        match self.tokens.get(key) {
            Some(ResolvedValue::FontFamily(f)) => Some(f),
            _ => None,
        }
    }

    /// Resolved numeric font weight at `key`, if present.
    #[must_use]
    pub fn font_weight(&self, key: &str) -> Option<u16> {
        match self.tokens.get(key) {
            Some(ResolvedValue::FontWeight(w)) => Some(*w),
            _ => None,
        }
    }

    /// Resolved number at `key`, if present.
    #[must_use]
    pub fn number(&self, key: &str) -> Option<f64> {
        match self.tokens.get(key) {
            Some(ResolvedValue::Number(n)) => Some(*n),
            _ => None,
        }
    }

    /// Resolved duration at `key`, if present.
    #[must_use]
    pub fn duration(&self, key: &str) -> Option<ResolvedDuration> {
        match self.tokens.get(key) {
            Some(ResolvedValue::Duration(d)) => Some(*d),
            _ => None,
        }
    }

    /// Deterministic canonical JSON of the snapshot (keys sorted),
    /// suitable for golden-file comparison.
    #[must_use]
    pub fn to_normalized_json(&self) -> String {
        let mut tokens = Map::new();
        for (key, value) in &self.tokens {
            tokens.insert(key.clone(), resolved_value_json(value));
        }
        let sources: Vec<Value> = self
            .sources
            .iter()
            .map(|s| {
                json!({
                    "layer": s.layer.as_str(),
                    "id": s.id,
                    "fingerprint": s.fingerprint,
                })
            })
            .collect();
        let root = json!({
            "schema": self.schema_version,
            "mode": self.mode,
            "scale": {
                "scale_factor": self.scale.scale_factor,
                "rem_base_px": self.scale.rem_base_px,
            },
            "sources": sources,
            "tokens": Value::Object(tokens),
        });
        serde_json::to_string_pretty(&root).unwrap_or_else(|_| "{}".to_owned())
    }
}

fn resolved_value_json(value: &ResolvedValue) -> Value {
    match value {
        ResolvedValue::Color(c) => json!({
            "type": "color",
            "space": "srgb",
            "components": c.components,
            "alpha": c.alpha,
        }),
        ResolvedValue::Length(l) => json!({
            "type": "dimension",
            "value": l.value,
            "unit": match l.unit {
                LengthUnit::Px => "px",
                LengthUnit::Rem => "rem",
            },
            "rounding": match l.rounding {
                Rounding::None => "none",
                Rounding::Nearest => "nearest",
                Rounding::Floor => "floor",
                Rounding::Ceil => "ceil",
            },
        }),
        ResolvedValue::FontFamily(f) => json!({ "type": "fontFamily", "value": f }),
        ResolvedValue::FontWeight(w) => json!({ "type": "fontWeight", "value": w }),
        ResolvedValue::Number(n) => json!({ "type": "number", "value": n }),
        ResolvedValue::Duration(d) => json!({
            "type": "duration",
            "value": d.value,
            "unit": match d.unit {
                DurationUnit::Milliseconds => "ms",
                DurationUnit::Seconds => "s",
            },
        }),
        ResolvedValue::CubicBezier(b) => json!({ "type": "cubicBezier", "value": b }),
        ResolvedValue::StrokeStyle(s) => json!({
            "type": "strokeStyle",
            "value": match s {
                StrokeStyle::Solid => "solid",
                StrokeStyle::Dashed => "dashed",
                StrokeStyle::Dotted => "dotted",
                StrokeStyle::Double => "double",
                StrokeStyle::Groove => "groove",
                StrokeStyle::Ridge => "ridge",
                StrokeStyle::Outset => "outset",
                StrokeStyle::Inset => "inset",
            },
        }),
    }
}
