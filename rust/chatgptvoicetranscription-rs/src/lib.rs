//! JS bindings for `packages/shared/src/chatGptVoiceTranscription.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use napi_derive::napi;

pub mod js;
pub mod port;

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".into())
}

#[allow(dead_code)]
fn run<T>(f: impl FnOnce() -> Result<T, String>) -> napi::Result<T> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(result) => result.map_err(napi::Error::from_reason),
        Err(payload) => Err(napi::Error::from_reason(format!(
            "Rust panic: {}",
            panic_message(&*payload)
        ))),
    }
}

#[allow(dead_code)]
async fn run_async<T>(f: impl std::future::Future<Output = Result<T, String>>) -> napi::Result<T> {
    use futures::FutureExt as _;
    match std::panic::AssertUnwindSafe(f).catch_unwind().await {
        Ok(result) => result.map_err(napi::Error::from_reason),
        Err(payload) => Err(napi::Error::from_reason(format!(
            "Rust panic: {}",
            panic_message(&*payload)
        ))),
    }
}

/// `prewarmChatGptVoiceTranscriptionConnection`.
#[napi(js_name = "prewarmChatGptVoiceTranscriptionConnection")]
pub async fn prewarm_chat_gpt_voice_transcription_connection() -> napi::Result<()> {
    run_async(async move {
        port::prewarm_chat_gpt_voice_transcription_connection().await?;
        Ok(())
    })
    .await
}

/// `requestChatGptVoiceTranscription`.
#[napi(js_name = "requestChatGptVoiceTranscription")]
pub async fn request_chat_gpt_voice_transcription(input: String) -> napi::Result<String> {
    run_async(async move {
        let input = crate::js::from_json(&input)?;
        let out = port::request_chat_gpt_voice_transcription(input).await?;
        crate::js::to_json(&out)
    })
    .await
}
