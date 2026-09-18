use super::super::{Incoming, Owner, RpcPeer, TraceLog};
use super::*;
use serde_json::json;
use std::{
    collections::{HashMap, HashSet},
    sync::{Mutex, atomic::AtomicU64},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader, DuplexStream, duplex},
    sync::watch,
};

fn peer_without_writer() -> (RpcPeer, mpsc::Receiver<Frame>) {
    let (failure, _) = watch::channel(None);
    let state = Arc::new(State {
        pending: Mutex::new(HashMap::new()),
        incoming_ids: Mutex::new(HashSet::new()),
        trace: Mutex::new(TraceLog::default()),
        stop: CancellationToken::new(),
        failure,
    });
    let (writer, outgoing) = OutboundQueue::new();
    let (incoming, _) = mpsc::channel::<Incoming>(1);
    (
        RpcPeer {
            owner: Arc::new(Owner { state }),
            writer,
            incoming,
            next: Arc::new(AtomicU64::new(1)),
        },
        outgoing,
    )
}

async fn line(reader: &mut BufReader<DuplexStream>) -> Value {
    let mut text = String::new();
    let count = tokio::time::timeout(Duration::from_secs(2), reader.read_line(&mut text))
        .await
        .unwrap()
        .unwrap();
    assert!(count > 0);
    serde_json::from_str(&text).unwrap()
}

async fn unsent_request_is_skipped(use_timeout: bool) {
    let (peer, mut outgoing) = peer_without_writer();
    let request = tokio::spawn({
        let peer = peer.clone();
        async move {
            peer.request(
                "session/new",
                json!({"cwd":"/not-to-be-executed"}),
                if use_timeout {
                    Duration::from_millis(20)
                } else {
                    Duration::from_secs(60)
                },
            )
            .await
        }
    });
    // Receiving the frame proves the request reached the output queue. No
    // transport writer exists yet, making this independent of scheduler speed.
    let frame = tokio::time::timeout(Duration::from_secs(2), outgoing.recv())
        .await
        .unwrap()
        .unwrap();
    let lifetime = frame.lifetime.clone().unwrap();
    peer.writer.sender.send(frame).await.unwrap();
    if use_timeout {
        assert!(matches!(request.await.unwrap(), Err(AgentError::Timeout)));
    } else {
        request.abort();
        assert!(request.await.unwrap_err().is_cancelled());
    }
    assert!(lifetime.is_cancelled());
    assert!(peer.state().pending.lock().unwrap().is_empty());
    peer.notify("session/cancel", json!({"sessionId":"live"}))
        .await
        .unwrap();
    let (writer, agent) = duplex(4096);
    let task = tokio::spawn(write_loop(Box::new(writer), peer.state().clone(), outgoing));
    let mut agent = BufReader::new(agent);
    let sent = line(&mut agent).await;
    assert_eq!(sent["method"], "session/cancel");
    assert!(sent.get("id").is_none());
    assert!(!peer.cancelled().is_cancelled());
    peer.fail("test complete");
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(peer.writer.budget.available_permits(), MAX_OUTBOUND_BYTES);
}

#[tokio::test]
async fn timed_out_queued_requests_never_reach_agent_stdin() {
    unsent_request_is_skipped(true).await;
}

#[tokio::test]
async fn aborted_queued_requests_never_reach_agent_stdin() {
    unsent_request_is_skipped(false).await;
}

#[tokio::test]
async fn cancelling_a_partial_frame_closes_stream_without_sending_siblings() {
    let (peer, outgoing) = peer_without_writer();
    let lifetime = CancellationToken::new();
    peer.writer
        .send(
            json!({"jsonrpc":"2.0","id":1,"method":"session/new","params":{}}),
            peer.state(),
            Some(lifetime.clone()),
        )
        .await
        .unwrap();
    peer.notify("session/cancel", json!({"sessionId":"sibling"}))
        .await
        .unwrap();
    let (writer, mut agent) = duplex(1);
    let task = tokio::spawn(write_loop(Box::new(writer), peer.state().clone(), outgoing));
    let mut prefix = [0; 1];
    tokio::time::timeout(Duration::from_secs(2), agent.read_exact(&mut prefix))
        .await
        .unwrap()
        .unwrap();
    lifetime.cancel();
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
    let mut rest = Vec::new();
    agent.read_to_end(&mut rest).await.unwrap();
    assert_eq!(prefix, [b'{']);
    assert!(!rest.contains(&b'\n'));
    assert!(rest.len() <= 1);
    assert!(peer.cancelled().is_cancelled());
    assert_eq!(
        peer.failures().borrow().as_deref(),
        Some("request cancelled during protocol frame write")
    );
    assert_eq!(peer.writer.budget.available_permits(), MAX_OUTBOUND_BYTES);
}

#[tokio::test]
async fn cancelling_a_delivered_frame_keeps_sibling_sessions_usable() {
    let (peer, outgoing) = peer_without_writer();
    let lifetime = CancellationToken::new();
    peer.writer
        .send(
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
            peer.state(),
            Some(lifetime.clone()),
        )
        .await
        .unwrap();
    let (writer, agent) = duplex(4096);
    let task = tokio::spawn(write_loop(Box::new(writer), peer.state().clone(), outgoing));
    let mut agent = BufReader::new(agent);
    assert_eq!(line(&mut agent).await["id"], 1);
    lifetime.cancel();
    peer.notify("session/cancel", json!({"sessionId":"sibling"}))
        .await
        .unwrap();
    assert_eq!(line(&mut agent).await["params"]["sessionId"], "sibling");
    assert!(!peer.cancelled().is_cancelled());
    peer.fail("test complete");
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn oversized_serialization_releases_its_reservation_without_enqueuing() {
    let (peer, mut outgoing) = peer_without_writer();
    let result = peer
        .writer
        .send(json!("x".repeat(MAX_FRAME)), peer.state(), None)
        .await;
    assert!(matches!(result, Err(AgentError::Limit)));
    assert!(matches!(
        outgoing.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
    assert_eq!(peer.writer.budget.available_permits(), MAX_OUTBOUND_BYTES);
    assert!(!peer.cancelled().is_cancelled());
}

#[tokio::test]
async fn serialized_frames_and_blocked_producers_share_one_byte_budget() {
    let (peer, mut outgoing) = peer_without_writer();
    for _ in 0..2 {
        peer.writer
            .send(json!("x".repeat(MAX_FRAME - 2)), peer.state(), None)
            .await
            .unwrap();
    }
    assert_eq!(peer.writer.budget.available_permits(), 0);
    let lifetime = CancellationToken::new();
    let blocked = peer
        .writer
        .send(json!({}), peer.state(), Some(lifetime.clone()));
    tokio::pin!(blocked);
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut blocked)
            .await
            .is_err()
    );
    lifetime.cancel();
    assert!(matches!(blocked.await, Err(AgentError::Cancelled)));
    assert_eq!(peer.writer.budget.available_permits(), 0);
    drop(outgoing.recv().await.unwrap());
    assert_eq!(peer.writer.budget.available_permits(), MAX_FRAME + 1);
    drop(outgoing.recv().await.unwrap());
    assert_eq!(peer.writer.budget.available_permits(), MAX_OUTBOUND_BYTES);
    assert!(matches!(
        outgoing.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
}

#[tokio::test]
async fn closed_writer_queue_does_not_leak_a_serialization_reservation() {
    let (peer, outgoing) = peer_without_writer();
    drop(outgoing);
    assert!(matches!(
        peer.notify("session/cancel", json!({})).await,
        Err(AgentError::Disconnected(_))
    ));
    assert_eq!(peer.writer.budget.available_permits(), MAX_OUTBOUND_BYTES);
}

#[tokio::test]
async fn already_cancelled_requests_do_not_allocate_or_enqueue_frames() {
    let (peer, mut outgoing) = peer_without_writer();
    let lifetime = CancellationToken::new();
    lifetime.cancel();
    assert!(matches!(
        peer.writer
            .send(json!({}), peer.state(), Some(lifetime))
            .await,
        Err(AgentError::Cancelled)
    ));
    assert!(matches!(
        outgoing.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
    assert_eq!(peer.writer.budget.available_permits(), MAX_OUTBOUND_BYTES);
}

#[tokio::test]
async fn connection_shutdown_wakes_a_blocked_budget_waiter() {
    let (peer, outgoing) = peer_without_writer();
    let reservation = peer
        .writer
        .budget
        .clone()
        .acquire_many_owned(MAX_OUTBOUND_BYTES as u32)
        .await
        .unwrap();
    let blocked = peer.notify("session/cancel", json!({}));
    tokio::pin!(blocked);
    assert!(
        tokio::time::timeout(Duration::from_millis(10), &mut blocked)
            .await
            .is_err()
    );
    peer.fail("explicit shutdown");
    assert!(matches!(blocked.await, Err(AgentError::Disconnected(_))));
    drop(reservation);
    drop(outgoing);
    assert_eq!(peer.writer.budget.available_permits(), MAX_OUTBOUND_BYTES);
}
