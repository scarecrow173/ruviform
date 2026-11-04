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
    let len = u32::from_le_bytes(len);
    if len as usize > MAX_INTENT_BYTES {
        // Drain the declared body so the stream stays frame-aligned —
        // an oversized frame must not desync the session.
        let mut discard = r.take(u64::from(len));
        io::copy(&mut discard, &mut io::sink())?;
        return Ok(FrameRead::TooLarge { declared: len });
    }
    let mut buf = vec![0u8; len as usize];
    r.read_exact(&mut buf)?;
    String::from_utf8(buf)
        .map(FrameRead::Frame)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Writes one length-prefixed frame and flushes.
///
/// # Errors
/// [`io::Error`] on write failure, or when `body` exceeds
/// [`MAX_INTENT_BYTES`].
pub fn write_frame(w: &mut impl Write, body: &str) -> io::Result<()> {
    if body.len() > MAX_INTENT_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("frame body {} exceeds {MAX_INTENT_BYTES} bytes", body.len()),
        ));
    }
    let len = u32::try_from(body.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "frame too large"))?;
    w.write_all(&len.to_le_bytes())?;
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
