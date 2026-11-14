//! canonical serialization + sha256 digest（契約設計 C3 `ScriptArtifact`
//! の「canonical form・数値正規化・同一 digest」要件の PoC 実装）。
//!
//! 規則（design.md C3' に対応）:
//! - Record: key を UTF-8 バイト順ソート、`"key":value` を `,` 連結。
//! - Array: `[v,…]`。String: JSON string escape（serde_json 準拠）。
//! - Bool: `true`/`false`。Null: `null`。Absent: Record 内では key を
//!   出さない（= 欠落）。トップレベルの Absent は直列化しない。
//! - Integer: 10 進。Number: `v==trunc && |v|<=2^53` → 整数 10 進に
//!   正規化（backend 間の数値表現差を吸収）。それ以外は `{:e}` 最短表現。
//!   NaN→`nan`、±Inf→`inf`/`-inf`、-0.0→`0`。
//! - tag 値（Time/ResourceId/Bytes/Null）は codec と同じ `{"$rv":…}`
//!   構造を canonical key で展開し、i64 は 10 進文字列。

use sha2::{Digest, Sha256};

use crate::value::{DefinitionDigest, ScriptValue, SourceDigest};

/// tag Record の予約 key（codec と canonical で共有）。
pub const TAG_KEY: &str = "$rv";

/// `ScriptValue` を canonical バイト列（UTF-8）へ直列化する。
/// `Absent` を含む Record ではその key を省略する。
pub fn canonical_value(v: &ScriptValue) -> Result<String, String> {
    let mut out = String::new();
    write_value(v, &mut out)?;
    Ok(out)
}

fn write_value(v: &ScriptValue, out: &mut String) -> Result<(), String> {
    match v {
        ScriptValue::Absent => return Err("cannot canonicalize top-level Absent".to_owned()),
        ScriptValue::Null => out.push_str("null"),
        ScriptValue::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        ScriptValue::Integer(i) => out.push_str(&i.to_string()),
        ScriptValue::Number(f) => out.push_str(&canonical_number(*f)),
        ScriptValue::String(s) => {
            out.push_str(&serde_json::to_string(s).map_err(|e| e.to_string())?);
        }
        ScriptValue::Bytes(b) => {
            // codec の hex tag と同じ構造で canonical 化する。
            let tag = bytes_tag_record(&hex_encode(b));
            write_record(&tag, out)?;
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
        }
        ScriptValue::Record(rec) => write_record(rec, out)?,
        ScriptValue::Time {
            domain,
            timebase,
            count,
        } => {
            let mut m = std::collections::BTreeMap::new();
            m.insert(TAG_KEY.to_owned(), ScriptValue::String("time".to_owned()));
            m.insert("count".to_owned(), ScriptValue::String(count.to_string()));
            m.insert("domain".to_owned(), ScriptValue::String(domain.0.clone()));
            let mut tb = std::collections::BTreeMap::new();
            tb.insert(
                "num".to_owned(),
                ScriptValue::String(timebase.num.to_string()),
            );
            tb.insert(
                "den".to_owned(),
                ScriptValue::String(timebase.den.to_string()),
            );
            m.insert("timebase".to_owned(), ScriptValue::Record(tb));
            write_record(&m, out)?;
        }
        ScriptValue::ResourceId {
            domain,
            kind,
            value,
        } => {
            let mut m = std::collections::BTreeMap::new();
            m.insert(TAG_KEY.to_owned(), ScriptValue::String("res".to_owned()));
            m.insert("domain".to_owned(), ScriptValue::String(domain.0.clone()));
            m.insert("kind".to_owned(), ScriptValue::String(kind.clone()));
            m.insert("value".to_owned(), ScriptValue::String(value.clone()));
            write_record(&m, out)?;
        }
    }
    Ok(())
}

fn write_record(
    rec: &std::collections::BTreeMap<String, ScriptValue>,
    out: &mut String,
) -> Result<(), String> {
    out.push('{');
    let mut first = true;
    for (k, val) in rec {
        if matches!(val, ScriptValue::Absent) {
            continue;
        }
        if !first {
            out.push(',');
        }
        first = false;
        out.push_str(&serde_json::to_string(k).map_err(|e| e.to_string())?);
        out.push(':');
        write_value(val, out)?;
    }
    out.push('}');
    Ok(())
}

/// f64 の canonical 表現（design.md C3' の数値正規化）。
fn canonical_number(v: f64) -> String {
    const TWO_POW_53: f64 = 9_007_199_254_740_992.0;
    if v.is_nan() {
        "nan".to_owned()
    } else if v == f64::INFINITY {
        "inf".to_owned()
    } else if v == f64::NEG_INFINITY {
        "-inf".to_owned()
    } else if v == 0.0 {
        // -0.0 を 0 に正規化。
        "0".to_owned()
    } else if v.fract() == 0.0 && v.abs() <= TWO_POW_53 {
        format!("{}", v as i64)
    } else {
        format!("{v:e}")
    }
}

fn bytes_tag_record(hex: &str) -> std::collections::BTreeMap<String, ScriptValue> {
    let mut m = std::collections::BTreeMap::new();
    m.insert(TAG_KEY.to_owned(), ScriptValue::String("bytes".to_owned()));
    m.insert("hex".to_owned(), ScriptValue::String(hex.to_owned()));
    m
}

/// `payload` の DefinitionDigest（canonical bytes の sha256）。
pub fn definition_digest(payload: &ScriptValue) -> Result<DefinitionDigest, String> {
    let canonical = canonical_value(payload)?;
    Ok(DefinitionDigest(format!(
        "sha256:v1:{}",
        sha256_hex(canonical.as_bytes())
    )))
}

/// source text の SourceDigest。
pub fn source_digest(text: &str) -> SourceDigest {
    SourceDigest(format!("sha256:v1:{}", sha256_hex(text.as_bytes())))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// codec 用の hex encode（Bytes tag / 証拠出力）。
pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// codec 用の hex decode。失敗時は `None`。
pub fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

// ---------------------------------------------------------------------------
// tag codec（`{"$rv": …}` 形式 — codec と canonical で共有する構造）
// ---------------------------------------------------------------------------

/// tag Record を構築する補助（codec の push 方向で使う）。
pub fn tag_record(kind: &str, fields: &[(&str, ScriptValue)]) -> ScriptValue {
    let mut m = std::collections::BTreeMap::new();
    m.insert(TAG_KEY.to_owned(), ScriptValue::String(kind.to_owned()));
    for (k, v) in fields {
        m.insert((*k).to_owned(), v.clone());
    }
    ScriptValue::Record(m)
}

/// `{"$rv": kind}` tag かどうかと kind を返す。tag でなければ `None`。
pub fn tag_kind(v: &ScriptValue) -> Option<&str> {
    let rec = v.as_record()?;
    match rec.get(TAG_KEY) {
        Some(ScriptValue::String(s)) => Some(s.as_str()),
        _ => None,
    }
}

/// Record が tag 構造（`$rv` key を持つ）かどうか。
pub fn has_tag_key(v: &ScriptValue) -> bool {
    v.as_record().is_some_and(|r| r.contains_key(TAG_KEY))
}
