//! Byte-level TAR crafting helpers for malformed-structure tests.
//!
//! Valid baselines come from the crate's own [`TarArchiveWriter`];
//! corrupted or format-variant fixtures are assembled field-by-field
//! so every stored byte can be controlled.
//!
//! Shared across the TAR test binaries; individual helpers are not all
//! used by each one.
#![allow(dead_code)]

use std::io::Write;

use ruviform_io_archive::tar::{TarArchiveWriter, TarEntryOptions};

pub const BLOCK: usize = 512;

/// Unsigned header checksum with the `chksum` field treated as spaces.
#[must_use]
pub fn checksum(block: &[u8; BLOCK]) -> u32 {
    block
        .iter()
        .enumerate()
        .map(|(i, &b)| {
            if (148..156).contains(&i) {
                u32::from(b' ')
            } else {
                u32::from(b)
            }
        })
        .sum()
}

/// Zero-padded octal digits + NUL terminator in an N-byte field.
fn put_octal(field: &mut [u8], value: u64) {
    let digits = field.len() - 1;
    let text = format!("{value:0digits$o}");
    field[..digits].copy_from_slice(text.as_bytes());
    field[digits] = 0;
}

/// GNU base-256 numeric field (first byte's 0x80 bit is the marker).
pub fn put_base256(field: &mut [u8], value: u64) {
    field.fill(0);
    field[0] = 0x80;
    let bytes = value.to_be_bytes();
    let start = field.len() - bytes.len();
    field[start..].copy_from_slice(&bytes);
}

/// Padding after `size` payload bytes to the next 512 boundary.
#[must_use]
pub const fn pad_len(size: u64) -> u64 {
    (BLOCK as u64 - size % BLOCK as u64) % BLOCK as u64
}

/// A TAR header fixture: every stored field independently controllable.
pub struct HeaderSpec<'a> {
    pub name: &'a [u8],
    pub prefix: &'a [u8],
    /// Payload size; written octal unless `base256_size`.
    pub size: u64,
    pub typeflag: u8,
    pub linkname: &'a [u8],
    /// `"ustar "` magic instead of `"ustar\0"`.
    pub gnu: bool,
    pub mtime: u64,
    /// Write the size field as GNU base-256.
    pub base256_size: bool,
    /// Corrupt the checksum field after computing it.
    pub corrupt_checksum: bool,
}

impl<'a> HeaderSpec<'a> {
    /// A regular-file header (`typeflag '0'`).
    #[must_use]
    pub fn file(name: &'a str, size: u64) -> Self {
        Self::new(b'0', name.as_bytes(), size)
    }

    /// Any-typeflag header with defaults for the rest.
    #[must_use]
    pub fn new(typeflag: u8, name: &'a [u8], size: u64) -> Self {
        Self {
            name,
            prefix: b"",
            size,
            typeflag,
            linkname: b"",
            gnu: false,
            mtime: 1_700_000_000,
            base256_size: false,
            corrupt_checksum: false,
        }
    }

    /// Serializes the 512-byte header (checksum computed last).
    #[must_use]
    pub fn block(&self) -> [u8; BLOCK] {
        let mut block = [0_u8; BLOCK];
        block[..self.name.len()].copy_from_slice(self.name);
        put_octal(&mut block[100..108], 0o644);
        put_octal(&mut block[108..116], 0);
        put_octal(&mut block[116..124], 0);
        if self.base256_size {
            put_base256(&mut block[124..136], self.size);
        } else {
            put_octal(&mut block[124..136], self.size);
        }
        put_octal(&mut block[136..148], self.mtime);
        block[148..156].copy_from_slice(b"        ");
        block[156] = self.typeflag;
        block[157..157 + self.linkname.len()].copy_from_slice(self.linkname);
        if self.gnu {
            block[257..263].copy_from_slice(b"ustar ");
            block[263..265].copy_from_slice(b" \0");
        } else {
            block[257..263].copy_from_slice(b"ustar\0");
            block[263..265].copy_from_slice(b"00");
        }
        block[345..345 + self.prefix.len()].copy_from_slice(self.prefix);
        let sum = checksum(&block);
        let text = format!("{sum:06o}\0 ");
        block[148..156].copy_from_slice(text.as_bytes());
        if self.corrupt_checksum {
            // Flip a content byte in the unused tail: the checksum
            // field stays valid octal, so the declared sum no longer
            // matches the measured one → ChecksumMismatch.
            block[510] ^= 0xFF;
        }
        block
    }
}

/// `"<len> <key>=<value>\n"` PAX record bytes.
#[must_use]
pub fn pax_record(key: &[u8], value: &[u8]) -> Vec<u8> {
    let body_len = key.len() + 1 + value.len() + 1;
    let mut len = body_len + 2;
    loop {
        let total = len.to_string().len() + 1 + body_len;
        if total == len {
            break;
        }
        len = total;
    }
    let mut record = Vec::with_capacity(len);
    record.extend_from_slice(len.to_string().as_bytes());
    record.push(b' ');
    record.extend_from_slice(key);
    record.push(b'=');
    record.extend_from_slice(value);
    record.push(b'\n');
    record
}

/// A complete extension entry (`x`/`g`/`L`/`K`): header + payload +
/// block padding.
#[must_use]
pub fn ext_entry(typeflag: u8, name: &str, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(
        &HeaderSpec::new(typeflag, name.as_bytes(), payload.len() as u64).block(),
    );
    out.extend_from_slice(payload);
    out.resize(
        out.len() + usize::try_from(pad_len(payload.len() as u64)).unwrap_or(0),
        0,
    );
    out
}

/// A complete file entry: header + payload + block padding.
#[must_use]
pub fn file_entry(name: &str, data: &[u8]) -> Vec<u8> {
    entry_with(&HeaderSpec::file(name, data.len() as u64), data)
}

/// A complete entry from a caller-crafted header.
#[must_use]
pub fn entry_with(header: &HeaderSpec<'_>, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&header.block());
    out.extend_from_slice(data);
    out.resize(
        out.len() + usize::try_from(pad_len(data.len() as u64)).unwrap_or(0),
        0,
    );
    out
}

/// Concatenates entry chunks and appends the two zero end blocks.
#[must_use]
pub fn tar(chunks: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    for chunk in chunks {
        out.extend_from_slice(chunk);
    }
    out.resize(out.len() + BLOCK * 2, 0);
    out
}

/// Valid archive produced through the public writer.
#[must_use]
pub fn writer_tar(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = TarArchiveWriter::new(Vec::new());
    for &(name, data) in entries {
        let mut entry = writer
            .start_entry(name, data.len() as u64, &TarEntryOptions::default())
            .expect("start_entry");
        entry.write_all(data).expect("write payload");
        entry.finish().expect("entry finish");
    }
    writer.finish().expect("archive finish")
}
