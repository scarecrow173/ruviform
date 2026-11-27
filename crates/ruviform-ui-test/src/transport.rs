//! Control-channel framing and environment-driven transport
//! selection (Issue #312).
//!
//! Frames are `u32le` length + JSON payload, capped at
//! [`ruviform_ui::ipc::MAX_INTENT_BYTES`]. Two transports exist:
//!
//! - **stdio** (default): the driver spawns the app with piped
//!   stdin/stdout — an OS-permission-inherited local pipe, no ports.
//! - **loopback TCP** (opt-in): the driver listens on
//!   `127.0.0.1:0`, the app dials [`ENV_ADDR`], and `test.hello`
//!   must carry the run-bound [`ENV_SECRET`] — single connection,
//!   loopback peers only, refused when no secret is configured.
//!
//! [`ENV_DISABLE`] takes precedence over every serve path (runtime
//! disable). The same `serve(Read+Write)` loop drives both
//! transports.
//!
//! **RT contract:** non-RT only.

use std::io::{self, Read, Write};
use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};

use ruviform_ui::ipc::MAX_INTENT_BYTES;

/// Selects the agent transport: `stdio` | `tcp` | `off` (unset = off).
pub const ENV_CONTROL: &str = "RVF_TEST_CONTROL";
/// Loopback address the app dials for the TCP transport.
pub const ENV_ADDR: &str = "RVF_TEST_ADDR";
/// Run-bound secret `test.hello` must present on the TCP transport.
pub const ENV_SECRET: &str = "RVF_TEST_SECRET";
/// Runtime kill-switch — any truthy value refuses every serve path.
pub const ENV_DISABLE: &str = "RVF_TEST_DISABLE";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FrameWriteLengthDecision {
    Accept { len: u32 },
    Reject,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FrameReadLengthDecision {
    InCap { declared: u32 },
    TooLarge { declared: u32 },
}

fn frame_length_within_cap(len: usize) -> bool {
    len <= MAX_INTENT_BYTES
}

fn frame_write_length_decision(body_len: usize) -> FrameWriteLengthDecision {
    if frame_length_within_cap(body_len) {
        match u32::try_from(body_len) {
            Ok(len) => FrameWriteLengthDecision::Accept { len },
            Err(_) => FrameWriteLengthDecision::Reject,
        }
    } else {
        FrameWriteLengthDecision::Reject
    }
}

fn encode_frame_length(len: u32) -> [u8; 4] {
    len.to_le_bytes()
}

fn frame_read_length_decision(prefix: [u8; 4]) -> FrameReadLengthDecision {
    let declared = u32::from_le_bytes(prefix);
    if frame_length_within_cap(declared as usize) {
        FrameReadLengthDecision::InCap { declared }
    } else {
        FrameReadLengthDecision::TooLarge { declared }
    }
}

/// Frame read outcome.
#[non_exhaustive]
#[derive(Debug)]
pub enum FrameRead {
    /// A complete frame body.
    Frame(String),
    /// Clean end of stream.
    Eof,
    /// The declared length exceeded [`MAX_INTENT_BYTES`].
    TooLarge {
        /// The declared length.
        declared: u32,
    },
}

/// Reads one length-prefixed frame. I/O errors propagate; a clean EOF
/// at a frame boundary is `Eof` (mid-frame EOF is an `UnexpectedEof`
/// error).
///
/// # Errors
/// [`io::Error`] on read failure or a truncated frame.
pub fn read_frame(r: &mut impl Read) -> io::Result<FrameRead> {
    let mut len = [0u8; 4];
    match r.read_exact(&mut len) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(FrameRead::Eof),
        Err(e) => return Err(e),
    }
    match frame_read_length_decision(len) {
        FrameReadLengthDecision::TooLarge { declared } => {
            // Drain the declared body so the stream stays frame-aligned —
            // an oversized frame must not desync the session.
            let mut discard = r.take(u64::from(declared));
            io::copy(&mut discard, &mut io::sink())?;
            Ok(FrameRead::TooLarge { declared })
        }
        FrameReadLengthDecision::InCap { declared } => {
            let mut buf = vec![0u8; declared as usize];
            r.read_exact(&mut buf)?;
            String::from_utf8(buf)
                .map(FrameRead::Frame)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        }
    }
}

/// Writes one length-prefixed frame and flushes.
///
/// # Errors
/// [`io::Error`] on write failure, or when `body` exceeds
/// [`MAX_INTENT_BYTES`].
pub fn write_frame(w: &mut impl Write, body: &str) -> io::Result<()> {
    let len = match frame_write_length_decision(body.len()) {
        FrameWriteLengthDecision::Accept { len } => len,
        FrameWriteLengthDecision::Reject => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("frame body {} exceeds {MAX_INTENT_BYTES} bytes", body.len()),
            ));
        }
    };
    w.write_all(&encode_frame_length(len))?;
    w.write_all(body.as_bytes())?;
    w.flush()
}

/// Whether `RVF_TEST_DISABLE` carries a truthy value.
#[must_use]
pub fn disabled_from_env() -> bool {
    std::env::var(ENV_DISABLE).is_ok_and(|v| {
        let v = v.to_ascii_lowercase();
        matches!(v.as_str(), "1" | "true" | "yes" | "on")
    })
}

/// The `RVF_TEST_SECRET` value, when set and non-empty.
#[must_use]
pub fn expected_secret_from_env() -> Option<String> {
    std::env::var(ENV_SECRET).ok().filter(|s| !s.is_empty())
}

/// The agent transport selected by `RVF_TEST_CONTROL`.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentTransport {
    /// No control channel (unset or `off`).
    Off,
    /// Serve framed messages over stdin/stdout.
    Stdio,
    /// Dial the loopback listener at this address.
    Tcp {
        /// `RVF_TEST_ADDR` value.
        addr: String,
    },
}

/// Resolves `RVF_TEST_CONTROL`/`RVF_TEST_ADDR` into an
/// [`AgentTransport`]. `tcp` without `RVF_TEST_ADDR` resolves to
/// `Off` — a misconfigured opt-in must fail closed, not fall back.
#[must_use]
pub fn transport_from_env() -> AgentTransport {
    match std::env::var(ENV_CONTROL).ok().as_deref() {
        Some("stdio") => AgentTransport::Stdio,
        Some("tcp") => match std::env::var(ENV_ADDR).ok().filter(|s| !s.is_empty()) {
            Some(addr) => AgentTransport::Tcp { addr },
            None => AgentTransport::Off,
        },
        _ => AgentTransport::Off,
    }
}

/// Whether an address is loopback-only (v4 or v6).
#[must_use]
pub fn is_loopback(addr: IpAddr) -> bool {
    addr.is_loopback()
}

/// Generates a run-bound token (run/session ids, TCP secret).
///
/// Seeded from a fresh `RandomState` key, the wall clock, and the pid
/// — adequate for a run-scoped loopback secret under this feature's
/// threat model (a local test channel, not production auth). Never
/// reused across runs.
#[must_use]
pub fn gen_token() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    let mut h = RandomState::new().build_hasher();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    h.write_u128(nanos);
    h.write_u32(std::process::id());
    format!("{:016x}", h.finish())
}

#[cfg(kani)]
mod proofs {
    use super::{
        FrameReadLengthDecision, FrameWriteLengthDecision, encode_frame_length,
        frame_read_length_decision, frame_write_length_decision,
    };
    use ruviform_ui::ipc::MAX_INTENT_BYTES;

    fn write_length_oracle(body_len: usize) -> Option<(u32, [u8; 4])> {
        if body_len > MAX_INTENT_BYTES {
            return None;
        }
        let len = u32::try_from(body_len).ok()?;
        let prefix = [
            (len & 0xff) as u8,
            ((len >> 8) & 0xff) as u8,
            ((len >> 16) & 0xff) as u8,
            ((len >> 24) & 0xff) as u8,
        ];
        Some((len, prefix))
    }

    fn decode_length_oracle(prefix: [u8; 4]) -> u32 {
        u32::from(prefix[0])
            | (u32::from(prefix[1]) << 8)
            | (u32::from(prefix[2]) << 16)
            | (u32::from(prefix[3]) << 24)
    }

    #[kani::proof]
    #[kani::unwind(2)]
    fn w_frm_codec_write_length_exactness() {
        let body_len: usize = kani::any();
        let actual = frame_write_length_decision(body_len);
        match (actual, write_length_oracle(body_len)) {
            (FrameWriteLengthDecision::Accept { len }, Some((oracle_len, oracle_prefix))) => {
                assert_eq!(len, oracle_len);
                let actual_prefix = encode_frame_length(len);
                assert_eq!(actual_prefix[0], oracle_prefix[0]);
                assert_eq!(actual_prefix[1], oracle_prefix[1]);
                assert_eq!(actual_prefix[2], oracle_prefix[2]);
                assert_eq!(actual_prefix[3], oracle_prefix[3]);
            }
            (FrameWriteLengthDecision::Reject, None) => {}
            _ => panic!("write length decision mismatch"),
        }

        kani::cover!(
            body_len == 0 && matches!(actual, FrameWriteLengthDecision::Accept { .. }),
            "write length 0 acceptance is reachable"
        );
        kani::cover!(
            body_len == MAX_INTENT_BYTES - 1
                && matches!(actual, FrameWriteLengthDecision::Accept { .. }),
            "write cap boundary-1 acceptance is reachable"
        );
        kani::cover!(
            body_len == MAX_INTENT_BYTES
                && matches!(actual, FrameWriteLengthDecision::Accept { .. }),
            "write cap boundary acceptance is reachable"
        );
        kani::cover!(
            body_len == MAX_INTENT_BYTES + 1 && matches!(actual, FrameWriteLengthDecision::Reject),
            "write cap boundary+1 rejection is reachable"
        );
    }

    #[kani::proof]
    #[kani::unwind(2)]
    fn w_frm_codec_prefix_exactness() {
        let prefix: [u8; 4] = kani::any();
        let declared = decode_length_oracle(prefix);
        let actual = frame_read_length_decision(prefix);
        match actual {
            FrameReadLengthDecision::InCap {
                declared: actual_declared,
            } => {
                assert_eq!(actual_declared, declared);
                assert!(declared as usize <= MAX_INTENT_BYTES);
            }
            FrameReadLengthDecision::TooLarge {
                declared: actual_declared,
            } => {
                assert_eq!(actual_declared, declared);
                assert!(declared as usize > MAX_INTENT_BYTES);
            }
        }

        kani::cover!(
            declared == 0 && matches!(actual, FrameReadLengthDecision::InCap { .. }),
            "read length 0 acceptance is reachable"
        );
        kani::cover!(
            declared as usize == MAX_INTENT_BYTES - 1
                && matches!(actual, FrameReadLengthDecision::InCap { .. }),
            "read cap boundary-1 acceptance is reachable"
        );
        kani::cover!(
            declared as usize == MAX_INTENT_BYTES
                && matches!(actual, FrameReadLengthDecision::InCap { .. }),
            "read cap boundary acceptance is reachable"
        );
        kani::cover!(
            declared as usize == MAX_INTENT_BYTES + 1
                && matches!(actual, FrameReadLengthDecision::TooLarge { .. }),
            "read cap boundary+1 rejection is reachable"
        );
    }

    #[kani::proof]
    #[kani::unwind(2)]
    fn w_frm_codec_length_roundtrip() {
        let body_len: usize = kani::any();
        match frame_write_length_decision(body_len) {
            FrameWriteLengthDecision::Accept { len } => {
                assert!(body_len <= MAX_INTENT_BYTES);
                let decoded = frame_read_length_decision(encode_frame_length(len));
                assert!(matches!(
                    decoded,
                    FrameReadLengthDecision::InCap { declared } if declared == len
                ));
            }
            FrameWriteLengthDecision::Reject => {
                assert!(body_len > MAX_INTENT_BYTES);
            }
        }

        kani::cover!(body_len == 0, "roundtrip length 0 input is reachable");
        kani::cover!(
            body_len == MAX_INTENT_BYTES - 1,
            "roundtrip cap boundary-1 input is reachable"
        );
        kani::cover!(
            body_len == MAX_INTENT_BYTES,
            "roundtrip cap boundary input is reachable"
        );
        kani::cover!(
            body_len == MAX_INTENT_BYTES + 1,
            "roundtrip cap boundary+1 input is reachable"
        );
    }
}
