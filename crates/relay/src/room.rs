use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use rand::Rng;

use crate::protocol::ServerMessage;

pub type MessageSender = mpsc::UnboundedSender<ServerMessage>;

pub struct Room {
    pub code: String,
    pub players: [Option<MessageSender>; 2],
}

impl Room {
    pub fn new(code: String) -> Self {
        Self {
            code,
            players: [None, None],
        }
    }

    /// Returns the player slot index (0 or 1) if there's space, None if full.
    pub fn add_player(&mut self, sender: MessageSender) -> Option<usize> {
        for (i, slot) in self.players.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(sender);
                return Some(i);
            }
        }
        None
    }

    pub fn remove_player(&mut self, player_idx: usize) {
        if player_idx < 2 {
            self.players[player_idx] = None;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.players.iter().all(|p| p.is_none())
    }

    /// Send a message to the other player in the room.
    pub fn send_to_other(&self, sender_idx: usize, msg: ServerMessage) {
        let other_idx = 1 - sender_idx;
        if let Some(ref tx) = self.players[other_idx] {
            let _ = tx.send(msg);
        }
    }
}

pub type RoomMap = Arc<Mutex<HashMap<String, Room>>>;

pub fn new_room_map() -> RoomMap {
    Arc::new(Mutex::new(HashMap::new()))
}

/// Generate a 4-character uppercase alphanumeric room code.
pub fn generate_room_code() -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    (0..4)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}
