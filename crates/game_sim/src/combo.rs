use crate::fixed::FixedPoint;
use crate::player::PlayerAction;

#[derive(Debug, Clone, Copy)]
pub struct ComboState {
    pub hit_count: i32,
    pub hitstun_scale: FixedPoint,
}

impl Default for ComboState {
    fn default() -> Self {
        Self::new()
    }
}

impl ComboState {
    pub fn new() -> Self {
        ComboState {
            hit_count: 0,
            hitstun_scale: FixedPoint::ONE,
        }
    }

    /// Register a hit and return the current hitstun scale factor.
    /// Hitstun scales down as combo grows to prevent infinites.
    pub fn register_hit(&mut self) -> FixedPoint {
        self.hit_count += 1;
        // Scale: 1.0 for first hit, decreasing by 5% per subsequent hit, min 0.3
        let reduction = FixedPoint(50) * FixedPoint::from_int(self.hit_count - 1);
        let scale = FixedPoint::ONE - FixedPoint(reduction.0 / 1000);
        let min_scale = FixedPoint(300); // 0.3
        self.hitstun_scale = if scale > min_scale { scale } else { min_scale };
        self.hitstun_scale
    }

    pub fn reset(&mut self) {
        self.hit_count = 0;
        self.hitstun_scale = FixedPoint::ONE;
    }

    /// Check if the current action can cancel into the next action (chain combo).
    pub fn can_cancel(&self, current_action: &PlayerAction, next_action: &PlayerAction) -> bool {
        match (current_action, next_action) {
            // Light chain: L1 -> L2 -> L3
            (PlayerAction::LightAttack1, PlayerAction::LightAttack2) => true,
            (PlayerAction::LightAttack2, PlayerAction::LightAttack3) => true,
            // Light into heavy
            (PlayerAction::LightAttack1, PlayerAction::HeavyAttack) => true,
            (PlayerAction::LightAttack2, PlayerAction::HeavyAttack) => true,
            (PlayerAction::LightAttack3, PlayerAction::HeavyAttack) => true,
            // Any grounded attack into special
            (
                PlayerAction::LightAttack1
                | PlayerAction::LightAttack2
                | PlayerAction::LightAttack3
                | PlayerAction::HeavyAttack,
                PlayerAction::Uppercut | PlayerAction::Fireball | PlayerAction::DashStrike,
            ) => true,
            _ => false,
        }
    }
}
