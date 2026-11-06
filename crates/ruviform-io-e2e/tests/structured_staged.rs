//! Structured serialization → `StagedFile` → publish boundary (R2).
//!
//! The layer boundary under test: a structured encoder writes into a
//! live `StagedFile`, the staged bytes are sealed, independently
//! decoded from the pre-publish readback, and only then published.
//! Failure cases assert that a serialization failure or an
//! incomplete document never reaches the destination.

mod common;

use std::io::Write;

use common::{TestDir, exists, read};
use ruviform_io_file::{Durability, PublishMode, PublishOptions, StagedFile};
use ruviform_io_structured::{DocumentError, DocumentFileWriteError, csv, json};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct Row {
    name: String,
    count: u64,
}

/// A serialize impl that fails mid-document: the encoder must
/// propagate the error and the incomplete bytes must not publish.
struct Bad;
impl serde::Serialize for Bad {
    fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom("deliberate serialize failure"))
    }
}

fn options() -> PublishOptions {
    PublishOptions::new(PublishMode::CreateNew, Durability::FileSync)
}

#[test]
fn json_to_staged_seal_independent_readback_publish() {
    let dir = TestDir::new("json-round");
    let dest = dir.dest("rows.json");
    let rows = vec![
        Row {
            name: "kick".into(),
            count: 4,
        },
        Row {
            name: "snare".into(),
            count: 8,
        },
    ];

    // Serialize directly into the staged file — the structured layer
    // treats StagedFile as a plain Write sink.
    let mut staged = StagedFile::create(&dest).expect("stage");
    json::to_writer(&mut staged, &rows, &json::JsonWriteOptions::default()).expect("encode");

    // Seal, then independently decode the sealed bytes pre-publish.
    let mut sealed = staged.seal();
    let mut readback = sealed.open_readback().expect("readback");
    let decoded: Vec<Row> = json::from_reader(&mut readback, &json::JsonReadOptions::default())
        .expect("independent decode of sealed bytes");
    assert_eq!(decoded, rows, "readback decode must match the source value");

    sealed.publish(options()).expect("publish");
    // Post-publish: decode the destination bytes independently.
    let bytes = read(&dest).expect("published bytes");
    let reparsed: Vec<Row> =
        json::from_reader(&mut &bytes[..], &json::JsonReadOptions::default()).expect("reparse");
    assert_eq!(reparsed, rows);
}

#[test]
fn json_write_file_composes_seal_and_publish() {
    let dir = TestDir::new("json-writefile");
    let dest = dir.dest("doc.json");
    let row = Row {
        name: "hat".into(),
        count: 16,
    };

    json::write_file(&dest, &row, &json::JsonWriteOptions::default(), options())
        .expect("write_file");
    assert!(exists(&dest));
    let bytes = read(&dest).expect("published bytes");
    let decoded: Row =
        json::from_reader(&mut &bytes[..], &json::JsonReadOptions::default()).expect("decode");
    assert_eq!(decoded, row);
}

#[test]
fn csv_stream_to_staged_then_publish() {
    let dir = TestDir::new("csv-round");
    let dest = dir.dest("rows.csv");
    let rows: Vec<Row> = (0..32)
        .map(|i| Row {
            name: format!("track-{i}"),
            count: i * 7,
        })
        .collect();

    let mut staged = StagedFile::create(&dest).expect("stage");
    {
        let mut writer =
            csv::CsvWriter::new(&mut staged, &csv::CsvWriteOptions::default()).expect("csv");
        for row in &rows {
            writer.write_typed(row).expect("csv row");
        }
        writer.finish().expect("csv finish");
    }

    let mut sealed = staged.seal();
    let mut readback = sealed.open_readback().expect("readback");
    let mut reader =
        csv::CsvReader::new(&mut readback, &csv::CsvReadOptions::default()).expect("csv reader");
    let mut decoded = Vec::new();
    while let Some(row) = reader.read_typed::<Row>().expect("decode staged csv") {
        decoded.push(row);
    }
    assert_eq!(decoded, rows);

    sealed.publish(options()).expect("publish");
}

#[test]
fn serialization_failure_leaves_no_published_destination() {
    let dir = TestDir::new("fail-encode");
    let dest = dir.dest("broken.json");

    let mut staged = StagedFile::create(&dest).expect("stage");
    let err = json::to_writer(&mut staged, &Bad, &json::JsonWriteOptions::default());
    assert!(err.is_err(), "serialization failure must surface");
    // Drop the staged file without sealing — abort semantics.
    drop(staged);

    assert!(!exists(&dest), "failed encode must not publish");
}

#[test]
fn truncated_json_in_staged_file_is_not_decodable() {
    let dir = TestDir::new("truncated");
    let dest = dir.dest("trunc.json");

    // Stage intentionally incomplete JSON — a format that never
    // reached its required completion proof.
    let mut staged = StagedFile::create(&dest).expect("stage");
    staged.write_all(b"{\"name\":\"hal").expect("write partial");
    let mut sealed = staged.seal();

    let mut readback = sealed.open_readback().expect("readback");
    let result: Result<Row, DocumentError> =
        json::from_reader(&mut readback, &json::JsonReadOptions::default());
    assert!(result.is_err(), "truncated JSON must fail strict decode");

    // The caller's gate: do not publish bytes that fail verification.
    sealed.abort().expect("abort unverified");
    assert!(!exists(&dest));
}

#[test]
fn write_file_over_existing_without_replace_reports_publish_failure() {
    let dir = TestDir::new("writefile-conflict");
    let dest = dir.dest("doc.json");
    std::fs::write(&dest, b"{}").expect("seed");

    let row = Row {
        name: "x".into(),
        count: 1,
    };
    let result = json::write_file(&dest, &row, &json::JsonWriteOptions::default(), options());
    // write_file honors the caller-passed PublishOptions (CreateNew
    // here); the failure is surfaced as Publish, not swallowed.
    match result {
        Err(DocumentFileWriteError::Publish(failure)) => {
            assert_eq!(
                failure.kind(),
                ruviform_io_file::PublishFailureKind::DestinationExists
            );
        }
        other => panic!("expected Publish failure, got {other:?}"),
    }
    assert_eq!(read(&dest).as_deref(), Some(b"{}".as_slice()));
}
