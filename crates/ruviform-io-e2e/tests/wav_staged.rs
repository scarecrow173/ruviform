//! `WavMuxer` → `StagedFile` → seal → independent decode → publish (R5).
//!
//! The boundary under test is the RF305 hand-off sequence:
//! `finalize()` proves every byte reached the sink, `into_sink()`
//! releases the `&mut StagedFile` borrow so `seal()` can consume the
//! file, the sealed bytes are decoded by an *independent* backend
//! (symphonia demuxer) before publish, and the published file
//! re-opens cleanly.

mod common;

use std::io::{Cursor, Read, Write};

use common::{TestDir, read};
use ruviform_audio_core::{ChannelLayout, SampleFormat, SampleRate};
use ruviform_io_file::{Durability, PublishMode, PublishOptions, StagedFile};
use ruviform_io_wav::{WavMuxer, WavOutputSpec};
use ruviform_media_core::{StreamId, TickTimestamp};
use ruviform_media_io::{ContainerFormat, Demuxer, Muxer, Packet, PacketFlags};

const RATE: u32 = 48_000;
const FRAMES: usize = 1_024;
const CHANNELS: usize = 2; // stereo

fn spec() -> WavOutputSpec {
    WavOutputSpec {
        sample_rate: SampleRate::new(RATE).expect("rate"),
        layout: ChannelLayout::Stereo,
        sample_format: SampleFormat::S16,
    }
}

/// Deterministic interleaved S16 PCM payload for `frames` frames.
fn pcm_payload(frames: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(frames * CHANNELS * 2);
    for i in 0..frames {
        // i64 intermediate keeps the value in i16 range (-16384..=16382).
        let v = i16::try_from((i64::try_from(i).expect("frame index") * 97) % 32_767 - 16_384)
            .expect("sample fits i16");
        for _ in 0..CHANNELS {
            data.extend_from_slice(&v.to_le_bytes());
        }
    }
    data
}

fn packet(index: u64, pts: u64, data: Vec<u8>) -> Packet {
    Packet {
        stream: StreamId(0),
        index,
        pts: Some(TickTimestamp::carried(i64::try_from(pts).unwrap())),
        dts: None,
        duration: Some(u64::try_from(data.len() / (CHANNELS * 2)).unwrap()),
        trim_start: 0,
        trim_end: 0,
        flags: PacketFlags::KEYFRAME,
        data,
    }
}

/// Independent decode oracle: symphonia demuxes the bytes and
/// concatenates recovered PCM payload — a parser this crate did not
/// write. Comparing payload *bytes* (not just the count) catches
/// same-length corruption.
fn independent_decode(bytes: &[u8]) -> ruviform_media_io::Result<(ContainerFormat, Vec<u8>)> {
    let mut demuxer = ruviform_io_symphonia::open_demuxer(Cursor::new(bytes.to_vec()))?;
    let format = demuxer.container().format;
    let mut payload = Vec::new();
    while let Some(pkt) = demuxer.read_packet()? {
        payload.extend_from_slice(&pkt.data);
    }
    Ok((format, payload))
}

#[test]
fn wav_finalize_borrow_release_seal_decode_publish_reopen() {
    let dir = TestDir::new("wav");
    let dest = dir.dest("tone.wav");
    let payload = pcm_payload(FRAMES);

    let mut staged = StagedFile::create(&dest).expect("stage");
    // The muxer borrows the staged file — Write+Seek over the
    // staging area while the format owns the frame stream.
    let mut muxer = WavMuxer::new(&mut staged, &[spec().stream_info()]).expect("muxer");
    muxer
        .write_packet(packet(0, 0, payload.clone()))
        .expect("write packet");
    let receipt = muxer.finalize().expect("finalize — header backpatched");
    assert_eq!(receipt.total_frames, u64::try_from(FRAMES).expect("frames"));
    // Releasing the borrow is the hand-off back to the file layer.
    let _staged_ref = muxer.into_sink();

    let mut sealed = staged.seal();
    // Pre-publish verification: an independent backend parses the
    // sealed bytes — header, fmt chunk, data chunk, PCM payload.
    let mut sealed_bytes = Vec::new();
    sealed
        .open_readback()
        .expect("readback")
        .read_to_end(&mut sealed_bytes)
        .expect("read sealed");
    let (format, decoded) = independent_decode(&sealed_bytes).expect("independent decode");
    assert_eq!(format, ContainerFormat::Wav);
    assert_eq!(decoded, payload, "sealed PCM must decode byte-exact");

    sealed
        .publish(PublishOptions::new(
            PublishMode::CreateNew,
            Durability::FileSync,
        ))
        .expect("publish verified wav");

    // Post-publish reopen: decode the destination file again.
    let published = read(&dest).expect("published bytes");
    let (format2, decoded2) = independent_decode(&published).expect("reopen decode");
    assert_eq!(format2, ContainerFormat::Wav);
    assert_eq!(decoded2, payload, "published PCM must decode byte-exact");
}

#[test]
fn truncated_wav_payload_is_caught_by_independent_decode() {
    // Stage a *truncated* container: a write-side fault (disk full,
    // crash mid-stage) leaves bytes that end before the header's
    // declared data size. The caller's pre-publish gate — independent
    // decode — observes fewer payload bytes than the container
    // promises and must not publish.
    let dir = TestDir::new("wav-truncated");
    let dest = dir.dest("cut.wav");
    let payload = pcm_payload(128);

    // Build a valid WAV in memory, then cut the tail mid-payload.
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut muxer = WavMuxer::new(&mut cursor, &[spec().stream_info()]).expect("muxer");
        muxer
            .write_packet(packet(0, 0, payload.clone()))
            .expect("write packet");
        muxer.finalize().expect("finalize");
    }
    let full = cursor.into_inner();
    let cut = &full[..full.len() - 200];

    let mut staged = StagedFile::create(&dest).expect("stage");
    staged.write_all(cut).expect("write truncated");
    let mut sealed = staged.seal();
    let mut sealed_bytes = Vec::new();
    sealed
        .open_readback()
        .expect("readback")
        .read_to_end(&mut sealed_bytes)
        .expect("read sealed");

    // Independent decode either errors outright or yields strictly
    // fewer/different payload bytes than the muxed stream — never a
    // silent pass matching the full payload.
    let decoded = independent_decode(&sealed_bytes).ok().map(|(_, p)| p);
    assert_ne!(
        decoded,
        Some(payload),
        "truncated wav must not decode to the full payload"
    );
    // Boundary contract: unverifiable bytes are not published.
    sealed.abort().expect("abort");
    assert!(read(&dest).is_none());
}

#[test]
fn second_finalize_is_rejected_and_receipt_is_stable() {
    let dir = TestDir::new("wav-double-final");
    let dest = dir.dest("twice.wav");
    let payload = pcm_payload(32);

    let mut staged = StagedFile::create(&dest).expect("stage");
    let mut muxer = WavMuxer::new(&mut staged, &[spec().stream_info()]).expect("muxer");
    muxer.write_packet(packet(0, 0, payload)).expect("packet");
    let first = muxer.finalize().expect("first finalize");
    assert!(muxer.finalize().is_err(), "second finalize must fail");
    assert_eq!(muxer.receipt(), Some(&first));
    let _ = muxer.into_sink();
    let sealed = staged.seal();
    sealed
        .publish(PublishOptions::new(
            PublishMode::CreateNew,
            Durability::FileSync,
        ))
        .expect("publish");
    assert!(common::exists(&dest));
}
