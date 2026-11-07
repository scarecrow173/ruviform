//! TAR/ZIP → `Gzip`/`Zstd` → `StagedFile` composition boundary (R3).
//!
//! The boundary under test: archive containers carry the payloads,
//! compression codecs wrap the container bytes, and the staged file
//! carries the composite to publish. Composition is only valid when
//! every layer produces its completion proof — archive finish,
//! encoder finish, decoder verified-EOF, sealed readback.

mod common;

use std::io::{BufReader, Cursor, Read, Write};

use common::{TestDir, exists};
use ruviform_io_archive::tar::{TarArchiveReader, TarArchiveWriter, TarEntryOptions};
use ruviform_io_archive::{ArchiveReadLimits, EntryKind};
use ruviform_io_compression::gzip;
use ruviform_io_compression::zstd;
use ruviform_io_file::{Durability, PublishMode, PublishOptions, StagedFile};

const FILES: &[(&str, &[u8])] = &[
    ("manifest.txt", b"release=rf306\n"),
    ("data/track-a.raw", b"\x00\x01\x02\x03pcm"),
    ("data/track-b.raw", b"\x04\x05\x06\x07pcm"),
];

fn options() -> PublishOptions {
    PublishOptions::new(PublishMode::CreateNew, Durability::FileSync)
}

/// Writes a small ustar TAR into `sink`, finishing every entry and
/// the archive itself — TAR's explicit completion proofs.
fn write_tar_into<W: Write>(sink: W) -> W {
    let mut writer = TarArchiveWriter::new(sink);
    for (name, payload) in FILES {
        let mut entry = writer
            .start_entry(
                name,
                u64::try_from(payload.len()).expect("len"),
                &TarEntryOptions::default(),
            )
            .expect("start entry");
        entry.write_all(payload).expect("entry payload");
        entry.finish().expect("finish entry");
    }
    writer.finish().expect("finish archive")
}

/// Reads the whole TAR stream back into `(name, payload)` pairs.
fn read_tar_from<R: Read>(reader: R) -> Vec<(String, Vec<u8>)> {
    let mut archive = TarArchiveReader::new(reader, ArchiveReadLimits::default());
    let mut entries = Vec::new();
    while let Some(mut entry) = archive.next_entry().expect("next entry") {
        assert_eq!(entry.info().kind(), EntryKind::File);
        let mut payload = Vec::new();
        entry.read_to_end(&mut payload).expect("entry payload");
        entries.push((entry.info().name().to_string_lossy().into_owned(), payload));
        entry.finish().expect("entry finish");
    }
    archive.finish().expect("archive finish");
    entries
}

#[test]
fn tar_gzip_staged_round_trip_with_layered_completion_proofs() {
    let dir = TestDir::new("tar-gz");
    let dest = dir.dest("bundle.tar.gz");

    let mut staged = StagedFile::create(&dest).expect("stage");
    {
        // gzip member must be explicitly finished before the TAR's
        // framing even matters — the encoder owns the trailer.
        let mut encoder =
            gzip::Encoder::new(&mut staged, gzip::EncodeOptions::default()).expect("encoder");
        write_tar_into(&mut encoder);
        encoder.finish().expect("gzip finish — trailer written");
    }

    let mut sealed = staged.seal();
    // Independent pre-publish verification: decode the sealed
    // composite through gzip → TAR, requiring verified EOF at both
    // layers.
    let mut readback = sealed.open_readback().expect("readback");
    let mut buffered = BufReader::new(&mut readback);
    let mut decoder = gzip::Decoder::new(&mut buffered, gzip::DecodeOptions::default());
    let entries = read_tar_from(&mut decoder);
    decoder.finish().expect("gzip verified EOF");

    let expected: Vec<(String, Vec<u8>)> = FILES
        .iter()
        .map(|(n, p)| (n.to_string(), p.to_vec()))
        .collect();
    assert_eq!(
        entries, expected,
        "composite round-trip must preserve entries"
    );

    sealed
        .publish(options())
        .expect("publish verified composite");
    assert!(exists(&dest));
}

#[test]
fn tar_zstd_staged_round_trip() {
    let dir = TestDir::new("tar-zst");
    let dest = dir.dest("bundle.tar.zst");

    let mut staged = StagedFile::create(&dest).expect("stage");
    {
        let mut encoder =
            zstd::Encoder::new(&mut staged, zstd::EncodeOptions::default()).expect("encoder");
        write_tar_into(&mut encoder);
        encoder.finish().expect("zstd finish");
    }

    let mut sealed = staged.seal();
    let mut readback = sealed.open_readback().expect("readback");
    let mut buffered = BufReader::new(&mut readback);
    let mut decoder = zstd::Decoder::new(&mut buffered, zstd::DecodeOptions::default());
    let entries = read_tar_from(&mut decoder);
    decoder.finish().expect("zstd verified EOF");
    let expected: Vec<(String, Vec<u8>)> = FILES
        .iter()
        .map(|(n, p)| (n.to_string(), p.to_vec()))
        .collect();
    assert_eq!(entries, expected);

    sealed.publish(options()).expect("publish");
}

#[test]
fn dropped_encoder_without_finish_yields_unverifiable_bytes() {
    let dir = TestDir::new("unfinished-gz");
    let dest = dir.dest("dangling.tar.gz");

    let mut staged = StagedFile::create(&dest).expect("stage");
    {
        let mut encoder =
            gzip::Encoder::new(&mut staged, gzip::EncodeOptions::default()).expect("encoder");
        write_tar_into(&mut encoder);
        // Deliberately drop without finish() — no trailer is emitted.
    }
    let mut sealed = staged.seal();
    let mut readback = sealed.open_readback().expect("readback");
    let mut buffered = BufReader::new(&mut readback);
    let mut decoder = gzip::Decoder::new(&mut buffered, gzip::DecodeOptions::default());
    // Payload may stream, but the finish proof must fail: no CRC/ISIZE.
    let mut sink = Vec::new();
    let _ = decoder.read_to_end(&mut sink);
    assert!(
        decoder.finish().is_err(),
        "decoder must not report verified EOF on a member with no trailer"
    );
    // Boundary contract: bytes that fail verification are not published.
    sealed.abort().expect("abort");
    assert!(!exists(&dest));
}

#[test]
fn truncated_gzip_member_fails_finish_before_publish() {
    let dir = TestDir::new("trunc-gz");
    let dest = dir.dest("cut.tar.gz");

    // Build a valid composite, then cut the last bytes — simulating a
    // staged artifact that lost its completion proof to an I/O fault.
    let mut bytes = Vec::new();
    {
        let mut encoder =
            gzip::Encoder::new(&mut bytes, gzip::EncodeOptions::default()).expect("encoder");
        write_tar_into(&mut encoder);
        encoder.finish().expect("finish");
    }
    bytes.truncate(bytes.len() - 5);

    let mut staged = StagedFile::create(&dest).expect("stage");
    staged.write_all(&bytes).expect("write truncated");
    let mut sealed = staged.seal();
    let mut readback = sealed.open_readback().expect("readback");
    let mut buffered = BufReader::new(&mut readback);
    let mut decoder = gzip::Decoder::new(&mut buffered, gzip::DecodeOptions::default());
    let mut sink = Vec::new();
    let _ = decoder.read_to_end(&mut sink);
    assert!(
        decoder.finish().is_err(),
        "truncated member must not verify"
    );
    sealed.abort().expect("abort");
    assert!(!exists(&dest));
}

#[test]
fn declared_size_hint_does_not_relax_streamed_budget() {
    // A TAR entry whose declared size under-reports reality is caught
    // by the writer side (declared size is a contract), and a reader
    // budget caps streamed output regardless of declared values.
    let dir = TestDir::new("budget");
    let dest = dir.dest("budget.tar.gz");

    let mut staged = StagedFile::create(&dest).expect("stage");
    {
        let mut encoder =
            gzip::Encoder::new(&mut staged, gzip::EncodeOptions::default()).expect("encoder");
        let mut writer = TarArchiveWriter::new(&mut encoder);
        let mut entry = writer
            .start_entry("big.bin", 4, &TarEntryOptions::default())
            .expect("entry");
        // The entry writer enforces the declared size at write time —
        // bytes beyond the declaration never reach the stream.
        assert!(
            entry.write_all(b"0123456789").is_err(),
            "payload exceeding declared size must fail at write"
        );
    }

    // Budget-limited read of a well-formed archive: a small
    // max_total_uncompressed_bytes limit must bind during streaming.
    let mut bytes = Vec::new();
    write_tar_into(&mut bytes);
    let mut limits = ArchiveReadLimits::default();
    limits.max_total_uncompressed_bytes = 8;
    let mut archive = TarArchiveReader::new(Cursor::new(&bytes), limits);
    // The streamed cap can surface as an ArchiveError (header/finish
    // path) or an I/O error mid-entry — either classification means
    // the budget bound the output before the caller saw the bytes.
    let result: Result<(), String> = (|| {
        while let Some(mut entry) = archive.next_entry().map_err(|e| e.to_string())? {
            let mut sink = Vec::new();
            entry.read_to_end(&mut sink).map_err(|e| e.to_string())?;
            entry.finish().map_err(|e| e.to_string())?;
        }
        archive.finish().map(|_| ()).map_err(|e| e.to_string())
    })();
    assert!(result.is_err(), "total-bytes budget must bind");
    drop(staged);
    assert!(!exists(&dest));
}

#[test]
fn staged_seek_cursor_supports_zip_indexed_layout() {
    // ZIP needs Seek on the sink — prove StagedFile's Seek impl lets a
    // ZIP writer lay down data descriptors + central directory, then
    // an indexed reader verifies from the sealed readback.
    use ruviform_io_archive::zip::{
        ZipArchiveReader, ZipArchiveWriter, ZipCompressionMethod, ZipEntryOptions,
    };

    let dir = TestDir::new("zip-staged");
    let dest = dir.dest("bundle.zip");

    let mut staged = StagedFile::create(&dest).expect("stage");
    {
        let mut writer = ZipArchiveWriter::new(&mut staged);
        for (name, payload) in FILES {
            let mut entry = writer
                .start_entry(name, &ZipEntryOptions::new(ZipCompressionMethod::Stored))
                .expect("entry");
            entry.write_all(payload).expect("payload");
            entry.finish().expect("entry finish");
        }
        writer.finish().expect("central directory written");
    }

    let mut sealed = staged.seal();
    let mut readback = sealed.open_readback().expect("readback");
    let mut archive = ZipArchiveReader::new(&mut readback, ArchiveReadLimits::default())
        .expect("indexed open on sealed bytes");
    assert_eq!(archive.entry_count(), FILES.len());
    let receipt = archive.verify_all().expect("verify_all");
    assert_eq!(
        usize::try_from(receipt.entries_verified()).expect("count"),
        FILES.len()
    );

    sealed.publish(options()).expect("publish zip");
    assert!(exists(&dest));
}
