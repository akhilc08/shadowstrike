use std::net::SocketAddr;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};

use crate::protocol::{ClientMessage, ServerMessage};
use crate::room::{self, RoomMap};

pub async fn handle_connection(raw_stream: TcpStream, addr: SocketAddr, rooms: RoomMap) {
    let ws_stream = match tokio_tungstenite::accept_async(raw_stream).await {
        Ok(ws) => ws,
        Err(e) => {
            warn!("WebSocket handshake failed for {}: {}", addr, e);
            return;
        }
    };

    info!("New WebSocket connection: {}", addr);

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Channel for sending messages to this client
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMessage>();

    // Track this player's room and slot
    let mut my_room: Option<String> = None;
    let mut my_slot: Option<usize> = None;

    // Spawn a task to forward messages from the channel to the WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let text = match serde_json::to_string(&msg) {
                Ok(t) => t,
                Err(_) => continue,
            };
            if ws_sender.send(Message::Text(text)).await.is_err() {
                break;
            }
        }
    });

    // Read loop
    while let Some(msg_result) = ws_receiver.next().await {
        let msg = match msg_result {
            Ok(Message::Text(text)) => text,
            Ok(Message::Close(_)) => break,
            Ok(_) => continue,
            Err(e) => {
                warn!("Error reading from {}: {}", addr, e);
                break;
            }
        };

        let client_msg: ClientMessage = match serde_json::from_str(&msg) {
            Ok(m) => m,
            Err(e) => {
                let _ = tx.send(ServerMessage::Error {
                    msg: format!("Invalid message: {}", e),
                });
                continue;
            }
        };

        match client_msg {
            ClientMessage::CreateRoom => {
                let mut rooms_guard = rooms.lock().await;
                // Generate a unique room code
                let code = loop {
                    let candidate = room::generate_room_code();
                    if !rooms_guard.contains_key(&candidate) {
                        break candidate;
                    }
                };
                let mut r = room::Room::new(code.clone());
                r.add_player(tx.clone());
                rooms_guard.insert(code.clone(), r);
                my_room = Some(code.clone());
                my_slot = Some(0);
                info!("Room {} created by {}", code, addr);
                let _ = tx.send(ServerMessage::RoomCreated { room_code: code });
            }
            ClientMessage::JoinRoom { room_code } => {
                let mut rooms_guard = rooms.lock().await;
                match rooms_guard.get_mut(&room_code) {
                    None => {
                        let _ = tx.send(ServerMessage::Error {
                            msg: format!("Room {} not found", room_code),
                        });
                    }
                    Some(r) => match r.add_player(tx.clone()) {
                        None => {
                            let _ = tx.send(ServerMessage::Error {
                                msg: "Room is full".to_string(),
                            });
                        }
                        Some(slot) => {
                            my_room = Some(room_code.clone());
                            my_slot = Some(slot);
                            info!("Player {} joined room {} as player {}", addr, room_code, slot + 1);
                            let _ = tx.send(ServerMessage::RoomJoined {
                                player_id: (slot + 1) as u8,
                            });
                            // Notify the other player
                            r.send_to_other(slot, ServerMessage::PeerJoined);
                        }
                    },
                }
            }
            ClientMessage::Signal { payload } => {
                if let (Some(ref code), Some(slot)) = (&my_room, my_slot) {
                    let rooms_guard = rooms.lock().await;
                    if let Some(r) = rooms_guard.get(code) {
                        r.send_to_other(slot, ServerMessage::Signal { payload });
                    }
                }
            }
            ClientMessage::InputRelay { frame, data } => {
                if let (Some(ref code), Some(slot)) = (&my_room, my_slot) {
                    let rooms_guard = rooms.lock().await;
                    if let Some(r) = rooms_guard.get(code) {
                        r.send_to_other(slot, ServerMessage::InputRelay { frame, data });
                    }
                }
            }
        }
    }

    // Cleanup on disconnect
    if let (Some(code), Some(slot)) = (my_room, my_slot) {
        let mut rooms_guard = rooms.lock().await;
        let should_remove = if let Some(r) = rooms_guard.get_mut(&code) {
            r.remove_player(slot);
            r.is_empty()
        } else {
            false
        };
        if should_remove {
            rooms_guard.remove(&code);
            info!("Room {} removed (all players disconnected)", code);
        }
    }

    info!("Connection {} closed", addr);
    send_task.abort();
}
