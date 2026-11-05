//! TAR reader verification: sequential parsing, USTAR/GNU/PAX
//! effective fields, the `Pending`/`Failed` state machine, budgets,
//! declared-size and padding checks, and end-of-archive verification.

mod common;

use std::io::{Cursor, Read};

use common::tar::{
    BLOCK, HeaderSpec, entry_with, ext_entry, file_entry, pax_record, tar, writer_tar,
};
use ruviform_io_archive::tar::TarArchiveReader;
use ruviform_io_archive::{
    ArchiveError, ArchiveErrorKind, ArchiveReadLimits, EntryChecksum, EntryDisposition, EntryKind,
};

fn limits(configure: impl Fn(&mut ArchiveReadLimits)) -> ArchiveReadLimits {
    let mut limits = ArchiveReadLimits::default();
    configure(&mut limits);
    limits
}

fn reader(bytes: Vec<u8>) -> TarArchiveReader<Cursor<Vec<u8>>> {
    reader_with(bytes, ArchiveReadLimits::default())
}

fn reader_with(bytes: Vec<u8>, limits: ArchiveReadLimits) -> TarArchiveReader<Cursor<Vec<u8>>> {
    TarArchiveReader::new(Cursor::new(bytes), limits)
}

fn kind_of(result: Result<impl Sized, ArchiveError>) -> ArchiveErrorKind {
    match result {
        Ok(_) => panic!("expected error"),
        Err(error) => error.kind(),
    }
}

fn io_kind(error: &std::io::Error) -> Option<ArchiveErrorKind> {
    error
        .get_ref()
        .and_then(|e| e.downcast_ref::<ArchiveError>())
        .map(ArchiveError::kind)
}

fn read_all(entry: &mut impl Read) -> Vec<u8> {
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).expect("read payload");
    buf
}

// --- round trip ----------------------------------------------------

#[test]
fn round_trip_reads_all_entries_and_receipts() {
    let bytes = writer_tar(&[
        ("a.txt", b"alpha"),
        ("dir/b.bin", b"beta-beta"),
        ("empty", b""),
    ]);
    let mut archive = reader(bytes);
    assert_eq!(archive.entry_count(), 0);

    {
        let mut entry = archive.next_entry().expect("entry 0").expect("some");
        assert_eq!(entry.info().name().as_bytes(), b"a.txt");
        assert_eq!(entry.info().kind(), EntryKind::File);
        assert_eq!(entry.info().index(), 0);
        assert_eq!(entry.info().uncompressed_size(), Some(5));
        assert_eq!(entry.info().link_name(), None);
        assert_eq!(read_all(&mut entry), b"alpha");
        let receipt = entry.finish().expect("finish 0");
        assert_eq!(receipt.checksum(), EntryChecksum::NotPresent);
        assert_eq!(receipt.disposition(), EntryDisposition::ReadToEnd);
        assert_eq!(receipt.uncompressed_bytes(), 5);
    }
    {
        let mut entry = archive.next_entry().expect("entry 1").expect("some");
        assert_eq!(entry.info().name().as_bytes(), b"dir/b.bin");
        assert_eq!(entry.info().index(), 1);
        assert_eq!(read_all(&mut entry), b"beta-beta");
        entry.finish().expect("finish 1");
    }
    {
        let entry = archive.next_entry().expect("entry 2").expect("some");
        assert_eq!(entry.info().uncompressed_size(), Some(0));
        entry.finish().expect("zero-size finish");
    }
    assert_eq!(archive.entry_count(), 3);
    assert!(archive.next_entry().expect("end").is_none());
    let (_inner, receipt) = archive.finish().expect("archive finish");
    assert_eq!(receipt.entries_total(), 3);
    assert_eq!(receipt.entries_verified(), 3);
    assert_eq!(receipt.entries_unverified(), 0);
    assert_eq!(receipt.payload_bytes(), 14);
}

#[test]
fn empty_archive_has_no_entries() {
    let mut archive = reader(tar(&[]));
    assert!(archive.next_entry().expect("end").is_none());
    let (_inner, receipt) = archive.finish().expect("finish");
    assert_eq!(receipt.entries_total(), 0);
}

#[test]
fn directory_marker_is_directory_kind() {
    let bytes = tar(&[entry_with(&HeaderSpec::new(b'5', b"dir/", 0), b"")]);
    let mut archive = reader(bytes);
    let entry = archive.next_entry().expect("entry").expect("some");
    assert_eq!(entry.info().kind(), EntryKind::Directory);
    assert_eq!(entry.info().uncompressed_size(), Some(0));
}

// --- name / link / size formats ------------------------------------

#[test]
fn ustar_prefix_joins_effective_name() {
    let mut spec = HeaderSpec::file("file.txt", 3);
    spec.prefix = b"dir/sub";
    let bytes = tar(&[entry_with(&spec, b"abc")]);
    let mut archive = reader(bytes);
    let entry = archive.next_entry().expect("entry").expect("some");
    assert_eq!(entry.info().name().as_bytes(), b"dir/sub/file.txt");
}

#[test]
fn gnu_longname_sets_effective_name() {
    let long_name = "a".repeat(120) + "/with/more/path/file.dat";
    let bytes = tar(&[
        ext_entry(b'L', "././@LongLink", long_name.as_bytes()),
        file_entry("short", b"data"),
    ]);
    let mut archive = reader(bytes);
    let entry = archive.next_entry().expect("entry").expect("some");
    assert_eq!(entry.info().name().as_bytes(), long_name.as_bytes());
}

#[test]
fn gnu_longlink_sets_symlink_target() {
    let long_target = "t".repeat(150) + "/deep/target";
    let mut spec = HeaderSpec::new(b'2', b"short", 0);
    spec.linkname = b"";
    let bytes = tar(&[
        ext_entry(b'K', "././@LongLink", long_target.as_bytes()),
        entry_with(&spec, b""),
    ]);
    let mut archive = reader(bytes);
    let entry = archive.next_entry().expect("entry").expect("some");
    assert_eq!(entry.info().kind(), EntryKind::SymbolicLink);
    assert_eq!(
        entry.info().link_name().expect("link").as_bytes(),
        long_target.as_bytes()
    );
}

#[test]
fn pax_path_linkpath_and_size_override_header_fields() {
    let records = [
        pax_record(b"path", b"pax/renamed.txt"),
        pax_record(b"linkpath", b"pax/link-target"),
        pax_record(b"size", b"700"),
    ]
    .concat();
    let mut spec = HeaderSpec::new(b'2', b"hdr-name", 0);
    spec.linkname = b"hdr-target";
    let bytes = tar(&[
        ext_entry(b'x', "PaxHeader", &records),
        entry_with(&spec, b""),
    ]);
    let mut archive = reader(bytes);
    let entry = archive.next_entry().expect("entry").expect("some");
    assert_eq!(entry.info().name().as_bytes(), b"pax/renamed.txt");
    assert_eq!(
        entry.info().link_name().expect("link").as_bytes(),
        b"pax/link-target"
    );
    // PAX size applies even when the header field says something else.
    assert_eq!(entry.info().uncompressed_size(), Some(700));
}

#[test]
fn pax_global_record_is_validated_but_never_applied() {
    let global = pax_record(b"path", b"global/evil-name");
    let bytes = tar(&[
        ext_entry(b'g', "PaxHeader", &global),
        file_entry("real.txt", b"x"),
    ]);
    let mut archive = reader(bytes);
    let entry = archive.next_entry().expect("entry").expect("some");
    // A global `path` record must not rename the following entry.
    assert_eq!(entry.info().name().as_bytes(), b"real.txt");
}

#[test]
fn gnu_magic_header_is_accepted() {
    let mut spec = HeaderSpec::file("gnu.txt", 4);
    spec.gnu = true;
    let bytes = tar(&[entry_with(&spec, b"gnu!")]);
    let mut archive = reader(bytes);
    let mut entry = archive.next_entry().expect("entry").expect("some");
    assert_eq!(read_all(&mut entry), b"gnu!");
    entry.finish().expect("finish");
}

#[test]
fn base256_size_field_is_parsed() {
    let mut spec = HeaderSpec::file("big", 0x1234);
    spec.base256_size = true;
    let payload = vec![b'x'; 0x1234];
    let bytes = tar(&[entry_with(&spec, &payload)]);
    let mut archive = reader(bytes);
    let mut entry = archive.next_entry().expect("entry").expect("some");
    assert_eq!(entry.info().uncompressed_size(), Some(0x1234));
    assert_eq!(read_all(&mut entry).len(), 0x1234);
    entry.finish().expect("finish");
}

#[test]
fn symlink_and_hardlink_expose_targets() {
    let mut sym = HeaderSpec::new(b'2', b"sym", 0);
    sym.linkname = b"rel/target";
    let mut hard = HeaderSpec::new(b'1', b"hard", 0);
    hard.linkname = b"other/file";
    let bytes = tar(&[entry_with(&sym, b""), entry_with(&hard, b"")]);
    let mut archive = reader(bytes);
    let first = archive.next_entry().expect("e0").expect("some");
    assert_eq!(first.info().kind(), EntryKind::SymbolicLink);
    assert_eq!(
        first.info().link_name().expect("link").as_bytes(),
        b"rel/target"
    );
    drop(first);
    archive.discard_pending().expect("discard");
    let second = archive.next_entry().expect("e1").expect("some");
    assert_eq!(second.info().kind(), EntryKind::HardLink);
    assert_eq!(
        second.info().link_name().expect("link").as_bytes(),
        b"other/file"
    );
}

#[test]
fn non_utf8_name_bytes_are_preserved() {
    let spec = HeaderSpec::new(b'0', &[b'n', 0xFF, b'x'], 1);
    let bytes = tar(&[entry_with(&spec, b"z")]);
    let mut archive = reader(bytes);
    let entry = archive.next_entry().expect("entry").expect("some");
    assert_eq!(entry.info().name().as_bytes(), &[b'n', 0xFF, b'x']);
    assert!(entry.info().name().to_str().is_none());
}

// --- structural failures --------------------------------------------

#[test]
fn bad_header_checksum_is_checksum_mismatch() {
    let mut spec = HeaderSpec::file("a", 1);
    spec.corrupt_checksum = true;
    let bytes = tar(&[entry_with(&spec, b"x")]);
    let mut archive = reader(bytes);
    assert_eq!(
        kind_of(archive.next_entry()),
        ArchiveErrorKind::ChecksumMismatch
    );
}

#[test]
fn non_octal_size_field_is_malformed() {
    let mut block = HeaderSpec::file("a", 1).block();
    block[124..131].copy_from_slice(b"12a4567");
    let sum = common::tar::checksum(&block);
    block[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&block);
    bytes.extend_from_slice(b"x");
    bytes.resize(bytes.len() + BLOCK * 2, 0);
    let mut archive = reader(bytes);
    assert_eq!(kind_of(archive.next_entry()), ArchiveErrorKind::Malformed);
}

#[test]
fn bad_magic_is_malformed() {
    let mut block = HeaderSpec::file("a", 1).block();
    block[257..263].copy_from_slice(b"nosuch");
    let sum = common::tar::checksum(&block);
    block[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&block);
    bytes.resize(bytes.len() + BLOCK, 0);
    let mut archive = reader(bytes);
    assert_eq!(kind_of(archive.next_entry()), ArchiveErrorKind::Malformed);
}

#[test]
fn truncated_header_mid_block_is_truncated() {
    let block = HeaderSpec::file("a", 1).block();
    let bytes = block[..300].to_vec();
    let mut archive = reader(bytes);
    assert_eq!(kind_of(archive.next_entry()), ArchiveErrorKind::Truncated);
}

#[test]
fn missing_end_marker_is_truncated() {
    // A bare EOF right after the payload — no zero block at all.
    let bytes = file_entry("a", b"x");
    let mut archive = reader(bytes);
    let mut entry = archive.next_entry().expect("entry").expect("some");
    entry.read_exact(&mut [0_u8; 1]).expect("payload");
    entry.finish().expect("entry finish");
    assert_eq!(kind_of(archive.next_entry()), ArchiveErrorKind::Truncated);
}

#[test]
fn non_zero_trailing_data_after_marker_is_malformed() {
    let mut bytes = tar(&[file_entry("a", b"x")]);
    bytes.push(b'!');
    let mut archive = reader(bytes);
    let mut entry = archive.next_entry().expect("entry").expect("some");
    read_all(&mut entry);
    entry.finish().expect("entry finish");
    assert_eq!(kind_of(archive.finish()), ArchiveErrorKind::Malformed);
}

#[test]
fn single_zero_block_then_eof_is_accepted() {
    // One zero block is a legal (if old-fashioned) end marker.
    let mut bytes = file_entry("a", b"x");
    bytes.resize(bytes.len() + BLOCK, 0);
    let mut archive = reader(bytes);
    let mut entry = archive.next_entry().expect("entry").expect("some");
    read_all(&mut entry);
    entry.finish().expect("entry finish");
    assert!(archive.next_entry().expect("end").is_none());
    archive.finish().expect("lone zero block accepted");
}

// --- unsupported / limits -------------------------------------------

#[test]
fn special_file_typeflags_are_unsupported() {
    for flag in *b"346" {
        let bytes = tar(&[entry_with(&HeaderSpec::new(flag, b"dev", 0), b"")]);
        let mut archive = reader(bytes);
        assert_eq!(
            kind_of(archive.next_entry()),
            ArchiveErrorKind::Unsupported,
            "typeflag {flag}"
        );
    }
}

#[test]
fn sparse_and_unknown_typeflags_are_unsupported() {
    for flag in *b"SVZ" {
        let bytes = tar(&[entry_with(&HeaderSpec::new(flag, b"x", 0), b"")]);
        let mut archive = reader(bytes);
        assert_eq!(
            kind_of(archive.next_entry()),
            ArchiveErrorKind::Unsupported,
            "typeflag {flag}"
        );
    }
}

#[test]
fn declared_size_over_entry_limit_is_rejected() {
    let bytes = writer_tar(&[("big", b"1234567890")]);
    let mut archive = reader_with(bytes, limits(|l| l.max_entry_uncompressed_bytes = 4));
    assert_eq!(
        kind_of(archive.next_entry()),
        ArchiveErrorKind::LimitExceeded
    );
}

#[test]
fn entry_count_limit_is_enforced() {
    let bytes = writer_tar(&[("a", b"1"), ("b", b"2")]);
    let mut archive = reader_with(bytes, limits(|l| l.max_entries = 1));
    let first = archive.next_entry().expect("e0").expect("some");
    first.discard_remaining().expect("discard");
    assert_eq!(
        kind_of(archive.next_entry()),
        ArchiveErrorKind::LimitExceeded
    );
}

#[test]
fn path_bytes_limit_rejects_effective_name() {
    let long = "x".repeat(300) + "/file.txt";
    let records = pax_record(b"path", long.as_bytes());
    let bytes = tar(&[
        ext_entry(b'x', "PaxHeader", &records),
        file_entry("short", b"d"),
    ]);
    let mut archive = reader_with(bytes, limits(|l| l.max_path_bytes = 64));
    assert_eq!(
        kind_of(archive.next_entry()),
        ArchiveErrorKind::LimitExceeded
    );
}

#[test]
fn metadata_bytes_limit_covers_extensions() {
    // The 'x' payload (~25 bytes) plus two header blocks exceeds the cap.
    let records = pax_record(b"path", b"some/path.txt");
    let bytes = tar(&[
        ext_entry(b'x', "PaxHeader", &records),
        file_entry("x", b"d"),
    ]);
    let mut archive = reader_with(bytes, limits(|l| l.max_metadata_bytes = 1030));
    assert_eq!(
        kind_of(archive.next_entry()),
        ArchiveErrorKind::LimitExceeded
    );
}

#[test]
fn malformed_pax_payload_is_malformed() {
    let bytes = tar(&[
        ext_entry(b'x', "PaxHeader", b"not-a-record"),
        file_entry("x", b"d"),
    ]);
    let mut archive = reader(bytes);
    assert_eq!(kind_of(archive.next_entry()), ArchiveErrorKind::Malformed);
}

// --- payload bounds --------------------------------------------------

#[test]
fn read_never_yields_beyond_declared_size() {
    // Header declares 4 but a full block of payload bytes follows.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&HeaderSpec::file("a", 4).block());
    bytes.extend_from_slice(&[b'y'; BLOCK]);
    bytes.resize(bytes.len() + BLOCK * 2, 0);
    let mut archive = reader(bytes);
    let mut entry = archive.next_entry().expect("entry").expect("some");
    let data = read_all(&mut entry);
    assert_eq!(data.len(), 4, "declared bound must cap the payload");
}

#[test]
fn short_payload_is_truncated_and_fails_reader() {
    // Declares 100 bytes but the stream ends after 10.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&HeaderSpec::file("a", 100).block());
    bytes.extend_from_slice(&[b'x'; 10]);
    let mut archive = reader(bytes);
    let mut entry = archive.next_entry().expect("entry").expect("some");
    let mut buf = Vec::new();
    // The available bytes are yielded first; EOF before the declared
    // end is reported on the following call.
    let error = entry.read_to_end(&mut buf).expect_err("read must fail");
    assert_eq!(io_kind(&error), Some(ArchiveErrorKind::Truncated));
    assert!(buf.len() <= 10);
    drop(entry);
    // The reader is Failed: recovery cannot resync the stream.
    assert_eq!(
        kind_of(archive.next_entry()),
        ArchiveErrorKind::InvalidState
    );
    assert_eq!(
        kind_of(archive.discard_pending()),
        ArchiveErrorKind::InvalidState
    );
}

#[test]
fn total_budget_cut_mid_read_reports_limit() {
    // Declared fits the per-entry cap; the *total* budget is smaller.
    let bytes = writer_tar(&[("big", b"0123456789")]);
    let mut archive = reader_with(bytes, limits(|l| l.max_total_uncompressed_bytes = 6));
    let mut entry = archive.next_entry().expect("entry").expect("some");
    let mut buf = Vec::new();
    let error = entry.read_to_end(&mut buf).expect_err("must hit the cap");
    assert_eq!(io_kind(&error), Some(ArchiveErrorKind::LimitExceeded));
    assert!(buf.len() <= 6, "bytes past the budget must not be yielded");
}

#[test]
fn nonzero_padding_is_malformed_but_position_recovers() {
    // Corrupt one padding byte after a 5-byte payload: the error is
    // reported but the stream is already at the next header.
    let mut chunk = entry_with(&HeaderSpec::file("a", 5), b"12345");
    let pad_at = BLOCK + 5;
    chunk[pad_at] = 0xAA;
    let bytes = tar(&[chunk, file_entry("b", b"ok")]);
    let mut archive = reader(bytes);
    let mut entry = archive.next_entry().expect("e0").expect("some");
    read_all(&mut entry);
    assert_eq!(kind_of(entry.finish()), ArchiveErrorKind::Malformed);
    // Recovery: the next header parses normally.
    let mut second = archive.next_entry().expect("e1").expect("some");
    read_all(&mut second);
    second.finish().expect("second finish");
    let (_inner, receipt) = archive.finish().expect("archive finish");
    assert_eq!(receipt.entries_verified(), 1);
    assert_eq!(receipt.entries_unverified(), 1);
}

// --- pending state machine -------------------------------------------

#[test]
fn dropped_entry_creates_pending_until_discard() {
    let bytes = writer_tar(&[("first", b"0123456789"), ("second", b"rest")]);
    let mut archive = reader(bytes);
    {
        let mut entry = archive.next_entry().expect("e0").expect("some");
        entry.read_exact(&mut [0_u8; 2]).expect("partial");
        // dropped mid-read → Pending
    }
    assert!(archive.has_pending_entry());
    // next_entry and finish are both rejected while Pending —
    // discard_pending is the only recovery path.
    assert_eq!(
        kind_of(archive.next_entry()),
        ArchiveErrorKind::InvalidState
    );
    archive.discard_pending().expect("discard_pending");
    assert!(!archive.has_pending_entry());
    let mut second = archive.next_entry().expect("e1").expect("some");
    assert_eq!(read_all(&mut second), b"rest");
    second.finish().expect("finish 1");
    let (_inner, receipt) = archive.finish().expect("archive finish");
    assert_eq!(receipt.entries_unverified(), 1);
    assert_eq!(receipt.entries_verified(), 1);
    // 2 read bytes + 8 discarded bytes + 4 second-entry bytes.
    assert_eq!(receipt.payload_bytes(), 14);
}

#[test]
fn discard_pending_without_pending_is_invalid() {
    let bytes = writer_tar(&[("a", b"x")]);
    let mut archive = reader(bytes);
    assert_eq!(
        kind_of(archive.discard_pending()),
        ArchiveErrorKind::InvalidState
    );
}

#[test]
fn discard_pending_io_failure_latches_failed() {
    // Declared payload outlives the physical stream: the drain hits EOF.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&HeaderSpec::file("a", 5000).block());
    bytes.extend_from_slice(&[b'x'; 100]);
    let mut archive = reader(bytes);
    {
        let _entry = archive.next_entry().expect("e0").expect("some");
        // dropped without reading → Pending{skip:5000}
    }
    assert_eq!(
        kind_of(archive.discard_pending()),
        ArchiveErrorKind::Truncated
    );
    assert_eq!(
        kind_of(archive.next_entry()),
        ArchiveErrorKind::InvalidState
    );
}

#[test]
fn failed_reader_rejects_every_operation() {
    let mut spec = HeaderSpec::file("a", 1);
    spec.corrupt_checksum = true;
    let bytes = tar(&[entry_with(&spec, b"x")]);
    let mut archive = reader(bytes);
    assert_eq!(
        kind_of(archive.next_entry()),
        ArchiveErrorKind::ChecksumMismatch
    );
    assert_eq!(
        kind_of(archive.next_entry()),
        ArchiveErrorKind::InvalidState
    );
    assert_eq!(
        kind_of(archive.discard_pending()),
        ArchiveErrorKind::InvalidState
    );
    assert_eq!(kind_of(archive.finish()), ArchiveErrorKind::InvalidState);
}

#[test]
fn discard_remaining_drains_and_reports_discarded() {
    let bytes = writer_tar(&[("skipme", b"0123456789"), ("kept", b"ab")]);
    let mut archive = reader(bytes);
    {
        let mut entry = archive.next_entry().expect("e0").expect("some");
        entry.read_exact(&mut [0_u8; 4]).expect("partial");
        let receipt = entry.discard_remaining().expect("discard_remaining");
        assert_eq!(receipt.disposition(), EntryDisposition::Discarded);
        assert_eq!(receipt.checksum(), EntryChecksum::NotPresent);
        assert_eq!(receipt.uncompressed_bytes(), 10);
    }
    assert!(!archive.has_pending_entry());
    let mut second = archive.next_entry().expect("e1").expect("some");
    read_all(&mut second);
    second.finish().expect("finish");
    let (_inner, receipt) = archive.finish().expect("archive finish");
    assert_eq!(receipt.entries_unverified(), 1);
    assert_eq!(receipt.entries_verified(), 1);
    assert_eq!(receipt.payload_bytes(), 12);
}

#[test]
fn finish_drains_unread_entries_as_unverified() {
    let bytes = writer_tar(&[("a", b"123"), ("b", b"4567"), ("c", b"")]);
    let archive = reader(bytes);
    let (_inner, receipt) = archive.finish().expect("finish without reads");
    assert_eq!(receipt.entries_total(), 3);
    assert_eq!(receipt.entries_verified(), 0);
    assert_eq!(receipt.entries_unverified(), 3);
    assert_eq!(receipt.payload_bytes(), 7);
}

#[test]
fn finish_while_pending_is_rejected() {
    let bytes = writer_tar(&[("a", b"0123456789")]);
    let mut archive = reader(bytes);
    {
        let entry = archive.next_entry().expect("e0").expect("some");
        drop(entry); // Pending
    }
    // finish takes ownership — the rejection is terminal for this
    // reader regardless of recovery potential.
    assert_eq!(kind_of(archive.finish()), ArchiveErrorKind::InvalidState);
}

#[test]
fn finish_early_on_entry_is_invalid_state_and_leaves_pending() {
    let bytes = writer_tar(&[("a", b"0123456789")]);
    let mut archive = reader(bytes);
    let mut entry = archive.next_entry().expect("e0").expect("some");
    entry.read_exact(&mut [0_u8; 4]).expect("partial");
    assert_eq!(kind_of(entry.finish()), ArchiveErrorKind::InvalidState);
    // The dropped half-consumed entry is now Pending — recoverable.
    assert!(archive.has_pending_entry());
    archive.discard_pending().expect("recover");
    archive.finish().expect("archive finish");
}

// --- stream ownership / two-layer composition ------------------------

#[test]
fn into_inner_returns_the_stream_without_verifying() {
    let bytes = writer_tar(&[("a", b"x")]);
    let archive = reader(bytes);
    let cursor = archive.into_inner();
    assert_eq!(cursor.position(), 0);
}

#[test]
fn garbage_bytes_are_classified_not_panics() {
    // Fuzz-lite: deterministic corrupt inputs must produce classified
    // errors — never a panic or an infinite loop.
    for input in [
        vec![0xAA; BLOCK],
        vec![0u8; 10],
        b"ustar garbage not a header at all".repeat(40),
        vec![0xFF; 2000],
    ] {
        let mut archive = reader(input);
        // At most a handful of steps — either an error or an end.
        for _ in 0..8 {
            match archive.next_entry() {
                Ok(Some(entry)) => drop(entry),
                Ok(None) | Err(_) => break,
            }
        }
        let _ = archive.discard_pending();
        let _ = archive.finish();
    }
}

#[test]
fn tar_gz_layers_finish_independently() {
    use ruviform_io_compression::gzip::{DecodeOptions, Decoder, EncodeOptions, Encoder};
    use std::io::{BufReader, Write};

    let tar_bytes = writer_tar(&[("inner.txt", b"payload")]);
    let mut compressed = Vec::new();
    {
        let mut encoder =
            Encoder::new(&mut compressed, EncodeOptions::default()).expect("gzip encoder");
        encoder.write_all(&tar_bytes).expect("write tar");
        encoder.finish().expect("gzip finish");
    }

    let mut source = BufReader::new(Cursor::new(compressed));
    let mut decoder = Decoder::new(&mut source, DecodeOptions::default());
    let mut archive = TarArchiveReader::new(&mut decoder, ArchiveReadLimits::default());
    {
        let mut entry = archive.next_entry().expect("e0").expect("some");
        assert_eq!(read_all(&mut entry), b"payload");
        entry.finish().expect("entry finish");
    }
    // TAR-level verification completes first…
    let (decoder_ref, receipt) = archive.finish().expect("tar finish");
    assert_eq!(receipt.entries_verified(), 1);
    // …then the gzip layer verifies its own trailer separately.
    decoder_ref.finish().expect("gzip trailer verified");
}

#[test]
fn tar_gz_bad_trailer_surfaces_through_tar_finish() {
    use ruviform_io_compression::gzip::{DecodeOptions, Decoder, EncodeOptions, Encoder};
    use std::io::{BufReader, Write};

    let tar_bytes = writer_tar(&[("inner.txt", b"payload")]);
    let mut compressed = Vec::new();
    {
        let mut encoder =
            Encoder::new(&mut compressed, EncodeOptions::default()).expect("gzip encoder");
        encoder.write_all(&tar_bytes).expect("write tar");
        encoder.finish().expect("gzip finish");
    }
    // Corrupt the final trailer byte (CRC32/ISIZE).
    let last = compressed.len() - 1;
    compressed[last] ^= 0xFF;

    let mut source = BufReader::new(Cursor::new(compressed));
    let mut decoder = Decoder::new(&mut source, DecodeOptions::default());
    let archive = TarArchiveReader::new(&mut decoder, ArchiveReadLimits::default());
    // The TAR layer's tail drain hits the decoder's trailer failure —
    // layers surface their own errors independently.
    assert!(archive.finish().is_err());
}
