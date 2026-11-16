//! Codec round-trip and canonical digest tests (test-spec R3/R4/E3/E4).

mod common;

use std::collections::BTreeMap;

use common::exotic_value;
use ruviform_script::prelude::*;

fn value_without_nan() -> ScriptValue {
    // Same shape as `exotic_value` minus the non-finite Numbers, for
    // structural equality assertions (NaN != NaN under PartialEq).
    let mut record = BTreeMap::new();
    record.insert("nested".to_string(), ScriptValue::Bool(true));
    record.insert("absent_field".to_string(), ScriptValue::Absent);
    ScriptValue::Array(vec![
        ScriptValue::Absent,
        ScriptValue::Null,
        ScriptValue::Bool(false),
        ScriptValue::Integer(i64::MAX),
        ScriptValue::Integer(i64::MIN),
        ScriptValue::Integer(-9_007_199_254_740_993),
        ScriptValue::Integer(9_007_199_254_740_993),
        ScriptValue::Number(1.5),
        ScriptValue::Number(-0.0),
        ScriptValue::String("he\"llo\n".to_string()),
        ScriptValue::Bytes(vec![0xde, 0xad, 0xbe, 0xef]),
        ScriptValue::Time {
            domain: TimeDomainId::new("audio.transport"),
            timebase: Rational::new(1, 48000),
            count: 1024,
        },
        ScriptValue::ResourceId {
            domain: ResourceDomainId::new("track"),
            kind: "audio".to_string(),
            value: "trk-42".to_string(),
        },
        ScriptValue::Record(record),
    ])
}

#[test]
fn codec_round_trip_all_variants_structural() {
    let value = value_without_nan();
    let materialized = materialize(&value);
    let restored = dematerialize(materialized).expect("dematerialize");
    assert_eq!(restored, value, "full variant round-trip must be equal");
}

#[test]
fn codec_round_trip_canonical_equality_with_nan() {
    // NaN != NaN under PartialEq, so compare canonical forms — which
    // render non-finite numbers deterministically.
    let value = exotic_value();
    let restored = dematerialize(materialize(&value)).expect("dematerialize");
    assert_eq!(
        canonical_value(&restored).expect("canonical"),
        canonical_value(&value).expect("canonical"),
        "canonical form must survive the codec round-trip"
    );
}

#[test]
fn materialize_tags_only_unsafe_variants() {
    // Small integers and plain values pass through untagged.
    assert_eq!(
        materialize(&ScriptValue::Integer(42)),
        ScriptValue::Integer(42)
    );
    assert_eq!(materialize(&ScriptValue::Null), ScriptValue::Null);
    assert_eq!(materialize(&ScriptValue::Absent), ScriptValue::Absent);
    assert!(!is_tagged(&ScriptValue::Integer(42)));

    // >2^53 integers, Time, ResourceId, Bytes, non-finite Number tag.
    assert!(is_tagged(&materialize(&ScriptValue::Integer(i64::MAX))));
    assert!(is_tagged(&materialize(&ScriptValue::Time {
        domain: TimeDomainId::new("d"),
        timebase: Rational::new(1, 1),
        count: 0,
    })));
    assert!(is_tagged(&materialize(&ScriptValue::ResourceId {
        domain: ResourceDomainId::new("d"),
        kind: "k".to_string(),
        value: "v".to_string(),
    })));
    assert!(is_tagged(&materialize(&ScriptValue::Bytes(vec![1]))));
    assert!(is_tagged(&materialize(&ScriptValue::Number(f64::NAN))));
    assert!(!is_tagged(&materialize(&ScriptValue::Number(1.5))));
}

#[test]
fn dematerialize_rejects_unknown_tag() {
    let mut map = BTreeMap::new();
    map.insert(
        TAG_KEY.to_string(),
        ScriptValue::String("bogus".to_string()),
    );
    let err = dematerialize(ScriptValue::Record(map)).expect_err("unknown tag");
    assert!(matches!(err, CodecError::UnknownTag { .. }));
}

#[test]
fn dematerialize_rejects_non_string_tag_key() {
    let mut map = BTreeMap::new();
    map.insert(TAG_KEY.to_string(), ScriptValue::Integer(1));
    let err = dematerialize(ScriptValue::Record(map)).expect_err("non-string tag");
    assert!(matches!(err, CodecError::MalformedTag { .. }));
}

#[test]
fn dematerialize_rejects_missing_and_extra_fields() {
    // int tag missing "value".
    let mut map = BTreeMap::new();
    map.insert(TAG_KEY.to_string(), ScriptValue::String("int".to_string()));
    assert!(matches!(
        dematerialize(ScriptValue::Record(map)),
        Err(CodecError::MalformedTag { .. })
    ));

    // int tag with an extra field.
    let mut map = BTreeMap::new();
    map.insert(TAG_KEY.to_string(), ScriptValue::String("int".to_string()));
    map.insert("value".to_string(), ScriptValue::String("5".to_string()));
    map.insert("extra".to_string(), ScriptValue::Bool(true));
    assert!(matches!(
        dematerialize(ScriptValue::Record(map)),
        Err(CodecError::MalformedTag { .. })
    ));
}

#[test]
fn dematerialize_rejects_malformed_field_values() {
    // int value not parseable as i64.
    let mut map = BTreeMap::new();
    map.insert(TAG_KEY.to_string(), ScriptValue::String("int".to_string()));
    map.insert(
        "value".to_string(),
        ScriptValue::String("not-a-number".to_string()),
    );
    assert!(matches!(
        dematerialize(ScriptValue::Record(map)),
        Err(CodecError::MalformedTag { .. })
    ));

    // bytes hex invalid (odd length).
    let mut map = BTreeMap::new();
    map.insert(
        TAG_KEY.to_string(),
        ScriptValue::String("bytes".to_string()),
    );
    map.insert("hex".to_string(), ScriptValue::String("abc".to_string()));
    assert!(matches!(
        dematerialize(ScriptValue::Record(map)),
        Err(CodecError::MalformedTag { .. })
    ));

    // number tag with unknown spelling.
    let mut map = BTreeMap::new();
    map.insert(
        TAG_KEY.to_string(),
        ScriptValue::String("number".to_string()),
    );
    map.insert(
        "value".to_string(),
        ScriptValue::String("1e999".to_string()),
    );
    assert!(matches!(
        dematerialize(ScriptValue::Record(map)),
        Err(CodecError::MalformedTag { .. })
    ));
}

#[test]
fn dematerialize_accepts_vendor_null_and_absent_tags() {
    for (tag, expected) in [("null", ScriptValue::Null), ("absent", ScriptValue::Absent)] {
        let mut map = BTreeMap::new();
        map.insert(TAG_KEY.to_string(), ScriptValue::String(tag.to_string()));
        assert_eq!(
            dematerialize(ScriptValue::Record(map)).expect("tag"),
            expected
        );
    }
}

#[test]
fn canonical_record_keys_sorted_and_absent_omitted() {
    let mut map = BTreeMap::new();
    map.insert("z".to_string(), ScriptValue::Integer(1));
    map.insert("a".to_string(), ScriptValue::Integer(2));
    map.insert("skip".to_string(), ScriptValue::Absent);
    let canonical = canonical_value(&ScriptValue::Record(map)).expect("canonical");
    assert_eq!(canonical, r#"{"a":2,"z":1}"#);
}

#[test]
fn canonical_top_level_absent_fails() {
    assert!(matches!(
        canonical_value(&ScriptValue::Absent),
        Err(CanonicalError::Absent)
    ));
}

#[test]
fn canonical_number_normalization() {
    use canonical_number;
    assert_eq!(canonical_number(1.0), "1");
    assert_eq!(canonical_number(-0.0), "0");
    assert_eq!(canonical_number(0.5), "0.5");
    assert_eq!(
        canonical_number(f64::NAN),
        "{\"$rv\":\"number\",\"value\":\"nan\"}"
    );
    assert_eq!(
        canonical_number(f64::INFINITY),
        "{\"$rv\":\"number\",\"value\":\"inf\"}"
    );
    assert_eq!(
        canonical_number(f64::NEG_INFINITY),
        "{\"$rv\":\"number\",\"value\":\"-inf\"}"
    );
}

#[test]
fn digest_stable_for_same_meaning_and_diverges_for_different() {
    // Same meaning, different construction order → BTreeMap normalizes.
    let mut a = BTreeMap::new();
    a.insert("x".to_string(), ScriptValue::Integer(1));
    a.insert("y".to_string(), ScriptValue::Integer(2));
    let mut b = BTreeMap::new();
    b.insert("y".to_string(), ScriptValue::Integer(2));
    b.insert("x".to_string(), ScriptValue::Integer(1));
    // BTreeMap makes order irrelevant by construction; also verify
    // Number(1.0)/Integer(1) normalize to the same digest.
    let d_a = definition_digest(&ScriptValue::Record(a)).expect("digest");
    let d_b = definition_digest(&ScriptValue::Record(b)).expect("digest");
    assert_eq!(d_a, d_b);

    let d_int = definition_digest(&ScriptValue::Integer(1)).expect("digest");
    let d_num = definition_digest(&ScriptValue::Number(1.0)).expect("digest");
    assert_eq!(d_int, d_num, "integral Number and Integer share digest");

    let d_other = definition_digest(&ScriptValue::Integer(2)).expect("digest");
    assert_ne!(d_int, d_other, "meaning difference → different digest");

    // Absent vs Null must diverge.
    let mut with_absent = BTreeMap::new();
    with_absent.insert("f".to_string(), ScriptValue::Absent);
    let mut with_null = BTreeMap::new();
    with_null.insert("f".to_string(), ScriptValue::Null);
    let d_absent = definition_digest(&ScriptValue::Record(with_absent)).expect("digest");
    let d_null = definition_digest(&ScriptValue::Record(with_null)).expect("digest");
    assert_ne!(d_absent, d_null);
}

#[test]
fn source_digest_format_and_stability() {
    let d1 = source_digest("hello");
    let d2 = source_digest("hello");
    let d3 = source_digest("world");
    assert_eq!(d1, d2);
    assert_ne!(d1, d3);
    assert!(d1.as_str().starts_with("sha256:v1:"));
    assert_eq!(d1.as_str().len(), "sha256:v1:".len() + 64);
}

#[test]
fn canonical_value_matches_materialized_form() {
    // canonical_value(v) == canonical_value(materialize(v)) for every
    // variant except >2^53 Integer (typed decimal vs int tag) and
    // reachable Absent (no canonical form — excluded here).
    for v in [
        ScriptValue::Null,
        ScriptValue::Bool(true),
        ScriptValue::Integer(42),
        ScriptValue::Integer(-42),
        ScriptValue::Number(2.5),
        ScriptValue::Number(-0.0),
        ScriptValue::Number(f64::NAN),
        ScriptValue::Number(f64::INFINITY),
        ScriptValue::String("s".to_string()),
        ScriptValue::Bytes(vec![1, 2]),
        ScriptValue::Time {
            domain: TimeDomainId::new("d"),
            timebase: Rational::new(1, 48000),
            count: 7,
        },
        ScriptValue::ResourceId {
            domain: ResourceDomainId::new("d"),
            kind: "k".to_string(),
            value: "v".to_string(),
        },
        ScriptValue::Array(vec![
            ScriptValue::Integer(1),
            ScriptValue::String("x".to_string()),
        ]),
        ScriptValue::Record(BTreeMap::from([(
            "k".to_string(),
            ScriptValue::Bytes(vec![9]),
        )])),
    ] {
        let direct = canonical_value(&v).expect("canonical");
        let via_codec = canonical_value(&materialize(&v)).expect("canonical");
        assert_eq!(direct, via_codec, "canonical drift for {v:?}");
    }

    // Documented exception: >2^53 Integer keeps exact decimal in typed
    // canonical form while the codec transports the int tag record.
    let big = ScriptValue::Integer(i64::MAX);
    assert_ne!(
        canonical_value(&big).expect("canonical"),
        canonical_value(&materialize(&big)).expect("canonical"),
        ">2^53 Integer is the documented canonical/codec exception"
    );
}
