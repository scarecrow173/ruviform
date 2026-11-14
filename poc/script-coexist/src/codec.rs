//! ScriptValue ↔ vendor 値の変換で共有する tag record codec。
//!
//! `ScriptValue` の閉じた variant のうち、vendor がネイティブに表現
//! できないもの（`Time`/`ResourceId`/`Bytes`/`Null`(Rhai)/`Absent`(Luau)/
//! f64 精度を超える `Integer`）は、canonical.rs と同じ `{"$rv":…}` tag
//! record 構造で vendor 境界を越える。両 backend で同一 schema を使う
//! ことで、artifact payload の canonical digest が一致する。

use std::collections::BTreeMap;

use crate::canonical::{has_tag_key, tag_kind, tag_record};
use crate::value::{Rational, ResourceDomainId, ScriptValue, TimeDomainId};

/// f64 で i64 を厳密に往復できる範囲（|v| <= 2^53）。
pub const F64_SAFE_I64: i64 = 9_007_199_254_740_992; // 2^53

/// vendor へ渡す前に typed 値を「vendor 中立の ScriptValue」へ写像する。
/// 戻り値は依然 `ScriptValue` だが、tag record 化された値は vendor 側で
/// 通常の Record/Table/Map として扱われる。
pub fn materialize(v: &ScriptValue) -> ScriptValue {
    match v {
        ScriptValue::Integer(i) if i.unsigned_abs() > F64_SAFE_I64 as u64 => {
            // f64 精度を超える i64 は 10 進文字列 tag で厳密に運ぶ。
            tag_record("int", &[("v", ScriptValue::String(i.to_string()))])
        }
        ScriptValue::Time {
            domain,
            timebase,
            count,
        } => {
            let mut tb = BTreeMap::new();
            tb.insert(
                "num".to_owned(),
                ScriptValue::String(timebase.num.to_string()),
            );
            tb.insert(
                "den".to_owned(),
                ScriptValue::String(timebase.den.to_string()),
            );
            tag_record(
                "time",
                &[
                    ("count", ScriptValue::String(count.to_string())),
                    ("domain", ScriptValue::String(domain.0.clone())),
                    ("timebase", ScriptValue::Record(tb)),
                ],
            )
        }
        ScriptValue::ResourceId {
            domain,
            kind,
            value,
        } => tag_record(
            "res",
            &[
                ("domain", ScriptValue::String(domain.0.clone())),
                ("kind", ScriptValue::String(kind.clone())),
                ("value", ScriptValue::String(value.clone())),
            ],
        ),
        ScriptValue::Bytes(b) => tag_record(
            "bytes",
            &[("hex", ScriptValue::String(crate::canonical::hex_encode(b)))],
        ),
        // Rhai には null がないため、null は tag で運ぶ。
        // （Luau は nil→Null をネイティブに使う。Absent は tag で運ぶ。）
        ScriptValue::Null => tag_record("null", &[]),
        ScriptValue::Absent => tag_record("absent", &[]),
        ScriptValue::Array(items) => ScriptValue::Array(items.iter().map(materialize).collect()),
        ScriptValue::Record(rec) => ScriptValue::Record(
            rec.iter()
                .map(|(k, v)| (k.clone(), materialize(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// vendor から復元した `ScriptValue` を typed 値へ戻す。
/// 未知の `$rv` tag は fail-closed で `Err`（silent pass-through しない）。
/// vendor 側の null/absent 写像（Luau: nil→Null, Rhai: ()→Absent）は
/// vendor→ScriptValue 変換の段階で行い、ここでは tag のみを処理する。
pub fn dematerialize(v: ScriptValue) -> Result<ScriptValue, String> {
    // まず tag record かどうかを見る。
    if let Some(kind) = tag_kind(&v) {
        let rec = v.as_record().expect("tag_kind implies record");
        let field = |name: &str| -> Result<String, String> {
            match rec.get(name) {
                Some(ScriptValue::String(s)) => Ok(s.clone()),
                _ => Err(format!("tag {kind:?} missing string field {name:?}")),
            }
        };
        let record_field = |name: &str| -> Result<ScriptValue, String> {
            rec.get(name)
                .cloned()
                .ok_or_else(|| format!("tag {kind:?} missing field {name:?}"))
        };
        return match kind {
            "int" => {
                let s = field("v")?;
                let i: i64 = s
                    .parse()
                    .map_err(|_| format!("tag int value out of i64 range: {s:?}"))?;
                Ok(ScriptValue::Integer(i))
            }
            "time" => {
                let tb = record_field("timebase")?;
                let tb_rec = tb
                    .as_record()
                    .ok_or_else(|| "tag time.timebase not a record".to_owned())?;
                let get_i64 = |name: &str| -> Result<i64, String> {
                    match tb_rec.get(name) {
                        Some(ScriptValue::String(s)) => s
                            .parse()
                            .map_err(|_| format!("tag time.timebase.{name} bad int")),
                        _ => Err(format!("tag time.timebase.{name} missing")),
                    }
                };
                Ok(ScriptValue::Time {
                    domain: TimeDomainId(field("domain")?),
                    timebase: Rational {
                        num: get_i64("num")?,
                        den: get_i64("den")?,
                    },
                    count: field("count")?
                        .parse()
                        .map_err(|_| "tag time.count bad int".to_owned())?,
                })
            }
            "res" => Ok(ScriptValue::ResourceId {
                domain: ResourceDomainId(field("domain")?),
                kind: field("kind")?,
                value: field("value")?,
            }),
            "bytes" => {
                let hex = field("hex")?;
                let bytes = crate::canonical::hex_decode(&hex)
                    .ok_or_else(|| "tag bytes.hex invalid".to_owned())?;
                Ok(ScriptValue::Bytes(bytes))
            }
            "null" => Ok(ScriptValue::Null),
            "absent" => Ok(ScriptValue::Absent),
            other => Err(format!("unknown value tag {other:?}")),
        };
    }
    // tag でない通常値: 再帰的に dematerialize（ネストした tag を拾う）。
    match v {
        ScriptValue::Array(items) => Ok(ScriptValue::Array(
            items
                .into_iter()
                .map(dematerialize)
                .collect::<Result<_, _>>()?,
        )),
        ScriptValue::Record(rec) => {
            // `$rv` key を持つが tag_kind が None = `$rv` が String でない
            // 等の不正形 → fail-closed。
            if has_tag_key(&ScriptValue::Record(rec.clone())) {
                return Err("malformed $rv tag record".to_owned());
            }
            Ok(ScriptValue::Record(
                rec.into_iter()
                    .map(|(k, v)| dematerialize(v).map(|d| (k, d)))
                    .collect::<Result<_, _>>()?,
            ))
        }
        other => Ok(other),
    }
}

/// `materialize` で作った tag record が「ユーザー供給の Record」と
/// 区別できるよう、tag record は reserved key `$rv` を必ず持つ。
pub fn is_tagged(v: &ScriptValue) -> bool {
    tag_kind(v).is_some()
}
