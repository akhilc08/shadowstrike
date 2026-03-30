pub mod animation;
pub mod collision;
pub mod fixed_point;
pub mod game_state;
pub mod input;
pub mod rollback;
pub mod simulation;

use game_state::GameState;
use input::InputFrame;
use wasm_bindgen::prelude::*;

/// Create a new initial game state, returned as JSON.
#[wasm_bindgen]
pub fn new_game() -> String {
    let state = GameState::initial();
    serde_json::to_string(&state).unwrap_or_default()
}

/// Advance the game by one tick. All args and return are JSON strings.
#[wasm_bindgen]
pub fn game_tick(state_json: &str, p1_input_json: &str, p2_input_json: &str) -> String {
    let state: GameState = match serde_json::from_str(state_json) {
        Ok(s) => s,
        Err(_) => return String::new(),
    };
    let p1: InputFrame = match serde_json::from_str(p1_input_json) {
        Ok(i) => i,
        Err(_) => return String::new(),
    };
    let p2: InputFrame = match serde_json::from_str(p2_input_json) {
        Ok(i) => i,
        Err(_) => return String::new(),
    };
    let next = simulation::tick(&state, p1, p2);
    serde_json::to_string(&next).unwrap_or_default()
}

/// Get the hash of a game state for determinism verification.
#[wasm_bindgen]
pub fn hash_game_state(state_json: &str) -> u64 {
    let state: GameState = match serde_json::from_str(state_json) {
        Ok(s) => s,
        Err(_) => return 0,
    };
    state.hash_state()
}
