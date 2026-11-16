//! Vendor-boundary codec: `materialize`/`dematerialize` between the
//! typed [`ScriptValue`] model and the neutral `{"$rv": …}` tag-record
//! representation a vendor adapter can transport.
//!
//! Tag records are [`ScriptValue::Record`] maps containing the
//! reserved [`TAG_KEY`] (`"$rv"`) key whose value is the tag name:
//!
//! | tag       | fields                                        | meaning |
//! | --------- | --------------------------------------------- | ------- |
//! | `"int"`   | `value: String` (decimal `i64`)               | `Integer` outside the `f64`-safe `2^53` range |
//! | `"time"`  | `domain`, `timebase: {num, den}`, `count`     | `Time` |
//! | `"res"`   | `domain`, `kind`, `value` (all strings)       | `ResourceId` |
//! | `"bytes"` | `hex: String`                                 | `Bytes` |
//! | `"number"`| `value: "nan" \| "inf" \| "-inf"`            | non-finite `Number` |
//! | `"null"`  | *(none)*                                      | explicit `Null` (vendor-produced form) |
//! | `"absent"`| *(none)*                                      | explicit `Absent` (vendor-produced form) |
//!
//! `materialize` emits tags only where the typed value cannot cross a
//! boundary safely as-is: `Integer` with `|i| > 2^53`, `Time`,
//! `ResourceId`, `Bytes`, and non-finite `Number`. Everything else —
//! including `Null`, `Absent`, and finite `Number` — passes through
//! structurally with recursive descent into arrays and records.
//!
//! `dematerialize` restores typed values and **fails closed**:
//! unknown tags, a non-string `"$rv"` value, missing or mistyped
//! fields, and malformed field contents (bad decimal, bad hex,
//! non-finite spellings other than the three defined) all produce
//! [`CodecError`]. A record containing `"$rv"` is always interpreted
//! as a tag record — user payloads must not use the reserved key.

use std::collections::BTreeMap;
use std::fmt;

use crate::canonical::{TAG_KEY, hex_decode, hex_encode};
use crate::id::{ResourceDomainId, TimeDomainId};
use crate::value::{Rational, ScriptValue};

/// `f64` safe-integer boundary: `Integer` values with
/// `unsigned_abs() > F64_SAFE_I64` cannot round-trip through an `f64`
/// vendor boundary and are transported as `{"$rv":"int","value":…}`.
pub const F64_SAFE_I64: i64 = 9_007_199_254_740_992; // 2^53

/// Errors produced by [`dematerialize`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CodecError {
    /// A `"$rv"` tag name the codec does not define.
    UnknownTag {
        /// The unrecognized tag string.
        tag: String,
    },
    /// A tag record whose shape does not match its tag's contract.
    MalformedTag {
        /// Human-readable reason.
        reason: String,
    },
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTag { tag } => write!(f, "unknown codec tag: {tag:?}"),
            Self::MalformedTag { reason } => write!(f, "malformed codec tag: {reason}"),
        }
    }
}

impl std::error::Error for CodecError {}

/// Returns `true` if `value` is a `Record` containing the reserved
/// [`TAG_KEY`] — i.e. the shape [`dematerialize`] interprets as a tag.
#[must_use]
pub fn is_tagged(value: &ScriptValue) -> bool {
    matches!(value, ScriptValue::Record(map) if map.contains_key(TAG_KEY))
}

/// Maps a typed [`ScriptValue`] to its vendor-neutral tagged form.
///
/// Infallible: every variant has a neutral representation. See the
/// module docs for which variants are tagged.
#[must_use]
pub fn materialize(value: &ScriptValue) -> ScriptValue {
    match value {
        ScriptValue::Integer(i) if i.unsigned_abs() > F64_SAFE_I64.unsigned_abs() => tag_record(
            "int",
            [("value".to_string(), ScriptValue::String(i.to_string()))]
                .into_iter()
                .collect(),
        ),
        ScriptValue::Time {
            domain,
            timebase,
            count,
        } => {
            let mut timebase_map = BTreeMap::new();
            timebase_map.insert("num".to_string(), ScriptValue::Integer(timebase.numerator));
            timebase_map.insert(
                "den".to_string(),
                ScriptValue::Integer(timebase.denominator),
            );
            tag_record(
                "time",
                [
                    (
                        "domain".to_string(),
                        ScriptValue::String(domain.as_str().to_string()),
                    ),
                    ("timebase".to_string(), ScriptValue::Record(timebase_map)),
                    ("count".to_string(), ScriptValue::Integer(*count)),
                ]
                .into_iter()
                .collect(),
            )
        }
        ScriptValue::ResourceId {
            domain,
            kind,
            value: id_value,
        } => tag_record(
            "res",
            [
                (
                    "domain".to_string(),
                    ScriptValue::String(domain.as_str().to_string()),
                ),
                ("kind".to_string(), ScriptValue::String(kind.clone())),
                ("value".to_string(), ScriptValue::String(id_value.clone())),
            ]
            .into_iter()
            .collect(),
        ),
        ScriptValue::Bytes(bytes) => tag_record(
            "bytes",
            [("hex".to_string(), ScriptValue::String(hex_encode(bytes)))]
                .into_iter()
                .collect(),
        ),
        ScriptValue::Number(n) if !n.is_finite() => {
            let spelling = if n.is_nan() {
                "nan"
            } else if n.is_sign_positive() {
                "inf"
            } else {
                "-inf"
            };
            tag_record(
                "number",
                [(
                    "value".to_string(),
                    ScriptValue::String(spelling.to_string()),
                )]
                .into_iter()
                .collect(),
            )
        }
        ScriptValue::Array(items) => ScriptValue::Array(items.iter().map(materialize).collect()),
        ScriptValue::Record(map) => ScriptValue::Record(
            map.iter()
                .map(|(k, v)| (k.clone(), materialize(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Restores a typed [`ScriptValue`] from its tagged form.
///
/// Recursively descends arrays and records; any record containing
/// [`TAG_KEY`] is decoded as a tag and fails closed on unknown or
/// malformed shapes.
///
/// # Errors
///
/// Returns [`CodecError::UnknownTag`] for undefined tag names and
/// [`CodecError::MalformedTag`] for shape violations.
pub fn dematerialize(value: ScriptValue) -> Result<ScriptValue, CodecError> {
    match value {
        ScriptValue::Record(ref map) if map.contains_key(TAG_KEY) => decode_tag(map),
        ScriptValue::Record(map) => Ok(ScriptValue::Record(
            map.into_iter()
                .map(|(k, v)| dematerialize(v).map(|dv| (k, dv)))
                .collect::<Result<BTreeMap<_, _>, _>>()?,
        )),
        ScriptValue::Array(items) => Ok(ScriptValue::Array(
            items
                .into_iter()
                .map(dematerialize)
                .collect::<Result<Vec<_>, _>>()?,
        )),
        other => Ok(other),
    }
}

fn tag_record(tag: &str, fields: BTreeMap<String, ScriptValue>) -> ScriptValue {
    let mut map = fields;
    map.insert(TAG_KEY.to_string(), ScriptValue::String(tag.to_string()));
    ScriptValue::Record(map)
}

fn decode_tag(map: &BTreeMap<String, ScriptValue>) -> Result<ScriptValue, CodecError> {
    let tag = match map.get(TAG_KEY) {
        Some(ScriptValue::String(s)) => s.clone(),
        _ => {
            return Err(CodecError::MalformedTag {
                reason: format!("{TAG_KEY:?} must map to a string tag name"),
            });
        }
    };
    // Known tags only — anything else fails closed. Every tag record
    // must contain exactly its declared fields plus the tag key —
    // extra or missing fields are malformed.
    let expected: &[&str] = match tag.as_str() {
        "int" | "number" => &["value"],
        "bytes" => &["hex"],
        "time" => &["count", "domain", "timebase"],
        "res" => &["domain", "kind", "value"],
        "null" | "absent" => &[],
        _ => return Err(CodecError::UnknownTag { tag }),
    };
    check_fields(map, &tag, expected)?;
    match tag.as_str() {
        "int" => {
            let text = string_field(map, "value")?;
            let value: i64 = text.parse().map_err(|_| CodecError::MalformedTag {
                reason: format!("int tag value is not a valid i64: {text:?}"),
            })?;
            Ok(ScriptValue::Integer(value))
        }
        "bytes" => {
            let hex = string_field(map, "hex")?;
            let bytes = hex_decode(&hex).map_err(|reason| CodecError::MalformedTag {
                reason: format!("bytes tag hex is invalid: {reason}"),
            })?;
            Ok(ScriptValue::Bytes(bytes))
        }
        "number" => {
            let spelling = string_field(map, "value")?;
            match spelling.as_str() {
                "nan" => Ok(ScriptValue::Number(f64::NAN)),
                "inf" => Ok(ScriptValue::Number(f64::INFINITY)),
                "-inf" => Ok(ScriptValue::Number(f64::NEG_INFINITY)),
                other => Err(CodecError::MalformedTag {
                    reason: format!("number tag value must be nan|inf|-inf, got {other:?}"),
                }),
            }
        }
        "time" => {
            let domain = TimeDomainId::new(string_field(map, "domain")?);
            let count = integer_field(map, "count")?;
            let timebase = match map.get("timebase") {
                Some(ScriptValue::Record(tb)) => {
                    let num = match tb.get("num") {
                        Some(ScriptValue::Integer(n)) => *n,
                        _ => {
                            return Err(CodecError::MalformedTag {
                                reason: "time tag timebase.num must be an integer".to_string(),
                            });
                        }
                    };
                    let den = match tb.get("den") {
                        Some(ScriptValue::Integer(n)) => *n,
                        _ => {
                            return Err(CodecError::MalformedTag {
                                reason: "time tag timebase.den must be an integer".to_string(),
                            });
                        }
                    };
                    if tb.len() != 2 {
                        return Err(CodecError::MalformedTag {
                            reason: "time tag timebase must contain exactly num and den"
                                .to_string(),
                        });
                    }
                    Rational::new(num, den)
                }
                _ => {
                    return Err(CodecError::MalformedTag {
                        reason: "time tag timebase must be a record".to_string(),
                    });
                }
            };
            Ok(ScriptValue::Time {
                domain,
                timebase,
                count,
            })
        }
        "res" => Ok(ScriptValue::ResourceId {
            domain: ResourceDomainId::new(string_field(map, "domain")?),
            kind: string_field(map, "kind")?,
            value: string_field(map, "value")?,
        }),
        "null" => Ok(ScriptValue::Null),
        "absent" => Ok(ScriptValue::Absent),
        _ => unreachable!("tag validated above"),
    }
}

/// Verifies the tag record contains exactly `TAG_KEY` plus the
/// declared fields — no missing, no extras.
fn check_fields(
    map: &BTreeMap<String, ScriptValue>,
    tag: &str,
    expected: &[&str],
) -> Result<(), CodecError> {
    let mut want: Vec<&str> = expected.to_vec();
    want.push(TAG_KEY);
    want.sort_unstable();
    let mut have: Vec<&str> = map.keys().map(String::as_str).collect();
    have.sort_unstable();
    if have != want {
        return Err(CodecError::MalformedTag {
            reason: format!("{tag:?} tag fields must be exactly {want:?}, got {have:?}"),
        });
    }
    Ok(())
}

fn string_field(map: &BTreeMap<String, ScriptValue>, key: &str) -> Result<String, CodecError> {
    match map.get(key) {
        Some(ScriptValue::String(s)) => Ok(s.clone()),
        _ => Err(CodecError::MalformedTag {
            reason: format!("tag field {key:?} must be a string"),
        }),
    }
}

fn integer_field(map: &BTreeMap<String, ScriptValue>, key: &str) -> Result<i64, CodecError> {
    match map.get(key) {
        Some(ScriptValue::Integer(n)) => Ok(*n),
        _ => Err(CodecError::MalformedTag {
            reason: format!("tag field {key:?} must be an integer"),
        }),
    }
}
