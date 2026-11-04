//! Internal ZIP record layout and little-endian helpers.
//!
//! These types mirror the on-disk structures of APPNOTE.TXT. They are
//! private implementation details — none of them appear in the public
//! API.

/// Fixed size of an end-of-central-directory record (without comment).
pub(crate) const EOCD_LEN: usize = 22;
/// Fixed size of a central-directory file header (without variable
/// fields).
pub(crate) const CD_RECORD_LEN: usize = 46;
/// Fixed size of a local file header (without variable fields).
pub(crate) const LFH_LEN: usize = 30;

pub(crate) const EOCD_SIG: u32 = 0x0605_4b50;
pub(crate) const CD_SIG: u32 = 0x0201_4b50;
pub(crate) const LFH_SIG: u32 = 0x0403_4b50;

/// General-purpose flag bit 0: entry is encrypted.
pub(crate) const FLAG_ENCRYPTED: u16 = 1;
/// General-purpose flag bit 3: sizes/CRC live in a trailing data
/// descriptor, not the local header.
pub(crate) const FLAG_DATA_DESCRIPTOR: u16 = 1 << 3;
/// General-purpose flag bit 11: name/comment are UTF-8.
pub(crate) const FLAG_UTF8: u16 = 1 << 11;

pub(crate) const METHOD_STORED: u16 = 0;
pub(crate) const METHOD_DEFLATE: u16 = 8;

/// Maximum byte span that can hold an EOCD record: 22-byte record plus
/// the maximum 64 KiB comment.
pub(crate) const EOCD_SEARCH_SPAN: u64 = EOCD_LEN as u64 + u16::MAX as u64;

pub(crate) fn u16le(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

pub(crate) fn u32le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// Parsed end-of-central-directory record.
pub(crate) struct Eocd {
    pub disk_no: u16,
    pub cd_disk: u16,
    pub cd_count_disk: u16,
    pub cd_count: u16,
    pub cd_size: u32,
    pub cd_offset: u32,
    /// Absolute position of this EOCD record within the input.
    pub position: u64,
}

impl Eocd {
    /// Parses the fixed 22-byte record starting at `buf[0]` (signature
    /// already matched).
    pub(crate) fn parse(buf: &[u8], position: u64) -> Self {
        Self {
            disk_no: u16le(&buf[4..6]),
            cd_disk: u16le(&buf[6..8]),
            cd_count_disk: u16le(&buf[8..10]),
            cd_count: u16le(&buf[10..12]),
            cd_size: u32le(&buf[12..16]),
            cd_offset: u32le(&buf[16..20]),
            position,
        }
    }

    /// `true` when any field is saturated and the real values live in a
    /// ZIP64 EOCD record — which this crate does not yet read.
    pub(crate) fn requires_zip64(&self) -> bool {
        self.cd_count == u16::MAX
            || self.cd_count_disk == u16::MAX
            || self.cd_size == u32::MAX
            || self.cd_offset == u32::MAX
            || self.disk_no == u16::MAX
            || self.cd_disk == u16::MAX
    }

    /// `true` when this record claims a multi-disk archive.
    pub(crate) fn is_multi_disk(&self) -> bool {
        self.disk_no != 0 || self.cd_disk != 0 || self.cd_count_disk != self.cd_count
    }
}

/// One parsed central-directory file header (index entry).
pub(crate) struct CdRecord {
    pub flags: u16,
    pub method: u16,
    pub crc32: u32,
    /// Compressed size as declared by the CD (u32 on disk; ZIP64-wide
    /// values are rejected during header validation).
    pub compressed_size: u32,
    /// Uncompressed size as declared by the CD.
    pub uncompressed_size: u32,
    /// Absolute offset of the local file header, as declared by the CD.
    pub local_offset: u32,
    pub external_attrs: u32,
    /// Stored name bytes, unmodified.
    pub name: Vec<u8>,
}

/// Writes a u16 little-endian into `buf`.
pub(crate) fn put_u16le(buf: &mut [u8], value: u16) {
    buf[..2].copy_from_slice(&value.to_le_bytes());
}

/// Writes a u32 little-endian into `buf`.
pub(crate) fn put_u32le(buf: &mut [u8], value: u32) {
    buf[..4].copy_from_slice(&value.to_le_bytes());
}
