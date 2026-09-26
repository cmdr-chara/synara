//! Native voice capture and ChatGPT transcription for the main composer.
//!
//! Recordings are kept in memory, capped at 120 seconds, and encoded as mono
//! 24 kHz PCM WAV before upload. Transcription only supplies an unsent draft.
use super::*;
use std::{
    process::Stdio,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader};
use tokio_util::sync::CancellationToken;

const MAX_DURATION: Duration = Duration::from_secs(120);
const MAX_DURATION_MS: u64 = 120_000;
const MAX_AUDIO_BYTES: usize = 10 * 1024 * 1024;
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_MULTIPART_BYTES: usize = MAX_AUDIO_BYTES + 64 * 1024;
const AUTH_TIMEOUT: Duration = Duration::from_secs(10);
const UPLOAD_TIMEOUT: Duration = Duration::from_secs(30);
const CHATGPT_VOICE_URL: &str = "https://chatgpt.com/backend-api/transcribe";
const VOICE_SAMPLE_RATE: u32 = 24_000;
const WAV_HEADER_BYTES: usize = 44;
const MAX_VOICE_TEXT_BYTES: usize = 1024 * 1024;
const MAX_AUTH_LINE_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct DraftStamp {
    task: TaskId,
    project: Option<ProjectId>,
    selection_revision: u64,
    draft_epoch: u64,
    draft_text: String,
}

impl DraftStamp {
    fn current(
        task: TaskId,
        project: Option<ProjectId>,
        selection_revision: u64,
        draft_epoch: u64,
        draft_text: String,
    ) -> Self {
        Self {
            task,
            project,
            selection_revision,
            draft_epoch,
            draft_text,
        }
    }
}

pub(super) struct Reply {
    pub operation: u64,
    pub stamp: DraftStamp,
    pub result: Result<String, String>,
}

#[derive(Default)]
enum Phase {
    #[default]
    Idle,
    Recording {
        operation: u64,
        recorder: Recorder,
    },
    Transcribing {
        operation: u64,
        cancel: CancellationToken,
    },
}

pub(super) struct VoiceState {
    phase: Phase,
    next_operation: u64,
    pub message: Option<String>,
    pub failed: bool,
}

impl VoiceState {
    fn phase(&self) -> &Phase {
        &self.phase
    }

    pub(super) fn active(&self) -> bool {
        !matches!(self.phase(), Phase::Idle)
    }

    pub(super) fn recording(&self) -> bool {
        matches!(self.phase(), Phase::Recording { .. })
    }

    pub(super) fn recording_status(&self) -> Option<(String, u8)> {
        match self.phase() {
            Phase::Recording { recorder, .. } => Some((
                format_recording_duration(recorder.elapsed()),
                recorder.level(),
            )),
            _ => None,
        }
    }

    pub(super) fn transcribing(&self) -> bool {
        matches!(self.phase(), Phase::Transcribing { .. })
    }

    fn operation(&mut self) -> u64 {
        self.next_operation = self.next_operation.wrapping_add(1);
        self.next_operation
    }

    fn is_current(&self, operation: u64) -> bool {
        match self.phase() {
            Phase::Recording {
                operation: active, ..
            }
            | Phase::Transcribing {
                operation: active, ..
            } => *active == operation,
            Phase::Idle => false,
        }
    }

    fn take_phase(&mut self) -> Phase {
        std::mem::replace(&mut self.phase, Phase::Idle)
    }

    fn set_message(&mut self, message: impl Into<String>, failed: bool) {
        self.message = Some(message.into());
        self.failed = failed;
    }

    fn clear_message(&mut self) {
        self.message = None;
        self.failed = false;
    }
}

impl Drop for VoiceState {
    fn drop(&mut self) {
        if let Phase::Transcribing { cancel, .. } = &self.phase {
            cancel.cancel();
        }
    }
}

impl Default for VoiceState {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            next_operation: 0,
            message: None,
            failed: false,
        }
    }
}

#[derive(Clone, Copy)]
struct Clip {
    duration_ms: u64,
}

struct Recorder {
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    stream: Option<cpal::Stream>,
    capture: Arc<Mutex<CaptureBuffer>>,
    started: Instant,
    stamp: DraftStamp,
}

impl Recorder {
    fn start(stamp: DraftStamp) -> Result<Self, String> {
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
        {
            return start_platform_capture(stamp);
        }
        #[allow(unreachable_code)]
        Err("Voice recording is not supported on this platform yet.".into())
    }

    fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    fn level(&self) -> u8 {
        self.capture.lock().map_or(0, |capture| capture.level)
    }

    fn capture_error(&self) -> Option<String> {
        self.capture
            .lock()
            .ok()
            .and_then(|capture| capture.error.clone())
    }

    fn reached_limit(&self) -> bool {
        self.capture
            .lock()
            .is_ok_and(|capture| capture.limit_reached)
    }

    fn finish(mut self) -> Result<(Vec<u8>, Clip, DraftStamp), String> {
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
        self.stream.take();
        let capture = self
            .capture
            .lock()
            .map_err(|_| "The microphone recorder stopped unexpectedly.".to_owned())?;
        if let Some(error) = &capture.error {
            return Err(error.clone());
        }
        let duration_ms = sample_count_to_duration_ms(capture.samples);
        if duration_ms == 0 {
            return Err(
                "No microphone audio was captured. Check microphone permission and try again."
                    .into(),
            );
        }
        let mut wav = capture.wav.clone();
        drop(capture);
        finalize_wav_header(&mut wav)?;
        validate_wav(&wav, duration_ms)?;
        Ok((wav, Clip { duration_ms }, self.stamp))
    }
}

#[derive(Default)]
struct CaptureBuffer {
    wav: Vec<u8>,
    samples: usize,
    resample_phase: u64,
    limit_reached: bool,
    level: u8,
    error: Option<String>,
}

impl CaptureBuffer {
    fn new() -> Self {
        let mut capture = Self::default();
        capture.wav.resize(WAV_HEADER_BYTES, 0);
        capture
    }

    fn append_frame(&mut self, mono: i16, source_rate: u32) {
        if source_rate == 0 {
            self.error = Some("The microphone reported an invalid sample rate.".into());
            return;
        }
        self.resample_phase = self
            .resample_phase
            .saturating_add(u64::from(VOICE_SAMPLE_RATE));
        while self.resample_phase >= u64::from(source_rate) {
            if self.samples >= max_samples() || self.wav.len().saturating_add(2) > MAX_AUDIO_BYTES {
                self.limit_reached = true;
                return;
            }
            self.wav.extend_from_slice(&mono.to_le_bytes());
            self.samples += 1;
            self.resample_phase -= u64::from(source_rate);
        }
        if self.samples >= max_samples() {
            self.limit_reached = true;
        }
    }
}

fn format_recording_duration(elapsed: Duration) -> String {
    let seconds = elapsed.as_secs().min(MAX_DURATION.as_secs());
    format!("{:02}:{:02} / 02:00", seconds / 60, seconds % 60)
}

fn voice_level_from_peak(peak: u16) -> u8 {
    if peak < 256 {
        return 0;
    }
    let max = u32::from(i16::MAX as u16);
    u8::try_from((u32::from(peak).saturating_mul(5).div_ceil(max)).clamp(1, 5)).unwrap_or(5)
}

fn max_samples() -> usize {
    (MAX_DURATION.as_secs() as usize) * VOICE_SAMPLE_RATE as usize
}

fn sample_count_to_duration_ms(samples: usize) -> u64 {
    ((samples as u64) * 1000).div_ceil(u64::from(VOICE_SAMPLE_RATE))
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn start_platform_capture(stamp: DraftStamp) -> Result<Recorder, String> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let host = cpal::default_host();
    let device = host.default_input_device().ok_or_else(|| {
        "No microphone is available. Connect a microphone and try again.".to_owned()
    })?;
    let supported = device.default_input_config().map_err(|_| {
        "Could not open a microphone. Check microphone permission and device availability."
            .to_owned()
    })?;
    let config: cpal::StreamConfig = supported.clone().into();
    let channels = usize::from(config.channels);
    if channels == 0 {
        return Err("The microphone reported an invalid channel count.".into());
    }
    let sample_rate = config.sample_rate;
    let capture = Arc::new(Mutex::new(CaptureBuffer::new()));
    let data_capture = capture.clone();
    let error_capture = capture.clone();
    let on_error = move |_error: cpal::StreamError| {
        if let Ok(mut capture) = error_capture.lock() {
            capture.error =
                Some("The microphone stopped unexpectedly. Check the device and try again.".into());
        }
    };
    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config,
            move |data: &[f32], _| append_audio_samples(&data_capture, data, channels, sample_rate),
            on_error,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config,
            move |data: &[i16], _| append_audio_samples(&data_capture, data, channels, sample_rate),
            on_error,
            None,
        ),
        cpal::SampleFormat::U16 => device.build_input_stream(
            &config,
            move |data: &[u16], _| append_audio_samples(&data_capture, data, channels, sample_rate),
            on_error,
            None,
        ),
        _ => {
            return Err("The microphone uses an audio format this build cannot record yet.".into());
        }
    }
    .map_err(|_| {
        "Could not start microphone recording. Check microphone permission and device availability."
            .to_owned()
    })?;
    stream.play().map_err(|_| {
        "Could not start microphone recording. Check microphone permission and device availability.".to_owned()
    })?;
    Ok(Recorder {
        stream: Some(stream),
        capture,
        started: Instant::now(),
        stamp,
    })
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
trait ToVoiceSample {
    fn to_voice_sample(self) -> i16;
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
impl ToVoiceSample for f32 {
    fn to_voice_sample(self) -> i16 {
        (self.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16
    }
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
impl ToVoiceSample for i16 {
    fn to_voice_sample(self) -> i16 {
        self
    }
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
impl ToVoiceSample for u16 {
    fn to_voice_sample(self) -> i16 {
        (i32::from(self) - i32::from(u16::MAX) / 2) as i16
    }
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn append_audio_samples<T: ToVoiceSample + Copy>(
    capture: &Arc<Mutex<CaptureBuffer>>,
    data: &[T],
    channels: usize,
    sample_rate: u32,
) {
    if channels == 0 {
        return;
    }
    let Ok(mut capture) = capture.lock() else {
        return;
    };
    let mut peak = 0_u16;
    for frame in data.chunks_exact(channels) {
        if capture.limit_reached || capture.error.is_some() {
            return;
        }
        let total = frame
            .iter()
            .map(|sample| i64::from((*sample).to_voice_sample()))
            .sum::<i64>();
        let mono = (total / channels as i64) as i16;
        peak = peak.max(mono.unsigned_abs());
        capture.append_frame(mono, sample_rate);
    }
    capture.level = voice_level_from_peak(peak);
}

fn finalize_wav_header(wav: &mut [u8]) -> Result<(), String> {
    if wav.len() < WAV_HEADER_BYTES || wav.len() > MAX_AUDIO_BYTES {
        return Err("Voice recordings are limited to 10 MiB.".into());
    }
    let data_len = u32::try_from(wav.len() - WAV_HEADER_BYTES)
        .map_err(|_| "Voice recordings are limited to 10 MiB.".to_owned())?;
    let riff_len = data_len
        .checked_add(36)
        .ok_or_else(|| "The microphone recording exceeded its size limit.".to_owned())?;
    wav[..4].copy_from_slice(b"RIFF");
    wav[4..8].copy_from_slice(&riff_len.to_le_bytes());
    wav[8..12].copy_from_slice(b"WAVE");
    wav[12..16].copy_from_slice(b"fmt ");
    wav[16..20].copy_from_slice(&16_u32.to_le_bytes());
    wav[20..22].copy_from_slice(&1_u16.to_le_bytes());
    wav[22..24].copy_from_slice(&1_u16.to_le_bytes());
    wav[24..28].copy_from_slice(&VOICE_SAMPLE_RATE.to_le_bytes());
    wav[28..32].copy_from_slice(&(VOICE_SAMPLE_RATE * 2).to_le_bytes());
    wav[32..34].copy_from_slice(&2_u16.to_le_bytes());
    wav[34..36].copy_from_slice(&16_u16.to_le_bytes());
    wav[36..40].copy_from_slice(b"data");
    wav[40..44].copy_from_slice(&data_len.to_le_bytes());
    Ok(())
}

fn validate_wav(wav: &[u8], duration_ms: u64) -> Result<(), String> {
    if duration_ms == 0 || duration_ms > MAX_DURATION_MS {
        return Err("Voice messages are limited to 120 seconds.".into());
    }
    if wav.len() < WAV_HEADER_BYTES || wav.len() > MAX_AUDIO_BYTES {
        return Err("Voice messages are limited to 10 MiB.".into());
    }
    let pcm_bytes = wav.len() - WAV_HEADER_BYTES;
    let expected_duration = sample_count_to_duration_ms(pcm_bytes / 2);
    if &wav[0..4] != b"RIFF"
        || &wav[8..12] != b"WAVE"
        || &wav[12..16] != b"fmt "
        || u32::from_le_bytes([wav[4], wav[5], wav[6], wav[7]]) as usize != wav.len() - 8
        || u32::from_le_bytes([wav[16], wav[17], wav[18], wav[19]]) != 16
        || &wav[36..40] != b"data"
        || u16::from_le_bytes([wav[20], wav[21]]) != 1
        || u16::from_le_bytes([wav[22], wav[23]]) != 1
        || u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]) != VOICE_SAMPLE_RATE
        || u32::from_le_bytes([wav[28], wav[29], wav[30], wav[31]]) != VOICE_SAMPLE_RATE * 2
        || u16::from_le_bytes([wav[32], wav[33]]) != 2
        || u16::from_le_bytes([wav[34], wav[35]]) != 16
        || u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]) as usize != pcm_bytes
        || !pcm_bytes.is_multiple_of(2)
        || expected_duration != duration_ms
    {
        return Err("The microphone recording is not a supported 24 kHz mono WAV file.".into());
    }
    Ok(())
}

fn transcription_url(value: Option<&str>) -> Result<reqwest::Url, String> {
    let value = value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(CHATGPT_VOICE_URL);
    let url = reqwest::Url::parse(value)
        .map_err(|_| "The ChatGPT transcription URL was invalid.".to_owned())?;
    if url.scheme() != "https"
        || url.host_str() != Some("chatgpt.com")
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.origin().ascii_serialization() != "https://chatgpt.com"
    {
        return Err("Voice transcription only allows the official ChatGPT upload origin.".into());
    }
    Ok(url)
}

fn multipart_body(wav: &[u8]) -> Result<(String, Vec<u8>), String> {
    if wav.len() > MAX_AUDIO_BYTES {
        return Err("Voice messages are limited to 10 MiB.".into());
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let boundary = format!("synara-voice-{}-{nonce}", std::process::id());
    let prefix = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"voice.wav\"\r\nContent-Type: audio/wav\r\n\r\n"
    );
    let suffix = format!("\r\n--{boundary}--\r\n");
    let total_len = prefix
        .len()
        .checked_add(wav.len())
        .and_then(|len| len.checked_add(suffix.len()))
        .filter(|len| *len <= MAX_MULTIPART_BYTES)
        .ok_or_else(|| "Voice messages are limited to 10 MiB.".to_owned())?;
    let mut body = Vec::with_capacity(total_len);
    body.extend_from_slice(prefix.as_bytes());
    body.extend_from_slice(wav);
    body.extend_from_slice(suffix.as_bytes());
    Ok((boundary, body))
}

struct Auth {
    token: String,
    url: reqwest::Url,
}

async fn resolve_codex_auth() -> Result<Auth, String> {
    let mut child = tokio::process::Command::new("codex")
        .arg("app-server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| {
            "Could not start Codex authentication. Check that Codex is installed.".to_owned()
        })?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Could not read Codex authentication.".to_owned())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Could not read Codex authentication.".to_owned())?;
    let mut reader = BufReader::new(stdout);
    let initialize = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "clientInfo": {
                "name": "synara-desktop",
                "title": "Synara Desktop",
                "version": env!("CARGO_PKG_VERSION"),
            },
            "capabilities": { "experimentalApi": true },
        },
    });
    write_rpc_line(&mut stdin, &initialize).await?;
    let _initialize = read_rpc_response(&mut reader, 1).await?;
    write_rpc_line(
        &mut stdin,
        &serde_json::json!({"jsonrpc":"2.0", "method":"initialized", "params":{}}),
    )
    .await?;
    write_rpc_line(
        &mut stdin,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "getAuthStatus",
            "params": {"includeToken": true, "refreshToken": true},
        }),
    )
    .await?;
    let response = read_rpc_response(&mut reader, 2).await?;
    let result = response
        .get("result")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| "Could not read ChatGPT authentication from Codex.".to_owned())?;
    let method = result
        .get("authMethod")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if method != "chatgpt" && method != "chatgptAuthTokens" {
        return Err("Voice transcription requires a ChatGPT-authenticated Codex session.".into());
    }
    let token = result
        .get("authToken")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .ok_or_else(|| {
            "No ChatGPT session token is available. Sign in to ChatGPT in Codex.".to_owned()
        })?
        .to_owned();
    let url = transcription_url(
        result
            .get("transcriptionUrl")
            .and_then(serde_json::Value::as_str),
    )?;
    let _ = child.start_kill();
    Ok(Auth { token, url })
}

async fn write_rpc_line<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    value: &serde_json::Value,
) -> Result<(), String> {
    let mut bytes = serde_json::to_vec(value)
        .map_err(|_| "Could not initialize Codex authentication.".to_owned())?;
    bytes.push(b'\n');
    writer
        .write_all(&bytes)
        .await
        .map_err(|_| "Could not initialize Codex authentication.".to_owned())
}

async fn read_rpc_response<R: AsyncRead + Unpin>(
    reader: &mut BufReader<R>,
    wanted: u64,
) -> Result<serde_json::Value, String> {
    loop {
        let Some(line) = read_limited_line(reader, MAX_AUTH_LINE_BYTES).await? else {
            return Err("Codex authentication ended before it completed.".into());
        };
        let Ok(message) = serde_json::from_slice::<serde_json::Value>(&line) else {
            continue;
        };
        if message.get("id").and_then(serde_json::Value::as_u64) == Some(wanted) {
            if message.get("error").is_some() {
                return Err("Codex could not provide ChatGPT authentication.".into());
            }
            return Ok(message);
        }
    }
}

async fn read_limited_line<R: AsyncRead + Unpin>(
    reader: &mut BufReader<R>,
    max_bytes: usize,
) -> Result<Option<Vec<u8>>, String> {
    let mut line = Vec::new();
    loop {
        let available = reader
            .fill_buf()
            .await
            .map_err(|_| "Could not read Codex authentication.".to_owned())?;
        if available.is_empty() {
            return if line.is_empty() {
                Ok(None)
            } else {
                Err("Codex authentication returned an incomplete response.".into())
            };
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let count = newline.unwrap_or(available.len());
        if line.len().saturating_add(count) > max_bytes {
            return Err("Codex authentication returned an oversized response.".into());
        }
        line.extend_from_slice(&available[..count]);
        reader.consume(count + usize::from(newline.is_some()));
        if newline.is_some() {
            return Ok(Some(line));
        }
    }
}

async fn upload_voice(wav: &[u8], auth: Auth) -> Result<String, String> {
    let (boundary, body) = multipart_body(wav)?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(UPLOAD_TIMEOUT)
        .build()
        .map_err(|_| "Could not prepare the ChatGPT transcription request.".to_owned())?;
    let mut request = client
        .post(auth.url)
        .header(
            reqwest::header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Safari/605.1.15",
        )
        .body(body);
    request = request.bearer_auth(auth.token);
    let response = request
        .send()
        .await
        .map_err(|_| "Could not connect to ChatGPT for transcription.".to_owned())?;
    let status = response.status();
    if !status.is_success() {
        return Err(match status.as_u16() {
            401 => "Your ChatGPT login has expired. Sign in again.".into(),
            403 => "ChatGPT rejected the transcription request.".into(),
            code => format!("ChatGPT transcription failed with status {code}."),
        });
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err("ChatGPT returned an oversized transcription response.".into());
    }
    let mut response = response;
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Could not read the ChatGPT transcription response.".to_owned())?
    {
        if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err("ChatGPT returned an oversized transcription response.".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    #[derive(serde::Deserialize)]
    struct TranscriptionResponse {
        text: Option<String>,
        transcript: Option<String>,
    }
    let response: TranscriptionResponse = serde_json::from_slice(&bytes)
        .map_err(|_| "ChatGPT returned an invalid transcription response.".to_owned())?;
    let transcript = response
        .text
        .or(response.transcript)
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
        .ok_or_else(|| "ChatGPT did not return any transcription text.".to_owned())?;
    if transcript.len() > MAX_VOICE_TEXT_BYTES {
        return Err("The transcription is too long to add to the composer.".into());
    }
    Ok(transcript)
}

async fn transcribe(
    wav: Vec<u8>,
    duration_ms: u64,
    cancel: CancellationToken,
) -> Result<String, String> {
    validate_wav(&wav, duration_ms)?;
    let auth = tokio::select! {
        _ = cancel.cancelled() => return Err("Voice transcription was cancelled.".into()),
        result = tokio::time::timeout(AUTH_TIMEOUT, resolve_codex_auth()) => match result {
            Ok(result) => result?,
            Err(_) => return Err("Timed out while reading ChatGPT authentication from Codex.".into()),
        },
    };
    tokio::select! {
        _ = cancel.cancelled() => Err("Voice transcription was cancelled.".into()),
        result = upload_voice(&wav, auth) => result,
    }
}

fn draft_matches(
    stamp: &DraftStamp,
    selected: Option<TaskId>,
    project: Option<ProjectId>,
    revision: u64,
    epoch: u64,
    text: &str,
) -> bool {
    selected == Some(stamp.task)
        && project == stamp.project
        && revision == stamp.selection_revision
        && epoch == stamp.draft_epoch
        && text == stamp.draft_text
}

impl Shell {
    pub(super) fn voice_primary(&mut self, cx: &mut Context<Self>) {
        if self.voice.recording() {
            self.stop_voice_recording(cx);
        } else if !self.voice.active() {
            self.start_voice_recording(cx);
        }
    }

    pub(super) fn voice_cancel(&mut self, cx: &mut Context<Self>) {
        self.cancel_voice_operation(true);
        cx.notify();
    }

    pub(super) fn cancel_voice_operation(&mut self, show_message: bool) {
        let phase = self.voice.take_phase();
        match phase {
            Phase::Recording { .. } => {
                if show_message {
                    self.voice.set_message(
                        "Voice recording cancelled. Your draft was left unchanged.",
                        false,
                    );
                } else {
                    self.voice.clear_message();
                }
            }
            Phase::Transcribing { cancel, .. } => {
                cancel.cancel();
                if show_message {
                    self.voice.set_message(
                        "Voice transcription cancelled. Your draft was left unchanged.",
                        false,
                    );
                } else {
                    self.voice.clear_message();
                }
            }
            Phase::Idle => {}
        }
        self.voice.operation();
    }

    pub(super) fn tick_voice(&mut self, cx: &mut Context<Self>) {
        let navigated = matches!(self.voice.phase(), Phase::Recording { recorder, .. }
            if self.selected != Some(recorder.stamp.task)
                || self.project != recorder.stamp.project
                || self.selection_revision != recorder.stamp.selection_revision);
        if navigated {
            self.cancel_voice_operation(false);
            return;
        }
        let issue = match self.voice.phase() {
            Phase::Recording { recorder, .. } => recorder
                .capture_error()
                .map(Err)
                .or_else(|| recorder.elapsed().ge(&MAX_DURATION).then_some(Ok(())))
                .or_else(|| recorder.reached_limit().then_some(Ok(()))),
            _ => None,
        };
        match issue {
            Some(Err(error)) => {
                self.cancel_voice_operation(false);
                self.voice.set_message(error, true);
                cx.notify();
            }
            Some(Ok(())) => self.stop_voice_recording(cx),
            None => {
                if self.voice.recording() {
                    cx.notify();
                }
            }
        }
    }

    fn start_voice_recording(&mut self, cx: &mut Context<Self>) {
        if self.close != CloseState::Open || self.panel != Panel::Conversation {
            return;
        }
        let Some(task) = self.selected else {
            self.voice
                .set_message("Select a task before recording a voice draft.", true);
            cx.notify();
            return;
        };
        if self.loading_task.is_some() || self.draft_state.loading.contains(&task) {
            self.voice.set_message(
                "Wait for this task and its draft to finish loading before recording.",
                true,
            );
            cx.notify();
            return;
        }
        if self.composer.read(cx).is_composing() {
            self.voice.set_message(
                "Finish text composition before recording a voice draft.",
                true,
            );
            cx.notify();
            return;
        }
        let stamp = DraftStamp::current(
            task,
            self.project,
            self.selection_revision,
            self.draft_state.version(task),
            self.composer.read(cx).text().to_owned(),
        );
        match Recorder::start(stamp) {
            Ok(recorder) => {
                let operation = self.voice.operation();
                self.voice.phase = Phase::Recording {
                    operation,
                    recorder,
                };
                self.voice.set_message("Recording. Stop to transcribe; the result will be added only if this draft stays unchanged.", false);
            }
            Err(error) => self.voice.set_message(error, true),
        }
        cx.notify();
    }

    fn stop_voice_recording(&mut self, cx: &mut Context<Self>) {
        let Phase::Recording {
            operation,
            recorder,
        } = self.voice.take_phase()
        else {
            return;
        };
        let current_text = self.composer.read(cx).text().to_owned();
        if !draft_matches(
            &recorder.stamp,
            self.selected,
            self.project,
            self.selection_revision,
            self.draft_state.version(recorder.stamp.task),
            &current_text,
        ) {
            self.voice.set_message(
                "The selected task or draft changed while recording, so the clip was discarded without upload.",
                true,
            );
            cx.notify();
            return;
        }
        match recorder.finish() {
            Ok((wav, clip, stamp)) => {
                let cancel = CancellationToken::new();
                self.voice.phase = Phase::Transcribing {
                    operation,
                    cancel: cancel.clone(),
                };
                self.voice.set_message("Transcribing with ChatGPT. The recording is uploaded for transcription only; Synara will not send it as a message.", false);
                let sender = self.sender.clone();
                self.runtime.spawn(async move {
                    let result = transcribe(wav, clip.duration_ms, cancel).await;
                    let _ = sender
                        .send(Update::Voice(Box::new(Reply {
                            operation,
                            stamp,
                            result,
                        })))
                        .await;
                });
            }
            Err(error) => self.voice.set_message(error, true),
        }
        cx.notify();
    }

    pub(super) fn apply_voice_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        if !self.voice.is_current(reply.operation) {
            return;
        }
        self.voice.phase = Phase::Idle;
        match reply.result {
            Err(error) => self.voice.set_message(error, true),
            Ok(transcript) => {
                let task = self.selected;
                let project = self.project;
                let revision = self.selection_revision;
                let epoch = self.draft_state.version(reply.stamp.task);
                let current_text = self.composer.read(cx).text().to_owned();
                if !draft_matches(&reply.stamp, task, project, revision, epoch, &current_text) {
                    self.voice.set_message("The draft changed while transcription was running, so the transcript was not inserted. Your current draft is untouched.", true);
                    return;
                }
                let mut next = reply.stamp.draft_text;
                if !next.is_empty() && !next.ends_with(char::is_whitespace) {
                    next.push('\n');
                }
                next.push_str(&transcript);
                if next.len() > MAX_VOICE_TEXT_BYTES {
                    self.voice.set_message("The transcript would exceed the composer limit, so your draft was left unchanged.", true);
                    return;
                }
                self.composer
                    .update(cx, |entry, cx| entry.set_text(next, cx));
                self.remember_draft(cx);
                // set_text intentionally emits no Changed event. Persist this transcript
                // through the same task draft store before returning control to the user.
                self.flush_drafts(true);
                self.focus_composer = true;
                self.voice.set_message(
                    "Transcript added to the unsent draft. Review it, then choose Send when ready.",
                    false,
                );
            }
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav_with_audio_bytes(data: &[u8]) -> Vec<u8> {
        let mut wav = vec![0; WAV_HEADER_BYTES];
        wav.extend_from_slice(data);
        finalize_wav_header(&mut wav).unwrap();
        wav
    }

    #[test]
    fn recording_presentation_is_bounded_and_levels_are_scaled() {
        assert_eq!(format_recording_duration(Duration::ZERO), "00:00 / 02:00");
        assert_eq!(
            format_recording_duration(Duration::from_secs(61)),
            "01:01 / 02:00"
        );
        assert_eq!(
            format_recording_duration(Duration::from_secs(9_999)),
            "02:00 / 02:00"
        );
        assert_eq!(voice_level_from_peak(0), 0);
        assert_eq!(voice_level_from_peak(255), 0);
        assert_eq!(voice_level_from_peak(i16::MAX as u16), 5);
        assert!((1..=5).contains(&voice_level_from_peak(8_000)));
    }

    #[test]
    fn wav_and_duration_limits_are_enforced() {
        let wav = wav_with_audio_bytes(&vec![0; max_samples() * 2]);
        assert_eq!(wav.len(), max_samples() * 2 + WAV_HEADER_BYTES);
        assert!(wav.len() <= MAX_AUDIO_BYTES);
        assert!(validate_wav(&wav, MAX_DURATION_MS).is_ok());
        assert!(validate_wav(&wav, MAX_DURATION_MS + 1).is_err());
        assert!(validate_wav(&wav, 0).is_err());
        assert!(finalize_wav_header(&mut vec![0; MAX_AUDIO_BYTES + 1]).is_err());
    }

    #[test]
    fn transcription_auth_url_requires_exact_chatgpt_https_origin() {
        assert_eq!(transcription_url(None).unwrap().as_str(), CHATGPT_VOICE_URL);
        assert!(transcription_url(Some("https://chatgpt.com/backend-api/transcribe")).is_ok());
        for hostile in [
            "http://chatgpt.com/backend-api/transcribe",
            "https://chatgpt.com.evil.example/upload",
            "https://evil.example/chatgpt.com",
            "https://chatgpt.com:444/backend-api/transcribe",
            "https://user@chatgpt.com/backend-api/transcribe",
        ] {
            assert!(
                transcription_url(Some(hostile)).is_err(),
                "accepted {hostile}"
            );
        }
    }

    #[test]
    fn late_or_cancelled_results_cannot_replace_a_changed_draft() {
        let stamp = DraftStamp {
            task: TaskId::new(),
            project: Some(ProjectId::new()),
            selection_revision: 4,
            draft_epoch: 12,
            draft_text: "original draft".into(),
        };
        assert!(draft_matches(
            &stamp,
            Some(stamp.task),
            stamp.project,
            4,
            12,
            "original draft"
        ));
        assert!(!draft_matches(
            &stamp,
            Some(stamp.task),
            stamp.project,
            5,
            12,
            "original draft"
        ));
        assert!(!draft_matches(
            &stamp,
            Some(stamp.task),
            stamp.project,
            4,
            13,
            "original draft"
        ));
        assert!(!draft_matches(
            &stamp,
            Some(TaskId::new()),
            stamp.project,
            4,
            12,
            "original draft"
        ));
        assert!(!draft_matches(
            &stamp,
            Some(stamp.task),
            stamp.project,
            4,
            12,
            "newer draft"
        ));
    }

    #[test]
    fn multipart_payload_stays_bounded_and_contains_wav_file() {
        let wav = wav_with_audio_bytes(&[0, 0]);
        let (boundary, body) = multipart_body(&wav).unwrap();
        assert!(body.len() <= MAX_MULTIPART_BYTES);
        assert!(body.windows(8).any(|window| window == b"RIFF\x26\0\0\0"));
        assert!(body.starts_with(format!("--{boundary}\r\n").as_bytes()));
        assert!(multipart_body(&vec![0; MAX_AUDIO_BYTES + 1]).is_err());
    }

    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    #[tokio::test(flavor = "current_thread")]
    #[ignore = "requires a real microphone and a ChatGPT-authenticated Codex session"]
    async fn live_microphone_chatgpt_transcription_end_to_end() {
        assert_eq!(
            std::env::var("SYNARA_VOICE_ACCEPTANCE").as_deref(),
            Ok("live-microphone-chatgpt"),
            "set SYNARA_VOICE_ACCEPTANCE only on the dedicated live acceptance runner"
        );
        let stamp = DraftStamp {
            task: TaskId::new(),
            project: None,
            selection_revision: 0,
            draft_epoch: 0,
            draft_text: String::new(),
        };
        let recorder = Recorder::start(stamp)
            .expect("open the real default microphone; grant OS microphone permission first");
        tokio::time::sleep(Duration::from_secs(2)).await;
        assert!(
            recorder.level() > 0,
            "the live microphone opened but no audible input was observed; speak during the acceptance capture"
        );
        tokio::time::sleep(Duration::from_secs(2)).await;
        let (wav, clip, _) = recorder
            .finish()
            .expect("capture real microphone audio for the acceptance phrase");
        assert!(
            clip.duration_ms >= 2_000,
            "live microphone capture was unexpectedly short"
        );
        let transcript = transcribe(wav, clip.duration_ms, CancellationToken::new())
            .await
            .expect("real Codex ChatGPT authentication and transcription upload");
        assert!(
            !transcript.trim().is_empty(),
            "ChatGPT returned an empty live transcription"
        );
        println!(
            "VOICE_LIVE_ACCEPTANCE: microphone capture + Codex ChatGPT auth + official transcription upload; transcript_bytes={}",
            transcript.len()
        );
    }

    #[test]
    fn cancelling_a_transcription_invalidates_its_operation() {
        let mut state = VoiceState::default();
        let cancellation = CancellationToken::new();
        let operation = state.operation();
        state.phase = Phase::Transcribing {
            operation,
            cancel: cancellation.clone(),
        };
        assert!(state.is_current(operation));
        let phase = state.take_phase();
        if let Phase::Transcribing { cancel, .. } = phase {
            cancel.cancel();
        }
        state.operation();
        assert!(cancellation.is_cancelled());
        assert!(!state.is_current(operation));
    }
}
