//! Encoder lifecycle contract: explicit `finish` as the only
//! completion proof, idempotence, terminal `Failed`, `Drop` writes
//! nothing, writer-borrow recovery, and byte budgets.

#![cfg(any(feature = "gzip", feature = "deflate", feature = "zstd"))]

mod common;

use std::io::Write;

use common::{FailAfter, FailFlush, RecoveringWriter, assert_io_kind, assert_kind};
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

    #[test]
    fn new_fails_when_header_write_fails() {
        // The fixed 10-byte header is emitted inside `new` — a dead
        // writer fails construction and the caller keeps the writer.
        let mut sink = FailAfter::new(0);
        assert_kind(
            Encoder::new(&mut sink, EncodeOptions::default()),
            CompressionErrorKind::Io,
        );
        assert!(
            sink.attempts > 0,
            "the header write must have been attempted"
        );
    }

    #[test]
    fn header_write_respects_output_budget() {
        // The header must not be emitted across a smaller output cap —
        // `new` fails before any physical write.
        let mut out = Vec::new();
        let limits = EncodeLimits::new().with_max_output_bytes(9);
        assert_kind(
            Encoder::new(&mut out, EncodeOptions::new(6, limits)),
            CompressionErrorKind::LimitExceeded,
        );
        assert!(out.is_empty(), "no partial header may be published");
    }

    #[test]
    fn flush_after_finish_is_ok() {
        let mut out = Vec::new();
        let mut enc = Encoder::new(&mut out, EncodeOptions::default()).expect("encoder");
        enc.finish().expect("finish");
        // `Finished` flush is a documented no-op so stacked writers
        // can flush unconditionally.
        enc.flush().expect("flush after finish");
    }

    #[test]
    fn flush_mid_stream_then_finish_still_decodes() {
        // A sync flush is not completion but must not corrupt the
        // member — write/flush/write/finish stays a valid stream.
        let mut out = Vec::new();
        {
            let mut enc = Encoder::new(&mut out, EncodeOptions::default()).expect("encoder");
            enc.write_all(b"before").expect("write");
            enc.flush().expect("mid-stream flush");
            enc.write_all(b"after").expect("write");
            enc.finish().expect("finish");
        }
        let mut source = std::io::BufReader::new(&out[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut decoded = Vec::new();
        dec.read_to_end(&mut decoded).expect("decode");
        assert_eq!(decoded, b"beforeafter");
    }

    #[test]
    fn failed_encoder_drop_writes_no_trailer() {
        // Budget covers the 10-byte header plus 2 bytes of the
        // flush-time sync marker; the rest fails. Drop must not
        // append a trailer over the terminal failure.
        let mut sink = FailAfter::new(12);
        {
            let mut enc = Encoder::new(&mut sink, EncodeOptions::default()).expect("encoder");
            enc.write_all(b"abcdef").expect("write");
            let err = enc.flush().expect_err("flush must hit the failing writer");
            assert_io_kind(&err, CompressionErrorKind::Io);
        }
        assert_eq!(sink.inner.len(), 12, "only the accepted prefix was emitted");
        let mut source = std::io::BufReader::new(&sink.inner[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut decoded = Vec::new();
        assert_io_kind(
            &dec.read_to_end(&mut decoded)
                .expect_err("partial member must not verify"),
            CompressionErrorKind::TruncatedInput,
        );
    }

    #[test]
    fn failed_encoder_releases_usable_writer() {
        // After terminal `Failed` the encoder is dropped and the
        // caller's writer must be fully usable again — partial encoder
        // output stays owned by the caller (publish-or-discard theirs).
        // call 1 = header (ok); call 2 = first body emission (fails).
        let mut sink = RecoveringWriter::after_ok(1, 1);
        {
            let mut enc = Encoder::new(&mut sink, EncodeOptions::default()).expect("encoder");
            let _ = enc.write_all(b"data");
            let _ = enc.flush();
            let _ = enc.finish();
            assert_io_kind(
                &enc.write_all(b"x").expect_err("terminal state"),
                CompressionErrorKind::InvalidState,
            );
        }
        sink.write_all(b"caller-owned")
            .expect("writer usable after failure");
        assert!(sink.inner.ends_with(b"caller-owned"));
        assert!(
            sink.inner.starts_with(&[0x1f, 0x8b]),
            "partial encoder output is retained for the caller"
        );
    }

    #[test]
    fn finish_fails_when_underlying_flush_fails() {
        // `finish` ends with the caller's `flush` — a flush failure is
        // an encoder I/O failure and latches `Failed` like any other.
        let mut sink = FailFlush::default();
        let mut enc = Encoder::new(&mut sink, EncodeOptions::default()).expect("encoder");
        enc.write_all(b"data").expect("write");
        assert_kind(enc.finish(), CompressionErrorKind::Io);
        assert_kind(enc.finish(), CompressionErrorKind::InvalidState);
        assert!(
            sink.inner.len() > 10,
            "compressed bytes were emitted before the flush failure"
        );
    }

    #[test]
    fn sequential_encoders_concatenate_members_on_one_writer() {
        // Each `Encoder` writes exactly one member; finishing one and
        // starting another on the same writer produces a valid
        // concatenated stream under `WholeInput`.
        let mut out = Vec::new();
        for part in [&b"first"[..], &b"second"[..]] {
            let mut enc = Encoder::new(&mut out, EncodeOptions::default()).expect("encoder");
            enc.write_all(part).expect("write");
            enc.finish().expect("finish");
        }
        let mut source = std::io::BufReader::new(&out[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut decoded = Vec::new();
        dec.read_to_end(&mut decoded).expect("decode");
        assert_eq!(decoded, b"firstsecond");
        let receipt = dec.finish().expect("finish");
        assert_eq!(receipt.units_decoded(), 2);
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
    fn flush_mid_stream_then_finish_still_decodes() {
        let mut out = Vec::new();
        {
            let mut enc = Encoder::new(&mut out, EncodeOptions::default()).expect("encoder");
            enc.write_all(b"before").expect("write");
            enc.flush().expect("mid-stream flush");
            enc.write_all(b"after").expect("write");
            enc.finish().expect("finish");
        }
        let mut source = std::io::BufReader::new(&out[..]);
        let mut dec = ruviform_io_compression::deflate::Decoder::new(
            &mut source,
            ruviform_io_compression::deflate::DecodeOptions::default(),
        );
        let mut decoded = Vec::new();
        std::io::Read::read_to_end(&mut dec, &mut decoded).expect("decode");
        assert_eq!(decoded, b"beforeafter");
    }

    #[test]
    fn failed_encoder_drop_writes_no_final_block() {
        // The failed encoder's partial output must not grow on Drop —
        // no final block may appear over the terminal failure.
        let mut sink = FailAfter::new(4);
        {
            let mut enc = Encoder::new(&mut sink, EncodeOptions::default()).expect("encoder");
            enc.write_all(b"abcdef").expect("write");
            let err = enc.flush().expect_err("flush must hit the failing writer");
            assert_io_kind(&err, CompressionErrorKind::Io);
        }
        let accepted = sink.inner.len();
        assert!(accepted <= 4, "only the budgeted prefix was emitted");
        let mut source = std::io::BufReader::new(&sink.inner[..]);
        let mut dec = ruviform_io_compression::deflate::Decoder::new(
            &mut source,
            ruviform_io_compression::deflate::DecodeOptions::default(),
        );
        let mut decoded = Vec::new();
        assert_io_kind(
            &std::io::Read::read_to_end(&mut dec, &mut decoded)
                .expect_err("partial stream must not verify"),
            CompressionErrorKind::TruncatedInput,
        );
    }

    #[test]
    fn failed_encoder_releases_usable_writer() {
        // call 1 = first emitted chunk (fails); the writer recovers
        // afterwards and the caller owns whatever prefix was accepted.
        let mut sink = RecoveringWriter::new(1);
        {
            let mut enc = Encoder::new(&mut sink, EncodeOptions::default()).expect("encoder");
            let _ = enc.write_all(b"data");
            let _ = enc.flush();
            let _ = enc.finish();
            assert_io_kind(
                &enc.write_all(b"x").expect_err("terminal state"),
                CompressionErrorKind::InvalidState,
            );
        }
        sink.write_all(b"caller-owned")
            .expect("writer usable after failure");
        assert!(sink.inner.ends_with(b"caller-owned"));
    }

    #[test]
    fn finish_fails_when_underlying_flush_fails() {
        let mut sink = FailFlush::default();
        let mut enc = Encoder::new(&mut sink, EncodeOptions::default()).expect("encoder");
        enc.write_all(b"data").expect("write");
        assert_kind(enc.finish(), CompressionErrorKind::Io);
        assert_kind(enc.finish(), CompressionErrorKind::InvalidState);
        assert!(!sink.inner.is_empty(), "deflate bytes were emitted");
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
    fn flush_mid_stream_then_finish_still_decodes() {
        let mut out = Vec::new();
        {
            let mut enc = Encoder::new(&mut out, EncodeOptions::default()).expect("encoder");
            enc.write_all(b"before").expect("write");
            enc.flush().expect("mid-stream flush");
            enc.write_all(b"after").expect("write");
            enc.finish().expect("finish");
        }
        let mut source = std::io::BufReader::new(&out[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut decoded = Vec::new();
        dec.read_to_end(&mut decoded).expect("decode");
        assert_eq!(decoded, b"beforeafter");
        dec.finish().expect("verified end");
    }

    #[test]
    fn failed_encoder_drop_writes_no_epilogue() {
        // The failed encoder's partial frame must not grow on Drop —
        // no epilogue may appear over the terminal failure.
        let mut sink = FailAfter::new(6);
        {
            let mut enc = Encoder::new(&mut sink, EncodeOptions::default()).expect("encoder");
            enc.write_all(b"abcdef").expect("write");
            let err = enc
                .finish()
                .expect_err("finish must hit the failing writer");
            assert_eq!(err.kind(), CompressionErrorKind::Io);
        }
        let accepted = sink.inner.len();
        assert!(accepted <= 6, "only the budgeted prefix was emitted");
        let mut source = std::io::BufReader::new(&sink.inner[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut decoded = Vec::new();
        assert!(
            dec.read_to_end(&mut decoded).is_err() || dec.finish().is_err(),
            "a partial frame must not verify"
        );
    }

    #[test]
    fn failed_encoder_releases_usable_writer() {
        // call 1 = first emitted chunk (fails); the writer recovers
        // afterwards and the caller owns whatever prefix was accepted.
        let mut sink = RecoveringWriter::new(1);
        {
            let mut enc = Encoder::new(&mut sink, EncodeOptions::default()).expect("encoder");
            let _ = enc.write_all(b"data");
            let _ = enc.flush();
            let _ = enc.finish();
            assert_io_kind(
                &enc.write_all(b"x").expect_err("terminal state"),
                CompressionErrorKind::InvalidState,
            );
        }
        sink.write_all(b"caller-owned")
            .expect("writer usable after failure");
        assert!(sink.inner.ends_with(b"caller-owned"));
    }

    #[test]
    fn finish_fails_when_underlying_flush_fails() {
        let mut sink = FailFlush::default();
        let mut enc = Encoder::new(&mut sink, EncodeOptions::default()).expect("encoder");
        enc.write_all(b"data").expect("write");
        assert_kind(enc.finish(), CompressionErrorKind::Io);
        assert_kind(enc.finish(), CompressionErrorKind::InvalidState);
        assert!(
            sink.inner.len() > 4,
            "frame bytes were emitted before the flush failure"
        );
    }

    #[test]
    fn sequential_encoders_concatenate_frames() {
        // Each `Encoder` writes exactly one frame; finishing one and
        // starting another on the same writer produces a valid
        // concatenated stream under `WholeInput`.
        let mut out = Vec::new();
        for part in [&b"first"[..], &b"second"[..]] {
            let mut enc = Encoder::new(&mut out, EncodeOptions::default()).expect("encoder");
            enc.write_all(part).expect("write");
            enc.finish().expect("finish");
        }
        let mut source = std::io::BufReader::new(&out[..]);
        let mut dec = Decoder::new(&mut source, DecodeOptions::default());
        let mut decoded = Vec::new();
        dec.read_to_end(&mut decoded).expect("decode");
        assert_eq!(decoded, b"firstsecond");
        let receipt = dec.finish().expect("finish");
        assert_eq!(receipt.units_decoded(), 2);
    }
}
