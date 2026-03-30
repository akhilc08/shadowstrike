use std::collections::HashSet;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::CanvasRenderingContext2d;

use game_sim::input::Input;
use game_sim::player::{Element, PlayerAction};
use game_sim::ring_buffer::RingBuffer;
use game_sim::{GamePhase, GameState};

pub mod animation;
pub mod audio;
pub mod input_handler;
pub mod networking;
pub mod particles;
pub mod renderer;

use animation::{AnimId, AnimationState, get_animation};
use particles::{EffectType, ParticlePool};

const TICK_DT: f64 = 1000.0 / 60.0; // ~16.667ms per tick

fn element_from_u8(v: u8) -> Element {
    match v {
        0 => Element::Fire,
        1 => Element::Lightning,
        2 => Element::DarkMagic,
        3 => Element::Ice,
        _ => Element::Fire,
    }
}

/// Rollback netcode manager: snapshots, input history, prediction, and re-simulation.
pub struct RollbackManager {
    /// Last 8 state snapshots for rollback
    snapshots: RingBuffer<GameState, 8>,
    /// Player 1 input history (up to 120 frames)
    p1_inputs: RingBuffer<Input, 120>,
    /// Player 2 input history (up to 120 frames)
    p2_inputs: RingBuffer<Input, 120>,
    /// Which player slot is local (1 or 2)
    local_player: u8,
    /// Last frame where we received confirmed remote input
    last_confirmed_remote_frame: u64,
    /// Predicted input for remote player (repeats last confirmed)
    predicted_remote_input: Input,
}

impl RollbackManager {
    pub fn new(initial_state: &GameState, local_player: u8) -> Self {
        let mut rm = RollbackManager {
            snapshots: RingBuffer::new(*initial_state),
            p1_inputs: RingBuffer::new(Input(0)),
            p2_inputs: RingBuffer::new(Input(0)),
            local_player,
            last_confirmed_remote_frame: 0,
            predicted_remote_input: Input(0),
        };
        rm.snapshots.write(0, *initial_state);
        rm
    }

    pub fn save_snapshot(&mut self, frame: u64, state: &GameState) {
        self.snapshots.write(frame, *state);
    }

    pub fn record_local_input(&mut self, frame: u64, input: Input) {
        if self.local_player == 1 {
            self.p1_inputs.write(frame, input);
        } else {
            self.p2_inputs.write(frame, input);
        }
    }

    /// Record confirmed remote input. Returns true if rollback is needed
    /// (the frame was previously simulated with a different predicted input).
    pub fn record_remote_input(&mut self, frame: u64, input: Input) -> bool {
        let needs_rollback = {
            let buffer = if self.local_player == 1 {
                &self.p2_inputs
            } else {
                &self.p1_inputs
            };
            match buffer.read(frame) {
                Some(&stored) => stored != input,
                None => false,
            }
        };

        if self.local_player == 1 {
            self.p2_inputs.write(frame, input);
        } else {
            self.p1_inputs.write(frame, input);
        }

        if frame > self.last_confirmed_remote_frame {
            self.last_confirmed_remote_frame = frame;
            self.predicted_remote_input = input;
        }

        needs_rollback
    }

    /// Write predicted remote input for current frame so mispredictions
    /// can be detected when actual input arrives later.
    pub fn write_remote_prediction(&mut self, frame: u64) {
        let pred = self.predicted_remote_input;
        if self.local_player == 1 {
            self.p2_inputs.write(frame, pred);
        } else {
            self.p1_inputs.write(frame, pred);
        }
    }

    pub fn predict_remote_input(&self) -> Input {
        self.predicted_remote_input
    }

    /// Rollback to from_frame snapshot and resimulate up to current_frame.
    pub fn perform_rollback(
        &mut self,
        game: &mut GameState,
        from_frame: u64,
        current_frame: u64,
    ) {
        if let Some(snapshot) = self.snapshots.read(from_frame) {
            game.restore_snapshot(*snapshot);
        }
        for f in from_frame..current_frame {
            let p1 = self.p1_inputs.read(f).copied().unwrap_or(Input(0));
            let p2 = self.p2_inputs.read(f).copied().unwrap_or(Input(0));
            game.tick(p1, p2);
            self.snapshots.write(f + 1, *game);
        }
    }
}

#[wasm_bindgen]
pub struct ShadowStrike {
    game_state: GameState,
    anim_states: [AnimationState; 2],
    particles: ParticlePool,
    sound: audio::SoundEngine,
    keys: HashSet<String>,
    last_tick: f64,
    tick_accumulator: f64,
    prev_actions: [AnimId; 2],
    // Online multiplayer state
    net: Option<networking::NetworkManager>,
    rollback: Option<RollbackManager>,
    is_online: bool,
    local_player: u8, // 1 or 2
    // Visual effects
    hit_flash_frames: [i32; 2],
    screen_shake_frames: i32,
    screen_shake_intensity: f64,
    // Sound state
    bell_played: bool,
    // Touch input from mobile virtual gamepad
    touch_input: Input,
    // Debug overlay (F1 toggle)
    debug_overlay: bool,
    last_p1_input: Input,
    last_p2_input: Input,
}

#[wasm_bindgen]
impl ShadowStrike {
    #[wasm_bindgen(constructor)]
    pub fn new(p1_element: u8, p2_element: u8) -> Self {
        let e1 = element_from_u8(p1_element);
        let e2 = element_from_u8(p2_element);
        let state = GameState::new(e1, e2);
        ShadowStrike {
            game_state: state,
            anim_states: [AnimationState::new(), AnimationState::new()],
            particles: ParticlePool::new(),
            sound: audio::SoundEngine::new(),
            keys: HashSet::new(),
            last_tick: 0.0,
            tick_accumulator: 0.0,
            prev_actions: [AnimId::Idle; 2],
            net: None,
            rollback: None,
            is_online: false,
            local_player: 1,
            hit_flash_frames: [0; 2],
            screen_shake_frames: 0,
            screen_shake_intensity: 0.0,
            bell_played: false,
            touch_input: Input(0),
            debug_overlay: false,
            last_p1_input: Input(0),
            last_p2_input: Input(0),
        }
    }

    /// Get the NetworkManager for JS to configure callbacks.
    pub fn get_network_manager(&mut self) -> networking::NetworkManager {
        networking::NetworkManager::new()
    }

    /// Start online mode: create a room.
    pub fn create_online_room(&mut self, ws_url: &str) {
        let mut net = networking::NetworkManager::new();
        net.create_room(ws_url);
        self.is_online = true;
        self.local_player = 1;
        self.rollback = Some(RollbackManager::new(&self.game_state, 1));
        self.net = Some(net);
    }

    /// Start online mode: join a room.
    pub fn join_online_room(&mut self, ws_url: &str, room_code: &str) {
        let mut net = networking::NetworkManager::new();
        net.join_room(ws_url, room_code);
        self.is_online = true;
        self.local_player = 2;
        self.rollback = Some(RollbackManager::new(&self.game_state, 2));
        self.net = Some(net);
    }

    /// Get network mode (0=local, 1=connecting, 2=webrtc_connecting, 3=relay, 4=disconnected, 5=webrtc_direct)
    pub fn network_mode(&self) -> u8 {
        self.net.as_ref().map(|n| n.mode()).unwrap_or(0)
    }

    /// Check if currently using WebRTC P2P
    pub fn is_p2p(&self) -> bool {
        self.net.as_ref().map(|n| n.is_p2p()).unwrap_or(false)
    }

    /// Get room code if available
    pub fn room_code(&self) -> Option<String> {
        self.net.as_ref().and_then(|n| n.room_code())
    }

    /// Set JS callbacks on the internal network manager.
    pub fn set_on_room_created(&self, cb: js_sys::Function) {
        if let Some(ref net) = self.net {
            net.set_on_room_created(cb);
        }
    }
    pub fn set_on_room_joined(&self, cb: js_sys::Function) {
        if let Some(ref net) = self.net {
            net.set_on_room_joined(cb);
        }
    }
    pub fn set_on_peer_joined(&self, cb: js_sys::Function) {
        if let Some(ref net) = self.net {
            net.set_on_peer_joined(cb);
        }
    }
    pub fn set_on_net_error(&self, cb: js_sys::Function) {
        if let Some(ref net) = self.net {
            net.set_on_error(cb);
        }
    }

    pub fn key_down(&mut self, key: String) {
        if key == "F1" {
            self.debug_overlay = !self.debug_overlay;
            return;
        }
        self.keys.insert(key);
    }

    pub fn key_up(&mut self, key: String) {
        self.keys.remove(&key);
    }

    /// Set raw input bitmask from touch controls (mobile virtual gamepad).
    /// Bits: 0=left, 1=right, 2=up, 3=down, 4=light, 5=heavy, 6=special, 7=block
    pub fn set_touch_input(&mut self, bits: u8) {
        self.touch_input = Input(bits);
    }

    /// Get game phase as string for JS UI overlays.
    pub fn phase_info(&self) -> String {
        match self.game_state.phase {
            GamePhase::Fighting => "fighting".to_string(),
            GamePhase::RoundEnd { winner, countdown } => {
                format!("round_end:{}:{}", winner, countdown)
            }
            GamePhase::MatchEnd { winner } => {
                format!("match_end:{}", winner)
            }
        }
    }

    pub fn round_number(&self) -> i32 {
        self.game_state.round_number
    }

    pub fn p1_health(&self) -> i32 {
        self.game_state.players[0].health
    }

    pub fn p2_health(&self) -> i32 {
        self.game_state.players[1].health
    }

    /// Called by requestAnimationFrame — timestamp in ms from performance.now().
    pub fn update(&mut self, timestamp: f64) {
        if self.last_tick == 0.0 {
            self.last_tick = timestamp;
            return;
        }

        let mut dt = timestamp - self.last_tick;
        self.last_tick = timestamp;

        if dt > 200.0 {
            dt = 200.0;
        }

        self.tick_accumulator += dt;

        while self.tick_accumulator >= TICK_DT {
            self.tick_accumulator -= TICK_DT;
            self.run_game_tick();
        }

        // Play round start bell
        if self.game_state.frame_number == 40 && !self.bell_played {
            self.sound.play_bell();
            self.bell_played = true;
        }
        // Reset bell for next round
        if self.game_state.frame_number == 0 {
            self.bell_played = false;
        }
    }

    fn run_game_tick(&mut self) {
        let frame = self.game_state.frame_number;

        let (p1_input, p2_input) = if self.is_online {
            // Online mode: always use WASD (P1 controls) for the local player, merged with touch
            let kb_input = input_handler::read_p1_input(&self.keys);
            let local_input = Input(kb_input.0 | self.touch_input.0);

            // Send local input to remote peer via relay
            if let Some(ref net) = self.net {
                net.send_input(frame, local_input.0);
            }

            // Record local input and save state snapshot before tick
            if let Some(ref mut rm) = self.rollback {
                rm.record_local_input(frame, local_input);
                rm.save_snapshot(frame, &self.game_state);
            }

            // Collect all pending remote inputs from network
            let remote_inputs: Vec<(u64, Input)> = {
                let mut inputs = Vec::new();
                if let Some(ref mut net) = self.net {
                    while let Some(data) = net.poll_input() {
                        if data.len() >= 9 {
                            let remote_frame = u64::from_be_bytes([
                                data[0], data[1], data[2], data[3],
                                data[4], data[5], data[6], data[7],
                            ]);
                            inputs.push((remote_frame, Input(data[8])));
                        }
                    }
                }
                inputs
            };

            // Process remote inputs against rollback manager, detect mispredictions
            let mut earliest_correction: Option<u64> = None;
            if let Some(ref mut rm) = self.rollback {
                for &(remote_frame, remote_input) in &remote_inputs {
                    let needs = rm.record_remote_input(remote_frame, remote_input);
                    if needs && remote_frame < frame {
                        earliest_correction = Some(match earliest_correction {
                            Some(f) => f.min(remote_frame),
                            None => remote_frame,
                        });
                    }
                }
            }

            // Perform rollback and resimulation if misprediction detected
            if let Some(correction_frame) = earliest_correction {
                if let Some(ref mut rm) = self.rollback {
                    rm.perform_rollback(&mut self.game_state, correction_frame, frame);
                }
            }

            // Predict remote input for this frame and store in buffer
            let predicted = self
                .rollback
                .as_ref()
                .map(|rm| rm.predict_remote_input())
                .unwrap_or(Input(0));

            if let Some(ref mut rm) = self.rollback {
                rm.write_remote_prediction(frame);
            }

            if self.local_player == 1 {
                (local_input, predicted)
            } else {
                (predicted, local_input)
            }
        } else {
            // Local mode: both players on same keyboard, P1 also gets touch input
            let p1_kb = input_handler::read_p1_input(&self.keys);
            let p1 = Input(p1_kb.0 | self.touch_input.0);
            let p2 = input_handler::read_p2_input(&self.keys);
            (p1, p2)
        };

        // Track inputs for debug overlay
        self.last_p1_input = p1_input;
        self.last_p2_input = p2_input;

        // Store health before tick for hit detection
        let health_before = [
            self.game_state.players[0].health,
            self.game_state.players[1].health,
        ];

        self.game_state.tick(p1_input, p2_input);

        // Detect hits for visual effects and sound
        for (i, &prev_hp) in health_before.iter().enumerate() {
            if self.game_state.players[i].health < prev_hp {
                self.hit_flash_frames[i] = 6;
                self.screen_shake_frames = 4;
                let damage = prev_hp - self.game_state.players[i].health;
                self.screen_shake_intensity = (damage as f64 / 100.0).clamp(2.0, 8.0);
                // Sound based on damage
                if self.game_state.players[i].health <= 0 {
                    self.sound.play_ko();
                } else if damage >= 60 {
                    self.sound.play_sword_clash();
                } else {
                    self.sound.play_punch();
                }
            }
        }

        // Decay visual effects
        for i in 0..2 {
            if self.hit_flash_frames[i] > 0 {
                self.hit_flash_frames[i] -= 1;
            }
        }
        if self.screen_shake_frames > 0 {
            self.screen_shake_frames -= 1;
        }

        // Update animation states from game state
        for i in 0..2 {
            let action = &self.game_state.players[i].action;
            let anim_id = AnimId::from_action(action);
            self.anim_states[i].set(anim_id);
            let anim = get_animation(anim_id);
            self.anim_states[i].advance(&anim);

            // Use actual player element for particle theme
            let visual_elem = self.game_state.players[i].element;

            // Emit particles and play sounds on action changes
            let prev = self.prev_actions[i];
            if anim_id != prev {
                let px = self.game_state.players[i].x.to_f32();
                let py = self.game_state.players[i].y.to_f32();

                // Sound effects for new actions
                match action {
                    PlayerAction::Jump => self.sound.play_jump(),
                    PlayerAction::Block => {}
                    PlayerAction::Blockstun { .. } => self.sound.play_block(),
                    PlayerAction::Fireball => self.sound.play_fireball(),
                    PlayerAction::DashStrike => self.sound.play_dash(),
                    PlayerAction::ShadowSurge => self.sound.play_shadow_surge(),
                    PlayerAction::VoidDash => self.sound.play_void_dash(),
                    _ => {}
                }

                match action {
                    PlayerAction::Uppercut => {
                        self.particles
                            .emit(px, py - 30.0, visual_elem, EffectType::SpecialActivation);
                        self.particles
                            .emit(px, py - 30.0, visual_elem, EffectType::SwordTrail);
                    }
                    PlayerAction::Fireball => {
                        self.particles
                            .emit(px, py - 40.0, visual_elem, EffectType::SpecialActivation);
                    }
                    PlayerAction::ShadowSurge => {
                        self.particles
                            .emit(px, py - 40.0, visual_elem, EffectType::SpecialActivation);
                        self.particles
                            .emit(px, py - 30.0, visual_elem, EffectType::SwordTrail);
                    }
                    PlayerAction::DashStrike => {
                        self.particles
                            .emit(px, py - 30.0, visual_elem, EffectType::SpecialActivation);
                        self.particles
                            .emit(px, py - 30.0, visual_elem, EffectType::SwordTrail);
                    }
                    PlayerAction::VoidDash => {
                        // Shadow burst at departure point
                        self.particles
                            .emit(px, py - 30.0, visual_elem, EffectType::SpecialActivation);
                    }
                    PlayerAction::LightAttack1
                    | PlayerAction::LightAttack2
                    | PlayerAction::LightAttack3
                    | PlayerAction::HeavyAttack
                    | PlayerAction::AerialAttack => {
                        self.particles
                            .emit(px, py - 30.0, visual_elem, EffectType::SwordTrail);
                    }
                    PlayerAction::Hitstun { .. } => {
                        self.particles
                            .emit(px, py - 30.0, visual_elem, EffectType::HitImpact);
                    }
                    PlayerAction::Blockstun { .. } => {
                        self.particles
                            .emit(px, py - 40.0, visual_elem, EffectType::BlockSpark);
                    }
                    PlayerAction::Knockdown { .. } => {
                        self.particles
                            .emit(px, py, visual_elem, EffectType::KnockdownSlam);
                    }
                    PlayerAction::WalkForward | PlayerAction::WalkBack => {
                        self.particles
                            .emit(px, py, visual_elem, EffectType::WalkDust);
                    }
                    _ => {}
                }
            }
            self.prev_actions[i] = anim_id;
        }

        // Idle ambient particles (occasional)
        if self.game_state.frame_number % 15 == 0 {
            for i in 0..2 {
                let px = self.game_state.players[i].x.to_f32();
                let py = self.game_state.players[i].y.to_f32();
                let visual_elem = self.game_state.players[i].element;
                self.particles
                    .emit(px, py - 40.0, visual_elem, EffectType::IdleAmbient);
            }
        }

        self.particles.update(1.0 / 60.0);
    }

    pub fn render(&self, canvas_id: &str) {
        let document = web_sys::window()
            .expect("no window")
            .document()
            .expect("no document");
        let canvas = document
            .get_element_by_id(canvas_id)
            .expect("canvas not found")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("not a canvas");
        let ctx = canvas
            .get_context("2d")
            .expect("no 2d context")
            .expect("null context")
            .dyn_into::<CanvasRenderingContext2d>()
            .expect("not CanvasRenderingContext2d");

        // Apply screen shake
        if self.screen_shake_frames > 0 {
            let intensity = self.screen_shake_intensity;
            let frame_seed = self.game_state.frame_number as f64;
            let ox = (frame_seed * 7.3).sin() * intensity;
            let oy = (frame_seed * 11.7).cos() * intensity;
            ctx.save();
            ctx.translate(ox, oy).ok();
        }

        renderer::render_frame(
            &ctx,
            &self.game_state,
            &self.anim_states,
            &self.hit_flash_frames,
        );
        // Render projectiles
        renderer::render_projectiles(&ctx, &self.game_state.projectiles);
        self.particles.render(&ctx);

        // Draw game phase overlays
        self.render_phase_overlay(&ctx);

        if self.screen_shake_frames > 0 {
            ctx.restore();
        }

        // F1 debug overlay (drawn after shake restore so it's stable)
        if self.debug_overlay {
            renderer::render_debug_overlay(
                &ctx,
                &self.game_state,
                self.last_p1_input,
                self.last_p2_input,
            );
        }
    }

    fn render_phase_overlay(&self, ctx: &CanvasRenderingContext2d) {
        let cx = 600.0;
        let cy = 280.0;

        match self.game_state.phase {
            GamePhase::Fighting => {
                // Show "ROUND X" / "FIGHT!" at the start of each round
                if self.game_state.frame_number < 90 {
                    let alpha = if self.game_state.frame_number < 60 {
                        1.0
                    } else {
                        1.0 - (self.game_state.frame_number as f64 - 60.0) / 30.0
                    };
                    let color = format!("rgba(255,255,255,{:.2})", alpha);
                    ctx.set_fill_style_str(&color);
                    ctx.set_font("bold 48px monospace");
                    ctx.set_text_align("center");
                    if self.game_state.frame_number < 40 {
                        let text = format!("ROUND {}", self.game_state.round_number);
                        let _ = ctx.fill_text(&text, cx, cy);
                    } else {
                        let _ = ctx.fill_text("FIGHT!", cx, cy);
                    }
                }
            }
            GamePhase::RoundEnd { winner, countdown } => {
                ctx.set_font("bold 56px monospace");
                ctx.set_text_align("center");

                // "KO!" flash
                if countdown > 80 {
                    ctx.set_fill_style_str("#ff3333");
                    let _ = ctx.fill_text("K.O.!", cx, cy);
                } else {
                    let winner_text = format!("Player {} wins the round!", winner + 1);
                    ctx.set_fill_style_str("#ffffff");
                    ctx.set_font("bold 36px monospace");
                    let _ = ctx.fill_text(&winner_text, cx, cy);
                }
            }
            GamePhase::MatchEnd { winner } => {
                // Darken background
                ctx.set_fill_style_str("rgba(0,0,0,0.6)");
                ctx.fill_rect(0.0, 0.0, 1200.0, 600.0);

                ctx.set_font("bold 64px monospace");
                ctx.set_text_align("center");
                ctx.set_fill_style_str("#ffd700");
                let _ = ctx.fill_text("MATCH OVER", cx, cy - 40.0);

                ctx.set_font("bold 40px monospace");
                ctx.set_fill_style_str("#ffffff");
                let winner_text = format!("Player {} Wins!", winner + 1);
                let _ = ctx.fill_text(&winner_text, cx, cy + 30.0);

                ctx.set_font("24px monospace");
                ctx.set_fill_style_str("#888888");
                let _ = ctx.fill_text("Refresh to play again", cx, cy + 80.0);
            }
        }
    }
}
