//! Deterministic negative/fuzz fixtures: hostile archive entries the
//! writer cannot emit, resource budgets, rejected document features,
//! corrupted inputs classified by typed errors, and the mutation-sweep
//! publish gate.
//!
//! Slice-2 rules F5–F9
//! (`docs/test-specs/issue-306-io-e2e-fault-fuzz.md`). Every fixture
//! is deterministic and bounded — no coverage-guided fuzzing, no
//! unbounded loops; a panic is a test failure by construction.

mod common;

use std::io::{BufReader, Cursor, Read, Write};

use common::{TestDir, crafted_tar_entry, exists, patch_zip_external_attrs};
use ruviform_io_archive::extract::{ExtractOptions, ExtractionSession};
use ruviform_io_archive::tar::{TarArchiveReader, TarArchiveWriter, TarEntryOptions};
use ruviform_io_archive::zip::{
    ZipArchiveReader, ZipArchiveWriter, ZipCompressionMethod, ZipEntryOptions,
};
use ruviform_io_archive::{ArchiveErrorKind, ArchiveReadLimits, EntryKind};
use ruviform_io_compression::gzip;
use ruviform_io_file::{Durability, PublishMode, PublishOptions, StagedFile};
use ruviform_io_structured::{DocumentErrorKind, DocumentFileReadError, json, xml};

fn publish_options() -> PublishOptions {
    PublishOptions::new(PublishMode::CreateNew, Durability::FileSync)
}

/// Minimal document type for `json::read_file` fixtures — the parse
/// failure under test fires before the type matters.
#[derive(Debug, serde::Deserialize)]
struct Doc {
    #[allow(dead_code)]
    name: String,
}

/// Creates an extraction session whose staging parent lives under
/// `dir` (same shape as `extraction.rs`).
fn session(dir: &TestDir, tag: &str, limits: ArchiveReadLimits) -> ExtractionSession {
    let parent = dir.dest(tag);
    std::fs::create_dir(&parent).expect("staging parent");
    let mut options = ExtractOptions::default();
    options.limits = limits;
    ExtractionSession::create(&parent, options).expect("session")
}

/// Builds a single-entry Stored ZIP in memory and returns its bytes.
fn zip_bytes(name: &str, payload: &[u8]) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    {
        let mut writer = ZipArchiveWriter::new(&mut bytes);
        let mut entry = writer
            .start_entry(name, &ZipEntryOptions::new(ZipCompressionMethod::Stored))
            .expect("entry");
        entry.write_all(payload).expect("payload");
        entry.finish().expect("entry finish");
        writer.finish().expect("zip finish");
    }
    bytes.into_inner()
}

/// Little-endian u16 at `at` — local fixture inspection only.
fn u16le(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

// -------------------------------------------------------------------
// F5: hostile entry kinds the writer cannot emit
// -------------------------------------------------------------------

#[test]
fn zip_symlink_entry_is_classified_and_extraction_refused() {
    let dir = TestDir::new("zip-symlink");

    // A Stored entry whose payload is a would-be link target. The
    // writer cannot emit link kinds — patch the central-directory
    // external_attrs to unix S_IFLNK|0777 (high 16 bits), leaving
    // every CRC/size field untouched.
    let mut hostile = zip_bytes("link.txt", b"/etc/passwd");
    let patched = patch_zip_external_attrs(&mut hostile, 0o120_777 << 16);
    assert_eq!(patched, 1, "exactly one CD record must be patched");

    // Stage + publish the archive: it is still a *valid* ZIP — the
    // hostile kind is a classification/policy matter, not corruption.
    let dest = dir.dest("hostile.zip");
    let mut staged = StagedFile::create(&dest).expect("stage");
    staged.write_all(&hostile).expect("write");
    let mut sealed = staged.seal();
    {
        let mut readback = sealed.open_readback().expect("readback");
        let archive = ZipArchiveReader::new(&mut readback, ArchiveReadLimits::default())
            .expect("indexed open");
        assert_eq!(archive.entry_count(), 1);
        let info = archive.entry_info(0).expect("entry info");
        assert_eq!(
            info.kind(),
            EntryKind::SymbolicLink,
            "patched external_attrs must classify as symlink"
        );
    }
    sealed
        .publish(publish_options())
        .expect("publish valid zip");
    assert!(exists(&dest));

    // The extraction layer — not the archive parser — owns the
    // refusal: links are never extracted under the secure default.
    let published = common::read(&dest).expect("published bytes");
    let mut cursor = Cursor::new(&published[..]);
    let mut archive =
        ZipArchiveReader::new(&mut cursor, ArchiveReadLimits::default()).expect("reopen");
    let failure = session(&dir, "stage", ArchiveReadLimits::default())
        .extract_zip(&mut archive)
        .expect_err("symlink entry must be refused");
    assert_eq!(failure.error().kind(), ArchiveErrorKind::InvalidInput);
    assert!(
        !exists(&failure.staging_root),
        "refused extraction must not leave a usable staging tree"
    );
}

// -------------------------------------------------------------------
// F6: resource budgets
// -------------------------------------------------------------------

/// 1 MiB of zeros compresses to a few KiB — the declared-vs-streamed
/// budget fixture (zip-bomb shape without malicious intent).
fn bomb_zip_bytes() -> Vec<u8> {
    let payload = vec![0_u8; 1 << 20];
    let mut bytes = Cursor::new(Vec::new());
    {
        let mut writer = ZipArchiveWriter::new(&mut bytes);
        let mut entry = writer
            .start_entry(
                "bomb.bin",
                &ZipEntryOptions::new(ZipCompressionMethod::Deflate),
            )
            .expect("entry");
        // Write in chunks so the writer sees a realistic stream.
        for chunk in payload.chunks(64 * 1024) {
            entry.write_all(chunk).expect("payload chunk");
        }
        entry.finish().expect("entry finish");
        writer.finish().expect("zip finish");
    }
    bytes.into_inner()
}

#[test]
fn zip_bomb_entry_budget_binds_at_read_time() {
    let zip = bomb_zip_bytes();
    assert!(
        zip.len() < 128 * 1024,
        "fixture must actually compress (got {} bytes)",
        zip.len()
    );

    let mut limits = ArchiveReadLimits::default();
    limits.max_entry_uncompressed_bytes = 64 * 1024; // << 1 MiB declared

    let mut cursor = Cursor::new(&zip[..]);
    let outcome = ZipArchiveReader::new(&mut cursor, limits)
        .and_then(|mut reader| reader.verify_all().map(|_| ()));
    let error = outcome.expect_err("entry budget must bind");
    assert_eq!(error.kind(), ArchiveErrorKind::LimitExceeded);
}

#[test]
fn zip_bomb_extraction_budgets_bind_declared_and_streamed() {
    let zip = bomb_zip_bytes();

    // Declared-total budget: the plan phase refuses before any write.
    let mut limits = ArchiveReadLimits::default();
    limits.max_total_uncompressed_bytes = 128 * 1024;
    let mut cursor = Cursor::new(&zip[..]);
    let mut reader = ZipArchiveReader::new(&mut cursor, limits.clone()).expect("indexed open");
    let dir = TestDir::new("bomb-total");
    let failure = session(&dir, "stage", limits)
        .extract_zip(&mut reader)
        .expect_err("declared total must exceed budget");
    assert_eq!(failure.error().kind(), ArchiveErrorKind::LimitExceeded);
    assert!(!exists(&failure.staging_root));

    // Entry-count budget on a multi-entry archive.
    let mut many = Cursor::new(Vec::new());
    {
        let mut writer = ZipArchiveWriter::new(&mut many);
        for i in 0..4 {
            let name = format!("f{i}.txt");
            let mut entry = writer
                .start_entry(&name, &ZipEntryOptions::new(ZipCompressionMethod::Stored))
                .expect("entry");
            entry.write_all(b"x").expect("payload");
            entry.finish().expect("entry finish");
        }
        writer.finish().expect("zip finish");
    }
    let many_bytes = many.into_inner();
    let mut count_limits = ArchiveReadLimits::default();
    count_limits.max_entries = 2;
    let mut cursor = Cursor::new(&many_bytes[..]);
    // The reader itself refuses an over-budget declared count at
    // index time; either binding point proves the budget holds.
    match ZipArchiveReader::new(&mut cursor, count_limits.clone()) {
        Err(error) => assert_eq!(error.kind(), ArchiveErrorKind::LimitExceeded),
        Ok(mut reader) => {
            let dir = TestDir::new("bomb-count");
            let failure = session(&dir, "stage", count_limits)
                .extract_zip(&mut reader)
                .expect_err("entry count must exceed extraction budget");
            assert_eq!(failure.error().kind(), ArchiveErrorKind::LimitExceeded);
            assert!(!exists(&failure.staging_root));
        }
    }
}

// -------------------------------------------------------------------
// F7: rejected document features (XML secure defaults)
// -------------------------------------------------------------------

#[test]
fn xml_doctype_internal_subset_is_rejected() {
    let dir = TestDir::new("xml-doctype-int");
    let dest = dir.dest("doc.xml");
    let mut staged = StagedFile::create(&dest).expect("stage");
    staged
        .write_all(b"<!DOCTYPE r [<!ENTITY x \"y\">]><r>&x;</r>")
        .expect("write");
    staged.seal().publish(publish_options()).expect("publish");

    match xml::read_file(&dest, &xml::XmlReadOptions::default()) {
        Err(DocumentFileReadError::Decode(error)) => {
            assert_eq!(error.kind(), DocumentErrorKind::Unsupported);
        }
        other => panic!("expected Decode(Unsupported), got {other:?}"),
    }
}

#[test]
fn xml_doctype_external_identifier_is_rejected_without_fetch() {
    let dir = TestDir::new("xml-doctype-ext");
    let dest = dir.dest("doc.xml");
    let mut staged = StagedFile::create(&dest).expect("stage");
    // A file:// external identifier: the reader must refuse the
    // DOCTYPE outright — no fetch is ever attempted.
    staged
        .write_all(b"<!DOCTYPE r SYSTEM \"file:///etc/passwd\"><r/>")
        .expect("write");
    staged.seal().publish(publish_options()).expect("publish");

    match xml::read_file(&dest, &xml::XmlReadOptions::default()) {
        Err(DocumentFileReadError::Decode(error)) => {
            assert_eq!(error.kind(), DocumentErrorKind::Unsupported);
        }
        other => panic!("expected Decode(Unsupported), got {other:?}"),
    }
}

#[test]
fn xml_undefined_general_entity_is_rejected() {
    let dir = TestDir::new("xml-entity");
    let dest = dir.dest("doc.xml");
    let mut staged = StagedFile::create(&dest).expect("stage");
    staged.write_all(b"<r>&undefined;</r>").expect("write");
    staged.seal().publish(publish_options()).expect("publish");

    match xml::read_file(&dest, &xml::XmlReadOptions::default()) {
        Err(DocumentFileReadError::Decode(error)) => {
            assert_eq!(error.kind(), DocumentErrorKind::Unsupported);
        }
        other => panic!("expected Decode(Unsupported), got {other:?}"),
    }
}

// -------------------------------------------------------------------
// F8: corrupted fixtures fail with typed errors, never panic
// -------------------------------------------------------------------

#[test]
fn corrupted_zip_fixtures_fail_with_typed_archive_errors() {
    // Bad magic / missing EOCD → Truncated (the reader's own
    // end-of-central-directory search finds nothing).
    let mut cursor = Cursor::new(b"PK\x03\x04not-a-real-zip".to_vec());
    let error = ZipArchiveReader::new(&mut cursor, ArchiveReadLimits::default())
        .err()
        .expect("garbage must not open");
    assert_eq!(error.kind(), ArchiveErrorKind::Truncated);

    // Valid ZIP with one payload byte flipped → CRC32 mismatch on
    // verification. The payload offset is derived from the local
    // header's own name/extra lengths, not hard-coded.
    let mut zip = zip_bytes("a.txt", b"hello world");
    let name_len = usize::from(u16le(&zip, 26));
    let extra_len = usize::from(u16le(&zip, 28));
    let payload_at = 30 + name_len + extra_len;
    zip[payload_at] ^= 0xFF;
    let mut cursor = Cursor::new(&zip[..]);
    let mut reader = ZipArchiveReader::new(&mut cursor, ArchiveReadLimits::default())
        .expect("structurally indexed");
    let error = reader
        .verify_all()
        .expect_err("flipped payload must fail CRC");
    assert_eq!(error.kind(), ArchiveErrorKind::ChecksumMismatch);

    // Truncated central directory: cut the EOCD tail off a valid ZIP.
    let zip = zip_bytes("a.txt", b"hello");
    let truncated = zip[..zip.len() - 10].to_vec();
    let mut cursor = Cursor::new(&truncated[..]);
    let Err(error) = ZipArchiveReader::new(&mut cursor, ArchiveReadLimits::default()) else {
        panic!("missing EOCD must not open")
    };
    assert!(
        matches!(
            error.kind(),
            ArchiveErrorKind::Truncated | ArchiveErrorKind::Malformed
        ),
        "unexpected kind: {}",
        error.kind()
    );
}

#[test]
fn corrupted_tar_fixtures_fail_with_typed_archive_errors() {
    // Header checksum mismatch: flip a byte inside the crafted
    // entry's name field — the stored checksum no longer matches.
    let mut body = crafted_tar_entry("good.txt", b'0', "", b"data");
    body[5] ^= 0xFF;
    let mut reader = TarArchiveReader::new(Cursor::new(body), ArchiveReadLimits::default());
    let Err(error) = reader.next_entry() else {
        panic!("bad checksum must surface at header parse")
    };
    assert_eq!(error.kind(), ArchiveErrorKind::ChecksumMismatch);

    // Truncated archive: cut inside the payload region.
    let body = crafted_tar_entry("big.bin", b'0', "", &[0xAB; 1024]);
    let truncated = body[..600].to_vec();
    let reader = TarArchiveReader::new(Cursor::new(truncated), ArchiveReadLimits::default());
    let result = drain_tar(reader);
    let error = result.expect_err("truncated tar must fail");
    assert!(
        matches!(
            error.kind(),
            ArchiveErrorKind::Truncated | ArchiveErrorKind::Malformed
        ),
        "unexpected kind: {}",
        error.kind()
    );
}

/// Drains a TAR reader to `finish`, surfacing the first typed error.
/// Takes ownership because `finish` consumes the reader.
fn drain_tar<R: Read>(
    mut reader: TarArchiveReader<R>,
) -> Result<(), ruviform_io_archive::ArchiveError> {
    while let Some(mut entry) = reader.next_entry()? {
        let mut sink = Vec::new();
        if entry.read_to_end(&mut sink).is_err() {
            // The entry reader embeds the typed ArchiveError inside the
            // io::Error; finish() re-surfaces it with its real kind.
            return entry.finish().map(|_| ());
        }
        entry.finish()?;
    }
    reader.finish().map(|_| ())
}

#[test]
fn corrupted_gzip_and_json_fixtures_fail_without_panic() {
    // Bad gzip magic: the decoder surfaces an error on read/finish,
    // never a panic.
    let mut buffered = BufReader::new(Cursor::new(b"\x1f\x8bcorrupted".to_vec()));
    let mut decoder = gzip::Decoder::new(&mut buffered, gzip::DecodeOptions::default());
    let mut sink = Vec::new();
    let read_result = decoder.read_to_end(&mut sink);
    assert!(
        read_result.is_err() || decoder.finish().is_err(),
        "corrupt gzip member must fail verification"
    );

    // Invalid UTF-8 published JSON: the document read boundary
    // classifies it as Decode, never panics.
    let dir = TestDir::new("json-utf8");
    let dest = dir.dest("doc.json");
    let mut staged = StagedFile::create(&dest).expect("stage");
    staged.write_all(&[0xFF, 0xFE, 0x00, 0x41]).expect("write");
    staged.seal().publish(publish_options()).expect("publish");
    let result: Result<Doc, DocumentFileReadError> =
        json::read_file(&dest, &json::JsonReadOptions::default());
    assert!(
        matches!(result, Err(DocumentFileReadError::Decode(_))),
        "invalid UTF-8 JSON must be a Decode failure"
    );
}

// -------------------------------------------------------------------
// F9: mutation sweep — the publish gate
// -------------------------------------------------------------------

/// The composite fixture: TAR entries through a finished gzip
/// encoder — identical layering to `archive_compression_staged.rs`.
fn composite_fixture() -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder =
            gzip::Encoder::new(&mut out, gzip::EncodeOptions::default()).expect("encoder");
        {
            let mut writer = TarArchiveWriter::new(&mut encoder);
            let mut entry = writer
                .start_entry("a.txt", 5, &TarEntryOptions::default())
                .expect("entry");
            entry.write_all(b"hello").expect("payload");
            entry.finish().expect("entry finish");
            writer.finish().expect("tar finish");
        }
        encoder.finish().expect("gzip finish");
    }
    out
}

/// Independent layered verification: gzip verified EOF wrapping a
/// TAR reader that reaches `finish`. `false` means the bytes are not
/// a complete composite — the publish gate must hold.
fn composite_verifies(bytes: &[u8]) -> bool {
    let mut buffered = BufReader::new(Cursor::new(bytes));
    let mut decoder = gzip::Decoder::new(&mut buffered, gzip::DecodeOptions::default());
    let tar_ok = (|| -> Option<()> {
        let mut archive = TarArchiveReader::new(&mut decoder, ArchiveReadLimits::default());
        while let Some(mut entry) = archive.next_entry().ok()? {
            let mut sink = Vec::new();
            entry.read_to_end(&mut sink).ok()?;
            entry.finish().ok()?;
        }
        archive.finish().ok()?;
        Some(())
    })();
    tar_ok.is_some() && decoder.finish().is_ok()
}

/// Deterministic mutants of the valid composite. Each must fail
/// independent verification; the test then proves the caller-side
/// gate (abort, no publish) leaves no destination.
fn mutants(valid: &[u8]) -> Vec<(&'static str, Vec<u8>)> {
    let mut out: Vec<(&'static str, Vec<u8>)> = Vec::new();

    // Trailer truncation: the gzip CRC/ISIZE proof is cut.
    let mut m = valid.to_vec();
    m.truncate(m.len() - 8);
    out.push(("truncated-gzip-trailer", m));

    // Magic-byte flip: not a gzip member at all.
    let mut m = valid.to_vec();
    m[0] ^= 0xFF;
    out.push(("gzip-magic-flip", m));

    // Mid-stream flip: deflate corruption surfaces as decode error
    // or trailer mismatch.
    let mut m = valid.to_vec();
    let mid = m.len() / 2;
    m[mid] ^= 0xFF;
    out.push(("mid-stream-flip", m));

    // ISIZE flip: decompressed length disagrees with the trailer.
    let mut m = valid.to_vec();
    let last = m.len() - 1;
    m[last] ^= 0xFF;
    out.push(("isize-flip", m));

    // Valid gzip wrapping a TAR whose end-of-archive marker was cut —
    // the compression layer verifies while the archive layer must not.
    // Both zero blocks are removed: a single remaining zero block is a
    // legal terminator for this reader, so one block is not a mutant.
    let mut tar = Vec::new();
    {
        let mut writer = TarArchiveWriter::new(&mut tar);
        let mut entry = writer
            .start_entry("a.txt", 5, &TarEntryOptions::default())
            .expect("entry");
        entry.write_all(b"hello").expect("payload");
        entry.finish().expect("entry finish");
        writer.finish().expect("tar finish");
    }
    tar.truncate(tar.len().saturating_sub(1024));
    let mut gz = Vec::new();
    {
        let mut encoder = gzip::Encoder::new(&mut gz, gzip::EncodeOptions::default()).expect("enc");
        encoder.write_all(&tar).expect("tar bytes");
        encoder.finish().expect("gzip finish");
    }
    out.push(("tar-terminator-missing", gz));

    // TAR header corruption inside a valid gzip wrapper: the ustar
    // name byte is flipped so the header checksum no longer matches.
    let mut tar = Vec::new();
    {
        let mut writer = TarArchiveWriter::new(&mut tar);
        let mut entry = writer
            .start_entry("a.txt", 5, &TarEntryOptions::default())
            .expect("entry");
        entry.write_all(b"hello").expect("payload");
        entry.finish().expect("entry finish");
        writer.finish().expect("tar finish");
    }
    tar[5] ^= 0xFF;
    let mut gz = Vec::new();
    {
        let mut encoder = gzip::Encoder::new(&mut gz, gzip::EncodeOptions::default()).expect("enc");
        encoder.write_all(&tar).expect("tar bytes");
        encoder.finish().expect("gzip finish");
    }
    out.push(("tar-header-checksum", gz));

    out
}

#[test]
fn mutation_sweep_never_publishes_unverified_composite() {
    let valid = composite_fixture();
    assert!(composite_verifies(&valid), "fixture must verify");

    for (tag, bytes) in mutants(&valid) {
        assert!(
            !composite_verifies(&bytes),
            "{tag}: mutant must fail independent verification"
        );

        // The publish gate: stage the mutant, prove verification
        // fails on the sealed readback, abort — destination absent.
        let dir = TestDir::new(tag);
        let dest = dir.dest("mutant.tar.gz");
        let mut staged = StagedFile::create(&dest).expect("stage");
        staged.write_all(&bytes).expect("write mutant");
        let mut sealed = staged.seal();
        {
            let mut readback = sealed.open_readback().expect("readback");
            let mut staged_bytes = Vec::new();
            readback
                .read_to_end(&mut staged_bytes)
                .expect("read staged");
            assert!(
                !composite_verifies(&staged_bytes),
                "{tag}: staged mutant must fail verification"
            );
        }
        sealed.abort().expect("abort unverified");
        assert!(!exists(&dest), "{tag}: unverified bytes must not publish");
    }
}
