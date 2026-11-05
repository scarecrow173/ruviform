//! Encoder lifecycle contract: explicit `finish` as the only
//! completion proof, idempotence, terminal `Failed`, `Drop` writes
//! nothing, writer-borrow recovery, and byte budgets.

#![cfg(any(feature = "gzip", feature = "deflate", feature = "zstd"))]

mod common;

use std::io::Write;

use common::{FailAfter, assert_io_kind, assert_kind};
use ruviform_io_compression::CompressionErrorKind;

#[cfg(feature = "gzip")]
mod gzip {
    use super::*;
    use std::io::Read;

    use ruviform_io_compression::EncodeLimits;
    use ruviform_io_compression::gzip::{DecodeOptions, Decoder, EncodeOptions, Encoder};

    fn encode(parts: &[&[u8]], level: u32) -> Vec<u8> {
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

    #[test]
    fn finish_returns_receipt_and_is_idempotent() {
        let mut out = Vec::new();
        let mut enc = Encoder::new(&mut out, EncodeOptions::default()).expect("encoder");
        enc.write_all(b"payload").expect("write");
        let first = enc.finish().expect("finish");
        let second = enc.finish().expect("finish again");
        assert_eq!(first, second);
        assert_eq!(first.input_bytes(), 7);
        assert_eq!(first.units_encoded(), 1);
        let len_after = first.output_bytes();
        // A repeated finish emits nothing further.
        assert_eq!(enc.finish().expect("third").output_bytes(), len_after);
        drop(enc);
        assert_eq!(out.len() as u64, len_after);
    }

    #[test]
    fn writer_borrow_recovered_after_scope() {
        let mut out = Vec::new();
        {
            let mut enc = Encoder::new(&mut out, EncodeOptions::default()).expect("encoder");
            enc.write_all(b"x").expect("write");
            enc.finish().expect("finish");
        }
        out.extend_from_slice(b"tail"); // borrow released — caller owns `out` again
        assert!(out.ends_with(b"tail"));
    }

    #[test]
    fn byte_at_a_time_writes_roundtrip() {
        let mut out = Vec::new();
        {
            let mut enc = Encoder::new(&mut out, EncodeOptions::default()).expect("encoder");
            for byte in b"abcdef" {
                enc.write_all(&[*byte]).expect("write");
            }
            enc.finish().expect("finish");
        }
        let mut source = std::io::BufReader::new(&out[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut decoded = Vec::new();
        dec.read_to_end(&mut decoded).expect("decode");
        assert_eq!(decoded, b"abcdef");
    }

    #[test]
    fn flush_pushes_pending_bytes_but_is_not_completion() {
        let mut unflushed = Vec::new();
        {
            let mut enc = Encoder::new(&mut unflushed, EncodeOptions::default()).unwrap();
            enc.write_all(b"abc").unwrap();
        }
        let mut flushed = Vec::new();
        {
            let mut enc = Encoder::new(&mut flushed, EncodeOptions::default()).unwrap();
            enc.write_all(b"abc").unwrap();
            enc.flush().expect("sync flush");
        }
        assert!(
            flushed.len() > unflushed.len(),
            "sync flush must push pending deflate bits out"
        );
        // The flushed bytes are still not a complete member.
        let mut source = std::io::BufReader::new(&flushed[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut decoded = Vec::new();
        let err = dec.read_to_end(&mut decoded).unwrap_err();
        assert_io_kind(&err, CompressionErrorKind::TruncatedInput);
    }

    #[test]
    fn drop_writes_no_trailer() {
        let mut out = Vec::new();
        {
            let mut enc = Encoder::new(&mut out, EncodeOptions::default()).expect("encoder");
            enc.write_all(b"unfinished").expect("write");
            // Drop without finish — no trailer may appear.
        }
        let mut source = std::io::BufReader::new(&out[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        // The body may decode, but verified end is unreachable.
        let result = dec.read_to_end(&mut Vec::new());
        assert_io_kind(
            &result.expect_err("unterminated stream must not verify"),
            CompressionErrorKind::TruncatedInput,
        );
    }

    #[test]
    fn io_failure_latches_failed_and_rejects_reuse() {
        // Budget covers exactly the 10-byte header. `flush` emits a
        // deflate sync marker even with no input, so it is the
        // deterministic trigger that reaches the failing writer.
        let mut sink = FailAfter::new(10);
        let mut enc = Encoder::new(&mut sink, EncodeOptions::default()).expect("encoder");
        let err = enc.flush().expect_err("flush must hit the failing writer");
        assert_io_kind(&err, CompressionErrorKind::Io);
        assert_io_kind(
            &enc.write_all(b"more").expect_err("reuse rejected"),
            CompressionErrorKind::InvalidState,
        );
        assert_kind(enc.finish(), CompressionErrorKind::InvalidState);
    }

    #[test]
    fn write_after_finish_rejected_but_finish_stays_valid() {
        let mut out = Vec::new();
        let mut enc = Encoder::new(&mut out, EncodeOptions::default()).expect("encoder");
        enc.finish().expect("finish");
        assert_io_kind(
            &enc.write_all(b"x").expect_err("write after finish"),
            CompressionErrorKind::InvalidState,
        );
        enc.finish().expect("finish stays idempotent");
    }

    #[test]
    fn invalid_level_rejected_at_construction() {
        let mut out = Vec::new();
        assert_kind(
            Encoder::new(&mut out, EncodeOptions::new(10, EncodeLimits::new())),
            CompressionErrorKind::InvalidInput,
        );
        assert_eq!(out.len(), 0);
    }

    #[test]
    fn input_budget_allows_exact_and_rejects_over() {
        let mut out = Vec::new();
        let limits = EncodeLimits::new().with_max_input_bytes(4);
        let mut enc = Encoder::new(&mut out, EncodeOptions::new(6, limits)).expect("encoder");
        enc.write_all(b"1234").expect("exactly at budget");
        assert_io_kind(
            &enc.write_all(b"5").expect_err("one byte over"),
            CompressionErrorKind::LimitExceeded,
        );
    }

    #[test]
    fn output_budget_caps_physical_writes() {
        // header(10) fits the cap; the finish-time final block plus
        // 8-byte trailer is a deterministic crossing emission.
        let mut out = Vec::new();
        let limits = EncodeLimits::new().with_max_output_bytes(12);
        let mut enc = Encoder::new(&mut out, EncodeOptions::new(0, limits)).expect("encoder");
        enc.write_all(b"data").expect("write");
        assert_kind(enc.finish(), CompressionErrorKind::LimitExceeded);
        assert!(out.len() <= 12, "no write may cross the budget");
    }

    #[test]
    fn output_budget_exact_boundary_succeeds() {
        let probe = encode(&[b"data"], 6);
        let exact = probe.len() as u64;
        let mut out = Vec::new();
        let limits = EncodeLimits::new().with_max_output_bytes(exact);
        let mut enc = Encoder::new(&mut out, EncodeOptions::new(6, limits)).expect("encoder");
        enc.write_all(b"data").expect("write");
        let receipt = enc.finish().expect("exact output budget");
        assert_eq!(receipt.output_bytes(), exact);
    }
}

#[cfg(feature = "deflate")]
mod deflate {
    use super::*;
    use ruviform_io_compression::deflate::{EncodeOptions, Encoder};

    #[test]
    fn finish_returns_receipt_and_is_idempotent() {
        let mut out = Vec::new();
        let mut enc = Encoder::new(&mut out, EncodeOptions::default()).expect("encoder");
        enc.write_all(b"payload").expect("write");
        let first = enc.finish().expect("finish");
        assert_eq!(enc.finish().expect("again"), first);
        assert_eq!(first.input_bytes(), 7);
        assert_eq!(first.units_encoded(), 1);
        let len_after = first.output_bytes();
        drop(enc);
        assert_eq!(out.len() as u64, len_after);
    }

    #[test]
    fn drop_writes_no_final_block() {
        let mut out = Vec::new();
        {
            let mut enc = Encoder::new(&mut out, EncodeOptions::default()).expect("encoder");
            enc.write_all(b"unfinished").expect("write");
        }
        let mut source = std::io::BufReader::new(&out[..]);
        let mut dec = ruviform_io_compression::deflate::Decoder::new(
            &mut source,
            ruviform_io_compression::deflate::DecodeOptions::default(),
        );
        let mut decoded = Vec::new();
        let result = std::io::Read::read_to_end(&mut dec, &mut decoded);
        assert_io_kind(
            &result.expect_err("unterminated stream must not verify"),
            CompressionErrorKind::TruncatedInput,
        );
    }

    #[test]
    fn failed_encoder_rejects_reuse() {
        // Raw deflate buffers small writes without emitting, so the
        // failure is driven through `flush` — the first call that is
        // guaranteed to push bytes at the writer.
        let mut sink = FailAfter::new(0);
        let mut enc = Encoder::new(&mut sink, EncodeOptions::default()).expect("encoder");
        enc.write_all(b"abcdef").expect("write");
        let err = enc.flush().expect_err("flush must hit the failing writer");
        assert_io_kind(&err, CompressionErrorKind::Io);
        assert_io_kind(
            &enc.write_all(b"more").expect_err("reuse rejected"),
            CompressionErrorKind::InvalidState,
        );
        assert_kind(enc.finish(), CompressionErrorKind::InvalidState);
    }
}

#[cfg(feature = "zstd")]
mod zstd {
    use super::*;
    use std::io::Read;

    use ruviform_io_compression::EncodeLimits;
    use ruviform_io_compression::zstd::{DecodeOptions, Decoder, EncodeOptions, Encoder};

    #[test]
    fn finish_returns_receipt_and_is_idempotent() {
        let mut out = Vec::new();
        let mut enc = Encoder::new(&mut out, EncodeOptions::default()).expect("encoder");
        enc.write_all(b"payload").expect("write");
        let first = enc.finish().expect("finish");
        let second = enc.finish().expect("finish again");
        assert_eq!(first, second);
        assert_eq!(first.input_bytes(), 7);
        assert_eq!(first.units_encoded(), 1);
        let len_after = first.output_bytes();
        drop(enc);
        assert_eq!(out.len() as u64, len_after);
    }

    #[test]
    fn writer_borrow_recovered_after_scope() {
        let mut out = Vec::new();
        {
            let mut enc = Encoder::new(&mut out, EncodeOptions::default()).expect("encoder");
            enc.write_all(b"x").expect("write");
            enc.finish().expect("finish");
        }
        out.extend_from_slice(b"tail");
        assert!(out.ends_with(b"tail"));
    }

    #[test]
    fn byte_at_a_time_writes_roundtrip() {
        let mut out = Vec::new();
        {
            let mut enc = Encoder::new(&mut out, EncodeOptions::default()).expect("encoder");
            for byte in b"abcdef" {
                enc.write_all(&[*byte]).expect("write");
            }
            enc.finish().expect("finish");
        }
        let mut source = std::io::BufReader::new(&out[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut decoded = Vec::new();
        dec.read_to_end(&mut decoded).expect("decode");
        assert_eq!(decoded, b"abcdef");
    }

    #[test]
    fn drop_writes_no_frame_epilogue() {
        let mut out = Vec::new();
        {
            let mut enc = Encoder::new(&mut out, EncodeOptions::default()).expect("encoder");
            enc.write_all(b"unfinished").expect("write");
        }
        // Whatever the backend buffered, the frame was never ended —
        // the decoder must not reach a verified end.
        let mut source = std::io::BufReader::new(&out[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut decoded = Vec::new();
        let result = dec.read_to_end(&mut decoded);
        assert!(
            result.is_err() || dec.finish().is_err(),
            "an unfinished frame must not verify"
        );
    }

    #[test]
    fn failed_encoder_rejects_reuse() {
        // A `write` buffers inside the zstd context without touching
        // the writer, so drive the failure through `finish` — the
        // first call guaranteed to emit at the writer.
        let mut sink = FailAfter::new(0);
        let mut enc = Encoder::new(&mut sink, EncodeOptions::default()).expect("encoder");
        enc.write_all(b"abcdef").expect("write");
        assert_kind(enc.finish(), CompressionErrorKind::Io);
        assert_io_kind(
            &enc.write_all(b"more").expect_err("reuse rejected"),
            CompressionErrorKind::InvalidState,
        );
        assert_kind(enc.finish(), CompressionErrorKind::InvalidState);
    }

    #[test]
    fn flush_emits_block_but_is_not_completion() {
        let mut flushed = Vec::new();
        {
            let mut enc = Encoder::new(&mut flushed, EncodeOptions::default()).expect("encoder");
            enc.write_all(b"data").expect("write");
            enc.flush().expect("flush");
        } // dropped without finish
        assert!(!flushed.is_empty(), "block flush emits the pending block");
        // The flushed block is real output yet not a complete frame —
        // `flush` is not a completion proof.
        let mut source = std::io::BufReader::new(&flushed[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut decoded = Vec::new();
        let result = dec.read_to_end(&mut decoded);
        assert!(
            result.is_err() || dec.finish().is_err(),
            "a flushed-but-unfinished frame must not verify"
        );
    }

    #[test]
    fn input_budget_rejects_writes() {
        let mut out = Vec::new();
        let limits = EncodeLimits::new().with_max_input_bytes(4);
        let mut enc =
            Encoder::new(&mut out, EncodeOptions::new(0, false, limits)).expect("encoder");
        assert_io_kind(
            &enc.write_all(b"12345").expect_err("over budget"),
            CompressionErrorKind::LimitExceeded,
        );
    }

    #[test]
    fn output_budget_caps_physical_writes() {
        // The frame epilogue cannot fit a near-zero output cap.
        let mut out = Vec::new();
        let limits = EncodeLimits::new().with_max_output_bytes(12);
        let mut enc =
            Encoder::new(&mut out, EncodeOptions::new(0, false, limits)).expect("encoder");
        enc.write_all(b"data").expect("write");
        assert_kind(enc.finish(), CompressionErrorKind::LimitExceeded);
        assert!(out.len() <= 12, "no write may cross the budget");
    }

    #[test]
    fn output_budget_exact_boundary_succeeds() {
        let probe = {
            let mut out = Vec::new();
            let mut enc = Encoder::new(&mut out, EncodeOptions::default()).expect("enc");
            enc.write_all(b"data").expect("write");
            enc.finish().expect("finish");
            out.len() as u64
        };
        let mut out = Vec::new();
        let limits = EncodeLimits::new().with_max_output_bytes(probe);
        let mut enc =
            Encoder::new(&mut out, EncodeOptions::new(0, false, limits)).expect("encoder");
        enc.write_all(b"data").expect("write");
        let receipt = enc.finish().expect("exact output budget");
        assert_eq!(receipt.output_bytes(), probe);
    }
}
