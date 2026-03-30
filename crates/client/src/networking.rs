use wasm_bindgen::prelude::*;
use web_sys::WebSocket;

use game_sim::input::Input;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkMode {
    Local,
    Connecting,
    WebRTC,
    WebSocketRelay,
    Disconnected,
}

#[wasm_bindgen]
pub struct NetworkManager {
    mode: NetworkMode,
    room_code: Option<String>,
    player_id: u8,
    ws: Option<WebSocket>,
    pending_inputs: Vec<(u64, Input)>,
}

#[wasm_bindgen]
impl NetworkManager {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        NetworkManager {
            mode: NetworkMode::Local,
            room_code: None,
            player_id: 0,
            ws: None,
            pending_inputs: Vec::new(),
        }
    }

    /// Connect to the relay server and create a new room.
    /// Returns a NetworkManager in Connecting state.
    pub fn create_room(ws_url: &str) -> Self {
        let ws = WebSocket::new(ws_url).ok();
        let mode = if ws.is_some() {
            NetworkMode::Connecting
        } else {
            NetworkMode::Disconnected
        };

        // TODO: Send CreateRoom message once connection opens
        // TODO: Set up onmessage callback to receive room code
        // TODO: Implement WebRTC signaling through relay for peer-to-peer upgrade

        NetworkManager {
            mode,
            room_code: None,
            player_id: 1,
            ws,
            pending_inputs: Vec::new(),
        }
    }

    /// Connect to the relay server and join an existing room.
    pub fn join_room(ws_url: &str, code: &str) -> Self {
        let ws = WebSocket::new(ws_url).ok();
        let mode = if ws.is_some() {
            NetworkMode::Connecting
        } else {
            NetworkMode::Disconnected
        };

        // TODO: Send JoinRoom message with code once connection opens
        // TODO: Set up onmessage callback
        // TODO: Implement WebRTC signaling through relay for peer-to-peer upgrade

        NetworkManager {
            mode,
            room_code: Some(code.to_string()),
            player_id: 2,
            ws,
            pending_inputs: Vec::new(),
        }
    }

    /// Send local input to the remote peer.
    pub fn send_input(&self, frame: u64, inputs: &[u8]) {
        if let Some(ws) = &self.ws {
            if ws.ready_state() == WebSocket::OPEN {
                // Frame (8 bytes big-endian) + input bytes
                let mut payload = Vec::with_capacity(8 + inputs.len());
                payload.extend_from_slice(&frame.to_be_bytes());
                payload.extend_from_slice(inputs);
                let _ = ws.send_with_u8_array(&payload);
            }
        }
        // TODO: When WebRTC data channel is available, prefer sending over
        // the data channel instead of the WebSocket relay for lower latency.
    }

    /// Poll for received remote input. Returns (frame, Input) if available.
    pub fn poll_input(&mut self) -> Option<Vec<u8>> {
        // TODO: Check WebRTC data channel first
        // TODO: Fall back to WebSocket relay messages
        // For now, inputs are pushed via JS callbacks into pending_inputs
        if self.pending_inputs.is_empty() {
            return None;
        }
        let (frame, input) = self.pending_inputs.remove(0);
        let mut result = Vec::with_capacity(9);
        result.extend_from_slice(&frame.to_be_bytes());
        result.push(input.0);
        Some(result)
    }

    pub fn mode(&self) -> u8 {
        match self.mode {
            NetworkMode::Local => 0,
            NetworkMode::Connecting => 1,
            NetworkMode::WebRTC => 2,
            NetworkMode::WebSocketRelay => 3,
            NetworkMode::Disconnected => 4,
        }
    }

    pub fn room_code(&self) -> Option<String> {
        self.room_code.clone()
    }

    pub fn player_id(&self) -> u8 {
        self.player_id
    }

    /// Called from JS when a WebSocket message is received containing remote input.
    pub fn receive_input(&mut self, frame: u64, input_bits: u8) {
        self.pending_inputs.push((frame, Input(input_bits)));
        // Upgrade mode to relay if still connecting
        if self.mode == NetworkMode::Connecting {
            self.mode = NetworkMode::WebSocketRelay;
        }
    }
}

impl Default for NetworkManager {
    fn default() -> Self {
        Self::new()
    }
}
