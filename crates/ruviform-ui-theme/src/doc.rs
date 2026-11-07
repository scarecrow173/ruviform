//! Token document model: profile metadata + validated raw token tree.
//!
//! Parsing is strict per `rvf-dtcg-profile.v1`: unknown `$`-prefixed keys
//! are `ReservedKey` errors, unknown `$type` values are `UnknownTokenType`
//! errors, and non-`$` unknown keys / unknown `$extensions` entries are
//! ignored. `serde_json::Map` iterates keys in sorted order, so the first
//! reported error is deterministic.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::errors::TokenError;
use crate::snapshot::{
    ColorSpace, DurationUnit, LengthUnit, ResolvedColor, ResolvedDuration, ResolvedLength,
    ResolvedValue, Rounding, StrokeStyle,
};

/// Precedence layer of a token document.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentLayer {
    /// Base palette/units. Lowest precedence.
    Primitive,
    /// Common semantic tokens.
    Semantic,
    /// Common component tokens.
    Component,
    /// Consumer product/theme explicit overrides. Highest precedence.
    Product,
}

impl DocumentLayer {
    /// Wire name used in the profile block and normalized output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Primitive => "primitive",
            Self::Semantic => "semantic",
            Self::Component => "component",
            Self::Product => "product",
        }
    }

    /// Precedence rank; lower wins less (applied in ascending order).
    pub(crate) const fn rank(self) -> u8 {
        match self {
            Self::Primitive => 0,
            Self::Semantic => 1,
            Self::Component => 2,
            Self::Product => 3,
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        match name {
            "primitive" => Some(Self::Primitive),
            "semantic" => Some(Self::Semantic),
            "component" => Some(Self::Component),
            "product" => Some(Self::Product),
            _ => None,
        }
    }
}

/// Adopted v1 `$type` set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenType {
    Color,
    Dimension,
    FontFamily,
    FontWeight,
    Number,
    Duration,
    CubicBezier,
    StrokeStyle,
}

impl TokenType {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Color => "color",
            Self::Dimension => "dimension",
            Self::FontFamily => "fontFamily",
            Self::FontWeight => "fontWeight",
            Self::Number => "number",
            Self::Duration => "duration",
            Self::CubicBezier => "cubicBezier",
            Self::StrokeStyle => "strokeStyle",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        match name {
            "color" => Some(Self::Color),
            "dimension" => Some(Self::Dimension),
            "fontFamily" => Some(Self::FontFamily),
            "fontWeight" => Some(Self::FontWeight),
            "number" => Some(Self::Number),
            "duration" => Some(Self::Duration),
            "cubicBezier" => Some(Self::CubicBezier),
            "strokeStyle" => Some(Self::StrokeStyle),
            _ => None,
        }
    }
}

/// Validated token value; either literal or a single whole-value alias.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RawValue {
    /// Validated literal value.
    Literal(ResolvedValue),
    /// `"{path.to.token}"` reference, resolved after merge.
    Alias(String),
}

/// One validated token: declared type + base value + per-mode values.
#[derive(Debug, Clone)]
pub(crate) struct RawToken {
    pub(crate) ty: TokenType,
    pub(crate) value: RawValue,
    /// `$extensions."dev.ruviform.modes"` — mode-scoped alternative values.
    pub(crate) modes: BTreeMap<String, RawValue>,
}

/// One parsed DTCG token document with its profile identity.
#[derive(Debug, Clone)]
pub struct TokenDocument {
    layer: DocumentLayer,
    id: String,
    modes: Vec<String>,
    pub(crate) tokens: BTreeMap<String, RawToken>,
    pub(crate) fingerprint: u64,
}

const PROFILE_EXT: &str = "dev.ruviform.dtcg-profile";
const MODES_EXT: &str = "dev.ruviform.modes";
const ROUNDING_EXT: &str = "dev.ruviform.rounding";
const KNOWN_DOLLAR_KEYS: [&str; 4] = ["$type", "$value", "$description", "$extensions"];

impl TokenDocument {
    /// Parse and validate a DTCG JSON document.
    ///
    /// # Errors
    ///
    /// `TokenError` on malformed JSON, missing/invalid profile block,
    /// unknown layer/`$type`, invalid values, or reserved `$` keys.
    pub fn from_json(json: &str) -> Result<Self, TokenError> {
        let root: Value = serde_json::from_str(json).map_err(|e| TokenError::MalformedJson {
            line: e.line(),
            column: e.column(),
            message: e.to_string(),
        })?;
        let obj = root.as_object().ok_or_else(|| TokenError::InvalidValue {
            path: "$".to_owned(),
            reason: "root must be a JSON object".to_owned(),
        })?;
        let (layer, id, modes) = parse_profile(obj)?;
        check_reserved_keys(obj, "$")?;

        let mut tokens = BTreeMap::new();
        let root_ty = group_type(obj, "$", None)?;
        for (key, child) in obj {
            if key.starts_with('$') {
                continue;
            }
            walk_node(child, key, root_ty, &mut tokens)?;
        }
        let fingerprint = fingerprint_tokens(&tokens);
        Ok(Self {
            layer,
            id,
            modes,
            tokens,
            fingerprint,
        })
    }

    /// Document precedence layer.
    #[must_use]
    pub const fn layer(&self) -> DocumentLayer {
        self.layer
    }

    /// Document identity from its profile block.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Modes declared by this document (`[]` = mode-agnostic).
    #[must_use]
    pub fn modes(&self) -> &[String] {
        &self.modes
    }

    /// Number of tokens defined in this document.
    #[must_use]
    pub fn token_count(&self) -> usize {
        self.tokens.len()
    }
}

fn parse_profile(
    obj: &Map<String, Value>,
) -> Result<(DocumentLayer, String, Vec<String>), TokenError> {
    let profile = obj
        .get("$extensions")
        .and_then(Value::as_object)
        .and_then(|e| e.get(PROFILE_EXT))
        .and_then(Value::as_object)
        .ok_or(TokenError::MissingProfile)?;

    let found = profile
        .get("profile")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if found != crate::SCHEMA_VERSION {
        return Err(TokenError::InvalidProfile {
            found: found.to_owned(),
        });
    }

    let layer_name = profile
        .get("layer")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let layer = DocumentLayer::from_name(layer_name).ok_or_else(|| TokenError::UnknownLayer {
        found: layer_name.to_owned(),
    })?;

    let id = profile
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if id.is_empty() {
        return Err(TokenError::InvalidValue {
            path: format!("$extensions.{PROFILE_EXT}.id"),
            reason: "document id must be a non-empty string".to_owned(),
        });
    }

    let mut modes = Vec::new();
    if let Some(raw) = profile.get("modes") {
        let list = raw.as_array().ok_or_else(|| TokenError::InvalidValue {
            path: format!("$extensions.{PROFILE_EXT}.modes"),
            reason: "modes must be an array of strings".to_owned(),
        })?;
        for m in list {
            let name = m.as_str().ok_or_else(|| TokenError::InvalidValue {
                path: format!("$extensions.{PROFILE_EXT}.modes"),
                reason: "modes must be an array of strings".to_owned(),
            })?;
            modes.push(name.to_owned());
        }
    }
    Ok((layer, id.to_owned(), modes))
}

fn check_reserved_keys(obj: &Map<String, Value>, path: &str) -> Result<(), TokenError> {
    for key in obj.keys() {
        if key.starts_with('$') && !KNOWN_DOLLAR_KEYS.contains(&key.as_str()) {
            return Err(TokenError::ReservedKey {
                path: path.to_owned(),
                key: key.clone(),
            });
        }
    }
    Ok(())
}

/// Effective `$type` declared on this node (or inherited).
fn group_type(
    obj: &Map<String, Value>,
    path: &str,
    inherited: Option<TokenType>,
) -> Result<Option<TokenType>, TokenError> {
    match obj.get("$type") {
        None => Ok(inherited),
        Some(v) => {
            let name = v.as_str().ok_or_else(|| TokenError::InvalidValue {
                path: path.to_owned(),
                reason: "$type must be a string".to_owned(),
            })?;
            TokenType::from_name(name)
                .map(Some)
                .ok_or_else(|| TokenError::UnknownTokenType {
                    path: path.to_owned(),
                    found: name.to_owned(),
                })
        }
    }
}

fn walk_node(
    node: &Value,
    path: &str,
    inherited: Option<TokenType>,
    tokens: &mut BTreeMap<String, RawToken>,
) -> Result<(), TokenError> {
    let obj = node.as_object().ok_or_else(|| TokenError::InvalidValue {
        path: path.to_owned(),
        reason: "token/group must be a JSON object".to_owned(),
    })?;
    check_reserved_keys(obj, path)?;
    let ty = group_type(obj, path, inherited)?;
    if obj.contains_key("$value") {
        let ty = ty.ok_or_else(|| TokenError::InvalidValue {
            path: path.to_owned(),
            reason: "missing $type (no inherited group type)".to_owned(),
        })?;
        let rounding = parse_rounding(obj, path)?;
        let value = parse_value(
            ty,
            obj.get("$value").unwrap_or(&Value::Null),
            path,
            rounding,
        )?;
        let modes = parse_modes(obj, path, ty, rounding)?;
        if tokens
            .insert(path.to_owned(), RawToken { ty, value, modes })
            .is_some()
        {
            // Reachable only when a `.`-containing JSON key collides with a
            // nested group path.
            return Err(invalid(path, "duplicate token path"));
        }
        return Ok(());
    }
    for (key, child) in obj {
        if key.starts_with('$') {
            continue;
        }
        walk_node(child, &format!("{path}.{key}"), ty, tokens)?;
    }
    Ok(())
}

fn parse_rounding(obj: &Map<String, Value>, path: &str) -> Result<Rounding, TokenError> {
    let Some(ext) = obj.get("$extensions").and_then(Value::as_object) else {
        return Ok(Rounding::None);
    };
    match ext.get(ROUNDING_EXT).and_then(Value::as_str) {
        None | Some("none") => Ok(Rounding::None),
        Some("nearest") => Ok(Rounding::Nearest),
        Some("floor") => Ok(Rounding::Floor),
        Some("ceil") => Ok(Rounding::Ceil),
        Some(other) => Err(TokenError::InvalidValue {
            path: path.to_owned(),
            reason: format!("unknown rounding {other:?}"),
        }),
    }
}

fn parse_modes(
    obj: &Map<String, Value>,
    path: &str,
    ty: TokenType,
    rounding: Rounding,
) -> Result<BTreeMap<String, RawValue>, TokenError> {
    let mut modes = BTreeMap::new();
    let Some(ext) = obj.get("$extensions").and_then(Value::as_object) else {
        return Ok(modes);
    };
    let Some(raw) = ext.get(MODES_EXT) else {
        return Ok(modes);
    };
    let map = raw.as_object().ok_or_else(|| TokenError::InvalidValue {
        path: path.to_owned(),
        reason: format!("{MODES_EXT} must be an object"),
    })?;
    for (mode, v) in map {
        modes.insert(
            mode.clone(),
            parse_value(ty, v, &format!("{path} (mode {mode})"), rounding)?,
        );
    }
    Ok(modes)
}

fn parse_value(
    ty: TokenType,
    v: &Value,
    path: &str,
    rounding: Rounding,
) -> Result<RawValue, TokenError> {
    if let Some(s) = v.as_str() {
        if let Some(reference) = alias_reference(s) {
            return Ok(RawValue::Alias(reference));
        }
        // Any other braced string is an attempted (unsupported) reference —
        // fail strictly so partial-embedded aliases like "{a}-{b}px" cannot
        // silently become literals for string-typed values.
        if s.contains('{') || s.contains('}') {
            return Err(invalid(
                path,
                "malformed alias (v1 supports whole-value \"{path.to.token}\" only)",
            ));
        }
    }
    let literal = match ty {
        TokenType::Color => parse_color(v, path)?,
        TokenType::Dimension => parse_dimension(v, path, rounding)?,
        TokenType::FontFamily => parse_font_family(v, path)?,
        TokenType::FontWeight => parse_font_weight(v, path)?,
        TokenType::Number => parse_number(v, path)?,
        TokenType::Duration => parse_duration(v, path)?,
        TokenType::CubicBezier => parse_bezier(v, path)?,
        TokenType::StrokeStyle => parse_stroke(v, path)?,
    };
    Ok(RawValue::Literal(literal))
}

/// `"{path.to.token}"` whole-value alias; partial embedded references are
/// out of v1 scope.
fn alias_reference(s: &str) -> Option<String> {
    let inner = s.strip_prefix('{')?.strip_suffix('}')?;
    if inner.is_empty() || inner.contains(['{', '}']) || inner.contains(char::is_whitespace) {
        return None;
    }
    Some(inner.to_owned())
}

fn invalid(path: &str, reason: &str) -> TokenError {
    TokenError::InvalidValue {
        path: path.to_owned(),
        reason: reason.to_owned(),
    }
}

fn finite_f64(v: &Value, path: &str, what: &str) -> Result<f64, TokenError> {
    let n = v
        .as_f64()
        .ok_or_else(|| invalid(path, &format!("{what} must be a number")))?;
    if !n.is_finite() {
        return Err(invalid(path, &format!("{what} must be finite")));
    }
    Ok(n)
}

fn parse_color(v: &Value, path: &str) -> Result<ResolvedValue, TokenError> {
    if let Some(text) = v.as_str() {
        let (red, green, blue, alpha) = parse_hex_color(text).ok_or_else(|| {
            invalid(
                path,
                "color must be #rgb, #rrggbb, #rrggbbaa, or a colorSpace object",
            )
        })?;
        return Ok(ResolvedValue::Color(ResolvedColor {
            space: ColorSpace::Srgb,
            components: [red, green, blue],
            alpha,
        }));
    }
    let obj = v
        .as_object()
        .ok_or_else(|| invalid(path, "color must be a hex string or a colorSpace object"))?;
    if obj.get("colorSpace").and_then(Value::as_str) != Some("srgb") {
        return Err(invalid(path, "colorSpace must be \"srgb\""));
    }
    let comps = obj
        .get("components")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid(path, "components must be an array of 3 numbers"))?;
    if comps.len() != 3 {
        return Err(invalid(path, "components must contain exactly 3 numbers"));
    }
    let mut components = [0.0; 3];
    for (slot, c) in components.iter_mut().zip(comps.iter()) {
        let n = finite_f64(c, path, "color component")?;
        if !(0.0..=1.0).contains(&n) {
            return Err(invalid(path, "color component must be in 0.0..=1.0"));
        }
        *slot = n;
    }
    let alpha = match obj.get("alpha") {
        None => 1.0,
        Some(a) => {
            let n = finite_f64(a, path, "alpha")?;
            if !(0.0..=1.0).contains(&n) {
                return Err(invalid(path, "alpha must be in 0.0..=1.0"));
            }
            n
        }
    };
    Ok(ResolvedValue::Color(ResolvedColor {
        space: ColorSpace::Srgb,
        components,
        alpha,
    }))
}

fn parse_hex_color(text: &str) -> Option<(f64, f64, f64, f64)> {
    let hex = text.strip_prefix('#')?;
    let digit = |i: usize| u8::from_str_radix(&hex[i..=i], 16).ok();
    let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    let (red, green, blue, alpha) = match hex.len() {
        3 => (digit(0)? * 0x11, digit(1)? * 0x11, digit(2)? * 0x11, 0xFF),
        6 => (byte(0)?, byte(2)?, byte(4)?, 0xFF),
        8 => (byte(0)?, byte(2)?, byte(4)?, byte(6)?),
        _ => return None,
    };
    Some((
        f64::from(red) / 255.0,
        f64::from(green) / 255.0,
        f64::from(blue) / 255.0,
        f64::from(alpha) / 255.0,
    ))
}

fn parse_dimension(v: &Value, path: &str, rounding: Rounding) -> Result<ResolvedValue, TokenError> {
    let obj = v
        .as_object()
        .ok_or_else(|| invalid(path, "dimension must be {value, unit}"))?;
    let value = finite_f64(
        obj.get("value").unwrap_or(&Value::Null),
        path,
        "dimension value",
    )?;
    let unit = match obj.get("unit").and_then(Value::as_str) {
        Some("px") => LengthUnit::Px,
        Some("rem") => LengthUnit::Rem,
        _ => return Err(invalid(path, "dimension unit must be \"px\" or \"rem\"")),
    };
    Ok(ResolvedValue::Length(ResolvedLength {
        value,
        unit,
        rounding,
    }))
}

fn parse_font_family(v: &Value, path: &str) -> Result<ResolvedValue, TokenError> {
    let families: Vec<String> = if let Some(s) = v.as_str() {
        vec![s.to_owned()]
    } else if let Some(list) = v.as_array() {
        let mut out = Vec::with_capacity(list.len());
        for f in list {
            let name = f
                .as_str()
                .ok_or_else(|| invalid(path, "fontFamily entries must be strings"))?;
            out.push(name.to_owned());
        }
        out
    } else {
        return Err(invalid(
            path,
            "fontFamily must be a string or array of strings",
        ));
    };
    if families.is_empty() || families.iter().any(String::is_empty) {
        return Err(invalid(path, "fontFamily must not be empty"));
    }
    Ok(ResolvedValue::FontFamily(families))
}

fn parse_font_weight(v: &Value, path: &str) -> Result<ResolvedValue, TokenError> {
    let weight: u16 = if let Some(n) = v.as_u64() {
        u16::try_from(n)
            .ok()
            .filter(|w| (1..=1000).contains(w))
            .ok_or_else(|| invalid(path, "fontWeight must be an integer in 1..=1000"))?
    } else if let Some(name) = v.as_str() {
        named_font_weight(name).ok_or_else(|| invalid(path, "unknown fontWeight name"))?
    } else {
        return Err(invalid(
            path,
            "fontWeight must be an integer or a named weight",
        ));
    };
    Ok(ResolvedValue::FontWeight(weight))
}

fn named_font_weight(name: &str) -> Option<u16> {
    Some(match name {
        "thin" | "hairline" => 100,
        "extra-light" | "ultra-light" => 200,
        "light" => 300,
        "normal" | "regular" => 400,
        "medium" => 500,
        "semi-bold" | "demi-bold" => 600,
        "bold" => 700,
        "extra-bold" | "ultra-bold" => 800,
        "black" => 900,
        "extra-black" | "ultra-black" => 950,
        _ => return None,
    })
}

fn parse_number(v: &Value, path: &str) -> Result<ResolvedValue, TokenError> {
    Ok(ResolvedValue::Number(finite_f64(v, path, "number")?))
}

fn parse_duration(v: &Value, path: &str) -> Result<ResolvedValue, TokenError> {
    let obj = v
        .as_object()
        .ok_or_else(|| invalid(path, "duration must be {value, unit}"))?;
    let value = finite_f64(
        obj.get("value").unwrap_or(&Value::Null),
        path,
        "duration value",
    )?;
    let unit = match obj.get("unit").and_then(Value::as_str) {
        Some("ms") => DurationUnit::Milliseconds,
        Some("s") => DurationUnit::Seconds,
        _ => return Err(invalid(path, "duration unit must be \"ms\" or \"s\"")),
    };
    Ok(ResolvedValue::Duration(ResolvedDuration { value, unit }))
}

fn parse_bezier(v: &Value, path: &str) -> Result<ResolvedValue, TokenError> {
    let list = v
        .as_array()
        .ok_or_else(|| invalid(path, "cubicBezier must be an array of 4 numbers"))?;
    if list.len() != 4 {
        return Err(invalid(path, "cubicBezier must contain exactly 4 numbers"));
    }
    let mut out = [0.0; 4];
    for (slot, c) in out.iter_mut().zip(list.iter()) {
        *slot = finite_f64(c, path, "cubicBezier component")?;
    }
    Ok(ResolvedValue::CubicBezier(out))
}

fn parse_stroke(v: &Value, path: &str) -> Result<ResolvedValue, TokenError> {
    let style = match v.as_str() {
        Some("solid") => StrokeStyle::Solid,
        Some("dashed") => StrokeStyle::Dashed,
        Some("dotted") => StrokeStyle::Dotted,
        Some("double") => StrokeStyle::Double,
        Some("groove") => StrokeStyle::Groove,
        Some("ridge") => StrokeStyle::Ridge,
        Some("outset") => StrokeStyle::Outset,
        Some("inset") => StrokeStyle::Inset,
        _ => return Err(invalid(path, "unknown strokeStyle")),
    };
    Ok(ResolvedValue::StrokeStyle(style))
}

/// FNV-1a 64-bit fingerprint over the canonical token serialization.
/// No external hash dependency; used for `TokenSource` identity only.
fn fingerprint_tokens(tokens: &BTreeMap<String, RawToken>) -> u64 {
    let mut canonical = Map::new();
    for (path, token) in tokens {
        canonical.insert(
            path.clone(),
            serde_json::json!({
                "type": token.ty.as_str(),
                "value": raw_value_json(&token.value),
                "modes": token.modes.iter().map(|(m, v)| (m.clone(), raw_value_json(v))).collect::<Map<String, Value>>(),
            }),
        );
    }
    let text = serde_json::to_string(&Value::Object(canonical)).unwrap_or_else(|_| String::new());
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.as_bytes() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn raw_value_json(v: &RawValue) -> Value {
    match v {
        RawValue::Literal(rv) => serde_json::json!({ "literal": format!("{rv:?}") }),
        RawValue::Alias(target) => serde_json::json!({ "alias": target }),
    }
}
