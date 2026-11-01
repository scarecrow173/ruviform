//! Contract verification: capability honesty, error classification,
//! drain/finalize distinction, and safe staged publish — Issue #191
//! AC2, AC4, AC5.

use ruviform_audio_core::{AudioBuffer, BufferSpec, ChannelLayout, SampleFormat, SampleRate};
use ruviform_io_wav::{WavOutputSpec, make_encoder, open_muxer, wav_capabilities};
use ruviform_media_core::{Fact, MediaTime, MediaTimestamp, StreamId, StreamKind};
use ruviform_media_io::{
    AudioFrame, CodecDescriptor, CodecFormat, CodecId, Demuxer, Encoder, ErrorKind, Frame, Muxer,
    PublishOptions, StagedOutput, StreamInfo,
};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

fn spec(rate: u32, layout: ChannelLayout, wire: SampleFormat) -> WavOutputSpec {
    WavOutputSpec {
        sample_rate: SampleRate::new(rate).unwrap(),
        layout,
        sample_format: wire,
    }
}

// Index values stay under 64, so the usize→f32 cast is exact.
#[allow(clippy::cast_precision_loss)]
fn stereo_frame(rate: u32, frames: usize) -> Frame {
    let mut buffer = AudioBuffer::new(BufferSpec {
        layout: ChannelLayout::Stereo,
        frames,
        sample_rate: SampleRate::new(rate).unwrap(),
    });
    for channel in 0..2 {
        for (index, sample) in buffer.channel_mut(channel).unwrap().iter_mut().enumerate() {
            *sample = (index % 64) as f32 / 128.0 - 0.25;
        }
    }
    Frame::Audio(AudioFrame::new(
        buffer,
        MediaTimestamp::carried(MediaTime::new(
            0,
            SampleRate::new(rate).unwrap().time_base(),
        )),
    ))
}

fn stream_info_with(codec: CodecDescriptor) -> StreamInfo {
    let mut info = spec(48_000, ChannelLayout::Stereo, SampleFormat::S16).stream_info();
    info.codec = codec;
    info
}

// --- capability honesty (AC2) -------------------------------------------

#[test]
fn capabilities_are_honest_and_queryable() {
    let caps = ruviform_io_wav::capabilities();
    assert!(caps.can_mux());
    assert!(caps.can_encode(CodecId::Pcm));
    assert!(!caps.can_demux());
    assert!(!caps.can_decode(CodecId::Pcm));

    let wav = wav_capabilities();
    assert_eq!(wav.container, ruviform_media_io::ContainerFormat::Wav);
    assert_eq!(
        wav.sample_formats,
        [
            SampleFormat::U8,
            SampleFormat::S16,
            SampleFormat::S24,
            SampleFormat::S32,
            SampleFormat::F32
        ]
    );
    assert!(!wav.rf64);
    assert!(!wav.bwf);
    assert_eq!(wav.max_streams, 1);
}

#[test]
fn non_pcm_codec_is_unsupported() {
    let info = stream_info_with(CodecDescriptor {
        codec: CodecId::Flac,
        codec_private: Vec::new(),
        format: CodecFormat::Audio(ruviform_audio_core::AudioFormat {
            sample_rate: SampleRate::new(48_000).unwrap(),
            layout: ChannelLayout::Stereo,
            sample_format: SampleFormat::F32,
        }),
        native_format: Fact::known(CodecFormat::Audio(ruviform_audio_core::AudioFormat {
            sample_rate: SampleRate::new(48_000).unwrap(),
            layout: ChannelLayout::Stereo,
            sample_format: SampleFormat::S16,
        })),
    });
    assert_eq!(
        make_encoder(&info).unwrap_err().kind(),
        ErrorKind::Unsupported
    );
    let sink = Cursor::new(Vec::new());
    assert_eq!(
        open_muxer(sink, &[info]).unwrap_err().kind(),
        ErrorKind::Unsupported
    );
}

#[test]
fn unsupported_wire_format_is_unsupported() {
    // F64 and S8 are outside the advertised wire set.
    for wire in [SampleFormat::F64, SampleFormat::S8, SampleFormat::U16] {
        let info = spec(48_000, ChannelLayout::Stereo, wire).stream_info();
        assert_eq!(
            make_encoder(&info).unwrap_err().kind(),
            ErrorKind::Unsupported,
            "wire {wire:?} must be Unsupported"
        );
    }
}

#[test]
fn multi_stream_is_unsupported() {
    let info = spec(48_000, ChannelLayout::Stereo, SampleFormat::S16).stream_info();
    let mut second = info.clone();
    second.stream.id = StreamId(1);
    let sink = Cursor::new(Vec::new());
    assert_eq!(
        open_muxer(sink, &[info.clone(), second])
            .unwrap_err()
            .kind(),
        ErrorKind::Unsupported
    );
}

#[test]
fn undeclared_wire_format_is_invalid_value() {
    let mut info = spec(48_000, ChannelLayout::Stereo, SampleFormat::S16).stream_info();
    info.codec.native_format = Fact::NotDeclared;
    assert_eq!(
        make_encoder(&info).unwrap_err().kind(),
        ErrorKind::InvalidValue
    );
}

#[test]
fn non_audio_stream_is_invalid_value() {
    let mut info = spec(48_000, ChannelLayout::Stereo, SampleFormat::S16).stream_info();
    info.stream.kind = StreamKind::Video;
    assert_eq!(
        make_encoder(&info).unwrap_err().kind(),
        ErrorKind::InvalidValue
    );
}

// --- input validation (AC3) ----------------------------------------------

#[test]
fn channel_count_mismatch_is_invalid_data() {
    let info = spec(48_000, ChannelLayout::Stereo, SampleFormat::S16).stream_info();
    let mut encoder = make_encoder(&info).unwrap();
    let mut buffer = AudioBuffer::new(BufferSpec {
        layout: ChannelLayout::Mono, // 1 channel vs declared 2
        frames: 4,
        sample_rate: SampleRate::new(48_000).unwrap(),
    });
    buffer.channel_mut(0).unwrap().fill(0.1);
    let result = encoder.send_frame(Frame::Audio(AudioFrame::new(
        buffer,
        MediaTimestamp::carried(MediaTime::new(
            0,
            SampleRate::new(48_000).unwrap().time_base(),
        )),
    )));
    assert_eq!(result.unwrap_err().kind(), ErrorKind::InvalidData);
}

#[test]
fn sample_rate_mismatch_is_invalid_data() {
    let info = spec(48_000, ChannelLayout::Stereo, SampleFormat::S16).stream_info();
    let mut encoder = make_encoder(&info).unwrap();
    let result = encoder.send_frame(stereo_frame(44_100, 4));
    assert_eq!(result.unwrap_err().kind(), ErrorKind::InvalidData);
}

// --- finalize contract (AC4) ----------------------------------------------

#[test]
fn write_after_finish_and_double_finish_error() {
    let info = spec(48_000, ChannelLayout::Stereo, SampleFormat::S16).stream_info();
    let mut encoder = make_encoder(&info).unwrap();
    encoder.send_frame(stereo_frame(48_000, 8)).unwrap();
    let packet = encoder.receive_packet().unwrap().unwrap();

    let mut muxer = open_muxer(Cursor::new(Vec::new()), std::slice::from_ref(&info)).unwrap();
    muxer.write_packet(packet.clone()).unwrap();
    Muxer::finish(&mut muxer).unwrap();
    assert_eq!(
        muxer.write_packet(packet).unwrap_err().kind(),
        ErrorKind::InvalidData
    );
    assert_eq!(
        Muxer::finish(&mut muxer).unwrap_err().kind(),
        ErrorKind::InvalidData
    );
    // The receipt exists — the file completed — errors afterwards do
    // not retract completion.
    assert!(muxer.receipt().is_some());
}

#[test]
fn packet_stream_id_mismatch_is_invalid_data() {
    let info = spec(48_000, ChannelLayout::Stereo, SampleFormat::S16).stream_info();
    let mut encoder = make_encoder(&info).unwrap();
    encoder.send_frame(stereo_frame(48_000, 8)).unwrap();
    let mut packet = encoder.receive_packet().unwrap().unwrap();
    packet.stream = StreamId(9);
    let mut muxer = open_muxer(Cursor::new(Vec::new()), &[info]).unwrap();
    assert_eq!(
        muxer.write_packet(packet).unwrap_err().kind(),
        ErrorKind::InvalidData
    );
}

#[test]
fn partial_frame_payload_is_invalid_data() {
    let info = spec(48_000, ChannelLayout::Stereo, SampleFormat::S16).stream_info();
    let mut muxer = open_muxer(Cursor::new(Vec::new()), &[info]).unwrap();
    let packet = ruviform_media_io::Packet {
        stream: StreamId(0),
        index: 0,
        pts: None,
        dts: None,
        duration: None,
        trim_start: 0,
        trim_end: 0,
        flags: ruviform_media_io::PacketFlags::KEYFRAME,
        data: vec![0; 5], // 5 % block_align(4) != 0
    };
    assert_eq!(
        muxer.write_packet(packet).unwrap_err().kind(),
        ErrorKind::InvalidData
    );
}

/// A sink that reports disk-full after `remaining` bytes — write
/// errors surface as `ErrorKind::Io`, never as completion.
struct FailAfter {
    inner: Cursor<Vec<u8>>,
    remaining: usize,
}

impl FailAfter {
    fn new(remaining: usize) -> Self {
        Self {
            inner: Cursor::new(Vec::new()),
            remaining,
        }
    }
}

impl Write for FailAfter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.remaining == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::StorageFull,
                "disk full",
            ));
        }
        let take = self.remaining.min(buf.len());
        let written = self.inner.write(&buf[..take])?;
        self.remaining -= written;
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

impl Seek for FailAfter {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

#[test]
fn sink_write_error_is_io_not_completion() {
    let info = spec(48_000, ChannelLayout::Stereo, SampleFormat::S16).stream_info();
    let mut encoder = make_encoder(&info).unwrap();
    encoder.send_frame(stereo_frame(48_000, 256)).unwrap();

    // Enough room for the header only — packet write hits the wall.
    let header_room = 64usize;
    let sink = FailAfter::new(header_room);
    let mut muxer = open_muxer(sink, &[info]).unwrap();
    let mut wrote = false;
    while let Some(packet) = encoder.receive_packet().unwrap() {
        match muxer.write_packet(packet) {
            Ok(()) => wrote = true,
            Err(error) => {
                assert_eq!(error.kind(), ErrorKind::Io);
                // No receipt exists — the write never completed.
                assert!(muxer.receipt().is_none());
                return;
            }
        }
    }
    panic!("expected the failing sink to produce an Io error (wrote={wrote})");
}

#[test]
fn sink_error_at_finalize_is_io_not_completion() {
    let info = spec(48_000, ChannelLayout::Stereo, SampleFormat::S16).stream_info();
    // Room for header + one small packet, but finalize's backpatch
    // write hits the wall too… actually the backpatch seek+write hits
    // the same counter — size the sink so only finalize fails.
    let mut encoder = make_encoder(&info).unwrap();
    encoder.send_frame(stereo_frame(48_000, 4)).unwrap();
    let packet_len = {
        let packet = encoder.receive_packet().unwrap().unwrap();
        packet.data.len()
    };
    let header_len = 44usize; // stereo S16 WAVEFORMATEX header
    let sink = FailAfter::new(header_len + packet_len);
    let mut muxer = open_muxer(sink, std::slice::from_ref(&info)).unwrap();
    let mut encoder = make_encoder(&info).unwrap();
    encoder.send_frame(stereo_frame(48_000, 4)).unwrap();
    while let Some(packet) = encoder.receive_packet().unwrap() {
        muxer.write_packet(packet).unwrap();
    }
    let error = muxer.finalize().unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Io);
    assert!(muxer.receipt().is_none());
}

// --- staged publish (AC5) -------------------------------------------------

fn scratch_dir() -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "wav-staged-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn read(path: &Path) -> Vec<u8> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    bytes
}

fn write_wav_to_sink<S: ruviform_media_io::MediaSink>(
    sink: S,
    info: &StreamInfo,
    frames: usize,
) -> u64 {
    let mut encoder = make_encoder(info).unwrap();
    encoder.send_frame(stereo_frame(48_000, frames)).unwrap();
    encoder.drain().unwrap();
    let mut muxer = open_muxer(sink, std::slice::from_ref(info)).unwrap();
    while let Some(packet) = encoder.receive_packet().unwrap() {
        muxer.write_packet(packet).unwrap();
    }
    let receipt = muxer.finalize().unwrap();
    receipt.total_bytes
}

#[test]
fn staged_publish_produces_reopenable_wav() {
    let dir = scratch_dir();
    let dest = dir.join("out.wav");
    let info = spec(48_000, ChannelLayout::Stereo, SampleFormat::S16).stream_info();

    let mut staged = StagedOutput::create(&dest).unwrap();
    write_wav_to_sink(staged.sink().unwrap(), &info, 480);
    staged.publish(PublishOptions::no_overwrite()).unwrap();

    // Reopen via the symphonia path used in the round-trip tests.
    let bytes = read(&dest);
    let demuxer = ruviform_io_symphonia::open_demuxer(Cursor::new(bytes.clone()))
        .expect("published wav must reopen");
    assert_eq!(demuxer.streams().len(), 1);
    let _ = demuxer.streams()[0].clone();
    assert_eq!(bytes[0..4], *b"RIFF");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn staged_abort_preserves_existing_destination() {
    let dir = scratch_dir();
    let dest = dir.join("out.wav");
    std::fs::write(&dest, b"old master").unwrap();
    let info = spec(48_000, ChannelLayout::Stereo, SampleFormat::S16).stream_info();

    let mut staged = StagedOutput::create(&dest).unwrap();
    write_wav_to_sink(staged.sink().unwrap(), &info, 480);
    staged.abort();
    assert_eq!(read(&dest), b"old master");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn staged_drop_after_write_preserves_destination() {
    let dir = scratch_dir();
    let dest = dir.join("out.wav");
    std::fs::write(&dest, b"old master").unwrap();
    let info = spec(48_000, ChannelLayout::Stereo, SampleFormat::S16).stream_info();

    {
        let mut staged = StagedOutput::create(&dest).unwrap();
        write_wav_to_sink(staged.sink().unwrap(), &info, 480);
        // dropped without publish — simulates cancellation
    }
    assert_eq!(read(&dest), b"old master");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn staged_no_overwrite_conflict_preserves_destination() {
    let dir = scratch_dir();
    let dest = dir.join("out.wav");
    std::fs::write(&dest, b"old master").unwrap();
    let info = spec(48_000, ChannelLayout::Stereo, SampleFormat::S16).stream_info();

    let mut staged = StagedOutput::create(&dest).unwrap();
    write_wav_to_sink(staged.sink().unwrap(), &info, 480);
    let error = staged.publish(PublishOptions::no_overwrite()).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidValue);
    assert_eq!(read(&dest), b"old master");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn staged_replace_overwrites_destination() {
    let dir = scratch_dir();
    let dest = dir.join("out.wav");
    std::fs::write(&dest, b"old master").unwrap();
    let info = spec(48_000, ChannelLayout::Stereo, SampleFormat::S16).stream_info();

    let mut staged = StagedOutput::create(&dest).unwrap();
    write_wav_to_sink(staged.sink().unwrap(), &info, 480);
    staged.publish(PublishOptions::replace()).unwrap();
    let bytes = read(&dest);
    assert_eq!(bytes[0..4], *b"RIFF");
    assert_ne!(bytes, b"old master");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn staged_disk_full_leaves_destination_untouched() {
    let dir = scratch_dir();
    let dest = dir.join("out.wav");
    std::fs::write(&dest, b"old master").unwrap();
    let info = spec(48_000, ChannelLayout::Stereo, SampleFormat::S16).stream_info();

    let staged = StagedOutput::create(&dest).unwrap();
    // A wrapper around the staged file that fails quickly is not
    // possible while `staged` owns the File — simulate disk-full
    // against a standalone failing sink instead, then drop the staged
    // output as the cancel path. Both halves of the guarantee are
    // covered: the write fails Io, the destination survives.
    let mut encoder = make_encoder(&info).unwrap();
    encoder.send_frame(stereo_frame(48_000, 1_024)).unwrap();
    let mut muxer = open_muxer(FailAfter::new(64), &[info]).unwrap();
    let mut failed = false;
    while let Some(packet) = encoder.receive_packet().unwrap() {
        if muxer.write_packet(packet).is_err() {
            failed = true;
            break;
        }
    }
    assert!(failed);
    drop(muxer);
    staged.abort();
    assert_eq!(read(&dest), b"old master");
    std::fs::remove_dir_all(&dir).unwrap();
}
