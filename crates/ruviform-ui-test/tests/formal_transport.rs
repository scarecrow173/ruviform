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
//!   clean EOF before the first length-prefix byte → `Eof`, a partial
//!   length prefix (1..=3 bytes) → `Err(UnexpectedEof)`, a declared
//!   length over the cap → `TooLarge { declared }` after draining the
//!   declared body so the stream stays frame-aligned, a truncated body
//!   → `Err(UnexpectedEof)`, and a complete body that is not UTF-8 →
//!   `Err(InvalidData)`.
//!
//! The read-classification and length-prefix/write-cap decision-core
//! proofs live beside the production core in `src/transport.rs` and run
//! via `mise run test-formal-kani w-frm-readclass` /
//! `mise run test-formal-kani w-frm-codec`; the bounds and assumptions
//! are documented in `docs/specs/formal-verification.md`.

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
                prop_assert!(bytes.is_empty(), "Eof requires EOF before the first prefix byte");
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
            Err(e) => match e.kind() {
                std::io::ErrorKind::UnexpectedEof if bytes.len() < 4 => {
                    prop_assert!(!bytes.is_empty(), "empty input must be clean EOF");
                }
                std::io::ErrorKind::UnexpectedEof => {
                    let declared = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
                    prop_assert!(declared as usize <= MAX_INTENT_BYTES);
                    prop_assert!(bytes.len() - 4 < declared as usize);
                }
                // complete body that is not valid UTF-8
                std::io::ErrorKind::InvalidData => {
                    prop_assert!(bytes.len() >= 4);
                    let declared = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
                    prop_assert!(declared as usize <= MAX_INTENT_BYTES);
                    prop_assert!(bytes.len() - 4 >= declared as usize);
                }
                other => prop_assert!(false, "unexpected io error kind {other:?}"),
            },
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

/// Deterministic connection regression for the pure length decision core:
/// exact-cap write/read acceptance, cap+1 pre-write rejection, u32le byte
/// order, and read-side `TooLarge` classification must reach the public shell.
#[test]
fn frame_length_boundary_connection_regression() {
    let max_body = "a".repeat(MAX_INTENT_BYTES);
    for len in [0, MAX_INTENT_BYTES - 1, MAX_INTENT_BYTES] {
        let body = &max_body[..len];
        let mut buf = Vec::new();
        write_frame(&mut buf, body).expect("in-cap boundary body must write");
        assert_eq!(&buf[..4], &u32::try_from(len).unwrap().to_le_bytes());
        let mut cur = Cursor::new(&buf);
        match read_frame(&mut cur).expect("in-cap boundary frame must read") {
            FrameRead::Frame(got) => assert_eq!(got, body),
            _ => panic!("in-cap boundary frame must come back Frame"),
        }
    }

    let over_body = "x".repeat(MAX_INTENT_BYTES + 1);
    let mut buf = Vec::new();
    let err = write_frame(&mut buf, &over_body).expect_err("cap+1 body must fail");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert!(buf.is_empty(), "cap+1 body must not write bytes");

    let declared = u32::try_from(MAX_INTENT_BYTES + 1).unwrap();
    let mut cur = Cursor::new(declared.to_le_bytes());
    match read_frame(&mut cur).expect("cap+1 prefix must classify") {
        FrameRead::TooLarge { declared: got } => assert_eq!(got, declared),
        _ => panic!("cap+1 prefix must be TooLarge"),
    }
}

/// Deterministic connection regression for read classification: clean EOF,
/// every partial-prefix length, body truncation, invalid UTF-8, extra bytes
/// as the next frame's input, and oversized-frame drain alignment must reach
/// the public shell.
#[test]
fn read_frame_classification_connection_regression() {
    let mut cur = Cursor::new(Vec::<u8>::new());
    match read_frame(&mut cur).expect("empty input must classify") {
        FrameRead::Eof => {}
        _ => panic!("empty input must be clean EOF"),
    }

    for prefix_len in 1usize..=3 {
        let bytes = vec![0u8; prefix_len];
        let mut cur = Cursor::new(bytes);
        let err = read_frame(&mut cur).expect_err("partial prefix must fail");
        assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
    }

    let mut truncated = Vec::new();
    truncated.extend_from_slice(&2u32.to_le_bytes());
    truncated.push(b'a');
    let mut cur = Cursor::new(truncated);
    let err = read_frame(&mut cur).expect_err("truncated body must fail");
    assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);

    let mut invalid_utf8 = Vec::new();
    invalid_utf8.extend_from_slice(&1u32.to_le_bytes());
    invalid_utf8.push(0xff);
    let mut cur = Cursor::new(invalid_utf8);
    let err = read_frame(&mut cur).expect_err("invalid UTF-8 body must fail");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);

    let mut two_frames = Vec::new();
    write_frame(&mut two_frames, "first").unwrap();
    write_frame(&mut two_frames, "second").unwrap();
    let mut cur = Cursor::new(two_frames);
    match read_frame(&mut cur).expect("first frame must read") {
        FrameRead::Frame(body) => assert_eq!(body, "first"),
        _ => panic!("first complete frame must come back Frame"),
    }
    match read_frame(&mut cur).expect("extra bytes must remain as the next frame") {
        FrameRead::Frame(body) => assert_eq!(body, "second"),
        _ => panic!("bytes after the first frame must remain frame-aligned"),
    }

    let declared = u32::try_from(MAX_INTENT_BYTES + 1).unwrap();
    let tail = "aligned";
    let mut stream = Vec::with_capacity(4 + declared as usize + 4 + tail.len());
    stream.extend_from_slice(&declared.to_le_bytes());
    stream.extend_from_slice(&vec![b'z'; declared as usize]);
    write_frame(&mut stream, tail).unwrap();
    let mut cur = Cursor::new(stream);
    match read_frame(&mut cur).expect("oversized frame must classify") {
        FrameRead::TooLarge { declared: got } => assert_eq!(got, declared),
        _ => panic!("oversized frame must be TooLarge"),
    }
    match read_frame(&mut cur).expect("tail frame must remain aligned") {
        FrameRead::Frame(body) => assert_eq!(body, tail),
        _ => panic!("oversized drain must preserve frame alignment"),
    }
}
