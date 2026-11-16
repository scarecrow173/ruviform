//! Canonical serialization and SHA-256 digest utilities — the single
//! authoritative canonical representation for [`ScriptValue`].
//!
//! Canonical rules (ported from `poc/script-coexist`, Issue #559):
//!
//! - `Record` keys are sorted by UTF-8 byte order; `Absent` fields are
//!   omitted. `Array` order is preserved.
//! - Strings use JSON escaping (via `serde_json`, internal use only).
//! - `Integer` renders as decimal `i64`. `Number` normalizes through
//!   [`canonical_number`]: integral `f64` values render without a
//!   decimal point (so `Number(1.0)` and `Integer(1)` share a digest),
//!   `-0.0` renders as `0`, and NaN/±inf render as deterministic
//!   `{"$rv":"number",…}` tag records.
//! - [`ScriptValue::Time`], [`ScriptValue::ResourceId`], and
//!   [`ScriptValue::Bytes`] render in the same `{"$rv": …}` tag-record
//!   structure the codec produces, so `canonical_value(v)` equals
//!   `canonical_value(materialize(v))` for every variant except
//!   `Integer` values outside the `f64`-safe `2^53` range (typed
//!   canonical keeps the exact decimal; the codec tag preserves it
//!   across vendor boundaries instead).
//! - [`ScriptValue::Absent`] has no canonical form — not at the top
//!   level and not inside an array (record fields omit it before
//!   recursion).
//!
//! Digests are `"sha256:v1:<64 lowercase hex>"` over the UTF-8 bytes of
//! the canonical string. Equal canonical bytes ⇒ equal digest; the
//! digest commits to meaning, not surface formatting.

use std::collections::BTreeMap;
use std::fmt;

use sha2::{Digest, Sha256};

use crate::value::{DefinitionDigest, ScriptValue, SourceDigest};

/// Reserved record key marking a codec tag record. A [`ScriptValue`]
/// `Record` containing this key is interpreted as a tagged value by
/// [`crate::dematerialize`]; user payloads must not use it.
pub const TAG_KEY: &str = "$rv";

/// Errors produced while canonicalizing a [`ScriptValue`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CanonicalError {
    /// [`ScriptValue::Absent`] was reached — it has no canonical form.
    Absent,
    /// The value could not be represented canonically.
    Unsupported {
        /// Human-readable reason.
        reason: String,
    },
}

impl fmt::Display for CanonicalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Absent => f.write_str("Absent has no canonical form"),
            Self::Unsupported { reason } => {
                write!(f, "value is not canonicalizable: {reason}")
            }
        }
    }
}

impl std::error::Error for CanonicalError {}

/// Serializes `value` to its canonical string form.
///
/// # Errors
///
/// Returns [`CanonicalError::Absent`] when an [`ScriptValue::Absent`]
/// is reached (top level or inside an array) and
/// [`CanonicalError::Unsupported`] if a string cannot be JSON-escaped
/// (in practice unreachable).
pub fn canonical_value(value: &ScriptValue) -> Result<String, CanonicalError> {
    let mut out = String::new();
    write_value(value, &mut out)?;
    Ok(out)
}

/// Computes the [`DefinitionDigest`] of a typed payload value — the
/// digest committed to by [`crate::ScriptArtifact`].
///
/// # Errors
///
/// Same conditions as [`canonical_value`].
pub fn definition_digest(value: &ScriptValue) -> Result<DefinitionDigest, CanonicalError> {
    let canonical = canonical_value(value)?;
    Ok(DefinitionDigest::from_sha256_hex(&sha256_hex(
        canonical.as_bytes(),
    )))
}

/// Computes the [`SourceDigest`] of UTF-8 source text.
#[must_use]
pub fn source_digest(text: &str) -> SourceDigest {
    SourceDigest::from_sha256_hex(&sha256_hex(text.as_bytes()))
}

/// Lowercase hex SHA-256 of `bytes`.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex_encode(&Sha256::digest(bytes))
}

/// Lowercase hex encoding of `bytes`.
#[must_use]
pub fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(char::from(HEX[usize::from(b >> 4)]));
        out.push(char::from(HEX[usize::from(b & 0x0f)]));
    }
    out
}

/// Decodes hex (either case) into bytes.
///
/// # Errors
///
/// Returns `Err(reason)` on odd length or a non-hex digit.
pub fn hex_decode(input: &str) -> Result<Vec<u8>, String> {
    let bytes = input.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        return Err("odd-length hex string".to_string());
    }
    let nibble = |b: u8| -> Result<u8, String> {
        match b {
            b'0'..=b'9' => Ok(b - b'0'),
            b'a'..=b'f' => Ok(b - b'a' + 10),
            b'A'..=b'F' => Ok(b - b'A' + 10),
            _ => Err(format!("invalid hex digit: {b}")),
        }
    };
    bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| Ok((nibble(pair[0])? << 4) | nibble(pair[1])?))
        .collect()
}

fn write_value(value: &ScriptValue, out: &mut String) -> Result<(), CanonicalError> {
    match value {
        ScriptValue::Absent => Err(CanonicalError::Absent),
        ScriptValue::Null => {
            out.push_str("null");
            Ok(())
        }
        ScriptValue::Bool(b) => {
            out.push_str(if *b { "true" } else { "false" });
            Ok(())
        }
        ScriptValue::Integer(i) => {
            out.push_str(&i.to_string());
            Ok(())
        }
        ScriptValue::Number(n) => {
            out.push_str(&canonical_number(*n));
            Ok(())
        }
        ScriptValue::String(s) => {
            let escaped = serde_json::to_string(s).map_err(|e| CanonicalError::Unsupported {
                reason: e.to_string(),
            })?;
            out.push_str(&escaped);
            Ok(())
        }
        ScriptValue::Bytes(_) | ScriptValue::Time { .. } | ScriptValue::ResourceId { .. } => {
            // Canonical form equals the codec tag record so that
            // canonical_value(v) == canonical_value(materialize(v)).
            // `materialize` maps these variants to a `Record` containing
            // only canonical-safe leaves, so recursion terminates.
            let tagged = crate::codec::materialize(value);
            write_value(&tagged, out)
        }
        ScriptValue::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(item, out)?;
            }
            out.push(']');
            Ok(())
        }
        ScriptValue::Record(map) => write_record(map, out),
    }
}

fn write_record(
    map: &BTreeMap<String, ScriptValue>,
    out: &mut String,
) -> Result<(), CanonicalError> {
    out.push('{');
    let mut first = true;
    for (key, val) in map {
        if val.is_absent() {
            continue;
        }
        if !first {
            out.push(',');
        }
        first = false;
        let escaped = serde_json::to_string(key).map_err(|e| CanonicalError::Unsupported {
            reason: e.to_string(),
        })?;
        out.push_str(&escaped);
        out.push(':');
        write_value(val, out)?;
    }
    out.push('}');
    Ok(())
}

/// Renders an `f64` in canonical numeric form.
///
/// - `±0.0` renders as `0` (detected via raw bits so no float `==`
///   comparison is needed).
/// - Non-finite values render as a `{"$rv":"number",…}` tag record —
///   a quoted token would collide with [`ScriptValue::String`], and a
///   bare `NaN`/`inf` is not valid JSON number syntax.
/// - All other finite values render via Rust's shortest round-trip
///   `f64` `Display`, which is deterministic for a given IEEE-754
///   value and prints integral values without a decimal point — so
///   `Number(1.0)` and `Integer(1)` share the canonical text `1` (and
///   therefore a digest) with no special-casing.
#[must_use]
pub fn canonical_number(n: f64) -> String {
    if n.is_nan() {
        return "{\"$rv\":\"number\",\"value\":\"nan\"}".to_string();
    }
    if n.is_infinite() {
        return if n.is_sign_positive() {
            "{\"$rv\":\"number\",\"value\":\"inf\"}".to_string()
        } else {
            "{\"$rv\":\"number\",\"value\":\"-inf\"}".to_string()
        };
    }
    // `to_bits() << 1` clears the sign bit: zero iff the value is
    // either +0.0 or -0.0.
    if n.to_bits() << 1 == 0 {
        return "0".to_string();
    }
    format!("{n}")
}
