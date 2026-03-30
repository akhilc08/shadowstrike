use serde::{Deserialize, Serialize};

/// Messages sent from client to server.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    CreateRoom,
    JoinRoom {
        room_code: String,
    },
    Signal {
        payload: serde_json::Value,
    },
    InputRelay {
        frame: u64,
        data: [u8; 8],
    },
}

/// Messages sent from server to client.
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    RoomCreated {
        room_code: String,
    },
    RoomJoined {
        player_id: u8,
    },
    PeerJoined,
    Signal {
        payload: serde_json::Value,
    },
    InputRelay {
        frame: u64,
        data: [u8; 8],
    },
    Error {
        msg: String,
    },
}
