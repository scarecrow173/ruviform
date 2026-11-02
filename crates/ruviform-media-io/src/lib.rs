//! `ruviform-media-io` — the backend-agnostic media-I/O abstraction
//! boundary of the Ruviform media foundation.
//!
//! Container demux/mux and codec decode/encode are **separated trait
//! families**, never merged into a single read/write abstraction
//! (media-io ADR, design baseline "メディア I/O 抽象"):
//!
//! - [`Demuxer`] — container → [`Packet`] (stream enumeration,
//!   packet reads, time-based seek *request*)
//! - [`Muxer`] — [`Packet`] → container (unverified: no implementor yet)
//! - [`Decoder`] — [`Packet`] → [`Frame`] (`send`/`receive` pump +
//!   `flush` residual-state reset)
//! - [`Encoder`] — [`Frame`] → [`Packet`] (unverified: no implementor yet)
//!
//! Around them sit the boundary value types — [`Packet`] /
//! [`PacketFlags`], [`AudioFrame`] / [`NativeAudioFrame`] / [`Frame`],
//! [`StreamInfo`] with the codec-level [`CodecDescriptor`] /
//! [`CodecId`] / [`CodecFormat`], the [`MediaSource`] input port,
//! [`Capabilities`] / [`Features`] (capability negotiation, query
//! side) and the normalized [`Error`] / [`Result`] / [`ErrorKind`]
//! model.
//!
//! ## Observation contract
//!
//! Everything the boundary reports is a *classified fact about the
//! input* — never a fabricated default:
//!
//! - Packet timestamps are raw [`TickTimestamp`] ticks + provenance
//!   (`Carried` / `Derived` / `Estimated`) resolved against
//!   `StreamDescriptor::time_base` — itself a `Fact` that can report
//!   `NotDeclared`/`Unsupported`/`Malformed` instead of a guessed base.
//! - [`StreamFacts`] carries container-native identity (`container_id`,
//!   `codec_tag`, `language`), the declared clock origin (`start`),
//!   declared unit counts and per-stream decode capability
//!   (`decodable`); [`ContainerInfo`] identifies the container family
//!   (`ContainerDetail` carries verbatim per-family facts such as
//!   ISOBMFF brands); [`BackendInfo`] names the reporting backend.
//! - [`NativeAudioFrame`] is the non-destructive PCM path: interleaved
//!   `f64` in the decoder's native scale with an explicit
//!   [`PcmTransform`] conversion history — parallel to, never a
//!   replacement for, the canonical `f32` [`AudioFrame`].
//!
//! This crate holds **no backend implementation**: every trait is
//! implemented by separate `ruviform-io-*` crates selected as opt-in
//! dependencies, and no external (backend) type, error or feature name
//! ever appears in this public API — the surface is Ruviform value
//! types (`ruviform-media-core` / `-audio-core` / `-video-core`) and
//! `std` types only. Domain types are *used* but never re-exported:
//! backend crates take direct `path` dependencies on the domain
//! crates instead.
//!
//! ## Allowed dependency layers
//!
//! `ruviform-media-core`, `ruviform-audio-core` and
//! `ruviform-video-core` only (design baseline crate table). No
//! external crate dependencies, no backend crates — `cargo tree -p
//! ruviform-media-io` must show exactly those three edges.
//!
//! ## RT contract
//!
//! **Non-RT only.** The whole crate lives behind the I/O/decode-thread
//! boundary (design baseline, thread-model section): every trait method
//! may block on I/O, allocate and do unbounded codec work, so calling
//! any of it from an RT path is a contract violation. Data types are
//! `Send` — packets, frames and descriptors travel from the
//! I/O/decode side to the processing side through bounded queues.
//! Per-API labels read `**RT contract:** non-RT only`.
//!
//! Contract source: `docs/designs/ruviform-architecture-design-baseline.md`
//! (crate table, "メディア I/O 抽象", thread model),
//! `docs/adr/media-io-backend-abstraction.md` and
//! `docs/adr/realtime-contract-and-verification-policy.md`.

// The public names and shapes below are fixed API decisions of the
// media-I/O boundary: they deliberately echo their module
// (`packet::Packet`, `codec::CodecId`, …) while being re-exported at
// the crate root. As a `publish = false` workspace-internal crate the
// boundary value types rely on literal construction for structs, like
// the domain crates. Every public enum carries enum-level
// `#[non_exhaustive]` — see
// `docs/adr/public-enum-non-exhaustive-policy.md`.
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::exhaustive_structs)]

mod capability;
mod codec;
mod decode;
mod demux;
mod encode;
mod error;
mod frame;
mod mux;
mod packet;
mod sink;
mod source;
mod staged;

pub use capability::{Capabilities, Features};
pub use codec::{CodecDescriptor, CodecFormat, CodecId};
pub use decode::{DecodeMode, Decoder};
pub use demux::{
    BackendInfo, CodecTag, ContainerDetail, ContainerFormat, ContainerInfo, Demuxer, StreamFacts,
    StreamInfo, VideoStreamFacts,
};
pub use encode::Encoder;
pub use error::{Error, ErrorKind, Result};
pub use frame::{AudioFrame, Frame, NativeAudioFrame, PcmTransform};
pub use mux::Muxer;
pub use packet::{Packet, PacketFlags};
pub use sink::MediaSink;
pub use source::MediaSource;
pub use staged::{PublishOptions, StagedOutput};

/// Returns the crate version string.
///
/// **RT contract:** non-RT only — trivial pure function, but part of a
/// non-RT boundary crate.
///
/// # Examples
///
/// ```
/// assert_ne!(ruviform_media_io::version(), "");
/// ```
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_send<T: Send>() {}

    #[test]
    fn version_is_not_empty() {
        assert_ne!(version(), "");
    }

    /// The boundary data types and the trait objects must all be able
    /// to cross the I/O/decode-thread → bounded-queue boundary.
    #[test]
    fn boundary_types_cross_threads() {
        check_send::<Packet>();
        check_send::<PacketFlags>();
        check_send::<AudioFrame>();
        check_send::<NativeAudioFrame>();
        check_send::<PcmTransform>();
        check_send::<Frame>();
        check_send::<StreamInfo>();
        check_send::<StreamFacts>();
        check_send::<ContainerInfo>();
        check_send::<CodecTag>();
        check_send::<BackendInfo>();
        check_send::<VideoStreamFacts>();
        check_send::<CodecId>();
        check_send::<CodecFormat>();
        check_send::<CodecDescriptor>();
        check_send::<Capabilities>();
        check_send::<Features>();
        check_send::<Error>();
        check_send::<ErrorKind>();
        check_send::<Box<dyn Demuxer>>();
        check_send::<Box<dyn Muxer>>();
        check_send::<Box<dyn Decoder>>();
        check_send::<Box<dyn Encoder>>();
        check_send::<Box<dyn MediaSource>>();
    }
}
