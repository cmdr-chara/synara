use super::*;
use tokio::io::{AsyncBufReadExt, BufReader, DuplexStream, duplex, split};

fn pair() -> (RpcPeer, mpsc::Receiver<Incoming>, DuplexStream) {
    let (client, agent) = duplex(65536);
    let (reader, writer) = split(client);
    let (peer, incoming) = RpcPeer::start(
        Box::new(reader),
        Box::new(writer),
        Box::new(tokio::io::empty()),
    );
    (peer, incoming, agent)
}

async fn next_request(agent: &mut BufReader<DuplexStream>) -> RpcId {
    let mut line = String::new();
    tokio::time::timeout(Duration::from_secs(2), agent.read_line(&mut line))
        .await
        .unwrap()
        .unwrap();
    let value: Value = serde_json::from_str(&line).unwrap();
    RpcId::parse(&value["id"]).unwrap()
}

fn request(peer: &RpcPeer) -> tokio::task::JoinHandle<AgentResult<Value>> {
    let peer = peer.clone();
    tokio::spawn(async move {
        peer.request(
            "authenticate",
            json!({"methodId":"login"}),
            Duration::from_secs(10),
        )
        .await
    })
}

#[tokio::test]
async fn a_response_expires_its_own_interactions_before_completing() {
    let (peer, _incoming, agent) = pair();
    let mut agent = BufReader::new(agent);
    let first = request(&peer);
    let first_id = next_request(&mut agent).await;
    let first_lifetime = peer.request_lifetime(&first_id).unwrap();
    let second = request(&peer);
    let second_id = next_request(&mut agent).await;
    let second_lifetime = peer.request_lifetime(&second_id).unwrap();
    agent
        .get_mut()
        .write_all(
            format!(
                "{}\n",
                json!({"jsonrpc":"2.0","id":first_id.value(),"result":{}})
            )
            .as_bytes(),
        )
        .await
        .unwrap();
    first.await.unwrap().unwrap();
    assert!(first_lifetime.is_cancelled());
    assert!(peer.request_lifetime(&first_id).is_none());
    assert!(!second_lifetime.is_cancelled());
    second.abort();
    let _ = second.await;
    assert!(second_lifetime.is_cancelled());
    assert!(!peer.cancelled().is_cancelled());
}

#[tokio::test]
async fn dropping_a_request_expires_forms_without_disconnecting_siblings() {
    let (peer, _incoming, agent) = pair();
    let mut agent = BufReader::new(agent);
    let task = request(&peer);
    let id = next_request(&mut agent).await;
    let lifetime = peer.request_lifetime(&id).unwrap();
    task.abort();
    let _ = task.await;
    assert!(lifetime.is_cancelled());
    assert!(peer.request_lifetime(&id).is_none());
    assert!(!peer.cancelled().is_cancelled());
}

#[tokio::test]
async fn shutdown_expires_requests_and_retains_the_first_failure() {
    let (peer, _incoming, agent) = pair();
    let mut agent = BufReader::new(agent);
    let task = request(&peer);
    let id = next_request(&mut agent).await;
    let lifetime = peer.request_lifetime(&id).unwrap();
    peer.fail("original failure");
    peer.fail("later shutdown");
    assert_eq!(
        peer.failures().borrow().as_deref(),
        Some("original failure")
    );
    assert!(lifetime.is_cancelled());
    assert!(matches!(
        task.await.unwrap(),
        Err(AgentError::Disconnected(_))
    ));
    assert!(peer.state().pending.lock().unwrap().is_empty());
}
