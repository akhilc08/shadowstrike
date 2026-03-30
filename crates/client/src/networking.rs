use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, WebSocket};

use game_sim::input::Input;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkMode {
    Local,
    Connecting,
    WebSocketRelay,
    Disconnected,
}

/// Shared state between the NetworkManager and its WebSocket callbacks.
struct NetInner {
    mode: NetworkMode,
    room_code: Option<String>,
    player_id: u8,
    pending_inputs: Vec<(u64, Input)>,
    on_room_created: Option<js_sys::Function>,
    on_room_joined: Option<js_sys::Function>,
    on_peer_joined: Option<js_sys::Function>,
    on_error: Option<js_sys::Function>,
}

#[wasm_bindgen]
pub struct NetworkManager {
    ws: Option<WebSocket>,
    inner: Rc<RefCell<NetInner>>,
    // Stored closures so they aren't dropped while WebSocket is open
    _on_open: Option<Closure<dyn FnMut()>>,
    _on_message: Option<Closure<dyn FnMut(MessageEvent)>>,
    _on_close: Option<Closure<dyn FnMut()>>,
    _on_error: Option<Closure<dyn FnMut()>>,
}

impl Default for NetworkManager {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl NetworkManager {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        NetworkManager {
            ws: None,
            inner: Rc::new(RefCell::new(NetInner {
                mode: NetworkMode::Local,
                room_code: None,
                player_id: 0,
                pending_inputs: Vec::new(),
                on_room_created: None,
                on_room_joined: None,
                on_peer_joined: None,
                on_error: None,
            })),
            _on_open: None,
            _on_message: None,
            _on_close: None,
            _on_error: None,
        }
    }

    /// Set JS callback for when a room is created. Called with (room_code: string).
    pub fn set_on_room_created(&self, cb: js_sys::Function) {
        self.inner.borrow_mut().on_room_created = Some(cb);
    }

    /// Set JS callback for when we join a room. Called with (player_id: number).
    pub fn set_on_room_joined(&self, cb: js_sys::Function) {
        self.inner.borrow_mut().on_room_joined = Some(cb);
    }

    /// Set JS callback for when the peer joins. Called with no args.
    pub fn set_on_peer_joined(&self, cb: js_sys::Function) {
        self.inner.borrow_mut().on_peer_joined = Some(cb);
    }

    /// Set JS callback for errors. Called with (msg: string).
    pub fn set_on_error(&self, cb: js_sys::Function) {
        self.inner.borrow_mut().on_error = Some(cb);
    }

    /// Connect to relay and create a new room.
    pub fn create_room(&mut self, ws_url: &str) {
        self.connect(ws_url, None);
    }

    /// Connect to relay and join an existing room.
    pub fn join_room(&mut self, ws_url: &str, room_code: &str) {
        self.connect(ws_url, Some(room_code.to_string()));
    }

    fn connect(&mut self, ws_url: &str, join_code: Option<String>) {
        let ws = match WebSocket::new(ws_url) {
            Ok(ws) => ws,
            Err(_) => {
                self.inner.borrow_mut().mode = NetworkMode::Disconnected;
                return;
            }
        };

        self.inner.borrow_mut().mode = NetworkMode::Connecting;

        // onopen: send CreateRoom or JoinRoom
        let inner_open = self.inner.clone();
        let ws_open = ws.clone();
        let join_code_clone = join_code.clone();
        let on_open = Closure::wrap(Box::new(move || {
            let msg = if let Some(ref code) = join_code_clone {
                format!(r#"{{"type":"join_room","room_code":"{}"}}"#, code)
            } else {
                r#"{"type":"create_room"}"#.to_string()
            };
            let _ = ws_open.send_with_str(&msg);

            // Set player_id guess: creator=1, joiner=2
            let mut inner = inner_open.borrow_mut();
            inner.player_id = if join_code_clone.is_some() { 2 } else { 1 };
        }) as Box<dyn FnMut()>);
        ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));

        // onmessage: parse ServerMessage JSON
        let inner_msg = self.inner.clone();
        let on_message = Closure::wrap(Box::new(move |e: MessageEvent| {
            let text = match e.data().dyn_into::<js_sys::JsString>() {
                Ok(s) => String::from(s),
                Err(_) => return,
            };
            Self::handle_server_message(&inner_msg, &text);
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

        // onclose
        let inner_close = self.inner.clone();
        let on_close = Closure::wrap(Box::new(move || {
            inner_close.borrow_mut().mode = NetworkMode::Disconnected;
        }) as Box<dyn FnMut()>);
        ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));

        // onerror
        let inner_err = self.inner.clone();
        let on_error = Closure::wrap(Box::new(move || {
            inner_err.borrow_mut().mode = NetworkMode::Disconnected;
        }) as Box<dyn FnMut()>);
        ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));

        self.ws = Some(ws);
        self._on_open = Some(on_open);
        self._on_message = Some(on_message);
        self._on_close = Some(on_close);
        self._on_error = Some(on_error);
    }

    fn handle_server_message(inner: &Rc<RefCell<NetInner>>, text: &str) {
        // Minimal JSON parsing without serde (to avoid pulling serde into WASM)
        // Server messages have a "type" field.
        if text.contains("\"type\":\"room_created\"") {
            // Extract room_code
            if let Some(code) = extract_json_string(text, "room_code") {
                let mut state = inner.borrow_mut();
                state.room_code = Some(code.clone());
                state.mode = NetworkMode::WebSocketRelay;
                state.player_id = 1;
                if let Some(ref cb) = state.on_room_created {
                    let _ = cb.call1(&JsValue::NULL, &JsValue::from_str(&code));
                }
            }
        } else if text.contains("\"type\":\"room_joined\"") {
            if let Some(pid_str) = extract_json_string(text, "player_id") {
                let pid: u8 = pid_str.parse().unwrap_or(2);
                let mut state = inner.borrow_mut();
                state.player_id = pid;
                state.mode = NetworkMode::WebSocketRelay;
                if let Some(ref cb) = state.on_room_joined {
                    let _ = cb.call1(&JsValue::NULL, &JsValue::from_f64(pid as f64));
                }
            } else {
                // player_id might be a number, not a string
                let pid = extract_json_number(text, "player_id").unwrap_or(2.0) as u8;
                let mut state = inner.borrow_mut();
                state.player_id = pid;
                state.mode = NetworkMode::WebSocketRelay;
                if let Some(ref cb) = state.on_room_joined {
                    let _ = cb.call1(&JsValue::NULL, &JsValue::from_f64(pid as f64));
                }
            }
        } else if text.contains("\"type\":\"peer_joined\"") {
            let state = inner.borrow();
            if let Some(ref cb) = state.on_peer_joined {
                let _ = cb.call0(&JsValue::NULL);
            }
        } else if text.contains("\"type\":\"input_relay\"") {
            // Extract frame (number) and data (array of u8)
            if let Some(frame) = extract_json_number(text, "frame") {
                if let Some(data_arr) = extract_json_array_u8(text, "data") {
                    if !data_arr.is_empty() {
                        let input_bits = data_arr[0];
                        inner.borrow_mut().pending_inputs.push((frame as u64, Input(input_bits)));
                    }
                }
            }
        } else if text.contains("\"type\":\"error\"") {
            if let Some(msg) = extract_json_string(text, "msg") {
                let state = inner.borrow();
                if let Some(ref cb) = state.on_error {
                    let _ = cb.call1(&JsValue::NULL, &JsValue::from_str(&msg));
                }
            }
        }
    }

    /// Send local input to the remote peer via relay.
    pub fn send_input(&self, frame: u64, input_bits: u8) {
        if let Some(ref ws) = self.ws {
            if ws.ready_state() == WebSocket::OPEN {
                let msg = format!(
                    r#"{{"type":"input_relay","frame":{},"data":[{},0,0,0,0,0,0,0]}}"#,
                    frame, input_bits
                );
                let _ = ws.send_with_str(&msg);
            }
        }
    }

    /// Poll for received remote input. Returns Some((frame, Input)) if available.
    pub fn poll_input(&mut self) -> Option<Vec<u8>> {
        let mut inner = self.inner.borrow_mut();
        if inner.pending_inputs.is_empty() {
            return None;
        }
        let (frame, input) = inner.pending_inputs.remove(0);
        let mut result = Vec::with_capacity(9);
        result.extend_from_slice(&frame.to_be_bytes());
        result.push(input.0);
        Some(result)
    }

    pub fn mode(&self) -> u8 {
        match self.inner.borrow().mode {
            NetworkMode::Local => 0,
            NetworkMode::Connecting => 1,
            NetworkMode::WebSocketRelay => 3,
            NetworkMode::Disconnected => 4,
        }
    }

    pub fn room_code(&self) -> Option<String> {
        self.inner.borrow().room_code.clone()
    }

    pub fn player_id(&self) -> u8 {
        self.inner.borrow().player_id
    }

    pub fn is_connected(&self) -> bool {
        self.inner.borrow().mode == NetworkMode::WebSocketRelay
    }

    /// Called from JS when a WebSocket message is received containing remote input.
    pub fn receive_input(&mut self, frame: u64, input_bits: u8) {
        let mut inner = self.inner.borrow_mut();
        inner.pending_inputs.push((frame, Input(input_bits)));
        if inner.mode == NetworkMode::Connecting {
            inner.mode = NetworkMode::WebSocketRelay;
        }
    }
}

// --- Minimal JSON helpers (no serde dependency) ---

fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":\"", key);
    let start = json.find(&pattern)? + pattern.len();
    let end = json[start..].find('"')? + start;
    Some(json[start..end].to_string())
}

fn extract_json_number(json: &str, key: &str) -> Option<f64> {
    let pattern = format!("\"{}\":", key);
    let start = json.find(&pattern)? + pattern.len();
    let rest = json[start..].trim_start();
    let end = rest.find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-').unwrap_or(rest.len());
    rest[..end].parse::<f64>().ok()
}

fn extract_json_array_u8(json: &str, key: &str) -> Option<Vec<u8>> {
    let pattern = format!("\"{}\":[", key);
    let start = json.find(&pattern)? + pattern.len();
    let end = json[start..].find(']')? + start;
    let nums: Vec<u8> = json[start..end]
        .split(',')
        .filter_map(|s| s.trim().parse::<u8>().ok())
        .collect();
    Some(nums)
}
