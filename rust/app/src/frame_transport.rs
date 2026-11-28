//! Binary frame transport, port of packages/shared/src/frameTransport.ts.
//! A fixed 17-byte little-endian header — magic, version, flags, sequence,
//! float64 timestamp, stream-id length — precedes the stream id and payload.
//! Subscribers join mid-stream gated on the next keyframe: delta frames drop
//! until one arrives, the latest keyframe and codec config replay on
//! subscribe, and slow sockets shed queue rather than stall the publisher.

use std::collections::HashMap;
use std::sync::Mutex;

pub const FRAME_HEADER_FIXED_BYTES: usize = 17;
pub const FRAME_MAX_STREAM_ID_BYTES: usize = 255;
pub const FRAME_FLAG_KEYFRAME: u8 = 0b0000_0001;
pub const FRAME_FLAG_CODEC_CONFIG: u8 = 0b0000_0010;

#[derive(Clone, Debug, PartialEq)]
pub struct FrameHeader {
    pub stream_id: String,
    pub sequence: u32,
    pub timestamp_ms: f64,
    pub keyframe: bool,
    pub codec_config: bool,
}

#[derive(Clone, Debug)]
pub struct FrameEnvelope {
    pub header: FrameHeader,
    pub payload: Vec<u8>,
}

#[derive(Clone, Copy, Debug)]
pub struct FrameCodecConfig {
    pub magic: u16,
    pub version: u8,
}

#[derive(Debug, PartialEq)]
pub enum DecodeError {
    TooShort,
    BadMagic,
    UnsupportedVersion,
    TruncatedStreamId,
    InvalidStreamId,
}

pub fn encode_frame(config: FrameCodecConfig, frame: &FrameEnvelope) -> Result<Vec<u8>, String> {
    let stream_id_bytes = frame.header.stream_id.as_bytes();
    if stream_id_bytes.is_empty() {
        return Err("frame header requires a non-empty stream id".to_string());
    }
    if stream_id_bytes.len() > FRAME_MAX_STREAM_ID_BYTES {
        return Err(format!(
            "frame stream id exceeds {FRAME_MAX_STREAM_ID_BYTES} UTF-8 bytes"
        ));
    }
    let mut out =
        Vec::with_capacity(FRAME_HEADER_FIXED_BYTES + stream_id_bytes.len() + frame.payload.len());
    let mut flags = 0u8;
    if frame.header.keyframe {
        flags |= FRAME_FLAG_KEYFRAME;
    }
    if frame.header.codec_config {
        flags |= FRAME_FLAG_CODEC_CONFIG;
    }
    out.extend_from_slice(&config.magic.to_le_bytes());
    out.push(config.version);
    out.push(flags);
    out.extend_from_slice(&frame.header.sequence.to_le_bytes());
    out.extend_from_slice(&frame.header.timestamp_ms.to_le_bytes());
    out.push(stream_id_bytes.len() as u8);
    out.extend_from_slice(stream_id_bytes);
    out.extend_from_slice(&frame.payload);
    Ok(out)
}

pub fn decode_frame(config: FrameCodecConfig, bytes: &[u8]) -> Result<FrameEnvelope, DecodeError> {
    if bytes.len() < FRAME_HEADER_FIXED_BYTES {
        return Err(DecodeError::TooShort);
    }
    if u16::from_le_bytes([bytes[0], bytes[1]]) != config.magic {
        return Err(DecodeError::BadMagic);
    }
    if bytes[2] != config.version {
        return Err(DecodeError::UnsupportedVersion);
    }
    let flags = bytes[3];
    let sequence = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let timestamp_ms = f64::from_le_bytes(bytes[8..16].try_into().unwrap());
    let stream_id_len = bytes[16] as usize;
    let payload_offset = FRAME_HEADER_FIXED_BYTES + stream_id_len;
    if stream_id_len == 0 || bytes.len() < payload_offset {
        return Err(DecodeError::TruncatedStreamId);
    }
    let stream_id = std::str::from_utf8(&bytes[FRAME_HEADER_FIXED_BYTES..payload_offset])
        .map_err(|_| DecodeError::InvalidStreamId)?
        .to_string();
    Ok(FrameEnvelope {
        header: FrameHeader {
            stream_id,
            sequence,
            timestamp_ms,
            keyframe: flags & FRAME_FLAG_KEYFRAME != 0,
            codec_config: flags & FRAME_FLAG_CODEC_CONFIG != 0,
        },
        payload: bytes[payload_offset..].to_vec(),
    })
}

pub trait FrameSink: Send {
    fn send(&mut self, bytes: &[u8]);
    fn is_open(&self) -> bool;
    /// Buffered-but-unwritten byte count; delivery sheds queue past budget.
    fn buffered_amount(&self) -> usize {
        0
    }
}

struct Subscriber {
    id: u64,
    sink: Box<dyn FrameSink>,
    queue: std::collections::VecDeque<Vec<u8>>,
    queued_bytes: usize,
    awaiting_keyframe: bool,
    sent: u64,
    dropped: u64,
}

pub struct FrameSubscriberStats {
    pub sent: u64,
    pub dropped: u64,
    pub awaiting_keyframe: bool,
    pub queued: usize,
}

pub struct FrameTransport {
    config: FrameCodecConfig,
    subscribers: Mutex<HashMap<String, Vec<Subscriber>>>,
    latest_keyframe: Mutex<HashMap<String, Vec<u8>>>,
    codec_config_frames: Mutex<HashMap<String, Vec<u8>>>,
    next_id: Mutex<u64>,
    pub queue_limit: usize,
    pub socket_budget_bytes: usize,
}

impl FrameTransport {
    pub fn new(config: FrameCodecConfig) -> Self {
        Self {
            config,
            subscribers: Mutex::new(HashMap::new()),
            latest_keyframe: Mutex::new(HashMap::new()),
            codec_config_frames: Mutex::new(HashMap::new()),
            next_id: Mutex::new(0),
            queue_limit: 8,
            socket_budget_bytes: 2 * 1024 * 1024,
        }
    }

    pub fn subscribe(&self, stream_id: &str, sink: Box<dyn FrameSink>) {
        let id = {
            let mut next = self.next_id.lock().unwrap();
            *next += 1;
            *next
        };
        let mut subscriber = Subscriber {
            id,
            sink,
            queue: std::collections::VecDeque::new(),
            queued_bytes: 0,
            awaiting_keyframe: true,
            sent: 0,
            dropped: 0,
        };
        // Replay codec config then the latest keyframe — a joining decoder is
        // usable immediately rather than at the next natural keyframe.
        if let Some(config_bytes) = self
            .codec_config_frames
            .lock()
            .unwrap()
            .get(stream_id)
            .cloned()
        {
            deliver(
                &mut subscriber,
                &config_bytes,
                true,
                self.queue_limit,
                self.socket_budget_bytes,
            );
        }
        if let Some(keyframe) = self.latest_keyframe.lock().unwrap().get(stream_id).cloned() {
            subscriber.awaiting_keyframe = false;
            deliver(
                &mut subscriber,
                &keyframe,
                false,
                self.queue_limit,
                self.socket_budget_bytes,
            );
        }
        self.subscribers
            .lock()
            .unwrap()
            .entry(stream_id.to_string())
            .or_default()
            .push(subscriber);
    }

    pub fn publish(&self, envelope: &FrameEnvelope) {
        let Ok(encoded) = encode_frame(self.config, envelope) else {
            return;
        };
        let stream_id = &envelope.header.stream_id;
        if envelope.header.codec_config {
            self.codec_config_frames
                .lock()
                .unwrap()
                .insert(stream_id.clone(), encoded.clone());
            self.latest_keyframe.lock().unwrap().remove(stream_id);
        } else if envelope.header.keyframe {
            self.latest_keyframe
                .lock()
                .unwrap()
                .insert(stream_id.clone(), encoded.clone());
        }
        let mut subscribers = self.subscribers.lock().unwrap();
        let Some(list) = subscribers.get_mut(stream_id) else {
            return;
        };
        let mut dead = Vec::new();
        for subscriber in list.iter_mut() {
            if !subscriber.sink.is_open() {
                dead.push(subscriber.id);
                continue;
            }
            if envelope.header.codec_config {
                subscriber.awaiting_keyframe = true;
                deliver(
                    subscriber,
                    &encoded,
                    true,
                    self.queue_limit,
                    self.socket_budget_bytes,
                );
                continue;
            }
            if subscriber.awaiting_keyframe {
                if !envelope.header.keyframe {
                    subscriber.dropped += 1;
                    continue;
                }
                subscriber.awaiting_keyframe = false;
            }
            deliver(
                subscriber,
                &encoded,
                false,
                self.queue_limit,
                self.socket_budget_bytes,
            );
        }
        list.retain(|s| !dead.contains(&s.id) && s.sink.is_open());
        if list.is_empty() {
            subscribers.remove(stream_id);
        }
    }

    pub fn reset(&self, stream_id: &str) {
        self.latest_keyframe.lock().unwrap().remove(stream_id);
        self.codec_config_frames.lock().unwrap().remove(stream_id);
        if let Some(list) = self.subscribers.lock().unwrap().get_mut(stream_id) {
            for subscriber in list.iter_mut() {
                subscriber.queue.clear();
                subscriber.queued_bytes = 0;
                subscriber.awaiting_keyframe = true;
            }
        }
    }

    pub fn stats_for(&self, stream_id: &str) -> Vec<FrameSubscriberStats> {
        self.subscribers
            .lock()
            .unwrap()
            .get(stream_id)
            .map(|list| {
                list.iter()
                    .map(|s| FrameSubscriberStats {
                        sent: s.sent,
                        dropped: s.dropped,
                        awaiting_keyframe: s.awaiting_keyframe,
                        queued: s.queue.len(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn subscriber_count(&self) -> usize {
        self.subscribers
            .lock()
            .unwrap()
            .values()
            .map(Vec::len)
            .sum()
    }
}

fn deliver(
    subscriber: &mut Subscriber,
    encoded: &[u8],
    essential: bool,
    queue_limit: usize,
    socket_budget_bytes: usize,
) {
    if subscriber.sink.buffered_amount() <= socket_budget_bytes && subscriber.queue.is_empty() {
        subscriber.sink.send(encoded);
        subscriber.sent += 1;
        return;
    }
    // Slow socket: queue within bound, shedding the oldest delta frames —
    // never the essential codec config — exactly upstream's refusal to stall
    // the publisher for one consumer.
    if subscriber.queue.len() >= queue_limit {
        let dropped_essential = subscriber
            .queue
            .front()
            .is_some_and(|front| front.len() >= 4 && front[3] & FRAME_FLAG_CODEC_CONFIG != 0);
        if essential || dropped_essential {
            return;
        }
        if let Some(evicted) = subscriber.queue.pop_front() {
            subscriber.queued_bytes = subscriber.queued_bytes.saturating_sub(evicted.len());
            subscriber.dropped += 1;
        }
    }
    subscriber.queued_bytes += encoded.len();
    subscriber.queue.push_back(encoded.to_vec());
    // Opportunistic flush: if the budget opened up, drain what fits.
    while let Some(front) = subscriber.queue.front() {
        if subscriber.sink.buffered_amount() > socket_budget_bytes {
            break;
        }
        let front = front.clone();
        subscriber.sink.send(&front);
        subscriber.sent += 1;
        subscriber.queue.pop_front();
        subscriber.queued_bytes = subscriber.queued_bytes.saturating_sub(front.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

    const CONFIG: FrameCodecConfig = FrameCodecConfig {
        magic: 0x5359,
        version: 1,
    };

    fn frame(stream: &str, seq: u32, keyframe: bool, codec: bool, payload: &[u8]) -> FrameEnvelope {
        FrameEnvelope {
            header: FrameHeader {
                stream_id: stream.to_string(),
                sequence: seq,
                timestamp_ms: seq as f64 * 16.6,
                keyframe,
                codec_config: codec,
            },
            payload: payload.to_vec(),
        }
    }

    struct TestSink {
        received: StdMutex<Vec<Vec<u8>>>,
        open: bool,
    }
    impl FrameSink for TestSink {
        fn send(&mut self, bytes: &[u8]) {
            self.received.lock().unwrap().push(bytes.to_vec());
        }
        fn is_open(&self) -> bool {
            self.open
        }
    }

    #[test]
    fn encode_decode_roundtrip() {
        let f = frame("main", 42, true, false, b"payload-bytes");
        let encoded = encode_frame(CONFIG, &f).unwrap();
        assert_eq!(encoded[0..2], 0x5359u16.to_le_bytes());
        let decoded = decode_frame(CONFIG, &encoded).unwrap();
        assert_eq!(decoded.header.stream_id, "main");
        assert_eq!(decoded.header.sequence, 42);
        assert!(decoded.header.keyframe);
        assert_eq!(decoded.payload, b"payload-bytes");
    }

    #[test]
    fn decode_rejects_bad_magic_and_truncation() {
        let f = frame("main", 1, false, false, b"x");
        let encoded = encode_frame(CONFIG, &f).unwrap();
        assert_eq!(
            decode_frame(CONFIG, &encoded[..10]).err(),
            Some(DecodeError::TooShort)
        );
        let mut bad = encoded.clone();
        bad[0] = 0xFF;
        assert_eq!(
            decode_frame(CONFIG, &bad).err(),
            Some(DecodeError::BadMagic)
        );
    }

    #[test]
    fn subscriber_gated_on_keyframe() {
        let transport = FrameTransport::new(CONFIG);
        // Publish a keyframe BEFORE subscribing so replay seeds the joiner.
        transport.publish(&frame("s", 1, true, false, b"key"));
        let sink = Box::new(TestSink {
            received: StdMutex::new(Vec::new()),
            open: true,
        });
        let received = sink.received.lock().unwrap().len();
        drop(received);
        // We need shared access to inspect: use an Arc-shared sink instead.
        let shared = std::sync::Arc::new(StdMutex::new(Vec::new()));
        struct SharedSink(std::sync::Arc<StdMutex<Vec<Vec<u8>>>>);
        impl FrameSink for SharedSink {
            fn send(&mut self, bytes: &[u8]) {
                self.0.lock().unwrap().push(bytes.to_vec());
            }
            fn is_open(&self) -> bool {
                true
            }
        }
        transport.subscribe("s", Box::new(SharedSink(shared.clone())));
        // The replayed keyframe arrives immediately.
        assert_eq!(shared.lock().unwrap().len(), 1);
        // Delta frames flow now that the subscriber is past the gate.
        transport.publish(&frame("s", 2, false, false, b"delta"));
        assert_eq!(shared.lock().unwrap().len(), 2);
    }

    #[test]
    fn deltas_drop_until_first_keyframe() {
        let transport = FrameTransport::new(CONFIG);
        let shared = std::sync::Arc::new(StdMutex::new(Vec::new()));
        struct SharedSink(std::sync::Arc<StdMutex<Vec<Vec<u8>>>>);
        impl FrameSink for SharedSink {
            fn send(&mut self, bytes: &[u8]) {
                self.0.lock().unwrap().push(bytes.to_vec());
            }
            fn is_open(&self) -> bool {
                true
            }
        }
        transport.subscribe("fresh", Box::new(SharedSink(shared.clone())));
        transport.publish(&frame("fresh", 1, false, false, b"delta"));
        assert_eq!(shared.lock().unwrap().len(), 0);
        transport.publish(&frame("fresh", 2, true, false, b"key"));
        assert_eq!(shared.lock().unwrap().len(), 1);
        let stats = transport.stats_for("fresh");
        assert_eq!(stats[0].dropped, 1);
    }
}
