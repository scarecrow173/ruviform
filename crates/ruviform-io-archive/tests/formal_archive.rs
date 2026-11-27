//! Issue #806 slice 2 (D-layer formal verification) — TAR/ZIP archive
//! reader property suite plus `#[cfg(kani)]` bounded proof harnesses.
//!
//! Target contracts (product source of truth:
//! `crates/ruviform-io-archive/src/{tar,zip}/read.rs`,
//! `src/limits.rs`, `src/error.rs`):
//!
//! - `TarArchiveReader<R: Read>`: `next_entry` / `discard_pending` /
//!   `finish` never panic on arbitrary bytes; `Err` kinds stay in the
//!   documented set {`InvalidState`, `Truncated`, `Malformed`,
//!   `ChecksumMismatch`, `LimitExceeded`, `Unsupported`, `Io`}. A
//!   dropped mid-read entry leaves the archive `Pending` until
//!   `discard_pending` drains it; a structural failure is `Failed`.
//! - `ZipArchiveReader<R: Read + Seek>`: `new` / `open_entry` /
//!   `verify_all` never panic; `Err` kinds stay in the TAR set plus
//!   {`InvalidInput`, `Inconsistent`, `SizeMismatch`, `Overlap`}.
//! - `ArchiveReadLimits` gates run **before** allocation (entry count,
//!   metadata/path/declared-size budgets) and cut measured output
//!   mid-read (`max_total_uncompressed_bytes`).
//! - `ArchiveReadReceipt` counts (`entries_total` / `entries_verified`
//!   / `entries_unverified` / `payload_bytes`) agree with what the
//!   drive actually consumed.
//!
//! The ZIP index gate and declared entry-size gate proofs live beside their
//! production decision cores in `src/zip/read.rs`; the TAR declared-size and
//! typeflag classification proofs live beside their production decision cores
//! in `src/tar/read.rs`. Bounds and assumptions are documented in
//! `docs/specs/formal-verification.md`.

mod common;

use std::io::{Cursor, Read};

use common::tar::{
    BLOCK, HeaderSpec, entry_with, ext_entry, file_entry, pax_record, tar, writer_tar,
};
use common::{
    EntrySpec, EocdSpec, FLAG_DATA_DESCRIPTOR, FLAG_ENCRYPTED, FLAG_UTF8, METHOD_DEFLATE,
    METHOD_STORED, build_simple, build_zip, cd_offset_of, compressed_region, crc32, writer_zip,
};
use proptest::prelude::*;
use ruviform_io_archive::tar::TarArchiveReader;
use ruviform_io_archive::zip::{ZipArchiveReader, ZipCompressionMethod};
use ruviform_io_archive::{
    ArchiveError, ArchiveErrorKind, ArchiveName, ArchiveReadLimits, EntryChecksum,
    EntryDisposition, EntryKind,
};

/// Modest budgets for the arbitrary-input drives: large enough that
/// limit gates do not dominate the outcome classification, small
/// enough that crafted expansion (e.g. deflate output) is bounded.
fn drive_limits() -> ArchiveReadLimits {
    let mut limits = ArchiveReadLimits::default();
    limits.max_entries = 64;
    limits.max_metadata_bytes = 1 << 20;
    limits.max_path_bytes = 4096;
    limits.max_path_depth = 32;
    limits.max_entry_uncompressed_bytes = 64 << 10;
    limits.max_total_uncompressed_bytes = 256 << 10;
    limits
}

/// The documented `ArchiveErrorKind` set for the TAR read path
/// (`next_entry` / entry ops / `discard_pending` / `finish`).
/// `InvalidInput`/`Inconsistent`/`SizeMismatch`/`Overlap` have no
/// TAR-read path; a future enum variant also fails here — extend the
/// property when the documented set grows.
fn tar_kind_is_stable(kind: ArchiveErrorKind) -> bool {
    matches!(
        kind,
        ArchiveErrorKind::InvalidState
            | ArchiveErrorKind::Truncated
            | ArchiveErrorKind::Malformed
            | ArchiveErrorKind::ChecksumMismatch
            | ArchiveErrorKind::LimitExceeded
            | ArchiveErrorKind::Unsupported
            | ArchiveErrorKind::Io
    )
}

/// The documented `ArchiveErrorKind` set for the ZIP read path — the
/// TAR set plus the index/local-header consistency kinds.
fn zip_kind_is_stable(kind: ArchiveErrorKind) -> bool {
    matches!(
        kind,
        ArchiveErrorKind::Io
            | ArchiveErrorKind::InvalidInput
            | ArchiveErrorKind::InvalidState
            | ArchiveErrorKind::Truncated
            | ArchiveErrorKind::Malformed
            | ArchiveErrorKind::Inconsistent
            | ArchiveErrorKind::ChecksumMismatch
            | ArchiveErrorKind::SizeMismatch
            | ArchiveErrorKind::LimitExceeded
            | ArchiveErrorKind::Overlap
            | ArchiveErrorKind::Unsupported
    )
}

/// Extracts the `ArchiveErrorKind` wrapped inside an entry-reader
/// `io::Error` (`From<ArchiveError>` keeps it as the inner error).
fn io_kind(error: &std::io::Error) -> Option<ArchiveErrorKind> {
    error
        .get_ref()
        .and_then(|e| e.downcast_ref::<ArchiveError>())
        .map(ArchiveError::kind)
}

// --- generators ------------------------------------------------------

fn arb_bytes() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..=2048)
}

/// Non-NUL bytes for fields that truncate at NUL (header name/linkname,
/// GNU longname/longlink payloads).
fn arb_field_bytes(range: std::ops::RangeInclusive<usize>) -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(1_u8..=255, range)
}

/// Printable-ish names for the writer APIs (`&str` input, no trailing
/// `/`, at least one alphanumeric so `EntryKind::File` holds).
fn arb_writer_name() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_./-]{0,62}[a-z0-9_-]"
}

fn arb_payload() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..=600)
}

fn arb_tar_entries() -> impl Strategy<Value = Vec<(String, Vec<u8>)>> {
    prop::collection::vec((arb_writer_name(), arb_payload()), 0..=4)
}

fn writer_tar_bytes(entries: &[(String, Vec<u8>)]) -> Vec<u8> {
    let refs: Vec<(&str, &[u8])> = entries
        .iter()
        .map(|(n, d)| (n.as_str(), d.as_slice()))
        .collect();
    writer_tar(&refs)
}

fn writer_zip_bytes(entries: &[(String, bool, Vec<u8>)]) -> Vec<u8> {
    let refs: Vec<(&str, ZipCompressionMethod, &[u8])> = entries
        .iter()
        .map(|(n, deflate, d)| {
            (
                n.as_str(),
                if *deflate {
                    ZipCompressionMethod::Deflate
                } else {
                    ZipCompressionMethod::Stored
                },
                d.as_slice(),
            )
        })
        .collect();
    writer_zip(&refs)
}

/// Byte-level mutations applied to a valid archive — flips, truncation,
/// insertion, and short overwrites exercise the deep parser paths that
/// raw noise cannot reach.
fn mutate(mut bytes: Vec<u8>, ops: &[(u8, u8, u8)]) -> Vec<u8> {
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

fn arb_ops() -> impl Strategy<Value = Vec<(u8, u8, u8)>> {
    prop::collection::vec(any::<(u8, u8, u8)>(), 1..=4)
}

fn arb_actions() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 1..=8)
}

/// A crafted TAR chunk: either a header entry with fully controllable
/// fields or an extension entry (`x`/`g`/`L`/`K`).
#[derive(Debug, Clone)]
enum TarChunk {
    Entry {
        name: Vec<u8>,
        prefix: Vec<u8>,
        size: u64,
        typeflag: u8,
        linkname: Vec<u8>,
        gnu: bool,
        base256: bool,
        corrupt: bool,
        data: Vec<u8>,
    },
    Ext {
        typeflag: u8,
        payload: Vec<u8>,
    },
}

fn arb_tar_chunk() -> impl Strategy<Value = TarChunk> {
    prop_oneof![
        6 => (
            arb_field_bytes(1..=96),
            arb_field_bytes(0..=96),
            0_u64..=600,
            prop_oneof![
                Just(b'0'),
                Just(0_u8),
                Just(b'7'),
                Just(b'5'),
                Just(b'1'),
                Just(b'2')
            ],
            arb_field_bytes(0..=96),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            arb_payload()
        )
            .prop_map(
                |(name, prefix, size, typeflag, linkname, gnu, base256, corrupt, data)| {
                    TarChunk::Entry {
                        name,
                        prefix,
                        size,
                        typeflag,
                        linkname,
                        gnu,
                        base256,
                        corrupt,
                        data,
                    }
                }
            ),
        2 => (
            Just(b'x'),
            prop::collection::vec(arb_pax_record(), 0..=4)
                .prop_map(|records| records.concat())
        )
            .prop_map(|(typeflag, payload)| TarChunk::Ext { typeflag, payload }),
        1 => (
            prop_oneof![Just(b'g'), Just(b'L'), Just(b'K')],
            prop::collection::vec(any::<u8>(), 0..=200)
        )
            .prop_map(|(typeflag, payload)| TarChunk::Ext { typeflag, payload }),
    ]
}

fn arb_pax_record() -> impl Strategy<Value = Vec<u8>> {
    (
        prop_oneof![
            3 => Just(b"path".to_vec()),
            2 => Just(b"linkpath".to_vec()),
            2 => Just(b"size".to_vec()),
            1 => prop::collection::vec(33_u8..=126, 1..=8)
                .prop_map(|k| k.into_iter().filter(|&b| b != b'=').collect::<Vec<u8>>())
                .prop_filter("key must be non-empty", |k| !k.is_empty()),
        ],
        prop::collection::vec(any::<u8>(), 0..=80),
    )
        .prop_map(|(key, value)| {
            let value = if key == b"size" {
                // keep the size record parseable: decimal digits only
                let digits: Vec<u8> = value.iter().map(|b| b'0' + b % 10).take(6).collect();
                if digits.is_empty() {
                    b"0".to_vec()
                } else {
                    digits
                }
            } else {
                value
            };
            pax_record(&key, &value)
        })
}

fn build_tar_chunks(chunks: &[TarChunk]) -> Vec<u8> {
    let parts: Vec<Vec<u8>> = chunks
        .iter()
        .map(|chunk| match chunk {
            TarChunk::Entry {
                name,
                prefix,
                size,
                typeflag,
                linkname,
                gnu,
                base256,
                corrupt,
                data,
            } => {
                let mut spec = HeaderSpec::new(*typeflag, name, *size);
                spec.prefix = prefix;
                spec.linkname = linkname;
                spec.gnu = *gnu;
                spec.base256_size = *base256;
                spec.corrupt_checksum = *corrupt;
                entry_with(&spec, data)
            }
            TarChunk::Ext { typeflag, payload } => ext_entry(*typeflag, "x", payload),
        })
        .collect();
    tar(&parts)
}

fn arb_tar_input() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        4 => arb_bytes(),
        5 => (arb_tar_entries(), arb_ops())
            .prop_map(|(entries, ops)| mutate(writer_tar_bytes(&entries), &ops)),
        3 => prop::collection::vec(arb_tar_chunk(), 0..=5).prop_map(|c| build_tar_chunks(&c)),
    ]
}

fn arb_zip_spec() -> impl Strategy<Value = EntrySpec> {
    prop_oneof![
        // A real deflate stream lifted out of a writer archive — keeps
        // the decode path reachable instead of degenerating to
        // Malformed on every case.
        3 => arb_payload().prop_map(|data| {
            let comp = compressed_region(&writer_zip(&[(
                "d",
                ZipCompressionMethod::Deflate,
                &data,
            )]))
            .to_vec();
            EntrySpec::raw(
                b"d",
                METHOD_DEFLATE,
                &comp,
                u32::try_from(data.len()).unwrap_or(u32::MAX),
                crc32(&data),
            )
        }),
        // A record whose every stored field is arbitrary.
        5 => (
            arb_field_bytes(1..=40),
            prop_oneof![
                4 => Just(METHOD_STORED),
                3 => Just(METHOD_DEFLATE),
                1 => 9_u16..=99_u16
            ],
            prop_oneof![
                3 => Just(FLAG_UTF8),
                1 => Just(FLAG_UTF8 | FLAG_ENCRYPTED),
                1 => Just(FLAG_DATA_DESCRIPTOR),
                1 => Just(0_u16)
            ],
            arb_payload(),
            any::<u32>(),
            any::<u32>(),
            prop_oneof![9 => Just(0_u16), 1 => Just(1_u16)],
        )
            .prop_map(
                |(name, method, flags, data, decl_u, crc, disk)| {
                    let mut spec = EntrySpec::raw(&name, method, &data, decl_u, crc);
                    spec.flags = flags;
                    spec.disk_start = disk;
                    spec
                }
            ),
    ]
}

fn arb_eocd() -> impl Strategy<Value = EocdSpec> {
    (
        prop_oneof![9 => Just(0_u16), 1 => Just(1_u16)],
        prop_oneof![9 => Just(0_u16), 1 => Just(1_u16)],
        prop::option::of(0_u16..=8),
        -64_i64..=64,
        prop::collection::vec(any::<u8>(), 0..=32),
    )
        .prop_map(|(disk_no, cd_disk, count, size_delta, comment)| EocdSpec {
            disk_no,
            cd_disk,
            count_disk: count,
            count,
            cd_size_delta: size_delta,
            cd_offset_override: None,
            comment,
        })
}

fn arb_zip_input() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        4 => arb_bytes(),
        5 => (
            prop::collection::vec(
                (arb_writer_name(), any::<bool>(), arb_payload()),
                0..=4
            ),
            arb_ops()
        )
            .prop_map(|(entries, ops)| mutate(writer_zip_bytes(&entries), &ops)),
        3 => (
            prop::collection::vec(arb_zip_spec(), 0..=3),
            arb_eocd()
        )
            .prop_map(|(specs, eocd)| build_zip(&specs, &eocd)),
    ]
}

/// Drives `TarArchiveReader` over `bytes` with a per-entry action
/// stream: full read + `finish`, `discard_remaining`, partial read +
/// `finish` (often `InvalidState`), or unresolved drop + `discard_pending`.
/// Every observed error kind must stay in the documented TAR set.
fn drive_tar(bytes: &[u8], actions: &[u8]) -> Result<(), TestCaseError> {
    let mut archive = TarArchiveReader::new(Cursor::new(bytes.to_vec()), drive_limits());
    let mut idx = 0_usize;
    let mut failed = false;
    loop {
        // consume the `Result` in the `let`: the temporary drops at the
        // semicolon, so `discard_pending`/`has_pending_entry` in the drop
        // arm below only contend with the entry's own borrow.
        let mut entry = match archive.next_entry() {
            Err(e) => {
                prop_assert!(
                    tar_kind_is_stable(e.kind()),
                    "next_entry kind {:?} outside the TAR set",
                    e.kind()
                );
                break;
            }
            Ok(None) => break,
            Ok(Some(entry)) => entry,
        };
        let act = actions.get(idx).copied().unwrap_or(0);
        idx += 1;
        match act % 4 {
            0 => {
                let mut buf = Vec::new();
                if let Err(e) = entry.read_to_end(&mut buf) {
                    prop_assert!(
                        io_kind(&e).is_some_and(tar_kind_is_stable),
                        "entry read kind {:?} outside the TAR set",
                        io_kind(&e)
                    );
                    failed = true;
                } else if let Err(e) = entry.finish() {
                    prop_assert!(
                        tar_kind_is_stable(e.kind()),
                        "entry finish kind {:?} outside the TAR set",
                        e.kind()
                    );
                    failed = true;
                }
            }
            1 => {
                if let Err(e) = entry.discard_remaining() {
                    prop_assert!(
                        tar_kind_is_stable(e.kind()),
                        "discard_remaining kind {:?} outside the TAR set",
                        e.kind()
                    );
                    failed = true;
                }
            }
            2 => {
                let mut buf = [0_u8; 37];
                if let Err(e) = entry.read(&mut buf) {
                    prop_assert!(
                        io_kind(&e).is_some_and(tar_kind_is_stable),
                        "entry read kind {:?} outside the TAR set",
                        io_kind(&e)
                    );
                    failed = true;
                } else if let Err(e) = entry.finish() {
                    // partial reads legitimately hit InvalidState
                    prop_assert!(
                        tar_kind_is_stable(e.kind()),
                        "entry finish kind {:?} outside the TAR set",
                        e.kind()
                    );
                    failed = true;
                }
            }
            _ => {
                drop(entry);
                prop_assert!(
                    archive.has_pending_entry(),
                    "unresolved drop must leave the archive Pending"
                );
                if let Err(e) = archive.discard_pending() {
                    prop_assert!(
                        tar_kind_is_stable(e.kind()),
                        "discard_pending kind {:?} outside the TAR set",
                        e.kind()
                    );
                    failed = true;
                }
            }
        }
        if failed {
            break;
        }
    }
    match archive.finish() {
        Err(e) => prop_assert!(
            tar_kind_is_stable(e.kind()),
            "archive finish kind {:?} outside the TAR set",
            e.kind()
        ),
        Ok((_inner, receipt)) => {
            prop_assert_eq!(
                receipt.entries_verified() + receipt.entries_unverified(),
                receipt.entries_total(),
                "receipt counts must cover every surfaced entry"
            );
        }
    }
    Ok(())
}

/// Drives `ZipArchiveReader`: `new`, per-entry `open_entry` +
/// `read_to_end` + `finish`, an out-of-range `InvalidInput` probe, then
/// `verify_all`. Every observed error kind must stay in the documented
/// ZIP set.
fn drive_zip(bytes: &[u8]) -> Result<(), TestCaseError> {
    let mut archive = match ZipArchiveReader::new(Cursor::new(bytes.to_vec()), drive_limits()) {
        Err(e) => {
            prop_assert!(
                zip_kind_is_stable(e.kind()),
                "new kind {:?} outside the ZIP set",
                e.kind()
            );
            return Ok(());
        }
        Ok(a) => a,
    };
    let count = archive.entry_count();
    prop_assert!(count <= usize::try_from(drive_limits().max_entries).unwrap_or(usize::MAX));
    for i in 0..count.min(8) {
        let info = archive.entry_info(i);
        prop_assert!(info.is_some(), "entry_info({i}) out of range");
        match archive.open_entry(i) {
            Err(e) => prop_assert!(
                zip_kind_is_stable(e.kind()),
                "open_entry kind {:?} outside the ZIP set",
                e.kind()
            ),
            Ok(mut entry) => {
                let mut buf = Vec::new();
                if let Err(e) = entry.read_to_end(&mut buf) {
                    prop_assert!(
                        io_kind(&e).is_some_and(zip_kind_is_stable),
                        "entry read kind {:?} outside the ZIP set",
                        io_kind(&e)
                    );
                }
                if let Err(e) = entry.finish() {
                    prop_assert!(
                        zip_kind_is_stable(e.kind()),
                        "entry finish kind {:?} outside the ZIP set",
                        e.kind()
                    );
                }
            }
        }
    }
    match archive.open_entry(count) {
        Err(e) => prop_assert_eq!(
            e.kind(),
            ArchiveErrorKind::InvalidInput,
            "out-of-range open_entry must be InvalidInput"
        ),
        Ok(_) => prop_assert!(false, "out-of-range open_entry must fail"),
    }
    match archive.verify_all() {
        Err(e) => prop_assert!(
            zip_kind_is_stable(e.kind()),
            "verify_all kind {:?} outside the ZIP set",
            e.kind()
        ),
        Ok(receipt) => {
            prop_assert_eq!(receipt.entries_unverified(), 0);
            prop_assert_eq!(receipt.entries_verified(), receipt.entries_total());
            prop_assert!(receipt.payload_bytes() <= drive_limits().max_total_uncompressed_bytes);
        }
    }
    Ok(())
}

/// Parser-core connection regression for the typeflag decision: the raw
/// byte classification must reach the observable entry kind/link target,
/// extension folding must stay internal, global PAX must not override
/// effective fields, and an unknown typeflag must stay `Unsupported`.
#[test]
fn tar_typeflag_connection_regression() {
    let mut hard = HeaderSpec::new(b'1', b"hard", 0);
    hard.linkname = b"hard-target";
    let mut sym = HeaderSpec::new(b'2', b"sym", 0);
    sym.linkname = b"sym-target";
    let bytes = tar(&[
        entry_with(&HeaderSpec::new(0, b"nul-file", 0), b""),
        entry_with(&HeaderSpec::new(b'0', b"regular-file", 0), b""),
        entry_with(&HeaderSpec::new(b'7', b"contiguous-file", 0), b""),
        entry_with(&HeaderSpec::new(b'5', b"directory", 0), b""),
        entry_with(&hard, b""),
        entry_with(&sym, b""),
    ]);
    let mut archive = TarArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default());
    let expected = [
        EntryKind::File,
        EntryKind::File,
        EntryKind::File,
        EntryKind::Directory,
        EntryKind::HardLink,
        EntryKind::SymbolicLink,
    ];
    for (index, expected_kind) in expected.iter().enumerate() {
        let entry = archive
            .next_entry()
            .expect("surfaced entry")
            .expect("some entry");
        assert_eq!(entry.info().kind(), *expected_kind);
        if index == 4 {
            assert_eq!(
                entry.info().link_name().map(ArchiveName::as_bytes),
                Some(b"hard-target".as_slice())
            );
        }
        if index == 5 {
            assert_eq!(
                entry.info().link_name().map(ArchiveName::as_bytes),
                Some(b"sym-target".as_slice())
            );
        }
        entry.finish().expect("entry finish");
    }
    assert!(archive.next_entry().expect("end").is_none());

    let local_pax = pax_record(b"path", b"solaris-pax-name");
    let global_pax = pax_record(b"path", b"global-must-not-apply");
    let bytes = tar(&[
        ext_entry(b'X', "X", &local_pax),
        ext_entry(b'g', "g", &global_pax),
        entry_with(&HeaderSpec::file("fallback", 0), b""),
    ]);
    let mut archive = TarArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default());
    let entry = archive
        .next_entry()
        .expect("post-extension entry")
        .expect("some entry");
    assert_eq!(entry.info().name().as_bytes(), b"solaris-pax-name");
    entry.finish().expect("entry finish");
    assert!(archive.next_entry().expect("end").is_none());

    let bytes = tar(&[entry_with(&HeaderSpec::new(b'3', b"char-device", 0), b"")]);
    let mut archive = TarArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default());
    match archive.next_entry() {
        Err(error) => assert_eq!(error.kind(), ArchiveErrorKind::Unsupported),
        Ok(_) => panic!("unsupported typeflag must not surface"),
    }
}

proptest! {
    /// Arbitrary TAR input: the full drive — `next_entry`, entry reads,
    /// `discard_pending`, `finish` — never panics and every error kind
    /// stays in the documented set.
    #[test]
    fn tar_arbitrary_stream_error_set(
        bytes in arb_tar_input(),
        actions in arb_actions(),
    ) {
        drive_tar(&bytes, &actions)?;
    }

    /// A stored name longer than `max_path_bytes` is `LimitExceeded`
    /// before the entry is handed out — via header name (≤100 B), GNU
    /// `L` longname, or PAX `path` (arbitrary bytes incl. non-UTF-8).
    #[test]
    fn tar_name_limit_gate(
        placement in 0_u8..3,
        short in arb_field_bytes(1..=100),
        long in arb_field_bytes(101..=300),
        pax_name in prop::collection::vec(any::<u8>(), 101..=300),
        limit_raw in any::<u32>(),
        data in prop::collection::vec(any::<u8>(), 0..=64),
    ) {
        let (name, bytes) = match placement {
            0 => (
                short.clone(),
                tar(&[entry_with(
                    &HeaderSpec::new(b'0', &short, u64::try_from(data.len()).unwrap_or(0)),
                    &data,
                )]),
            ),
            1 => (
                long.clone(),
                tar(&[ext_entry(b'L', "L", &long), file_entry("f", &data)]),
            ),
            _ => (
                pax_name.clone(),
                tar(&[
                    ext_entry(b'x', "x", &pax_record(b"path", &pax_name)),
                    file_entry("f", &data),
                ]),
            ),
        };
        let span = u32::try_from(name.len() + 9).unwrap_or(u32::MAX);
        let limit = usize::try_from(limit_raw % span).unwrap_or(usize::MAX);
        let mut limits = ArchiveReadLimits::default();
        limits.max_path_bytes = u32::try_from(limit).unwrap_or(u32::MAX);
        let mut archive = TarArchiveReader::new(Cursor::new(bytes), limits);
        if name.len() > limit {
            match archive.next_entry() {
                Err(e) => prop_assert_eq!(e.kind(), ArchiveErrorKind::LimitExceeded),
                _ => prop_assert!(false, "over-limit name must be LimitExceeded"),
            }
        } else {
            let entry = archive
                .next_entry()
                .expect("in-cap name must surface")
                .expect("file entry");
            prop_assert_eq!(entry.info().name().as_bytes(), name.as_slice());
        }
    }

    /// A stored link target longer than `max_path_bytes` is
    /// `LimitExceeded` — via header linkname or GNU `K` longlink.
    #[test]
    fn tar_link_limit_gate(
        placement in 0_u8..2,
        short in arb_field_bytes(1..=100),
        long in arb_field_bytes(101..=300),
        limit_raw in any::<u32>(),
    ) {
        let (link, bytes) = match placement {
            0 => {
                let mut spec = HeaderSpec::new(b'2', b"s", 0);
                spec.linkname = &short;
                (short.clone(), tar(&[entry_with(&spec, b"")]))
            }
            _ => (
                long.clone(),
                tar(&[
                    ext_entry(b'K', "K", &long),
                    entry_with(&HeaderSpec::new(b'2', b"s", 0), b""),
                ]),
            ),
        };
        let span = u32::try_from(link.len() + 9).unwrap_or(u32::MAX);
        let limit = usize::try_from(limit_raw % span).unwrap_or(usize::MAX);
        let mut limits = ArchiveReadLimits::default();
        limits.max_path_bytes = u32::try_from(limit).unwrap_or(u32::MAX);
        let mut archive = TarArchiveReader::new(Cursor::new(bytes), limits);
        if link.len() > limit {
            match archive.next_entry() {
                Err(e) => prop_assert_eq!(e.kind(), ArchiveErrorKind::LimitExceeded),
                _ => prop_assert!(false, "over-limit link target must be LimitExceeded"),
            }
        } else {
            let entry = archive
                .next_entry()
                .expect("in-cap link target must surface")
                .expect("symlink entry");
            prop_assert_eq!(
                entry.info().link_name().map(ArchiveName::as_bytes),
                Some(link.as_slice())
            );
            prop_assert_eq!(entry.info().kind(), EntryKind::SymbolicLink);
        }
    }

    /// A declared payload size above `max_entry_uncompressed_bytes` is
    /// rejected at `surface` — before any payload byte is read.
    #[test]
    fn tar_declared_size_gate(declared in 0_u64..=8192, limit_raw in any::<u32>()) {
        let span = u32::try_from(declared + 9).unwrap_or(u32::MAX);
        let limit = u64::from(limit_raw % span);
        let mut limits = ArchiveReadLimits::default();
        limits.max_entry_uncompressed_bytes = limit;
        let data = vec![0_u8; usize::try_from(declared.min(32)).unwrap_or(0)];
        let bytes = tar(&[entry_with(&HeaderSpec::file("f", declared), &data)]);
        let mut archive = TarArchiveReader::new(Cursor::new(bytes), limits);
        if declared > limit {
            match archive.next_entry() {
                Err(e) => prop_assert_eq!(e.kind(), ArchiveErrorKind::LimitExceeded),
                _ => prop_assert!(false, "over-limit declared size must be LimitExceeded"),
            }
        } else {
            let entry = archive
                .next_entry()
                .expect("in-cap declared size must surface")
                .expect("file entry");
            prop_assert_eq!(entry.info().uncompressed_size(), Some(declared));
        }
    }

    /// `max_entries` caps surfaced entries mid-walk: the first
    /// `min(k, max)` entries surface, the rest are `LimitExceeded`.
    #[test]
    fn tar_max_entries_gate(k in 1_usize..=4, max in 0_u32..=4) {
        let parts: Vec<Vec<u8>> = (0..k)
            .map(|i| file_entry(&format!("f{i}"), b"d"))
            .collect();
        let mut limits = ArchiveReadLimits::default();
        limits.max_entries = max;
        let mut archive = TarArchiveReader::new(Cursor::new(tar(&parts)), limits);
        let cap = usize::try_from(max).unwrap_or(usize::MAX);
        for i in 0..k {
            if i < cap {
                let entry = archive
                    .next_entry()
                    .expect("in-cap entry must surface")
                    .expect("file entry");
                entry.discard_remaining().expect("valid payload discards");
            } else {
                match archive.next_entry() {
                    Err(e) => prop_assert_eq!(e.kind(), ArchiveErrorKind::LimitExceeded),
                    _ => prop_assert!(false, "over-cap entry must be LimitExceeded"),
                }
                return Ok(());
            }
        }
        prop_assert!(archive.next_entry().expect("walk").is_none());
    }

    /// `max_metadata_bytes` accumulates per header block (512 B each,
    /// including the end marker): the drive completes iff the limit
    /// covers `512 * (entries + 1)` bytes.
    #[test]
    fn tar_metadata_gate(k in 1_usize..=4, limit_raw in any::<u32>()) {
        let needed = u64::try_from(BLOCK * (k + 1)).unwrap_or(u64::MAX);
        let span = u32::try_from(needed + 9).unwrap_or(u32::MAX);
        let limit = u64::from(limit_raw % span);
        let parts: Vec<Vec<u8>> = (0..k)
            .map(|i| file_entry(&format!("f{i}"), b"d"))
            .collect();
        let mut limits = ArchiveReadLimits::default();
        limits.max_metadata_bytes = limit;
        let mut archive = TarArchiveReader::new(Cursor::new(tar(&parts)), limits);
        let mut outcome = Ok(());
        for _ in 0..k {
            match archive.next_entry() {
                Ok(Some(entry)) => {
                    if let Err(e) = entry.discard_remaining() {
                        outcome = Err(e.kind());
                        break;
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    outcome = Err(e.kind());
                    break;
                }
            }
        }
        if outcome.is_ok()
            && let Err(e) = archive.finish()
        {
            outcome = Err(e.kind());
        }
        if limit < needed {
            prop_assert_eq!(outcome, Err(ArchiveErrorKind::LimitExceeded));
        } else {
            prop_assert_eq!(outcome, Ok(()));
        }
    }

    /// `max_total_uncompressed_bytes` cuts measured output mid-read:
    /// payloads summing past the budget hit `LimitExceeded`; in-budget
    /// drives complete with `payload_bytes` equal to the measured sum.
    #[test]
    fn tar_total_measured_gate(lens in prop::collection::vec(1_u64..=64, 1..=4), limit_raw in any::<u32>()) {
        let total: u64 = lens.iter().sum();
        let span = u32::try_from(total + 9).unwrap_or(u32::MAX);
        let limit = u64::from(limit_raw % span);
        let parts: Vec<Vec<u8>> = lens
            .iter()
            .enumerate()
            .map(|(i, &len)| {
                entry_with(
                    &HeaderSpec::file(&format!("f{i}"), len),
                    &vec![b'x'; usize::try_from(len).unwrap_or(0)],
                )
            })
            .collect();
        let mut limits = ArchiveReadLimits::default();
        limits.max_total_uncompressed_bytes = limit;
        let mut archive = TarArchiveReader::new(Cursor::new(tar(&parts)), limits);
        let mut hit = None;
        while let Ok(Some(mut entry)) = archive.next_entry() {
            let mut buf = Vec::new();
            match entry.read_to_end(&mut buf) {
                Err(e) => {
                    hit = io_kind(&e);
                    break;
                }
                Ok(_) => {
                    if let Err(e) = entry.finish() {
                        hit = Some(e.kind());
                        break;
                    }
                }
            }
        }
        if total > limit {
            prop_assert_eq!(hit, Some(ArchiveErrorKind::LimitExceeded));
        } else {
            prop_assert_eq!(hit, None);
            let (_inner, receipt) = archive.finish().expect("in-budget finish");
            prop_assert_eq!(receipt.payload_bytes(), total);
            prop_assert_eq!(
                receipt.entries_verified(),
                u32::try_from(lens.len()).unwrap_or(u32::MAX)
            );
        }
    }

    /// Dropping a mid-read entry leaves the archive `Pending`:
    /// `next_entry`/`finish` are `InvalidState` until `discard_pending`
    /// drains the remainder, after which the archive recovers.
    #[test]
    fn tar_pending_recovery(
        entries in prop::collection::vec((arb_writer_name(), prop::collection::vec(any::<u8>(), 1..=64)), 1..=3),
        read_prefix in any::<usize>(),
    ) {
        let bytes = writer_tar_bytes(&entries);
        let declared0 = u64::try_from(entries[0].1.len()).unwrap_or(0);
        let read = u64::try_from(read_prefix).unwrap_or(0) % (declared0 + 1);
        // Drive A: drop mid-read → Pending → next_entry InvalidState →
        // discard_pending → drain the rest → receipt accounts for all.
        {
            let mut archive =
                TarArchiveReader::new(Cursor::new(bytes.clone()), ArchiveReadLimits::default());
            let mut entry = archive
                .next_entry()
                .expect("first entry")
                .expect("some");
            let mut tmp = vec![0_u8; usize::try_from(read).unwrap_or(0)];
            entry.read_exact(&mut tmp).expect("prefix read");
            drop(entry);
            prop_assert!(archive.has_pending_entry());
            match archive.next_entry() {
                Err(e) => prop_assert_eq!(e.kind(), ArchiveErrorKind::InvalidState),
                _ => prop_assert!(false, "next_entry while pending must be InvalidState"),
            }
            archive.discard_pending().expect("pending drain succeeds");
            while let Ok(Some(entry)) = archive.next_entry() {
                entry.discard_remaining().expect("drain");
            }
            let (_inner, receipt) = archive.finish().expect("finish");
            let total_entries = u32::try_from(entries.len()).unwrap_or(u32::MAX);
            prop_assert_eq!(receipt.entries_total(), total_entries);
            prop_assert_eq!(receipt.entries_verified(), 0);
            prop_assert_eq!(receipt.entries_unverified(), total_entries);
            let total_bytes: u64 = entries
                .iter()
                .map(|(_, d)| u64::try_from(d.len()).unwrap_or(0))
                .sum();
            prop_assert_eq!(receipt.payload_bytes(), total_bytes);
        }
        // Drive B: finish while pending is InvalidState too.
        {
            let mut archive =
                TarArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default());
            let entry = archive
                .next_entry()
                .expect("first entry")
                .expect("some");
            drop(entry);
            prop_assert!(archive.has_pending_entry());
            match archive.finish() {
                Err(e) => prop_assert_eq!(e.kind(), ArchiveErrorKind::InvalidState),
                _ => prop_assert!(false, "finish while pending must be InvalidState"),
            }
        }
    }

    /// `finish` drains unsurfaced entries structurally: entries read to
    /// their verified end count `entries_verified`, the rest count
    /// `entries_unverified`, and `payload_bytes` is the measured total.
    #[test]
    fn tar_finish_receipt_invariants(
        entries in arb_tar_entries(),
        read_first in any::<usize>(),
    ) {
        let k = entries.len();
        let verified = read_first % (k + 1);
        let bytes = writer_tar_bytes(&entries);
        let mut archive =
            TarArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default());
        for _ in 0..verified {
            let mut entry = archive
                .next_entry()
                .expect("entry")
                .expect("some");
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).expect("read");
            entry.finish().expect("entry finish");
        }
        let (_inner, receipt) = archive.finish().expect("archive finish");
        let total_entries = u32::try_from(k).unwrap_or(u32::MAX);
        prop_assert_eq!(receipt.entries_total(), total_entries);
        prop_assert_eq!(
            receipt.entries_verified(),
            u32::try_from(verified).unwrap_or(u32::MAX)
        );
        prop_assert_eq!(
            receipt.entries_unverified(),
            total_entries - u32::try_from(verified).unwrap_or(u32::MAX)
        );
        let total_bytes: u64 = entries
            .iter()
            .map(|(_, d)| u64::try_from(d.len()).unwrap_or(0))
            .sum();
        prop_assert_eq!(receipt.payload_bytes(), total_bytes);
    }

    /// Writer-generated archives round-trip exactly: effective name,
    /// `EntryKind::File`, declared size, payload bytes, and receipts —
    /// TAR payloads carry no checksum (`NotPresent`) and a fully read
    /// entry is `ReadToEnd`.
    #[test]
    fn tar_writer_roundtrip(entries in arb_tar_entries()) {
        let bytes = writer_tar_bytes(&entries);
        let mut archive =
            TarArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default());
        for (i, (name, data)) in entries.iter().enumerate() {
            let mut entry = archive
                .next_entry()
                .expect("entry")
                .expect("some");
            prop_assert_eq!(entry.info().name().as_bytes(), name.as_bytes());
            prop_assert_eq!(entry.info().kind(), EntryKind::File);
            prop_assert_eq!(entry.info().index(), i);
            prop_assert_eq!(
                entry.info().uncompressed_size(),
                Some(u64::try_from(data.len()).unwrap_or(u64::MAX))
            );
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).expect("payload");
            prop_assert_eq!(&buf, data);
            let receipt = entry.finish().expect("entry finish");
            prop_assert_eq!(receipt.checksum(), EntryChecksum::NotPresent);
            prop_assert_eq!(receipt.disposition(), EntryDisposition::ReadToEnd);
            prop_assert_eq!(
                receipt.uncompressed_bytes(),
                u64::try_from(data.len()).unwrap_or(u64::MAX)
            );
        }
        prop_assert!(archive.next_entry().expect("end").is_none());
        let (_inner, receipt) = archive.finish().expect("archive finish");
        let total_entries = u32::try_from(entries.len()).unwrap_or(u32::MAX);
        prop_assert_eq!(receipt.entries_total(), total_entries);
        prop_assert_eq!(receipt.entries_verified(), total_entries);
        prop_assert_eq!(receipt.entries_unverified(), 0);
    }

    /// Extension records fold into the following real entry: GNU `L`
    /// longname, PAX `path`/`size`, and GNU `K` longlink each override
    /// the header-carried fields.
    #[test]
    fn tar_pax_gnu_extensions(
        gnu_name in arb_field_bytes(101..=300),
        pax_name in arb_field_bytes(101..=300),
        link in arb_field_bytes(101..=300),
        data in prop::collection::vec(any::<u8>(), 1..=64),
    ) {
        // GNU longname + PAX path (PAX wins) + a declared-size override.
        let size_rec = pax_record(b"size", data.len().to_string().as_bytes());
        let mut pax_payload = pax_record(b"path", &pax_name);
        pax_payload.extend_from_slice(&size_rec);
        let bytes = tar(&[
            ext_entry(b'L', "L", &gnu_name),
            ext_entry(b'x', "x", &pax_payload),
            entry_with(&HeaderSpec::file("stub", 0), &data),
            ext_entry(b'K', "K", &link),
            entry_with(&HeaderSpec::new(b'2', b"s", 0), b""),
        ]);
        let mut archive =
            TarArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default());
        {
            let mut entry = archive
                .next_entry()
                .expect("file entry")
                .expect("some");
            prop_assert_eq!(entry.info().name().as_bytes(), pax_name.as_slice());
            prop_assert_eq!(
                entry.info().uncompressed_size(),
                Some(u64::try_from(data.len()).unwrap_or(u64::MAX))
            );
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).expect("payload");
            prop_assert_eq!(&buf, &data);
            entry.finish().expect("finish");
        }
        {
            let entry = archive
                .next_entry()
                .expect("symlink entry")
                .expect("some");
            prop_assert_eq!(entry.info().kind(), EntryKind::SymbolicLink);
            prop_assert_eq!(
                entry.info().link_name().map(ArchiveName::as_bytes),
                Some(link.as_slice())
            );
            entry.discard_remaining().expect("discard");
        }
        prop_assert!(archive.next_entry().expect("end").is_none());
        let (_inner, receipt) = archive.finish().expect("archive finish");
        prop_assert_eq!(receipt.entries_total(), 2);
    }
}

proptest! {
    /// Arbitrary ZIP input: `new` plus per-entry and archive-wide
    /// verification never panics; every error kind stays in the
    /// documented ZIP set.
    #[test]
    fn zip_arbitrary_stream_error_set(bytes in arb_zip_input()) {
        drive_zip(&bytes)?;
    }

    /// The `max_entries` gates: declared `cd_count` over the limit is
    /// `LimitExceeded` before the CD is buffered; when the declared
    /// count passes, a record stream longer than the limit is still
    /// `LimitExceeded` mid-parse. Otherwise a count that disagrees with
    /// the record stream is `Malformed`.
    #[test]
    fn zip_index_limit_gates(
        k in 0_usize..=3,
        declared in 0_u16..=6,
        max in 0_u32..=6,
    ) {
        let specs: Vec<EntrySpec> = (0..k)
            .map(|i| EntrySpec::stored(&format!("f{i}"), b"d"))
            .collect();
        // `count_disk` must match `count` or the EOCD reads multi-disk
        // (`Unsupported`) before the entry-count gates are reached.
        let eocd = EocdSpec {
            count_disk: Some(declared),
            count: Some(declared),
            ..EocdSpec::default()
        };
        let bytes = build_zip(&specs, &eocd);
        let mut limits = ArchiveReadLimits::default();
        limits.max_entries = max;
        let over_declared = u64::from(declared) > u64::from(max);
        let over_actual = k > usize::try_from(max).unwrap_or(usize::MAX);
        match ZipArchiveReader::new(Cursor::new(bytes), limits) {
            Err(e) if over_declared || over_actual => {
                prop_assert_eq!(e.kind(), ArchiveErrorKind::LimitExceeded);
            }
            Err(e) => {
                prop_assert_eq!(e.kind(), ArchiveErrorKind::Malformed);
            }
            Ok(a) => {
                prop_assert_eq!(usize::from(declared), k, "Ok requires declared == actual");
                prop_assert_eq!(a.entry_count(), k);
            }
        }
    }

    /// `max_metadata_bytes` rejects a declared CD size above the limit
    /// before the region is buffered.
    #[test]
    fn zip_metadata_limit_gate(
        names in prop::collection::vec(arb_writer_name(), 1..=3),
        limit_raw in any::<u32>(),
    ) {
        let specs: Vec<EntrySpec> = names
            .iter()
            .map(|n| EntrySpec::stored(n, b"d"))
            .collect();
        let bytes = build_simple(&specs);
        let cd_size = u64::try_from(bytes.len() - 22 - cd_offset_of(&bytes)).unwrap_or(0);
        let span = u32::try_from(cd_size + 9).unwrap_or(u32::MAX);
        let limit = u64::from(limit_raw % span);
        let mut limits = ArchiveReadLimits::default();
        limits.max_metadata_bytes = limit;
        match ZipArchiveReader::new(Cursor::new(bytes), limits) {
            Err(e) => {
                prop_assert_eq!(e.kind(), ArchiveErrorKind::LimitExceeded);
                prop_assert!(limit < cd_size);
            }
            Ok(a) => {
                prop_assert!(limit >= cd_size);
                prop_assert_eq!(a.entry_count(), names.len());
            }
        }
    }

    /// `max_path_bytes` rejects a stored name above the limit during CD
    /// parsing — before the name bytes are allocated.
    #[test]
    fn zip_path_limit_gate(
        name in arb_field_bytes(1..=64),
        limit_raw in any::<u32>(),
    ) {
        let span = u32::try_from(name.len() + 9).unwrap_or(u32::MAX);
        let limit = limit_raw % span;
        let spec = EntrySpec::raw(&name, METHOD_STORED, b"d", 1, crc32(b"d"));
        let bytes = build_simple(&[spec]);
        let mut limits = ArchiveReadLimits::default();
        limits.max_path_bytes = limit;
        match ZipArchiveReader::new(Cursor::new(bytes), limits) {
            Err(e) => {
                prop_assert_eq!(e.kind(), ArchiveErrorKind::LimitExceeded);
                prop_assert!(name.len() > usize::try_from(limit).unwrap_or(usize::MAX));
            }
            Ok(a) => {
                prop_assert!(name.len() <= usize::try_from(limit).unwrap_or(usize::MAX));
                prop_assert_eq!(a.entry_count(), 1);
            }
        }
    }

    /// A declared uncompressed size above `max_entry_uncompressed_bytes`
    /// is rejected at `open_entry` — before any data byte is decoded.
    #[test]
    fn zip_entry_size_limit_gate(declared in 0_u32..=8192, limit_raw in any::<u64>()) {
        let span = declared.saturating_add(9);
        let limit = limit_raw % u64::from(span);
        let mut spec = EntrySpec::stored("f", b"d");
        spec.declared_uncompressed = declared;
        let bytes = build_simple(&[spec]);
        let mut limits = ArchiveReadLimits::default();
        limits.max_entry_uncompressed_bytes = limit;
        let mut archive = ZipArchiveReader::new(Cursor::new(bytes), limits)
            .expect("index parses within budget");
        if u64::from(declared) > limit {
            match archive.open_entry(0) {
                Err(e) => prop_assert_eq!(e.kind(), ArchiveErrorKind::LimitExceeded),
                _ => prop_assert!(false, "over-limit entry must be LimitExceeded"),
            }
        } else {
            prop_assert!(archive.open_entry(0).is_ok());
        }
    }

    /// `max_total_uncompressed_bytes` accumulates across `verify_all`:
    /// a declared total over the budget is `LimitExceeded`; in-budget
    /// archives verify with `payload_bytes` equal to the measured sum.
    #[test]
    fn zip_total_measured_gate(
        lens in prop::collection::vec(1_u32..=64, 1..=3),
        limit_raw in any::<u64>(),
    ) {
        let total: u64 = lens.iter().map(|&l| u64::from(l)).sum();
        let span = u32::try_from(total + 9).unwrap_or(u32::MAX);
        let limit = limit_raw % u64::from(span);
        let specs: Vec<EntrySpec> = lens
            .iter()
            .enumerate()
            .map(|(i, &len)| {
                EntrySpec::stored(&format!("f{i}"), &vec![b'x'; usize::try_from(len).unwrap_or(0)])
            })
            .collect();
        let bytes = build_simple(&specs);
        let mut limits = ArchiveReadLimits::default();
        limits.max_total_uncompressed_bytes = limit;
        let mut archive = ZipArchiveReader::new(Cursor::new(bytes), limits)
            .expect("index parses within budget");
        if total > limit {
            match archive.verify_all() {
                Err(e) => prop_assert_eq!(e.kind(), ArchiveErrorKind::LimitExceeded),
                _ => prop_assert!(false, "over-budget verify_all must be LimitExceeded"),
            }
        } else {
            let receipt = archive.verify_all().expect("in-budget verify_all");
            prop_assert_eq!(receipt.payload_bytes(), total);
            prop_assert_eq!(
                receipt.entries_verified(),
                u32::try_from(lens.len()).unwrap_or(u32::MAX)
            );
            prop_assert_eq!(receipt.entries_unverified(), 0);
        }
    }

    /// Writer-generated archives round-trip exactly through both the
    /// per-entry path (`open_entry` + `finish`) and `verify_all`.
    #[test]
    fn zip_writer_roundtrip(
        entries in prop::collection::vec(
            (arb_writer_name(), any::<bool>(), arb_payload()),
            0..=4
        ),
    ) {
        let bytes = writer_zip_bytes(&entries);
        let mut archive = ZipArchiveReader::new(Cursor::new(bytes), ArchiveReadLimits::default())
            .expect("writer archive parses");
        prop_assert_eq!(archive.entry_count(), entries.len());
        for (i, (name, _method, data)) in entries.iter().enumerate() {
            let info = archive.entry_info(i).expect("info");
            prop_assert_eq!(info.name().as_bytes(), name.as_bytes());
            prop_assert_eq!(info.kind(), EntryKind::File);
            prop_assert_eq!(info.index(), i);
            let mut entry = archive.open_entry(i).expect("open");
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).expect("payload");
            prop_assert_eq!(&buf, data);
            let receipt = entry.finish().expect("entry finish");
            prop_assert_eq!(receipt.checksum(), EntryChecksum::Verified);
            prop_assert_eq!(receipt.disposition(), EntryDisposition::ReadToEnd);
            prop_assert_eq!(
                receipt.uncompressed_bytes(),
                u64::try_from(data.len()).unwrap_or(u64::MAX)
            );
            prop_assert!(archive.entry_is_verified(i));
        }
        let receipt = archive.verify_all().expect("verify_all");
        let total_entries = u32::try_from(entries.len()).unwrap_or(u32::MAX);
        prop_assert_eq!(receipt.entries_total(), total_entries);
        prop_assert_eq!(receipt.entries_verified(), total_entries);
        prop_assert_eq!(receipt.entries_unverified(), 0);
        let total_bytes: u64 = entries
            .iter()
            .map(|(_, _, d)| u64::try_from(d.len()).unwrap_or(0))
            .sum();
        prop_assert_eq!(receipt.payload_bytes(), total_bytes);
    }
}

/// Parser-core connection regression for the ZIP declared entry-size
/// decision: boundary-1 and boundary values are accepted, boundary+1 is
/// rejected with the entry-size budget before the local header or payload
/// bytes are decoded.
#[test]
fn zip_declared_entry_size_connection_regression() {
    const MAX_ENTRY: u64 = 16;

    for declared in [MAX_ENTRY - 1, MAX_ENTRY] {
        let mut spec = EntrySpec::stored("f", b"d");
        spec.declared_uncompressed = u32::try_from(declared).expect("boundary fits u32");
        let bytes = build_simple(&[spec]);
        let mut limits = ArchiveReadLimits::default();
        limits.max_entry_uncompressed_bytes = MAX_ENTRY;
        let mut archive =
            ZipArchiveReader::new(Cursor::new(bytes), limits).expect("index parses within budget");
        let entry = archive
            .open_entry(0)
            .expect("in-cap declared entry size must be accepted");
        assert_eq!(entry.info().uncompressed_size(), Some(declared));
    }

    let declared = MAX_ENTRY + 1;
    let mut spec = EntrySpec::stored("f", b"d");
    spec.declared_uncompressed = u32::try_from(declared).expect("boundary fits u32");
    let mut bytes = build_simple(&[spec]);
    bytes[0] = 0;
    let mut limits = ArchiveReadLimits::default();
    limits.max_entry_uncompressed_bytes = MAX_ENTRY;
    let mut archive = ZipArchiveReader::new(Cursor::new(bytes), limits)
        .expect("index parses without reading local headers");
    match archive.open_entry(0) {
        Err(error) => {
            assert_eq!(error.kind(), ArchiveErrorKind::LimitExceeded);
            assert!(
                error.message().contains("max_entry_uncompressed_bytes"),
                "over-limit rejection must name the entry-size budget: {}",
                error.message()
            );
            assert!(
                error.message().contains("declared 17 exceeds limit 16"),
                "over-limit rejection must keep the declared/limit detail: {}",
                error.message()
            );
        }
        Ok(_) => panic!("over-limit declared entry size must be rejected"),
    }
}
