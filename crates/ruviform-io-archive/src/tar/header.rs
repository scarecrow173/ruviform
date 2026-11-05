//! Internal TAR header layout, field parsers, and serializers.
//!
//! These types mirror the on-disk POSIX ustar/GNU structures. They are
//! private implementation details — none of them appear in the public
//! API.

/// TAR block size — every header and data region is a multiple of this.
pub(crate) const BLOCK: usize = 512;

/// Largest value an 11-digit octal field (12-byte field with NUL
/// terminator) can hold. Sizes beyond this need PAX `size=` or GNU
/// base-256.
pub(crate) const OCTAL_MAX: u64 = 0o777_7777_7777;

/// Bytes `0..100`: entry name.
pub(crate) const NAME: std::ops::Range<usize> = 0..100;
/// Bytes `100..108`: mode (octal).
pub(crate) const MODE: std::ops::Range<usize> = 100..108;
/// Bytes `108..116`: uid (octal).
pub(crate) const UID: std::ops::Range<usize> = 108..116;
/// Bytes `116..124`: gid (octal).
pub(crate) const GID: std::ops::Range<usize> = 116..124;
/// Bytes `124..136`: payload size (octal or base-256).
pub(crate) const SIZE: std::ops::Range<usize> = 124..136;
/// Bytes `136..148`: mtime (octal or base-256).
pub(crate) const MTIME: std::ops::Range<usize> = 136..148;
/// Bytes `148..156`: header checksum (octal).
pub(crate) const CHKSUM: std::ops::Range<usize> = 148..156;
/// Byte `156`: typeflag.
pub(crate) const TYPEFLAG: usize = 156;
/// Bytes `157..257`: link target name.
pub(crate) const LINKNAME: std::ops::Range<usize> = 157..257;
/// Bytes `257..263`: format magic.
pub(crate) const MAGIC: std::ops::Range<usize> = 257..263;
/// Bytes `263..265`: format version.
pub(crate) const VERSION: std::ops::Range<usize> = 263..265;
// Bytes 265..297 (uname) and 297..329 (gname) exist on disk but are
// never surfaced or written — not modeled.
/// Bytes `329..337`: device major (octal).
pub(crate) const DEVMAJOR: std::ops::Range<usize> = 329..337;
/// Bytes `337..345`: device minor (octal).
pub(crate) const DEVMINOR: std::ops::Range<usize> = 337..345;
/// Bytes `345..500`: ustar name prefix.
pub(crate) const PREFIX: std::ops::Range<usize> = 345..500;

/// POSIX ustar magic/version pair.
pub(crate) const USTAR_MAGIC: &[u8] = b"ustar\0";
/// GNU tar magic (`"ustar "` with the space).
pub(crate) const GNU_MAGIC: &[u8] = b"ustar ";

// Typeflags surfaced as real entries.
pub(crate) const TYPE_REGULAR: u8 = b'0';
pub(crate) const TYPE_REGULAR_ALT: u8 = 0;
pub(crate) const TYPE_HARDLINK: u8 = b'1';
pub(crate) const TYPE_SYMLINK: u8 = b'2';
pub(crate) const TYPE_DIRECTORY: u8 = b'5';
pub(crate) const TYPE_CONTIGUOUS: u8 = b'7';
// Extension typeflags consumed internally.
pub(crate) const TYPE_PAX: u8 = b'x';
pub(crate) const TYPE_PAX_GLOBAL: u8 = b'g';
pub(crate) const TYPE_PAX_SOLARIS: u8 = b'X';
pub(crate) const TYPE_GNU_LONGNAME: u8 = b'L';
pub(crate) const TYPE_GNU_LONGLINK: u8 = b'K';

/// `true` when every byte of the block is zero (end-of-archive marker /
/// padding block).
pub(crate) fn is_zero_block(block: &[u8; BLOCK]) -> bool {
    block.iter().all(|&b| b == 0)
}

/// Padding needed after `size` payload bytes to reach the next block
/// boundary (`0..=BLOCK-1`).
pub(crate) const fn pad_len(size: u64) -> u64 {
    (BLOCK as u64 - size % BLOCK as u64) % BLOCK as u64
}

/// Computes the header checksum: the unsigned sum of every header byte
/// with the `chksum` field itself treated as spaces.
pub(crate) fn header_checksum(block: &[u8; BLOCK]) -> u32 {
    block
        .iter()
        .enumerate()
        .map(|(i, &b)| {
            if CHKSUM.contains(&i) {
                u32::from(b' ')
            } else {
                u32::from(b)
            }
        })
        .sum()
}

/// Field bytes up to the first NUL (or the full field when none).
/// Trailing garbage after a NUL is ignored per tar convention.
pub(crate) fn text_field(field: &[u8]) -> &[u8] {
    match field.iter().position(|&b| b == 0) {
        Some(end) => &field[..end],
        None => field,
    }
}

/// Parses a numeric header field: GNU base-256 when the first byte's
/// high bit is set, otherwise octal with space/NUL padding. Returns
/// `None` for non-octal, overflowing, or negative values.
pub(crate) fn parse_numeric(field: &[u8]) -> Option<u64> {
    if field.is_empty() {
        return None;
    }
    if field[0] & 0x80 != 0 {
        return parse_base256(field);
    }
    let mut value = 0_u64;
    let mut pos = 0_usize;
    while pos < field.len() && (field[pos] == b' ' || field[pos] == 0) {
        pos += 1;
    }
    let digits_start = pos;
    while pos < field.len() && field[pos].is_ascii_digit() {
        let digit = field[pos] - b'0';
        if digit > 7 {
            return None;
        }
        value = value.checked_mul(8)?.checked_add(u64::from(digit))?;
        pos += 1;
    }
    if pos == digits_start {
        // Field with no digits at all is an empty (zero) field.
        return if field.iter().all(|&b| b == b' ' || b == 0) {
            Some(0)
        } else {
            None
        };
    }
    if field[pos..].iter().any(|&b| b != b' ' && b != 0) {
        return None;
    }
    Some(value)
}

/// GNU base-256 numeric field: first byte's 0x80 bit is the format
/// marker; the remainder is a big-endian binary integer. Negative
/// values (sign bit set) and u64 overflows are rejected.
fn parse_base256(field: &[u8]) -> Option<u64> {
    // Byte 0 carries the 0x80 marker; bit 0x40 is the sign bit.
    if field[0] & 0x40 != 0 {
        return None;
    }
    let mut value = 0_u128;
    for (i, &byte) in field.iter().enumerate() {
        let byte = if i == 0 { byte & 0x3f } else { byte };
        value = value.checked_mul(256)?.checked_add(u128::from(byte))?;
    }
    u64::try_from(value).ok()
}

/// Parsed ustar/GNU header (borrowed slices into the block).
pub(crate) struct Header<'a> {
    /// Raw name field bytes (up to NUL).
    pub name: &'a [u8],
    /// Declared payload size.
    pub size: u64,
    /// Typeflag byte.
    pub typeflag: u8,
    /// Raw linkname field bytes (up to NUL).
    pub linkname: &'a [u8],
    /// Raw ustar prefix field bytes (empty for GNU magic headers).
    pub prefix: &'a [u8],
}

impl<'a> Header<'a> {
    /// Parses a header block: verifies the checksum, accepts ustar or
    /// GNU magic, and validates every numeric field. Returns
    /// `(kind, message)`-style errors via the caller's classification
    /// helpers — this function reports `(is_checksum, detail)`.
    pub(crate) fn parse(block: &'a [u8; BLOCK]) -> Result<Self, HeaderError> {
        let declared = parse_numeric(&block[CHKSUM])
            .ok_or(HeaderError::Malformed("header checksum field is not octal"))?;
        if u64::from(header_checksum(block)) != declared {
            return Err(HeaderError::Checksum);
        }
        let magic = &block[MAGIC];
        let ustar = magic == USTAR_MAGIC;
        let gnu = magic == GNU_MAGIC;
        if !ustar && !gnu {
            return Err(HeaderError::Malformed(
                "header magic is not ustar or GNU tar",
            ));
        }
        // Every numeric field must be well-formed octal/base-256.
        for (range, label) in [
            (MODE, "mode"),
            (UID, "uid"),
            (GID, "gid"),
            (SIZE, "size"),
            (MTIME, "mtime"),
            (DEVMAJOR, "devmajor"),
            (DEVMINOR, "devminor"),
        ] {
            if parse_numeric(&block[range]).is_none() {
                return Err(HeaderError::Malformed(label));
            }
        }
        Ok(Self {
            name: text_field(&block[NAME]),
            size: parse_numeric(&block[SIZE]).unwrap_or(0),
            typeflag: block[TYPEFLAG],
            linkname: text_field(&block[LINKNAME]),
            prefix: if ustar {
                text_field(&block[PREFIX])
            } else {
                &[]
            },
        })
    }
}

/// Header-level parse failure, classified by the caller.
#[derive(Debug)]
pub(crate) enum HeaderError {
    /// Declared checksum does not match the measured header sum.
    Checksum,
    /// Structurally invalid field (non-octal numeric, bad magic).
    Malformed(&'static str),
}

/// One parsed PAX record (`key=value`).
pub(crate) struct PaxRecord<'a> {
    pub key: &'a [u8],
    pub value: &'a [u8],
}

/// Parses a PAX extended-header payload into records. The format is a
/// sequence of `"<len> <key>=<value>\n"` where `len` counts the whole
/// record including its own digits. The record list must consume the
/// payload exactly.
pub(crate) fn parse_pax_records(payload: &[u8]) -> Result<Vec<PaxRecord<'_>>, &'static str> {
    let mut records = Vec::new();
    let mut pos = 0_usize;
    while pos < payload.len() {
        if !payload[pos].is_ascii_digit() {
            return Err("pax record does not start with a length");
        }
        let digits_start = pos;
        while pos < payload.len() && payload[pos].is_ascii_digit() {
            pos += 1;
        }
        let digits = &payload[digits_start..pos];
        let len_str = std::str::from_utf8(digits).map_err(|_| "pax length is not ascii")?;
        let len: usize = len_str.parse().map_err(|_| "pax length overflows")?;
        if pos >= payload.len() || payload[pos] != b' ' {
            return Err("pax record length is not followed by a space");
        }
        if len == 0 || digits_start + len > payload.len() {
            return Err("pax record length exceeds the payload");
        }
        let record = &payload[digits_start..digits_start + len];
        if record[len - 1] != b'\n' {
            return Err("pax record does not end with a newline");
        }
        let body = &record[digits.len() + 1..len - 1];
        let eq = body
            .iter()
            .position(|&b| b == b'=')
            .ok_or("pax record has no =")?;
        if eq == 0 {
            return Err("pax record has an empty key");
        }
        records.push(PaxRecord {
            key: &body[..eq],
            value: &body[eq + 1..],
        });
        pos = digits_start + len;
    }
    Ok(records)
}

/// Builds a `"<len> <key>=<value>\n"` PAX record. The length field
/// counts the whole record, so the digit count is iterated to a fixed
/// point.
pub(crate) fn pax_record(key: &[u8], value: &[u8]) -> Vec<u8> {
    let body_len = key.len() + 1 + value.len() + 1; // key '=' value '\n'
    let mut len = body_len + 2; // at least one digit + space
    loop {
        let digits = len.to_string().len();
        let total = digits + 1 + body_len;
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

/// Writes `value` as zero-padded octal with a NUL terminator into an
/// N-byte field (N-1 digits + NUL). Callers bound the value to what the
/// field can hold.
fn put_octal(field: &mut [u8], value: u64) {
    let digits = field.len() - 1;
    let text = format!("{value:0digits$o}");
    field[..digits].copy_from_slice(text.as_bytes());
    field[digits] = 0;
}

/// Builds a 512-byte ustar header. `header_size` is what goes in the
/// size field (already clamped by the caller when a PAX `size=` record
/// carries the real value).
pub(crate) fn build_header(
    name: &[u8],
    prefix: &[u8],
    mode: u32,
    size: u64,
    mtime: u64,
    typeflag: u8,
    linkname: &[u8],
) -> [u8; BLOCK] {
    let mut block = [0_u8; BLOCK];
    block[NAME.start..NAME.start + name.len()].copy_from_slice(name);
    put_octal(&mut block[MODE], u64::from(mode));
    put_octal(&mut block[UID], 0);
    put_octal(&mut block[GID], 0);
    put_octal(&mut block[SIZE], size);
    put_octal(&mut block[MTIME], mtime);
    // Checksum field holds spaces while the sum is computed.
    block[CHKSUM].copy_from_slice(b"        ");
    block[TYPEFLAG] = typeflag;
    block[LINKNAME.start..LINKNAME.start + linkname.len()].copy_from_slice(linkname);
    block[MAGIC].copy_from_slice(USTAR_MAGIC);
    block[VERSION].copy_from_slice(b"00");
    block[PREFIX.start..PREFIX.start + prefix.len()].copy_from_slice(prefix);
    let sum = header_checksum(&block);
    let text = format!("{sum:06o}\0 ");
    block[CHKSUM].copy_from_slice(text.as_bytes());
    block
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn octal_parse_accepts_gnu_and_posix_forms() {
        assert_eq!(parse_numeric(b"0000644\0"), Some(0o644));
        assert_eq!(parse_numeric(b"644 \0   "), Some(0o644));
        assert_eq!(parse_numeric(b"        "), Some(0));
        assert_eq!(parse_numeric(b"\0\0\0\0\0\0\0\0"), Some(0));
    }

    #[test]
    fn octal_parse_rejects_non_octal() {
        assert_eq!(parse_numeric(b"0000abc\0"), None);
        assert_eq!(parse_numeric(b"123456789"), None);
        assert_eq!(parse_numeric(b"644junk\0\0"), None);
    }

    #[test]
    fn base256_parse() {
        // 0x80 marker + big-endian value 0x1234 in a 12-byte field.
        let mut field = [0_u8; 12];
        field[0] = 0x80;
        field[10] = 0x12;
        field[11] = 0x34;
        assert_eq!(parse_numeric(&field), Some(0x1234));
    }

    #[test]
    fn pax_record_round_trip() {
        let record = pax_record(b"path", b"dir/file.txt");
        let records = parse_pax_records(&record).expect("parse");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].key, b"path");
        assert_eq!(records[0].value, b"dir/file.txt");
    }

    #[test]
    fn pax_record_len_fixed_point() {
        // Lengths whose digit count changes as len grows (e.g. 99->100).
        let value = vec![b'x'; 90];
        let record = pax_record(b"path", &value);
        let records = parse_pax_records(&record).expect("parse");
        assert_eq!(records[0].value.len(), 90);
    }

    #[test]
    fn pax_rejects_malformed() {
        assert!(parse_pax_records(b"abc").is_err());
        assert!(parse_pax_records(b"10 k=v\n").is_err()); // len mismatch
        assert!(parse_pax_records(b"6 =v\n").is_err()); // empty key
        assert!(parse_pax_records(b"6 k=v").is_err()); // no newline / len
    }

    #[test]
    fn header_round_trip() {
        let block = build_header(
            b"name.txt",
            b"",
            0o644,
            11,
            1_700_000_000,
            TYPE_REGULAR,
            b"",
        );
        let header = Header::parse(&block).expect("parse own header");
        assert_eq!(header.name, b"name.txt");
        assert_eq!(header.size, 11);
        assert_eq!(header.typeflag, TYPE_REGULAR);
    }
}
