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
    use super::fixtures::{Manifest, NameDoc};
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
}

#[cfg(feature = "yaml")]
mod yaml_tests {
    use super::fixtures::{Manifest, NameDoc};
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
}

#[cfg(feature = "toml")]
mod toml_tests {
    use super::fixtures::{Manifest, NameDoc};
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
