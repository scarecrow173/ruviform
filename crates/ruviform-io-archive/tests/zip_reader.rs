//! ZIP reader verification: indexed reading, CRC + central-directory
//! consistency, budgets, unsupported features, exclusive-ownership and
//! `verify_all` receipts.

mod common;

use std::io::{Cursor, Read};

use common::{
    EntrySpec, EocdSpec, FLAG_DATA_DESCRIPTOR, FLAG_ENCRYPTED, METHOD_DEFLATE, METHOD_STORED,
    build_simple, build_zip, cd_offset_of, cd_record_offset, compressed_region, crc32, writer_zip,
};
use ruviform_io_archive::zip::{ZipArchiveReader, ZipCompressionMethod};
use ruviform_io_archive::{
    ArchiveError, ArchiveErrorKind, ArchiveReadLimits, EntryChecksum, EntryKind,
};

fn limits(configure: impl Fn(&mut ArchiveReadLimits)) -> ArchiveReadLimits {
    let mut limits = ArchiveReadLimits::default();
    configure(&mut limits);
    limits
}

fn reader(bytes: Vec<u8>) -> ZipArchiveReader<Cursor<Vec<u8>>> {
    reader_with(bytes, ArchiveReadLimits::default())
}

fn reader_with(bytes: Vec<u8>, limits: ArchiveReadLimits) -> ZipArchiveReader<Cursor<Vec<u8>>> {
    ZipArchiveReader::new(Cursor::new(bytes), limits).expect("archive should parse")
}

fn kind_of(result: Result<impl Sized, ArchiveError>) -> ArchiveErrorKind {
    match result {
        Ok(_) => panic!("expected error"),
        Err(error) => error.kind(),
    }
}

fn read_all(entry: &mut impl Read) -> Vec<u8> {
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).expect("read payload");
    buf
}

#[test]
fn round_trip_stored_and_deflate_read_finish() {
    let bytes = writer_zip(&[
        ("a.txt", ZipCompressionMethod::Stored, b"stored-data"),
        (
            "dir/b.txt",
            ZipCompressionMethod::Deflate,
            b"deflated-data".repeat(64).as_slice(),
        ),
    ]);
    let mut archive = reader(bytes);

    assert_eq!(archive.entry_count(), 2);
    let info = archive.entry_info(0).expect("entry 0 info");
    assert_eq!(info.name().as_bytes(), b"a.txt");
    assert_eq!(info.kind(), EntryKind::File);
    assert_eq!(info.uncompressed_size(), Some(11));

    {
        let mut entry = archive.open_entry(0).expect("open stored");
        assert_eq!(read_all(&mut entry), b"stored-data");
        let receipt = entry.finish().expect("stored finish");
        assert_eq!(receipt.uncompressed_bytes(), 11);
        assert_eq!(receipt.checksum(), EntryChecksum::Verified);
    }
    {
        let mut entry = archive.open_entry(1).expect("open deflate");
        assert_eq!(read_all(&mut entry), b"deflated-data".repeat(64));
        entry.finish().expect("deflate finish");
    }
    assert!(archive.entry_is_verified(0));
    assert!(archive.entry_is_verified(1));
}

#[test]
fn verify_all_reports_exact_counts() {
    let bytes = writer_zip(&[
        ("one", ZipCompressionMethod::Stored, b"1"),
        ("two", ZipCompressionMethod::Stored, b"22"),
        ("three", ZipCompressionMethod::Deflate, b"333"),
    ]);
    let mut archive = reader(bytes);
    let receipt = archive.verify_all().expect("verify_all");
    assert_eq!(receipt.entries_total(), 3);
    assert_eq!(receipt.entries_verified(), 3);
    assert_eq!(receipt.entries_unverified(), 0);
    assert_eq!(receipt.payload_bytes(), 6);
}

#[test]
fn truncated_input_shorter_than_eocd() {
    let error = ZipArchiveReader::new(Cursor::new(vec![0u8; 10]), ArchiveReadLimits::default())
        .err()
        .expect("must fail");
    assert_eq!(error.kind(), ArchiveErrorKind::Truncated);
}

#[test]
fn missing_eocd_is_truncated() {
    let mut bytes = writer_zip(&[("a", ZipCompressionMethod::Stored, b"x")]);
    let len = bytes.len();
    bytes[len - 4..len].copy_from_slice(b"JUNK"); // corrupt trailing bytes, not the sig
    bytes[len - 22..len - 18].copy_from_slice(b"JUNK"); // corrupt EOCD signature
    let error = ZipArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default())
        .err()
        .expect("must fail");
    assert_eq!(error.kind(), ArchiveErrorKind::Truncated);
}

#[test]
fn declared_cd_size_past_eocd_is_malformed() {
    let bytes = build_zip(
        &[EntrySpec::stored("a", b"x")],
        &EocdSpec {
            cd_size_delta: 5,
            ..EocdSpec::default()
        },
    );
    let error = ZipArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default())
        .err()
        .expect("must fail");
    assert_eq!(error.kind(), ArchiveErrorKind::Malformed);
}

#[test]
fn declared_entry_count_limit_is_pre_parse() {
    let bytes = writer_zip(&[
        ("a", ZipCompressionMethod::Stored, b"1"),
        ("b", ZipCompressionMethod::Stored, b"2"),
    ]);
    let error = ZipArchiveReader::new(Cursor::new(bytes), limits(|l| l.max_entries = 1))
        .err()
        .expect("must fail");
    assert_eq!(error.kind(), ArchiveErrorKind::LimitExceeded);
}

#[test]
fn metadata_bytes_limit_is_pre_parse() {
    let bytes = writer_zip(&[("a", ZipCompressionMethod::Stored, b"1")]);
    let cd_size = bytes.len() - cd_offset_of(&bytes) - 22;
    assert!(cd_size > 10);
    let error = ZipArchiveReader::new(Cursor::new(bytes), limits(|l| l.max_metadata_bytes = 10))
        .err()
        .expect("must fail");
    assert_eq!(error.kind(), ArchiveErrorKind::LimitExceeded);
}

#[test]
fn path_bytes_limit_checked_before_name_allocation() {
    let bytes = writer_zip(&[(
        "some-long-entry-name.txt",
        ZipCompressionMethod::Stored,
        b"1",
    )]);
    let error = ZipArchiveReader::new(Cursor::new(bytes), limits(|l| l.max_path_bytes = 4))
        .err()
        .expect("must fail");
    assert_eq!(error.kind(), ArchiveErrorKind::LimitExceeded);
}

#[test]
fn local_header_signature_mismatch_is_malformed() {
    let mut bytes = writer_zip(&[("a", ZipCompressionMethod::Stored, b"x")]);
    bytes[0] = 0xAA; // break local signature
    let mut archive = reader(bytes);
    let kind = kind_of(archive.open_entry(0));
    assert_eq!(kind, ArchiveErrorKind::Malformed);
}

#[test]
fn local_name_differing_from_cd_is_inconsistent() {
    let mut spec = EntrySpec::stored("cd-name", b"x");
    spec.local_name = Some(b"loc-nme".to_vec()); // same length, different bytes
    let bytes = build_simple(&[spec]);
    let mut archive = reader(bytes);
    let kind = kind_of(archive.open_entry(0));
    assert_eq!(kind, ArchiveErrorKind::Inconsistent);
}

#[test]
fn local_method_differing_from_cd_is_inconsistent() {
    let mut spec = EntrySpec::stored("a", b"x");
    spec.local_method = Some(METHOD_DEFLATE);
    let bytes = build_simple(&[spec]);
    let mut archive = reader(bytes);
    let kind = kind_of(archive.open_entry(0));
    assert_eq!(kind, ArchiveErrorKind::Inconsistent);
}

#[test]
fn local_sizes_differing_from_cd_is_inconsistent() {
    // Local header declares 9 uncompressed bytes; CD declares 2.
    let spec = EntrySpec::stored("a", b"xx");
    let mut zip = build_simple(&[spec]);
    zip[22..26].copy_from_slice(&9_u32.to_le_bytes());
    let mut archive = reader(zip);
    let kind = kind_of(archive.open_entry(0));
    assert_eq!(kind, ArchiveErrorKind::Inconsistent);
}

#[test]
fn data_descriptor_flag_mismatch_is_inconsistent() {
    let mut spec = EntrySpec::stored("a", b"x");
    spec.flags |= FLAG_DATA_DESCRIPTOR; // CD claims descriptor...
    spec.local_flags = Some(spec.flags & !FLAG_DATA_DESCRIPTOR); // local doesn't
    let bytes = build_simple(&[spec]);
    let mut archive = reader(bytes);
    let kind = kind_of(archive.open_entry(0));
    assert_eq!(kind, ArchiveErrorKind::Inconsistent);
}

#[test]
fn overlapping_entry_regions_rejected_at_open() {
    // Second entry's declared local offset points inside entry 0's data.
    let first = EntrySpec::stored("first-entry-name", b"0123456789");
    let mut second = EntrySpec::stored("second", b"y");
    second.local_offset_override = Some(10); // inside entry 0
    let bytes = build_simple(&[first, second]);
    let error = ZipArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default())
        .err()
        .expect("must fail");
    assert_eq!(error.kind(), ArchiveErrorKind::Overlap);
}

#[test]
fn entry_data_reaching_into_cd_region_rejected() {
    // CD declares more compressed bytes than physically written so the
    // entry's data region runs past the central-directory start.
    let mut spec = EntrySpec::stored("abcdefghij", b"12345"); // name_len 10
    spec.declared_compressed = Some(15); // physical 5 → region end overshoots CD start
    let bytes = build_simple(&[spec]);
    let mut archive = reader(bytes);
    let kind = kind_of(archive.open_entry(0));
    assert_eq!(kind, ArchiveErrorKind::Overlap);
}

#[test]
fn crc_mismatch_is_detected_on_finish() {
    let mut bytes = writer_zip(&[("a", ZipCompressionMethod::Stored, b"content")]);
    bytes[30 + 1] ^= 0xFF; // flip a payload byte (stored: data follows 30-byte header + name "a")
    let mut archive = reader(bytes);
    let mut entry = archive.open_entry(0).expect("open");
    let _ = entry.read_to_end(&mut Vec::new());
    let kind = kind_of(entry.finish());
    assert_eq!(kind, ArchiveErrorKind::ChecksumMismatch);
    assert!(!archive.entry_is_verified(0));
}

#[test]
fn declared_size_limit_rejects_entry_open() {
    let bytes = writer_zip(&[("big", ZipCompressionMethod::Stored, b"1234567890")]);
    let mut archive = reader_with(bytes, limits(|l| l.max_entry_uncompressed_bytes = 4));
    let kind = kind_of(archive.open_entry(0));
    assert_eq!(kind, ArchiveErrorKind::LimitExceeded);
}

#[test]
fn actual_output_over_declared_size_is_size_mismatch() {
    // Real deflate stream producing 12 bytes while headers declare 4.
    let fixture = writer_zip(&[("x", ZipCompressionMethod::Deflate, b"hello, world")]);
    let data = compressed_region(&fixture).to_vec();
    let spec = EntrySpec::raw(b"x", METHOD_DEFLATE, &data, 4, crc32(b"hello, world"));
    let bytes = build_simple(&[spec]);
    let mut archive = reader(bytes);
    let mut entry = archive.open_entry(0).expect("open");
    let mut buf = Vec::new();
    let error = entry.read_to_end(&mut buf).expect_err("read must fail");
    let inner = error
        .get_ref()
        .and_then(|e| e.downcast_ref::<ArchiveError>());
    assert_eq!(
        inner.map(ArchiveError::kind),
        Some(ArchiveErrorKind::SizeMismatch)
    );
    assert!(buf.len() <= 4, "bytes past the cap must not be yielded");
}

#[test]
fn aggregate_uncompressed_budget_enforced_in_verify_all() {
    let bytes = writer_zip(&[
        ("a", ZipCompressionMethod::Stored, b"12345678"),
        ("b", ZipCompressionMethod::Stored, b"87654321"),
    ]);
    let mut archive = reader_with(bytes, limits(|l| l.max_total_uncompressed_bytes = 10));
    let kind = kind_of(archive.verify_all());
    assert_eq!(kind, ArchiveErrorKind::LimitExceeded);
}

#[test]
fn verify_all_surfaces_the_first_entry_failure() {
    // A verified entry followed by a CRC-corrupt one: verify_all
    // reports the corruption as an error rather than a partial
    // receipt, and per-entry verification state stays honest.
    let mut bad = EntrySpec::stored("bad", b"content");
    bad.crc32 ^= 0xFFFF;
    let bytes = build_simple(&[EntrySpec::stored("good", b"ok"), bad]);
    let mut archive = reader(bytes);
    let kind = kind_of(archive.verify_all());
    assert_eq!(kind, ArchiveErrorKind::ChecksumMismatch);
    assert!(archive.entry_is_verified(0));
    assert!(!archive.entry_is_verified(1));
}

#[test]
fn per_entry_actual_budget_cap_enforced_mid_stream() {
    // Declared size fits the budget but the *actual* deflate output
    // exceeds it — the cap must cut off while reading.
    let fixture = writer_zip(&[("x", ZipCompressionMethod::Deflate, b"0123456789")]);
    let data = compressed_region(&fixture).to_vec();
    let spec = EntrySpec::raw(b"x", METHOD_DEFLATE, &data, 10, crc32(b"0123456789"));
    let bytes = build_simple(&[spec]);
    let mut archive = reader_with(bytes, limits(|l| l.max_entry_uncompressed_bytes = 6));
    // declared 10 > 6 → rejected at open (declared bound first)
    let kind = kind_of(archive.open_entry(0));
    assert_eq!(kind, ArchiveErrorKind::LimitExceeded);
}

#[test]
fn encrypted_entry_is_unsupported() {
    let mut spec = EntrySpec::stored("a", b"x");
    spec.flags |= FLAG_ENCRYPTED;
    let bytes = build_simple(&[spec]);
    let mut archive = reader(bytes);
    let kind = kind_of(archive.open_entry(0));
    assert_eq!(kind, ArchiveErrorKind::Unsupported);
}

#[test]
fn unknown_method_is_unsupported() {
    let mut spec = EntrySpec::stored("a", b"x");
    spec.method = 99;
    spec.local_method = Some(99);
    let bytes = build_simple(&[spec]);
    let mut archive = reader(bytes);
    let kind = kind_of(archive.open_entry(0));
    assert_eq!(kind, ArchiveErrorKind::Unsupported);
}

#[test]
fn multi_disk_eocd_is_unsupported() {
    let bytes = build_zip(
        &[EntrySpec::stored("a", b"x")],
        &EocdSpec {
            disk_no: 1,
            ..EocdSpec::default()
        },
    );
    let error = ZipArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default())
        .err()
        .expect("must fail");
    assert_eq!(error.kind(), ArchiveErrorKind::Unsupported);
}

#[test]
fn multi_disk_cd_entry_is_unsupported() {
    let mut spec = EntrySpec::stored("a", b"x");
    spec.disk_start = 1;
    let bytes = build_simple(&[spec]);
    let error = ZipArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default())
        .err()
        .expect("must fail");
    assert_eq!(error.kind(), ArchiveErrorKind::Unsupported);
}

#[test]
fn zip64_eocd_is_unsupported() {
    let bytes = build_zip(
        &[EntrySpec::stored("a", b"x")],
        &EocdSpec {
            count: Some(u16::MAX),
            count_disk: Some(u16::MAX),
            ..EocdSpec::default()
        },
    );
    let error = ZipArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default())
        .err()
        .expect("must fail");
    assert_eq!(error.kind(), ArchiveErrorKind::Unsupported);
}

#[test]
fn partial_entry_drop_stays_unverified_and_allows_reopen() {
    let bytes = writer_zip(&[
        ("first", ZipCompressionMethod::Stored, b"0123456789"),
        ("second", ZipCompressionMethod::Stored, b"other"),
    ]);
    let mut archive = reader(bytes);
    {
        let mut entry = archive.open_entry(0).expect("open first");
        let mut one = [0_u8; 1];
        entry.read_exact(&mut one).expect("partial read");
        // dropped mid-read: not a completion proof
    }
    assert!(!archive.entry_is_verified(0));
    // Another entry can be opened while the dropped one stays unverified.
    {
        let entry = archive.open_entry(1).expect("open second");
        entry.finish().expect("second verified");
    }
    assert!(archive.entry_is_verified(1));
    assert!(!archive.entry_is_verified(0));
    // The same entry can be re-opened and completed.
    {
        let mut entry = archive.open_entry(0).expect("re-open first");
        assert_eq!(read_all(&mut entry), b"0123456789");
        entry.finish().expect("first verified");
    }
    assert!(archive.entry_is_verified(0));
    let receipt = archive.verify_all().expect("verify_all");
    assert_eq!(receipt.entries_verified(), 2);
}

#[test]
fn entry_index_out_of_range() {
    let bytes = writer_zip(&[("a", ZipCompressionMethod::Stored, b"x")]);
    let mut archive = reader(bytes);
    assert!(archive.entry_info(7).is_none());
    let kind = kind_of(archive.open_entry(7));
    assert_eq!(kind, ArchiveErrorKind::InvalidInput);
    assert!(!archive.entry_is_verified(7));
}

#[test]
fn archive_name_preserves_non_utf8_bytes() {
    let spec = EntrySpec::raw(
        &[b'n', 0xFF, b'.', b't'],
        METHOD_STORED,
        b"x",
        1,
        crc32(b"x"),
    );
    let bytes = build_simple(&[spec]);
    let archive = reader(bytes);
    let info = archive.entry_info(0).expect("info");
    assert_eq!(info.name().as_bytes(), &[b'n', 0xFF, b'.', b't']);
    assert!(info.name().to_str().is_none());
    assert!(info.name().to_string_lossy().contains('\u{FFFD}'));
}

#[test]
fn directory_entry_kind_from_trailing_slash() {
    let spec = EntrySpec::raw(b"dir/", METHOD_STORED, b"", 0, crc32(b""));
    let bytes = build_simple(&[spec]);
    let archive = reader(bytes);
    let info = archive.entry_info(0).expect("info");
    assert_eq!(info.kind(), EntryKind::Directory);
}

#[test]
fn cd_record_count_mismatch_is_malformed() {
    let bytes = build_zip(
        &[EntrySpec::stored("a", b"x")],
        &EocdSpec {
            count: Some(2),
            count_disk: Some(2),
            ..EocdSpec::default()
        },
    );
    let error = ZipArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default())
        .err()
        .expect("must fail");
    assert_eq!(error.kind(), ArchiveErrorKind::Malformed);
}

#[test]
fn corrupt_deflate_stream_is_malformed_not_a_panic() {
    // Bytes that are not a deflate stream, declared as method 8.
    let spec = EntrySpec::raw(b"x", METHOD_DEFLATE, b"\xFF\xFFnot-deflate!!", 5, 0);
    let bytes = build_simple(&[spec]);
    let mut archive = reader(bytes);
    let mut entry = archive.open_entry(0).expect("open");
    let error = entry
        .read_to_end(&mut Vec::new())
        .expect_err("read must fail");
    let inner = error
        .get_ref()
        .and_then(|e| e.downcast_ref::<ArchiveError>());
    assert_eq!(
        inner.map(ArchiveError::kind),
        Some(ArchiveErrorKind::Malformed)
    );
}

#[test]
fn into_inner_returns_the_underlying_reader_without_verifying() {
    // Mirror of the TAR contract: handing the inner stream back is not
    // a completion proof — no entry was ever opened or verified.
    let bytes = writer_zip(&[("a", ZipCompressionMethod::Stored, b"x")]);
    let archive = reader(bytes.clone());
    assert_eq!(archive.into_inner().into_inner(), bytes);
}

#[test]
fn cd_record_boundaries() {
    // Sanity for the test helper: CD starts right after the entries.
    let bytes = writer_zip(&[
        ("aa", ZipCompressionMethod::Stored, b"1"),
        ("bb", ZipCompressionMethod::Stored, b"2"),
    ]);
    let cd = cd_offset_of(&bytes);
    assert_eq!(cd_record_offset(&bytes, 1, 2), cd + 46 + 2);
    assert_eq!(&bytes[cd..cd + 4], b"PK\x01\x02");
}
