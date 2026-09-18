//! Bounded, cancellation-aware ownership of protocol output.
use super::{MAX_FRAME, State};
use serde_json::Value;
use std::{fmt, io, sync::Arc, time::Duration};
use synara_agent::{AgentError, AgentResult};
use synara_runtime::ProcessWriter;
use tokio::{
    io::AsyncWriteExt,
    sync::{OwnedSemaphorePermit, Semaphore, mpsc},
};
use tokio_util::sync::CancellationToken;

// Includes serialization reservations, queued frames and the frame being written.
// This is a byte budget, not a claim about total process memory or Value storage.
const MAX_OUTBOUND_BYTES: usize = 2 * (MAX_FRAME + 1);

pub(super) struct Frame {
    bytes: Vec<u8>,
    lifetime: Option<CancellationToken>,
    _budget: OwnedSemaphorePermit,
}

impl fmt::Debug for Frame {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Frame")
            .field("byte_count", &self.bytes.len())
            .field("request_scoped", &self.lifetime.is_some())
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub(super) struct OutboundQueue {
    sender: mpsc::Sender<Frame>,
    budget: Arc<Semaphore>,
}

impl OutboundQueue {
    pub(super) fn new() -> (Self, mpsc::Receiver<Frame>) {
        let (sender, receiver) = mpsc::channel(64);
        (
            Self {
                sender,
                budget: Arc::new(Semaphore::new(MAX_OUTBOUND_BYTES)),
            },
            receiver,
        )
    }

    pub(super) async fn send(
        &self,
        value: Value,
        state: &State,
        lifetime: Option<CancellationToken>,
    ) -> AgentResult<()> {
        let operation = async {
            // Reserve before serializing, so concurrent producers cannot each
            // allocate a full frame outside the shared output budget.
            let mut budget = self
                .budget
                .clone()
                .acquire_many_owned((MAX_FRAME + 1) as u32)
                .await
                .map_err(|_| AgentError::Disconnected("writer closed".into()))?;
            let mut buffer = CappedBuffer(Vec::new());
            serde_json::to_writer(&mut buffer, &value).map_err(|_| AgentError::Limit)?;
            buffer.0.push(b'\n');
            // Account for allocated capacity, not only logical payload length.
            let unused = MAX_FRAME + 1 - buffer.0.capacity();
            drop(budget.split(unused).expect("reserved maximum frame size"));
            state.trace.lock().unwrap().push("out", &value);
            self.sender
                .send(Frame {
                    bytes: buffer.0,
                    lifetime: lifetime.clone(),
                    _budget: budget,
                })
                .await
                .map_err(|_| AgentError::Disconnected("writer closed".into()))
        };
        tokio::select! {
            biased;
            () = state.stop.cancelled() => Err(AgentError::Disconnected("connection closed".into())),
            () = cancelled(lifetime.as_ref()) => Err(AgentError::Cancelled),
            result = tokio::time::timeout(Duration::from_secs(10), operation) => {
                result.map_err(|_| AgentError::Timeout)?
            }
        }
    }
}

struct CappedBuffer(Vec<u8>);

impl io::Write for CappedBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_FRAME.saturating_sub(self.0.len()) {
            return Err(io::Error::other("protocol frame exceeds limit"));
        }
        // Grow geometrically for escaped strings/arrays without Vec's implicit
        // doubling past the reserved maximum when the delimiter is appended.
        let required = self.0.len() + bytes.len() + 1;
        if required > self.0.capacity() {
            let capacity = self
                .0
                .capacity()
                .max(64)
                .saturating_mul(2)
                .max(required)
                .min(MAX_FRAME + 1);
            self.0.reserve_exact(capacity - self.0.len());
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

async fn cancelled(lifetime: Option<&CancellationToken>) {
    match lifetime {
        Some(lifetime) => lifetime.cancelled().await,
        None => std::future::pending::<()>().await,
    }
}

async fn write_frame(
    writer: &mut ProcessWriter,
    state: &State,
    frame: &Frame,
) -> Result<(), &'static str> {
    let mut offset = 0;
    while offset < frame.bytes.len() {
        let count = tokio::select! {
            biased;
            () = state.stop.cancelled() => return Err("connection closed"),
            () = cancelled(frame.lifetime.as_ref()) => {
                // Once any bytes reached stdin, another frame would corrupt the
                // newline-delimited stream. Never resume a cancelled partial write.
                return if offset == 0 {
                    Ok(())
                } else {
                    Err("request cancelled during protocol frame write")
                };
            }
            result = writer.write(&frame.bytes[offset..]) => {
                result.map_err(|_| "protocol stdin write failed")?
            }
        };
        if count == 0 {
            return Err("protocol stdin write returned zero");
        }
        offset += count;
    }
    // The complete frame, including its newline, has been handed to the writer.
    // A response may already expire its lifetime. That must not cancel flushing
    // or disconnect healthy sibling sessions. This is not remote-operation undo.
    tokio::select! {
        biased;
        () = state.stop.cancelled() => Err("connection closed"),
        result = writer.flush() => result.map_err(|_| "protocol stdin flush failed"),
    }
}

pub(super) async fn write_loop(
    mut writer: ProcessWriter,
    state: Arc<State>,
    mut outgoing: mpsc::Receiver<Frame>,
) {
    loop {
        let frame = tokio::select! {
            biased;
            () = state.stop.cancelled() => break,
            frame = outgoing.recv() => frame,
        };
        let Some(frame) = frame else {
            break;
        };
        match tokio::time::timeout(
            Duration::from_secs(30),
            write_frame(&mut writer, &state, &frame),
        )
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(reason)) => {
                state.fail(reason);
                break;
            }
            Err(_) => {
                state.fail("protocol stdin write timed out");
                break;
            }
        }
    }
    // Release queued byte permits before the bounded shutdown attempt.
    drop(outgoing);
    let _ = tokio::time::timeout(Duration::from_secs(1), writer.shutdown()).await;
}

#[cfg(test)]
#[path = "rpc_outbound_tests.rs"]
mod tests;

#[cfg(test)]
mod allocation_tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn maximum_frame_and_delimiter_fit_the_reserved_capacity() {
        let mut buffer = CappedBuffer(Vec::new());
        buffer.write_all(&vec![b'x'; MAX_FRAME]).unwrap();
        assert!(buffer.0.capacity() <= MAX_FRAME + 1);
        let capacity = buffer.0.capacity();
        buffer.0.push(b'\n');
        assert_eq!(buffer.0.capacity(), capacity);
    }

    #[test]
    fn many_small_segments_preserve_contents_and_the_capacity_cap() {
        let mut buffer = CappedBuffer(Vec::new());
        for _ in 0..4096 {
            buffer.write_all(b"abcdef").unwrap();
        }
        assert_eq!(buffer.0, b"abcdef".repeat(4096));
        assert!(buffer.0.capacity() <= MAX_FRAME + 1);
    }

    #[tokio::test]
    async fn frame_debug_never_includes_protocol_contents() {
        let budget = Arc::new(Semaphore::new(128));
        let frame = Frame {
            bytes: b"secret-canary-never-log".to_vec(),
            lifetime: None,
            _budget: budget.acquire_many_owned(128).await.unwrap(),
        };
        let debug = format!("{frame:?}");
        assert!(debug.contains("byte_count"));
        assert!(!debug.contains("secret-canary"));
    }
}
