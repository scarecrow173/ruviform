//! TAR writer verification: declared-size capping, padding,
//! USTAR/PAX output selection, poison-on-drop, and explicit finish as
//! the only completion proof.

mod common;

use std::io::{Cursor, Read, Write};

use common::tar::{BLOCK, writer_tar};
use ruviform_io_archive::tar::{TarArchiveReader, TarArchiveWriter, TarEntryOptions};
use ruviform_io_archive::{ArchiveError, ArchiveErrorKind, ArchiveReadLimits, EntryKind};

fn limits() -> ArchiveReadLimits {
    ArchiveReadLimits::default()
}

fn kind_of(result: Result<impl Sized, ArchiveError>) -> ArchiveErrorKind {
    match result {
        Ok(_) => panic!("expected error"),
        Err(error) => error.kind(),
    }
}

#[test]
fn written_archive_round_trips_through_reader() {
    let bytes = writer_tar(&[("one.txt", b"1"), ("dir/two.txt", b"22")]);
    assert_eq!(bytes.len() % BLOCK, 0);
    // Two zero end blocks are the last thing written.
    assert!(bytes[bytes.len() - BLOCK * 2..].iter().all(|&b| b == 0));

    let mut archive = TarArchiveReader::new(Cursor::new(bytes), limits());
    let mut names = Vec::new();
    while let Some(mut entry) = archive.next_entry().expect("next") {
        names.push(entry.info().name().as_bytes().to_vec());
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).expect("read");
        entry.finish().expect("entry finish");
    }
    assert_eq!(names, [b"one.txt".as_slice(), b"dir/two.txt"]);
    archive.finish().expect("archive finish");
}

#[test]
fn ustar_prefix_split_used_for_splittable_long_names() {
    // 130 bytes total: prefix 100 + "/file" → splits under 155/100.
    let name = format!("{}/file", "d".repeat(100));
    let bytes = writer_tar(&[(&name, b"x")]);
    // The first header's prefix field (345..500) must be populated —
    // no PAX entry precedes it.
    assert_eq!(&bytes[257..263], b"ustar\0");
    assert_eq!(&bytes[345..445], "d".repeat(100).as_bytes());
    let mut archive = TarArchiveReader::new(Cursor::new(bytes), limits());
    let entry = archive.next_entry().expect("e").expect("some");
    assert_eq!(entry.info().name().as_bytes(), name.as_bytes());
}

#[test]
fn unsplittable_long_name_goes_through_pax() {
    // 150-byte single component — no legal 155/100 split.
    let name = "n".repeat(150);
    let bytes = writer_tar(&[(&name, b"x")]);
    // First header must be the PAX 'x' extension carrying `path=`.
    assert_eq!(bytes[156], b'x');
    let payload_start = BLOCK;
    let payload = &bytes[payload_start..payload_start + 64];
    assert!(
        payload.windows(5).any(|w| w == b"path="),
        "pax payload must carry path="
    );
    let mut archive = TarArchiveReader::new(Cursor::new(bytes), limits());
    let entry = archive.next_entry().expect("e").expect("some");
    assert_eq!(entry.info().name().as_bytes(), name.as_bytes());
}

#[test]
fn size_beyond_octal_range_goes_through_pax() {
    let mut writer = TarArchiveWriter::new(Vec::new());
    {
        // Declared > 11-digit octal max → PAX size= record.
        let entry = writer
            .start_entry("huge", 8_589_934_592, &TarEntryOptions::default())
            .expect("start");
        // Never finished — poisoned, but the headers were emitted.
        drop(entry);
    }
    let bytes = writer.into_inner();
    assert_eq!(bytes[156], b'x');
    assert!(
        bytes[BLOCK..BLOCK * 2].windows(5).any(|w| w == b"size="),
        "pax payload must carry size="
    );
    // A reader surfaces the PAX-declared size on the entry (the
    // declared value exceeds the default per-entry cap, so raise it).
    let mut limit = ArchiveReadLimits::default();
    limit.max_entry_uncompressed_bytes = u64::MAX;
    let mut archive = TarArchiveReader::new(Cursor::new(bytes), limit);
    let entry = archive.next_entry().expect("e").expect("some");
    assert_eq!(
        entry.info().uncompressed_size(),
        Some(8_589_934_592),
        "pax size must override the clamped header field"
    );
}

#[test]
fn pax_size_header_field_stays_in_octal_range() {
    let mut writer = TarArchiveWriter::new(Vec::new());
    {
        let entry = writer
            .start_entry("huge", 8_589_934_592, &TarEntryOptions::default())
            .expect("start");
        drop(entry);
    }
    let bytes = writer.into_inner();
    // The file header follows the 'x' header AND its payload block —
    // its size field must hold the clamp value, not overflow garbage.
    let file_header = &bytes[BLOCK * 2..BLOCK * 3];
    let size_field = &file_header[124..136];
    assert_eq!(size_field, b"77777777777\0");
}

#[test]
fn declared_size_caps_writes() {
    let mut writer = TarArchiveWriter::new(Vec::new());
    {
        let mut entry = writer
            .start_entry("a", 4, &TarEntryOptions::default())
            .expect("start");
        // write() accepts at most the declared count.
        assert_eq!(entry.write(b"12345678").expect("write"), 4);
        // …and a further call fails rather than leaking bytes.
        assert!(entry.write(b"9").is_err());
        entry.finish().expect("finish");
    }
    let bytes = writer.finish().expect("archive finish");
    let mut archive = TarArchiveReader::new(Cursor::new(bytes), limits());
    let mut entry = archive.next_entry().expect("e").expect("some");
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).expect("read");
    assert_eq!(buf, b"1234");
}

#[test]
fn short_payload_finish_is_size_mismatch_and_poisons() {
    let mut writer = TarArchiveWriter::new(Vec::new());
    {
        let mut entry = writer
            .start_entry("a", 10, &TarEntryOptions::default())
            .expect("start");
        entry.write_all(b"1234").expect("write");
        assert_eq!(kind_of(entry.finish()), ArchiveErrorKind::SizeMismatch);
    }
    // The header already declared 10 — the archive is poisoned.
    assert_eq!(
        kind_of(writer.start_entry("b", 1, &TarEntryOptions::default())),
        ArchiveErrorKind::InvalidState
    );
    assert_eq!(kind_of(writer.finish()), ArchiveErrorKind::InvalidState);
}

#[test]
fn dropped_entry_writer_poisons_archive() {
    let mut writer = TarArchiveWriter::new(Vec::new());
    {
        let _entry = writer
            .start_entry("a", 10, &TarEntryOptions::default())
            .expect("start");
        // dropped without finish
    }
    assert_eq!(kind_of(writer.finish()), ArchiveErrorKind::InvalidState);
}

#[test]
fn archive_not_complete_until_finish() {
    // into_inner yields an unfinished TAR (no end blocks).
    let writer = TarArchiveWriter::new(Vec::new());
    let bytes = writer.into_inner();
    assert_eq!(bytes, [] as [u8; 0]);
}

#[test]
fn directory_markers_round_trip() {
    let mut writer = TarArchiveWriter::new(Vec::new());
    writer.add_directory("dir").expect("dir");
    let bytes = writer.finish().expect("finish");
    let mut archive = TarArchiveReader::new(Cursor::new(bytes), limits());
    let entry = archive.next_entry().expect("e").expect("some");
    assert_eq!(entry.info().kind(), EntryKind::Directory);
    assert_eq!(entry.info().name().as_bytes(), b"dir/");
    assert_eq!(entry.info().uncompressed_size(), Some(0));
}

#[test]
fn invalid_entry_names_are_rejected() {
    let mut writer = TarArchiveWriter::new(Vec::new());
    assert_eq!(
        kind_of(writer.start_entry("", 1, &TarEntryOptions::default())),
        ArchiveErrorKind::InvalidInput
    );
    assert_eq!(
        kind_of(writer.start_entry("bad\0name", 1, &TarEntryOptions::default())),
        ArchiveErrorKind::InvalidInput
    );
    // The archive itself is still usable.
    writer.finish().expect("finish");
}

#[test]
fn deterministic_output_is_byte_reproducible() {
    // Default mtime 0: two identical writes produce identical bytes.
    assert_eq!(
        writer_tar(&[("a", b"1"), ("b", b"2")]),
        writer_tar(&[("a", b"1"), ("b", b"2")])
    );
}

#[test]
fn take_composition_leaves_surplus_in_source() {
    // Copy exactly `declared` bytes from a source: the writer never
    // pulls beyond the declaration, surplus stays in the source.
    let mut writer = TarArchiveWriter::new(Vec::new());
    let mut source = Cursor::new(b"1234EXTRA".to_vec());
    {
        let mut entry = writer
            .start_entry("a", 4, &TarEntryOptions::default())
            .expect("start");
        let copied = std::io::copy(&mut (&mut source).take(4), &mut entry).expect("copy");
        assert_eq!(copied, 4);
        entry.finish().expect("entry finish");
    }
    writer.finish().expect("archive finish");
    // The extra bytes were never consumed into the entry.
    let mut rest = String::new();
    source.read_to_string(&mut rest).expect("source read");
    assert_eq!(rest, "EXTRA");
}

#[test]
fn entry_count_tracks_finished_entries() {
    let mut writer = TarArchiveWriter::new(Vec::new());
    assert_eq!(writer.entry_count(), 0);
    writer.add_directory("d").expect("dir");
    {
        let mut entry = writer
            .start_entry("a", 1, &TarEntryOptions::default())
            .expect("start");
        entry.write_all(b"x").expect("write");
        entry.finish().expect("finish");
    }
    assert_eq!(writer.entry_count(), 2);
    writer.finish().expect("archive finish");
}

#[test]
fn writer_produced_archive_parses_byte_for_byte() {
    // Sanity on the emitted header: name, octal size, checksum, magic.
    let bytes = writer_tar(&[("check.txt", b"abc")]);
    let header = &bytes[..BLOCK];
    assert_eq!(&header[..9], b"check.txt");
    assert_eq!(&header[124..136], b"00000000003\0");
    assert_eq!(&header[257..263], b"ustar\0");
    assert_eq!(&header[263..265], b"00");
    let sum: u32 = header
        .iter()
        .enumerate()
        .map(|(i, &b)| {
            if (148..156).contains(&i) {
                u32::from(b' ')
            } else {
                u32::from(b)
            }
        })
        .sum();
    // Checksum field: six octal digits + NUL + space.
    let declared = u32::from_str_radix(std::str::from_utf8(&header[148..154]).unwrap(), 8).unwrap();
    assert_eq!(declared, sum);
    assert_eq!(header[154], 0);
    assert_eq!(header[155], b' ');
}
