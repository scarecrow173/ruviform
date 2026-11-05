//! Gzip wire-format contract: header/trailer validation, CRC32/ISIZE
//! verification, single and concatenated members, strict trailing
//! policy, and cross-checks against the flate2 reference backend.

#![cfg(feature = "gzip")]

mod common;

use std::io::{BufReader, Read, Write};

use common::assert_io_kind;
use flate2::Compression;
use flate2::write::GzEncoder;
use ruviform_io_compression::gzip::{DecodeOptions, Decoder, EncodeOptions, Encoder};
use ruviform_io_compression::{CompressionErrorKind, DecodeLimits, DecodeScope};

fn gzip_encode(parts: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = Encoder::new(&mut out, EncodeOptions::default()).expect("encoder");
        for part in parts {
            enc.write_all(part).expect("write");
        }
        enc.finish().expect("finish");
    }
    out
}

fn decode_all(data: &[u8], options: DecodeOptions) -> std::io::Result<Vec<u8>> {
    let mut source = BufReader::new(data);
    let mut dec = Decoder::new(&mut source, options);
    let mut out = Vec::new();
    dec.read_to_end(&mut out).map(|_| out)
}

#[test]
fn emitted_header_is_fixed_and_reproducible() {
    let encoded = gzip_encode(&[b""]);
    assert_eq!(
        &encoded[..10],
        &[0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff],
        "fixed header: magic, deflate, no flags, mtime=0, xfl=0 (level 6), os=unknown"
    );
    // Trailer of an empty member: CRC32(“”)=0, ISIZE=0.
    assert_eq!(&encoded[encoded.len() - 8..], &[0u8; 8]);
}

#[test]
fn roundtrip_through_backend_decoder() {
    let payload = "ruviform gzip interop payload — いろは".as_bytes();
    let encoded = gzip_encode(&[payload]);
    let mut reference = flate2::read::GzDecoder::new(&encoded[..]);
    let mut decoded = Vec::new();
    reference.read_to_end(&mut decoded).expect("flate2 decode");
    assert_eq!(decoded, payload);
}

#[test]
fn decodes_backend_encoder_output() {
    let payload = b"flate2-produced gzip member";
    let mut out = Vec::new();
    {
        let mut enc = GzEncoder::new(&mut out, Compression::new(6));
        enc.write_all(payload).expect("backend write");
        enc.finish().expect("backend finish");
    }
    let decoded = decode_all(&out, DecodeOptions::default()).expect("our decode");
    assert_eq!(decoded, payload);
}

#[test]
fn single_member_receipt_reports_verified_integrity() {
    let payload = b"member payload";
    let encoded = gzip_encode(&[payload]);
    let mut source = BufReader::new(&encoded[..]);
    let mut dec = Decoder::new(&mut source, DecodeOptions::default());
    let mut out = Vec::new();
    dec.read_to_end(&mut out).expect("decode");
    let receipt = dec.finish().expect("finish");
    assert_eq!(out, payload);
    assert_eq!(receipt.decoded_bytes_delivered(), payload.len() as u64);
    assert_eq!(receipt.compressed_bytes_consumed(), encoded.len() as u64);
    assert_eq!(receipt.units_decoded(), 1);
    assert_eq!(receipt.integrity().checked_units(), 1);
    assert_eq!(receipt.integrity().unchecked_units(), 0);
}

#[test]
fn concatenated_members_decode_as_one_stream() {
    let mut concatenated = gzip_encode(&[b"first-"]);
    concatenated.extend_from_slice(&gzip_encode(&[b"second-"]));
    concatenated.extend_from_slice(&gzip_encode(&[b"third"]));

    let mut source = BufReader::new(&concatenated[..]);
    let mut dec = Decoder::new(&mut source, DecodeOptions::default());
    let mut out = Vec::new();
    dec.read_to_end(&mut out).expect("decode");
    let receipt = dec.finish().expect("finish");
    assert_eq!(out, b"first-second-third");
    assert_eq!(receipt.units_decoded(), 3);
    assert_eq!(receipt.integrity().checked_units(), 3);
    assert_eq!(
        receipt.compressed_bytes_consumed(),
        concatenated.len() as u64
    );
}

#[test]
fn concatenated_members_decode_with_flate2_too() {
    // Reference backend agrees our encoder's members concatenate.
    let mut concatenated = gzip_encode(&[b"a"]);
    concatenated.extend_from_slice(&gzip_encode(&[b"b"]));
    let mut reference = flate2::read::MultiGzDecoder::new(&concatenated[..]);
    let mut out = Vec::new();
    reference.read_to_end(&mut out).expect("flate2 multi");
    assert_eq!(out, b"ab");
}

#[test]
fn corrupt_trailer_crc_is_rejected() {
    let mut encoded = gzip_encode(&[b"payload"]);
    let trailer_at = encoded.len() - 8;
    encoded[trailer_at] ^= 0xff;
    let err = decode_all(&encoded, DecodeOptions::default()).unwrap_err();
    assert_io_kind(&err, CompressionErrorKind::ChecksumMismatch);
}

#[test]
fn corrupt_trailer_isize_is_rejected() {
    let mut encoded = gzip_encode(&[b"payload"]);
    let isize_at = encoded.len() - 4;
    encoded[isize_at] ^= 0xff;
    let err = decode_all(&encoded, DecodeOptions::default()).unwrap_err();
    assert_io_kind(&err, CompressionErrorKind::ChecksumMismatch);
}

#[test]
fn truncated_input_is_distinguished() {
    let encoded = gzip_encode(&[b"payload"]);
    for cut in [5usize, encoded.len() - 5, encoded.len() - 1] {
        let err = decode_all(&encoded[..cut], DecodeOptions::default()).unwrap_err();
        assert_io_kind(&err, CompressionErrorKind::TruncatedInput);
    }
}

#[test]
fn invalid_magic_is_rejected() {
    let mut encoded = gzip_encode(&[b"payload"]);
    encoded[0] = 0x00;
    let err = decode_all(&encoded, DecodeOptions::default()).unwrap_err();
    assert_io_kind(&err, CompressionErrorKind::InvalidFormat);
}

#[test]
fn unknown_method_is_rejected() {
    let mut encoded = gzip_encode(&[b"payload"]);
    encoded[2] = 0x07; // CM must be 8 (deflate)
    let err = decode_all(&encoded, DecodeOptions::default()).unwrap_err();
    assert_io_kind(&err, CompressionErrorKind::InvalidFormat);
}

#[test]
fn reserved_header_flags_are_rejected() {
    let mut encoded = gzip_encode(&[b"payload"]);
    encoded[3] |= 0x20; // reserved FLG bit
    let err = decode_all(&encoded, DecodeOptions::default()).unwrap_err();
    assert_io_kind(&err, CompressionErrorKind::InvalidFormat);
}

#[test]
fn trailing_garbage_after_member_is_rejected() {
    let mut encoded = gzip_encode(&[b"payload"]);
    encoded.extend_from_slice(b"junk");
    let err = decode_all(&encoded, DecodeOptions::default()).unwrap_err();
    assert_io_kind(&err, CompressionErrorKind::TrailingData);
}

#[test]
fn lone_magic_byte_at_eof_is_truncated() {
    let mut encoded = gzip_encode(&[b"payload"]);
    encoded.push(0x1f); // start of a member magic, then EOF
    let err = decode_all(&encoded, DecodeOptions::default()).unwrap_err();
    assert_io_kind(&err, CompressionErrorKind::TruncatedInput);
}

#[test]
fn magic_prefix_then_wrong_byte_is_trailing() {
    let mut encoded = gzip_encode(&[b"payload"]);
    encoded.extend_from_slice(&[0x1f, 0x00]);
    let err = decode_all(&encoded, DecodeOptions::default()).unwrap_err();
    assert_io_kind(&err, CompressionErrorKind::TrailingData);
}

#[test]
fn member_count_budget_rejects_extra_members() {
    let mut concatenated = gzip_encode(&[b"a"]);
    concatenated.extend_from_slice(&gzip_encode(&[b"b"]));
    let options = DecodeOptions::new(
        DecodeScope::WholeInput,
        DecodeLimits::new().with_max_units(1),
    );
    let err = decode_all(&concatenated, options).unwrap_err();
    assert_io_kind(&err, CompressionErrorKind::LimitExceeded);
}

#[test]
fn single_unit_leaves_trailing_bytes_unconsumed() {
    let first = gzip_encode(&[b"first"]);
    let rest = {
        let mut tail = gzip_encode(&[b"second"]);
        tail.extend_from_slice(b"-tail");
        tail
    };
    let mut input = first.clone();
    input.extend_from_slice(&rest);

    let mut source = BufReader::new(&input[..]);
    let mut out = Vec::new();
    let receipt = {
        let mut dec = Decoder::new(
            &mut source,
            DecodeOptions::new(DecodeScope::SingleUnit, DecodeLimits::new()),
        );
        dec.read_to_end(&mut out).expect("decode first member");
        dec.finish().expect("verified single member")
    };
    assert_eq!(out, b"first");
    assert_eq!(receipt.compressed_bytes_consumed(), first.len() as u64);

    // Everything after the first member stayed in the caller's BufRead.
    let mut leftover = Vec::new();
    source.read_to_end(&mut leftover).expect("leftover");
    assert_eq!(leftover, rest);
}

#[test]
fn optional_header_fields_are_parsed_and_ignored() {
    // flate2's GzBuilder emits FNAME/FCOMMENT/FEXTRA — our decoder
    // must tolerate them.
    let payload = b"header fields";
    let mut out = Vec::new();
    {
        let mut enc = flate2::GzBuilder::new()
            .filename("payload.bin")
            .comment("interop test")
            .extra(b"xx".to_vec())
            .mtime(1_700_000_000)
            .write(&mut out, Compression::new(6));
        enc.write_all(payload).expect("backend write");
        enc.finish().expect("backend finish");
    }
    assert_ne!(out[3], 0, "flags must mark optional fields present");
    let decoded = decode_all(&out, DecodeOptions::default()).expect("our decode");
    assert_eq!(decoded, payload);
}

#[test]
fn fhcrc_is_verified() {
    // Hand-build a member with FHCRC set: valid CRC16 must pass,
    // corrupted CRC16 must fail.
    let body = {
        let mut raw = Vec::new();
        let mut deflater = flate2::write::DeflateEncoder::new(&mut raw, Compression::new(6));
        deflater.write_all(b"data").unwrap();
        deflater.finish().unwrap();
        raw
    };
    let make_member = |corrupt: bool| {
        let mut header = vec![0x1f, 0x8b, 0x08, 0x02, 0, 0, 0, 0, 0x00, 0xff];
        let mut crc = flate2::Crc::new();
        crc.update(&header);
        let crc16 = (crc.sum() & 0xffff) as u16;
        let field = if corrupt { crc16 ^ 0xffff } else { crc16 };
        header.extend_from_slice(&field.to_le_bytes());
        let mut member = header;
        member.extend_from_slice(&body);
        // Trailer: CRC32("data") + ISIZE=4.
        let mut trailer_crc = flate2::Crc::new();
        trailer_crc.update(b"data");
        member.extend_from_slice(&trailer_crc.sum().to_le_bytes());
        member.extend_from_slice(&4u32.to_le_bytes());
        member
    };

    let good = make_member(false);
    let decoded = decode_all(&good, DecodeOptions::default()).expect("valid fhcrc");
    assert_eq!(decoded, b"data");

    let bad = make_member(true);
    let err = decode_all(&bad, DecodeOptions::default()).unwrap_err();
    assert_io_kind(&err, CompressionErrorKind::ChecksumMismatch);
}

#[test]
fn empty_input_is_truncated() {
    let err = decode_all(&[], DecodeOptions::default()).unwrap_err();
    assert_io_kind(&err, CompressionErrorKind::TruncatedInput);
}
