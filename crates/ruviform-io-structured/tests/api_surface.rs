//! Public API surface audit for `ruviform-io-structured`
//! (IO-02 / Issue #284; per-format audit rules: `issue-297` Rule 5,
//! `issue-298` S8, `issue-299` S7).
//!
//! The IO-02 boundary agreement allows exactly three groups of foreign
//! types in the public API: `serde` traits as bounds, `std::io`/`std`
//! traits and types, and Ruviform's own types. Every backend
//! (`serde_json`, `serde-saphyr`, `toml`, `csv`/`csv-core`,
//! `quick-xml`) must stay private — its types, errors, feature names
//! and versions never appear in signatures; diagnostics are reachable
//! only as `dyn Error` through [`std::error::Error::source`].
//!
//! These tests pin the audit mechanically at the type level: a
//! `pub use backend::Type` re-export or `pub type Alias = backend::T`
//! would surface the backend crate's path through
//! [`std::any::type_name`], which the assertions reject. Signature-level
//! audit remains covered by the per-format test-specs' visual/grep
//! rules; this file is the handwritten API audit test those rules
//! reference.
//!
//! **RT contract:** non-RT only — like the crate itself.

use ruviform_io_structured::{
    DocumentError, DocumentErrorKind, DocumentFormat, DocumentLimit, DocumentPosition, InputLimits,
};
use std::any::type_name;

/// Crate-name fragments that must never appear in a public type's
/// canonical path. `toml`/`csv` are excluded here on purpose: this
/// crate's own `toml`/`csv` modules legitimately contain those
/// substrings — the `ruviform_io_structured::` prefix assertion below
/// is what rejects backend aliases for them.
const BACKEND_FRAGMENTS: &[&str] = &[
    "serde_json",
    "serde_saphyr",
    "saphyr",
    "quick_xml",
    "quick-xml",
    "csv_core",
    "csv-core",
];

fn assert_ruviform_type_name(name: &str) {
    assert!(
        name.starts_with("ruviform_io_structured::"),
        "public type must live in the ruviform_io_structured namespace, got {name}"
    );
    for fragment in BACKEND_FRAGMENTS {
        assert!(
            !name.contains(fragment),
            "public type path leaks backend crate {fragment}: {name}"
        );
    }
}

/// Every public type reachable without a format feature must resolve
/// to a `ruviform_io_structured` type — never a backend alias.
#[test]
fn core_public_types_are_ruviform_types() {
    for name in [
        type_name::<DocumentError>(),
        type_name::<DocumentErrorKind>(),
        type_name::<DocumentFormat>(),
        type_name::<DocumentPosition>(),
        type_name::<DocumentLimit>(),
        type_name::<InputLimits>(),
    ] {
        assert_ruviform_type_name(name);
    }
}

/// `DocumentError` is `Send` (boxed `Send + Sync` diagnostic) and its
/// `Display` is the minimal normalized surface — no backend text.
#[test]
fn document_error_is_send_and_display_is_normalized() {
    fn assert_send<T: Send>() {}
    assert_send::<DocumentError>();

    let error = DocumentError::from_kind(DocumentErrorKind::Syntax, DocumentFormat::Json);
    let rendered = error.to_string();
    assert!(
        rendered.starts_with("ruviform-io-structured:"),
        "{rendered}"
    );
    assert!(rendered.contains("json"), "{rendered}");
    assert!(rendered.contains("syntax"), "{rendered}");
}

/// The file-convenience error enums are Ruviform types; their
/// `source()` chain returns the inner failure as `dyn Error`.
#[cfg(feature = "file")]
#[test]
fn file_error_types_are_ruviform_types() {
    for name in [
        type_name::<ruviform_io_structured::DocumentFileReadError>(),
        type_name::<ruviform_io_structured::DocumentFileWriteError>(),
    ] {
        assert_ruviform_type_name(name);
    }
}

#[cfg(feature = "json")]
mod json_surface {
    use super::{assert_ruviform_type_name, type_name};
    use ruviform_io_structured::json::{JsonReadOptions, JsonWriteOptions};

    #[test]
    fn json_public_types_are_ruviform_types() {
        for name in [
            type_name::<JsonReadOptions>(),
            type_name::<JsonWriteOptions>(),
        ] {
            assert_ruviform_type_name(name);
        }
    }

    /// A consumer needs only `serde` bounds plus `std::io` to use the
    /// typed document API — no backend type ever enters the call
    /// signature. This helper is exactly what a downstream crate can
    /// write against the public API.
    #[test]
    fn typed_io_is_usable_with_serde_bounds_alone() {
        use ruviform_io_structured::{DocumentError, json};
        use serde::{Serialize, de::DeserializeOwned};

        fn consumer_roundtrip<T>(value: &T) -> Result<T, DocumentError>
        where
            T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
        {
            let mut bytes = Vec::new();
            json::to_writer(&mut bytes, value, &JsonWriteOptions::default())?;
            let mut slice = bytes.as_slice();
            json::from_reader(&mut slice, &JsonReadOptions::default())
        }

        let value = vec![1u32, 2, 3];
        assert_eq!(consumer_roundtrip(&value).expect("round trip"), value);
    }
}

#[cfg(feature = "yaml")]
mod yaml_surface {
    use super::{assert_ruviform_type_name, type_name};
    use ruviform_io_structured::yaml::{YamlReadOptions, YamlWriteOptions};

    #[test]
    fn yaml_public_types_are_ruviform_types() {
        for name in [
            type_name::<YamlReadOptions>(),
            type_name::<YamlWriteOptions>(),
        ] {
            assert_ruviform_type_name(name);
        }
    }
}

#[cfg(feature = "toml")]
mod toml_surface {
    use super::{assert_ruviform_type_name, type_name};
    use ruviform_io_structured::toml::{TomlReadOptions, TomlWriteOptions};

    #[test]
    fn toml_public_types_are_ruviform_types() {
        for name in [
            type_name::<TomlReadOptions>(),
            type_name::<TomlWriteOptions>(),
        ] {
            assert_ruviform_type_name(name);
        }
    }
}

#[cfg(feature = "csv")]
mod csv_surface {
    use super::{assert_ruviform_type_name, type_name};
    use ruviform_io_structured::csv::{
        CsvBomPolicy, CsvDialect, CsvQuoteStyle, CsvReadOptions, CsvRecord, CsvTerminator, CsvTrim,
        CsvWriteOptions,
    };
    use std::io::Cursor;

    #[test]
    fn csv_public_types_are_ruviform_types() {
        for name in [
            type_name::<CsvBomPolicy>(),
            type_name::<CsvDialect>(),
            type_name::<CsvQuoteStyle>(),
            type_name::<CsvReadOptions>(),
            type_name::<CsvRecord>(),
            type_name::<CsvTerminator>(),
            type_name::<CsvTrim>(),
            type_name::<CsvWriteOptions>(),
            type_name::<ruviform_io_structured::csv::CsvReader<'static, Cursor<Vec<u8>>>>(),
            type_name::<ruviform_io_structured::csv::CsvWriter<'static, Cursor<Vec<u8>>>>(),
        ] {
            assert_ruviform_type_name(name);
        }
    }
}

#[cfg(feature = "xml")]
mod xml_surface {
    use super::{assert_ruviform_type_name, type_name};
    use ruviform_io_structured::xml::{
        XmlAttribute, XmlAttributeRef, XmlDeclaration, XmlDeclarationRef, XmlElement,
        XmlElementRef, XmlEvent, XmlEventBuffer, XmlEventReader, XmlEventRef, XmlName, XmlNameRef,
        XmlPi, XmlPiRef, XmlReadOptions, XmlReference, XmlReferenceValue,
    };
    use std::io::Cursor;

    #[test]
    fn xml_public_types_are_ruviform_types() {
        for name in [
            type_name::<XmlAttribute>(),
            type_name::<XmlAttributeRef<'static>>(),
            type_name::<XmlDeclaration>(),
            type_name::<XmlDeclarationRef<'static>>(),
            type_name::<XmlElement>(),
            type_name::<XmlElementRef<'static>>(),
            type_name::<XmlEvent>(),
            type_name::<XmlEventBuffer>(),
            type_name::<XmlEventReader<'static, Cursor<Vec<u8>>>>(),
            type_name::<XmlEventRef<'static>>(),
            type_name::<XmlName>(),
            type_name::<XmlNameRef<'static>>(),
            type_name::<XmlPi>(),
            type_name::<XmlPiRef<'static>>(),
            type_name::<XmlReadOptions>(),
            type_name::<XmlReference>(),
            type_name::<XmlReferenceValue>(),
        ] {
            assert_ruviform_type_name(name);
        }
    }
}
