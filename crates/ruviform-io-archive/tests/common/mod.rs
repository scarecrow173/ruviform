//! Byte-level ZIP crafting helpers for malformed-structure tests.
//!
//! Valid baseline fixtures come from the crate's own
//! [`ZipArchiveWriter`]; corrupted structures are assembled
//! record-by-record so every declared field can be controlled.

use std::io::{Cursor, Write};

use ruviform_io_archive::zip::{ZipArchiveWriter, ZipCompressionMethod, ZipEntryOptions};

pub const METHOD_STORED: u16 = 0;
pub const METHOD_DEFLATE: u16 = 8;
pub const FLAG_UTF8: u16 = 1 << 11;
pub const FLAG_ENCRYPTED: u16 = 1;
pub const FLAG_DATA_DESCRIPTOR: u16 = 1 << 3;

const EOCD_SIG: u32 = 0x0605_4b50;
const CD_SIG: u32 = 0x0201_4b50;
const LFH_SIG: u32 = 0x0403_4b50;
const EOCD_LEN: usize = 22;
const CD_LEN: usize = 46;
const LFH_LEN: usize = 30;

/// Reference CRC32 (IEEE, same polynomial as crc32fast) for fixtures.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// One crafted entry: every stored field is independently controllable.
pub struct EntrySpec {
    /// Raw name bytes written into the central directory record.
    pub name: Vec<u8>,
    /// Name written into the *local* header; `None` → same as `name`.
    pub local_name: Option<Vec<u8>>,
    pub method: u16,
    /// Method written into the local header; `None` → same as `method`.
    pub local_method: Option<u16>,
    pub flags: u16,
    /// Flags written into the local header; `None` → same as `flags`.
    pub local_flags: Option<u16>,
    /// Payload bytes physically written after the local header.
    pub data: Vec<u8>,
    /// Declared compressed size (both headers); `None` → `data.len()`.
    pub declared_compressed: Option<u32>,
    /// Declared uncompressed size (both headers).
    pub declared_uncompressed: u32,
    pub crc32: u32,
    pub local_extra: Vec<u8>,
    pub cd_extra: Vec<u8>,
    pub cd_comment: Vec<u8>,
    pub external_attrs: u32,
    pub disk_start: u16,
    /// CD `local_header_offset` override; `None` → the actual offset.
    pub local_offset_override: Option<u32>,
}

impl EntrySpec {
    /// A well-formed stored entry (CRC computed from `data`).
    pub fn stored(name: &str, data: &[u8]) -> Self {
        Self::raw(
            name.as_bytes(),
            METHOD_STORED,
            data,
            u32::try_from(data.len()).unwrap_or(u32::MAX),
            crc32(data),
        )
    }

    /// Fully-specified entry; remaining fields default to sane values.
    pub fn raw(name: &[u8], method: u16, data: &[u8], uncompressed_size: u32, crc32: u32) -> Self {
        Self {
            name: name.to_vec(),
            local_name: None,
            method,
            local_method: None,
            flags: FLAG_UTF8,
            local_flags: None,
            data: data.to_vec(),
            declared_compressed: None,
            declared_uncompressed: uncompressed_size,
            crc32,
            local_extra: Vec::new(),
            cd_extra: Vec::new(),
            cd_comment: Vec::new(),
            external_attrs: 0o100_644 << 16,
            disk_start: 0,
            local_offset_override: None,
        }
    }

    fn compressed_declared(&self) -> u32 {
        self.declared_compressed
            .unwrap_or(u32::try_from(self.data.len()).unwrap_or(u32::MAX))
    }
}

/// EOCD field overrides; `EocdSpec::default()` produces a correct record.
#[derive(Default)]
pub struct EocdSpec {
    pub disk_no: u16,
    pub cd_disk: u16,
    /// `cd_count` on this disk; `None` → entry count.
    pub count_disk: Option<u16>,
    /// `cd_count` total; `None` → entry count.
    pub count: Option<u16>,
    /// Added to the true CD size (may be negative).
    pub cd_size_delta: i64,
    /// Overrides the declared CD offset entirely.
    pub cd_offset_override: Option<u32>,
    pub comment: Vec<u8>,
}

/// Assembles a ZIP byte stream from the crafted entries and EOCD spec.
#[must_use]
pub fn build_zip(entries: &[EntrySpec], eocd: &EocdSpec) -> Vec<u8> {
    let mut out = Vec::new();
    let mut offsets = Vec::with_capacity(entries.len());
    for spec in entries {
        offsets.push(u32::try_from(out.len()).unwrap_or(u32::MAX));
        let local_name = spec.local_name.as_ref().unwrap_or(&spec.name);
        put_u32(&mut out, LFH_SIG);
        put_u16(&mut out, 20);
        put_u16(&mut out, spec.local_flags.unwrap_or(spec.flags));
        put_u16(&mut out, spec.local_method.unwrap_or(spec.method));
        put_u16(&mut out, 0);
        put_u16(&mut out, 0x21);
        put_u32(&mut out, spec.crc32);
        put_u32(&mut out, spec.compressed_declared());
        put_u32(&mut out, spec.declared_uncompressed);
        put_u16(
            &mut out,
            u16::try_from(local_name.len()).unwrap_or(u16::MAX),
        );
        put_u16(
            &mut out,
            u16::try_from(spec.local_extra.len()).unwrap_or(u16::MAX),
        );
        out.extend_from_slice(local_name);
        out.extend_from_slice(&spec.local_extra);
        out.extend_from_slice(&spec.data);
    }
    let cd_offset = u32::try_from(out.len()).unwrap_or(u32::MAX);
    for (spec, &offset) in entries.iter().zip(&offsets) {
        put_u32(&mut out, CD_SIG);
        put_u16(&mut out, 20);
        put_u16(&mut out, 20);
        put_u16(&mut out, spec.flags);
        put_u16(&mut out, spec.method);
        put_u16(&mut out, 0);
        put_u16(&mut out, 0x21);
        put_u32(&mut out, spec.crc32);
        put_u32(&mut out, spec.compressed_declared());
        put_u32(&mut out, spec.declared_uncompressed);
        put_u16(&mut out, u16::try_from(spec.name.len()).unwrap_or(u16::MAX));
        put_u16(
            &mut out,
            u16::try_from(spec.cd_extra.len()).unwrap_or(u16::MAX),
        );
        put_u16(
            &mut out,
            u16::try_from(spec.cd_comment.len()).unwrap_or(u16::MAX),
        );
        put_u16(&mut out, spec.disk_start);
        put_u16(&mut out, 0);
        put_u32(&mut out, spec.external_attrs);
        put_u32(&mut out, spec.local_offset_override.unwrap_or(offset));
        out.extend_from_slice(&spec.name);
        out.extend_from_slice(&spec.cd_extra);
        out.extend_from_slice(&spec.cd_comment);
    }
    let cd_size = u32::try_from(out.len() - usize::try_from(cd_offset).unwrap_or(usize::MAX))
        .unwrap_or(u32::MAX);
    let count = u16::try_from(entries.len()).unwrap_or(u16::MAX);
    put_u32(&mut out, EOCD_SIG);
    put_u16(&mut out, eocd.disk_no);
    put_u16(&mut out, eocd.cd_disk);
    put_u16(&mut out, eocd.count_disk.unwrap_or(count));
    put_u16(&mut out, eocd.count.unwrap_or(count));
    let cd_size = u64::from(cd_size).saturating_add_signed(eocd.cd_size_delta);
    put_u32(&mut out, u32::try_from(cd_size).unwrap_or(u32::MAX));
    put_u32(&mut out, eocd.cd_offset_override.unwrap_or(cd_offset));
    put_u16(
        &mut out,
        u16::try_from(eocd.comment.len()).unwrap_or(u16::MAX),
    );
    out.extend_from_slice(&eocd.comment);
    out
}

/// Convenience wrapper for a default EOCD.
#[must_use]
pub fn build_simple(entries: &[EntrySpec]) -> Vec<u8> {
    build_zip(entries, &EocdSpec::default())
}

/// Builds a valid archive through the public writer (baseline fixture).
#[must_use]
pub fn writer_zip(entries: &[(&str, ZipCompressionMethod, &[u8])]) -> Vec<u8> {
    let mut writer = ZipArchiveWriter::new(Cursor::new(Vec::new()));
    for &(name, method, data) in entries {
        let mut entry = writer
            .start_entry(name, &ZipEntryOptions::new(method))
            .expect("start_entry");
        entry.write_all(data).expect("write entry payload");
        entry.finish().expect("entry finish");
    }
    writer.finish().expect("archive finish").into_inner()
}

/// Extracts the compressed bytes of a single-entry `writer_zip`
/// archive — a real deflate/stored stream for crafted fixtures.
#[must_use]
pub fn compressed_region(zip: &[u8]) -> &[u8] {
    let name_len = usize::from(u16::from_le_bytes([zip[26], zip[27]]));
    let extra_len = usize::from(u16::from_le_bytes([zip[28], zip[29]]));
    let data_start = LFH_LEN + name_len + extra_len;
    let eocd = zip.len() - EOCD_LEN;
    let cd_offset = usize::try_from(u32::from_le_bytes([
        zip[eocd + 16],
        zip[eocd + 17],
        zip[eocd + 18],
        zip[eocd + 19],
    ]))
    .unwrap_or(usize::MAX);
    &zip[data_start..cd_offset]
}

/// Offset of the central directory inside a `writer_zip` archive.
#[must_use]
pub fn cd_offset_of(zip: &[u8]) -> usize {
    let eocd = zip.len() - EOCD_LEN;
    usize::try_from(u32::from_le_bytes([
        zip[eocd + 16],
        zip[eocd + 17],
        zip[eocd + 18],
        zip[eocd + 19],
    ]))
    .unwrap_or(usize::MAX)
}

/// Byte offset of the CD record for entry `idx` (all names equal length).
#[must_use]
pub fn cd_record_offset(zip: &[u8], idx: usize, name_len: usize) -> usize {
    cd_offset_of(zip) + idx * (CD_LEN + name_len)
}

fn put_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}
