//! Raw-DEFLATE wire-format contract: RFC 1951 only — no zlib
//! container, no checksum, single stream, strict trailing policy.

#![cfg(feature = "deflate")]

mod common;

use std::io::{BufReader, Read, Write};

use common::assert_io_kind;
use ruviform_io_compression::deflate::{DecodeOptions, Decoder, EncodeOptions, Encoder};
use ruviform_io_compression::{CompressionErrorKind, DecodeLimits, DecodeScope, EncodeLimits};

fn deflate_encode(parts: &[&[u8]], level: u32) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = Encoder::new(&mut out, EncodeOptions::new(level, EncodeLimits::new()))
            .expect("encoder");
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
fn roundtrip_across_levels() {
    let payload: Vec<u8> = (0u32..4096).map(|i| (i % 251) as u8).collect();
    for level in [0u32, 1, 6, 9] {
        let encoded = deflate_encode(&[&payload], level);
        let decoded = decode_all(&encoded, DecodeOptions::default())
            .unwrap_or_else(|e| panic!("level {level}: {e}"));
        assert_eq!(decoded, payload, "level {level}");
    }
}

#[test]
fn roundtrip_through_backend_decoder() {
    let payload = b"raw deflate interop";
    let encoded = deflate_encode(&[payload], 6);
    let mut reference = flate2::read::DeflateDecoder::new(&encoded[..]);
    let mut decoded = Vec::new();
    reference.read_to_end(&mut decoded).expect("flate2 decode");
    assert_eq!(decoded, payload);
}

#[test]
fn decodes_backend_encoder_output() {
    let payload = b"flate2-produced raw deflate";
    let mut raw = Vec::new();
    {
        let mut enc = flate2::write::DeflateEncoder::new(&mut raw, flate2::Compression::new(6));
        enc.write_all(payload).expect("backend write");
        enc.finish().expect("backend finish");
    }
    let decoded = decode_all(&raw, DecodeOptions::default()).expect("our decode");
    assert_eq!(decoded, payload);
}

#[test]
fn output_is_not_zlib_or_gzip_framed() {
    let encoded = deflate_encode(&[b"check"], 6);
    // zlib streams start `0x78 ..`; gzip starts `1f 8b`.
    assert_ne!(encoded[0], 0x78, "raw deflate must not be zlib-framed");
    assert_ne!(&encoded[..2], &[0x1f, 0x8b], "raw deflate must not be gzip");
}

#[test]
fn zlib_wrapped_input_is_rejected() {
    let mut raw = Vec::new();
    {
        let mut enc = flate2::write::ZlibEncoder::new(&mut raw, flate2::Compression::new(6));
        enc.write_all(b"zlib").unwrap();
        enc.finish().unwrap();
    }
    let err = decode_all(&raw, DecodeOptions::default()).unwrap_err();
    assert!(
        matches!(
            common::compression_kind(&err),
            Some(CompressionErrorKind::CorruptData | CompressionErrorKind::TrailingData)
        ),
        "zlib container must not decode as raw deflate: {err:?}"
    );
}

#[test]
fn gzip_wrapped_input_is_rejected() {
    let mut raw = Vec::new();
    {
        let mut enc = flate2::write::GzEncoder::new(&mut raw, flate2::Compression::new(6));
        enc.write_all(b"gzip").unwrap();
        enc.finish().unwrap();
    }
    let err = decode_all(&raw, DecodeOptions::default()).unwrap_err();
    assert!(
        common::compression_kind(&err).is_some(),
        "gzip container must not decode as raw deflate: {err:?}"
    );
}

#[test]
fn receipt_reports_checksum_free_integrity() {
    let payload = b"integrity accounting";
    let encoded = deflate_encode(&[payload], 6);
    let mut source = BufReader::new(&encoded[..]);
    let mut dec = Decoder::new(&mut source, DecodeOptions::default());
    let mut out = Vec::new();
    dec.read_to_end(&mut out).expect("decode");
    let receipt = dec.finish().expect("finish");
    assert_eq!(out, payload);
    assert_eq!(receipt.units_decoded(), 1);
    assert_eq!(receipt.integrity().checked_units(), 0);
    assert_eq!(receipt.integrity().unchecked_units(), 1);
    assert_eq!(receipt.compressed_bytes_consumed(), encoded.len() as u64);
}

#[test]
fn trailing_bytes_after_stream_end_are_rejected() {
    let mut encoded = deflate_encode(&[b"payload"], 6);
    encoded.extend_from_slice(b"junk");
    let err = decode_all(&encoded, DecodeOptions::default()).unwrap_err();
    assert_io_kind(&err, CompressionErrorKind::TrailingData);
}

#[test]
fn single_unit_leaves_trailing_bytes_unconsumed() {
    let stream = deflate_encode(&[b"payload"], 6);
    let mut input = stream.clone();
    input.extend_from_slice(b"-caller-bytes");

    let mut source = BufReader::new(&input[..]);
    let mut out = Vec::new();
    let receipt = {
        let mut dec = Decoder::new(
            &mut source,
            DecodeOptions::new(DecodeScope::SingleUnit, DecodeLimits::new()),
        );
        dec.read_to_end(&mut out).expect("decode");
        dec.finish().expect("verified stream")
    };
    assert_eq!(out, b"payload");
    assert_eq!(receipt.compressed_bytes_consumed(), stream.len() as u64);

    let mut leftover = Vec::new();
    source.read_to_end(&mut leftover).expect("leftover");
    assert_eq!(leftover, b"-caller-bytes");
}

#[test]
fn truncated_input_is_distinguished() {
    let encoded = deflate_encode(&[b"payload"], 6);
    let err = decode_all(&encoded[..encoded.len() - 2], DecodeOptions::default()).unwrap_err();
    assert_io_kind(&err, CompressionErrorKind::TruncatedInput);
}

#[test]
fn empty_input_is_truncated() {
    let err = decode_all(&[], DecodeOptions::default()).unwrap_err();
    assert_io_kind(&err, CompressionErrorKind::TruncatedInput);
}
