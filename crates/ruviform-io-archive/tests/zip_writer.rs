//! ZIP writer verification: state separation between
//! `ZipEntryWriter::finish` and `ZipArchiveWriter::finish`, poisoned
//! state on dropped entries, limits, and writer→reader round trips.

use std::io::{Cursor, Read, Write};

use ruviform_io_archive::zip::{
    ZipArchiveReader, ZipArchiveWriter, ZipCompressionMethod, ZipEntryOptions,
};
use ruviform_io_archive::{ArchiveError, ArchiveErrorKind, ArchiveReadLimits, EntryKind};

fn kind_of(result: Result<impl Sized, ArchiveError>) -> ArchiveErrorKind {
    match result {
        Ok(_) => panic!("expected error"),
        Err(error) => error.kind(),
    }
}

fn options(method: ZipCompressionMethod) -> ZipEntryOptions {
    ZipEntryOptions::new(method)
}

#[test]
fn stored_and_deflate_entries_round_trip() {
    let mut writer = ZipArchiveWriter::new(Cursor::new(Vec::new()));
    {
        let mut entry = writer
            .start_entry("plain.bin", &options(ZipCompressionMethod::Stored))
            .expect("start stored");
        entry.write_all(b"plain bytes").expect("write");
        entry.finish().expect("entry finish");
    }
    {
        let payload = b"compress me ".repeat(200);
        let mut entry = writer
            .start_entry("packed.bin", &options(ZipCompressionMethod::Deflate))
            .expect("start deflate");
        entry.write_all(&payload).expect("write");
        entry.finish().expect("entry finish");
    }
    assert_eq!(writer.entry_count(), 2);
    let bytes = writer.finish().expect("archive finish").into_inner();

    let mut archive = ZipArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default())
        .expect("round-trip parse");
    let receipt = archive.verify_all().expect("verify_all");
    assert_eq!(receipt.entries_total(), 2);

    let mut entry = archive.open_entry(0).expect("open");
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).expect("read");
    assert_eq!(buf, b"plain bytes");
    entry.finish().expect("finish");

    let info = archive.entry_info(1).expect("info");
    assert_eq!(info.name().as_bytes(), b"packed.bin");
    let mut entry = archive.open_entry(1).expect("open");
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).expect("read");
    assert_eq!(buf, b"compress me ".repeat(200));
    entry.finish().expect("finish");
}

#[test]
fn empty_archive_round_trip() {
    let writer = ZipArchiveWriter::new(Cursor::new(Vec::new()));
    let bytes = writer.finish().expect("finish").into_inner();
    let mut archive = ZipArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default())
        .expect("empty archive parses");
    assert_eq!(archive.entry_count(), 0);
    let receipt = archive.verify_all().expect("verify_all");
    assert_eq!(receipt.entries_total(), 0);
    assert_eq!(receipt.payload_bytes(), 0);
}

#[test]
fn directory_marker_round_trip() {
    let mut writer = ZipArchiveWriter::new(Cursor::new(Vec::new()));
    writer.add_directory("assets").expect("add directory");
    let bytes = writer.finish().expect("finish").into_inner();
    let mut archive =
        ZipArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default()).expect("parse");
    let info = archive.entry_info(0).expect("info");
    assert_eq!(info.kind(), EntryKind::Directory);
    assert_eq!(info.name().as_bytes(), b"assets/");
    archive.verify_all().expect("verify_all");
}

#[test]
fn incomplete_entry_drop_poisons_archive() {
    let mut writer = ZipArchiveWriter::new(Cursor::new(Vec::new()));
    {
        let mut entry = writer
            .start_entry("leak.bin", &options(ZipCompressionMethod::Stored))
            .expect("start");
        entry.write_all(b"never finished").expect("write");
        // dropped without finish — not a completion proof
    }
    // Starting a new entry is rejected: the archive is incomplete.
    let kind = kind_of(writer.start_entry("next", &options(ZipCompressionMethod::Stored)));
    assert_eq!(kind, ArchiveErrorKind::InvalidState);
    // And finishing the archive is rejected too.
    let kind = kind_of(writer.finish());
    assert_eq!(kind, ArchiveErrorKind::InvalidState);
}

#[test]
fn unfinished_entry_dropped_without_write_still_poisons() {
    let mut writer = ZipArchiveWriter::new(Cursor::new(Vec::new()));
    {
        let _entry = writer
            .start_entry("leak.bin", &options(ZipCompressionMethod::Stored))
            .expect("start");
    }
    let kind = kind_of(writer.finish());
    assert_eq!(kind, ArchiveErrorKind::InvalidState);
}

#[test]
fn archive_is_not_complete_until_finish() {
    // The bytes written before `ZipArchiveWriter::finish` contain no
    // central directory / EOCD — they must not parse as an archive.
    let mut writer = ZipArchiveWriter::new(Cursor::new(Vec::new()));
    {
        let mut entry = writer
            .start_entry("a", &options(ZipCompressionMethod::Stored))
            .expect("start");
        entry.write_all(b"data").expect("write");
        entry.finish().expect("entry finish");
    }
    let partial = writer
        .into_inner() // abandon: no CD/EOCD written
        .into_inner();
    let error = ZipArchiveReader::new(Cursor::new(partial), ArchiveReadLimits::default())
        .err()
        .expect("unfinished archive must not parse");
    assert_eq!(error.kind(), ArchiveErrorKind::Truncated);
}

#[test]
fn entry_finish_separated_from_archive_finish() {
    // Completing an entry still requires `ZipArchiveWriter::finish` to
    // emit the central directory + EOCD; only then is the archive valid.
    let mut writer = ZipArchiveWriter::new(Cursor::new(Vec::new()));
    {
        let mut entry = writer
            .start_entry("a", &options(ZipCompressionMethod::Stored))
            .expect("start");
        entry.write_all(b"data").expect("write");
        entry.finish().expect("entry finish");
    }
    let bytes = writer.finish().expect("archive finish").into_inner();
    let mut archive =
        ZipArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default()).expect("parse");
    archive.verify_all().expect("verify_all");
}

#[test]
fn invalid_entry_names_are_rejected() {
    let mut writer = ZipArchiveWriter::new(Cursor::new(Vec::new()));
    assert_eq!(
        kind_of(writer.start_entry("", &options(ZipCompressionMethod::Stored))),
        ArchiveErrorKind::InvalidInput
    );
    assert_eq!(
        kind_of(writer.start_entry("has\0nul", &options(ZipCompressionMethod::Stored))),
        ArchiveErrorKind::InvalidInput
    );
    let long_name = "n".repeat(65_536);
    assert_eq!(
        kind_of(writer.start_entry(&long_name, &options(ZipCompressionMethod::Stored))),
        ArchiveErrorKind::InvalidInput
    );
    let bytes = writer
        .finish()
        .expect("still a valid empty archive")
        .into_inner();
    let archive =
        ZipArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default()).expect("parse");
    assert_eq!(archive.entry_count(), 0);
}

#[test]
fn classic_entry_count_limit() {
    let mut writer = ZipArchiveWriter::new(Cursor::new(Vec::new()));
    for index in 0..u16::MAX {
        let entry = writer
            .start_entry("e", &options(ZipCompressionMethod::Stored))
            .unwrap_or_else(|error| panic!("entry {index} should open: {error}"));
        entry.finish().expect("entry finish");
    }
    let kind = kind_of(writer.start_entry("e", &options(ZipCompressionMethod::Stored)));
    assert_eq!(kind, ArchiveErrorKind::LimitExceeded);
}

#[test]
fn entry_finish_writes_measured_sizes() {
    let mut writer = ZipArchiveWriter::new(Cursor::new(Vec::new()));
    {
        let mut entry = writer
            .start_entry("a", &options(ZipCompressionMethod::Deflate))
            .expect("start");
        let payload = vec![0xAB_u8; 4096];
        entry.write_all(&payload).expect("write");
        entry.finish().expect("entry finish");
    }
    let bytes = writer.finish().expect("finish").into_inner();
    let archive =
        ZipArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default()).expect("parse");
    let info = archive.entry_info(0).expect("info");
    assert_eq!(info.uncompressed_size(), Some(4096));
    // 4096 identical bytes deflate far below their stored size.
    let compressed = info
        .compressed_size()
        .expect("deflate records compressed size");
    assert!(compressed < 4096);
}
