//! Zstd (RFC 8878) format tests: frame magic/header parsing,
//! single/concatenated/skippable frames, content-checksum
//! accounting, window limits, error normalization, byte-exact
//! consumption and interop against the reference `zstd` crate.

#![cfg(feature = "zstd")]

mod common;

use std::io::{BufReader, Read, Write};

use common::{assert_io_kind, assert_kind};
use ruviform_io_compression::{
    CompressionErrorKind, DecodeLimits, DecodeScope, EncodeLimits,
    zstd::{DecodeOptions, Decoder, EncodeOptions, Encoder, SkippableFramePolicy},
};

/// Reference frame produced by the `zstd` crate's stream encoder —
/// independent of our own framing code.
fn ref_frame(payload: &[u8], checksum: bool) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = zstd::stream::write::Encoder::new(&mut out, 3).expect("ref encoder");
        enc.include_checksum(checksum).expect("checksum flag");
        enc.write_all(payload).expect("ref write");
        enc.finish().expect("ref finish");
    }
    out
}

/// Reference frame with an explicit small window log.
fn ref_frame_window(payload: &[u8], window_log: u32) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = zstd::stream::write::Encoder::new(&mut out, 3).expect("ref encoder");
        enc.window_log(window_log).expect("window log");
        enc.write_all(payload).expect("ref write");
        enc.finish().expect("ref finish");
    }
    out
}

/// Frame produced by *our* encoder.
fn our_frame(payload: &[u8], checksum: bool) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = Encoder::new(
            &mut out,
            EncodeOptions::new(0, checksum, EncodeLimits::new()),
        )
        .expect("encoder");
        enc.write_all(payload).expect("write");
        enc.finish().expect("finish");
    }
    out
}

/// Hand-built skippable frame: magic `0x184D2A50` + LE size + payload.
fn skippable(payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(8 + payload.len());
    frame.extend_from_slice(&0x184D_2A50u32.to_le_bytes());
    frame.extend_from_slice(&u32::try_from(payload.len()).expect("size").to_le_bytes());
    frame.extend_from_slice(payload);
    frame
}

/// Decodes `data` to the end under `options`, returning the payload
/// and the verification receipt.
fn decode_all(
    data: &[u8],
    options: DecodeOptions,
) -> Result<(Vec<u8>, ruviform_io_compression::DecodeReceipt), std::io::Error> {
    let mut source = BufReader::new(data);
    let mut dec = Decoder::new(&mut source, options);
    let mut out = Vec::new();
    dec.read_to_end(&mut out)?;
    let receipt = dec.finish().expect("verified end");
    Ok((out, receipt))
}

fn decode_options() -> DecodeOptions {
    DecodeOptions::default()
}

// ---- AC1: single/concatenated/skippable --------------------------------

#[test]
fn single_frame_roundtrip() {
    let frame = ref_frame(b"hello zstd", false);
    let (out, receipt) = decode_all(&frame, decode_options()).expect("decode");
    assert_eq!(out, b"hello zstd");
    assert_eq!(receipt.units_decoded(), 1);
    assert_eq!(
        usize::try_from(receipt.compressed_bytes_consumed()).unwrap(),
        frame.len(),
        "whole frame must be logically consumed"
    );
    assert_eq!(
        usize::try_from(receipt.decoded_bytes_delivered()).unwrap(),
        out.len()
    );
}

#[test]
fn concatenated_frames_roundtrip() {
    let mut data = ref_frame(b"first ", false);
    data.extend_from_slice(&ref_frame(b"second", false));
    let (out, receipt) = decode_all(&data, decode_options()).expect("decode");
    assert_eq!(out, b"first second");
    assert_eq!(receipt.units_decoded(), 2);
    assert_eq!(
        usize::try_from(receipt.compressed_bytes_consumed()).unwrap(),
        data.len()
    );
}

#[test]
fn many_small_frames_accumulate_exactly() {
    let mut data = Vec::new();
    let mut payload = Vec::new();
    for i in 0..64u8 {
        data.extend_from_slice(&ref_frame(&[i], false));
        payload.push(i);
    }
    let (out, receipt) = decode_all(&data, decode_options()).expect("decode");
    assert_eq!(out, payload);
    assert_eq!(receipt.units_decoded(), 64);
}

#[test]
fn skippable_frames_are_skipped_not_counted() {
    // Skippable before, between and after data frames.
    let mut data = skippable(b"metadata-1");
    data.extend_from_slice(&ref_frame(b"payload", false));
    data.extend_from_slice(&skippable(b"metadata-2"));
    data.extend_from_slice(&ref_frame(b"-more", false));
    data.extend_from_slice(&skippable(b""));
    let (out, receipt) = decode_all(&data, decode_options()).expect("decode");
    assert_eq!(out, b"payload-more");
    assert_eq!(receipt.units_decoded(), 2, "skippable frames are not units");
    assert_eq!(receipt.integrity().unchecked_units(), 2);
    assert_eq!(
        usize::try_from(receipt.compressed_bytes_consumed()).unwrap(),
        data.len()
    );
}

#[test]
fn skippable_only_stream_verifies_empty() {
    let data = skippable(b"only metadata");
    let (out, receipt) = decode_all(&data, decode_options()).expect("decode");
    assert_eq!(out, Vec::<u8>::new());
    assert_eq!(receipt.units_decoded(), 0);
    assert_eq!(
        usize::try_from(receipt.compressed_bytes_consumed()).unwrap(),
        data.len()
    );
}

#[test]
fn skippable_reject_policy_fails_unsupported() {
    let mut data = skippable(b"metadata");
    data.extend_from_slice(&ref_frame(b"payload", false));
    let options = decode_options().with_skippable_frames(SkippableFramePolicy::Reject);
    let mut source = BufReader::new(&data[..]);
    let mut dec = Decoder::new(&mut source, options);
    let mut out = Vec::new();
    assert_io_kind(
        &dec.read_to_end(&mut out).expect_err("skippable rejected"),
        CompressionErrorKind::Unsupported,
    );
}

#[test]
fn single_unit_scope_leaves_later_bytes_untouched() {
    let frame1 = ref_frame(b"first", false);
    let frame2 = ref_frame(b"second", false);
    let mut data = frame1.clone();
    data.extend_from_slice(&frame2);
    let options = DecodeOptions::new(DecodeScope::SingleUnit, DecodeLimits::new());
    let mut source = BufReader::new(&data[..]);
    let mut dec = Decoder::new(&mut source, options);
    let mut out = Vec::new();
    dec.read_to_end(&mut out).expect("decode first");
    let receipt = dec.finish().expect("verified");
    assert_eq!(out, b"first");
    assert_eq!(receipt.units_decoded(), 1);
    assert_eq!(
        usize::try_from(receipt.compressed_bytes_consumed()).unwrap(),
        frame1.len(),
        "only the first frame may be logically consumed"
    );
    let mut rest = Vec::new();
    drop(dec);
    source
        .read_to_end(&mut rest)
        .expect("caller reads remainder");
    assert_eq!(rest, frame2, "the second frame belongs to the caller");
}

// ---- AC2: checksum accounting -------------------------------------------

#[test]
fn checksum_frame_counts_as_checked() {
    let frame = ref_frame(b"verified", true);
    let (_out, receipt) = decode_all(&frame, decode_options()).expect("decode");
    assert_eq!(receipt.integrity().checked_units(), 1);
    assert_eq!(receipt.integrity().unchecked_units(), 0);
}

#[test]
fn no_checksum_frame_counts_as_unchecked() {
    let frame = ref_frame(b"unverified", false);
    let (_out, receipt) = decode_all(&frame, decode_options()).expect("decode");
    assert_eq!(receipt.integrity().checked_units(), 0);
    assert_eq!(receipt.integrity().unchecked_units(), 1);
}

#[test]
fn mixed_checksum_stream_splits_integrity() {
    let mut data = ref_frame(b"checked ", true);
    data.extend_from_slice(&ref_frame(b"unchecked", false));
    let (out, receipt) = decode_all(&data, decode_options()).expect("decode");
    assert_eq!(out, b"checked unchecked");
    assert_eq!(receipt.integrity().checked_units(), 1);
    assert_eq!(receipt.integrity().unchecked_units(), 1);
}

#[test]
fn corrupted_content_checksum_fails() {
    let mut frame = ref_frame(b"checksummed payload", true);
    // The last 4 bytes are the xxHash-64 content checksum — flipping
    // one is a deterministic mismatch.
    let last = frame.len() - 1;
    frame[last] ^= 0xFF;
    let mut source = BufReader::new(&frame[..]);
    let mut dec = Decoder::new(&mut source, decode_options());
    let mut out = Vec::new();
    assert_io_kind(
        &dec.read_to_end(&mut out).expect_err("checksum must fail"),
        CompressionErrorKind::ChecksumMismatch,
    );
    assert_kind(dec.finish(), CompressionErrorKind::InvalidState);
}

// ---- AC3: window/format violations --------------------------------------

#[test]
fn oversized_window_rejected_as_limit_exceeded() {
    // magic + FHD(no sss, no fcs, no did, no checksum) +
    // Window_Descriptor with exponent 31 => window_log 41 > 27.
    let data = [0x28, 0xB5, 0x2F, 0xFD, 0x00, 0xF8];
    let mut source = BufReader::new(&data[..]);
    let mut dec = Decoder::new(&mut source, decode_options());
    let mut out = Vec::new();
    assert_io_kind(
        &dec.read_to_end(&mut out)
            .expect_err("window must be rejected"),
        CompressionErrorKind::LimitExceeded,
    );
}

#[test]
fn window_within_explicit_cap_decodes() {
    let frame = ref_frame_window(b"small window", 10);
    let options = decode_options().with_max_window_log(30);
    let (out, _receipt) = decode_all(&frame, options).expect("decode");
    assert_eq!(out, b"small window");
}

#[test]
fn small_window_cap_rejects_normal_frame() {
    // A default-level frame declares a window log around 20 — cap at
    // 12 to reject it deterministically at header parse.
    let frame = ref_frame(b"larger than the cap allows", false);
    let options = decode_options().with_max_window_log(12);
    let mut source = BufReader::new(&frame[..]);
    let mut dec = Decoder::new(&mut source, options);
    let mut out = Vec::new();
    assert_io_kind(
        &dec.read_to_end(&mut out)
            .expect_err("window cap must reject"),
        CompressionErrorKind::LimitExceeded,
    );
}

#[test]
fn wrong_magic_is_invalid_format() {
    let data = [0x28, 0xB5, 0x2F, 0x00, 0x00];
    let mut source = BufReader::new(&data[..]);
    let mut dec = Decoder::new(&mut source, decode_options());
    let mut out = Vec::new();
    assert_io_kind(
        &dec.read_to_end(&mut out).expect_err("bad magic"),
        CompressionErrorKind::InvalidFormat,
    );
}

#[test]
fn non_magic_first_byte_is_invalid_format() {
    let data = [0xFF, 0x00, 0x00, 0x00];
    let mut source = BufReader::new(&data[..]);
    let mut dec = Decoder::new(&mut source, decode_options());
    let mut out = Vec::new();
    assert_io_kind(
        &dec.read_to_end(&mut out).expect_err("junk"),
        CompressionErrorKind::InvalidFormat,
    );
}

#[test]
fn reserved_fhd_bit_is_invalid_format() {
    // FHD bit 3 is reserved by RFC 8878 and must be zero.
    let data = [0x28, 0xB5, 0x2F, 0xFD, 0x08, 0x00];
    let mut source = BufReader::new(&data[..]);
    let mut dec = Decoder::new(&mut source, decode_options());
    let mut out = Vec::new();
    assert_io_kind(
        &dec.read_to_end(&mut out).expect_err("reserved bit"),
        CompressionErrorKind::InvalidFormat,
    );
}

#[test]
fn dictionary_frame_is_unsupported() {
    // FHD did_flag = 1 declares a 1-byte Dictionary_ID — dictionary
    // frames are outside this crate's contract.
    let data = [0x28, 0xB5, 0x2F, 0xFD, 0x01, 0x00, 0x42];
    let mut source = BufReader::new(&data[..]);
    let mut dec = Decoder::new(&mut source, decode_options());
    let mut out = Vec::new();
    assert_io_kind(
        &dec.read_to_end(&mut out).expect_err("dictionary frame"),
        CompressionErrorKind::Unsupported,
    );
}

#[test]
fn truncated_magic_is_truncated_input() {
    for cut in 0..4 {
        let data = &[0x28, 0xB5, 0x2F, 0xFD][..cut];
        let mut source = BufReader::new(data);
        let mut dec = Decoder::new(&mut source, decode_options());
        let mut out = Vec::new();
        assert_io_kind(
            &dec.read_to_end(&mut out).expect_err("short magic"),
            CompressionErrorKind::TruncatedInput,
        );
    }
}

#[test]
fn truncated_body_is_truncated_input() {
    let frame = ref_frame(b"truncated payload", false);
    for cut in [frame.len() - 4, frame.len() - 2] {
        let mut source = BufReader::new(&frame[..cut]);
        let mut dec = Decoder::new(&mut source, decode_options());
        let mut out = Vec::new();
        assert_io_kind(
            &dec.read_to_end(&mut out).expect_err("short body"),
            CompressionErrorKind::TruncatedInput,
        );
    }
}

#[test]
fn truncated_checksum_is_truncated_input() {
    let frame = ref_frame(b"checksummed", true);
    // Drop 2 of the 4 checksum bytes.
    let cut = frame.len() - 2;
    let mut source = BufReader::new(&frame[..cut]);
    let mut dec = Decoder::new(&mut source, decode_options());
    let mut out = Vec::new();
    assert_io_kind(
        &dec.read_to_end(&mut out).expect_err("short checksum"),
        CompressionErrorKind::TruncatedInput,
    );
}

#[test]
fn trailing_bytes_after_frame_are_rejected() {
    let mut data = ref_frame(b"payload", false);
    data.extend_from_slice(b"junk");
    let mut source = BufReader::new(&data[..]);
    let mut dec = Decoder::new(&mut source, decode_options());
    let mut out = Vec::new();
    assert_io_kind(
        &dec.read_to_end(&mut out).expect_err("trailing data"),
        CompressionErrorKind::TrailingData,
    );
}

// ---- AC4: cumulative budgets --------------------------------------------

#[test]
fn decoded_budget_exact_boundary_succeeds() {
    let frame = ref_frame(b"payload!", false);
    let options = DecodeOptions::new(
        DecodeScope::WholeInput,
        DecodeLimits::new().with_max_decoded_bytes(8),
    );
    let (out, receipt) = decode_all(&frame, options).expect("exact cap");
    assert_eq!(out, b"payload!");
    assert_eq!(receipt.decoded_bytes_delivered(), 8);
}

#[test]
fn decoded_budget_one_over_fails() {
    let frame = ref_frame(b"payload!", false);
    let options = DecodeOptions::new(
        DecodeScope::WholeInput,
        DecodeLimits::new().with_max_decoded_bytes(7),
    );
    let mut source = BufReader::new(&frame[..]);
    let mut dec = Decoder::new(&mut source, options);
    let mut out = Vec::new();
    assert_io_kind(
        &dec.read_to_end(&mut out).expect_err("over cap"),
        CompressionErrorKind::LimitExceeded,
    );
    assert!(out.len() <= 7, "over-cap bytes are never delivered");
}

#[test]
fn compressed_budget_exact_and_over() {
    let frame = ref_frame(b"payload", false);
    let exact = frame.len() as u64;
    let options = DecodeOptions::new(
        DecodeScope::WholeInput,
        DecodeLimits::new().with_max_compressed_bytes(exact),
    );
    decode_all(&frame, options).expect("exact compressed cap");
    let options = DecodeOptions::new(
        DecodeScope::WholeInput,
        DecodeLimits::new().with_max_compressed_bytes(exact - 1),
    );
    let mut source = BufReader::new(&frame[..]);
    let mut dec = Decoder::new(&mut source, options);
    let mut out = Vec::new();
    assert_io_kind(
        &dec.read_to_end(&mut out).expect_err("over compressed cap"),
        CompressionErrorKind::LimitExceeded,
    );
}

#[test]
fn unit_budget_counts_data_frames_only() {
    let mut data = skippable(b"meta");
    data.extend_from_slice(&ref_frame(b"one", false));
    data.extend_from_slice(&ref_frame(b"two", false));
    let exact = DecodeOptions::new(
        DecodeScope::WholeInput,
        DecodeLimits::new().with_max_units(2),
    );
    let (_out, receipt) = decode_all(&data, exact).expect("two units within cap");
    assert_eq!(receipt.units_decoded(), 2);
    let tight = DecodeOptions::new(
        DecodeScope::WholeInput,
        DecodeLimits::new().with_max_units(1),
    );
    let mut source = BufReader::new(&data[..]);
    let mut dec = Decoder::new(&mut source, tight);
    let mut out = Vec::new();
    assert_io_kind(
        &dec.read_to_end(&mut out).expect_err("third unit over cap"),
        CompressionErrorKind::LimitExceeded,
    );
}

// ---- AC5: observation / boundaries --------------------------------------

#[test]
fn compressed_consumption_ignores_caller_prefetch() {
    let frame1 = ref_frame(b"one", false);
    let frame2 = ref_frame(b"two", false);
    let mut data = frame1.clone();
    data.extend_from_slice(&frame2);
    // A BufReader with a huge capacity physically prefetches the
    // whole input; the receipt must still report logical consumption.
    let mut source = BufReader::with_capacity(64 * 1024, &data[..]);
    let mut dec = Decoder::new(
        &mut source,
        DecodeOptions::new(DecodeScope::SingleUnit, DecodeLimits::new()),
    );
    let mut out = Vec::new();
    dec.read_to_end(&mut out).expect("decode");
    let receipt = dec.finish().expect("verified");
    assert_eq!(
        usize::try_from(receipt.compressed_bytes_consumed()).unwrap(),
        frame1.len()
    );
}

#[test]
fn encoder_frame_has_expected_magic() {
    let frame = our_frame(b"payload", false);
    assert_eq!(&frame[..4], &[0x28, 0xB5, 0x2F, 0xFD]);
}

#[test]
fn encoder_checksum_flag_reaches_the_frame() {
    let frame = our_frame(b"payload", true);
    // FHD bit 2 = checksum flag; the frame ends with 4 checksum bytes.
    assert_eq!(frame[4] & 0x04, 0x04);
    let (out, receipt) = decode_all(&frame, decode_options()).expect("decode");
    assert_eq!(out, b"payload");
    assert_eq!(receipt.integrity().checked_units(), 1);
}

#[test]
fn our_encoder_output_decodes_with_reference_crate() {
    for checksum in [false, true] {
        let frame = our_frame(b"interop payload", checksum);
        let decoded = zstd::stream::decode_all(&frame[..]).expect("ref decode");
        assert_eq!(decoded, b"interop payload");
    }
}

#[test]
fn reference_frames_decode_with_our_decoder() {
    // bulk::compress emits an FCS-carrying frame; the stream encoder's
    // frame has no FCS — both shapes must decode.
    let bulk = zstd::bulk::compress(b"bulk payload", 3).expect("bulk");
    let (out, _) = decode_all(&bulk, decode_options()).expect("decode bulk");
    assert_eq!(out, b"bulk payload");
    let streamed = ref_frame(b"streamed payload", false);
    let (out, _) = decode_all(&streamed, decode_options()).expect("decode stream");
    assert_eq!(out, b"streamed payload");
}

#[test]
fn roundtrip_through_short_fill() {
    // A 3-byte fill_buf must not corrupt magic/header boundary
    // detection (the boundary reads fixed-size fields explicitly).
    let mut data = ref_frame(b"short fill payload", true);
    data.extend_from_slice(&skippable(b"meta"));
    data.extend_from_slice(&ref_frame(b" more", false));
    let mut source = common::ShortFill::new(&data[..], 3);
    let mut dec = Decoder::new(&mut source, decode_options());
    let mut out = Vec::new();
    dec.read_to_end(&mut out).expect("decode");
    let receipt = dec.finish().expect("verified");
    assert_eq!(out, b"short fill payload more");
    assert_eq!(receipt.units_decoded(), 2);
    assert_eq!(receipt.integrity().checked_units(), 1);
    assert_eq!(receipt.integrity().unchecked_units(), 1);
}

#[test]
fn level_out_of_range_rejected() {
    let mut out = Vec::new();
    assert_kind(
        Encoder::new(
            &mut out,
            EncodeOptions::new(100, false, EncodeLimits::new()),
        ),
        CompressionErrorKind::InvalidInput,
    );
    assert!(out.is_empty(), "a rejected encoder emits nothing");
}

#[test]
fn negative_fast_level_roundtrip() {
    let mut out = Vec::new();
    {
        let mut enc = Encoder::new(&mut out, EncodeOptions::new(-3, false, EncodeLimits::new()))
            .expect("encoder");
        enc.write_all(b"fast payload").expect("write");
        enc.finish().expect("finish");
    }
    let (decoded, _) = decode_all(&out, decode_options()).expect("decode");
    assert_eq!(decoded, b"fast payload");
}
