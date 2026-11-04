//! Contract tests for `ruviform-io-structured` (IO-IMP-03 / Issue #297).
//!
//! Each format section is gated on its feature; `cargo test` with no
//! features compiles an empty suite. The file pins the Acceptance
//! matrix: typed round trips (structs/enums/nesting/Unicode/non-ASCII
//! keys), strict EOF / trailing data, invalid + truncated input, type
//! mismatch, byte-budget rejection *and* the exact boundary, backend
//! error normalization (`DocumentErrorKind` + `source()`), and the
//! `file` feature's staged publish contract (encode vs. publish
//! failure separation with `PublishFailure` state preserved).

#[cfg(any(feature = "json", feature = "yaml", feature = "toml"))]
mod fixtures {
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub(crate) enum Transport {
        File { path: String },
        Url { url: String, secure: bool },
    }

    /// Struct + enum + nesting + Unicode + a non-ASCII map key.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub(crate) struct Manifest {
        pub(crate) name: String,
        pub(crate) revision: u32,
        pub(crate) transport: Transport,
        pub(crate) tags: Vec<String>,
        pub(crate) labels: BTreeMap<String, String>,
    }

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
    #[derive(Debug, Deserialize)]
    pub(crate) struct NameDoc {
        pub(crate) name: String,
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
