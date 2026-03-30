use std::time::Duration;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Start the relay server on a random port and return the address.
async fn start_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let rooms = relay::room::new_room_map();

    tokio::spawn(async move {
        loop {
            if let Ok((stream, peer_addr)) = listener.accept().await {
                let rooms = rooms.clone();
                tokio::spawn(relay::server::handle_connection(stream, peer_addr, rooms));
            }
        }
    });

    format!("ws://127.0.0.1:{}", addr.port())
}

async fn connect(url: &str) -> (
    futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>,
    futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>,
) {
    let (ws, _) = connect_async(url).await.unwrap();
    ws.split()
}

async fn send(sink: &mut futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>, msg: &str) {
    sink.send(Message::Text(msg.to_string())).await.unwrap();
}

async fn recv(stream: &mut futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>) -> serde_json::Value {
    let msg = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("timeout waiting for message")
        .expect("stream ended")
        .expect("error reading message");
    match msg {
        Message::Text(t) => serde_json::from_str(&t).unwrap(),
        other => panic!("expected text message, got {:?}", other),
    }
}

#[tokio::test]
async fn test_create_and_join_room() {
    let url = start_server().await;

    // Player 1 creates room
    let (mut sink1, mut stream1) = connect(&url).await;
    send(&mut sink1, r#"{"type":"create_room"}"#).await;
    let resp = recv(&mut stream1).await;
    assert_eq!(resp["type"], "room_created");
    let room_code = resp["room_code"].as_str().unwrap().to_string();
    assert_eq!(room_code.len(), 4);

    // Player 2 joins room
    let (mut sink2, mut stream2) = connect(&url).await;
    send(&mut sink2, &format!(r#"{{"type":"join_room","room_code":"{}"}}"#, room_code)).await;
    let resp2 = recv(&mut stream2).await;
    assert_eq!(resp2["type"], "room_joined");
    assert_eq!(resp2["player_id"], 2);

    // Player 1 gets peer_joined notification
    let notif = recv(&mut stream1).await;
    assert_eq!(notif["type"], "peer_joined");

    // Cleanup
    drop(sink1);
    drop(sink2);
}

#[tokio::test]
async fn test_signal_forwarding() {
    let url = start_server().await;

    let (mut sink1, mut stream1) = connect(&url).await;
    send(&mut sink1, r#"{"type":"create_room"}"#).await;
    let resp = recv(&mut stream1).await;
    let room_code = resp["room_code"].as_str().unwrap().to_string();

    let (mut sink2, mut stream2) = connect(&url).await;
    send(&mut sink2, &format!(r#"{{"type":"join_room","room_code":"{}"}}"#, room_code)).await;
    let _ = recv(&mut stream2).await; // room_joined
    let _ = recv(&mut stream1).await; // peer_joined

    // P1 sends signal
    send(&mut sink1, r#"{"type":"signal","payload":{"sdp":"offer123"}}"#).await;
    let sig = recv(&mut stream2).await;
    assert_eq!(sig["type"], "signal");
    assert_eq!(sig["payload"]["sdp"], "offer123");

    // P2 sends signal back
    send(&mut sink2, r#"{"type":"signal","payload":{"sdp":"answer456"}}"#).await;
    let sig2 = recv(&mut stream1).await;
    assert_eq!(sig2["type"], "signal");
    assert_eq!(sig2["payload"]["sdp"], "answer456");
}

#[tokio::test]
async fn test_input_relay() {
    let url = start_server().await;

    let (mut sink1, mut stream1) = connect(&url).await;
    send(&mut sink1, r#"{"type":"create_room"}"#).await;
    let resp = recv(&mut stream1).await;
    let room_code = resp["room_code"].as_str().unwrap().to_string();

    let (mut sink2, mut stream2) = connect(&url).await;
    send(&mut sink2, &format!(r#"{{"type":"join_room","room_code":"{}"}}"#, room_code)).await;
    let _ = recv(&mut stream2).await; // room_joined
    let _ = recv(&mut stream1).await; // peer_joined

    // P1 sends input
    send(&mut sink1, r#"{"type":"input_relay","frame":42,"data":[1,2,3,4,5,6,7,8]}"#).await;
    let relay = recv(&mut stream2).await;
    assert_eq!(relay["type"], "input_relay");
    assert_eq!(relay["frame"], 42);
    assert_eq!(relay["data"], serde_json::json!([1,2,3,4,5,6,7,8]));
}

#[tokio::test]
async fn test_room_cleanup() {
    let url = start_server().await;
    let rooms = relay::room::new_room_map();
    // We can't directly access the server's room map, but we can verify
    // that after both players disconnect, a new player can create a room
    // with the same code concept (rooms are cleaned up).

    let (mut sink1, mut stream1) = connect(&url).await;
    send(&mut sink1, r#"{"type":"create_room"}"#).await;
    let resp = recv(&mut stream1).await;
    let room_code = resp["room_code"].as_str().unwrap().to_string();

    let (mut sink2, mut stream2) = connect(&url).await;
    send(&mut sink2, &format!(r#"{{"type":"join_room","room_code":"{}"}}"#, room_code)).await;
    let _ = recv(&mut stream2).await;
    let _ = recv(&mut stream1).await;

    // Disconnect both players
    drop(sink1);
    drop(stream1);
    drop(sink2);
    drop(stream2);

    // Give the server time to process disconnections
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Try to join the old room — should fail since it was cleaned up
    let (mut sink3, mut stream3) = connect(&url).await;
    send(&mut sink3, &format!(r#"{{"type":"join_room","room_code":"{}"}}"#, room_code)).await;
    let resp3 = recv(&mut stream3).await;
    assert_eq!(resp3["type"], "error");

    // Verify the room map we created locally is independent (sanity check)
    assert!(rooms.lock().await.is_empty());
}

#[tokio::test]
async fn test_max_players() {
    let url = start_server().await;

    let (mut sink1, mut stream1) = connect(&url).await;
    send(&mut sink1, r#"{"type":"create_room"}"#).await;
    let resp = recv(&mut stream1).await;
    let room_code = resp["room_code"].as_str().unwrap().to_string();

    let (mut sink2, mut stream2) = connect(&url).await;
    send(&mut sink2, &format!(r#"{{"type":"join_room","room_code":"{}"}}"#, room_code)).await;
    let _ = recv(&mut stream2).await; // room_joined
    let _ = recv(&mut stream1).await; // peer_joined

    // Third player tries to join
    let (mut sink3, mut stream3) = connect(&url).await;
    send(&mut sink3, &format!(r#"{{"type":"join_room","room_code":"{}"}}"#, room_code)).await;
    let resp3 = recv(&mut stream3).await;
    assert_eq!(resp3["type"], "error");
    assert_eq!(resp3["msg"], "Room is full");
}
