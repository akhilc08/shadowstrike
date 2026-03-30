use crate::animation::{action_frame_count, get_frame_data};
use crate::collision::{check_hit, CombatantInfo};
use crate::fixed_point::FixedPoint;
use crate::game_state::{ActionState, GameState, PlayerState, RoundPhase};
use crate::input::{InputFrame, BTN_CROUCH, BTN_HEAVY, BTN_JUMP, BTN_LEFT, BTN_LIGHT, BTN_RIGHT, BTN_SPECIAL};

const STAGE_X_MIN: i32 = 0;
const STAGE_X_MAX: i32 = 1_280_000; // fixed-point raw
const GRAVITY: i32 = -600; // fixed-point raw units per tick
const WALK_SPEED: i32 = 3_000; // fixed-point raw
const JUMP_VEL: i32 = 12_000; // fixed-point raw
const KNOCKDOWN_THRESHOLD: u8 = 7;

/// Pure function: advance game state by one tick.
pub fn tick(state: &GameState, p1_input: InputFrame, p2_input: InputFrame) -> GameState {
    let mut s = *state;

    // Step 0: Non-fighting phases
    if s.round_phase != RoundPhase::Fighting {
        return handle_non_fighting(&mut s);
    }

    // Step 1: Timer
    if s.timer_frames > 0 {
        s.timer_frames -= 1;
    }
    if s.timer_frames == 0 {
        return handle_timeout(s);
    }

    // Clear per-frame flags
    s.p1.hit_this_frame = false;
    s.p2.hit_this_frame = false;

    // Step 2-3: Process inputs for each player
    process_player_input(&mut s.p1, &p1_input);
    process_player_input(&mut s.p2, &p2_input);

    // Step 4: Gravity
    apply_gravity(&mut s.p1);
    apply_gravity(&mut s.p2);

    // Step 5: Update positions, clamp to stage
    update_position(&mut s.p1);
    update_position(&mut s.p2);

    // Step 6: Auto-facing
    if s.p1.pos_x < s.p2.pos_x {
        s.p1.facing = 1;
        s.p2.facing = -1;
    } else if s.p1.pos_x > s.p2.pos_x {
        s.p1.facing = -1;
        s.p2.facing = 1;
    }

    // Step 7: Advance action frames
    s.p1.action_frame = s.p1.action_frame.saturating_add(1);
    s.p2.action_frame = s.p2.action_frame.saturating_add(1);

    // Step 8: Handle action completion
    handle_action_completion(&mut s.p1);
    handle_action_completion(&mut s.p2);

    // Step 9: Decrement stun frames
    s.p1.hitstun_frames = s.p1.hitstun_frames.saturating_sub(1);
    s.p1.blockstun_frames = s.p1.blockstun_frames.saturating_sub(1);
    s.p2.hitstun_frames = s.p2.hitstun_frames.saturating_sub(1);
    s.p2.blockstun_frames = s.p2.blockstun_frames.saturating_sub(1);

    // Transition out of stun states when frames reach 0
    if s.p1.action == ActionState::Hitstun && s.p1.hitstun_frames == 0 {
        s.p1.action = ActionState::Idle;
        s.p1.action_frame = 0;
        // Reset combo when hitstun ends
        if s.combo_hit_count > 0 {
            s.combo_hit_count = 0;
            s.hitstun_scale_num = 100;
        }
    }
    if s.p1.action == ActionState::Blockstun && s.p1.blockstun_frames == 0 {
        s.p1.action = ActionState::Idle;
        s.p1.action_frame = 0;
    }
    if s.p2.action == ActionState::Hitstun && s.p2.hitstun_frames == 0 {
        s.p2.action = ActionState::Idle;
        s.p2.action_frame = 0;
        if s.combo_hit_count > 0 {
            s.combo_hit_count = 0;
            s.hitstun_scale_num = 100;
        }
    }
    if s.p2.action == ActionState::Blockstun && s.p2.blockstun_frames == 0 {
        s.p2.action = ActionState::Idle;
        s.p2.action_frame = 0;
    }

    // Step 10-12: Collision detection and hit/block resolution
    resolve_combat(&mut s, true);  // p1 attacks p2
    resolve_combat(&mut s, false); // p2 attacks p1

    // Step 13: Check win conditions
    if s.p1.health <= 0 || s.p1.guard_meter <= 0 {
        s.round_phase = RoundPhase::KO;
        s.p2_rounds_won += 1;
    } else if s.p2.health <= 0 || s.p2.guard_meter <= 0 {
        s.round_phase = RoundPhase::KO;
        s.p1_rounds_won += 1;
    }

    // Check for match end
    if s.round_phase == RoundPhase::KO
        && (s.p1_rounds_won >= 2 || s.p2_rounds_won >= 2)
    {
        s.round_phase = RoundPhase::MatchEnd;
    }

    // Step 14: Increment frame
    s.frame_number += 1;

    s
}

fn handle_non_fighting(s: &mut GameState) -> GameState {
    match s.round_phase {
        RoundPhase::Intro => {
            s.round_phase = RoundPhase::Fighting;
        }
        RoundPhase::KO | RoundPhase::RoundEnd => {
            // Start new round
            s.round_number += 1;
            s.p1 = PlayerState {
                pos_x: FixedPoint::from_pixels(300),
                pos_y: FixedPoint::ZERO,
                facing: 1,
                health: 1000,
                energy: 0,
                guard_meter: 100,
                ..PlayerState::default()
            };
            s.p2 = PlayerState {
                pos_x: FixedPoint::from_pixels(980),
                pos_y: FixedPoint::ZERO,
                facing: -1,
                health: 1000,
                energy: 0,
                guard_meter: 100,
                ..PlayerState::default()
            };
            s.combo_hit_count = 0;
            s.hitstun_scale_num = 100;
            s.timer_frames = GameState::TIMER_INITIAL;
            s.round_phase = RoundPhase::Fighting;
        }
        RoundPhase::MatchEnd => {
            // Match is over, do nothing
        }
        RoundPhase::Fighting => {} // handled in main tick
    }
    s.frame_number += 1;
    *s
}

fn handle_timeout(mut s: GameState) -> GameState {
    // Lower health wins
    if s.p1.health > s.p2.health {
        s.p1_rounds_won += 1;
    } else if s.p2.health > s.p1.health {
        s.p2_rounds_won += 1;
    } else {
        // Draw: both get a round (can lead to extra rounds)
        s.p1_rounds_won += 1;
        s.p2_rounds_won += 1;
    }
    if s.p1_rounds_won >= 2 || s.p2_rounds_won >= 2 {
        s.round_phase = RoundPhase::MatchEnd;
    } else {
        s.round_phase = RoundPhase::RoundEnd;
    }
    s.frame_number += 1;
    s
}

fn is_in_stun(p: &PlayerState) -> bool {
    p.hitstun_frames > 0 || p.blockstun_frames > 0
}

fn is_in_attack(action: ActionState) -> bool {
    matches!(
        action,
        ActionState::LightAttack
            | ActionState::HeavyAttack
            | ActionState::SpecialAttack
            | ActionState::CrouchLight
            | ActionState::CrouchHeavy
            | ActionState::AirLight
            | ActionState::AirHeavy
    )
}

fn process_player_input(p: &mut PlayerState, input: &InputFrame) {
    if is_in_stun(p) {
        return;
    }
    if p.action == ActionState::Knockdown || p.action == ActionState::Getup {
        return;
    }

    // Cancel window: if in an attack and the frame data says cancellable, allow new attack
    if is_in_attack(p.action) {
        let frame_data = get_frame_data(p.action, p.action_frame);
        if frame_data.is_cancellable && p.hit_this_frame {
            // Allow cancel into a different attack
            if input.pressed(BTN_HEAVY) && p.action != ActionState::HeavyAttack {
                start_attack(p, input, true);
                return;
            }
            if input.pressed(BTN_SPECIAL) && p.action != ActionState::SpecialAttack {
                start_attack(p, input, true);
                return;
            }
        }
        // Otherwise, can't act during attack
        return;
    }

    // Attack inputs (priority: special > heavy > light)
    if input.pressed(BTN_SPECIAL) {
        start_attack(p, input, false);
        return;
    }
    if input.pressed(BTN_HEAVY) {
        start_attack(p, input, false);
        return;
    }
    if input.pressed(BTN_LIGHT) {
        start_attack(p, input, false);
        return;
    }

    // Jump
    if input.pressed(BTN_JUMP) && !p.is_airborne {
        p.action = ActionState::Jump;
        p.action_frame = 0;
        p.vel_y = FixedPoint::raw(JUMP_VEL);
        p.is_airborne = true;
        // Allow horizontal influence on jump
        if input.pressed(BTN_LEFT) {
            p.vel_x = FixedPoint::raw(-WALK_SPEED);
        } else if input.pressed(BTN_RIGHT) {
            p.vel_x = FixedPoint::raw(WALK_SPEED);
        }
        return;
    }

    // Crouch (only on ground)
    if input.pressed(BTN_CROUCH) && !p.is_airborne {
        p.action = ActionState::Crouch;
        p.action_frame = 0;
        p.vel_x = FixedPoint::ZERO;
        return;
    }

    // Walk
    if !p.is_airborne {
        if input.pressed(BTN_LEFT) {
            p.vel_x = FixedPoint::raw(-WALK_SPEED);
            p.action = if p.facing == -1 {
                ActionState::WalkForward
            } else {
                ActionState::WalkBackward
            };
            p.action_frame = 0;
        } else if input.pressed(BTN_RIGHT) {
            p.vel_x = FixedPoint::raw(WALK_SPEED);
            p.action = if p.facing == 1 {
                ActionState::WalkForward
            } else {
                ActionState::WalkBackward
            };
            p.action_frame = 0;
        } else {
            p.vel_x = FixedPoint::ZERO;
            if p.action != ActionState::Idle {
                p.action = ActionState::Idle;
                p.action_frame = 0;
            }
        }
    }
}

fn start_attack(p: &mut PlayerState, input: &InputFrame, _is_cancel: bool) {
    let action = if p.is_airborne {
        if input.pressed(BTN_SPECIAL) || input.pressed(BTN_HEAVY) {
            ActionState::AirHeavy
        } else {
            ActionState::AirLight
        }
    } else if input.pressed(BTN_CROUCH) {
        if input.pressed(BTN_HEAVY) || input.pressed(BTN_SPECIAL) {
            ActionState::CrouchHeavy
        } else {
            ActionState::CrouchLight
        }
    } else if input.pressed(BTN_SPECIAL) {
        ActionState::SpecialAttack
    } else if input.pressed(BTN_HEAVY) {
        ActionState::HeavyAttack
    } else {
        ActionState::LightAttack
    };

    p.action = action;
    p.action_frame = 0;
    p.vel_x = FixedPoint::ZERO; // stop movement during attack
}

fn apply_gravity(p: &mut PlayerState) {
    if p.is_airborne {
        p.vel_y = FixedPoint::raw(p.vel_y.0 + GRAVITY);
    }
}

fn update_position(p: &mut PlayerState) {
    p.pos_x = FixedPoint::raw((p.pos_x.0 + p.vel_x.0).clamp(STAGE_X_MIN, STAGE_X_MAX));
    p.pos_y = FixedPoint::raw(p.pos_y.0 + p.vel_y.0);

    // Floor collision
    if p.pos_y.0 <= 0 {
        p.pos_y = FixedPoint::ZERO;
        p.vel_y = FixedPoint::ZERO;
        if p.is_airborne {
            p.is_airborne = false;
            if p.action == ActionState::Jump
                || p.action == ActionState::AirLight
                || p.action == ActionState::AirHeavy
            {
                p.action = ActionState::Idle;
                p.action_frame = 0;
            }
        }
    }
}

fn handle_action_completion(p: &mut PlayerState) {
    let max_frames = action_frame_count(p.action);
    // For looping states (Idle, Walk, Crouch, Hitstun, Blockstun), don't auto-transition
    match p.action {
        ActionState::Idle
        | ActionState::WalkForward
        | ActionState::WalkBackward
        | ActionState::Crouch
        | ActionState::Hitstun
        | ActionState::Blockstun
        | ActionState::Jump => {
            // These are either looping or duration-driven externally
        }
        ActionState::Knockdown => {
            if p.action_frame >= max_frames {
                p.action = ActionState::Getup;
                p.action_frame = 0;
            }
        }
        ActionState::Getup => {
            if p.action_frame >= max_frames {
                p.action = ActionState::Idle;
                p.action_frame = 0;
            }
        }
        _ => {
            // Attack animations
            if p.action_frame >= max_frames {
                p.action = ActionState::Idle;
                p.action_frame = 0;
            }
        }
    }
}

fn resolve_combat(s: &mut GameState, p1_is_attacker: bool) {
    let (attacker, defender) = if p1_is_attacker {
        (&s.p1, &s.p2)
    } else {
        (&s.p2, &s.p1)
    };

    if !is_in_attack(attacker.action) {
        return;
    }

    let atk_frame_data = get_frame_data(attacker.action, attacker.action_frame);
    if atk_frame_data.boxes.hitbox_count == 0 {
        return;
    }

    let def_frame_data = get_frame_data(defender.action, defender.action_frame);

    let hit = check_hit(
        &atk_frame_data.boxes,
        &CombatantInfo { pos: attacker.pos_x, pos_y: attacker.pos_y, facing: attacker.facing },
        &def_frame_data.boxes,
        &CombatantInfo { pos: defender.pos_x, pos_y: defender.pos_y, facing: defender.facing },
    );

    if !hit {
        return;
    }

    // Check if defender is blocking
    // Block: holding back relative to attacker, or holding block button
    let defender_ref = if p1_is_attacker { &s.p2 } else { &s.p1 };
    let defender_blocking = defender_ref.action == ActionState::Blockstun
        || defender_ref.action == ActionState::WalkBackward;

    // Don't re-hit if already in hitstun from this combo frame
    // (prevents multi-hit per active window — only hit once per attack)
    let defender_already_hit = if p1_is_attacker {
        s.p2.hit_this_frame
    } else {
        s.p1.hit_this_frame
    };
    if defender_already_hit {
        return;
    }

    if p1_is_attacker {
        if defender_blocking {
            // Block
            s.p2.blockstun_frames = atk_frame_data.blockstun;
            s.p2.action = ActionState::Blockstun;
            s.p2.action_frame = 0;
            s.p2.guard_meter -= atk_frame_data.guard_damage;
            s.p2.vel_x = FixedPoint::raw(atk_frame_data.knockback_x.0 / 2 * s.p1.facing as i32);
        } else {
            // Hit
            let scaled_damage = apply_combo_scaling(atk_frame_data.damage, s.hitstun_scale_num);
            s.p2.health -= scaled_damage;
            let scaled_hitstun =
                apply_hitstun_scaling(atk_frame_data.hitstun, s.hitstun_scale_num);
            s.p2.hitstun_frames = scaled_hitstun;
            s.p2.action = ActionState::Hitstun;
            s.p2.action_frame = 0;
            s.p2.vel_x =
                FixedPoint::raw(atk_frame_data.knockback_x.0 * s.p1.facing as i32);
            s.p2.vel_y = atk_frame_data.knockback_y;
            if atk_frame_data.knockback_y.0 > 0 {
                s.p2.is_airborne = true;
            }
            s.p1.energy = (s.p1.energy + atk_frame_data.energy_gain).min(100);
            s.p1.hit_this_frame = true;
            s.p2.hit_this_frame = true;
            s.combo_hit_count = s.combo_hit_count.saturating_add(1);
            // Hitstun scaling: 15% reduction per hit
            s.hitstun_scale_num = s.hitstun_scale_num.saturating_sub(15).max(20);
            // Techable knockdown after 7 hits
            if s.combo_hit_count >= KNOCKDOWN_THRESHOLD {
                s.p2.action = ActionState::Knockdown;
                s.p2.action_frame = 0;
                s.p2.hitstun_frames = 0;
                s.p2.juggle_count = 0;
                s.combo_hit_count = 0;
                s.hitstun_scale_num = 100;
            }
        }
    } else {
        if defender_blocking {
            s.p1.blockstun_frames = atk_frame_data.blockstun;
            s.p1.action = ActionState::Blockstun;
            s.p1.action_frame = 0;
            s.p1.guard_meter -= atk_frame_data.guard_damage;
            s.p1.vel_x = FixedPoint::raw(atk_frame_data.knockback_x.0 / 2 * s.p2.facing as i32);
        } else {
            let scaled_damage = apply_combo_scaling(atk_frame_data.damage, s.hitstun_scale_num);
            s.p1.health -= scaled_damage;
            let scaled_hitstun =
                apply_hitstun_scaling(atk_frame_data.hitstun, s.hitstun_scale_num);
            s.p1.hitstun_frames = scaled_hitstun;
            s.p1.action = ActionState::Hitstun;
            s.p1.action_frame = 0;
            s.p1.vel_x =
                FixedPoint::raw(atk_frame_data.knockback_x.0 * s.p2.facing as i32);
            s.p1.vel_y = atk_frame_data.knockback_y;
            if atk_frame_data.knockback_y.0 > 0 {
                s.p1.is_airborne = true;
            }
            s.p2.energy = (s.p2.energy + atk_frame_data.energy_gain).min(100);
            s.p2.hit_this_frame = true;
            s.p1.hit_this_frame = true;
            s.combo_hit_count = s.combo_hit_count.saturating_add(1);
            s.hitstun_scale_num = s.hitstun_scale_num.saturating_sub(15).max(20);
            if s.combo_hit_count >= KNOCKDOWN_THRESHOLD {
                s.p1.action = ActionState::Knockdown;
                s.p1.action_frame = 0;
                s.p1.hitstun_frames = 0;
                s.p1.juggle_count = 0;
                s.combo_hit_count = 0;
                s.hitstun_scale_num = 100;
            }
        }
    }
}

/// Apply combo damage scaling: damage * scale_percent / 100
fn apply_combo_scaling(base_damage: i32, scale_percent: u8) -> i32 {
    (base_damage * scale_percent as i32) / 100
}

/// Apply hitstun scaling: hitstun * scale_percent / 100, minimum 3 frames
fn apply_hitstun_scaling(base_hitstun: u8, scale_percent: u8) -> u8 {
    let scaled = (base_hitstun as u32 * scale_percent as u32) / 100;
    scaled.max(3) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{InputFrame, BTN_BLOCK};

    fn empty_input(frame: u32) -> InputFrame {
        InputFrame::new(frame, 0)
    }

    #[test]
    fn tick_advances_frame() {
        let s = GameState::initial();
        let s2 = tick(&s, empty_input(0), empty_input(0));
        assert_eq!(s2.frame_number, 1);
    }

    #[test]
    fn tick_decrements_timer() {
        let s = GameState::initial();
        let s2 = tick(&s, empty_input(0), empty_input(0));
        assert_eq!(s2.timer_frames, GameState::TIMER_INITIAL - 1);
    }

    #[test]
    fn walk_right() {
        let s = GameState::initial();
        let input = InputFrame::new(0, BTN_RIGHT);
        let s2 = tick(&s, input, empty_input(0));
        assert!(s2.p1.pos_x > s.p1.pos_x);
    }

    #[test]
    fn jump_and_land() {
        let s = GameState::initial();
        let jump_input = InputFrame::new(0, BTN_JUMP);
        let empty = empty_input(0);

        let mut state = tick(&s, jump_input, empty);
        assert!(state.p1.is_airborne);

        // Run until landing
        for _ in 0..100 {
            state = tick(&state, empty, empty);
            if !state.p1.is_airborne {
                break;
            }
        }
        assert!(!state.p1.is_airborne);
        assert_eq!(state.p1.pos_y.0, 0);
    }

    #[test]
    fn light_attack_deals_damage() {
        // Set up players close together
        let mut s = GameState::initial();
        s.p1.pos_x = FixedPoint::from_pixels(100);
        s.p2.pos_x = FixedPoint::from_pixels(140); // close enough for hitbox

        let light = InputFrame::new(0, BTN_LIGHT);
        let empty = empty_input(0);

        // Run through the attack animation
        let mut state = s;
        for _ in 0..15 {
            state = tick(&state, light, empty);
        }
        // p2 should have taken damage
        assert!(state.p2.health < 1000);
    }

    #[test]
    fn combo_scaling_reduces_damage() {
        assert_eq!(apply_combo_scaling(100, 100), 100);
        assert_eq!(apply_combo_scaling(100, 85), 85);
        assert_eq!(apply_combo_scaling(100, 70), 70);
    }

    #[test]
    fn determinism_test() {
        // Build a 300-frame scripted input sequence
        let mut inputs_p1 = [InputFrame::new(0, 0); 300];
        let mut inputs_p2 = [InputFrame::new(0, 0); 300];

        for i in 0..300u32 {
            let frame = i;
            // P1: alternating walk right, light attack, walk left, heavy attack
            let p1_buttons = match i % 20 {
                0..=4 => BTN_RIGHT,
                5..=7 => BTN_LIGHT,
                8..=12 => BTN_LEFT,
                13..=15 => BTN_HEAVY,
                16..=17 => BTN_JUMP,
                _ => 0,
            };
            // P2: alternating block, crouch, light, walk
            let p2_buttons = match i % 16 {
                0..=3 => BTN_LEFT,
                4..=6 => BTN_BLOCK,
                7..=9 => BTN_CROUCH,
                10..=12 => BTN_LIGHT,
                _ => BTN_RIGHT,
            };
            inputs_p1[i as usize] = InputFrame::new(frame, p1_buttons);
            inputs_p2[i as usize] = InputFrame::new(frame, p2_buttons);
        }

        // Run 1
        let mut state1 = GameState::initial();
        for i in 0..300 {
            state1 = tick(&state1, inputs_p1[i], inputs_p2[i]);
        }

        // Run 2 (identical)
        let mut state2 = GameState::initial();
        for i in 0..300 {
            state2 = tick(&state2, inputs_p1[i], inputs_p2[i]);
        }

        assert_eq!(
            state1.hash_state(),
            state2.hash_state(),
            "Determinism check failed: two identical runs produced different states"
        );
        // Also check individual fields
        assert_eq!(state1.p1.pos_x, state2.p1.pos_x);
        assert_eq!(state1.p2.pos_x, state2.p2.pos_x);
        assert_eq!(state1.p1.health, state2.p1.health);
        assert_eq!(state1.p2.health, state2.p2.health);
        assert_eq!(state1.frame_number, state2.frame_number);
        assert_eq!(state1.timer_frames, state2.timer_frames);
    }

    #[test]
    fn timeout_awards_round() {
        let mut s = GameState::initial();
        s.timer_frames = 1; // about to time out
        s.p1.health = 800;
        s.p2.health = 600;
        let s2 = tick(&s, empty_input(0), empty_input(0));
        assert_eq!(s2.p1_rounds_won, 1);
        assert_eq!(s2.p2_rounds_won, 0);
    }

    #[test]
    fn knockdown_after_7_hits() {
        let mut s = GameState::initial();
        s.combo_hit_count = 6;
        s.p2.action = ActionState::Hitstun;
        s.p2.hitstun_frames = 10;
        // Place players close
        s.p1.pos_x = FixedPoint::from_pixels(100);
        s.p2.pos_x = FixedPoint::from_pixels(140);

        let light = InputFrame::new(0, BTN_LIGHT);
        let empty = empty_input(0);

        // Run until hit lands
        let mut state = s;
        for _ in 0..15 {
            state = tick(&state, light, empty);
            if state.p2.action == ActionState::Knockdown {
                break;
            }
        }
        // After 7th hit, should be in knockdown
        assert_eq!(state.p2.action, ActionState::Knockdown);
        assert_eq!(state.combo_hit_count, 0); // reset after knockdown
    }

    #[test]
    fn stage_bounds() {
        let mut s = GameState::initial();
        s.p1.pos_x = FixedPoint::raw(0);
        s.p1.vel_x = FixedPoint::raw(-10_000);
        let empty = empty_input(0);
        let s2 = tick(&s, empty, empty);
        assert_eq!(s2.p1.pos_x.0, 0); // clamped to min
    }

    #[test]
    fn auto_facing() {
        let mut s = GameState::initial();
        // Swap positions
        s.p1.pos_x = FixedPoint::from_pixels(900);
        s.p2.pos_x = FixedPoint::from_pixels(300);
        let empty = empty_input(0);
        let s2 = tick(&s, empty, empty);
        assert_eq!(s2.p1.facing, -1); // p1 now faces left
        assert_eq!(s2.p2.facing, 1);  // p2 now faces right
    }
}
