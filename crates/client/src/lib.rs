use std::collections::HashSet;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::CanvasRenderingContext2d;

use game_sim::input::Input;
use game_sim::player::{Element, PlayerAction};
use game_sim::ring_buffer::RingBuffer;
use game_sim::GameState;

pub mod animation;
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

/// Manages rollback netcode state: snapshots, input history, and re-simulation.
pub struct RollbackManager {
    /// Last 8 state snapshots for rollback
    snapshots: RingBuffer<GameState, 8>,
    /// Player 1 input history (up to 120 frames)
    p1_inputs: RingBuffer<Input, 120>,
    /// Player 2 input history (up to 120 frames)
    p2_inputs: RingBuffer<Input, 120>,
    /// Last frame where we received confirmed P2 input
    last_confirmed_p2_frame: u64,
    /// Predicted input for remote player (repeat last known)
    predicted_p2_input: Input,
}

impl RollbackManager {
    pub fn new(initial_state: &GameState) -> Self {
        let dummy_state = *initial_state;
        RollbackManager {
            snapshots: RingBuffer::new(dummy_state),
            p1_inputs: RingBuffer::new(Input(0)),
            p2_inputs: RingBuffer::new(Input(0)),
            last_confirmed_p2_frame: 0,
            predicted_p2_input: Input(0),
        }
    }

    /// Save a snapshot of the game state at the given frame.
    pub fn save_snapshot(&mut self, frame: u64, state: &GameState) {
        self.snapshots.write(frame, *state);
    }

    /// Retrieve a snapshot for the given frame, if still in the ring buffer.
    pub fn get_snapshot(&self, frame: u64) -> Option<&GameState> {
        self.snapshots.read(frame)
    }

    /// Record local (P1) input for a frame.
    pub fn record_p1_input(&mut self, frame: u64, input: Input) {
        self.p1_inputs.write(frame, input);
    }

    /// Record confirmed remote (P2) input for a frame.
    pub fn record_p2_input(&mut self, frame: u64, input: Input) {
        self.p2_inputs.write(frame, input);
        if frame > self.last_confirmed_p2_frame {
            self.last_confirmed_p2_frame = frame;
            self.predicted_p2_input = input;
        }
    }

    /// Predict remote input for a frame: repeat last confirmed input.
    pub fn predict_remote_input(&self, _frame: u64) -> Input {
        self.predicted_p2_input
    }

    /// Check if rollback is needed: the actual input differs from the prediction
    /// that was used for that frame.
    pub fn needs_rollback(&self, frame: u64, actual_input: Input) -> bool {
        match self.p2_inputs.read(frame) {
            Some(&predicted) => predicted != actual_input,
            None => true,
        }
    }

    /// Perform a rollback: restore state at from_frame, then re-simulate
    /// forward to current_frame using corrected inputs.
    pub fn perform_rollback(
        &mut self,
        game: &mut GameState,
        from_frame: u64,
        current_frame: u64,
        p1_inputs: &[Input],
        corrected_p2_input: Input,
    ) {
        // Restore snapshot
        if let Some(snapshot) = self.snapshots.read(from_frame) {
            game.restore_snapshot(*snapshot);
        }

        // Write corrected P2 input for the rollback frame
        self.p2_inputs.write(from_frame, corrected_p2_input);

        // Re-simulate from from_frame to current_frame
        for frame in from_frame..current_frame {
            let idx = (frame - from_frame) as usize;
            let p1 = if idx < p1_inputs.len() {
                p1_inputs[idx]
            } else {
                self.p1_inputs
                    .read(frame)
                    .copied()
                    .unwrap_or(Input(0))
            };
            let p2 = self.p2_inputs
                .read(frame)
                .copied()
                .unwrap_or(self.predicted_p2_input);

            game.tick(p1, p2);

            // Save updated snapshots as we re-simulate
            self.snapshots.write(frame + 1, *game);
        }
    }
}

#[wasm_bindgen]
pub struct ShadowStrike {
    game_state: GameState,
    anim_states: [AnimationState; 2],
    particles: ParticlePool,
    keys: HashSet<String>,
    last_tick: f64,
    tick_accumulator: f64,
    prev_actions: [AnimId; 2],
}

#[wasm_bindgen]
impl ShadowStrike {
    #[wasm_bindgen(constructor)]
    pub fn new(p1_element: u8, p2_element: u8) -> Self {
        let e1 = element_from_u8(p1_element);
        let e2 = element_from_u8(p2_element);
        ShadowStrike {
            game_state: GameState::new(e1, e2),
            anim_states: [AnimationState::new(), AnimationState::new()],
            particles: ParticlePool::new(),
            keys: HashSet::new(),
            last_tick: 0.0,
            tick_accumulator: 0.0,
            prev_actions: [AnimId::Idle; 2],
        }
    }

    pub fn key_down(&mut self, key: String) {
        self.keys.insert(key);
    }

    pub fn key_up(&mut self, key: String) {
        self.keys.remove(&key);
    }

    /// Called by requestAnimationFrame — timestamp in ms from performance.now().
    pub fn update(&mut self, timestamp: f64) {
        if self.last_tick == 0.0 {
            self.last_tick = timestamp;
            return;
        }

        let mut dt = timestamp - self.last_tick;
        self.last_tick = timestamp;

        // Clamp to avoid spiral of death
        if dt > 200.0 {
            dt = 200.0;
        }

        self.tick_accumulator += dt;

        while self.tick_accumulator >= TICK_DT {
            self.tick_accumulator -= TICK_DT;

            let p1_input = input_handler::read_p1_input(&self.keys);
            let p2_input = input_handler::read_p2_input(&self.keys);

            self.game_state.tick(p1_input, p2_input);

            // Update animation states from game state
            for i in 0..2 {
                let action = &self.game_state.players[i].action;
                let anim_id = AnimId::from_action(action);
                self.anim_states[i].set(anim_id);
                let anim = get_animation(anim_id);
                self.anim_states[i].advance(&anim);

                // Emit particles on action changes
                let prev = self.prev_actions[i];
                if anim_id != prev {
                    let px = self.game_state.players[i].x.to_f32();
                    let py = self.game_state.players[i].y.to_f32();
                    let elem = self.game_state.players[i].element;

                    match action {
                        PlayerAction::LightAttack1
                        | PlayerAction::LightAttack2
                        | PlayerAction::LightAttack3
                        | PlayerAction::HeavyAttack
                        | PlayerAction::Uppercut
                        | PlayerAction::AerialAttack => {
                            self.particles.emit(px, py - 30.0, elem, EffectType::SwordTrail);
                        }
                        PlayerAction::Hitstun { .. } => {
                            self.particles.emit(px, py - 30.0, elem, EffectType::HitImpact);
                        }
                        PlayerAction::WalkForward | PlayerAction::WalkBack => {
                            self.particles.emit(px, py, elem, EffectType::WalkDust);
                        }
                        _ => {}
                    }
                }
                self.prev_actions[i] = anim_id;
            }

            // Idle ambient particles (occasional)
            if self.game_state.frame_number.is_multiple_of(15) {
                for i in 0..2 {
                    let px = self.game_state.players[i].x.to_f32();
                    let py = self.game_state.players[i].y.to_f32();
                    let elem = self.game_state.players[i].element;
                    self.particles.emit(px, py - 40.0, elem, EffectType::IdleAmbient);
                }
            }

            self.particles.update(1.0 / 60.0);
        }
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

        renderer::render_frame(&ctx, &self.game_state, &self.anim_states);
        self.particles.render(&ctx);
    }
}
