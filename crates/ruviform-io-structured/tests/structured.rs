//! Contract tests for `ruviform-io-structured` (IO-IMP-03 / Issue #297,
//! IO-IMP-04 / Issue #298 for `csv`).
//!
//! Each format section is gated on its feature; `cargo test` with no
//! features compiles an empty suite. The file pins the Acceptance
//! matrix: typed round trips (structs/enums/nesting/Unicode/non-ASCII
//! keys), strict EOF / trailing data, invalid + truncated input, type
//! mismatch, byte-budget rejection *and* the exact boundary, backend
//! error normalization (`DocumentErrorKind` + `source()`), and the
//! `file` feature's staged publish contract (encode vs. publish
//! failure separation with `PublishFailure` state preserved).

#[cfg(any(feature = "json", feature = "yaml", feature = "toml", feature = "csv"))]
mod fixtures {
    use serde::{Deserialize, Serialize};

    #[cfg(any(feature = "json", feature = "yaml", feature = "toml"))]
    use std::collections::BTreeMap;

    /// Struct + enum + nesting + Unicode + a non-ASCII map key.
    #[cfg(any(feature = "json", feature = "yaml", feature = "toml"))]
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub(crate) enum Transport {
        File { path: String },
        Url { url: String, secure: bool },
    }

    /// Struct + enum + nesting + Unicode + a non-ASCII map key.
    #[cfg(any(feature = "json", feature = "yaml", feature = "toml"))]
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub(crate) struct Manifest {
        pub(crate) name: String,
        pub(crate) revision: u32,
        pub(crate) transport: Transport,
        pub(crate) tags: Vec<String>,
        pub(crate) labels: BTreeMap<String, String>,
    }

    #[cfg(any(feature = "json", feature = "yaml", feature = "toml"))]
    impl Manifest {
        pub(crate) fn sample() -> Self {
            let mut labels = BTreeMap::new();
            labels.insert("日本語キー".to_owned(), "値".to_owned());
            labels.insert("owner".to_owned(), "ruviform".to_owned());
            Self {
                name: "démo 🎚️".to_owned(),
                revision: 7,
                transport: Transport::Url {
                    url: "https://example.invalid/メタ".to_owned(),
                    secure: true,
                },
                tags: vec!["alpha".to_owned(), "β".to_owned()],
                labels,
            }
        }
    }

    /// Target type for negative probes (`name` must be a string).
    #[cfg(any(feature = "json", feature = "yaml", feature = "toml"))]
    #[derive(Debug, Deserialize)]
    pub(crate) struct NameDoc {
        pub(crate) name: String,
    }

    /// `NameDoc` with serde's `deny_unknown_fields` — pins that schema
    /// strictness is the consumer's serde-level choice, not something
    /// the adapter imposes or forbids.
    #[cfg(any(feature = "json", feature = "yaml", feature = "toml"))]
    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(crate) struct StrictNameDoc {
        pub(crate) name: String,
    }

    /// High-precision `f64` values for round-trip exactness probes.
    #[cfg(any(feature = "json", feature = "yaml", feature = "toml"))]
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub(crate) struct Floats {
        pub(crate) a: f64,
        pub(crate) b: f64,
        pub(crate) c: f64,
        pub(crate) d: f64,
    }

    #[cfg(any(feature = "json", feature = "yaml", feature = "toml"))]
    impl Floats {
        pub(crate) fn precise() -> Self {
            Self {
                a: 0.1,
                b: 0.300_000_000_000_000_04,
                c: 1.797_693_134_862_315_7e308,
                d: -2.5e-10,
            }
        }
    }

    /// A CSV row bound by header names (`#[cfg]`-visible under `csv`).
    #[cfg(feature = "csv")]
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub(crate) struct Row {
        pub(crate) city: String,
        pub(crate) pop: u64,
    }
}

#[cfg(feature = "json")]
mod json_tests {
    use super::fixtures::{Floats, Manifest, NameDoc, StrictNameDoc};
    use ruviform_io_structured::{DocumentErrorKind, DocumentFormat, InputLimits, json};
    use std::collections::HashMap;
    use std::error::Error;

    #[test]
    fn round_trip_compact_and_pretty() {
        let manifest = Manifest::sample();
        for pretty in [false, true] {
            let mut bytes = Vec::new();
            json::to_writer(
                &mut bytes,
                &manifest,
                &json::JsonWriteOptions::new().with_pretty(pretty),
            )
            .expect("serialize");
            let mut slice = bytes.as_slice();
            let back: Manifest =
                json::from_reader(&mut slice, &json::JsonReadOptions::default()).expect("parse");
            assert_eq!(back, manifest, "pretty={pretty}");
        }
    }

    #[test]
    fn trailing_data_is_rejected() {
        let mut input = br#"{"name":"a"} trailing"#.as_slice();
        let err = json::from_reader::<NameDoc>(&mut input, &json::JsonReadOptions::default())
            .expect_err("trailing data must fail");
        assert_eq!(err.kind(), DocumentErrorKind::TrailingData);
        assert_eq!(err.format(), DocumentFormat::Json);
        // The backend reports a line/column for the trailing bytes.
        let pos = err.position().expect("position");
        assert_eq!(pos.line, Some(1));
    }

    #[test]
    fn trailing_whitespace_is_accepted() {
        let mut doc = br#"{"name":"a"}"#.to_vec();
        doc.extend_from_slice(b"  \n\t");
        let mut input = doc.as_slice();
        let doc: NameDoc =
            json::from_reader(&mut input, &json::JsonReadOptions::default()).expect("parse");
        assert_eq!(doc.name, "a");
    }

    #[test]
    fn second_json_value_is_trailing_data() {
        let mut input = br#"{"name":"a"}{"name":"b"}"#.as_slice();
        let err = json::from_reader::<NameDoc>(&mut input, &json::JsonReadOptions::default())
            .expect_err("second value must fail");
        assert_eq!(err.kind(), DocumentErrorKind::TrailingData);
    }

    #[test]
    fn truncated_document_is_syntax() {
        let mut input = br#"{"name":"a""#.as_slice();
        let err = json::from_reader::<NameDoc>(&mut input, &json::JsonReadOptions::default())
            .expect_err("truncated input must fail");
        assert_eq!(err.kind(), DocumentErrorKind::Syntax);
    }

    #[test]
    fn malformed_document_is_syntax_with_backend_source() {
        let mut input = b"{not json}".as_slice();
        let err = json::from_reader::<NameDoc>(&mut input, &json::JsonReadOptions::default())
            .expect_err("malformed input must fail");
        assert_eq!(err.kind(), DocumentErrorKind::Syntax);
        // Backend diagnostics stay reachable via the erased source chain.
        assert!(err.source().is_some());
        // But the DocumentError Display itself embeds no backend text.
        assert!(!err.to_string().contains("not json"));
    }

    #[test]
    fn type_mismatch_is_data_model() {
        let mut input = br#"{"name":5}"#.as_slice();
        let err = json::from_reader::<NameDoc>(&mut input, &json::JsonReadOptions::default())
            .expect_err("type mismatch must fail");
        assert_eq!(err.kind(), DocumentErrorKind::DataModel);
    }

    #[test]
    fn missing_required_field_is_data_model() {
        let mut input = br#"{"other":1}"#.as_slice();
        let err = json::from_reader::<NameDoc>(&mut input, &json::JsonReadOptions::default())
            .expect_err("missing field must fail");
        assert_eq!(err.kind(), DocumentErrorKind::DataModel);
    }

    #[test]
    fn over_budget_is_limit_exceeded() {
        let mut input = br#"{"name":"abcdefghij"}"#.as_slice();
        let options = json::JsonReadOptions::new().with_limits(InputLimits::new(8));
        let err = json::from_reader::<NameDoc>(&mut input, &options)
            .expect_err("over-budget input must fail");
        assert_eq!(err.kind(), DocumentErrorKind::LimitExceeded);
        let limit = err.limit().expect("limit detail");
        assert_eq!(limit.name, "max_bytes");
        assert_eq!(limit.max, 8);
    }

    #[test]
    fn exact_budget_boundary_succeeds_and_one_more_byte_fails() {
        let doc = br#"{"name":"ab"}"#; // exactly 13 bytes
        assert_eq!(doc.len(), 13);
        let options = json::JsonReadOptions::new().with_limits(InputLimits::new(doc.len() as u64));
        let mut ok_input = doc.as_slice();
        let parsed: NameDoc =
            json::from_reader(&mut ok_input, &options).expect("exact-budget doc parses");
        assert_eq!(parsed.name, "ab");

        // One byte beyond the budget — even though it is whitespace —
        // trips the limit while the EOF check drains the tail.
        let mut over_input = br#"{"name":"ab"} "#.as_slice();
        let err = json::from_reader::<NameDoc>(&mut over_input, &options)
            .expect_err("input past the budget must fail");
        assert_eq!(err.kind(), DocumentErrorKind::LimitExceeded);
    }

    #[test]
    fn unicode_and_non_ascii_keys_round_trip() {
        let mut map = HashMap::new();
        map.insert("キー🎵".to_owned(), "välüe".to_owned());
        let mut bytes = Vec::new();
        json::to_writer(&mut bytes, &map, &json::JsonWriteOptions::default()).expect("serialize");
        let mut slice = bytes.as_slice();
        let back: HashMap<String, String> =
            json::from_reader(&mut slice, &json::JsonReadOptions::default()).expect("parse");
        assert_eq!(back, map);
    }

    #[test]
    fn non_string_map_key_is_data_model_on_write() {
        // JSON object keys must be strings — a tuple key is
        // unserializable. (Integer keys *are* representable and
        // serialize fine.)
        let mut bad: HashMap<(i32, i32), String> = HashMap::new();
        bad.insert((1, 2), "x".to_owned());
        let mut out = Vec::new();
        let err = json::to_writer(&mut out, &bad, &json::JsonWriteOptions::default())
            .expect_err("non-string keys must fail");
        assert_eq!(err.kind(), DocumentErrorKind::DataModel);
    }

    #[test]
    fn io_failure_on_read_is_io() {
        struct Failing;
        impl std::io::Read for Failing {
            fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "nope",
                ))
            }
        }
        let mut reader = Failing;
        let err = json::from_reader::<NameDoc>(&mut reader, &json::JsonReadOptions::default())
            .expect_err("failing reader must fail");
        assert_eq!(err.kind(), DocumentErrorKind::Io);
    }

    #[test]
    fn unknown_fields_are_ignored_unless_the_consumer_denies_them() {
        // Serde's default skips unknown members — the adapter does not
        // impose schema policy. `deny_unknown_fields` is the consumer's
        // opt-in strictness and must still classify as `DataModel`.
        let mut input = br#"{"name":"a","surprise":1}"#.as_slice();
        let doc: NameDoc = json::from_reader(&mut input, &json::JsonReadOptions::default())
            .expect("lenient parse");
        assert_eq!(doc.name, "a");

        let mut strict_input = br#"{"name":"a","surprise":1}"#.as_slice();
        let err = json::from_reader::<StrictNameDoc>(
            &mut strict_input,
            &json::JsonReadOptions::default(),
        )
        .expect_err("deny_unknown_fields must reject");
        assert_eq!(err.kind(), DocumentErrorKind::DataModel);

        let mut clean_input = br#"{"name":"b"}"#.as_slice();
        let clean: StrictNameDoc =
            json::from_reader(&mut clean_input, &json::JsonReadOptions::default())
                .expect("strict parse of clean input");
        assert_eq!(clean.name, "b");
    }

    #[test]
    fn float_precision_round_trips_bit_exact() {
        // serde_json's shortest-round-trip float encoding must preserve
        // f64 values exactly — consumers rely on this for numeric
        // fidelity (manifest scores, timestamps, ratios).
        let value = Floats::precise();
        let mut bytes = Vec::new();
        json::to_writer(&mut bytes, &value, &json::JsonWriteOptions::default()).expect("serialize");
        let mut slice = bytes.as_slice();
        let back: Floats =
            json::from_reader(&mut slice, &json::JsonReadOptions::default()).expect("parse");
        assert_eq!(back, value);
        for (a, b) in [
            (back.a, value.a),
            (back.b, value.b),
            (back.c, value.c),
            (back.d, value.d),
        ] {
            assert_eq!(a.to_bits(), b.to_bits(), "f64 bits must round-trip exactly");
        }
    }

    #[test]
    fn extreme_nesting_is_backend_fixed_syntax_not_fake_limit() {
        // serde_json's recursion limit is fixed inside the backend —
        // exceeding it is honestly `Syntax`, never a pretend
        // `LimitExceeded` from `InputLimits` (which has no depth knob).
        let mut doc = vec![b'['; 256];
        doc.extend(std::iter::repeat_n(b']', 256));
        let mut input = doc.as_slice();
        // `Value` forces real recursive descent — `IgnoredAny` skips the
        // backend's depth accounting, so it cannot probe the limit.
        let err =
            json::from_reader::<serde_json::Value>(&mut input, &json::JsonReadOptions::default())
                .expect_err("pathological nesting must fail");
        assert_eq!(err.kind(), DocumentErrorKind::Syntax);
        assert_ne!(err.kind(), DocumentErrorKind::LimitExceeded);
    }
}

#[cfg(feature = "yaml")]
mod yaml_tests {
    use super::fixtures::{Floats, Manifest, NameDoc, StrictNameDoc};
    use ruviform_io_structured::{DocumentErrorKind, DocumentFormat, InputLimits, yaml};
    use std::collections::BTreeMap;
    use std::error::Error;

    #[test]
    fn round_trip() {
        let manifest = Manifest::sample();
        let mut bytes = Vec::new();
        yaml::to_writer(&mut bytes, &manifest, &yaml::YamlWriteOptions::new()).expect("serialize");
        let mut slice = bytes.as_slice();
        let back: Manifest =
            yaml::from_reader(&mut slice, &yaml::YamlReadOptions::default()).expect("parse");
        assert_eq!(back, manifest);
    }

    #[test]
    fn multi_document_input_is_trailing_data() {
        let mut input = b"---\nname: a\n---\nname: b\n".as_slice();
        let err = yaml::from_reader::<NameDoc>(&mut input, &yaml::YamlReadOptions::default())
            .expect_err("second document must fail");
        assert_eq!(err.kind(), DocumentErrorKind::TrailingData);
        assert_eq!(err.format(), DocumentFormat::Yaml);
    }

    #[test]
    fn malformed_document_is_syntax() {
        // `@` is a reserved indicator — the scanner rejects it.
        let mut input = b"name: @invalid\n".as_slice();
        let err = yaml::from_reader::<NameDoc>(&mut input, &yaml::YamlReadOptions::default())
            .expect_err("malformed input must fail");
        assert_eq!(err.kind(), DocumentErrorKind::Syntax);
        assert!(err.source().is_some());
    }

    #[test]
    fn type_mismatch_is_data_model() {
        // A sequence cannot deserialize into a String field. (Plain
        // scalars like `5` *do* coerce to strings in YAML — that is a
        // documented backend data-model behavior, not an error.)
        let mut input = b"name: [1, 2]\n".as_slice();
        let err = yaml::from_reader::<NameDoc>(&mut input, &yaml::YamlReadOptions::default())
            .expect_err("type mismatch must fail");
        assert_eq!(err.kind(), DocumentErrorKind::DataModel);
    }

    #[test]
    fn over_budget_is_limit_exceeded() {
        let mut input = b"name: abcdefghijklmnop\n".as_slice();
        let options = yaml::YamlReadOptions::new().with_limits(InputLimits::new(8));
        let err = yaml::from_reader::<NameDoc>(&mut input, &options)
            .expect_err("over-budget input must fail");
        assert_eq!(err.kind(), DocumentErrorKind::LimitExceeded);
        assert_eq!(err.limit().map(|l| l.name), Some("max_bytes"));
    }

    #[test]
    fn exact_budget_boundary_succeeds_and_one_more_byte_fails() {
        let doc = b"name: ok\n"; // exactly 9 bytes
        assert_eq!(doc.len(), 9);
        let options = yaml::YamlReadOptions::new().with_limits(InputLimits::new(doc.len() as u64));
        let mut ok_input = doc.as_slice();
        let parsed: NameDoc =
            yaml::from_reader(&mut ok_input, &options).expect("exact-budget doc parses");
        assert_eq!(parsed.name, "ok");

        let mut over_input = b"name: ok\n ".as_slice();
        let err = yaml::from_reader::<NameDoc>(&mut over_input, &options)
            .expect_err("input past the budget must fail");
        assert_eq!(err.kind(), DocumentErrorKind::LimitExceeded);
    }

    #[test]
    fn depth_budget_is_enforced() {
        // 7 nested sequences exceeds max_depth = 4.
        let mut input = b"[[[[[[[1]]]]]]]\n".as_slice();
        let options = yaml::YamlReadOptions::new().with_max_depth(4);
        let err = yaml::from_reader::<serde::de::IgnoredAny>(&mut input, &options)
            .expect_err("deep nesting must fail");
        assert_eq!(err.kind(), DocumentErrorKind::LimitExceeded);
        assert_eq!(err.limit().map(|l| l.name), Some("max_depth"));
    }

    #[test]
    fn alias_budget_is_enforced() {
        let mut input = b"a: &x 1\nb: *x\nc: *x\n".as_slice();
        let options = yaml::YamlReadOptions::new().with_max_aliases(1);
        let err = yaml::from_reader::<serde::de::IgnoredAny>(&mut input, &options)
            .expect_err("alias storm must fail");
        assert_eq!(err.kind(), DocumentErrorKind::LimitExceeded);
    }

    #[test]
    fn unicode_and_non_ascii_keys_round_trip() {
        let mut labels = BTreeMap::new();
        labels.insert("キー".to_owned(), "値 🎼".to_owned());
        let mut bytes = Vec::new();
        yaml::to_writer(&mut bytes, &labels, &yaml::YamlWriteOptions::new()).expect("serialize");
        let mut slice = bytes.as_slice();
        let back: BTreeMap<String, String> =
            yaml::from_reader(&mut slice, &yaml::YamlReadOptions::default()).expect("parse");
        assert_eq!(back, labels);
    }

    #[test]
    fn unsupported_tag_reports_not_panics() {
        // Explicit `!custom` tags the backend cannot resolve surface as
        // a classified error (or are tolerated per backend policy) —
        // either way they never panic or silently vanish.
        let mut input = b"name: !definitely-not-a-tag value\n".as_slice();
        if let Err(err) =
            yaml::from_reader::<NameDoc>(&mut input, &yaml::YamlReadOptions::default())
        {
            assert!(matches!(
                err.kind(),
                DocumentErrorKind::Unsupported
                    | DocumentErrorKind::Syntax
                    | DocumentErrorKind::DataModel
            ));
        }
    }

    #[test]
    fn display_does_not_embed_input_text() {
        // ERR-02: `DocumentError`'s own `Display` is the normalized
        // surface — input content may only appear inside the erased
        // `source()` diagnostic, never in the matchable error's text.
        let mut input = b"name: [unclosed MARKER7f3a9c\n".as_slice();
        let err = yaml::from_reader::<NameDoc>(&mut input, &yaml::YamlReadOptions::default())
            .expect_err("malformed input must fail");
        let rendered = err.to_string();
        assert!(
            !rendered.contains("MARKER7f3a9c"),
            "Display must not echo input text: {rendered}"
        );
        assert!(rendered.contains("yaml"), "{rendered}");
    }

    #[test]
    fn unknown_fields_are_ignored_unless_the_consumer_denies_them() {
        let mut input = b"name: a\nsurprise: 1\n".as_slice();
        let doc: NameDoc = yaml::from_reader(&mut input, &yaml::YamlReadOptions::default())
            .expect("lenient parse");
        assert_eq!(doc.name, "a");

        let mut strict_input = b"name: a\nsurprise: 1\n".as_slice();
        let err = yaml::from_reader::<StrictNameDoc>(
            &mut strict_input,
            &yaml::YamlReadOptions::default(),
        )
        .expect_err("deny_unknown_fields must reject");
        assert_eq!(err.kind(), DocumentErrorKind::DataModel);
    }

    #[test]
    fn float_precision_round_trips() {
        // Finite f64 values must survive the YAML emit/parse pair
        // without drift — consumers compare manifest/job numerics.
        let value = Floats::precise();
        let mut bytes = Vec::new();
        yaml::to_writer(&mut bytes, &value, &yaml::YamlWriteOptions::new()).expect("serialize");
        let mut slice = bytes.as_slice();
        let back: Floats =
            yaml::from_reader(&mut slice, &yaml::YamlReadOptions::default()).expect("parse");
        assert_eq!(back, value);
        for (a, b) in [
            (back.a, value.a),
            (back.b, value.b),
            (back.c, value.c),
            (back.d, value.d),
        ] {
            assert_eq!(a.to_bits(), b.to_bits(), "f64 bits must round-trip exactly");
        }
    }
}

#[cfg(feature = "toml")]
mod toml_tests {
    use super::fixtures::{Floats, Manifest, NameDoc, StrictNameDoc};
    use ruviform_io_structured::{DocumentErrorKind, DocumentFormat, InputLimits, toml};
    use std::error::Error;

    #[test]
    fn round_trip_compact_and_pretty() {
        let manifest = Manifest::sample();
        for pretty in [false, true] {
            let mut bytes = Vec::new();
            toml::to_writer(
                &mut bytes,
                &manifest,
                &toml::TomlWriteOptions::new().with_pretty(pretty),
            )
            .expect("serialize");
            let mut slice = bytes.as_slice();
            let back: Manifest =
                toml::from_reader(&mut slice, &toml::TomlReadOptions::default()).expect("parse");
            assert_eq!(back, manifest, "pretty={pretty}");
        }
    }

    #[test]
    fn malformed_document_is_syntax() {
        let mut input = b"name = ".as_slice();
        let err = toml::from_reader::<NameDoc>(&mut input, &toml::TomlReadOptions::default())
            .expect_err("malformed input must fail");
        assert_eq!(err.kind(), DocumentErrorKind::Syntax);
        assert_eq!(err.format(), DocumentFormat::Toml);
        assert!(err.source().is_some());
        // The byte position of the parse failure is reported.
        assert!(err.position().is_some());
    }

    #[test]
    fn trailing_garbage_is_syntax_not_trailing_data() {
        // Documented TOML contract: bytes after a valid document are an
        // invalid continuation — the grammar has no document-end marker.
        let mut input = b"name = \"a\"\n@@@".as_slice();
        let err = toml::from_reader::<NameDoc>(&mut input, &toml::TomlReadOptions::default())
            .expect_err("trailing garbage must fail");
        assert_eq!(err.kind(), DocumentErrorKind::Syntax);
    }

    #[test]
    fn type_mismatch_is_data_model() {
        let mut input = b"name = 5".as_slice();
        let err = toml::from_reader::<NameDoc>(&mut input, &toml::TomlReadOptions::default())
            .expect_err("type mismatch must fail");
        assert_eq!(err.kind(), DocumentErrorKind::DataModel);
    }

    #[test]
    fn non_utf8_input_is_invalid_encoding() {
        let mut input = b"name = \"\xFF\xFE\"".as_slice();
        let err = toml::from_reader::<NameDoc>(&mut input, &toml::TomlReadOptions::default())
            .expect_err("non-UTF-8 input must fail");
        assert_eq!(err.kind(), DocumentErrorKind::InvalidEncoding);
        assert!(err.position().is_some_and(|p| p.byte.is_some()));
    }

    #[test]
    fn over_budget_is_limit_exceeded() {
        let mut input = b"name = \"abcdefghijklmnop\"".as_slice();
        let options = toml::TomlReadOptions::new().with_limits(InputLimits::new(8));
        let err = toml::from_reader::<NameDoc>(&mut input, &options)
            .expect_err("over-budget input must fail");
        assert_eq!(err.kind(), DocumentErrorKind::LimitExceeded);
        assert_eq!(err.limit().map(|l| l.name), Some("max_bytes"));
    }

    #[test]
    fn exact_budget_boundary_succeeds_and_one_more_byte_fails() {
        let doc = b"name = \"a\""; // exactly 10 bytes
        assert_eq!(doc.len(), 10);
        let options = toml::TomlReadOptions::new().with_limits(InputLimits::new(doc.len() as u64));
        let mut ok_input = doc.as_slice();
        let parsed: NameDoc =
            toml::from_reader(&mut ok_input, &options).expect("exact-budget doc parses");
        assert_eq!(parsed.name, "a");

        let mut over_input = b"name = \"a\"\n".as_slice();
        let err = toml::from_reader::<NameDoc>(&mut over_input, &options)
            .expect_err("input past the budget must fail");
        assert_eq!(err.kind(), DocumentErrorKind::LimitExceeded);
    }

    #[test]
    fn unicode_output_is_utf8() {
        let manifest = Manifest::sample();
        let mut bytes = Vec::new();
        toml::to_writer(&mut bytes, &manifest, &toml::TomlWriteOptions::default())
            .expect("serialize");
        assert!(str::from_utf8(&bytes).is_ok(), "TOML output is UTF-8");
    }

    #[test]
    fn display_does_not_embed_input_text() {
        // ERR-02: even though the TOML backend diagnostic (`source()`)
        // is reduced to its bare message, `DocumentError`'s own
        // `Display` never renders input content.
        let mut input = b"name = \"MARKER7f3a9c".as_slice();
        let err = toml::from_reader::<NameDoc>(&mut input, &toml::TomlReadOptions::default())
            .expect_err("unterminated string must fail");
        let rendered = err.to_string();
        assert!(
            !rendered.contains("MARKER7f3a9c"),
            "Display must not echo input text: {rendered}"
        );
        assert!(rendered.contains("toml"), "{rendered}");
    }

    #[test]
    fn unknown_fields_are_ignored_unless_the_consumer_denies_them() {
        let mut input = b"name = \"a\"\nsurprise = 1\n".as_slice();
        let doc: NameDoc = toml::from_reader(&mut input, &toml::TomlReadOptions::default())
            .expect("lenient parse");
        assert_eq!(doc.name, "a");

        let mut strict_input = b"name = \"a\"\nsurprise = 1\n".as_slice();
        let err = toml::from_reader::<StrictNameDoc>(
            &mut strict_input,
            &toml::TomlReadOptions::default(),
        )
        .expect_err("deny_unknown_fields must reject");
        assert_eq!(err.kind(), DocumentErrorKind::DataModel);
    }

    #[test]
    fn float_precision_round_trips_bit_exact() {
        let value = Floats::precise();
        let mut bytes = Vec::new();
        toml::to_writer(&mut bytes, &value, &toml::TomlWriteOptions::default()).expect("serialize");
        let mut slice = bytes.as_slice();
        let back: Floats =
            toml::from_reader(&mut slice, &toml::TomlReadOptions::default()).expect("parse");
        assert_eq!(back, value);
        for (a, b) in [
            (back.a, value.a),
            (back.b, value.b),
            (back.c, value.c),
            (back.d, value.d),
        ] {
            assert_eq!(a.to_bits(), b.to_bits(), "f64 bits must round-trip exactly");
        }
    }
}

#[cfg(all(feature = "json", feature = "file"))]
mod file_tests {
    use super::fixtures::Manifest;
    use ruviform_io_file::{
        Durability, PublishEffect, PublishFailureKind, PublishMode, PublishOptions,
    };
    use ruviform_io_structured::{DocumentFileReadError, DocumentFileWriteError, json};
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;

    fn workdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rvf-structured-{tag}-{}-{:x}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        fs::create_dir_all(&dir).expect("workdir");
        dir
    }

    struct DirGuard(PathBuf);
    impl Drop for DirGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn write_then_read_round_trip() {
        let dir = workdir("roundtrip");
        let _guard = DirGuard(dir.clone());
        let dest = dir.join("doc.json");
        let manifest = Manifest::sample();

        json::write_file(
            &dest,
            &manifest,
            &json::JsonWriteOptions::default(),
            PublishOptions::new(PublishMode::CreateNew, Durability::FileSync),
        )
        .expect("publish");

        let back: Manifest =
            json::read_file(&dest, &json::JsonReadOptions::default()).expect("read back");
        assert_eq!(back, manifest);
    }

    #[test]
    fn read_file_missing_is_open_error() {
        let dir = workdir("missing");
        let _guard = DirGuard(dir.clone());
        let err =
            json::read_file::<Manifest>(dir.join("absent.json"), &json::JsonReadOptions::default())
                .expect_err("missing file must fail");
        assert!(matches!(err, DocumentFileReadError::Open(_)));
    }

    #[test]
    fn read_file_bad_content_is_decode_error() {
        let dir = workdir("badcontent");
        let _guard = DirGuard(dir.clone());
        let dest = dir.join("doc.json");
        fs::write(&dest, b"{not json}").expect("seed");
        let err = json::read_file::<Manifest>(&dest, &json::JsonReadOptions::default())
            .expect_err("bad content must fail");
        assert!(matches!(err, DocumentFileReadError::Decode(_)));
    }

    #[test]
    fn encode_failure_is_encode_and_never_publishes() {
        let dir = workdir("encodefail");
        let _guard = DirGuard(dir.clone());
        let dest = dir.join("doc.json");
        // A tuple key cannot be a JSON object key — encode fails
        // inside the staged writer before publish.
        let mut bad: HashMap<(i32, i32), String> = HashMap::new();
        bad.insert((1, 2), "x".to_owned());

        let err = json::write_file(
            &dest,
            &bad,
            &json::JsonWriteOptions::default(),
            PublishOptions::new(PublishMode::CreateNew, Durability::FileSync),
        )
        .expect_err("unserializable value must fail");
        assert!(matches!(err, DocumentFileWriteError::Encode(_)));
        // No partial document was installed at the destination.
        assert!(!dest.exists());
        // Best-effort staged cleanup removed the temp too.
        let leftovers = fs::read_dir(&dir)
            .expect("read_dir")
            .filter_map(Result::ok)
            .count();
        assert_eq!(leftovers, 0, "staged temp should be cleaned up on drop");
    }

    #[test]
    fn publish_failure_preserves_full_publish_failure_state() {
        let dir = workdir("pubfail");
        let _guard = DirGuard(dir.clone());
        let dest = dir.join("doc.json");
        fs::write(&dest, b"existing").expect("seed destination");

        let manifest = Manifest::sample();
        let mut err = json::write_file(
            &dest,
            &manifest,
            &json::JsonWriteOptions::default(),
            PublishOptions::new(PublishMode::CreateNew, Durability::FileSync),
        )
        .expect_err("existing destination under CreateNew must fail");

        let DocumentFileWriteError::Publish(failure) = &mut err else {
            panic!("expected Publish, got {err:?}");
        };
        // The full PublishFailure contract survives: kind, phase and
        // the honest NotPublished effect.
        assert_eq!(failure.kind(), PublishFailureKind::DestinationExists);
        assert_eq!(failure.effect(), PublishEffect::NotPublished);
        // Recovery info / handle stay attached for the caller.
        let _ = failure.info();
        let _ = failure.take_recovery_handle();
        // The pre-existing destination content is untouched.
        assert_eq!(fs::read(&dest).expect("read dest"), b"existing");
    }

    #[test]
    fn file_error_source_chain_is_reachable() {
        // ERR-01: every file-convenience failure keeps its inner cause
        // reachable through `Error::source()` — Open → io::Error,
        // Decode/Encode → DocumentError, Publish → PublishFailure.
        use std::error::Error as _;

        let dir = workdir("sourcechain");
        let _guard = DirGuard(dir.clone());

        let open_err =
            json::read_file::<Manifest>(dir.join("absent.json"), &json::JsonReadOptions::default())
                .expect_err("missing file must fail");
        assert!(open_err.source().is_some(), "Open keeps io::Error source");

        let bad = dir.join("bad.json");
        fs::write(&bad, b"{oops").expect("seed");
        let decode_err = json::read_file::<Manifest>(&bad, &json::JsonReadOptions::default())
            .expect_err("bad content must fail");
        assert!(
            decode_err.source().is_some(),
            "Decode keeps DocumentError source"
        );

        let mut bad_map: HashMap<(i32, i32), String> = HashMap::new();
        bad_map.insert((1, 2), "x".to_owned());
        let encode_err = json::write_file(
            dir.join("enc.json"),
            &bad_map,
            &json::JsonWriteOptions::default(),
            PublishOptions::new(PublishMode::CreateNew, Durability::FileSync),
        )
        .expect_err("unserializable value must fail");
        assert!(
            encode_err.source().is_some(),
            "Encode keeps DocumentError source"
        );

        let pub_dest = dir.join("pub.json");
        fs::write(&pub_dest, b"existing").expect("seed destination");
        let publish_err = json::write_file(
            &pub_dest,
            &Manifest::sample(),
            &json::JsonWriteOptions::default(),
            PublishOptions::new(PublishMode::CreateNew, Durability::FileSync),
        )
        .expect_err("existing destination must fail");
        assert!(
            publish_err.source().is_some(),
            "Publish keeps PublishFailure source"
        );
    }
}

#[cfg(feature = "csv")]
mod csv_tests {
    use super::fixtures::Row;
    use ruviform_io_structured::{
        DocumentErrorKind, DocumentFormat, InputLimits,
        csv::{self, CsvBomPolicy, CsvDialect, CsvReadOptions, CsvRecord, CsvTerminator, CsvTrim},
    };
    use std::error::Error;
    use std::io::Read;

    fn read_options() -> CsvReadOptions {
        CsvReadOptions::default()
    }

    #[test]
    fn typed_round_trip() {
        let rows = [
            Row {
                city: "Boston".into(),
                pop: 4_628_910,
            },
            Row {
                city: "Concord".into(),
                pop: 42_695,
            },
        ];
        let mut bytes = Vec::new();
        csv::to_writer(&mut bytes, &rows, &csv::CsvWriteOptions::default()).expect("serialize");
        // Default terminator is RFC-4180 CRLF and `has_headers` emits
        // the field-name row.
        assert_eq!(bytes, b"city,pop\r\nBoston,4628910\r\nConcord,42695\r\n");
        let mut slice = bytes.as_slice();
        let back: Vec<Row> = csv::from_reader(&mut slice, &read_options()).expect("parse");
        assert_eq!(back, rows);
    }

    #[test]
    fn read_record_yields_fields_and_positions() {
        let mut input = b"h1,h2\na,b\nc,d\n".as_slice();
        let mut reader = csv::CsvReader::new(&mut input, &read_options()).expect("reader");
        let mut record = CsvRecord::new();
        assert!(reader.read_record(&mut record).expect("record 0"));
        assert_eq!(record.len(), 2);
        assert_eq!(record.field_bytes(0), Some(&b"a"[..]));
        assert_eq!(record.field_bytes(2), None);
        let pos = record.position().expect("position");
        assert_eq!(pos.byte, Some(6));
        assert_eq!(pos.line, Some(2));
        assert_eq!(pos.column, None);
        assert_eq!(pos.record, Some(0));
        assert!(reader.read_record(&mut record).expect("record 1"));
        let pos = record.position().expect("position");
        assert_eq!(pos.byte, Some(10));
        assert_eq!(pos.line, Some(3));
        assert_eq!(pos.record, Some(1));
        // Strict EOF: drained input reports Ok(false), repeatably.
        assert!(!reader.read_record(&mut record).expect("eof"));
        assert!(!reader.read_record(&mut record).expect("eof again"));
        assert_eq!(reader.record_index(), 2);
    }

    #[test]
    fn headers_are_exposed_and_skipped() {
        let mut input = b"city,pop\nBoston,1\n".as_slice();
        let mut reader = csv::CsvReader::new(&mut input, &read_options()).expect("reader");
        let headers = reader.headers().expect("headers").expect("present");
        assert_eq!(headers.field_str(0).expect("utf8"), Some("city"));
        assert_eq!(headers.field_str(1).expect("utf8"), Some("pop"));
        assert_eq!(reader.record_index(), 0, "headers do not count as records");
    }

    #[test]
    fn no_headers_yields_first_row() {
        let mut input = b"a,b\nc,d\n".as_slice();
        let options = read_options().with_has_headers(false);
        let mut reader = csv::CsvReader::new(&mut input, &options).expect("reader");
        assert!(reader.headers().expect("headers").is_none());
        let mut record = CsvRecord::new();
        assert!(reader.read_record(&mut record).expect("record"));
        assert_eq!(record.field_bytes(0), Some(&b"a"[..]));
    }

    #[test]
    fn positional_typed_records() {
        #[derive(Debug, PartialEq, serde::Deserialize)]
        struct Pair(String, u64);
        let mut input = b"Boston,4628910\n".as_slice();
        let options = read_options().with_has_headers(false);
        let mut reader = csv::CsvReader::new(&mut input, &options).expect("reader");
        let pair = reader.read_typed::<Pair>().expect("typed").expect("record");
        assert_eq!(pair, Pair("Boston".into(), 4_628_910));
        assert!(reader.read_typed::<Pair>().expect("eof").is_none());
    }

    #[test]
    fn quoted_newlines_and_quotes() {
        let mut input = b"\"a\nb\",\"say \"\"hi\"\"\"\n".as_slice();
        let options = read_options().with_has_headers(false);
        let mut reader = csv::CsvReader::new(&mut input, &options).expect("reader");
        let mut record = CsvRecord::new();
        assert!(reader.read_record(&mut record).expect("record"));
        assert_eq!(record.field_str(0).expect("utf8"), Some("a\nb"));
        assert_eq!(record.field_str(1).expect("utf8"), Some("say \"hi\""));
        assert!(!reader.read_record(&mut record).expect("eof"));
    }

    #[test]
    fn crlf_lf_and_cr_terminators() {
        let mut input = b"a,b\r\nc,d\ne,f\r".as_slice();
        let options = read_options().with_has_headers(false);
        let mut reader = csv::CsvReader::new(&mut input, &options).expect("reader");
        let mut record = CsvRecord::new();
        let mut seen = Vec::new();
        while reader.read_record(&mut record).expect("record") {
            seen.push((
                record
                    .field_str(0)
                    .expect("utf8")
                    .expect("field")
                    .to_owned(),
                record
                    .field_str(1)
                    .expect("utf8")
                    .expect("field")
                    .to_owned(),
            ));
        }
        assert_eq!(
            seen,
            [
                ("a".to_owned(), "b".to_owned()),
                ("c".to_owned(), "d".to_owned()),
                ("e".to_owned(), "f".to_owned())
            ]
        );
    }

    #[test]
    fn utf8_bom_stripped_by_default() {
        let mut input = b"\xEF\xBB\xBFa,b\n".as_slice();
        let options = read_options().with_has_headers(false);
        let mut reader = csv::CsvReader::new(&mut input, &options).expect("reader");
        let mut record = CsvRecord::new();
        assert!(reader.read_record(&mut record).expect("record"));
        assert_eq!(record.field_bytes(0), Some(&b"a"[..]));
        // The stripped BOM still counts toward consumed bytes.
        assert_eq!(record.position().expect("pos").byte, Some(3));
    }

    #[test]
    fn utf8_bom_preserved_when_requested() {
        let mut input = b"\xEF\xBB\xBFa,b\n".as_slice();
        let options = read_options()
            .with_has_headers(false)
            .with_bom(CsvBomPolicy::Preserve);
        assert_eq!(options.bom, CsvBomPolicy::Preserve);
        let mut reader = csv::CsvReader::new(&mut input, &options).expect("reader");
        let mut record = CsvRecord::new();
        assert!(reader.read_record(&mut record).expect("record"));
        assert_eq!(record.field_bytes(0), Some(&b"\xEF\xBB\xBFa"[..]));
    }

    #[test]
    fn utf16_bom_is_invalid_encoding() {
        for bom in [b"\xFF\xFE".as_slice(), b"\xFE\xFF".as_slice()] {
            let input = [bom, b"a,b\n".as_slice()].concat();
            let mut slice = input.as_slice();
            let err =
                csv::CsvReader::new(&mut slice, &read_options()).expect_err("UTF-16 BOM must fail");
            assert_eq!(err.kind(), DocumentErrorKind::InvalidEncoding);
            assert_eq!(err.format(), DocumentFormat::Csv);
        }
    }

    #[test]
    fn empty_lines_are_skipped_and_empty_fields_kept() {
        let mut input = b"\n\na,b\n\n,\n".as_slice();
        let options = read_options().with_has_headers(false);
        let mut reader = csv::CsvReader::new(&mut input, &options).expect("reader");
        let mut record = CsvRecord::new();
        assert!(reader.read_record(&mut record).expect("record"));
        assert_eq!(record.len(), 2);
        assert!(reader.read_record(&mut record).expect("record"));
        assert_eq!(record.len(), 2, "\",\" is a two-field record");
        assert_eq!(record.field_bytes(0), Some(&b""[..]));
        assert!(!reader.read_record(&mut record).expect("eof"));
    }

    #[test]
    fn dialects_tsv_semicolon_quote_comment_and_terminator() {
        // TSV.
        let mut input = b"a\tb\nc\td\n".as_slice();
        let options = read_options()
            .with_has_headers(false)
            .with_dialect(CsvDialect::new().with_delimiter(b'\t'));
        let mut reader = csv::CsvReader::new(&mut input, &options).expect("reader");
        let mut record = CsvRecord::new();
        assert!(reader.read_record(&mut record).expect("record"));
        assert_eq!(record.field_bytes(1), Some(&b"b"[..]));

        // Custom quote + semicolon delimiter + `#` comments.
        let mut input = b"#note\n'a;b';c\n".as_slice();
        let options = read_options().with_has_headers(false).with_dialect(
            CsvDialect::new()
                .with_delimiter(b';')
                .with_quote(b'\'')
                .with_comment(Some(b'#')),
        );
        let mut reader = csv::CsvReader::new(&mut input, &options).expect("reader");
        let mut record = CsvRecord::new();
        assert!(reader.read_record(&mut record).expect("record"));
        assert_eq!(record.field_str(0).expect("utf8"), Some("a;b"));
        assert_eq!(record.field_bytes(1), Some(&b"c"[..]));

        // Custom terminator byte.
        let mut input = b"a,b;c,d;".as_slice();
        let options = read_options()
            .with_has_headers(false)
            .with_dialect(CsvDialect::new().with_terminator(CsvTerminator::Any(b';')));
        let mut reader = csv::CsvReader::new(&mut input, &options).expect("reader");
        let mut count = 0;
        while reader.read_record(&mut record).expect("record") {
            count += 1;
        }
        assert_eq!(count, 2);

        // Non-quoting dialect treats quotes as data.
        let mut input = b"a,\"b\"\n".as_slice();
        let options = read_options()
            .with_has_headers(false)
            .with_dialect(CsvDialect::new().with_quoting(false));
        let mut reader = csv::CsvReader::new(&mut input, &options).expect("reader");
        assert!(reader.read_record(&mut record).expect("record"));
        assert_eq!(record.field_bytes(1), Some(&b"\"b\""[..]));
    }

    #[test]
    fn record_buffer_is_reused_across_reads() {
        let mut input = b"h\none\ntwo\nthree\n".as_slice();
        let mut reader = csv::CsvReader::new(&mut input, &read_options()).expect("reader");
        let mut record = CsvRecord::new();
        for expected in ["one", "two", "three"] {
            assert!(reader.read_record(&mut record).expect("record"));
            assert_eq!(record.field_str(0).expect("utf8"), Some(expected));
        }
        assert!(!reader.read_record(&mut record).expect("eof"));
    }

    #[test]
    fn lookahead_is_bounded_by_chunk_and_probe() {
        // ~30 KiB of input — the reader must not drain it eagerly.
        let mut doc = b"h1,h2\n".to_vec();
        doc.extend(std::iter::repeat_n(b"a,b\n".as_slice(), 7_500).flatten());
        let mut cursor = std::io::Cursor::new(doc);
        let mut reader = csv::CsvReader::new(&mut cursor, &read_options()).expect("reader");
        let mut record = CsvRecord::new();
        assert!(reader.read_record(&mut record).expect("record"));
        // Lookahead = BOM probe (<=4 bytes) + one 8 KiB input chunk.
        assert!(
            cursor.position() <= 4 + 8 * 1024,
            "reader must not pull the whole input: position {}",
            cursor.position()
        );
    }

    #[test]
    fn reader_borrows_then_releases() {
        let mut input = b"a,b\nc,d\n".as_slice();
        {
            let mut reader =
                csv::CsvReader::new(&mut input, &read_options().with_has_headers(false))
                    .expect("reader");
            let mut record = CsvRecord::new();
            assert!(reader.read_record(&mut record).expect("record"));
        }
        // Borrowed reader released — the slice remains usable.
        let _ = input.len();
    }

    #[test]
    fn over_byte_budget_is_limit_exceeded() {
        let mut input = b"a,b\nc,d\n".as_slice();
        let options = read_options()
            .with_has_headers(false)
            .with_limits(InputLimits::new(5));
        let mut reader = csv::CsvReader::new(&mut input, &options).expect("reader");
        let mut record = CsvRecord::new();
        assert!(reader.read_record(&mut record).expect("first record fits"));
        let err = reader.read_record(&mut record).expect_err("over budget");
        assert_eq!(err.kind(), DocumentErrorKind::LimitExceeded);
        assert_eq!(err.limit().expect("limit").name, "max_bytes");
        assert_eq!(err.limit().expect("limit").max, 5);
    }

    #[test]
    fn exact_budget_boundary_succeeds_and_one_more_byte_fails() {
        let doc = b"a,b".as_slice(); // exactly 3 bytes, no terminator
        let options = read_options()
            .with_has_headers(false)
            .with_limits(InputLimits::new(3));
        let mut input = doc;
        let mut reader = csv::CsvReader::new(&mut input, &options).expect("reader");
        let mut record = CsvRecord::new();
        assert!(reader.read_record(&mut record).expect("record"));
        assert_eq!(record.len(), 2);
        assert!(!reader.read_record(&mut record).expect("eof"));

        // One byte over the budget is caught by the BOM probe at
        // construction — the byte is beyond `max_bytes`.
        let mut input = b"a,bX".as_slice();
        let Err(err) = csv::CsvReader::new(&mut input, &options) else {
            panic!("one byte over budget must fail");
        };
        assert_eq!(err.kind(), DocumentErrorKind::LimitExceeded);
    }

    #[test]
    fn oversized_record_is_limit_exceeded() {
        let mut input = b"abcdef,gh\n".as_slice();
        let options = read_options()
            .with_has_headers(false)
            .with_max_record_bytes(4);
        let mut reader = csv::CsvReader::new(&mut input, &options).expect("reader");
        let mut record = CsvRecord::new();
        let err = reader
            .read_record(&mut record)
            .expect_err("oversized record");
        assert_eq!(err.kind(), DocumentErrorKind::LimitExceeded);
        assert_eq!(err.limit().expect("limit").name, "max_record_bytes");
        assert_eq!(err.limit().expect("limit").max, 4);
    }

    #[test]
    fn oversized_field_is_limit_exceeded() {
        let mut input = b"abcd,e\n".as_slice();
        let options = read_options()
            .with_has_headers(false)
            .with_max_field_bytes(3);
        let mut reader = csv::CsvReader::new(&mut input, &options).expect("reader");
        let mut record = CsvRecord::new();
        let err = reader
            .read_record(&mut record)
            .expect_err("oversized field");
        assert_eq!(err.kind(), DocumentErrorKind::LimitExceeded);
        assert_eq!(err.limit().expect("limit").name, "max_field_bytes");
    }

    #[test]
    fn too_many_fields_is_limit_exceeded() {
        let mut input = b"a,b,c\n".as_slice();
        let options = read_options().with_has_headers(false).with_max_fields(2);
        let mut reader = csv::CsvReader::new(&mut input, &options).expect("reader");
        let mut record = CsvRecord::new();
        let err = reader
            .read_record(&mut record)
            .expect_err("too many fields");
        assert_eq!(err.kind(), DocumentErrorKind::LimitExceeded);
        assert_eq!(err.limit().expect("limit").name, "max_fields");
        assert_eq!(err.limit().expect("limit").max, 2);
    }

    #[test]
    fn too_many_records_is_limit_exceeded() {
        let mut input = b"h\na\nb\n".as_slice();
        let options = read_options().with_max_records(1);
        let mut reader = csv::CsvReader::new(&mut input, &options).expect("reader");
        let mut record = CsvRecord::new();
        assert!(reader.read_record(&mut record).expect("record 0"));
        let err = reader.read_record(&mut record).expect_err("record 1 over");
        assert_eq!(err.kind(), DocumentErrorKind::LimitExceeded);
        assert_eq!(err.limit().expect("limit").name, "max_records");
        assert_eq!(err.limit().expect("limit").max, 1);
    }

    #[test]
    fn unclosed_quote_yields_final_record_leniently() {
        // Permissive grammar: `"abc` at EOF is a complete last
        // record — pinned so the contract is explicit.
        let mut input = b"h1\n\"abc\n".as_slice();
        let mut reader = csv::CsvReader::new(&mut input, &read_options()).expect("reader");
        let mut record = CsvRecord::new();
        assert!(reader.read_record(&mut record).expect("record"));
        assert_eq!(record.field_str(0).expect("utf8"), Some("abc\n"));
        assert!(!reader.read_record(&mut record).expect("eof"));
    }

    #[test]
    fn non_utf8_field_str_is_invalid_encoding() {
        let mut input = b"a,\xFF\xFE\n".as_slice();
        let options = read_options().with_has_headers(false);
        let mut reader = csv::CsvReader::new(&mut input, &options).expect("reader");
        let mut record = CsvRecord::new();
        assert!(reader.read_record(&mut record).expect("record"));
        assert_eq!(record.field_bytes(1), Some(&b"\xFF\xFE"[..]));
        let err = record.field_str(1).expect_err("non-UTF8 must fail");
        assert_eq!(err.kind(), DocumentErrorKind::InvalidEncoding);
    }

    #[test]
    fn non_utf8_typed_record_is_invalid_encoding() {
        #[derive(Debug, PartialEq, serde::Deserialize)]
        struct StrRow(String);
        let mut input = b"x\nok\n\xFF\n".as_slice();
        let mut reader = csv::CsvReader::new(&mut input, &read_options()).expect("reader");
        let row = reader
            .read_typed::<StrRow>()
            .expect("typed")
            .expect("record");
        assert_eq!(row, StrRow("ok".into()));
        let err = reader
            .read_typed::<StrRow>()
            .expect_err("non-UTF8 typed field must fail");
        assert_eq!(err.kind(), DocumentErrorKind::InvalidEncoding);
    }

    #[test]
    fn ragged_records_are_data_model_unless_flexible() {
        let mut input = b"a,b\nx,y,z\n".as_slice();
        let mut reader = csv::CsvReader::new(&mut input, &read_options()).expect("reader");
        let mut record = CsvRecord::new();
        let err = reader.read_record(&mut record).expect_err("ragged");
        assert_eq!(err.kind(), DocumentErrorKind::DataModel);
        assert_eq!(err.position().expect("pos").record, Some(0));

        let mut input = b"a,b\nx,y,z\n".as_slice();
        let options = read_options().with_flexible(true);
        let mut reader = csv::CsvReader::new(&mut input, &options).expect("reader");
        let mut record = CsvRecord::new();
        assert!(reader.read_record(&mut record).expect("record"));
        assert_eq!(record.len(), 3);
    }

    #[test]
    fn typed_type_mismatch_is_data_model_not_truncated() {
        let mut input = b"city,pop\nBoston,notanumber\n".as_slice();
        let mut reader = csv::CsvReader::new(&mut input, &read_options()).expect("reader");
        let err = reader.read_typed::<Row>().expect_err("bad pop must fail");
        assert_eq!(err.kind(), DocumentErrorKind::DataModel);
        assert_eq!(err.format(), DocumentFormat::Csv);
        assert!(err.source().is_some());
    }

    #[test]
    fn contradictory_dialect_is_unsupported() {
        let options =
            read_options().with_dialect(CsvDialect::new().with_delimiter(b'\'').with_quote(b'\''));
        let mut input = b"a,b\n".as_slice();
        let err = csv::CsvReader::new(&mut input, &options).expect_err("dialect must fail");
        assert_eq!(err.kind(), DocumentErrorKind::Unsupported);
    }

    #[test]
    fn io_failure_on_read_is_io() {
        #[derive(Debug)]
        struct Failing;
        impl Read for Failing {
            fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "nope",
                ))
            }
        }
        let mut reader = Failing;
        let err = csv::CsvReader::new(&mut reader, &read_options()).expect_err("io must fail");
        assert_eq!(err.kind(), DocumentErrorKind::Io);
    }

    #[test]
    fn streaming_reads_incrementally_before_tail_failure() {
        // A failing tail must not prevent earlier records from being
        // readable — proves incremental consumption.
        struct FailAfter {
            inner: std::io::Cursor<Vec<u8>>,
            remaining: usize,
        }
        impl Read for FailAfter {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                if self.remaining == 0 {
                    return Err(std::io::Error::other("tail failure"));
                }
                let take = buf.len().min(self.remaining);
                let n = self.inner.read(&mut buf[..take])?;
                self.remaining -= n;
                Ok(n)
            }
        }
        let mut doc = b"h\none\ntwo\n".to_vec();
        doc.extend(std::iter::repeat_n(b"more\n".as_slice(), 1_000).flatten());
        let mut source = FailAfter {
            inner: std::io::Cursor::new(doc),
            remaining: 6,
        };
        let mut reader = csv::CsvReader::new(&mut source, &read_options()).expect("reader");
        let mut record = CsvRecord::new();
        assert!(reader.read_record(&mut record).expect("record before tail"));
        assert_eq!(record.field_str(0).expect("utf8"), Some("one"));
        let err = reader
            .read_record(&mut record)
            .expect_err("tail failure surfaces");
        assert_eq!(err.kind(), DocumentErrorKind::Io);
    }

    #[test]
    fn trim_fields_and_headers() {
        let mut input = b" h1 , h2 \n a , b \n".as_slice();
        let options = read_options().with_trim(CsvTrim::All);
        let mut reader = csv::CsvReader::new(&mut input, &options).expect("reader");
        let headers = reader.headers().expect("headers").expect("present");
        assert_eq!(headers.field_str(0).expect("utf8"), Some("h1"));
        let mut record = CsvRecord::new();
        assert!(reader.read_record(&mut record).expect("record"));
        assert_eq!(record.field_str(0).expect("utf8"), Some("a"));
        assert_eq!(record.field_str(1).expect("utf8"), Some("b"));
    }

    #[test]
    fn formula_like_fields_pass_through_verbatim() {
        // Policy boundary: `=`-leading fields are data, never
        // sanitized.
        let mut input = b"expr\n=1+1\n".as_slice();
        let mut reader = csv::CsvReader::new(&mut input, &read_options()).expect("reader");
        let mut record = CsvRecord::new();
        assert!(reader.read_record(&mut record).expect("record"));
        assert_eq!(record.field_str(0).expect("utf8"), Some("=1+1"));
    }

    #[test]
    fn write_record_and_finish() {
        let mut out = Vec::new();
        {
            let mut writer =
                csv::CsvWriter::new(&mut out, &csv::CsvWriteOptions::default()).expect("writer");
            writer.write_record(["a", "b"]).expect("record");
            writer.write_record(["x", "y"]).expect("record");
            writer.finish().expect("finish");
        }
        assert_eq!(out, b"a,b\r\nx,y\r\n");
    }

    #[test]
    fn write_ragged_is_data_model() {
        let mut out = Vec::new();
        let mut writer =
            csv::CsvWriter::new(&mut out, &csv::CsvWriteOptions::default()).expect("writer");
        writer.write_record(["a", "b"]).expect("record");
        let err = writer
            .write_record(["x", "y", "z"])
            .expect_err("ragged write");
        assert_eq!(err.kind(), DocumentErrorKind::DataModel);
        writer.finish().expect("finish");
    }

    #[test]
    fn write_failure_on_finish_is_io() {
        struct Failing;
        impl std::io::Write for Failing {
            fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
                Ok(0)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Err(std::io::Error::other("flush failed"))
            }
        }
        let mut out = Failing;
        let mut writer =
            csv::CsvWriter::new(&mut out, &csv::CsvWriteOptions::default()).expect("writer");
        writer.write_record(["a"]).expect("record");
        let err = writer.finish().expect_err("flush must fail");
        assert_eq!(err.kind(), DocumentErrorKind::Io);
    }

    #[test]
    fn deterministic_dialect_fuzz() {
        // Seeded generator: random dialect CSV documents — the reader
        // must never panic and every error must be a classified
        // DocumentError.
        let mut state = 0x9E37_79B9_7F4A_7C15u64;
        let mut rng = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let alphabet: &[u8] = b"ab,\"'\t\r\n ;=#@\xFF\xEF";
        let idx = |v: u64, len: usize| usize::try_from(v % len as u64).unwrap_or(0);
        for round in 0..200 {
            let mut doc = Vec::new();
            for _ in 0..idx(rng(), 96) {
                doc.push(alphabet[idx(rng(), alphabet.len())]);
            }
            let dialect = CsvDialect::new()
                .with_delimiter(alphabet[idx(rng(), 8)])
                .with_quote(alphabet[idx(rng(), 8)]);
            let options = read_options()
                .with_has_headers(rng() % 2 == 0)
                .with_flexible(rng() % 2 == 0)
                .with_dialect(dialect)
                .with_max_record_bytes(4 * 1024)
                .with_max_field_bytes(1024)
                .with_max_fields(256)
                .with_max_records(512);
            let mut input = doc.as_slice();
            let Ok(mut reader) = csv::CsvReader::new(&mut input, &options) else {
                continue;
            };
            let mut record = CsvRecord::new();
            loop {
                match reader.read_record(&mut record) {
                    Ok(true) => {
                        for i in 0..record.len() {
                            let _ = record.field_bytes(i);
                            let _ = record.field_str(i);
                        }
                    }
                    Ok(false) => break,
                    Err(err) => {
                        assert!(
                            matches!(
                                err.kind(),
                                DocumentErrorKind::LimitExceeded
                                    | DocumentErrorKind::DataModel
                                    | DocumentErrorKind::Io
                            ),
                            "round {round}: unexpected read error {:?}",
                            err.kind()
                        );
                        break;
                    }
                }
            }
        }
    }

    #[test]
    fn deterministic_truncation_fuzz() {
        // Every random prefix of a valid document must parse without
        // panic.
        let doc = b"city,pop\nBoston,4628910\nConcord,42695\n\"x\ny\",1\n";
        let mut state = 0xD1B5_4A32_D192_ED03u64;
        let mut rng = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for _ in 0..doc.len() {
            let cut = usize::try_from(rng() % doc.len() as u64).unwrap_or(0);
            let mut input = &doc[..cut];
            let options = read_options();
            if let Ok(mut reader) = csv::CsvReader::new(&mut input, &options) {
                let mut record = CsvRecord::new();
                while matches!(reader.read_record(&mut record), Ok(true)) {}
            }
        }
    }

    #[test]
    fn display_does_not_embed_field_text() {
        // ERR-02: a typed-record failure reports the normalized
        // classification — the offending field's bytes stay inside the
        // erased `source()` diagnostic, never in `Display`.
        let mut input = b"city,pop\nBoston,MARKER7f3a9c\n".as_slice();
        let mut reader = csv::CsvReader::new(&mut input, &read_options()).expect("reader");
        let err = reader
            .read_typed::<Row>()
            .expect_err("non-numeric pop must fail");
        let rendered = err.to_string();
        assert!(
            !rendered.contains("MARKER7f3a9c"),
            "Display must not echo field text: {rendered}"
        );
        assert!(rendered.contains("csv"), "{rendered}");
    }
}

#[cfg(all(feature = "csv", feature = "file"))]
mod csv_file_tests {
    use super::fixtures::Row;
    use ruviform_io_file::{Durability, PublishMode, PublishOptions};
    use ruviform_io_structured::{DocumentFileReadError, DocumentFileWriteError, csv};
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;

    fn workdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rvf-structured-csv-{tag}-{}-{:x}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        fs::create_dir_all(&dir).expect("workdir");
        dir
    }

    struct DirGuard(PathBuf);
    impl Drop for DirGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// A nested map is not representable as a CSV cell — serializing it
    /// fails inside the staged writer before publish.
    #[derive(serde::Serialize)]
    struct Unserializable {
        map: HashMap<String, String>,
    }

    fn rows() -> [Row; 2] {
        [
            Row {
                city: "Boston".into(),
                pop: 4_628_910,
            },
            Row {
                city: "Concord".into(),
                pop: 42_695,
            },
        ]
    }

    #[test]
    fn write_then_read_round_trip() {
        let dir = workdir("roundtrip");
        let _guard = DirGuard(dir.clone());
        let dest = dir.join("doc.csv");
        let rows = rows();
        csv::write_file(
            &dest,
            &rows,
            &csv::CsvWriteOptions::default(),
            PublishOptions::new(PublishMode::CreateNew, Durability::FileSync),
        )
        .expect("publish");
        let back: Vec<Row> =
            csv::read_file(&dest, &csv::CsvReadOptions::default()).expect("read back");
        assert_eq!(back, rows);
    }

    #[test]
    fn read_file_missing_is_open_error() {
        let dir = workdir("missing");
        let _guard = DirGuard(dir.clone());
        let err =
            csv::read_file::<Vec<Row>>(dir.join("absent.csv"), &csv::CsvReadOptions::default())
                .expect_err("missing file must fail");
        assert!(matches!(err, DocumentFileReadError::Open(_)));
    }

    #[test]
    fn read_file_bad_content_is_decode_error() {
        let dir = workdir("badcontent");
        let _guard = DirGuard(dir.clone());
        let dest = dir.join("doc.csv");
        fs::write(&dest, b"\xFF\xFE bad utf16\n").expect("seed");
        let err = csv::read_file::<Vec<Row>>(&dest, &csv::CsvReadOptions::default())
            .expect_err("bad content must fail");
        assert!(matches!(err, DocumentFileReadError::Decode(_)));
    }

    #[test]
    fn encode_failure_is_encode_and_never_publishes() {
        let dir = workdir("encodefail");
        let _guard = DirGuard(dir.clone());
        let dest = dir.join("doc.csv");
        let mut map = HashMap::new();
        map.insert("k".to_owned(), "v".to_owned());
        let rows = [Unserializable { map }];
        let err = csv::write_file(
            &dest,
            &rows,
            &csv::CsvWriteOptions::default(),
            PublishOptions::new(PublishMode::CreateNew, Durability::FileSync),
        )
        .expect_err("unserializable record must fail");
        assert!(matches!(err, DocumentFileWriteError::Encode(_)));
        assert!(!dest.exists());
        let leftovers = fs::read_dir(&dir)
            .expect("read_dir")
            .filter_map(Result::ok)
            .count();
        assert_eq!(leftovers, 0, "staged temp should be cleaned up on drop");
    }

    #[test]
    fn publish_failure_is_publish() {
        let dir = workdir("pubfail");
        let _guard = DirGuard(dir.clone());
        let dest = dir.join("doc.csv");
        fs::write(&dest, b"existing").expect("seed destination");
        let rows = rows();
        let err = csv::write_file(
            &dest,
            &rows,
            &csv::CsvWriteOptions::default(),
            PublishOptions::new(PublishMode::CreateNew, Durability::FileSync),
        )
        .expect_err("existing destination under CreateNew must fail");
        assert!(matches!(err, DocumentFileWriteError::Publish(_)));
        assert_eq!(fs::read(&dest).expect("read dest"), b"existing");
    }
}

#[cfg(feature = "xml")]
mod xml_tests {
    use ruviform_io_structured::xml::{
        XmlEvent, XmlEventBuffer, XmlEventReader, XmlEventRef, XmlReadOptions, XmlReferenceValue,
    };
    use ruviform_io_structured::{DocumentError, DocumentErrorKind, DocumentFormat, InputLimits};
    use std::error::Error;
    use std::io::{self, BufRead, Read};

    fn options() -> XmlReadOptions {
        XmlReadOptions::default()
    }

    /// Drives a document to owned events for assertion convenience.
    fn read_all(doc: &[u8]) -> Result<Vec<XmlEvent>, DocumentError> {
        read_all_with(doc, &options())
    }

    fn read_all_with(doc: &[u8], options: &XmlReadOptions) -> Result<Vec<XmlEvent>, DocumentError> {
        let mut input = doc;
        let mut reader = XmlEventReader::new(&mut input, options);
        let mut buffer = XmlEventBuffer::new();
        let mut events = Vec::new();
        while let Some(event) = reader.read_event(&mut buffer)? {
            events.push(event.to_owned());
        }
        Ok(events)
    }

    /// First error of a document stream, or panic if it parses cleanly.
    fn read_err(doc: &[u8], options: &XmlReadOptions) -> DocumentError {
        let mut input = doc;
        let mut reader = XmlEventReader::new(&mut input, options);
        let mut buffer = XmlEventBuffer::new();
        loop {
            match reader.read_event(&mut buffer) {
                Ok(Some(_)) => {}
                Ok(None) => panic!("document parsed without error"),
                Err(error) => return error,
            }
        }
    }

    fn start(event: &XmlEvent) -> &ruviform_io_structured::xml::XmlElement {
        match event {
            XmlEvent::Start(e) | XmlEvent::Empty(e) => e,
            other => panic!("expected Start/Empty, got {other:?}"),
        }
    }

    // G1 — basic nested Start/Text/End order.
    #[test]
    fn nested_elements_stream_in_document_order() {
        let events = read_all(b"<root><a>hi</a><b/></root>").expect("parse");
        assert!(matches!(events[0], XmlEvent::Start(_)));
        assert!(matches!(events[1], XmlEvent::Start(_)));
        assert!(matches!(&events[2], XmlEvent::Text(t) if t == "hi"));
        assert!(matches!(&events[3], XmlEvent::End(n) if n.local_name() == "a"));
        assert!(matches!(events[4], XmlEvent::Empty(_)));
        assert!(matches!(&events[5], XmlEvent::End(n) if n.local_name() == "root"));
        assert_eq!(events.len(), 6);
    }

    // G2 — default and prefixed namespace resolution on elements.
    #[test]
    fn default_and_prefixed_namespaces_resolve_on_elements() {
        let events = read_all(br#"<root xmlns="urn:d" xmlns:p="urn:p"><p:c/><plain/></root>"#)
            .expect("parse");
        assert_eq!(start(&events[0]).name().namespace(), Some("urn:d"));
        let prefixed = start(&events[1]);
        assert_eq!(prefixed.name().prefix(), Some("p"));
        assert_eq!(prefixed.name().local_name(), "c");
        assert_eq!(prefixed.name().namespace(), Some("urn:p"));
        assert_eq!(start(&events[2]).name().namespace(), Some("urn:d"));
    }

    // G3 — default namespace does NOT apply to unprefixed attributes.
    #[test]
    fn default_namespace_does_not_apply_to_unprefixed_attributes() {
        let events = read_all(br#"<e xmlns="urn:d" a="1"/>"#).expect("parse");
        let element = start(&events[0]);
        assert_eq!(element.name().namespace(), Some("urn:d"));
        let attr = element
            .attributes()
            .iter()
            .find(|a| a.name().qualified_name() == "a")
            .expect("attribute a");
        assert_eq!(attr.name().namespace(), None);
        assert_eq!(attr.value(), "1");
    }

    // G4 — prefixed attributes and the reserved `xml` prefix resolve.
    #[test]
    fn prefixed_and_reserved_xml_attributes_resolve() {
        let events = read_all(br#"<e xmlns:p="urn:p" p:b="2" xml:lang="en"/>"#).expect("parse");
        let element = start(&events[0]);
        let attrs = element.attributes();
        let p_attr = attrs
            .iter()
            .find(|a| a.name().qualified_name() == "p:b")
            .expect("p:b attribute");
        assert_eq!(p_attr.name().namespace(), Some("urn:p"));
        let lang = attrs
            .iter()
            .find(|a| a.name().qualified_name() == "xml:lang")
            .expect("xml:lang attribute");
        assert_eq!(
            lang.name().namespace(),
            Some("http://www.w3.org/XML/1998/namespace")
        );
        // xmlns declarations stay visible in the attribute stream.
        assert!(attrs.iter().any(|a| a.name().qualified_name() == "xmlns:p"));
    }

    // G5 — `xmlns=""` resets the default namespace.
    #[test]
    fn empty_default_namespace_resets_binding() {
        let events = read_all(br#"<e xmlns="urn:d"><f xmlns=""><g/></f></e>"#).expect("parse");
        assert_eq!(start(&events[0]).name().namespace(), Some("urn:d"));
        assert_eq!(start(&events[1]).name().namespace(), None);
        assert_eq!(start(&events[2]).name().namespace(), None);
    }

    // G6 — borrowed events and owned conversions agree.
    #[test]
    fn borrowed_and_owned_events_are_equivalent() {
        let owned = read_all(br#"<e a="1">text</e>"#).expect("parse");
        assert!(matches!(&owned[0], XmlEvent::Start(_)));
        let element = start(&owned[0]);
        assert_eq!(element.name().qualified_name(), "e");
        assert_eq!(element.attributes()[0].value(), "1");
        assert!(matches!(&owned[1], XmlEvent::Text(t) if t == "text"));
        assert!(matches!(&owned[2], XmlEvent::End(n) if n.qualified_name() == "e"));
    }

    // G7 — raw CDATA content.
    #[test]
    fn cdata_content_is_preserved_raw() {
        let events = read_all(b"<e><![CDATA[raw <>& ]]></e>").expect("parse");
        assert!(matches!(&events[1], XmlEvent::CData(t) if t == "raw <>& "));
    }

    // G8 — text interleaved with predefined and numeric references.
    #[test]
    fn references_are_explicit_events_never_expanded() {
        let events = read_all(b"<e>a&lt;b&#65;c</e>").expect("parse");
        assert!(matches!(&events[1], XmlEvent::Text(t) if t == "a"));
        assert!(matches!(
            &events[2],
            XmlEvent::Reference(r) if r.value() == XmlReferenceValue::Entity("<")
        ));
        assert!(matches!(&events[3], XmlEvent::Text(t) if t == "b"));
        assert!(matches!(
            &events[4],
            XmlEvent::Reference(r) if r.value() == XmlReferenceValue::Char('A')
        ));
        assert!(matches!(&events[5], XmlEvent::Text(t) if t == "c"));
    }

    // G9 — declaration version/encoding/standalone.
    #[test]
    fn declaration_carries_version_encoding_standalone() {
        let events = read_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><e/>"#)
            .expect("parse");
        match &events[0] {
            XmlEvent::Declaration(decl) => {
                assert_eq!(decl.version(), "1.0");
                assert_eq!(decl.encoding(), Some("UTF-8"));
                assert_eq!(decl.standalone(), Some(true));
            }
            other => panic!("expected Declaration, got {other:?}"),
        }
    }

    // G10 — processing instructions and comments.
    #[test]
    fn pi_and_comment_events_preserve_content() {
        let events = read_all(b"<?pi data?><!-- c --><e/>").expect("parse");
        match &events[0] {
            XmlEvent::ProcessingInstruction(pi) => {
                assert_eq!(pi.target(), "pi");
                assert_eq!(pi.content(), " data");
            }
            other => panic!("expected PI, got {other:?}"),
        }
        assert!(matches!(&events[1], XmlEvent::Comment(t) if t == " c "));
        assert!(matches!(events[2], XmlEvent::Empty(_)));
    }

    // G11 — empty and whitespace-only input.
    #[test]
    fn empty_and_whitespace_inputs() {
        let events = read_all(b"").expect("empty parses");
        assert!(matches!(events.as_slice(), []));
        let events = read_all(b"   \n").expect("whitespace parses");
        assert!(
            events
                .iter()
                .all(|e| matches!(e, XmlEvent::Text(t) if t.trim().is_empty())),
            "whitespace-only input must yield at most whitespace text: {events:?}"
        );
    }

    // G12 — End names resolve inside the element's scope before pop.
    #[test]
    fn end_names_resolve_before_scope_pop() {
        let events = read_all(br#"<p:e xmlns:p="urn:p"></p:e>"#).expect("parse");
        match &events[1] {
            XmlEvent::End(name) => {
                assert_eq!(name.qualified_name(), "p:e");
                assert_eq!(name.namespace(), Some("urn:p"));
            }
            other => panic!("expected End, got {other:?}"),
        }
    }

    // G13 — normalized attribute values (references + whitespace rules).
    #[test]
    fn attribute_normalized_value_unescapes_and_folds() {
        let mut input = br#"<e a="x&#9;y" b="p  q"/>"#.as_slice();
        let mut reader = XmlEventReader::new(&mut input, &options());
        let mut buffer = XmlEventBuffer::new();
        let event = reader
            .read_event(&mut buffer)
            .expect("read")
            .expect("event");
        let XmlEventRef::Empty(element) = event else {
            panic!("expected Empty, got {event:?}");
        };
        let attrs = element.attributes();
        // &#9; resolves to a tab which attribute normalization keeps.
        assert_eq!(
            attrs[0].normalized_value().expect("normalized").as_ref(),
            "x\ty"
        );
        // Literal whitespace stays (CDATA-type normalization only maps
        // each whitespace char to a space, it does not collapse runs).
        assert_eq!(
            attrs[1].normalized_value().expect("normalized").as_ref(),
            "p  q"
        );
    }

    // G14 — reader position/depth observability.
    #[test]
    fn position_and_depth_track_the_stream() {
        let mut input = b"<a><b/></a>".as_slice();
        let mut reader = XmlEventReader::new(&mut input, &options());
        let mut buffer = XmlEventBuffer::new();
        assert_eq!(reader.position().byte, Some(0));
        assert_eq!(reader.depth(), 0);
        let event = reader
            .read_event(&mut buffer)
            .expect("read")
            .expect("event");
        assert!(matches!(event, XmlEventRef::Start(_)));
        assert_eq!(reader.depth(), 1);
        assert_eq!(reader.position().byte, Some(3));
        let event = reader
            .read_event(&mut buffer)
            .expect("read")
            .expect("event");
        assert!(matches!(event, XmlEventRef::Empty(_)));
        assert_eq!(reader.depth(), 2);
        let event = reader
            .read_event(&mut buffer)
            .expect("read")
            .expect("event");
        assert!(matches!(event, XmlEventRef::End(_)));
        assert_eq!(reader.depth(), 1, "end element still open until next read");
    }

    // N1 — input byte budget.
    #[test]
    fn byte_budget_is_limit_exceeded() {
        let mut options = options();
        options.limits = InputLimits::new(8);
        let error = read_err(b"<root>long document</root>", &options);
        assert_eq!(error.kind(), DocumentErrorKind::LimitExceeded);
        assert_eq!(error.limit().expect("limit").name, "max_bytes");
        assert_eq!(error.format(), DocumentFormat::Xml);
    }

    // N2 — depth limit.
    #[test]
    fn depth_limit_is_limit_exceeded() {
        let mut options = options();
        options.max_depth = Some(1);
        let error = read_err(b"<a><b/></a>", &options);
        assert_eq!(error.kind(), DocumentErrorKind::LimitExceeded);
        assert_eq!(error.limit().expect("limit").name, "max_depth");
        assert_eq!(error.limit().expect("limit").max, 1);
    }

    // N3 — attribute limit.
    #[test]
    fn attribute_limit_is_limit_exceeded() {
        let mut options = options();
        options.max_attributes = Some(1);
        let error = read_err(br#"<e a="1" b="2"/>"#, &options);
        assert_eq!(error.kind(), DocumentErrorKind::LimitExceeded);
        assert_eq!(error.limit().expect("limit").name, "max_attributes");
    }

    // N4 — event byte limit.
    #[test]
    fn event_byte_limit_is_limit_exceeded() {
        let mut options = options();
        options.max_event_bytes = Some(5);
        let error = read_err(b"<e>hello world</e>", &options);
        assert_eq!(error.kind(), DocumentErrorKind::LimitExceeded);
        assert_eq!(error.limit().expect("limit").name, "max_event_bytes");
    }

    // N5 — event count limit.
    #[test]
    fn event_count_limit_is_limit_exceeded() {
        let mut options = options();
        options.max_events = Some(1);
        let error = read_err(b"<a><b/></a>", &options);
        assert_eq!(error.kind(), DocumentErrorKind::LimitExceeded);
        assert_eq!(error.limit().expect("limit").name, "max_events");
    }

    // N6 — namespace binding limit.
    #[test]
    fn namespace_binding_limit_is_limit_exceeded() {
        let mut options = options();
        options.max_namespace_bindings = Some(1);
        let error = read_err(br#"<e xmlns:a="u:a" xmlns:b="u:b"/>"#, &options);
        assert_eq!(error.kind(), DocumentErrorKind::LimitExceeded);
        assert_eq!(error.limit().expect("limit").name, "max_namespace_bindings");
    }

    // N7/N8 — internal and external DTD rejected.
    #[test]
    fn doctype_is_rejected_as_unsupported() {
        for doc in [
            &b"<!DOCTYPE e [<!ENTITY x \"y\">]><e/>"[..],
            &b"<!DOCTYPE e SYSTEM \"x.dtd\"><e/>"[..],
        ] {
            let error = read_err(doc, &options());
            assert_eq!(error.kind(), DocumentErrorKind::Unsupported);
            assert_eq!(error.format(), DocumentFormat::Xml);
        }
    }

    // N9 — undefined general entity rejected.
    #[test]
    fn undefined_entity_is_unsupported() {
        let error = read_err(b"<e>&foo;</e>", &options());
        assert_eq!(error.kind(), DocumentErrorKind::Unsupported);
    }

    // N10 — invalid UTF-8.
    #[test]
    fn invalid_utf8_is_invalid_encoding() {
        let error = read_err(b"<e>\xff\xfe</e>", &options());
        assert_eq!(error.kind(), DocumentErrorKind::InvalidEncoding);
    }

    // N11 — mismatched end tag.
    #[test]
    fn mismatched_end_tag_is_syntax() {
        let error = read_err(b"<a></b>", &options());
        assert_eq!(error.kind(), DocumentErrorKind::Syntax);
    }

    // N12 — duplicate attributes.
    #[test]
    fn duplicate_attributes_are_syntax() {
        let error = read_err(br#"<e a="1" a="2"/>"#, &options());
        assert_eq!(error.kind(), DocumentErrorKind::Syntax);
        assert!(error.source().is_some(), "backend diagnostic preserved");
    }

    // N13 — invalid comment (`--` inside).
    #[test]
    fn double_hyphen_in_comment_is_syntax() {
        let error = read_err(b"<e><!-- a -- b --></e>", &options());
        assert_eq!(error.kind(), DocumentErrorKind::Syntax);
    }

    // N14 — unclosed reference.
    #[test]
    fn unclosed_reference_is_syntax() {
        let error = read_err(b"<e>&amp</e>", &options());
        assert_eq!(error.kind(), DocumentErrorKind::Syntax);
    }

    // N15 — undeclared prefix.
    #[test]
    fn undeclared_prefix_is_syntax() {
        let error = read_err(b"<p:e/>", &options());
        assert_eq!(error.kind(), DocumentErrorKind::Syntax);
    }

    // N16 — invalid `xml` prefix binding.
    #[test]
    fn invalid_xml_prefix_bind_is_syntax() {
        let error = read_err(br#"<e xmlns:xml="http://wrong"/>"#, &options());
        assert_eq!(error.kind(), DocumentErrorKind::Syntax);
    }

    // N17 — non-UTF-8 declared encoding.
    #[test]
    fn non_utf8_declared_encoding_is_unsupported() {
        let error = read_err(
            br#"<?xml version="1.0" encoding="Shift_JIS"?><e/>"#,
            &options(),
        );
        assert_eq!(error.kind(), DocumentErrorKind::Unsupported);
    }

    // N18 — malformed character reference.
    #[test]
    fn malformed_char_ref_is_syntax() {
        let error = read_err(b"<e>&#xZZ;</e>", &options());
        assert_eq!(error.kind(), DocumentErrorKind::Syntax);
    }

    // N19 — lower-level I/O failure.
    #[test]
    fn io_failure_is_io() {
        struct Fail;
        impl Read for Fail {
            fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::other("boom"))
            }
        }
        impl BufRead for Fail {
            fn fill_buf(&mut self) -> io::Result<&[u8]> {
                Err(io::Error::other("boom"))
            }
            fn consume(&mut self, _amount: usize) {}
        }
        let mut fail = Fail;
        let mut reader = XmlEventReader::new(&mut fail, &options());
        let mut buffer = XmlEventBuffer::new();
        let error = reader.read_event(&mut buffer).expect_err("io failure");
        assert_eq!(error.kind(), DocumentErrorKind::Io);
    }

    // S1 — streaming consumption: lookahead stays bounded.
    #[test]
    fn lookahead_is_bounded() {
        let mut doc = b"<root>".to_vec();
        doc.extend(std::iter::repeat_n(b"<i>x</i>".as_slice(), 20_000).flatten());
        doc.extend_from_slice(b"</root>");
        let mut cursor = io::Cursor::new(doc);
        let mut reader = XmlEventReader::new(&mut cursor, &options());
        let mut buffer = XmlEventBuffer::new();
        assert!(reader.read_event(&mut buffer).expect("read").is_some());
        assert!(
            cursor.position() <= 16 * 1024,
            "reader must not drain the input: position {}",
            cursor.position()
        );
    }

    // S2 — buffer capacity is reused across reads.
    #[test]
    fn buffer_capacity_is_reused() {
        let mut input = b"<a><big>xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx</big><b/></a>".as_slice();
        let mut reader = XmlEventReader::new(&mut input, &options());
        let mut buffer = XmlEventBuffer::new();
        // Read past the large text event, then reuse the buffer.
        for _ in 0..3 {
            assert!(reader.read_event(&mut buffer).expect("read").is_some());
        }
        let capacity = buffer.capacity();
        assert!(capacity >= 40, "buffer should hold the largest event");
        assert!(reader.read_event(&mut buffer).expect("read").is_some());
        assert!(
            buffer.capacity() >= capacity,
            "capacity is reused, never shrunk"
        );
    }

    // S3 — owned events survive buffer reuse.
    #[test]
    fn owned_event_survives_buffer_reuse() {
        let mut input = b"<a><b>first</b><c>second</c></a>".as_slice();
        let mut reader = XmlEventReader::new(&mut input, &options());
        let mut buffer = XmlEventBuffer::new();
        assert!(reader.read_event(&mut buffer).expect("read").is_some());
        let mut owned = None;
        while let Some(event) = reader.read_event(&mut buffer).expect("read") {
            if matches!(&event, XmlEventRef::Text(t) if t == "first") {
                owned = Some(event.to_owned());
            }
        }
        assert!(matches!(&owned, Some(XmlEvent::Text(t)) if t == "first"));
    }

    // S4 — reader methods stay callable while an event is borrowed.
    #[test]
    fn reader_usable_while_event_borrowed() {
        let mut input = b"<a><b/></a>".as_slice();
        let mut reader = XmlEventReader::new(&mut input, &options());
        let mut buffer = XmlEventBuffer::new();
        let event = reader
            .read_event(&mut buffer)
            .expect("read")
            .expect("event");
        assert!(matches!(event, XmlEventRef::Start(_)));
        // position/depth usable while `event` borrows the buffer.
        assert_eq!(reader.depth(), 1);
        assert!(reader.position().byte.is_some());
        assert_eq!(reader.events_emitted(), 1);
    }

    // S6 — deterministic seeded fuzz: arbitrary bytes never panic.
    #[test]
    fn seeded_fuzz_never_panics() {
        // xorshift64 — deterministic across runs/platforms.
        let mut state = 0x9E37_79B9_7F4A_7C15_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let fragments: &[&[u8]] = &[
            b"<a>",
            b"</a>",
            b"<b",
            b"/>",
            b"&amp;",
            b"&#65;",
            b"&x;",
            b"<!--",
            b"-->",
            b"<![CDATA[",
            b"]]>",
            b"<?p ",
            b"?>",
            b"<!DOCTYPE",
            b"\"",
            b"'",
            b"=",
            b"xmlns",
            b":",
            b" ",
            b"<",
            b">",
            b"&",
            b";",
            b"text",
            b"\x00",
            b"\xff",
            b"<?xml version=\"1.0\"?>",
        ];
        for _ in 0..200 {
            let len = usize::try_from(next() % 40).expect("len fits usize");
            let mut doc = Vec::new();
            for _ in 0..len {
                let index = usize::try_from(next() % fragments.len() as u64).expect("index fits");
                doc.extend_from_slice(fragments[index]);
            }
            let mut input = doc.as_slice();
            let mut reader = XmlEventReader::new(&mut input, &options());
            let mut buffer = XmlEventBuffer::new();
            while let Ok(Some(_)) = reader.read_event(&mut buffer) {}
        }
    }

    // S8 — repeated reads after EOF keep returning Ok(None).
    #[test]
    fn reads_after_eof_stay_none() {
        let mut input = b"<a/>".as_slice();
        let mut reader = XmlEventReader::new(&mut input, &options());
        let mut buffer = XmlEventBuffer::new();
        assert!(reader.read_event(&mut buffer).expect("read").is_some());
        for _ in 0..3 {
            assert!(reader.read_event(&mut buffer).expect("read").is_none());
        }
    }

    // S9 — buffer clear/reuse and poisoned readers keep failing.
    #[test]
    fn buffer_clear_and_error_poisoning() {
        let mut input = b"<a>text</a><b/>".as_slice();
        let mut reader = XmlEventReader::new(&mut input, &options());
        let mut buffer = XmlEventBuffer::new();
        assert!(reader.read_event(&mut buffer).expect("read").is_some());
        buffer.clear();
        assert!(reader.read_event(&mut buffer).expect("read").is_some());

        // After an error the reader stays poisoned.
        let mut input = b"<a></b>".as_slice();
        let mut reader = XmlEventReader::new(&mut input, &options());
        assert!(reader.read_event(&mut buffer).expect("read").is_some());
        let first = reader.read_event(&mut buffer).expect_err("syntax");
        assert_eq!(first.kind(), DocumentErrorKind::Syntax);
        let second = reader.read_event(&mut buffer).expect_err("still poisoned");
        assert_eq!(second.kind(), DocumentErrorKind::Syntax);
    }

    #[test]
    fn display_does_not_embed_input_text() {
        // ERR-02: the adapter's own `XmlErrorDetail` source text is
        // generic; `DocumentError`'s `Display` never renders input
        // content even when the payload carries a marker.
        let mut input = b"<root>MARKER7f3a9c</unclosed>".as_slice();
        let mut reader = XmlEventReader::new(&mut input, &options());
        let mut buffer = XmlEventBuffer::new();
        let err = loop {
            match reader.read_event(&mut buffer) {
                Ok(Some(_)) => {}
                Ok(None) => panic!("mismatched document must not reach EOF cleanly"),
                Err(e) => break e,
            }
        };
        let rendered = err.to_string();
        assert!(
            !rendered.contains("MARKER7f3a9c"),
            "Display must not echo input text: {rendered}"
        );
        assert!(rendered.contains("xml"), "{rendered}");
    }
}

#[cfg(all(feature = "xml", feature = "file"))]
mod xml_file_tests {
    use ruviform_io_structured::xml::{self, XmlEvent, XmlReadOptions};
    use ruviform_io_structured::{DocumentErrorKind, DocumentFileReadError};
    use std::fs;
    use std::path::PathBuf;

    fn workdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ruviform-io-structured-xml-file-{tag}-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("create workdir");
        dir
    }

    struct DirGuard(PathBuf);
    impl Drop for DirGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    // S5 — `xml + file` composition and error-axis separation.
    #[test]
    fn read_file_round_trip_and_error_axes() {
        let dir = workdir("read");
        let _guard = DirGuard(dir.clone());
        let dest = dir.join("doc.xml");
        fs::write(&dest, br#"<root xmlns="urn:x"><item a="1">t</item></root>"#)
            .expect("write fixture");

        let events = xml::read_file(&dest, &XmlReadOptions::default()).expect("read");
        assert!(matches!(events[0], XmlEvent::Start(_)));
        assert!(
            events
                .iter()
                .any(|e| matches!(e, XmlEvent::Text(t) if t == "t"))
        );

        // Missing file → Open axis.
        let missing = dir.join("missing.xml");
        let err = xml::read_file(&missing, &XmlReadOptions::default())
            .expect_err("missing file must fail");
        assert!(matches!(err, DocumentFileReadError::Open(_)));

        // Malformed content → Decode axis with normalized kind.
        let bad = dir.join("bad.xml");
        fs::write(&bad, b"<a></b>").expect("write bad fixture");
        let err =
            xml::read_file(&bad, &XmlReadOptions::default()).expect_err("malformed must fail");
        match err {
            DocumentFileReadError::Decode(e) => {
                assert_eq!(e.kind(), DocumentErrorKind::Syntax);
            }
            other => panic!("expected Decode, got {other:?}"),
        }
    }
}
