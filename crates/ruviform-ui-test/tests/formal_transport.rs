//! Issue #806 slice 1 (D-layer formal verification) — `rvf.uitest.v1`
//! transport framing (u32le length prefix + `MAX_INTENT_BYTES` cap)
//! property suite plus `#[cfg(kani)]` bounded proof harnesses.
//!
//! Target contracts (product source of truth:
//! `crates/ruviform-ui-test/src/transport.rs`):
//!
//! - `write_frame` emits `u32le len || body` and flushes; a body
//!   longer than `MAX_INTENT_BYTES` is `Err(InvalidInput)` with
//!   nothing written.
//! - `read_frame` tri-classifies: a complete frame → `Frame`, a
//!   length prefix that cannot complete (< 4 bytes) → `Eof`, a
//!   declared length over the cap → `TooLarge { declared }` after
//!   draining the declared body so the stream stays frame-aligned,
//!   a truncated body → `Err(UnexpectedEof)`, and a complete body
//!   that is not UTF-8 → `Err(InvalidData)`.
//!
//! The `#[cfg(kani)]` module at the bottom carries the same contracts
//! as bounded proofs (run via `mise run test-formal-kani`); the
//! bounds and assumptions are documented in
//! `docs/specs/formal-verification.md`.

use std::io::Cursor;

use proptest::prelude::*;
use ruviform_ui::ipc::MAX_INTENT_BYTES;
use ruviform_ui_test::transport::{FrameRead, read_frame, write_frame};

/// Frame bodies — mostly small arbitrary strings, with boundary
/// cases at the byte cap.
fn arb_frame_body() -> impl Strategy<Value = String> {
    prop_oneof![
        16 => prop::collection::vec(any::<char>(), 0..=2048).prop_map(String::from_iter),
        4 => prop::collection::vec(any::<u8>(), 0..=2048)
            .prop_map(|v| String::from_utf8_lossy(&v).into_owned()),
        1 => Just("b".repeat(MAX_INTENT_BYTES)),
        1 => Just("c".repeat(MAX_INTENT_BYTES - 1)),
    ]
}

proptest! {
    /// `write_frame` → `read_frame` is the identity for in-cap
    /// bodies, including the exact-cap boundary.
    #[test]
    fn write_read_roundtrip(body in arb_frame_body()) {
        let mut buf = Vec::new();
        write_frame(&mut buf, &body).expect("in-cap body must write");
        prop_assert_eq!(
            &buf[..4],
            &u32::try_from(body.len()).unwrap().to_le_bytes()
        );
        prop_assert_eq!(&buf[4..], body.as_bytes());
        let mut cur = Cursor::new(&buf);
        match read_frame(&mut cur).expect("written frame must read") {
            FrameRead::Frame(got) => prop_assert_eq!(got, body),
            _ => prop_assert!(false, "written frame must come back Frame"),
        }
        prop_assert_eq!(cur.position(), buf.len() as u64);
    }

    /// Over-cap bodies are refused before a single byte is written.
    #[test]
    fn write_frame_over_cap_refused(over in 1usize..=512) {
        let body = "x".repeat(MAX_INTENT_BYTES + over);
        let mut buf = Vec::new();
        let err = write_frame(&mut buf, &body).expect_err("over-cap body must fail");
        prop_assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        prop_assert!(buf.is_empty(), "refused frame must not write bytes");
    }

    /// Arbitrary byte streams: `read_frame` never panics and its
    /// outcome obeys the framing contract — tri-class plus the stream
    /// position each class guarantees.
    #[test]
    fn read_frame_arbitrary_stream(bytes in prop::collection::vec(any::<u8>(), 0..=600)) {
        let mut cur = Cursor::new(&bytes);
        match read_frame(&mut cur) {
            Ok(FrameRead::Frame(body)) => {
                let declared = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
                prop_assert_eq!(declared as usize, body.len());
                prop_assert_eq!(cur.position(), 4 + u64::from(declared));
                prop_assert_eq!(body.as_bytes(), &bytes[4..4 + declared as usize]);
            }
            Ok(FrameRead::Eof) => {
                prop_assert!(bytes.len() < 4, "Eof requires a short length prefix");
            }
            Ok(FrameRead::TooLarge { declared }) => {
                prop_assert!(bytes.len() >= 4);
                let declared_wire = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
                prop_assert_eq!(declared, declared_wire);
                prop_assert!(declared as usize > MAX_INTENT_BYTES);
                // the declared body was drained → the position is at end
                prop_assert_eq!(cur.position(), bytes.len() as u64);
            }
            Ok(_) => prop_assert!(false, "new FrameRead variant — extend the property"),
            Err(e) => {
                prop_assert!(bytes.len() >= 4, "a short stream cannot produce Err");
                let declared = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
                match e.kind() {
                    // declared body truncated mid-frame
                    std::io::ErrorKind::UnexpectedEof => {
                        prop_assert!(declared as usize <= MAX_INTENT_BYTES);
                        prop_assert!(bytes.len() - 4 < declared as usize);
                    }
                    // complete body that is not valid UTF-8
                    std::io::ErrorKind::InvalidData => {
                        prop_assert!(declared as usize <= MAX_INTENT_BYTES);
                        prop_assert!(bytes.len() - 4 >= declared as usize);
                    }
                    other => prop_assert!(false, "unexpected io error kind {other:?}"),
                }
            }
        }
    }
}

proptest! {
    // ~1 MiB of frame body per case — keep the case count small.
    #![proptest_config(ProptestConfig::with_cases(8))]

    /// A declared length over the cap reports `TooLarge { declared }`
    /// and drains the declared body, so the next frame still reads
    /// aligned.
    #[test]
    fn too_large_drains_and_stays_aligned(
        extra in 1u32..=8,
        tail in prop::collection::vec(any::<char>(), 0..=64).prop_map(String::from_iter),
    ) {
        let declared = u32::try_from(MAX_INTENT_BYTES).unwrap() + extra;
        let mut stream = Vec::with_capacity(4 + declared as usize + 4 + tail.len());
        stream.extend_from_slice(&declared.to_le_bytes());
        stream.extend_from_slice(&vec![b'z'; declared as usize]);
        write_frame(&mut stream, &tail).unwrap();
        let mut cur = Cursor::new(&stream);
        match read_frame(&mut cur).unwrap() {
            FrameRead::TooLarge { declared: got } => prop_assert_eq!(got, declared),
            _ => prop_assert!(false, "oversized declared length must be TooLarge"),
        }
        match read_frame(&mut cur).unwrap() {
            FrameRead::Frame(body) => prop_assert_eq!(body, tail),
            _ => prop_assert!(false, "stream must stay frame-aligned after drain"),
        }
    }
}

/// Bounded proofs of the same contracts, verified exhaustively over
/// symbolic input by Kani (`cargo kani --tests`; run via
/// `mise run test-formal-kani`). Bounds and assumptions:
/// `docs/specs/formal-verification.md`.
#[cfg(kani)]
mod proofs {
    use super::*;

    /// Bounded stream length — covers every `FrameRead` arm plus
    /// mid-frame EOF while keeping the I/O unwinding small. The 1 MiB
    /// declared-cap path stays reachable through the symbolic u32
    /// prefix; the round-trip proof uses an in-cap symbolic body.
    const STREAM_LEN: usize = 32;

    /// A symbolic stream of symbolic length ≤ `STREAM_LEN`.
    fn any_stream() -> ([u8; STREAM_LEN], usize) {
        let bytes: [u8; STREAM_LEN] = kani::any();
        let len: usize = kani::any();
        kani::assume(len <= STREAM_LEN);
        (bytes, len)
    }

    /// `read_frame` terminates on every bounded stream and honors
    /// the tri-class contract and the TooLarge drain position. The
    /// unwind bound caps I/O, drain, and UTF-8 loops over the
    /// ≤32-byte stream.
    #[kani::proof]
    #[kani::unwind(48)]
    fn read_frame_contract() {
        let (bytes, len) = any_stream();
        let mut cur = Cursor::new(&bytes[..len]);
        match read_frame(&mut cur) {
            Ok(FrameRead::Frame(body)) => {
                let declared = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                assert_eq!(declared as usize, body.len());
                assert_eq!(cur.position(), 4 + u64::from(declared));
            }
            Ok(FrameRead::Eof) => assert!(len < 4),
            Ok(FrameRead::TooLarge { declared }) => {
                assert!(declared as usize > MAX_INTENT_BYTES);
                assert_eq!(cur.position(), len as u64);
            }
            Ok(_) => panic!("unexpected FrameRead variant"),
            Err(e) => {
                assert!(len >= 4);
                let declared = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                assert!(declared as usize <= MAX_INTENT_BYTES);
                match e.kind() {
                    std::io::ErrorKind::UnexpectedEof => assert!(len - 4 < declared as usize),
                    std::io::ErrorKind::InvalidData => assert!(len - 4 >= declared as usize),
                    _ => panic!("unexpected io error kind"),
                }
            }
        }
    }

    /// `write_frame` → `read_frame` is the identity for bounded
    /// in-cap bodies. Fixed-length body (16 arbitrary bytes accepted
    /// as UTF-8 — a symbolic length would force CBMC to unwind every
    /// allocation and read loop over a symbolic bound). The unwind
    /// bound caps write/read/UTF-8 loops over the 16-byte body.
    #[kani::proof]
    #[kani::unwind(40)]
    fn write_read_roundtrip() {
        let bytes: [u8; 16] = kani::any();
        let Ok(body) = core::str::from_utf8(&bytes) else {
            return;
        };
        let mut buf = Vec::with_capacity(4 + bytes.len());
        // `match`, not `.unwrap()` — Debug::fmt on `io::Error` recurses
        // through `dyn Error` and is a known CBMC cost bomb.
        if write_frame(&mut buf, body).is_err() {
            unreachable!("in-cap body must write")
        }
        let mut cur = Cursor::new(&buf);
        match read_frame(&mut cur) {
            Ok(FrameRead::Frame(got)) => assert!(got == body),
            _ => panic!("written frame must read back"),
        }
    }
}
