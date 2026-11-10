//! Decoder lifecycle contract: verified-end-only `finish`,
//! zero-length reads that prove nothing, `NotExhausted` for
//! undelivered output, terminal `Failed`, and cumulative budgets.

#![cfg(any(feature = "gzip", feature = "deflate", feature = "zstd"))]

mod common;

use std::io::{BufReader, Read, Write};

use common::{FailingReader, ShortFill, assert_io_kind, assert_kind};
use ruviform_io_compression::{CompressionErrorKind, DecodeLimits, DecodeScope};

#[cfg(feature = "gzip")]
mod gzip {
    use super::*;
    use ruviform_io_compression::gzip::{DecodeOptions, Decoder, EncodeOptions, Encoder};

    fn encoded(payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = Encoder::new(&mut out, EncodeOptions::default()).expect("encoder");
            enc.write_all(payload).expect("write");
            enc.finish().expect("finish");
        }
        out
    }

    #[test]
    fn zero_length_read_never_proves_eof() {
        let data = encoded(b"payload");
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        assert_eq!(dec.read(&mut []).expect("empty read"), 0);
        assert_eq!(dec.read(&mut []).expect("empty read"), 0);
        // Still not exhausted — a real read delivers output.
        let mut buf = [0u8; 16];
        let n = dec.read(&mut buf).expect("read");
        assert_eq!(&buf[..n], b"payload");
    }

    #[test]
    fn finish_before_drain_is_not_exhausted_and_preserves_state() {
        let data = encoded(b"payload");
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        // No reads yet — everything is still deliverable.
        assert_kind(dec.finish(), CompressionErrorKind::NotExhausted);
        // The failed call consumed nothing — decoding proceeds.
        let mut out = Vec::new();
        dec.read_to_end(&mut out).expect("decode");
        assert_eq!(out, b"payload");
        dec.finish().expect("verified end now reachable");
    }

    #[test]
    fn finish_mid_output_is_not_exhausted() {
        let data = encoded(b"0123456789");
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut buf = [0u8; 4];
        dec.read_exact(&mut buf).expect("partial read");
        assert_kind(dec.finish(), CompressionErrorKind::NotExhausted);
    }

    #[test]
    fn finish_after_verified_end_returns_receipt() {
        let data = encoded(b"payload");
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut out = Vec::new();
        dec.read_to_end(&mut out).expect("decode");
        let receipt = dec.finish().expect("finish");
        assert_eq!(receipt.decoded_bytes_delivered(), 7);
        assert_eq!(receipt.compressed_bytes_consumed(), data.len() as u64);
    }

    #[test]
    fn read_past_verified_end_keeps_returning_zero() {
        let data = encoded(b"x");
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut buf = [0u8; 8];
        assert_eq!(dec.read(&mut buf).expect("read"), 1);
        assert_eq!(dec.read(&mut buf).expect("eof"), 0);
        assert_eq!(dec.read(&mut buf).expect("eof again"), 0);
        dec.finish().expect("finish");
    }

    #[test]
    fn failed_decoder_rejects_further_reads_and_finish() {
        let mut data = encoded(b"payload");
        data[0] = 0x00; // break the magic
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut buf = [0u8; 8];
        assert_io_kind(
            &dec.read(&mut buf).expect_err("invalid input"),
            CompressionErrorKind::InvalidFormat,
        );
        assert_io_kind(
            &dec.read(&mut buf).expect_err("reuse rejected"),
            CompressionErrorKind::InvalidState,
        );
        assert_kind(dec.finish(), CompressionErrorKind::InvalidState);
    }

    #[test]
    fn consumed_counts_logical_bytes_not_prefetch() {
        let first = encoded(b"first");
        let mut input = first.clone();
        input.extend_from_slice(&encoded(b"second"));
        // A small BufReader prefetches at most 8 bytes at a time; a
        // large one grabs everything. Logical consumption must match
        // the member size either way.
        for capacity in [8usize, 1 << 16] {
            let mut source = BufReader::with_capacity(capacity, &input[..]);
            let mut dec = Decoder::new(
                &mut source,
                DecodeOptions::new(DecodeScope::SingleUnit, DecodeLimits::new()),
            );
            let mut out = Vec::new();
            dec.read_to_end(&mut out).expect("decode");
            let receipt = dec.finish().expect("finish");
            assert_eq!(
                receipt.compressed_bytes_consumed(),
                first.len() as u64,
                "capacity {capacity}: consumption is logical, not prefetch"
            );
        }
    }

    #[test]
    fn short_fill_bufread_roundtrip() {
        // Two members decoded through a BufRead that yields a single
        // byte per fill_buf — member-boundary detection must survive
        // minimal lookahead.
        let mut data = encoded(b"ab");
        data.extend_from_slice(&encoded(b"cd"));
        let mut source = ShortFill::new(&data[..], 1);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut out = Vec::new();
        dec.read_to_end(&mut out).expect("decode");
        assert_eq!(out, b"abcd");
        let receipt = dec.finish().expect("finish");
        assert_eq!(receipt.units_decoded(), 2);
    }

    #[test]
    fn decoded_budget_allows_exact_and_rejects_one_over() {
        let payload = b"0123456789";
        let data = encoded(payload);
        // Exact fit succeeds — the cap is inclusive.
        let options = DecodeOptions::new(
            DecodeScope::WholeInput,
            DecodeLimits::new().with_max_decoded_bytes(payload.len() as u64),
        );
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, options);
        let mut out = Vec::new();
        dec.read_to_end(&mut out).expect("exact budget decodes");
        assert_eq!(out, payload);
        dec.finish().expect("finish");

        // One byte under the cap fails — hitting the bound is not EOF.
        // The at-limit bytes are still delivered; the violation
        // surfaces on the next read, before the extra byte leaves.
        let options = DecodeOptions::new(
            DecodeScope::WholeInput,
            DecodeLimits::new().with_max_decoded_bytes(payload.len() as u64 - 1),
        );
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, options);
        let mut out = vec![0u8; 64];
        let n = dec.read(&mut out).expect("at-limit bytes still deliver");
        assert_eq!(n, payload.len() - 1);
        let err = dec.read(&mut out).expect_err("one over the budget");
        assert_io_kind(&err, CompressionErrorKind::LimitExceeded);
    }

    #[test]
    fn compressed_budget_rejects_overrun() {
        let data = encoded(b"payload");
        let options = DecodeOptions::new(
            DecodeScope::WholeInput,
            DecodeLimits::new().with_max_compressed_bytes(data.len() as u64 - 1),
        );
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, options);
        let mut out = Vec::new();
        let err = dec
            .read_to_end(&mut out)
            .expect_err("over compressed budget");
        assert_io_kind(&err, CompressionErrorKind::LimitExceeded);
    }

    #[test]
    fn cumulative_decoded_budget_spans_members() {
        let mut data = encoded(b"aaa");
        data.extend_from_slice(&encoded(b"bbb"));
        let options = DecodeOptions::new(
            DecodeScope::WholeInput,
            DecodeLimits::new().with_max_decoded_bytes(4),
        );
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, options);
        let mut out = Vec::new();
        let err = dec.read_to_end(&mut out).expect_err("cumulative cap");
        assert_io_kind(&err, CompressionErrorKind::LimitExceeded);
        // 4 bytes were delivered before the cap tripped.
        assert_eq!(out, b"aaab");
    }

    #[test]
    fn unit_budget_rejects_second_member() {
        let mut data = encoded(b"a");
        data.extend_from_slice(&encoded(b"b"));
        let options = DecodeOptions::new(
            DecodeScope::WholeInput,
            DecodeLimits::new().with_max_units(1),
        );
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, options);
        let mut out = Vec::new();
        let err = dec.read_to_end(&mut out).expect_err("unit cap");
        assert_io_kind(&err, CompressionErrorKind::LimitExceeded);
    }

    #[test]
    fn finish_is_idempotent_after_verified_end() {
        let data = encoded(b"payload");
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut out = Vec::new();
        dec.read_to_end(&mut out).expect("decode");
        let first = dec.finish().expect("finish");
        let second = dec.finish().expect("finish again");
        assert_eq!(first, second);
    }

    #[test]
    fn chained_decoder_resumes_at_member_boundary() {
        // `SingleUnit` must leave the second member readable *as a
        // stream* — a fresh decoder on the same `BufRead` verifies it
        // independently of the first decoder's receipt.
        let mut data = encoded(b"first");
        data.extend_from_slice(&encoded(b"second"));
        let mut source = BufReader::new(&data[..]);
        let single = || DecodeOptions::new(DecodeScope::SingleUnit, DecodeLimits::new());
        {
            let mut dec = Decoder::new(&mut source, single());
            let mut out = Vec::new();
            dec.read_to_end(&mut out).expect("first member");
            dec.finish().expect("first verified");
            assert_eq!(out, b"first");
        }
        let mut dec = Decoder::new(&mut source, single());
        let mut out = Vec::new();
        dec.read_to_end(&mut out).expect("second member");
        dec.finish().expect("second verified");
        assert_eq!(out, b"second");
    }

    #[test]
    fn read_error_latches_failed() {
        // A mid-stream source error surfaces as `Io` and latches the
        // decoder exactly like a format failure.
        let data = encoded(b"payload payload payload");
        let fail_at = data.len() / 2;
        let mut source = FailingReader::new(&data[..], fail_at);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut out = Vec::new();
        let err = dec.read_to_end(&mut out).expect_err("source error");
        assert_io_kind(&err, CompressionErrorKind::Io);
        let mut buf = [0u8; 8];
        assert_io_kind(
            &dec.read(&mut buf).expect_err("reuse rejected"),
            CompressionErrorKind::InvalidState,
        );
        assert_kind(dec.finish(), CompressionErrorKind::InvalidState);
    }

    #[test]
    fn decoded_budget_enforced_on_high_ratio_payload() {
        // SEC-01 zip-bomb shape: ~1 MiB of zeros compresses to ~1 KiB.
        // The decoded-byte cap must trip on real expansion — not just
        // on tiny fixtures — and exactly-at-cap must still succeed.
        let payload = vec![0u8; 1 << 20];
        let data = encoded(&payload);
        assert!(
            data.len() < payload.len() / 100,
            "fixture must actually compress"
        );

        let under = DecodeOptions::new(
            DecodeScope::WholeInput,
            DecodeLimits::new().with_max_decoded_bytes(payload.len() as u64 - 1),
        );
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, under);
        let mut out = Vec::new();
        let err = dec.read_to_end(&mut out).expect_err("one byte under cap");
        assert_io_kind(&err, CompressionErrorKind::LimitExceeded);
        assert_eq!(out.len(), payload.len() - 1, "cap bytes still deliver");

        let exact = DecodeOptions::new(
            DecodeScope::WholeInput,
            DecodeLimits::new().with_max_decoded_bytes(payload.len() as u64),
        );
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, exact);
        let mut out = Vec::new();
        dec.read_to_end(&mut out).expect("exact cap decodes");
        assert_eq!(out.len(), payload.len());
        dec.finish().expect("finish");
    }
}

#[cfg(feature = "deflate")]
mod deflate {
    use super::*;
    use ruviform_io_compression::EncodeLimits;
    use ruviform_io_compression::deflate::{DecodeOptions, Decoder, EncodeOptions, Encoder};

    fn encoded(payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = Encoder::new(&mut out, EncodeOptions::new(6, EncodeLimits::new()))
                .expect("encoder");
            enc.write_all(payload).expect("write");
            enc.finish().expect("finish");
        }
        out
    }

    #[test]
    fn zero_length_read_never_proves_eof() {
        let data = encoded(b"payload");
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        assert_eq!(dec.read(&mut []).expect("empty read"), 0);
        assert_kind(dec.finish(), CompressionErrorKind::NotExhausted);
        let mut buf = [0u8; 16];
        let n = dec.read(&mut buf).expect("read");
        assert_eq!(&buf[..n], b"payload");
    }

    #[test]
    fn short_fill_bufread_roundtrip() {
        let data = encoded(b"abcdef");
        let mut source = ShortFill::new(&data[..], 1);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut out = Vec::new();
        dec.read_to_end(&mut out).expect("decode");
        assert_eq!(out, b"abcdef");
        dec.finish().expect("finish");
    }

    #[test]
    fn decoded_budget_exact_and_one_over() {
        let payload = b"budget-check";
        let data = encoded(payload);
        let exact = DecodeOptions::new(
            DecodeScope::WholeInput,
            DecodeLimits::new().with_max_decoded_bytes(payload.len() as u64),
        );
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, exact);
        let mut out = Vec::new();
        dec.read_to_end(&mut out).expect("exact budget");
        assert_eq!(out, payload);

        let under = DecodeOptions::new(
            DecodeScope::WholeInput,
            DecodeLimits::new().with_max_decoded_bytes(payload.len() as u64 - 1),
        );
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, under);
        let mut buf = [0u8; 64];
        let n = dec.read(&mut buf).expect("at-limit bytes still deliver");
        assert_eq!(n, payload.len() - 1);
        let err = dec.read(&mut buf).expect_err("over budget");
        assert_io_kind(&err, CompressionErrorKind::LimitExceeded);
    }

    #[test]
    fn finish_mid_output_is_not_exhausted() {
        let data = encoded(b"0123456789");
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut buf = [0u8; 4];
        dec.read_exact(&mut buf).expect("partial read");
        assert_kind(dec.finish(), CompressionErrorKind::NotExhausted);
        // The failed call consumed nothing — decoding proceeds with
        // the 6 bytes not yet delivered by the partial read.
        let mut out = Vec::new();
        dec.read_to_end(&mut out).expect("decode");
        assert_eq!(out.len(), 6);
        dec.finish().expect("verified end now reachable");
    }

    #[test]
    fn finish_is_idempotent_after_verified_end() {
        let data = encoded(b"payload");
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut out = Vec::new();
        dec.read_to_end(&mut out).expect("decode");
        let first = dec.finish().expect("finish");
        let second = dec.finish().expect("finish again");
        assert_eq!(first, second);
    }

    #[test]
    fn chained_decoder_decodes_next_stream() {
        // Two finished streams concatenated: `SingleUnit` leaves the
        // second stream's bytes in the caller's `BufRead`, where a
        // fresh decoder verifies it independently.
        let mut data = encoded(b"first");
        data.extend_from_slice(&encoded(b"second"));
        let mut source = BufReader::new(&data[..]);
        let single = || DecodeOptions::new(DecodeScope::SingleUnit, DecodeLimits::new());
        {
            let mut dec = Decoder::new(&mut source, single());
            let mut out = Vec::new();
            dec.read_to_end(&mut out).expect("first stream");
            dec.finish().expect("first verified");
            assert_eq!(out, b"first");
        }
        let mut dec = Decoder::new(&mut source, single());
        let mut out = Vec::new();
        dec.read_to_end(&mut out).expect("second stream");
        dec.finish().expect("second verified");
        assert_eq!(out, b"second");
    }

    #[test]
    fn read_error_latches_failed() {
        let data = encoded(b"payload payload payload");
        let fail_at = data.len() / 2;
        let mut source = FailingReader::new(&data[..], fail_at);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut out = Vec::new();
        let err = dec.read_to_end(&mut out).expect_err("source error");
        assert_io_kind(&err, CompressionErrorKind::Io);
        let mut buf = [0u8; 8];
        assert_io_kind(
            &dec.read(&mut buf).expect_err("reuse rejected"),
            CompressionErrorKind::InvalidState,
        );
        assert_kind(dec.finish(), CompressionErrorKind::InvalidState);
    }

    #[test]
    fn decoded_budget_enforced_on_high_ratio_payload() {
        let payload = vec![0u8; 1 << 20];
        let data = encoded(&payload);
        assert!(
            data.len() < payload.len() / 100,
            "fixture must actually compress"
        );

        let under = DecodeOptions::new(
            DecodeScope::WholeInput,
            DecodeLimits::new().with_max_decoded_bytes(payload.len() as u64 - 1),
        );
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, under);
        let mut out = Vec::new();
        let err = dec.read_to_end(&mut out).expect_err("one byte under cap");
        assert_io_kind(&err, CompressionErrorKind::LimitExceeded);
        assert_eq!(out.len(), payload.len() - 1, "cap bytes still deliver");

        let exact = DecodeOptions::new(
            DecodeScope::WholeInput,
            DecodeLimits::new().with_max_decoded_bytes(payload.len() as u64),
        );
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, exact);
        let mut out = Vec::new();
        dec.read_to_end(&mut out).expect("exact cap decodes");
        assert_eq!(out.len(), payload.len());
        dec.finish().expect("finish");
    }
}

#[cfg(feature = "zstd")]
mod zstd {
    use super::*;
    use ruviform_io_compression::EncodeLimits;
    use ruviform_io_compression::zstd::{DecodeOptions, Decoder, EncodeOptions, Encoder};

    fn encoded(payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = Encoder::new(&mut out, EncodeOptions::new(0, false, EncodeLimits::new()))
                .expect("encoder");
            enc.write_all(payload).expect("write");
            enc.finish().expect("finish");
        }
        out
    }

    #[test]
    fn zero_length_read_never_proves_eof() {
        let data = encoded(b"payload");
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        assert_eq!(dec.read(&mut []).expect("empty read"), 0);
        assert_eq!(dec.read(&mut []).expect("empty read"), 0);
        let mut buf = [0u8; 16];
        let n = dec.read(&mut buf).expect("read");
        assert_eq!(&buf[..n], b"payload");
    }

    #[test]
    fn finish_before_drain_is_not_exhausted_and_preserves_state() {
        let data = encoded(b"payload");
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        assert_kind(dec.finish(), CompressionErrorKind::NotExhausted);
        let mut out = Vec::new();
        dec.read_to_end(&mut out).expect("decode");
        assert_eq!(out, b"payload");
        dec.finish().expect("verified end now reachable");
    }

    #[test]
    fn short_fill_bufread_roundtrip() {
        let data = encoded(b"abcdef");
        let mut source = ShortFill::new(&data[..], 1);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut out = Vec::new();
        dec.read_to_end(&mut out).expect("decode");
        assert_eq!(out, b"abcdef");
        dec.finish().expect("finish");
    }

    #[test]
    fn failed_decoder_rejects_reuse() {
        // Corrupt the checksum-flagged frame tail to force a terminal
        // failure, then prove the latch.
        let mut data = Vec::new();
        {
            let mut enc = Encoder::new(&mut data, EncodeOptions::new(0, true, EncodeLimits::new()))
                .expect("encoder");
            enc.write_all(b"payload").expect("write");
            enc.finish().expect("finish");
        }
        let last = data.len() - 1;
        data[last] ^= 0xFF;
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut out = Vec::new();
        dec.read_to_end(&mut out).expect_err("corrupt frame");
        let mut buf = [0u8; 8];
        assert_io_kind(
            &dec.read(&mut buf).expect_err("reuse rejected"),
            CompressionErrorKind::InvalidState,
        );
        assert_kind(dec.finish(), CompressionErrorKind::InvalidState);
    }

    #[test]
    fn decoded_budget_exact_and_one_over() {
        let payload = b"budget-check";
        let data = encoded(payload);
        let exact = DecodeOptions::new(
            DecodeScope::WholeInput,
            DecodeLimits::new().with_max_decoded_bytes(payload.len() as u64),
        );
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, exact);
        let mut out = Vec::new();
        dec.read_to_end(&mut out).expect("exact budget");
        assert_eq!(out, payload);

        let under = DecodeOptions::new(
            DecodeScope::WholeInput,
            DecodeLimits::new().with_max_decoded_bytes(payload.len() as u64 - 1),
        );
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, under);
        let mut buf = [0u8; 64];
        let n = dec.read(&mut buf).expect("at-limit bytes still deliver");
        assert_eq!(n, payload.len() - 1);
        let err = dec.read(&mut buf).expect_err("over budget");
        assert_io_kind(&err, CompressionErrorKind::LimitExceeded);
    }

    #[test]
    fn decoded_budget_is_cumulative_across_frames() {
        let mut data = encoded(b"aaa");
        data.extend_from_slice(&encoded(b"bbb"));
        let options = DecodeOptions::new(
            DecodeScope::WholeInput,
            DecodeLimits::new().with_max_decoded_bytes(4),
        );
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, options);
        let mut out = Vec::new();
        let err = dec.read_to_end(&mut out).expect_err("cumulative cap");
        assert_io_kind(&err, CompressionErrorKind::LimitExceeded);
        assert_eq!(out, b"aaab");
    }

    #[test]
    fn finish_mid_output_is_not_exhausted() {
        let data = encoded(b"0123456789");
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut buf = [0u8; 4];
        dec.read_exact(&mut buf).expect("partial read");
        assert_kind(dec.finish(), CompressionErrorKind::NotExhausted);
        // The failed call consumed nothing — decoding proceeds with
        // the 6 bytes not yet delivered by the partial read.
        let mut out = Vec::new();
        dec.read_to_end(&mut out).expect("decode");
        assert_eq!(out.len(), 6);
        dec.finish().expect("verified end now reachable");
    }

    #[test]
    fn finish_is_idempotent_after_verified_end() {
        let data = encoded(b"payload");
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut out = Vec::new();
        dec.read_to_end(&mut out).expect("decode");
        let first = dec.finish().expect("finish");
        let second = dec.finish().expect("finish again");
        assert_eq!(first, second);
    }

    #[test]
    fn chained_decoder_decodes_next_frame() {
        // Two finished frames concatenated: `SingleUnit` leaves the
        // second frame's bytes in the caller's `BufRead`, where a
        // fresh decoder verifies it independently.
        let mut data = encoded(b"first");
        data.extend_from_slice(&encoded(b"second"));
        let mut source = BufReader::new(&data[..]);
        let single = || DecodeOptions::new(DecodeScope::SingleUnit, DecodeLimits::new());
        {
            let mut dec = Decoder::new(&mut source, single());
            let mut out = Vec::new();
            dec.read_to_end(&mut out).expect("first frame");
            dec.finish().expect("first verified");
            assert_eq!(out, b"first");
        }
        let mut dec = Decoder::new(&mut source, single());
        let mut out = Vec::new();
        dec.read_to_end(&mut out).expect("second frame");
        dec.finish().expect("second verified");
        assert_eq!(out, b"second");
    }

    #[test]
    fn read_error_latches_failed() {
        let data = encoded(b"payload payload payload");
        let fail_at = data.len() / 2;
        let mut source = FailingReader::new(&data[..], fail_at);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut out = Vec::new();
        let err = dec.read_to_end(&mut out).expect_err("source error");
        assert_io_kind(&err, CompressionErrorKind::Io);
        let mut buf = [0u8; 8];
        assert_io_kind(
            &dec.read(&mut buf).expect_err("reuse rejected"),
            CompressionErrorKind::InvalidState,
        );
        assert_kind(dec.finish(), CompressionErrorKind::InvalidState);
    }

    #[test]
    fn decoded_budget_enforced_on_high_ratio_payload() {
        let payload = vec![0u8; 1 << 20];
        let data = encoded(&payload);
        assert!(
            data.len() < payload.len() / 100,
            "fixture must actually compress"
        );

        let under = DecodeOptions::new(
            DecodeScope::WholeInput,
            DecodeLimits::new().with_max_decoded_bytes(payload.len() as u64 - 1),
        );
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, under);
        let mut out = Vec::new();
        let err = dec.read_to_end(&mut out).expect_err("one byte under cap");
        assert_io_kind(&err, CompressionErrorKind::LimitExceeded);
        assert_eq!(out.len(), payload.len() - 1, "cap bytes still deliver");

        let exact = DecodeOptions::new(
            DecodeScope::WholeInput,
            DecodeLimits::new().with_max_decoded_bytes(payload.len() as u64),
        );
        let mut source = BufReader::new(&data[..]);
        let mut dec = Decoder::new(&mut source, exact);
        let mut out = Vec::new();
        dec.read_to_end(&mut out).expect("exact cap decodes");
        assert_eq!(out.len(), payload.len());
        dec.finish().expect("finish");
    }
}
