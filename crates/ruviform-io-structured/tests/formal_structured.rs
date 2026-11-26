//! Issue #806 slice 3 (D-layer formal verification) — structured
//! document parser property suite. Kani proofs for crate-private
//! decision gates live in-source beside the production implementation.
//!
//! Target contracts (product source of truth:
//! `crates/ruviform-io-structured/src/{json,yaml,toml,csv,xml}.rs`,
//! `src/input.rs` (`BoundedRead`), `src/limits.rs` (`InputLimits`),
//! `src/error.rs` (`DocumentError` / `DocumentErrorKind` /
//! `DocumentLimit`)):
//!
//! - Every format reader drives through `BoundedRead`: a document of
//!   exactly `max_bytes` may be accepted; the first byte past it trips
//!   `DocumentErrorKind::LimitExceeded` with `limit.name == "max_bytes"`.
//! - The readers never panic on arbitrary bytes and every `Err` keeps
//!   its `DocumentErrorKind` inside the documented per-format set:
//!   - json {`Io`, `Syntax`, `DataModel`, `TrailingData`, `LimitExceeded`}
//!   - yaml {`Io`, `InvalidEncoding`, `Syntax`, `DataModel`,
//!     `TrailingData`, `LimitExceeded`, `Unsupported`}
//!   - toml {`Io`, `InvalidEncoding`, `Syntax`, `DataModel`,
//!     `LimitExceeded`} (the TOML grammar folds trailing bytes into
//!     `Syntax` — the format has no `TrailingData`)
//!   - csv {`Io`, `InvalidEncoding`, `DataModel`, `LimitExceeded`,
//!     `Unsupported`} (permissive grammar — no `Syntax`/`TrailingData`)
//!   - xml {`Io`, `InvalidEncoding`, `Syntax`, `LimitExceeded`,
//!     `Unsupported`} (event model — no `DataModel`/`TrailingData`)
//! - `from_reader` strict-EOF: non-whitespace bytes after a JSON/YAML
//!   document are `TrailingData`; a second YAML document is
//!   `TrailingData`; a TOML continuation is `Syntax`.
//! - CSV `max_record_bytes` / `max_field_bytes` / `max_fields` /
//!   `max_records` gate *before* buffer growth and name the option in
//!   `DocumentError::limit`; YAML `max_depth` / `max_aliases` and XML
//!   `max_depth` / `max_attributes` / `max_event_bytes` / `max_events`
//!   report `LimitExceeded` with the exceeded option named.
//! - `CsvReader` yields records in input order with a monotone
//!   `position().record` index; `read_typed` consumes exactly one
//!   record per call and reports `DataModel`/`InvalidEncoding` — never
//!   a partial value; `CsvRecord::field_str` reports `InvalidEncoding`
//!   on non-UTF-8; the BOM policy strips or keeps the UTF-8 BOM and
//!   rejects UTF-16-family BOMs as `InvalidEncoding`; a dialect with
//!   `delimiter == quote` is `Unsupported`.
//! - `XmlEventReader` keeps secure defaults: DTD/doctype, non-UTF-8
//!   declared encodings and non-predefined general entities are
//!   `Unsupported`; after an error the reader is poisoned and keeps
//!   reporting a failure.
//!
//! `#[cfg]` gating mirrors `tests/structured.rs`: `default = []`
//! compiles an empty suite and `mise run test-formal` drives it with
//! `--all-features`.
//!
//! The `#[cfg(kani)]` module at the bottom carries the same contracts
//! as bounded proofs (run via `mise run test-formal-kani`); the bounds
//! and assumptions are documented in
//! `docs/specs/formal-verification.md`.

#[cfg(any(
    feature = "json",
    feature = "yaml",
    feature = "toml",
    feature = "csv",
    feature = "xml"
))]
mod common {
    use proptest::prelude::*;

    /// Bounded arbitrary byte noise for the raw side of each input
    /// strategy — big enough to span a small document plus structural
    /// bytes, small enough to keep backend parsing cheap.
    pub(crate) fn arb_bytes() -> impl Strategy<Value = Vec<u8>> {
        prop::collection::vec(any::<u8>(), 0..=2048)
    }

    /// Mutation op triples consumed by [`mutate`].
    pub(crate) fn arb_ops() -> impl Strategy<Value = Vec<(u8, u8, u8)>> {
        prop::collection::vec(any::<(u8, u8, u8)>(), 1..=4)
    }

    /// Byte-level mutations applied to a valid document — flips,
    /// truncation, insertion, and short overwrites exercise the deep
    /// parser paths that raw noise cannot reach.
    pub(crate) fn mutate(mut bytes: Vec<u8>, ops: &[(u8, u8, u8)]) -> Vec<u8> {
        for &(a, b, c) in ops {
            match a % 4 {
                0 => {
                    if !bytes.is_empty() {
                        let p = usize::from(b) % bytes.len();
                        bytes[p] ^= c.max(1);
                    }
                }
                1 => {
                    let keep = bytes.len() * usize::from(b) / 255;
                    bytes.truncate(keep);
                }
                2 => {
                    let p = usize::from(b) % (bytes.len() + 1);
                    bytes.insert(p, c);
                }
                _ => {
                    if !bytes.is_empty() {
                        let p = usize::from(b) % bytes.len();
                        let w = (usize::from(c) % 8 + 1).min(bytes.len() - p);
                        for byte in &mut bytes[p..p + w] {
                            *byte = c;
                        }
                    }
                }
            }
        }
        bytes
    }

    /// Document names that every backend can serialize — printable
    /// ASCII keeps the fixture-seeded side stable while arbitrary
    /// bytes cover the hostile-input side.
    #[cfg(any(feature = "json", feature = "yaml", feature = "toml"))]
    pub(crate) fn arb_name() -> impl Strategy<Value = String> {
        "[a-z0-9 ._-]{0,32}"
    }

    /// Minimal typed target for the single-document formats — the
    /// same shape as `tests/structured.rs`'s `NameDoc` (`name` must be
    /// a string). Fixtures are per-file; no reuse across test files.
    #[cfg(any(feature = "json", feature = "yaml", feature = "toml"))]
    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    pub(crate) struct Doc {
        pub(crate) name: String,
    }
}

#[cfg(feature = "json")]
mod json_props {
    use super::common::{Doc, arb_bytes, arb_name, arb_ops, mutate};
    use proptest::prelude::*;
    use ruviform_io_structured::{DocumentErrorKind, InputLimits, json};

    /// The documented `DocumentErrorKind` set for the JSON read path.
    /// `InvalidEncoding`/`Unsupported` have no JSON-read path; a future
    /// enum variant also fails here — extend the property when the
    /// documented set grows.
    fn kind_is_stable(kind: DocumentErrorKind) -> bool {
        matches!(
            kind,
            DocumentErrorKind::Io
                | DocumentErrorKind::Syntax
                | DocumentErrorKind::DataModel
                | DocumentErrorKind::TrailingData
                | DocumentErrorKind::LimitExceeded
        )
    }

    fn arb_input() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            4 => arb_bytes(),
            5 => (arb_name(), arb_ops()).prop_map(|(name, ops)| {
                let mut bytes = Vec::new();
                json::to_writer(&mut bytes, &Doc { name }, &json::JsonWriteOptions::default())
                    .expect("fixture serializes");
                mutate(bytes, &ops)
            }),
        ]
    }

    proptest! {
        /// Arbitrary input: `from_reader` never panics and every `Err`
        /// kind stays in the documented JSON set — checked both against
        /// a concrete typed target (`Doc`) and the accept-anything
        /// `IgnoredAny`.
        #[test]
        fn json_arbitrary_stream_error_set(bytes in arb_input()) {
            let mut input = bytes.as_slice();
            if let Err(e) = json::from_reader::<Doc>(&mut input, &json::JsonReadOptions::default()) {
                prop_assert!(
                    kind_is_stable(e.kind()),
                    "json Doc kind {:?} outside the JSON set",
                    e.kind()
                );
            }
            let mut input = bytes.as_slice();
            if let Err(e) = json::from_reader::<serde::de::IgnoredAny>(
                &mut input,
                &json::JsonReadOptions::default(),
            ) {
                prop_assert!(
                    kind_is_stable(e.kind()),
                    "json IgnoredAny kind {:?} outside the JSON set",
                    e.kind()
                );
            }
        }

        /// `InputLimits::max_bytes` is a hard budget: a document within
        /// the budget parses; any byte past the budget —
        /// *including trailing whitespace* — is `LimitExceeded` with
        /// `limit.name == "max_bytes"` (the `BoundedRead` probe sees
        /// the byte while draining the tail). The limit check is
        /// authoritative over `TrailingData`.
        #[test]
        fn json_max_bytes_gate(
            tail in prop::collection::vec(any::<u8>(), 0..=2),
            offset in -4i64..=4,
        ) {
            const DOC: &[u8] = br#"{"name":"ab"}"#; // 13 bytes
            let mut input = DOC.to_vec();
            input.extend_from_slice(&tail);
            let len = u64::try_from(input.len()).unwrap_or(u64::MAX);
            let limit = u64::try_from((i64::try_from(len).unwrap_or(0) + offset).max(0))
                .unwrap_or(u64::MAX);
            let options = json::JsonReadOptions::new().with_limits(InputLimits::new(limit));
            // Exact rule: the strict-EOF check reads the tail
            // byte-by-byte. A non-whitespace byte at position `p`
            // (1-based) is `TrailingData` when `p <= limit` (it was
            // read in-budget) and `LimitExceeded` when `p > limit`
            // (the budget probe trips first). An all-whitespace tail
            // must be fully consumed to confirm EOF — `LimitExceeded`
            // whenever a byte lies past the budget. Whitespace is the
            // *JSON* set {space, tab, LF, CR} — form feed is ASCII ws
            // but TrailingData here.
            let first_non_ws = input[DOC.len()..]
                .iter()
                .position(|b| !matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
                .map(|i| u64::try_from(DOC.len() + i + 1).unwrap_or(u64::MAX));
            let mut cur = input.as_slice();
            match json::from_reader::<Doc>(&mut cur, &options) {
                Ok(_) => {
                    prop_assert!(limit >= len, "accept requires the whole input in budget");
                    prop_assert!(first_non_ws.is_none(), "non-whitespace tail cannot be accepted");
                }
                Err(e) => match e.kind() {
                    DocumentErrorKind::LimitExceeded => {
                        prop_assert!(limit < len, "LimitExceeded requires an out-of-budget byte");
                        prop_assert!(
                            first_non_ws.is_none_or(|p| p > limit),
                            "an in-budget non-whitespace byte is TrailingData, not LimitExceeded"
                        );
                        prop_assert_eq!(e.limit().map(|l| l.name), Some("max_bytes"));
                        prop_assert_eq!(e.limit().map(|l| l.max), Some(limit));
                    }
                    DocumentErrorKind::TrailingData => {
                        let p = first_non_ws.expect("TrailingData needs a non-whitespace byte");
                        prop_assert!(p <= limit, "trailing byte {p} must be in-budget under {limit}");
                    }
                    other => prop_assert!(false, "unexpected kind {other:?}"),
                },
            }
        }

        /// Strict EOF: non-whitespace bytes after a complete document
        /// are `TrailingData` — including a whole second JSON value —
        /// while a whitespace-only tail is accepted.
        #[test]
        fn json_trailing_data_gate(whitespace_only in any::<bool>()) {
            let mut input = br#"{"name":"ab"}"#.to_vec();
            if whitespace_only {
                input.extend_from_slice(b" \n\t");
            } else {
                input.extend_from_slice(br#"{"name":"c"}"#);
            }
            let mut cur = input.as_slice();
            match json::from_reader::<Doc>(&mut cur, &json::JsonReadOptions::default()) {
                Ok(doc) => {
                    prop_assert!(whitespace_only);
                    prop_assert_eq!(doc.name.as_str(), "ab");
                }
                Err(e) => {
                    prop_assert!(!whitespace_only);
                    prop_assert_eq!(e.kind(), DocumentErrorKind::TrailingData);
                }
            }
        }

        /// `to_writer` → `from_reader` is the identity for valid
        /// documents, in both layouts.
        #[test]
        fn json_roundtrip(name in arb_name(), pretty in any::<bool>()) {
            let doc = Doc { name };
            let mut bytes = Vec::new();
            json::to_writer(
                &mut bytes,
                &doc,
                &json::JsonWriteOptions::new().with_pretty(pretty),
            )
            .expect("serialize");
            let mut cur = bytes.as_slice();
            let back: Doc =
                json::from_reader(&mut cur, &json::JsonReadOptions::default()).expect("parse");
            prop_assert_eq!(back, doc);
        }
    }
}

#[cfg(feature = "yaml")]
mod yaml_props {
    use super::common::{Doc, arb_bytes, arb_name, arb_ops, mutate};
    use proptest::prelude::*;
    use ruviform_io_structured::{DocumentErrorKind, InputLimits, yaml};

    /// The documented `DocumentErrorKind` set for the YAML read path —
    /// the widest set of the five formats (backend budgets surface as
    /// `LimitExceeded`, rejected YAML features as `Unsupported`).
    fn kind_is_stable(kind: DocumentErrorKind) -> bool {
        matches!(
            kind,
            DocumentErrorKind::Io
                | DocumentErrorKind::InvalidEncoding
                | DocumentErrorKind::Syntax
                | DocumentErrorKind::DataModel
                | DocumentErrorKind::TrailingData
                | DocumentErrorKind::LimitExceeded
                | DocumentErrorKind::Unsupported
        )
    }

    fn arb_input() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            4 => arb_bytes(),
            5 => (arb_name(), arb_ops()).prop_map(|(name, ops)| {
                let mut bytes = Vec::new();
                yaml::to_writer(&mut bytes, &Doc { name }, &yaml::YamlWriteOptions::new())
                    .expect("fixture serializes");
                mutate(bytes, &ops)
            }),
        ]
    }

    /// `d` nested flow sequences around a scalar.
    fn nested_doc(d: usize) -> Vec<u8> {
        let mut doc = vec![b'['; d];
        doc.push(b'1');
        doc.extend_from_slice(&vec![b']'; d]);
        doc.push(b'\n');
        doc
    }

    /// One anchor `&x` plus `k` alias uses.
    fn alias_doc(k: usize) -> Vec<u8> {
        let mut doc = b"a: &x 1\n".to_vec();
        for i in 0..k {
            doc.extend_from_slice(format!("b{i}: *x\n").as_bytes());
        }
        doc
    }

    proptest! {
        /// Arbitrary input: `from_reader` never panics and every `Err`
        /// kind stays in the documented YAML set — checked on both a
        /// typed target and `IgnoredAny`.
        #[test]
        fn yaml_arbitrary_stream_error_set(bytes in arb_input()) {
            let mut input = bytes.as_slice();
            if let Err(e) = yaml::from_reader::<Doc>(&mut input, &yaml::YamlReadOptions::default()) {
                prop_assert!(
                    kind_is_stable(e.kind()),
                    "yaml Doc kind {:?} outside the YAML set",
                    e.kind()
                );
            }
            let mut input = bytes.as_slice();
            if let Err(e) = yaml::from_reader::<serde::de::IgnoredAny>(
                &mut input,
                &yaml::YamlReadOptions::default(),
            ) {
                prop_assert!(
                    kind_is_stable(e.kind()),
                    "yaml IgnoredAny kind {:?} outside the YAML set",
                    e.kind()
                );
            }
        }

        /// `InputLimits::max_bytes` is a hard budget — a whitespace
        /// tail can never error in-budget, so the backend must pull
        /// past the budget to confirm the document is complete:
        /// `limit < len` is always `LimitExceeded` named `max_bytes`;
        /// `limit >= len` always parses.
        ///
        /// (Arbitrary tails are deliberately excluded: an in-budget
        /// tail byte can classify `Syntax`/`InvalidEncoding`/`Io`
        /// before the probe ever runs — the kind-set property covers
        /// that side.)
        #[test]
        fn yaml_max_bytes_gate(
            tail in prop::collection::vec(
                prop::sample::select(vec![b' ', b'\t', b'\n']),
                0..=2,
            ),
            offset in -4i64..=4,
        ) {
            const DOC: &[u8] = b"name: ok\n"; // 9 bytes
            let mut input = DOC.to_vec();
            input.extend_from_slice(&tail);
            let len = u64::try_from(input.len()).unwrap_or(u64::MAX);
            let limit = u64::try_from((i64::try_from(len).unwrap_or(0) + offset).max(0))
                .unwrap_or(u64::MAX);
            let options = yaml::YamlReadOptions::new().with_limits(InputLimits::new(limit));
            let mut cur = input.as_slice();
            match yaml::from_reader::<Doc>(&mut cur, &options) {
                Ok(_) => {
                    prop_assert!(limit >= len, "accept requires the whole input in budget");
                }
                Err(e) => {
                    prop_assert!(limit < len, "whitespace tail cannot error in-budget");
                    prop_assert_eq!(e.kind(), DocumentErrorKind::LimitExceeded);
                    prop_assert_eq!(e.limit().map(|l| l.name), Some("max_bytes"));
                    prop_assert_eq!(e.limit().map(|l| l.max), Some(limit));
                }
            }
        }

        /// `max_depth` is a real backend budget: `d` nested sequences
        /// exceed `m` iff `d > m` — reported `LimitExceeded` naming
        /// `max_depth`; at the boundary the document parses.
        #[test]
        fn yaml_depth_gate(d in 1usize..=9, m in 0usize..=10) {
            let doc = nested_doc(d);
            let options = yaml::YamlReadOptions::new().with_max_depth(m);
            let mut cur = doc.as_slice();
            match yaml::from_reader::<serde::de::IgnoredAny>(&mut cur, &options) {
                Ok(_) => prop_assert!(d <= m, "depth {d} accepted under max_depth {m}"),
                Err(e) => {
                    prop_assert!(d > m, "depth {d} within max_depth {m} must parse");
                    prop_assert_eq!(e.kind(), DocumentErrorKind::LimitExceeded);
                    prop_assert_eq!(e.limit().map(|l| l.name), Some("max_depth"));
                    prop_assert_eq!(e.limit().map(|l| l.max), Some(m as u64));
                }
            }
        }

        /// `max_aliases` counts alias events: `k` uses exceed `m` iff
        /// `k > m` — `LimitExceeded` naming `max_aliases`.
        #[test]
        fn yaml_alias_gate(k in 0usize..=5, m in 0usize..=5) {
            let doc = alias_doc(k);
            let options = yaml::YamlReadOptions::new().with_max_aliases(m);
            let mut cur = doc.as_slice();
            match yaml::from_reader::<serde::de::IgnoredAny>(&mut cur, &options) {
                Ok(_) => prop_assert!(k <= m, "{k} aliases accepted under max_aliases {m}"),
                Err(e) => {
                    prop_assert!(k > m, "{k} aliases within max_aliases {m} must parse");
                    prop_assert_eq!(e.kind(), DocumentErrorKind::LimitExceeded);
                    prop_assert_eq!(e.limit().map(|l| l.name), Some("max_aliases"));
                    prop_assert_eq!(e.limit().map(|l| l.max), Some(m as u64));
                }
            }
        }

        /// A second YAML document in the single-document API is
        /// `TrailingData`; a trailing comment is still one document.
        #[test]
        fn yaml_second_document_is_trailing_data(second in any::<bool>()) {
            let mut input = b"name: ok\n".to_vec();
            if second {
                input.extend_from_slice(b"---\nb: 1\n");
            } else {
                input.extend_from_slice(b"# tail\n");
            }
            let mut cur = input.as_slice();
            match yaml::from_reader::<Doc>(&mut cur, &yaml::YamlReadOptions::default()) {
                Ok(_) => prop_assert!(!second),
                Err(e) => {
                    prop_assert!(second);
                    prop_assert_eq!(e.kind(), DocumentErrorKind::TrailingData);
                }
            }
        }

        /// `to_writer` → `from_reader` is the identity for valid
        /// documents.
        #[test]
        fn yaml_roundtrip(name in arb_name()) {
            let doc = Doc { name };
            let mut bytes = Vec::new();
            yaml::to_writer(&mut bytes, &doc, &yaml::YamlWriteOptions::new())
                .expect("serialize");
            let mut cur = bytes.as_slice();
            let back: Doc =
                yaml::from_reader(&mut cur, &yaml::YamlReadOptions::default()).expect("parse");
            prop_assert_eq!(back, doc);
        }
    }
}

#[cfg(feature = "toml")]
mod toml_props {
    use super::common::{Doc, arb_bytes, arb_name, arb_ops, mutate};
    use proptest::prelude::*;
    use ruviform_io_structured::{DocumentErrorKind, InputLimits, toml};

    /// The documented `DocumentErrorKind` set for the TOML read path:
    /// no `TrailingData` (the grammar folds trailing bytes into
    /// `Syntax`), no `Unsupported`.
    fn kind_is_stable(kind: DocumentErrorKind) -> bool {
        matches!(
            kind,
            DocumentErrorKind::Io
                | DocumentErrorKind::InvalidEncoding
                | DocumentErrorKind::Syntax
                | DocumentErrorKind::DataModel
                | DocumentErrorKind::LimitExceeded
        )
    }

    fn arb_input() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            4 => arb_bytes(),
            5 => (arb_name(), arb_ops()).prop_map(|(name, ops)| {
                let mut bytes = Vec::new();
                toml::to_writer(&mut bytes, &Doc { name }, &toml::TomlWriteOptions::default())
                    .expect("fixture serializes");
                mutate(bytes, &ops)
            }),
        ]
    }

    proptest! {
        /// Arbitrary input: `from_reader` never panics and every `Err`
        /// kind stays in the documented TOML set.
        #[test]
        fn toml_arbitrary_stream_error_set(bytes in arb_input()) {
            let mut input = bytes.as_slice();
            if let Err(e) = toml::from_reader::<Doc>(&mut input, &toml::TomlReadOptions::default()) {
                prop_assert!(
                    kind_is_stable(e.kind()),
                    "toml kind {:?} outside the TOML set",
                    e.kind()
                );
            }
        }

        /// `InputLimits::max_bytes` is a hard budget: `read_to_string`
        /// must pull every byte, so `limit < len` is always
        /// `LimitExceeded` named `max_bytes` before UTF-8 decode or
        /// parse — regardless of tail content.
        #[test]
        fn toml_max_bytes_gate(
            tail in prop::collection::vec(any::<u8>(), 0..=2),
            offset in -4i64..=4,
        ) {
            const DOC: &[u8] = b"name = \"ok\"\n"; // 12 bytes
            let mut input = DOC.to_vec();
            input.extend_from_slice(&tail);
            let len = u64::try_from(input.len()).unwrap_or(u64::MAX);
            let limit = u64::try_from((i64::try_from(len).unwrap_or(0) + offset).max(0))
                .unwrap_or(u64::MAX);
            let options = toml::TomlReadOptions::new().with_limits(InputLimits::new(limit));
            let mut cur = input.as_slice();
            match toml::from_reader::<Doc>(&mut cur, &options) {
                Ok(_) => {
                    prop_assert!(limit >= len, "accept requires the whole input in budget");
                }
                Err(e) if limit < len => {
                    prop_assert_eq!(e.kind(), DocumentErrorKind::LimitExceeded);
                    prop_assert_eq!(e.limit().map(|l| l.name), Some("max_bytes"));
                    prop_assert_eq!(e.limit().map(|l| l.max), Some(limit));
                }
                Err(e) => {
                    // In-budget tail: non-UTF-8 is InvalidEncoding, a
                    // non-comment continuation is Syntax.
                    prop_assert!(
                        matches!(
                            e.kind(),
                            DocumentErrorKind::InvalidEncoding | DocumentErrorKind::Syntax
                        ),
                        "in-budget tail kind {:?} is not a decode/grammar failure",
                        e.kind()
                    );
                }
            }
        }

        /// The TOML grammar has no `TrailingData`: an ASCII
        /// continuation that cannot be a comment or a complete
        /// `key = value` is `Syntax`. `=`/`#`/whitespace are excluded
        /// and the tail is capped at two bytes, so it can never form a
        /// valid continuation.
        #[test]
        fn toml_trailing_is_syntax(
            tail in prop::collection::vec(
                any::<u8>().prop_filter(
                    "no =/#/whitespace — cannot form a continuation",
                    |b| b.is_ascii() && !b.is_ascii_whitespace() && *b != b'=' && *b != b'#',
                ),
                1..=2,
            ),
        ) {
            let mut input = b"name = \"ok\"\n".to_vec();
            input.extend_from_slice(&tail);
            let mut cur = input.as_slice();
            match toml::from_reader::<Doc>(&mut cur, &toml::TomlReadOptions::default()) {
                Ok(_) => prop_assert!(false, "non-comment ASCII tail must not parse"),
                Err(e) => prop_assert_eq!(e.kind(), DocumentErrorKind::Syntax),
            }
        }

        /// `to_writer` → `from_reader` is the identity for valid
        /// documents.
        #[test]
        fn toml_roundtrip(name in arb_name()) {
            let doc = Doc { name };
            let mut bytes = Vec::new();
            toml::to_writer(&mut bytes, &doc, &toml::TomlWriteOptions::default())
                .expect("serialize");
            let mut cur = bytes.as_slice();
            let back: Doc =
                toml::from_reader(&mut cur, &toml::TomlReadOptions::default()).expect("parse");
            prop_assert_eq!(back, doc);
        }
    }
}

#[cfg(feature = "csv")]
mod csv_props {
    use super::common::{arb_bytes, arb_ops, mutate};
    use proptest::prelude::*;
    use ruviform_io_structured::csv::{
        self, CsvBomPolicy, CsvDialect, CsvReadOptions, CsvReader, CsvRecord, CsvWriteOptions,
        CsvWriter,
    };
    use ruviform_io_structured::{DocumentError, DocumentErrorKind, InputLimits};

    /// The documented `DocumentErrorKind` set for the CSV read path:
    /// the permissive grammar has no `Syntax`/`TrailingData`; shape and
    /// encoding failures are `DataModel`/`InvalidEncoding`, dialect
    /// contradiction is `Unsupported`.
    fn kind_is_stable(kind: DocumentErrorKind) -> bool {
        matches!(
            kind,
            DocumentErrorKind::Io
                | DocumentErrorKind::InvalidEncoding
                | DocumentErrorKind::DataModel
                | DocumentErrorKind::LimitExceeded
                | DocumentErrorKind::Unsupported
        )
    }

    /// A CSV row bound by header names (same shape as
    /// `tests/structured.rs`'s `Row`; fixtures are per-file).
    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    struct Row {
        city: String,
        pop: u64,
    }

    /// Rectangular arbitrary rows: 1–4 fields of ≤12 arbitrary bytes.
    fn arb_rows() -> impl Strategy<Value = Vec<Vec<Vec<u8>>>> {
        (1usize..=4).prop_flat_map(|f| {
            prop::collection::vec(
                prop::collection::vec(prop::collection::vec(any::<u8>(), 0..=12), f..=f),
                0..=4,
            )
        })
    }

    /// Valid writer output then mutation, or raw noise.
    fn arb_input() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            4 => arb_bytes(),
            5 => (arb_rows(), arb_ops()).prop_map(|(rows, ops)| {
                let mut out = Vec::new();
                let options = CsvWriteOptions::new().with_flexible(true).with_has_headers(false);
                {
                    let mut writer = CsvWriter::new(&mut out, &options).expect("writer");
                    for row in &rows {
                        writer.write_record(row.iter()).expect("fixture row writes");
                    }
                    writer.finish().expect("finish");
                }
                mutate(out, &ops)
            }),
        ]
    }

    /// Drives `CsvReader` over `bytes`: `new` plus a `read_record`
    /// loop. Every observed error kind stays in the CSV set; record
    /// positions are the 0-based data-record index in order.
    fn drive(bytes: &[u8], has_headers: bool, flexible: bool) -> Result<(), TestCaseError> {
        let options = CsvReadOptions::default()
            .with_has_headers(has_headers)
            .with_flexible(flexible);
        let mut input = bytes;
        let mut reader = match CsvReader::new(&mut input, &options) {
            Ok(reader) => reader,
            Err(e) => {
                prop_assert!(
                    kind_is_stable(e.kind()),
                    "csv new kind {:?} outside the CSV set",
                    e.kind()
                );
                return Ok(());
            }
        };
        let mut record = CsvRecord::new();
        let mut expected_index = 0_u64;
        loop {
            match reader.read_record(&mut record) {
                Ok(true) => {
                    let pos = record.position().expect("position");
                    prop_assert_eq!(
                        pos.record,
                        Some(expected_index),
                        "record index must advance in order"
                    );
                    expected_index += 1;
                }
                Ok(false) => break,
                Err(e) => {
                    prop_assert!(
                        kind_is_stable(e.kind()),
                        "csv read_record kind {:?} outside the CSV set",
                        e.kind()
                    );
                    break;
                }
            }
        }
        prop_assert_eq!(
            reader.record_index(),
            expected_index,
            "record_index must count yielded data records"
        );
        Ok(())
    }

    /// Asserts the one `DocumentError` variant carrying a named limit.
    fn assert_limit(
        err: &DocumentError,
        name: &'static str,
        max: u64,
    ) -> Result<(), TestCaseError> {
        prop_assert_eq!(err.kind(), DocumentErrorKind::LimitExceeded);
        prop_assert_eq!(err.limit().map(|l| l.name), Some(name));
        prop_assert_eq!(err.limit().map(|l| l.max), Some(max));
        Ok(())
    }

    proptest! {
        /// Arbitrary input: `CsvReader::new` + a `read_record` loop
        /// never panics and every `Err` kind stays in the CSV set.
        /// Record positions stay 0-based and monotone; `record_index`
        /// counts exactly the yielded records.
        #[test]
        fn csv_arbitrary_stream_error_set(
            bytes in arb_input(),
            has_headers in any::<bool>(),
            flexible in any::<bool>(),
        ) {
            drive(&bytes, has_headers, flexible)?;
        }

        /// The shared byte budget: `CsvReader::new`'s BOM probe and the
        /// parse refill trip `LimitExceeded` named `max_bytes` as soon
        /// as any input byte lies past the budget. `flexible` keeps
        /// ragged tail records from surfacing `DataModel` in-budget —
        /// every tail byte is plain field data here.
        #[test]
        fn csv_max_bytes_gate(
            tail in prop::collection::vec(any::<u8>(), 0..=2),
            offset in -4i64..=4,
        ) {
            const DOC: &[u8] = b"a,b\nc,d\n"; // 8 bytes
            let mut input = DOC.to_vec();
            input.extend_from_slice(&tail);
            let len = u64::try_from(input.len()).unwrap_or(u64::MAX);
            let limit = u64::try_from((i64::try_from(len).unwrap_or(0) + offset).max(0))
                .unwrap_or(u64::MAX);
            let options = CsvReadOptions::default()
                .with_has_headers(false)
                .with_flexible(true)
                .with_limits(InputLimits::new(limit));
            let mut cur = input.as_slice();
            match CsvReader::new(&mut cur, &options) {
                Err(e) => {
                    prop_assert!(limit < len, "in-budget input must not fail construction");
                    assert_limit(&e, "max_bytes", limit)?;
                }
                Ok(mut reader) => {
                    let mut record = CsvRecord::new();
                    let mut err = None;
                    loop {
                        match reader.read_record(&mut record) {
                            Ok(true) => {}
                            Ok(false) => break,
                            Err(e) => {
                                err = Some(e);
                                break;
                            }
                        }
                    }
                    match err {
                        Some(e) => {
                            prop_assert!(limit < len, "in-budget input must not fail mid-read");
                            assert_limit(&e, "max_bytes", limit)?;
                        }
                        None => prop_assert!(
                            limit >= len,
                            "a fully in-budget input must read to end"
                        ),
                    }
                }
            }
        }

        /// `max_records` counts data records (headers excluded): the
        /// first `min(k, m)` records yield, then `LimitExceeded`
        /// naming `max_records`.
        #[test]
        fn csv_max_records_gate(k in 0usize..=5, m in 0u64..=6) {
            let mut input = Vec::new();
            for i in 0..k {
                input.extend_from_slice(format!("r{i}\n").as_bytes());
            }
            let options = CsvReadOptions::default()
                .with_has_headers(false)
                .with_max_records(m);
            let mut cur = input.as_slice();
            let mut reader = CsvReader::new(&mut cur, &options).expect("reader");
            let mut record = CsvRecord::new();
            let mut yielded = 0_u64;
            loop {
                match reader.read_record(&mut record) {
                    Ok(true) => yielded += 1,
                    Ok(false) => break,
                    Err(e) => {
                        prop_assert!(k as u64 > m, "{k} records within max_records {m}");
                        prop_assert_eq!(yielded, m, "gate fires after exactly m records");
                        assert_limit(&e, "max_records", m)?;
                        return Ok(());
                    }
                }
            }
            prop_assert!(k as u64 <= m, "{k} records must fit max_records {m}");
            prop_assert_eq!(yielded, k as u64);
            prop_assert_eq!(reader.record_index(), k as u64);
        }

        /// `max_fields` caps the per-record field count *before* the
        /// ends buffer grows: `f > m` is `LimitExceeded` naming
        /// `max_fields`; `f == m` parses.
        #[test]
        fn csv_max_fields_gate(f in 1usize..=10, m in 0usize..=10) {
            let mut input = b"a".to_vec();
            for _ in 1..f {
                input.extend_from_slice(b",a");
            }
            input.push(b'\n');
            let options = CsvReadOptions::default()
                .with_has_headers(false)
                .with_max_fields(m);
            let mut cur = input.as_slice();
            let mut reader = CsvReader::new(&mut cur, &options).expect("reader");
            let mut record = CsvRecord::new();
            match reader.read_record(&mut record) {
                Ok(true) => {
                    prop_assert!(f <= m, "{f} fields accepted under max_fields {m}");
                    prop_assert_eq!(record.len(), f);
                }
                Ok(false) => prop_assert!(false, "a record must surface or be gated"),
                Err(e) => {
                    prop_assert!(f > m, "{f} fields within max_fields {m} must parse");
                    assert_limit(&e, "max_fields", m as u64)?;
                }
            }
        }

        /// `max_field_bytes` audits field sizes *during* parse — a
        /// field of `s` bytes trips `LimitExceeded` naming
        /// `max_field_bytes` iff `s > m`.
        #[test]
        fn csv_max_field_bytes_gate(s in 1usize..=40, m in 0usize..=40) {
            let mut input = vec![b'x'; s];
            input.push(b'\n');
            let options = CsvReadOptions::default()
                .with_has_headers(false)
                .with_max_field_bytes(m);
            let mut cur = input.as_slice();
            let mut reader = CsvReader::new(&mut cur, &options).expect("reader");
            let mut record = CsvRecord::new();
            match reader.read_record(&mut record) {
                Ok(true) => {
                    prop_assert!(s <= m, "field of {s}B accepted under max_field_bytes {m}");
                    prop_assert_eq!(record.field_bytes(0).map(<[u8]>::len), Some(s));

                }
                Ok(false) => prop_assert!(false, "a record must surface or be gated"),
                Err(e) => {
                    prop_assert!(s > m, "field of {s}B within max_field_bytes {m}");
                    assert_limit(&e, "max_field_bytes", m as u64)?;
                }
            }
        }

        /// `max_record_bytes` caps the per-record output buffer: the
        /// gate is `data.len() >= max_record_bytes` on `OutputFull`, so
        /// a record of `r` field bytes *followed by a terminator* trips
        /// `LimitExceeded` naming `max_record_bytes` iff `r >= m` — the
        /// terminator still needs output headroom to complete the
        /// record.
        #[test]
        fn csv_max_record_bytes_gate(r in 1usize..=64, m in 0usize..=64) {
            let mut input = vec![b'x'; r];
            input.push(b'\n');
            let options = CsvReadOptions::default()
                .with_has_headers(false)
                .with_max_record_bytes(m);
            let mut cur = input.as_slice();
            let mut reader = CsvReader::new(&mut cur, &options).expect("reader");
            let mut record = CsvRecord::new();
            match reader.read_record(&mut record) {
                Ok(true) => {
                    prop_assert!(r < m, "record of {r}B accepted under max_record_bytes {m}");
                }
                Ok(false) => prop_assert!(false, "a record must surface or be gated"),
                Err(e) => {
                    prop_assert!(r >= m, "record of {r}B below max_record_bytes {m} must parse");
                    assert_limit(&e, "max_record_bytes", m as u64)?;
                }
            }
        }

        /// `CsvWriter` → `CsvReader` round-trips records byte-exact
        /// with per-record positions: record `i` starts at
        /// `position().record == Some(i)`, fields compare equal, and
        /// `record_index` ends at the row count.
        #[test]
        fn csv_streaming_roundtrip(rows in arb_rows()) {
            let mut bytes = Vec::new();
            let options = CsvWriteOptions::new().with_flexible(true).with_has_headers(false);
            {
                let mut writer = CsvWriter::new(&mut bytes, &options).expect("writer");
                for row in &rows {
                    writer.write_record(row.iter()).expect("row writes");
                }
                writer.finish().expect("finish");
            }
            let mut cur = bytes.as_slice();
            let read_options = CsvReadOptions::default()
                .with_has_headers(false)
                .with_flexible(true);
            let mut reader = CsvReader::new(&mut cur, &read_options).expect("reader");
            let mut record = CsvRecord::new();
            for (i, row) in rows.iter().enumerate() {
                prop_assert!(
                    reader.read_record(&mut record).expect("record"),
                    "row {i} must surface"
                );
                prop_assert_eq!(record.position().and_then(|p| p.record), Some(i as u64));
                prop_assert_eq!(record.len(), row.len());
                for (f, want) in row.iter().enumerate() {
                    prop_assert_eq!(record.field_bytes(f), Some(want.as_slice()));
                }
            }
            prop_assert!(!reader.read_record(&mut record).expect("eof"));
            prop_assert_eq!(reader.record_index(), rows.len() as u64);
        }

        /// `to_writer` → `from_reader` round-trips typed rows.
        #[test]
        fn csv_typed_roundtrip(
            rows in prop::collection::vec(
                ("[a-z0-9 ._-]{0,16}", any::<u64>()).prop_map(|(city, pop)| Row { city, pop }),
                0..=4,
            ),
        ) {
            let mut bytes = Vec::new();
            csv::to_writer(&mut bytes, &rows, &CsvWriteOptions::default()).expect("serialize");
            let mut cur = bytes.as_slice();
            let back: Vec<Row> =
                csv::from_reader(&mut cur, &CsvReadOptions::default()).expect("parse");
            prop_assert_eq!(back, rows);
        }

        /// `field_str` decodes UTF-8 per field: valid UTF-8 round-trips
        /// the `str`; invalid bytes are `InvalidEncoding` (never
        /// silently dropped). `field_bytes` always stays raw. Delimiter
        /// and terminator bytes are filtered out of the field so the
        /// single-field record keeps its shape (quoting is disabled).
        /// The document-level BOM contract runs before any field: a
        /// leading UTF-16 BOM is `InvalidEncoding` at construction and
        /// a leading UTF-8 BOM is stripped under the default policy.
        #[test]
        fn csv_field_str_encoding(
            field in prop::collection::vec(
                any::<u8>().prop_filter(
                    "delimiter/terminator would split the record",
                    |b| !b",\r\n".contains(b),
                ),
                0..=16,
            ),
        ) {
            let mut input = field.clone();
            input.push(b'\n');
            let options = CsvReadOptions::default()
                .with_has_headers(false)
                .with_dialect(CsvDialect::new().with_quoting(false));
            let mut cur = input.as_slice();
            if field.starts_with(&[0xFF, 0xFE]) || field.starts_with(&[0xFE, 0xFF]) {
                let err = CsvReader::new(&mut cur, &options)
                    .expect_err("a leading UTF-16 BOM must fail construction");
                prop_assert_eq!(err.kind(), DocumentErrorKind::InvalidEncoding);
                return Ok(());
            }
            // A leading UTF-8 BOM is stripped under `StripUtf8`, so the
            // first field reads back without those bytes.
            let field = field
                .strip_prefix(&[0xEF, 0xBB, 0xBF])
                .unwrap_or(&field);
            let mut reader = CsvReader::new(&mut cur, &options).expect("reader");
            let mut record = CsvRecord::new();
            match reader.read_record(&mut record) {
                Ok(true) => {
                    prop_assert_eq!(record.field_bytes(0), Some(field));
                    match (record.field_str(0), std::str::from_utf8(field)) {
                        (Ok(Some(s)), Ok(want)) => prop_assert_eq!(s, want),
                        (Err(e), Err(_)) => {
                            prop_assert_eq!(e.kind(), DocumentErrorKind::InvalidEncoding);
                        }
                        (Ok(Some(_)), Err(_)) => {
                            prop_assert!(false, "non-UTF-8 field must not decode");
                        }
                        (Err(e), Ok(_)) => prop_assert!(
                            false,
                            "valid UTF-8 field must not fail with {:?}",
                            e.kind()
                        ),
                        (Ok(None), _) => prop_assert!(false, "in-range field must exist"),
                    }
                }
                // A bare terminator is an empty line — it yields no
                // record (field is empty only then).
                Ok(false) => prop_assert!(field.is_empty(), "non-empty field must yield a record"),
                Err(e) => prop_assert!(false, "single-line parse must not fail: {:?}", e.kind()),
            }
        }

        /// BOM policy: UTF-16-family BOMs are `InvalidEncoding` at
        /// construction; a UTF-8 BOM is stripped under `StripUtf8` and
        /// kept as leading field bytes under `Preserve`; non-BOM
        /// prefixes are data.
        #[test]
        fn csv_bom_policy(prefix in 0u8..5, preserve in any::<bool>()) {
            let prefix_bytes: &[u8] = match prefix {
                0 => b"",
                1 => b"\xEF\xBB\xBF",
                2 => b"\xFF\xFE",
                3 => b"\xFE\xFF",
                _ => b"\xEF",
            };
            let mut input = prefix_bytes.to_vec();
            input.extend_from_slice(b"a,b\nc,d\n");
            let options = CsvReadOptions::default()
                .with_has_headers(false)
                .with_bom(if preserve {
                    CsvBomPolicy::Preserve
                } else {
                    CsvBomPolicy::StripUtf8
                });
            let mut cur = input.as_slice();
            match CsvReader::new(&mut cur, &options) {
                Err(e) => {
                    prop_assert!(matches!(prefix, 2 | 3), "only UTF-16 BOMs are rejected");
                    prop_assert_eq!(e.kind(), DocumentErrorKind::InvalidEncoding);
                }
                Ok(mut reader) => {
                    prop_assert!(!matches!(prefix, 2 | 3));
                    let mut record = CsvRecord::new();
                    prop_assert!(reader.read_record(&mut record).expect("record"));
                    let expected_first: &[u8] = match (prefix, preserve) {
                        (1, true) => b"\xEF\xBB\xBFa",
                        (4, _) => b"\xEFa",
                        _ => b"a",
                    };
                    prop_assert_eq!(record.field_bytes(0), Some(expected_first));
                }
            }
        }

        /// `delimiter == quote` is a contradictory dialect — reported
        /// `Unsupported` at construction on both reader and writer.
        #[test]
        fn csv_dialect_unsupported(byte in any::<u8>()) {
            let dialect = CsvDialect::new().with_delimiter(byte).with_quote(byte);
            let read_options = CsvReadOptions::default().with_dialect(dialect);
            let mut input = b"a,b\n".as_slice();
            match CsvReader::new(&mut input, &read_options) {
                Err(e) => prop_assert_eq!(e.kind(), DocumentErrorKind::Unsupported),
                Ok(_) => prop_assert!(false, "contradictory dialect must fail"),
            }
            let write_options = CsvWriteOptions::default().with_dialect(dialect);
            let mut out = Vec::new();
            match CsvWriter::new(&mut out, &write_options) {
                Err(e) => prop_assert_eq!(e.kind(), DocumentErrorKind::Unsupported),
                Ok(_) => prop_assert!(false, "contradictory dialect must fail"),
            }
        }

        /// Rectangularity is a data-model contract, not syntax:
        /// `flexible = false` rejects a field-count change as
        /// `DataModel`; `flexible = true` accepts it.
        #[test]
        fn csv_ragged_is_data_model_unless_flexible(flexible in any::<bool>()) {
            let mut input = b"a,b\nx,y,z\n".as_slice();
            let options = CsvReadOptions::default()
                .with_has_headers(false)
                .with_flexible(flexible);
            let mut reader = CsvReader::new(&mut input, &options).expect("reader");
            let mut record = CsvRecord::new();
            prop_assert!(reader.read_record(&mut record).expect("record 0"));
            match reader.read_record(&mut record) {
                Ok(true) => {
                    prop_assert!(flexible);
                    prop_assert_eq!(record.len(), 3);
                }
                Ok(false) => prop_assert!(false, "a record must surface or be gated"),
                Err(e) => {
                    prop_assert!(!flexible);
                    prop_assert_eq!(e.kind(), DocumentErrorKind::DataModel);
                }
            }
        }
    }

    /// `read_typed` consumes exactly one record per call: a
    /// record→`T` failure is `DataModel`/`InvalidEncoding`, the cursor
    /// stays record-aligned, and the next call reads the *following*
    /// record. Deterministic — a plain `#[test]`, not a property.
    #[test]
    fn csv_typed_failure_keeps_cursor_aligned() {
        let mut input = b"city,pop\na,1\nb,x\nc,3\n".as_slice();
        let mut reader = CsvReader::new(&mut input, &CsvReadOptions::default()).expect("reader");
        let first = reader.read_typed::<Row>().expect("row 0").expect("some");
        assert_eq!(
            first,
            Row {
                city: "a".into(),
                pop: 1
            }
        );
        let err = reader.read_typed::<Row>().expect_err("bad pop must fail");
        assert_eq!(err.kind(), DocumentErrorKind::DataModel);
        assert_eq!(
            reader.record_index(),
            2,
            "one record consumed by the failed read"
        );
        let third = reader.read_typed::<Row>().expect("row 2").expect("some");
        assert_eq!(
            third,
            Row {
                city: "c".into(),
                pop: 3
            }
        );
        assert!(reader.read_typed::<Row>().expect("eof").is_none());
    }
}

#[cfg(feature = "xml")]
mod xml_props {
    use super::common::{arb_bytes, arb_ops, mutate};
    use proptest::prelude::*;
    use ruviform_io_structured::xml::{
        XmlEventBuffer, XmlEventReader, XmlEventRef, XmlReadOptions, XmlReferenceValue,
    };
    use ruviform_io_structured::{DocumentErrorKind, InputLimits};

    /// The documented `DocumentErrorKind` set for the XML event path:
    /// the event model has no `DataModel`/`TrailingData`; secure
    /// defaults report `Unsupported` and the limit knobs report
    /// `LimitExceeded`.
    fn kind_is_stable(kind: DocumentErrorKind) -> bool {
        matches!(
            kind,
            DocumentErrorKind::Io
                | DocumentErrorKind::InvalidEncoding
                | DocumentErrorKind::Syntax
                | DocumentErrorKind::LimitExceeded
                | DocumentErrorKind::Unsupported
        )
    }

    const VALID_DOC: &[u8] = b"<root a=\"1\"><item>x</item><empty/></root>";

    fn arb_input() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            4 => arb_bytes(),
            5 => arb_ops().prop_map(|ops| mutate(VALID_DOC.to_vec(), &ops)),
        ]
    }

    /// Drives `XmlEventReader` over `bytes` to `Ok(None)` or the first
    /// `Err`; every error kind stays in the XML set, and once poisoned
    /// the reader keeps failing (still in-set). After `Ok(None)` the
    /// reader keeps reporting end-of-document.
    fn drive(bytes: &[u8], options: &XmlReadOptions) -> Result<(), TestCaseError> {
        let mut input = bytes;
        let mut reader = XmlEventReader::new(&mut input, options);
        let mut buffer = XmlEventBuffer::new();
        let mut events = 0_u32;
        loop {
            match reader.read_event(&mut buffer) {
                Ok(Some(_)) => events += 1,
                Ok(None) => {
                    // End-of-document is stable.
                    prop_assert!(reader.read_event(&mut buffer).expect("eof").is_none());
                    break;
                }
                Err(e) => {
                    prop_assert!(
                        kind_is_stable(e.kind()),
                        "xml read_event kind {:?} outside the XML set",
                        e.kind()
                    );
                    // Poisoned: a second failure is still in-set.
                    match reader.read_event(&mut buffer) {
                        Err(e2) => prop_assert!(
                            kind_is_stable(e2.kind()),
                            "poisoned read kind {:?} outside the XML set",
                            e2.kind()
                        ),
                        _ => prop_assert!(false, "poisoned reader must keep failing"),
                    }
                    break;
                }
            }
            if events > 4096 {
                break;
            }
        }
        Ok(())
    }

    /// Builds a document `<r>` + `k` children of `<e/>` or `<e>text</e>`
    /// + `</r>` from generated `(name, text_len, empty)` triples.
    fn items_doc(items: &[(u8, u8, bool)]) -> Vec<u8> {
        let mut doc = b"<r>".to_vec();
        for &(name, text_len, empty) in items {
            let tag = if name % 2 == 0 { b"a" } else { b"b" };
            if empty {
                doc.extend_from_slice(b"<");
                doc.extend_from_slice(tag);
                doc.extend_from_slice(b"/>");
            } else {
                doc.extend_from_slice(b"<");
                doc.extend_from_slice(tag);
                doc.extend_from_slice(b">");
                doc.extend_from_slice(&vec![b'x'; usize::from(text_len)]);
                doc.extend_from_slice(b"</");
                doc.extend_from_slice(tag);
                doc.extend_from_slice(b">");
            }
        }
        doc.extend_from_slice(b"</r>");
        doc
    }

    proptest! {
        /// Arbitrary input: `read_event` never panics, every `Err` kind
        /// stays in the XML set, errors poison the reader, and
        /// end-of-document is stable.
        #[test]
        fn xml_arbitrary_stream_error_set(bytes in arb_input()) {
            drive(&bytes, &XmlReadOptions::default())?;
        }

        /// The shared byte budget over the buffered pull parser: the
        /// parser must pull past the budget to confirm the document is
        /// complete, so `limit < len` trips `LimitExceeded` named
        /// `max_bytes`; `limit >= len` drains to `Ok(None)`. The tail is
        /// whitespace only — legal misc content after the root element —
        /// because an arbitrary in-budget tail byte can classify
        /// `Syntax`/`InvalidEncoding` before the probe runs (the
        /// arbitrary-drive property covers the kind set).
        #[test]
        fn xml_max_bytes_gate(
            tail in prop::collection::vec(
                prop::sample::select(vec![b' ', b'\t', b'\n', b'\r']),
                0..=2,
            ),
            offset in -4i64..=4,
        ) {
            const DOC: &[u8] = b"<r><a/></r>"; // 11 bytes
            let mut input = DOC.to_vec();
            input.extend_from_slice(&tail);
            let len = u64::try_from(input.len()).unwrap_or(u64::MAX);
            let limit = u64::try_from((i64::try_from(len).unwrap_or(0) + offset).max(0))
                .unwrap_or(u64::MAX);
            let mut options = XmlReadOptions::default();
            options.limits = InputLimits::new(limit);
            let mut cur = input.as_slice();
            let mut reader = XmlEventReader::new(&mut cur, &options);
            let mut buffer = XmlEventBuffer::new();
            for _ in 0..32 {
                match reader.read_event(&mut buffer) {
                    Ok(Some(_)) => {}
                    Ok(None) => {
                        prop_assert!(limit >= len, "in-budget read must reach end-of-document");
                        return Ok(());
                    }
                    Err(e) => {
                        prop_assert!(limit < len, "out-of-budget read must be the byte gate");
                        prop_assert_eq!(e.kind(), DocumentErrorKind::LimitExceeded);
                        prop_assert_eq!(e.limit().map(|l| l.name), Some("max_bytes"));
                        prop_assert_eq!(e.limit().map(|l| l.max), Some(limit));
                        return Ok(());
                    }
                }
            }
            prop_assert!(false, "event stream must terminate within the input");
        }

        /// `max_depth` counts open elements after each structural
        /// event: `d` nested elements inside `<r>` peak at depth
        /// `d + 1` — `LimitExceeded` naming `max_depth` iff `d + 1 > m`.
        #[test]
        fn xml_depth_gate(d in 1usize..=6, m in 0u64..=8) {
            let mut doc = b"<r>".to_vec();
            for _ in 0..d {
                doc.extend_from_slice(b"<e>");
            }
            for _ in 0..d {
                doc.extend_from_slice(b"</e>");
            }
            doc.extend_from_slice(b"</r>");
            let mut options = XmlReadOptions::default();
            options.max_depth = Some(m);
            let mut input = doc.as_slice();
            let mut reader = XmlEventReader::new(&mut input, &options);
            let mut buffer = XmlEventBuffer::new();
            for _ in 0..32 {
                match reader.read_event(&mut buffer) {
                    Ok(Some(_)) => {}
                    Ok(None) => {
                        prop_assert!(
                            (d as u64) < m,
                            "depth {} within max_depth {m} must parse",
                            d as u64 + 1
                        );
                        return Ok(());
                    }
                    Err(e) => {
                        prop_assert!(
                            (d as u64) >= m,
                            "depth {} must exceed max_depth {m} to fail",
                            d as u64 + 1
                        );
                        prop_assert_eq!(e.kind(), DocumentErrorKind::LimitExceeded);
                        prop_assert_eq!(e.limit().map(|l| l.name), Some("max_depth"));
                        prop_assert_eq!(e.limit().map(|l| l.max), Some(m));
                        return Ok(());
                    }
                }
            }
            prop_assert!(false, "event stream must terminate within the input");
        }

        /// `max_attributes` counts attributes per element: `a` attrs on
        /// the root trip `LimitExceeded` naming `max_attributes` iff
        /// `a > m`.
        #[test]
        fn xml_attribute_gate(a in 0usize..=8, m in 0u64..=9) {
            let mut doc = b"<r".to_vec();
            for i in 0..a {
                doc.extend_from_slice(format!(" a{i}=\"v\"").as_bytes());
            }
            doc.extend_from_slice(b"/>");
            let mut options = XmlReadOptions::default();
            options.max_attributes = Some(m);
            let mut input = doc.as_slice();
            let mut reader = XmlEventReader::new(&mut input, &options);
            let mut buffer = XmlEventBuffer::new();
            match reader.read_event(&mut buffer) {
                Ok(Some(XmlEventRef::Empty(element))) => {
                    prop_assert!(a as u64 <= m, "{a} attributes accepted under max_attributes {m}");
                    prop_assert_eq!(element.name().qualified_name(), "r");
                    prop_assert_eq!(element.attributes().len(), a);
                }
                Err(e) => {
                    prop_assert!(a as u64 > m, "{a} attributes within max_attributes {m}");
                    prop_assert_eq!(e.kind(), DocumentErrorKind::LimitExceeded);
                    prop_assert_eq!(e.limit().map(|l| l.name), Some("max_attributes"));
                    prop_assert_eq!(e.limit().map(|l| l.max), Some(m));
                }
                other => prop_assert!(false, "first event must be the root element or the gate: {other:?}"),
            }
        }

        /// `max_events` counts published events: `k` empty children
        /// under `<r>` emit `k + 2` events — `LimitExceeded` naming
        /// `max_events` at event `m + 1`.
        #[test]
        fn xml_event_count_gate(k in 0usize..=6, m in 0u64..=8) {
            let items: Vec<(u8, u8, bool)> = vec![(0, 0, true); k];
            let doc = items_doc(&items);
            let mut options = XmlReadOptions::default();
            options.max_events = Some(m);
            let mut input = doc.as_slice();
            let mut reader = XmlEventReader::new(&mut input, &options);
            let mut buffer = XmlEventBuffer::new();
            let mut events = 0_u64;
            for _ in 0..16 {
                match reader.read_event(&mut buffer) {
                    Ok(Some(_)) => events += 1,
                    Ok(None) => {
                        prop_assert!(k as u64 + 2 <= m, "{} events within max_events {m}", k as u64 + 2);
                        prop_assert_eq!(events, k as u64 + 2);
                        return Ok(());
                    }
                    Err(e) => {
                        prop_assert!(k as u64 + 2 > m, "{} events within max_events {m}", k as u64 + 2);
                        prop_assert_eq!(events, m, "gate fires at event m + 1");
                        prop_assert_eq!(e.kind(), DocumentErrorKind::LimitExceeded);
                        prop_assert_eq!(e.limit().map(|l| l.name), Some("max_events"));
                        prop_assert_eq!(e.limit().map(|l| l.max), Some(m));
                        return Ok(());
                    }
                }
            }
            prop_assert!(false, "event stream must terminate within the input");
        }

        /// `max_event_bytes` bounds each event's source span: a
        /// `b`-byte text node trips `LimitExceeded` naming
        /// `max_event_bytes` iff `max(b, 4) > m` (the `<r>`/`</r>`
        /// events span 3 and 4 bytes).
        #[test]
        fn xml_event_bytes_gate(b in 0usize..=64, m in 0u64..=72) {
            let mut doc = b"<r>".to_vec();
            doc.extend_from_slice(&vec![b'x'; b]);
            doc.extend_from_slice(b"</r>");
            let mut options = XmlReadOptions::default();
            options.max_event_bytes = Some(m);
            let mut input = doc.as_slice();
            let mut reader = XmlEventReader::new(&mut input, &options);
            let mut buffer = XmlEventBuffer::new();
            let widest = u64::try_from(b).unwrap_or(u64::MAX).max(4);
            for _ in 0..16 {
                match reader.read_event(&mut buffer) {
                    Ok(Some(_)) => {}
                    Ok(None) => {
                        prop_assert!(
                            widest <= m,
                            "widest span {widest} must fit max_event_bytes {m}"
                        );
                        return Ok(());
                    }
                    Err(e) => {
                        prop_assert!(widest > m, "widest event span {widest} within max_event_bytes {m}");
                        prop_assert_eq!(e.kind(), DocumentErrorKind::LimitExceeded);
                        prop_assert_eq!(e.limit().map(|l| l.name), Some("max_event_bytes"));
                        prop_assert_eq!(e.limit().map(|l| l.max), Some(m));
                        return Ok(());
                    }
                }
            }
            prop_assert!(false, "event stream must terminate within the input");
        }

        /// Secure defaults: DTD/doctype, non-UTF-8 declared encodings
        /// and non-predefined general entities are `Unsupported`;
        /// numeric character references publish a `Reference` event.
        #[test]
        fn xml_secure_defaults(case in 0u8..4) {
            let (doc, expect_unsupported): (&[u8], bool) = match case {
                0 => (b"<!DOCTYPE r><r/>", true),
                1 => (b"<?xml version=\"1.0\" encoding=\"utf-16\"?><r/>", true),
                2 => (b"<r>&undefined;</r>", true),
                _ => (b"<r>&#65;</r>", false),
            };
            let mut input = doc;
            let mut reader = XmlEventReader::new(&mut input, &XmlReadOptions::default());
            let mut buffer = XmlEventBuffer::new();
            let mut saw = None;
            while saw.is_none() {
                match reader.read_event(&mut buffer) {
                    Ok(Some(XmlEventRef::Reference(reference))) => {
                        saw = Some(Ok(reference));
                    }
                    Ok(Some(_)) => {}
                    Ok(None) => saw = Some(Err(None)),
                    Err(e) => saw = Some(Err(Some(e))),
                }
            }
            match saw {
                Some(Err(Some(e))) => {
                    prop_assert!(expect_unsupported);
                    prop_assert_eq!(e.kind(), DocumentErrorKind::Unsupported);
                }
                Some(Ok(reference)) => {
                    prop_assert!(!expect_unsupported);
                    prop_assert!(matches!(
                        reference.value(),
                        XmlReferenceValue::Char('A')
                    ));
                }
                _ => prop_assert!(false, "a Reference or an error must surface"),
            }
        }

        /// Generated well-formed documents produce a balanced event
        /// sequence: `Start`/`End` names nest correctly, `Empty` is
        /// self-closing, text payload matches, and depth returns to 0
        /// at end-of-document.
        #[test]
        fn xml_stream_alignment(
            items in prop::collection::vec((any::<u8>(), 0u8..=8, any::<bool>()), 0..=4),
        ) {
            let doc = items_doc(&items);
            let mut input = doc.as_slice();
            let mut reader = XmlEventReader::new(&mut input, &XmlReadOptions::default());
            let mut buffer = XmlEventBuffer::new();
            // Event 0: Start "r".
            match reader.read_event(&mut buffer).expect("root") {
                Some(XmlEventRef::Start(element)) => {
                    prop_assert_eq!(element.name().qualified_name(), "r");
                }
                _ => prop_assert!(false, "first event must be Start r"),
            }
            for &(name, text_len, empty) in &items {
                let tag = if name % 2 == 0 { "a" } else { "b" };
                if empty {
                    match reader.read_event(&mut buffer).expect("item") {
                        Some(XmlEventRef::Empty(element)) => {
                            prop_assert_eq!(element.name().qualified_name(), tag);
                        }
                        _ => prop_assert!(false, "expected Empty {tag}"),
                    }
                } else {
                    match reader.read_event(&mut buffer).expect("item") {
                        Some(XmlEventRef::Start(element)) => {
                            prop_assert_eq!(element.name().qualified_name(), tag);
                        }
                        _ => prop_assert!(false, "expected Start {tag}"),
                    }
                    if text_len > 0 {
                        let expected = "x".repeat(usize::from(text_len));
                        match reader.read_event(&mut buffer).expect("text") {
                            Some(XmlEventRef::Text(text)) => {
                                prop_assert_eq!(text.as_ref(), expected.as_str());
                            }
                            _ => prop_assert!(false, "expected Text"),
                        }
                    }
                    match reader.read_event(&mut buffer).expect("end") {
                        Some(XmlEventRef::End(name)) => {
                            prop_assert_eq!(name.qualified_name(), tag);
                        }
                        _ => prop_assert!(false, "expected End {tag}"),
                    }
                }
            }
            match reader.read_event(&mut buffer).expect("root end") {
                Some(XmlEventRef::End(name)) => {
                    prop_assert_eq!(name.qualified_name(), "r");
                }
                _ => prop_assert!(false, "expected End r"),
            }
            // `depth` still counts the element being ended; the next
            // read pops its scope and reports end-of-document.
            prop_assert!(reader.read_event(&mut buffer).expect("eof").is_none());
            prop_assert_eq!(reader.depth(), 0);
        }
    }
}
