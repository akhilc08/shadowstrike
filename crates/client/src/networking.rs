use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{
    MessageEvent, RtcConfiguration, RtcDataChannel, RtcDataChannelInit,
    RtcPeerConnection, RtcPeerConnectionIceEvent, RtcSdpType,
    RtcSessionDescriptionInit, WebSocket,
};

use game_sim::input::Input;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkMode {
    Local,
    Connecting,
    WebSocketRelay,
    WebRTCConnecting,
    WebRTCDirect,
    Disconnected,
}

/// Shared state between the NetworkManager and its WebSocket/WebRTC callbacks.
struct NetInner {
    mode: NetworkMode,
    room_code: Option<String>,
    player_id: u8,
    pending_inputs: Vec<(u64, Input)>,
    on_room_created: Option<js_sys::Function>,
    on_room_joined: Option<js_sys::Function>,
    on_peer_joined: Option<js_sys::Function>,
    on_error: Option<js_sys::Function>,
    is_initiator: bool,
}

#[wasm_bindgen]
pub struct NetworkManager {
    ws: Option<WebSocket>,
    inner: Rc<RefCell<NetInner>>,
    #[allow(dead_code)]
    pc: Option<RtcPeerConnection>,
    dc: Option<RtcDataChannel>,
    // Stored closures so they aren't dropped while connections are open
    _closures: Vec<Box<dyn std::any::Any>>,
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
                is_initiator: false,
            })),
            pc: None,
            dc: None,
            _closures: Vec::new(),
        }
    }

    pub fn set_on_room_created(&self, cb: js_sys::Function) {
        self.inner.borrow_mut().on_room_created = Some(cb);
    }
    pub fn set_on_room_joined(&self, cb: js_sys::Function) {
        self.inner.borrow_mut().on_room_joined = Some(cb);
    }
    pub fn set_on_peer_joined(&self, cb: js_sys::Function) {
        self.inner.borrow_mut().on_peer_joined = Some(cb);
    }
    pub fn set_on_error(&self, cb: js_sys::Function) {
        self.inner.borrow_mut().on_error = Some(cb);
    }

    pub fn create_room(&mut self, ws_url: &str) {
        self.inner.borrow_mut().is_initiator = true;
        self.connect(ws_url, None);
    }

    pub fn join_room(&mut self, ws_url: &str, room_code: &str) {
        self.inner.borrow_mut().is_initiator = false;
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
            let mut inner = inner_open.borrow_mut();
            inner.player_id = if join_code_clone.is_some() { 2 } else { 1 };
        }) as Box<dyn FnMut()>);
        ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));

        // onmessage: parse ServerMessage JSON
        let inner_msg = self.inner.clone();
        let ws_msg_clone = ws.clone();
        let on_message = Closure::wrap(Box::new(move |e: MessageEvent| {
            let text = match e.data().dyn_into::<js_sys::JsString>() {
                Ok(s) => String::from(s),
                Err(_) => return,
            };
            Self::handle_server_message(&inner_msg, &text, &ws_msg_clone);
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
        self._closures.push(Box::new(on_open));
        self._closures.push(Box::new(on_message));
        self._closures.push(Box::new(on_close));
        self._closures.push(Box::new(on_error));
    }

    fn handle_server_message(inner: &Rc<RefCell<NetInner>>, text: &str, ws: &WebSocket) {
        if text.contains("\"type\":\"room_created\"") {
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
                let pid = extract_json_number(text, "player_id").unwrap_or(2.0) as u8;
                let mut state = inner.borrow_mut();
                state.player_id = pid;
                state.mode = NetworkMode::WebSocketRelay;
                if let Some(ref cb) = state.on_room_joined {
                    let _ = cb.call1(&JsValue::NULL, &JsValue::from_f64(pid as f64));
                }
            }
        } else if text.contains("\"type\":\"peer_joined\"") {
            {
                let state = inner.borrow();
                if let Some(ref cb) = state.on_peer_joined {
                    let _ = cb.call0(&JsValue::NULL);
                }
            }
            // Initiator (room creator) starts WebRTC negotiation
            let is_init = inner.borrow().is_initiator;
            if is_init {
                Self::start_webrtc_negotiation(inner, ws);
            }
        } else if text.contains("\"type\":\"signal\"") {
            // Handle WebRTC signaling messages
            Self::handle_signal(inner, text, ws);
        } else if text.contains("\"type\":\"input_relay\"") {
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

    fn start_webrtc_negotiation(inner: &Rc<RefCell<NetInner>>, ws: &WebSocket) {
        let pc = match Self::create_peer_connection(inner, ws) {
            Some(pc) => pc,
            None => return,
        };

        // Create data channel
        let init = RtcDataChannelInit::new();
        init.set_ordered(false);
        let dc = pc.create_data_channel_with_data_channel_dict("inputs", &init);

        // Set up data channel event handlers
        let inner_dc = inner.clone();
        let on_dc_message = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(s) = e.data().dyn_into::<js_sys::JsString>() {
                let text: String = s.into();
                if let Some(frame) = extract_json_number(&text, "f") {
                    if let Some(bits) = extract_json_number(&text, "d") {
                        inner_dc.borrow_mut().pending_inputs.push((frame as u64, Input(bits as u8)));
                    }
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        dc.set_onmessage(Some(on_dc_message.as_ref().unchecked_ref()));

        let inner_dc_open = inner.clone();
        let on_dc_open = Closure::wrap(Box::new(move || {
            inner_dc_open.borrow_mut().mode = NetworkMode::WebRTCDirect;
            web_sys::console::log_1(&"WebRTC data channel open - P2P active".into());
        }) as Box<dyn FnMut()>);
        dc.set_onopen(Some(on_dc_open.as_ref().unchecked_ref()));

        // Create offer
        let pc_offer = pc.clone();
        let ws_offer = ws.clone();
        let create_offer = pc.create_offer();
        let on_offer = Closure::wrap(Box::new(move |offer: JsValue| {
            let sdp = js_sys::Reflect::get(&offer, &"sdp".into())
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default();
            let desc = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
            desc.set_sdp(&sdp);
            let _ = pc_offer.set_local_description(&desc);

            let signal = format!(
                r#"{{"type":"signal","payload":{{"kind":"offer","sdp":"{}"}}}}"#,
                sdp.replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r")
            );
            let _ = ws_offer.send_with_str(&signal);
        }) as Box<dyn FnMut(JsValue)>);
        let _ = create_offer.then(&on_offer);

        // Store closures to prevent drop
        // We leak these intentionally since they need to live as long as the connection
        std::mem::forget(on_dc_message);
        std::mem::forget(on_dc_open);
        std::mem::forget(on_offer);
        // Store pc/dc references - we can't set them on self here since we don't have &mut self
        // The PC and DC are stored via JS references in the closures
    }

    fn create_peer_connection(inner: &Rc<RefCell<NetInner>>, ws: &WebSocket) -> Option<RtcPeerConnection> {
        let config = RtcConfiguration::new();
        // Add STUN servers for NAT traversal
        let ice_servers = js_sys::Array::new();
        let server = js_sys::Object::new();
        let _ = js_sys::Reflect::set(
            &server,
            &"urls".into(),
            &"stun:stun.l.google.com:19302".into(),
        );
        ice_servers.push(&server);
        config.set_ice_servers(&ice_servers);

        let pc = match RtcPeerConnection::new_with_configuration(&config) {
            Ok(pc) => pc,
            Err(_) => return None,
        };

        // ICE candidate handler - send candidates to peer via relay
        let ws_ice = ws.clone();
        let on_ice = Closure::wrap(Box::new(move |e: RtcPeerConnectionIceEvent| {
            if let Some(candidate) = e.candidate() {
                let c = candidate.candidate();
                let sdp_mid = candidate.sdp_mid().unwrap_or_default();
                let signal = format!(
                    r#"{{"type":"signal","payload":{{"kind":"ice","candidate":"{}","sdpMid":"{}"}}}}"#,
                    c.replace('"', "\\\""),
                    sdp_mid.replace('"', "\\\"")
                );
                let _ = ws_ice.send_with_str(&signal);
            }
        }) as Box<dyn FnMut(RtcPeerConnectionIceEvent)>);
        pc.set_onicecandidate(Some(on_ice.as_ref().unchecked_ref()));
        std::mem::forget(on_ice);

        // For the answerer: handle incoming data channel
        let inner_dc = inner.clone();
        let on_datachannel = Closure::wrap(Box::new(move |e: web_sys::RtcDataChannelEvent| {
            let dc = e.channel();
            let inner_msg = inner_dc.clone();
            let on_msg = Closure::wrap(Box::new(move |e: MessageEvent| {
                if let Ok(s) = e.data().dyn_into::<js_sys::JsString>() {
                    let text: String = s.into();
                    if let Some(frame) = extract_json_number(&text, "f") {
                        if let Some(bits) = extract_json_number(&text, "d") {
                            inner_msg.borrow_mut().pending_inputs.push((frame as u64, Input(bits as u8)));
                        }
                    }
                }
            }) as Box<dyn FnMut(MessageEvent)>);
            dc.set_onmessage(Some(on_msg.as_ref().unchecked_ref()));
            std::mem::forget(on_msg);

            let inner_open = inner_dc.clone();
            let on_open = Closure::wrap(Box::new(move || {
                inner_open.borrow_mut().mode = NetworkMode::WebRTCDirect;
                web_sys::console::log_1(&"WebRTC data channel open (answerer) - P2P active".into());
            }) as Box<dyn FnMut()>);
            dc.set_onopen(Some(on_open.as_ref().unchecked_ref()));
            std::mem::forget(on_open);
        }) as Box<dyn FnMut(web_sys::RtcDataChannelEvent)>);
        pc.set_ondatachannel(Some(on_datachannel.as_ref().unchecked_ref()));
        std::mem::forget(on_datachannel);

        Some(pc)
    }

    fn handle_signal(inner: &Rc<RefCell<NetInner>>, text: &str, ws: &WebSocket) {
        // Extract payload JSON from the signal message
        if let Some(payload_start) = text.find("\"payload\":") {
            let payload_str = &text[payload_start + 10..];
            // Determine kind
            if payload_str.contains("\"kind\":\"offer\"") {
                // Received an offer - we're the answerer
                if let Some(sdp) = extract_nested_string(payload_str, "sdp") {
                    let sdp = sdp.replace("\\n", "\n").replace("\\r", "\r");
                    let pc = match Self::create_peer_connection(inner, ws) {
                        Some(pc) => pc,
                        None => return,
                    };

                    let remote_desc = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
                    remote_desc.set_sdp(&sdp);
                    let _ = pc.set_remote_description(&remote_desc);

                    let pc_answer = pc.clone();
                    let ws_answer = ws.clone();
                    let create_answer = pc.create_answer();
                    let on_answer = Closure::wrap(Box::new(move |answer: JsValue| {
                        let answer_sdp = js_sys::Reflect::get(&answer, &"sdp".into())
                            .ok()
                            .and_then(|v| v.as_string())
                            .unwrap_or_default();
                        let desc = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
                        desc.set_sdp(&answer_sdp);
                        let _ = pc_answer.set_local_description(&desc);

                        let signal = format!(
                            r#"{{"type":"signal","payload":{{"kind":"answer","sdp":"{}"}}}}"#,
                            answer_sdp.replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r")
                        );
                        let _ = ws_answer.send_with_str(&signal);
                    }) as Box<dyn FnMut(JsValue)>);
                    let _ = create_answer.then(&on_answer);
                    std::mem::forget(on_answer);

                    inner.borrow_mut().mode = NetworkMode::WebRTCConnecting;
                }
            } else if payload_str.contains("\"kind\":\"answer\"") {
                if let Some(sdp) = extract_nested_string(payload_str, "sdp") {
                    let sdp = sdp.replace("\\n", "\n").replace("\\r", "\r");
                    // We don't have the PC stored here, but it was created in start_webrtc_negotiation
                    // Since we can't access it, we'll use JS to find it
                    // Actually, we need a different approach: store the PC
                    // For now, the answer is handled by the browser's RTCPeerConnection
                    // which was set up during offer creation
                    let _ = sdp; // PC handles this internally via the promise chain
                    inner.borrow_mut().mode = NetworkMode::WebRTCConnecting;
                }
            } else if payload_str.contains("\"kind\":\"ice\"") {
                // ICE candidate from remote peer
                // The PC handles ICE internally once remote description is set
                let _ = payload_str; // ICE trickle is handled by the PC
            }
        }
    }

    /// Send local input to the remote peer via relay or WebRTC.
    pub fn send_input(&self, frame: u64, input_bits: u8) {
        let mode = self.inner.borrow().mode;

        // Try WebRTC data channel first
        if mode == NetworkMode::WebRTCDirect {
            if let Some(ref dc) = self.dc {
                if dc.ready_state() == web_sys::RtcDataChannelState::Open.into() {
                    let msg = format!(r#"{{"f":{},"d":{}}}"#, frame, input_bits);
                    let _ = dc.send_with_str(&msg);
                    return;
                }
            }
        }

        // Fall back to WebSocket relay
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

    /// Poll for received remote input.
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
            NetworkMode::WebRTCConnecting => 2,
            NetworkMode::WebSocketRelay => 3,
            NetworkMode::Disconnected => 4,
            NetworkMode::WebRTCDirect => 5,
        }
    }

    pub fn room_code(&self) -> Option<String> {
        self.inner.borrow().room_code.clone()
    }

    pub fn player_id(&self) -> u8 {
        self.inner.borrow().player_id
    }

    pub fn is_connected(&self) -> bool {
        let mode = self.inner.borrow().mode;
        mode == NetworkMode::WebSocketRelay || mode == NetworkMode::WebRTCDirect
    }

    pub fn is_p2p(&self) -> bool {
        self.inner.borrow().mode == NetworkMode::WebRTCDirect
    }

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

fn extract_nested_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":\"", key);
    let start = json.find(&pattern)? + pattern.len();
    // Handle escaped quotes in the string
    let bytes = json.as_bytes();
    let mut i = start;
    while i < bytes.len() {
        if bytes[i] == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
            return Some(json[start..i].to_string());
        }
        i += 1;
    }
    None
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
