//! Voice transcription through the ChatGPT session — port of upstream
//! `voiceTranscription.ts` + `chatGptVoiceTranscription.ts`, user-approved
//! ("Codex non banna"). Only the codex provider carries voice upstream; every
//! other provider kind is refused the same way. The bearer token comes from
//! the same disciplined auth machinery as the usage fetcher: rotations are
//! re-read, never burned, and the CLI login always survives.
use crate::provider_usage::err_usage as err;
use crate::todo::WsErr;
use base64::Engine;
use serde_json::{Value, json};

const TRANSCRIPTION_URL: &str = "https://chatgpt.com/backend-api/transcribe";
const MAX_DURATION_MS: i64 = 120_000;
const MAX_AUDIO_BYTES: usize = 10 * 1024 * 1024;
const MAX_MULTIPART_BYTES: usize = MAX_AUDIO_BYTES + 64 * 1024;
const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Safari/605.1.15";

pub fn handles(tag: &str) -> bool {
    matches!(tag, "server.transcribeVoice" | "server.prewarmVoice")
}

fn user_agent() -> String {
    std::env::var("SYNARA_VOICE_UPLOAD_USER_AGENT")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty() && !v.contains(['\r', '\n']))
        .unwrap_or_else(|| DEFAULT_USER_AGENT.to_string())
}

fn normalize_base64(value: &str) -> Option<String> {
    let normalized: String = value.chars().filter(|c| !c.is_whitespace()).collect();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn is_likely_base64(value: &str) -> bool {
    value.len() % 4 == 0
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
        && value.matches('=').count() <= 2
        && value.trim_end_matches('=').chars().all(|c| c != '=')
}

fn expected_decoded_length(value: &str) -> usize {
    let padding = if value.ends_with("==") {
        2
    } else if value.ends_with('=') {
        1
    } else {
        0
    };
    (value.len() / 4) * 3 - padding
}

/// Strict input validation mirroring upstream `decodeVoiceAudio`: the private
/// backend only ever sees normalized 24 kHz mono WAV clips.
fn decode_voice_audio(payload: &Value) -> Result<Vec<u8>, WsErr> {
    if payload.get("mimeType").and_then(Value::as_str) != Some("audio/wav") {
        return Err(err("Only WAV audio is supported for voice transcription."));
    }
    if payload.get("sampleRateHz").and_then(Value::as_i64) != Some(24_000) {
        return Err(err("Voice transcription requires 24 kHz mono WAV audio."));
    }
    let duration = payload
        .get("durationMs")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    if duration <= 0 {
        return Err(err("Voice messages must include a positive duration."));
    }
    if duration > MAX_DURATION_MS {
        return Err(err("Voice messages are limited to 120 seconds."));
    }
    let raw = payload
        .get("audioBase64")
        .and_then(Value::as_str)
        .unwrap_or("");
    let normalized =
        normalize_base64(raw).ok_or_else(|| err("The recorded audio could not be decoded."))?;
    if !is_likely_base64(&normalized) {
        return Err(err("The recorded audio could not be decoded."));
    }
    let audio = base64::engine::general_purpose::STANDARD
        .decode(&normalized)
        .map_err(|_| err("The recorded audio could not be decoded."))?;
    if audio.is_empty() || audio.len() != expected_decoded_length(&normalized) {
        return Err(err("The recorded audio could not be decoded."));
    }
    if audio.len() > MAX_AUDIO_BYTES {
        return Err(err("Voice messages are limited to 10 MB."));
    }
    if audio.len() < 12 || &audio[0..4] != b"RIFF" || &audio[8..12] != b"WAVE" {
        return Err(err("The recorded audio is not a valid WAV file."));
    }
    Ok(audio)
}

fn encode_multipart(audio: &[u8], mime: &str) -> Result<(Vec<u8>, String), WsErr> {
    let boundary = format!("Synara-{}", crate::orchestration_engine::uuid_v4());
    let mut body: Vec<u8> = Vec::new();
    let header = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"voice.wav\"\r\nContent-Type: {mime}\r\n\r\n"
    );
    body.extend_from_slice(header.as_bytes());
    body.extend_from_slice(audio);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    if body.len() > MAX_MULTIPART_BYTES {
        return Err(err("Multipart request exceeded its byte limit."));
    }
    Ok((body, format!("multipart/form-data; boundary={boundary}")))
}

fn post_transcription(audio: &[u8], token: &str) -> Result<(u16, Value), WsErr> {
    let (body, content_type) = encode_multipart(audio, "audio/wav")?;
    let token = token.to_string();
    let ua = user_agent();
    std::thread::scope(|scope| {
        scope
            .spawn(move || -> Result<(u16, Value), String> {
                let client = reqwest::blocking::Client::builder()
                    .timeout(std::time::Duration::from_secs(30))
                    .redirect(reqwest::redirect::Policy::none())
                    .build()
                    .map_err(|e| e.to_string())?;
                let response = client
                    .post(TRANSCRIPTION_URL)
                    .header("Authorization", format!("Bearer {token}"))
                    .header("Content-Type", content_type)
                    .header("User-Agent", ua)
                    .body(body)
                    .send()
                    .map_err(|e| e.to_string())?;
                let status = response.status().as_u16();
                let bytes = response.bytes().map_err(|e| e.to_string())?;
                if bytes.len() > 1024 * 1024 {
                    return Err("transcription response too large".to_string());
                }
                let body: Value = serde_json::from_slice(&bytes).unwrap_or(json!({}));
                Ok((status, body))
            })
            .join()
            .map_err(|_| err("transcription thread panicked"))?
            .map_err(|e| err(&e))
    })
}

fn transcribe(payload: &Value) -> Result<Value, WsErr> {
    let provider = payload
        .get("provider")
        .and_then(Value::as_str)
        .unwrap_or("codex");
    if provider != "codex" {
        return Err(err(&format!(
            "Voice transcription is unavailable for provider '{provider}'."
        )));
    }
    let audio = decode_voice_audio(payload)?;
    let auth = crate::codex_usage::resolve_codex_auth()
        .ok_or_else(|| err("Voice transcription requires a ChatGPT sign-in through Codex."))?;
    let (mut status, mut body) = post_transcription(&audio, &auth.access_token)?;
    if status == 401 || status == 403 {
        // One retry after re-resolving: the file may carry a newer rotation.
        if let Some(fresh) = crate::codex_usage::resolve_codex_auth() {
            if fresh.access_token != auth.access_token {
                let retry = post_transcription(&audio, &fresh.access_token)?;
                status = retry.0;
                body = retry.1;
            }
        }
    }
    if status != 200 {
        return Err(err(&format!("Voice transcription failed ({status}).")));
    }
    let text = body
        .get("text")
        .and_then(Value::as_str)
        .or_else(|| body.get("transcript").and_then(Value::as_str))
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string);
    let Some(text) = text else {
        return Err(err("The transcription response did not include any text."));
    };
    Ok(json!({ "text": text }))
}

fn prewarm() -> Result<Value, WsErr> {
    // Upstream warms the connection with a HEAD to the origin root; auth
    // resolution is the half that matters for first-use latency, and the
    // warm request itself is best-effort either way.
    let _ = crate::codex_usage::resolve_codex_auth();
    let ua = user_agent();
    std::thread::spawn(move || {
        if let Ok(client) = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
        {
            let _ = client
                .head("https://chatgpt.com/")
                .header("User-Agent", ua)
                .send();
        }
    });
    Ok(json!({}))
}

pub fn handle(tag: &str, payload: &Value) -> Result<Value, WsErr> {
    match tag {
        "server.transcribeVoice" => transcribe(payload),
        "server.prewarmVoice" => prewarm(),
        _ => Err(err("unhandled voice tag")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav_bytes() -> Vec<u8> {
        let mut v = b"RIFF".to_vec();
        v.extend_from_slice(&100u32.to_le_bytes());
        v.extend_from_slice(b"WAVE");
        v.extend_from_slice(&[0u8; 100]);
        v
    }

    fn payload_for(audio: &[u8]) -> Value {
        json!({
            "provider": "codex",
            "mimeType": "audio/wav",
            "sampleRateHz": 24000,
            "durationMs": 1500,
            "audioBase64": base64::engine::general_purpose::STANDARD.encode(audio),
        })
    }

    #[test]
    fn valid_wav_passes_validation() {
        let payload = payload_for(&wav_bytes());
        let audio = decode_voice_audio(&payload).ok().unwrap();
        assert_eq!(audio.len(), 112);
    }

    #[test]
    fn non_wav_rejected() {
        let mut payload = payload_for(&wav_bytes());
        payload["mimeType"] = json!("audio/mp3");
        assert!(decode_voice_audio(&payload).is_err());
        let mut wrong_rate = payload_for(&wav_bytes());
        wrong_rate["sampleRateHz"] = json!(16000);
        assert!(decode_voice_audio(&wrong_rate).is_err());
        let mut too_long = payload_for(&wav_bytes());
        too_long["durationMs"] = json!(121_000);
        assert!(decode_voice_audio(&too_long).is_err());
        let bad_riff = payload_for(b"NOTAWAVFILE!");
        assert!(decode_voice_audio(&bad_riff).is_err());
    }

    #[test]
    fn base64_strictness_matches_upstream() {
        assert!(is_likely_base64("AAAA"));
        assert!(is_likely_base64("AAA="));
        assert!(!is_likely_base64("AAA"));
        assert!(!is_likely_base64("A=A="));
        assert!(!is_likely_base64("####"));
        assert_eq!(expected_decoded_length("AAAA"), 3);
        assert_eq!(expected_decoded_length("AAA="), 2);
        assert_eq!(expected_decoded_length("AA=="), 1);
    }

    #[test]
    fn multipart_shape() {
        let (body, content_type) = encode_multipart(b"audio-bytes", "audio/wav").ok().unwrap();
        assert!(content_type.starts_with("multipart/form-data; boundary=Synara-"));
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("name=\"file\"; filename=\"voice.wav\""));
        assert!(text.contains("Content-Type: audio/wav"));
        assert!(text.ends_with("--\r\n"));
    }

    #[test]
    fn non_codex_provider_refused() {
        let payload = json!({"provider": "claude", "mimeType": "audio/wav", "sampleRateHz": 24000, "durationMs": 100, "audioBase64": "AAAA"});
        let result = transcribe(&payload);
        assert!(result.is_err());
        assert!(
            result
                .err()
                .map(|e| e.0.contains("unavailable for provider"))
                .unwrap_or(false)
        );
    }
}
