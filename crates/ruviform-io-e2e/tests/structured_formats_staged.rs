//! Structured formats beyond json/csv — `yaml`/`toml` `write_file` →
//! staged publish → `read_file`, `xml` `read_file` on published bytes,
//! and the reopen boundary's tamper/torn-document detection.
//!
//! Slice-2 rules F1/F2 (`docs/test-specs/issue-306-io-e2e-fault-fuzz.md`):
//! every format round-trips through the same `StagedFile` publish
//! boundary, and bytes mutated *after* publish are refused by the
//! reader with a typed `DocumentErrorKind` — never silently accepted.

mod common;

use std::io::Write;

use common::{TestDir, exists};
use ruviform_io_file::{Durability, PublishMode, PublishOptions, StagedFile};
use ruviform_io_structured::{DocumentErrorKind, DocumentFileReadError, json, toml, xml, yaml};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct Config {
    name: String,
    count: u64,
    enabled: bool,
}

fn sample() -> Config {
    Config {
        name: "mix-bus".to_owned(),
        count: 42,
        enabled: true,
    }
}

fn publish_options() -> PublishOptions {
    PublishOptions::new(PublishMode::CreateNew, Durability::FileSync)
}

/// Publishes `doc` bytes through the staged boundary and returns the
/// destination — shared by the xml positive/negative fixtures.
fn publish_bytes(dir: &TestDir, name: &str, doc: &[u8]) -> std::path::PathBuf {
    let dest = dir.dest(name);
    let mut staged = StagedFile::create(&dest).expect("stage");
    staged.write_all(doc).expect("write document bytes");
    staged.seal().publish(publish_options()).expect("publish");
    assert!(exists(&dest));
    dest
}

#[test]
fn yaml_write_file_round_trip_through_staged_publish() {
    let dir = TestDir::new("yaml-round");
    let dest = dir.dest("config.yaml");
    let value = sample();

    yaml::write_file(
        &dest,
        &value,
        &yaml::YamlWriteOptions::default(),
        publish_options(),
    )
    .expect("yaml write_file");

    let decoded: Config =
        yaml::read_file(&dest, &yaml::YamlReadOptions::default()).expect("yaml read_file");
    assert_eq!(decoded, value, "yaml round-trip must preserve the value");
}

#[test]
fn toml_write_file_round_trip_through_staged_publish() {
    let dir = TestDir::new("toml-round");
    let dest = dir.dest("config.toml");
    let value = sample();

    toml::write_file(
        &dest,
        &value,
        &toml::TomlWriteOptions::default(),
        publish_options(),
    )
    .expect("toml write_file");

    let decoded: Config =
        toml::read_file(&dest, &toml::TomlReadOptions::default()).expect("toml read_file");
    assert_eq!(decoded, value, "toml round-trip must preserve the value");
}

#[test]
fn xml_read_file_parses_events_from_published_bytes() {
    let dir = TestDir::new("xml-events");
    let dest = publish_bytes(
        &dir,
        "doc.xml",
        br#"<?xml version="1.0"?><root a="1"><item>hi</item></root>"#,
    );

    let events = xml::read_file(&dest, &xml::XmlReadOptions::default()).expect("xml read_file");

    assert!(
        events
            .iter()
            .any(|e| matches!(e, xml::XmlEvent::Declaration(_))),
        "xml declaration must surface as an event"
    );
    let starts: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            xml::XmlEvent::Start(element) => Some(element.name().local_name()),
            _ => None,
        })
        .collect();
    assert_eq!(starts, ["root", "item"], "element open order");
    assert!(
        events
            .iter()
            .any(|e| matches!(e, xml::XmlEvent::Text(text) if text == "hi")),
        "character data must surface as a Text event"
    );
    let ends: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            xml::XmlEvent::End(name) => Some(name.local_name()),
            _ => None,
        })
        .collect();
    assert_eq!(ends, ["item", "root"], "element close order");
}

#[test]
fn published_json_trailing_bytes_are_rejected_on_reopen() {
    let dir = TestDir::new("json-tamper");
    let dest = dir.dest("doc.json");
    let value = sample();
    json::write_file(
        &dest,
        &value,
        &json::JsonWriteOptions::default(),
        publish_options(),
    )
    .expect("json write_file");

    // External post-publish mutation: append non-JSON bytes to the
    // published destination — the reopen boundary must refuse.
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&dest)
        .expect("open for append");
    file.write_all(b" trailing").expect("tamper");
    drop(file);

    match json::read_file::<Config>(&dest, &json::JsonReadOptions::default()) {
        Err(DocumentFileReadError::Decode(error)) => {
            assert_eq!(
                error.kind(),
                DocumentErrorKind::TrailingData,
                "appended bytes must classify as trailing data"
            );
        }
        other => panic!("expected Decode(TrailingData), got {other:?}"),
    }
}

#[test]
fn torn_json_document_is_rejected_on_reopen() {
    let dir = TestDir::new("json-torn");
    let dest = dir.dest("doc.json");
    let value = sample();
    json::write_file(
        &dest,
        &value,
        &json::JsonWriteOptions::default(),
        publish_options(),
    )
    .expect("json write_file");

    // Simulate a torn publish: the destination keeps only a strict
    // prefix of the verified bytes.
    let bytes = common::read(&dest).expect("published bytes");
    std::fs::write(&dest, &bytes[..bytes.len() / 2]).expect("torn write");

    match json::read_file::<Config>(&dest, &json::JsonReadOptions::default()) {
        Err(DocumentFileReadError::Decode(error)) => {
            assert_eq!(
                error.kind(),
                DocumentErrorKind::Syntax,
                "premature EOF must classify as syntax"
            );
        }
        other => panic!("expected Decode(Syntax), got {other:?}"),
    }
}

#[test]
fn second_yaml_document_is_rejected_on_reopen() {
    let dir = TestDir::new("yaml-second");
    let dest = dir.dest("config.yaml");
    yaml::write_file(
        &dest,
        &sample(),
        &yaml::YamlWriteOptions::default(),
        publish_options(),
    )
    .expect("yaml write_file");

    // A second YAML document appended post-publish is TrailingData —
    // the single-document API never silently merges streams.
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&dest)
        .expect("open for append");
    file.write_all(b"---\nother: 1\n")
        .expect("append second doc");
    drop(file);

    match yaml::read_file::<Config>(&dest, &yaml::YamlReadOptions::default()) {
        Err(DocumentFileReadError::Decode(error)) => {
            assert_eq!(error.kind(), DocumentErrorKind::TrailingData);
        }
        other => panic!("expected Decode(TrailingData), got {other:?}"),
    }
}

#[test]
fn torn_toml_document_is_rejected_on_reopen() {
    let dir = TestDir::new("toml-torn");
    let dest = dir.dest("config.toml");
    toml::write_file(
        &dest,
        &sample(),
        &toml::TomlWriteOptions::default(),
        publish_options(),
    )
    .expect("toml write_file");

    let bytes = common::read(&dest).expect("published bytes");
    std::fs::write(&dest, &bytes[..bytes.len() / 2]).expect("torn write");

    match toml::read_file::<Config>(&dest, &toml::TomlReadOptions::default()) {
        Err(DocumentFileReadError::Decode(error)) => {
            assert_eq!(error.kind(), DocumentErrorKind::Syntax);
        }
        other => panic!("expected Decode(Syntax), got {other:?}"),
    }
}

#[test]
fn invalid_utf8_toml_is_rejected_on_reopen() {
    let dir = TestDir::new("toml-utf8");
    let dest = dir.dest("config.toml");
    toml::write_file(
        &dest,
        &sample(),
        &toml::TomlWriteOptions::default(),
        publish_options(),
    )
    .expect("toml write_file");

    std::fs::write(&dest, [0xFF, 0xFE, 0xFD, 0x00, 0x41]).expect("invalid utf-8 write");

    match toml::read_file::<Config>(&dest, &toml::TomlReadOptions::default()) {
        Err(DocumentFileReadError::Decode(error)) => {
            assert_eq!(error.kind(), DocumentErrorKind::InvalidEncoding);
        }
        other => panic!("expected Decode(InvalidEncoding), got {other:?}"),
    }
}
