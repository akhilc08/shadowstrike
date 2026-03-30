use crate::fixed_point::FixedPoint;
use serde::{Deserialize, Serialize};

/// All possible action states for a player.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionState {
    #[default]
    Idle,
    WalkForward,
    WalkBackward,
    Jump,
    Crouch,
    LightAttack,
    HeavyAttack,
    SpecialAttack,
    CrouchLight,
    CrouchHeavy,
    AirLight,
    AirHeavy,
    Hitstun,
    Blockstun,
    Knockdown,
    Getup,
}

/// Round phase.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoundPhase {
    Intro,
    #[default]
    Fighting,
    KO,
    RoundEnd,
    MatchEnd,
}

/// Per-player state. No heap pointers, fully copyable.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct PlayerState {
    pub pos_x: FixedPoint,
    pub pos_y: FixedPoint,
    pub vel_x: FixedPoint,
    pub vel_y: FixedPoint,
    pub facing: i8,
    pub health: i32,
    pub energy: i32,
    pub guard_meter: i32,
    pub action: ActionState,
    pub action_frame: u8,
    pub hitstun_frames: u8,
    pub blockstun_frames: u8,
    pub is_airborne: bool,
    pub juggle_count: u8,
    pub hit_this_frame: bool,
}

/// Flat game state — no heap pointers, fully copyable.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct GameState {
    pub p1: PlayerState,
    pub p2: PlayerState,
    pub combo_hit_count: u8,
    pub hitstun_scale_num: u8,
    pub timer_frames: u32,
    pub round_number: u8,
    pub p1_rounds_won: u8,
    pub p2_rounds_won: u8,
    pub round_phase: RoundPhase,
    pub frame_number: u64,
}

impl GameState {
    /// 90 seconds at 60 fps.
    pub const TIMER_INITIAL: u32 = 90 * 60;

    pub fn initial() -> Self {
        Self {
            p1: PlayerState {
                pos_x: FixedPoint::from_pixels(300),
                pos_y: FixedPoint::ZERO,
                facing: 1,
                health: 1000,
                energy: 0,
                guard_meter: 100,
                ..PlayerState::default()
            },
            p2: PlayerState {
                pos_x: FixedPoint::from_pixels(980),
                pos_y: FixedPoint::ZERO,
                facing: -1,
                health: 1000,
                energy: 0,
                guard_meter: 100,
                ..PlayerState::default()
            },
            combo_hit_count: 0,
            hitstun_scale_num: 100,
            timer_frames: Self::TIMER_INITIAL,
            round_number: 1,
            p1_rounds_won: 0,
            p2_rounds_won: 0,
            round_phase: RoundPhase::Fighting,
            frame_number: 0,
        }
    }

    /// Simple XOR hash of all meaningful fields for determinism checks.
    pub fn hash_state(&self) -> u64 {
        let mut h: u64 = 0;
        h ^= self.p1.pos_x.0 as u64;
        h ^= (self.p1.pos_y.0 as u64).wrapping_shl(3);
        h ^= (self.p1.vel_x.0 as u64).wrapping_shl(5);
        h ^= (self.p1.vel_y.0 as u64).wrapping_shl(7);
        h ^= (self.p1.health as u64).wrapping_shl(11);
        h ^= (self.p1.energy as u64).wrapping_shl(13);
        h ^= (self.p1.guard_meter as u64).wrapping_shl(17);
        h ^= (self.p1.action_frame as u64).wrapping_shl(19);
        h ^= (self.p1.hitstun_frames as u64).wrapping_shl(23);
        h ^= (self.p1.facing as u64).wrapping_shl(29);
        h ^= (self.p1.juggle_count as u64).wrapping_shl(31);

        h ^= (self.p2.pos_x.0 as u64).wrapping_shl(1);
        h ^= (self.p2.pos_y.0 as u64).wrapping_shl(4);
        h ^= (self.p2.vel_x.0 as u64).wrapping_shl(6);
        h ^= (self.p2.vel_y.0 as u64).wrapping_shl(8);
        h ^= (self.p2.health as u64).wrapping_shl(12);
        h ^= (self.p2.energy as u64).wrapping_shl(14);
        h ^= (self.p2.guard_meter as u64).wrapping_shl(18);
        h ^= (self.p2.action_frame as u64).wrapping_shl(20);
        h ^= (self.p2.hitstun_frames as u64).wrapping_shl(24);
        h ^= (self.p2.facing as u64).wrapping_shl(30);
        h ^= (self.p2.juggle_count as u64).wrapping_shl(32);

        h ^= self.combo_hit_count as u64;
        h ^= (self.hitstun_scale_num as u64).wrapping_shl(2);
        h ^= (self.timer_frames as u64).wrapping_shl(9);
        h ^= (self.round_number as u64).wrapping_shl(15);
        h ^= (self.p1_rounds_won as u64).wrapping_shl(21);
        h ^= (self.p2_rounds_won as u64).wrapping_shl(25);
        h ^= self.frame_number.wrapping_shl(33);
        h
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_state_is_copy() {
        let a = GameState::initial();
        let b = a; // Copy
        assert_eq!(a.p1.health, b.p1.health);
    }

    #[test]
    fn hash_deterministic() {
        let a = GameState::initial();
        let b = GameState::initial();
        assert_eq!(a.hash_state(), b.hash_state());
    }

    #[test]
    fn game_state_size() {
        let size = core::mem::size_of::<GameState>();
        assert!(
            size <= 512,
            "GameState is {} bytes, must be <= 512",
            size
        );
    }
}
