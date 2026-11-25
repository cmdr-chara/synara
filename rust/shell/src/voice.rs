//! Voice note capture for the composer mic: cpal input stream recording to
//! 24 kHz mono WAV, matching the backend `server.transcribeVoice` contract
//! (strict WAV validation upstream: PCM16, mono, 24 kHz, max 120 s).

use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

const TARGET_RATE: u32 = 24_000;
const MAX_DURATION_MS: u64 = 120_000;

pub struct VoiceCapture {
    stream: cpal::Stream,
    buf: Arc<Mutex<Vec<f32>>>,
    source_rate: u32,
    channels: u16,
    started: std::time::Instant,
}

pub enum StartError {
    NoDevice,
    Stream(String),
}

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StartError::NoDevice => write!(f, "no audio input device"),
            StartError::Stream(e) => write!(f, "audio capture failed: {e}"),
        }
    }
}

impl VoiceCapture {
    /// Start capturing from the default input device. Samples are accumulated
    /// as f32 in the device-native rate and channel layout; conversion happens
    /// once in `finish`, so recording never blocks the audio callback.
    pub fn start() -> Result<Self, StartError> {
        let host = cpal::default_host();
        let device = host.default_input_device().ok_or(StartError::NoDevice)?;
        let supported = device
            .default_input_config()
            .map_err(|e| StartError::Stream(e.to_string()))?;
        let source_rate = supported.sample_rate().0;
        let channels = supported.channels();
        let buf: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let writer = buf.clone();
        let stream = device
            .build_input_stream(
                &supported.config(),
                move |data: &[f32], _| {
                    if let Ok(mut guard) = writer.lock() {
                        guard.extend_from_slice(data);
                    }
                },
                |err| eprintln!("voice capture error: {err}"),
                None,
            )
            .map_err(|e| StartError::Stream(e.to_string()))?;
        stream.play().map_err(|e| StartError::Stream(e.to_string()))?;
        Ok(Self {
            stream,
            buf,
            source_rate,
            channels,
            started: std::time::Instant::now(),
        })
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }

    /// Stop capturing and produce the WAV bytes: downmix to mono, linear
    /// resample to 24 kHz, PCM16 encode. Empty when nothing was captured.
    pub fn finish(self) -> Vec<u8> {
        drop(self.stream);
        let samples = match self.buf.lock() {
            Ok(guard) => guard.clone(),
            Err(_) => return Vec::new(),
        };
        if samples.is_empty() {
            return Vec::new();
        }
        let mono = downmix_mono(&samples, self.channels);
        let resampled = resample_linear(&mono, self.source_rate, TARGET_RATE);
        let pcm: Vec<i16> = resampled
            .iter()
            .map(|s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
            .collect();
        encode_wav(&pcm, TARGET_RATE)
    }
}

fn downmix_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    let n = channels.max(1) as usize;
    if n == 1 {
        return samples.to_vec();
    }
    samples
        .chunks(n)
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect()
}

/// Linear-interpolation resampler. Voice notes only ever downsample (device
/// rates are 44.1/48 kHz against the 24 kHz target), where the artifacts of
/// linear interpolation are inaudible for speech.
fn resample_linear(samples: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || samples.is_empty() {
        return samples.to_vec();
    }
    let ratio = from as f64 / to as f64;
    let out_len = (samples.len() as f64 / ratio).floor() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let pos = i as f64 * ratio;
        let idx = pos.floor() as usize;
        let frac = (pos - idx as f64) as f32;
        let a = samples[idx];
        let b = samples.get(idx + 1).copied().unwrap_or(a);
        out.push(a + (b - a) * frac);
    }
    out
}

fn encode_wav(pcm: &[i16], rate: u32) -> Vec<u8> {
    let data_len = (pcm.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes()); // byte rate
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in pcm {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

pub fn max_duration_ms() -> u64 {
    MAX_DURATION_MS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_matches_backend_contract() {
        let wav = encode_wav(&[0i16; 2400], 24_000);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(u16::from_le_bytes([wav[22], wav[23]]), 1); // mono
        assert_eq!(u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]), 24_000);
        assert_eq!(u16::from_le_bytes([wav[34], wav[35]]), 16); // pcm16
    }

    #[test]
    fn downmix_averages_channels() {
        let stereo = [1.0f32, -1.0, 0.5, 0.5];
        assert_eq!(downmix_mono(&stereo, 2), vec![0.0, 0.5]);
    }

    #[test]
    fn resample_halves_length_for_48k_to_24k() {
        let input: Vec<f32> = (0..480).map(|i| (i as f32).sin()).collect();
        let out = resample_linear(&input, 48_000, 24_000);
        assert!((out.len() as i64 - 240).abs() <= 1);
    }
}
