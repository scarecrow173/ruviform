//! Integration coverage for ISOBMFF (MP4) demux/decode through the
//! media-io boundary.
//!
//! Fixtures are generated in-test by a minimal MP4 writer (ported from
//! the `AudioLens` `al-testkit` synthetic writer, trimmed to what the
//! coverage needs) — no binary assets. MP4 coverage is feature-gated:
//! `mp4` enables the isomp4 demuxer, `aac` additionally enables the AAC
//! decoder.
// The byte-writer is fixture plumbing: fourcc-shaped binding names and
// fixed-width field casts are the point, and the writer is one long
// serialization routine by nature.
#![allow(
    clippy::cast_possible_truncation,
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref
)]

use ruviform_io_symphonia as io_symphonia;

// ---------------------------------------------------------------------------
// Minimal synthetic MP4 writer (ftyp + mdat + moov).
// ---------------------------------------------------------------------------

fn put_box(out: &mut Vec<u8>, tag: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(&((body.len() + 8) as u32).to_be_bytes());
    out.extend_from_slice(tag);
    out.extend_from_slice(body);
}

fn full_body(version_flags: u32, rest: &[u8]) -> Vec<u8> {
    let mut b = Vec::with_capacity(rest.len() + 4);
    b.extend_from_slice(&version_flags.to_be_bytes());
    b.extend_from_slice(rest);
    b
}

enum Entry {
    Sound {
        codec: [u8; 4],
        channels: u16,
        bits: u16,
        rate: u32,
    },
    #[cfg(feature = "mp4")]
    Video {
        codec: [u8; 4],
        width: u16,
        height: u16,
    },
}

struct Track {
    id: u32,
    handler: [u8; 4],
    timescale: u32,
    duration: u32,
    entry: Entry,
    /// `stts` runs: (`sample_count`, delta ticks).
    stts: Vec<(u32, u32)>,
    /// `stsc` runs: (`first_chunk`, `samples_per_chunk`, `desc_index`).
    stsc: Vec<(u32, u32, u32)>,
    /// `stss` sync sample indices (1-based); `None` = all sync.
    stss: Option<Vec<u32>>,
    /// Chunk payloads in mdat order.
    chunks: Vec<Vec<u8>>,
    /// `stsz` per-sample sizes.
    sizes: Vec<u32>,
}

struct Spec {
    tracks: Vec<Track>,
}

fn sound_body(channels: u16, bits: u16, rate: u32) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&[0u8; 6]); // reserved
    b.extend_from_slice(&1u16.to_be_bytes()); // data_reference_index
    b.extend_from_slice(&0u16.to_be_bytes()); // version 0
    b.extend_from_slice(&0u16.to_be_bytes()); // revision
    b.extend_from_slice(&0u32.to_be_bytes()); // vendor
    b.extend_from_slice(&channels.to_be_bytes());
    b.extend_from_slice(&bits.to_be_bytes());
    b.extend_from_slice(&0u16.to_be_bytes()); // compression id
    b.extend_from_slice(&0u16.to_be_bytes()); // packet size
    b.extend_from_slice(&(rate << 16).to_be_bytes()); // 16.16 rate
    b
}

#[cfg(feature = "mp4")]
fn video_body(width: u16, height: u16) -> Vec<u8> {
    let mut b = vec![0u8; 78];
    b[6..8].copy_from_slice(&1u16.to_be_bytes()); // data_reference_index
    b[24..26].copy_from_slice(&width.to_be_bytes());
    b[26..28].copy_from_slice(&height.to_be_bytes());
    b[40..42].copy_from_slice(&1u16.to_be_bytes()); // frame_count
    b[74..76].copy_from_slice(&24u16.to_be_bytes()); // depth
    b[76..78].copy_from_slice(&(-1i16).to_be_bytes()); // pre_defined
    b
}

/// Serializes `ftyp`, `mdat`, `moov` — `moov` last so `stco` offsets
/// are known (offsets assume no extra top-level boxes before `mdat`).
fn write_mp4(spec: &Spec) -> Vec<u8> {
    let mut ftyp = Vec::new();
    ftyp.extend_from_slice(b"isom");
    ftyp.extend_from_slice(&0u32.to_be_bytes());
    ftyp.extend_from_slice(b"isom");

    let mdat_start = 8 + ftyp.len() + 8;
    let mut chunk_offsets: Vec<Vec<u64>> = Vec::new();
    let mut mdat = Vec::new();
    let mut off = mdat_start as u64;
    for t in &spec.tracks {
        let mut offs = Vec::new();
        for c in &t.chunks {
            offs.push(off);
            off += c.len() as u64;
            mdat.extend_from_slice(c);
        }
        chunk_offsets.push(offs);
    }

    let mut moov = Vec::new();
    let mut mvhd = full_body(0, &[]);
    mvhd.extend_from_slice(&[0u8; 8]);
    mvhd.extend_from_slice(&1000u32.to_be_bytes()); // movie timescale
    mvhd.extend_from_slice(&0u32.to_be_bytes()); // duration
    mvhd.extend_from_slice(&0x0001_0000u32.to_be_bytes()); // rate
    mvhd.extend_from_slice(&0x0100u16.to_be_bytes()); // volume
    mvhd.extend_from_slice(&[0u8; 10]);
    mvhd.extend_from_slice(&[0u8; 36]);
    mvhd.extend_from_slice(&[0u8; 24]);
    mvhd.extend_from_slice(&1u32.to_be_bytes());
    put_box(&mut moov, b"mvhd", &mvhd);

    for (ti, t) in spec.tracks.iter().enumerate() {
        let mut tkhd = full_body(7, &[]);
        tkhd.extend_from_slice(&[0u8; 8]);
        tkhd.extend_from_slice(&t.id.to_be_bytes());
        tkhd.extend_from_slice(&0u32.to_be_bytes());
        tkhd.extend_from_slice(&t.duration.to_be_bytes());
        tkhd.extend_from_slice(&[0u8; 8]);
        tkhd.extend_from_slice(&[0u8; 4]); // layer + alt group
        tkhd.extend_from_slice(&if t.handler == *b"soun" { 0x0100u16 } else { 0 }.to_be_bytes());
        tkhd.extend_from_slice(&[0u8; 2]);
        tkhd.extend_from_slice(&[0u8; 36]);
        let (w, h) = match &t.entry {
            #[cfg(feature = "mp4")]
            Entry::Video { width, height, .. } => (*width, *height),
            _ => (0u16, 0u16),
        };
        tkhd.extend_from_slice(&(u32::from(w) << 16).to_be_bytes());
        tkhd.extend_from_slice(&(u32::from(h) << 16).to_be_bytes());

        let mut mdhd = full_body(0, &[]);
        mdhd.extend_from_slice(&[0u8; 8]);
        mdhd.extend_from_slice(&t.timescale.to_be_bytes());
        mdhd.extend_from_slice(&t.duration.to_be_bytes());
        mdhd.extend_from_slice(&0u16.to_be_bytes()); // language: und
        mdhd.extend_from_slice(&0u16.to_be_bytes()); // quality

        let mut hdlr = full_body(0, &[]);
        hdlr.extend_from_slice(&0u32.to_be_bytes());
        hdlr.extend_from_slice(&t.handler);
        hdlr.extend_from_slice(&[0u8; 12]);
        hdlr.push(0);

        let (mhtag, mhbody) = if t.handler == *b"soun" {
            (b"smhd", full_body(0, &[0u8; 4]))
        } else {
            (b"vmhd", full_body(1, &[0u8; 8]))
        };
        let mut minf = Vec::new();
        put_box(&mut minf, mhtag, &mhbody);
        let mut dref = full_body(0, &[]);
        dref.extend_from_slice(&1u32.to_be_bytes());
        put_box(&mut dref, b"url ", &full_body(1, &[]));
        let mut dinf = Vec::new();
        put_box(&mut dinf, b"dref", &dref);
        minf.extend_from_slice(&dinf);

        let mut stbl = Vec::new();
        let mut stsd = full_body(0, &[]);
        stsd.extend_from_slice(&1u32.to_be_bytes());
        let (etag, ebody) = match &t.entry {
            Entry::Sound {
                codec,
                channels,
                bits,
                rate,
            } => (codec, sound_body(*channels, *bits, *rate)),
            #[cfg(feature = "mp4")]
            Entry::Video {
                codec,
                width,
                height,
            } => (codec, video_body(*width, *height)),
        };
        put_box(&mut stsd, etag, &ebody);
        put_box(&mut stbl, b"stsd", &stsd);

        let mut stts = full_body(0, &[]);
        stts.extend_from_slice(&(t.stts.len() as u32).to_be_bytes());
        for &(count, delta) in &t.stts {
            stts.extend_from_slice(&count.to_be_bytes());
            stts.extend_from_slice(&delta.to_be_bytes());
        }
        put_box(&mut stbl, b"stts", &stts);

        let mut stsc = full_body(0, &[]);
        stsc.extend_from_slice(&(t.stsc.len() as u32).to_be_bytes());
        for &(first, spc, sd) in &t.stsc {
            stsc.extend_from_slice(&first.to_be_bytes());
            stsc.extend_from_slice(&spc.to_be_bytes());
            stsc.extend_from_slice(&sd.to_be_bytes());
        }
        put_box(&mut stbl, b"stsc", &stsc);

        if let Some(stss) = &t.stss {
            let mut body = full_body(0, &[]);
            body.extend_from_slice(&(stss.len() as u32).to_be_bytes());
            for &i in stss {
                body.extend_from_slice(&i.to_be_bytes());
            }
            put_box(&mut stbl, b"stss", &body);
        }

        let mut stsz = full_body(0, &[]);
        stsz.extend_from_slice(&0u32.to_be_bytes());
        stsz.extend_from_slice(&(t.sizes.len() as u32).to_be_bytes());
        for &s in &t.sizes {
            stsz.extend_from_slice(&s.to_be_bytes());
        }
        put_box(&mut stbl, b"stsz", &stsz);

        let mut stco = full_body(0, &[]);
        stco.extend_from_slice(&(chunk_offsets[ti].len() as u32).to_be_bytes());
        for &o in &chunk_offsets[ti] {
            stco.extend_from_slice(&(o as u32).to_be_bytes());
        }
        put_box(&mut stbl, b"stco", &stco);

        put_box(&mut minf, b"stbl", &stbl);
        let mut mdia = Vec::new();
        put_box(&mut mdia, b"mdhd", &mdhd);
        put_box(&mut mdia, b"hdlr", &hdlr);
        put_box(&mut mdia, b"minf", &minf);
        let mut trak = Vec::new();
        put_box(&mut trak, b"tkhd", &tkhd);
        put_box(&mut trak, b"mdia", &mdia);
        put_box(&mut moov, b"trak", &trak);
    }

    let mut out = Vec::new();
    put_box(&mut out, b"ftyp", &ftyp);
    put_box(&mut out, b"mdat", &mdat);
    put_box(&mut out, b"moov", &moov);
    out
}

/// One 16-bit stereo PCM `sowt` track: 4 samples × 4 bytes, 1 kHz
/// timescale, one chunk — interleaved little-endian samples.
fn pcm_mp4() -> Vec<u8> {
    let samples: [i16; 8] = [1000, -1000, 2000, -2000, 3000, -3000, 4000, -4000];
    let mut chunk = Vec::new();
    for s in samples {
        chunk.extend_from_slice(&s.to_le_bytes());
    }
    write_mp4(&Spec {
        tracks: vec![Track {
            id: 1,
            handler: *b"soun",
            timescale: 1000,
            duration: 4,
            entry: Entry::Sound {
                codec: *b"sowt",
                channels: 2,
                bits: 16,
                rate: 1000,
            },
            stts: vec![(4, 250)],
            stsc: vec![(1, 4, 1)],
            stss: None,
            chunks: vec![chunk],
            sizes: vec![4; 4],
        }],
    })
}

/// One `avc1` video track (undecodable payloads — demux/facts only).
#[cfg(feature = "mp4")]
fn av_mp4() -> Vec<u8> {
    write_mp4(&Spec {
        tracks: vec![
            Track {
                id: 1,
                handler: *b"soun",
                timescale: 1000,
                duration: 4,
                entry: Entry::Sound {
                    codec: *b"sowt",
                    channels: 1,
                    bits: 16,
                    rate: 1000,
                },
                stts: vec![(4, 250)],
                stsc: vec![(1, 4, 1)],
                stss: None,
                chunks: vec![vec![0u8; 8]],
                sizes: vec![2; 4],
            },
            Track {
                id: 2,
                handler: *b"vide",
                timescale: 90000,
                duration: 60000,
                entry: Entry::Video {
                    codec: *b"avc1",
                    width: 64,
                    height: 36,
                },
                stts: vec![(2, 3000)],
                stsc: vec![(1, 2, 1)],
                stss: Some(vec![1]),
                chunks: vec![vec![0xAAu8; 8]],
                sizes: vec![4; 2],
            },
        ],
    })
}

// ---------------------------------------------------------------------------
// Tests — `mp4` feature (container demux, PCM decode, video facts).
// ---------------------------------------------------------------------------

/// Without the `mp4` feature an ISOBMFF file must fail explicitly as
/// `Unsupported` — never silently decoded or misprobed.
#[cfg(not(feature = "mp4"))]
#[test]
fn mp4_without_feature_fails_unsupported() {
    use ruviform_media_io::ErrorKind;
    let Err(err) = io_symphonia::open_demuxer(std::io::Cursor::new(pcm_mp4())) else {
        panic!("mp4 probe must fail without the mp4 feature");
    };
    assert_eq!(err.kind(), ErrorKind::Unsupported);
}

#[cfg(feature = "mp4")]
mod mp4 {
    use super::*;
    use ruviform_media_core::{Fact, StreamId, StreamKind, TimestampProvenance};
    use ruviform_media_io::{
        Capabilities, CodecId, ContainerFormat, Decoder, Demuxer, ErrorKind, Frame,
    };

    #[test]
    fn capabilities_reflect_aac_feature() {
        let caps: Capabilities = io_symphonia::capabilities();
        assert!(caps.decode_codecs.contains(&CodecId::Pcm));
        #[cfg(feature = "aac")]
        assert!(caps.decode_codecs.contains(&CodecId::Aac));
        #[cfg(not(feature = "aac"))]
        assert!(!caps.decode_codecs.contains(&CodecId::Aac));
    }

    #[test]
    fn pcm_track_demuxes_with_timescale_and_ticks() {
        let demuxer = io_symphonia::open_demuxer(std::io::Cursor::new(pcm_mp4())).unwrap();
        assert_eq!(demuxer.container().format, ContainerFormat::Isobmff);
        let streams = demuxer.streams();
        assert_eq!(streams.len(), 1);
        let info = &streams[0];
        assert_eq!(info.stream.kind, StreamKind::Audio);
        assert_eq!(info.codec.codec, CodecId::Pcm);
        assert_eq!(info.facts.decodable, Fact::Known(()));
        // mdhd timescale 1000 → time base 1/1000.
        let Fact::Known(base) = info.stream.time_base else {
            panic!("expected known time base");
        };
        assert_eq!(base.denominator(), 1000);

        let mut demuxer = demuxer;
        let packet = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(packet.stream, StreamId(1));
        let pts = packet.pts.expect("mp4 carries pts");
        assert_eq!(pts.ticks, 0);
        assert_eq!(pts.provenance, TimestampProvenance::Carried);
        assert_eq!(packet.duration, Some(250));
    }

    #[test]
    fn pcm_track_decodes_canonical_and_native() {
        let mut demuxer = io_symphonia::open_demuxer(std::io::Cursor::new(pcm_mp4())).unwrap();
        let stream_id = demuxer.streams()[0].stream.id;
        let packet = demuxer.read_packet().unwrap().unwrap();
        assert_eq!(packet.data.len(), 4); // one sample per packet: 2 ch × 2 B

        let mut decoder = demuxer.make_decoder(stream_id).unwrap();
        decoder.send_packet(packet).unwrap();
        match decoder.receive_frame().unwrap() {
            Some(Frame::Audio(frame)) => {
                assert_eq!(frame.buffer().frames(), 1);
                assert_eq!(frame.buffer().channels(), 2);
                // First interleaved sample pair: 1000 / -1000.
                let buf = frame.buffer();
                assert!((buf.channel(0).unwrap()[0] - 1000.0 / 32768.0).abs() < 1e-7);
                assert!((buf.channel(1).unwrap()[0] - (-1000.0 / 32768.0)).abs() < 1e-7);
            }
            _ => panic!("expected canonical audio frame"),
        }

        let mut demuxer = io_symphonia::open_demuxer(std::io::Cursor::new(pcm_mp4())).unwrap();
        let packet = demuxer.read_packet().unwrap().unwrap();
        let mut native = demuxer.make_native_decoder(stream_id).unwrap();
        native.send_packet(packet).unwrap();
        match native.receive_frame().unwrap() {
            Some(Frame::NativeAudio(frame)) => {
                assert_eq!(
                    frame.format().sample_format,
                    ruviform_audio_core::SampleFormat::S16
                );
                assert_eq!(frame.samples()[0], 1000.0);
            }
            _ => panic!("expected native audio frame"),
        }
    }

    #[test]
    fn video_track_carries_facts_but_decode_is_unsupported() {
        let mut demuxer = io_symphonia::open_demuxer(std::io::Cursor::new(av_mp4())).unwrap();
        let streams = demuxer.streams();
        assert_eq!(streams.len(), 2);
        let video_id = {
            let video = &streams[1];
            assert_eq!(video.stream.kind, StreamKind::Video);
            let facts = video.facts.video.as_ref().expect("video facts declared");
            assert_eq!(facts.width, Some(64));
            assert_eq!(facts.height, Some(36));
            assert_eq!(video.facts.decodable, Fact::Unsupported);
            video.stream.id
        };
        match demuxer.make_decoder(video_id) {
            Err(e) => assert_eq!(e.kind(), ErrorKind::Unsupported),
            Ok(_) => panic!("no video decoder may exist"),
        }

        // Packets still demux with carried ticks from stts.
        let mut video_packets = 0;
        while let Some(packet) = demuxer.read_packet().unwrap() {
            if packet.stream == video_id {
                video_packets += 1;
            }
        }
        assert_eq!(video_packets, 2);
    }
}
