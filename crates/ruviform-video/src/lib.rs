//! `ruviform-video` — video engine (Issue #27).
//!
//! Responsibilities (architecture baseline): frame pipeline, A/V sync
//! via audio master + PTS follow, seek mediation. Codec and UI live
//! elsewhere; the engine drives `ruviform-media-io` boundary traits.
//!
//! **RT contract:** every public API is non-RT. The A/V sync face
//! reads [`ruviform_rt::MasterClock`] as a non-blocking value load —
//! the only RT-adjacent surface, per the baseline's engine contract.

mod pipeline;
mod session;
mod sync;

pub use pipeline::{FramePipeline, VideoStage};
pub use session::VideoSession;
pub use sync::{FrameScheduler, SyncDecision, SyncThresholds};

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    use ruviform_media_core::{MediaTime, StreamId, TimeBase};
    use ruviform_media_io::{Decoder, Demuxer, Frame, Packet, PacketFlags, Result, StreamInfo};
    use ruviform_rt::MasterClock;
    use ruviform_video_core::{PixelFormat, VideoFrame};

    use super::*;

    const TB: TimeBase = TimeBase::COMMON_90K;

    fn frame_at(secs_90k: i64) -> VideoFrame {
        VideoFrame::new_owned(PixelFormat::Yuv420p, 4, 2, MediaTime::new(secs_90k, TB)).unwrap()
    }

    fn packet() -> Packet {
        Packet {
            stream: StreamId(0),
            pts: None,
            dts: None,
            duration: None,
            flags: PacketFlags::empty(),
            data: Vec::new(),
        }
    }

    struct MockDemuxer {
        packets: VecDeque<Packet>,
        events: Arc<Mutex<Vec<&'static str>>>,
    }
    impl Demuxer for MockDemuxer {
        fn streams(&self) -> &[StreamInfo] {
            &[]
        }
        fn read_packet(&mut self) -> Result<Option<Packet>> {
            Ok(self.packets.pop_front())
        }
        fn seek(&mut self, _position: MediaTime) -> Result<()> {
            self.events.lock().unwrap().push("demux_seek");
            Ok(())
        }
    }

    struct MockDecoder {
        queue: VecDeque<Frame>,
        events: Arc<Mutex<Vec<&'static str>>>,
    }
    impl Decoder for MockDecoder {
        fn send_packet(&mut self, _packet: Packet) -> Result<()> {
            Ok(())
        }
        fn receive_frame(&mut self) -> Result<Option<Frame>> {
            self.events.lock().unwrap().push("receive");
            Ok(self.queue.pop_front())
        }
        fn flush(&mut self) -> Result<()> {
            self.events.lock().unwrap().push("flush");
            Ok(())
        }
    }

    fn session(
        packets: VecDeque<Packet>,
        queue: VecDeque<Frame>,
    ) -> (
        VideoSession<MockDemuxer, MockDecoder>,
        Arc<Mutex<Vec<&'static str>>>,
    ) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let s = VideoSession::new(
            MockDemuxer {
                packets,
                events: Arc::clone(&events),
            },
            MockDecoder {
                queue,
                events: Arc::clone(&events),
            },
            FramePipeline::new(),
            FrameScheduler::new(SyncThresholds::default()),
        );
        (s, events)
    }

    #[test]
    fn scheduler_decision_table() {
        let th = SyncThresholds {
            early: 0.040,
            drop_late: 0.040,
        };
        let sched = FrameScheduler::new(th);
        let now = MediaTime::new(90_000, TB); // 1.0s

        // early > threshold → Wait
        let early = MediaTime::new(90_000 + 9_000, TB); // +0.1s
        assert_eq!(sched.decide(early, now), SyncDecision::Wait(early));
        // within tolerance → Present
        let on_time = MediaTime::new(90_000 + 1_000, TB); // +11ms
        assert_eq!(sched.decide(on_time, now), SyncDecision::Present);
        // late beyond drop → Drop
        let late = MediaTime::new(90_000 - 9_000, TB); // -0.1s
        assert_eq!(sched.decide(late, now), SyncDecision::Drop);
        // floor: seek resync drops frames predating target
        let mut sched = FrameScheduler::new(th);
        sched.resync_at(now);
        assert_eq!(sched.decide(late, now), SyncDecision::Drop);
        sched.reset();
        assert_eq!(sched.decide(on_time, now), SyncDecision::Present);
    }

    struct Tag(u8, Arc<Mutex<Vec<u8>>>);
    impl VideoStage for Tag {
        fn process(&mut self, frame: VideoFrame) -> Result<VideoFrame> {
            self.1.lock().unwrap().push(self.0);
            Ok(frame)
        }
    }

    #[test]
    fn pipeline_applies_stages_in_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut p = FramePipeline::new();
        p.push(Box::new(Tag(1, Arc::clone(&log))));
        p.push(Box::new(Tag(2, Arc::clone(&log))));
        p.run(frame_at(0)).unwrap();
        assert_eq!(*log.lock().unwrap(), vec![1, 2]);
    }

    #[test]
    fn session_drives_decode_loop() {
        // decoder queue starts empty → session reads packet, feeds it,
        // then receives the video frame through the pipeline.
        let mut packets = VecDeque::new();
        packets.push_back(packet());
        let mut queue = VecDeque::new();
        queue.push_back(Frame::Video(frame_at(45_000)));
        // deliver after one packet: emulate by letting receive_frame
        // return the frame immediately — but first call is a miss.
        let (mut s, _ev) = session(packets, queue);
        let got = s.next_frame().unwrap();
        assert_eq!(got.unwrap().pts(), MediaTime::new(45_000, TB));
        // EOS: next call drains and returns None
        assert!(s.next_frame().unwrap().is_none());
        assert!(s.next_frame().unwrap().is_none());
    }

    #[test]
    fn session_seek_resyncs() {
        let (mut s, events) = session(VecDeque::new(), VecDeque::new());
        let target = MediaTime::new(180_000, TB);
        s.seek(target).unwrap();
        assert_eq!(*events.lock().unwrap(), vec!["demux_seek", "flush"]);
        // stale frame predating the seek is dropped on decide
        let stale = frame_at(45_000);
        let clock = MasterClock::new(1, 90_000).unwrap();
        clock.publish_tick(180_000);
        assert_eq!(s.decide(&stale, &clock), SyncDecision::Drop);
        let on_time = frame_at(181_000);
        assert_eq!(s.decide(&on_time, &clock), SyncDecision::Present);
    }

    #[test]
    fn audio_frame_is_skipped() {
        use ruviform_audio_core::{AudioBuffer, BufferSpec, ChannelLayout, SampleRate};
        use ruviform_media_io::AudioFrame;
        let buffer = AudioBuffer::new(BufferSpec {
            layout: ChannelLayout::Mono,
            frames: 4,
            sample_rate: SampleRate::new(48_000).unwrap(),
        });
        let mut queue = VecDeque::new();
        queue.push_back(Frame::Audio(AudioFrame::new(buffer, MediaTime::new(0, TB))));
        queue.push_back(Frame::Video(frame_at(90_000)));
        let (mut s, _ev) = session(VecDeque::new(), queue);
        let got = s.next_frame().unwrap();
        assert_eq!(got.unwrap().pts(), MediaTime::new(90_000, TB));
    }
}
