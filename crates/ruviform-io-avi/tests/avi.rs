//! Generated-fixture coverage for `ruviform-io-avi`: a minimal AVI
//! writer (same chunk grammar the `AudioLens` `al-testkit` builder
//! produces) plus demux/decode/capability assertions through the
//! `ruviform-media-io` boundary.
// The byte-writer is fixture plumbing: fixed-width field casts are the
// point of the RIFF grammar it serializes.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::similar_names,
    clippy::trivially_copy_pass_by_ref
)]

use ruviform_io_avi::{capabilities, open_demuxer};
use ruviform_media_core::{Fact, StreamKind, TimestampProvenance};
use ruviform_media_io::Features;
use ruviform_media_io::{
    CodecFormat, CodecId, CodecTag, ContainerFormat, Decoder, Demuxer, ErrorKind, Frame,
    PacketFlags,
};
use ruviform_video_core::PixelFormat;

// ---------- generated AVI fixture ----------

fn put_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn chunk(out: &mut Vec<u8>, id: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(id);
    put_u32(out, body.len() as u32);
    out.extend_from_slice(body);
    if body.len() % 2 == 1 {
        out.push(0);
    }
}

fn list(out: &mut Vec<u8>, ty: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(b"LIST");
    put_u32(out, (body.len() + 4) as u32);
    out.extend_from_slice(ty);
    out.extend_from_slice(body);
    if body.len() % 2 == 1 {
        out.push(0);
    }
}

#[allow(clippy::too_many_arguments)]
fn strh(
    fcc: &[u8; 4],
    scale: u32,
    rate: u32,
    start: u32,
    length: u32,
    sample_size: u32,
) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(fcc);
    body.extend_from_slice(fcc); // fccHandler — reuse the type code
    put_u32(&mut body, 0); // dwFlags
    put_u16(&mut body, 0); // wPriority
    put_u16(&mut body, 0); // wLanguage
    put_u32(&mut body, 0); // dwInitialFrames
    put_u32(&mut body, scale);
    put_u32(&mut body, rate);
    put_u32(&mut body, start);
    put_u32(&mut body, length);
    put_u32(&mut body, 0); // dwSuggestedBufferSize
    put_u32(&mut body, 0xFFFF_FFFF); // dwQuality
    put_u32(&mut body, sample_size);
    body.extend_from_slice(&[0u8; 16]); // rcFrame
    body
}

fn waveformatex(tag: u16, channels: u16, rate: u32, bits: u16) -> Vec<u8> {
    let bytes_per = u32::from(bits / 8);
    let mut body = Vec::new();
    put_u16(&mut body, tag);
    put_u16(&mut body, channels);
    put_u32(&mut body, rate);
    put_u32(&mut body, rate * bytes_per * u32::from(channels)); // avg bytes/sec
    put_u16(&mut body, (bytes_per * u32::from(channels)) as u16); // block align
    put_u16(&mut body, bits);
    put_u16(&mut body, 0); // cbSize
    body
}

fn bitmapinfo(width: i32, height: i32, bpp: u16, compression: u32) -> Vec<u8> {
    let mut body = Vec::new();
    put_u32(&mut body, 40); // biSize
    body.extend_from_slice(&width.to_le_bytes());
    body.extend_from_slice(&height.to_le_bytes());
    put_u16(&mut body, 1); // biPlanes
    put_u16(&mut body, bpp);
    put_u32(&mut body, compression);
    put_u32(&mut body, 0); // biSizeImage
    put_u32(&mut body, 0); // biXPelsPerMeter
    put_u32(&mut body, 0); // biYPelsPerMeter
    put_u32(&mut body, 0); // biClrUsed
    put_u32(&mut body, 0); // biClrImportant
    body
}

fn strl(strh_body: &[u8], strf_body: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    chunk(&mut body, b"strh", strh_body);
    chunk(&mut body, b"strf", strf_body);
    body
}

/// Builds `RIFF AVI ` with `hdrl` + `movi` from per-stream headers and
/// an ordered movi chunk list `[(id, body)]`.
fn write_avi(strls: &[Vec<u8>], movi: &[([u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut hdrl = Vec::new();
    for s in strls {
        list(&mut hdrl, b"strl", s);
    }
    let mut movi_body = Vec::new();
    for (id, body) in movi {
        chunk(&mut movi_body, id, body);
    }
    let mut riff_body = Vec::new();
    list(&mut riff_body, b"hdrl", &hdrl);
    list(&mut riff_body, b"movi", &movi_body);
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    put_u32(&mut out, (riff_body.len() + 4) as u32);
    out.extend_from_slice(b"AVI ");
    out.extend_from_slice(&riff_body);
    if riff_body.len() % 2 == 1 {
        out.push(0);
    }
    out
}

/// 2×2 24bpp bottom-up DIB body: bottom row red, top row blue.
fn dib_2x2() -> Vec<u8> {
    let stride = 8; // 2 px * 3 bpp aligned to 4
    let mut body = vec![0u8; stride * 2];
    // bottom-up: first stored row = bottom = red (BGR)
    body[0..3].copy_from_slice(&[0, 0, 255]);
    body[3..6].copy_from_slice(&[0, 0, 255]);
    // second stored row = top = blue
    body[stride..stride + 3].copy_from_slice(&[255, 0, 0]);
    body[stride + 3..stride + 6].copy_from_slice(&[255, 0, 0]);
    body
}

fn av_audio_video() -> Vec<u8> {
    let vids = strl(&strh(b"vids", 1, 30, 0, 3, 0), &bitmapinfo(2, 2, 24, 0));
    let auds = strl(
        &strh(b"auds", 4, 192_000, 0, 16, 4),
        &waveformatex(1, 2, 48_000, 16),
    );
    let movi: Vec<([u8; 4], Vec<u8>)> = vec![
        (*b"00db", dib_2x2()),
        (*b"01wb", vec![1, 0, 2, 0, 3, 0, 4, 0]),
        (*b"00db", dib_2x2()),
        (*b"01wb", vec![5, 0, 6, 0, 7, 0, 8, 0]),
        (*b"00db", Vec::new()), // drop frame
    ];
    write_avi(&[vids, auds], &movi)
}

fn packets(demuxer: &mut AviDemuxerAlias) -> Vec<ruviform_media_io::Packet> {
    let mut out = Vec::new();
    while let Some(packet) = demuxer.read_packet().unwrap() {
        out.push(packet);
    }
    out
}

type AviDemuxerAlias = ruviform_io_avi::AviDemuxer;

// ---------- capability / probe ----------

#[test]
fn capability_table_declares_avi_scope() {
    let caps = capabilities();
    assert!(caps.functions.contains(Features::DEMUX));
    assert!(caps.functions.contains(Features::DECODE));
    assert!(caps.functions.contains(Features::NATIVE_AUDIO));
    assert!(!caps.functions.contains(Features::MUX));
    assert!(caps.decode_codecs.contains(&CodecId::Pcm));
    assert!(caps.decode_codecs.contains(&CodecId::Uncompressed));
}

#[test]
fn probe_enumerates_streams_with_declared_facts() {
    let demuxer = open_demuxer(std::io::Cursor::new(av_audio_video())).unwrap();
    assert_eq!(demuxer.container().format, ContainerFormat::Avi);
    let streams = demuxer.streams();
    assert_eq!(streams.len(), 2);

    let video = &streams[0];
    assert_eq!(video.stream.kind, StreamKind::Video);
    assert_eq!(video.codec.codec, CodecId::Uncompressed);
    assert_eq!(video.facts.codec_tag, Some(CodecTag::FourCc([0, 0, 0, 0])));
    assert_eq!(video.facts.decodable, Fact::Known(()));
    assert_eq!(video.facts.declared_units, Fact::Known(3));
    assert_eq!(video.facts.start, Fact::Known(0));
    let vf = video.facts.video.unwrap();
    assert_eq!(vf.width, Some(2));
    assert_eq!(vf.height, Some(2));
    assert_eq!(
        vf.nominal_frame_rate.unwrap(),
        ruviform_media_core::FrameRate::new(30, 1).unwrap()
    );
    match video.codec.format {
        CodecFormat::Video(v) => {
            assert_eq!(v.width, 2);
            assert_eq!(v.format, PixelFormat::Rgba8);
        }
        _ => panic!("expected video codec format"),
    }
    // Native BGR encoding is inexpressible → Unsupported, not a relabel.
    assert_eq!(video.codec.native_format, Fact::Unsupported);
    assert_eq!(
        video.stream.time_base,
        Fact::Known(ruviform_media_core::TimeBase::new(1, 30).unwrap())
    );

    let audio = &streams[1];
    assert_eq!(audio.stream.kind, StreamKind::Audio);
    assert_eq!(audio.codec.codec, CodecId::Pcm);
    assert_eq!(audio.facts.codec_tag, Some(CodecTag::Numeric(1)));
    assert_eq!(audio.facts.decodable, Fact::Known(()));
    assert_eq!(audio.facts.declared_units, Fact::Known(16));
    match audio.codec.native_format {
        Fact::Known(CodecFormat::Audio(f)) => {
            assert_eq!(f.sample_format, ruviform_audio_core::SampleFormat::S16);
            assert_eq!(f.sample_rate.get(), 48_000);
        }
        _ => panic!("expected native audio format"),
    }
}

#[test]
fn packets_carry_derived_ticks_and_ordinals() {
    let mut demuxer = open_demuxer(std::io::Cursor::new(av_audio_video())).unwrap();
    let packets = packets(&mut demuxer);
    assert_eq!(packets.len(), 5);
    // Interleaved container order, per-stream ordinals, derived ticks.
    let video_ticks: Vec<i64> = packets
        .iter()
        .filter(|p| p.stream.0 == 0)
        .map(|p| p.pts.unwrap().ticks)
        .collect();
    assert_eq!(video_ticks, [0, 1, 2]);
    let audio_ticks: Vec<i64> = packets
        .iter()
        .filter(|p| p.stream.0 == 1)
        .map(|p| p.pts.unwrap().ticks)
        .collect();
    // 8 body bytes * scale 4 / sample_size 4 = 8 ticks per wb chunk.
    assert_eq!(audio_ticks, [0, 8]);
    for packet in &packets {
        assert_eq!(packet.pts.unwrap().provenance, TimestampProvenance::Derived);
        assert_eq!(packet.dts, packet.pts);
        assert_eq!(
            packet.duration,
            Some(if packet.stream.0 == 0 { 1 } else { 8 })
        );
        if packet.stream.0 == 0 {
            assert!(packet.flags.contains(PacketFlags::KEYFRAME));
        }
    }
    assert_eq!(packets[0].index, 0);
    assert_eq!(packets[2].index, 1);
    assert_eq!(packets[4].index, 2); // the zero-length drop chunk emits
    assert_eq!(packets[4].data, [] as [u8; 0]);
    assert_eq!(packets[0].data, dib_2x2());
}

// ---------- decode ----------

#[test]
fn audio_decode_canonical_and_native() {
    let mut demuxer = open_demuxer(std::io::Cursor::new(av_audio_video())).unwrap();
    let mut canonical = demuxer
        .make_decoder(ruviform_media_core::StreamId(1))
        .unwrap();
    let mut native = demuxer
        .make_native_decoder(ruviform_media_core::StreamId(1))
        .unwrap();

    while let Some(packet) = demuxer.read_packet().unwrap() {
        if packet.stream.0 != 1 {
            continue;
        }
        canonical.send_packet(packet.clone()).unwrap();
        native.send_packet(packet).unwrap();
        let Frame::Audio(frame) = canonical.receive_frame().unwrap().unwrap() else {
            panic!("expected canonical audio frame");
        };
        assert_eq!(frame.pts().provenance, TimestampProvenance::Derived);
        let buffer = frame.buffer();
        assert_eq!(buffer.channels(), 2);
        // s16 samples 1..8 normalized to /32768.
        let expect_l = [
            1.0f32 / 32768.0,
            3.0 / 32768.0,
            5.0 / 32768.0,
            7.0 / 32768.0,
        ];
        let expect_r = [
            2.0f32 / 32768.0,
            4.0 / 32768.0,
            6.0 / 32768.0,
            8.0 / 32768.0,
        ];
        if frame.pts().time.value() == 0 {
            assert_eq!(buffer.channel(0).unwrap(), &expect_l[..2]);
            assert_eq!(buffer.channel(1).unwrap(), &expect_r[..2]);
        } else {
            assert_eq!(buffer.channel(0).unwrap(), &expect_l[2..]);
            assert_eq!(buffer.channel(1).unwrap(), &expect_r[2..]);
        }

        let Frame::NativeAudio(native_frame) = native.receive_frame().unwrap().unwrap() else {
            panic!("expected native audio frame");
        };
        assert_eq!(
            native_frame.format().sample_format,
            ruviform_audio_core::SampleFormat::S16
        );
        assert_eq!(
            native_frame.transforms(),
            &[ruviform_media_io::PcmTransform::IntegerWidenedToF64]
        );
        let s = native_frame.samples();
        if native_frame.pts().ticks == 0 {
            assert_eq!(s, &[1.0, 2.0, 3.0, 4.0]);
        } else {
            assert_eq!(s, &[5.0, 6.0, 7.0, 8.0]);
        }
    }
}

#[test]
fn video_decode_swizzles_flips_and_repeats_drops() {
    let mut demuxer = open_demuxer(std::io::Cursor::new(av_audio_video())).unwrap();
    let mut decoder = demuxer
        .make_decoder(ruviform_media_core::StreamId(0))
        .unwrap();
    let mut frames = Vec::new();
    while let Some(packet) = demuxer.read_packet().unwrap() {
        if packet.stream.0 != 0 {
            continue;
        }
        decoder.send_packet(packet).unwrap();
        while let Some(frame) = decoder.receive_frame().unwrap() {
            frames.push(frame);
        }
    }
    assert_eq!(frames.len(), 3);
    for (i, frame) in frames.iter().enumerate() {
        let Frame::Video(v) = frame else {
            panic!("expected video frame")
        };
        assert_eq!(v.format(), PixelFormat::Rgba8);
        assert_eq!((v.width(), v.height()), (2, 2));
        assert_eq!(v.pts().value(), i as i64); // base 1/30 → tick = frame index
        let plane = v.plane(0).unwrap();
        // top row = blue (RGBA), bottom row = red.
        assert_eq!(plane.row(0).unwrap()[0..4], [0, 0, 255, 255]);
        assert_eq!(plane.row(1).unwrap()[0..4], [255, 0, 0, 255]);
    }
}

// ---------- explicit unsupported / malformed ----------

#[test]
fn compressed_video_reports_unsupported_not_decodable() {
    let vids = strl(
        &strh(b"vids", 1, 30, 0, 1, 0),
        &bitmapinfo(2, 2, 24, u32::from_le_bytes(*b"MJPG")),
    );
    let movi: Vec<([u8; 4], Vec<u8>)> = vec![(*b"00dc", vec![0xAA; 16])];
    let mut demuxer = open_demuxer(std::io::Cursor::new(write_avi(&[vids], &movi))).unwrap();
    let info = &demuxer.streams()[0];
    assert_eq!(info.codec.codec, CodecId::Mjpeg);
    assert_eq!(info.facts.codec_tag, Some(CodecTag::FourCc(*b"MJPG")));
    assert_eq!(info.facts.decodable, Fact::Unsupported);
    let err = demuxer
        .make_decoder(ruviform_media_core::StreamId(0))
        .err()
        .unwrap();
    assert_eq!(err.kind(), ErrorKind::Unsupported);
    // Packets still demux with their raw payload and derived ticks.
    let packet = demuxer.read_packet().unwrap().unwrap();
    assert_eq!(packet.data, vec![0xAA; 16]);
    assert_eq!(packet.flags, PacketFlags::NONE);
}

#[test]
fn unsupported_audio_tag_reports_unsupported() {
    // wFormatTag 0x0055 = MP3 — declared, not decodable.
    let mut strf = waveformatex(0x0055, 2, 48_000, 16);
    strf[12] = 1; // block_align 1 — still consistent-ish, unsupported anyway
    let auds = strl(&strh(b"auds", 1, 48_000, 0, 4, 1), &strf);
    let movi: Vec<([u8; 4], Vec<u8>)> = vec![(*b"00wb", vec![0; 8])];
    let demuxer = open_demuxer(std::io::Cursor::new(write_avi(&[auds], &movi))).unwrap();
    let info = &demuxer.streams()[0];
    assert_eq!(info.codec.codec, CodecId::Other);
    assert_eq!(info.facts.codec_tag, Some(CodecTag::Numeric(0x0055)));
    assert_eq!(info.facts.decodable, Fact::Unsupported);
    let err = demuxer
        .make_decoder(ruviform_media_core::StreamId(0))
        .err()
        .unwrap();
    assert_eq!(err.kind(), ErrorKind::Unsupported);
}

#[test]
fn malformed_and_non_avi_fail_explicitly() {
    // Not RIFF at all → capability, not corruption.
    let err = open_demuxer(std::io::Cursor::new(b"NOPE".to_vec()))
        .err()
        .unwrap();
    assert_eq!(err.kind(), ErrorKind::Unsupported);
    // RIFF but not AVI.
    let mut not_avi = Vec::new();
    not_avi.extend_from_slice(b"RIFF");
    put_u32(&mut not_avi, 4);
    not_avi.extend_from_slice(b"WAVE");
    let err = open_demuxer(std::io::Cursor::new(not_avi)).err().unwrap();
    assert_eq!(err.kind(), ErrorKind::Unsupported);
    // RIFF size ≠ source length → malformed.
    let mut avi = av_audio_video();
    avi[7] = 0x10; // inflate declared RIFF size
    let err = open_demuxer(std::io::Cursor::new(avi)).err().unwrap();
    assert_eq!(err.kind(), ErrorKind::InvalidData);
}

#[test]
fn malformed_strf_marks_decodable_malformed() {
    // BITMAPINFOHEADER truncated below 40 bytes.
    let vids = strl(&strh(b"vids", 1, 30, 0, 1, 0), &[1, 2, 3]);
    let movi: Vec<([u8; 4], Vec<u8>)> = vec![(*b"00db", vec![0; 8])];
    let demuxer = open_demuxer(std::io::Cursor::new(write_avi(&[vids], &movi))).unwrap();
    assert_eq!(demuxer.streams()[0].facts.decodable, Fact::Malformed);
    let err = demuxer
        .make_decoder(ruviform_media_core::StreamId(0))
        .err()
        .unwrap();
    assert_eq!(err.kind(), ErrorKind::InvalidData);
}

#[test]
fn seek_reports_unsupported() {
    let mut demuxer = open_demuxer(std::io::Cursor::new(av_audio_video())).unwrap();
    let err = demuxer
        .seek(ruviform_media_core::MediaTime::new(
            0,
            ruviform_media_core::TimeBase::COMMON_90K,
        ))
        .err()
        .unwrap();
    assert_eq!(err.kind(), ErrorKind::Unsupported);
}

/// Non-seekable wrapper — exercises the read-discard skip path.
struct NoSeek(std::io::Cursor<Vec<u8>>);

impl std::io::Read for NoSeek {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

impl std::io::Seek for NoSeek {
    fn seek(&mut self, _pos: std::io::SeekFrom) -> std::io::Result<u64> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "no seek",
        ))
    }
}

#[test]
fn non_seekable_source_still_demuxes() {
    let source = NoSeek(std::io::Cursor::new(av_audio_video()));
    let mut demuxer = open_demuxer(source).unwrap();
    let packets = packets(&mut demuxer);
    assert_eq!(packets.len(), 5);
}

// ---------- Issue #197 boundary additions ----------

#[test]
fn boundary_seekable_fact_declares_unsupported() {
    let demuxer = open_demuxer(std::io::Cursor::new(av_audio_video())).unwrap();
    // AVI seek is not implemented — declared honestly at construction.
    assert_eq!(demuxer.seekable(), Fact::Unsupported);
}

#[test]
fn boundary_make_decoder_dispatches_both_modes() {
    use ruviform_media_io::DecodeMode;

    let demuxer = open_demuxer(std::io::Cursor::new(av_audio_video())).unwrap();
    let streams = demuxer.streams();
    let audio = streams
        .iter()
        .find(|s| s.stream.kind == StreamKind::Audio)
        .unwrap()
        .stream
        .id;

    let _canonical = Demuxer::make_decoder(&demuxer, audio, DecodeMode::Canonical).unwrap();
    let _native = Demuxer::make_decoder(&demuxer, audio, DecodeMode::Native).unwrap();
}
