use crate::collision::{Aabb, AnimationFrameData, HitboxSet};
use crate::fixed_point::FixedPoint;
use crate::game_state::ActionState;

/// Get the total frame count for an action.
pub fn action_frame_count(action: ActionState) -> u8 {
    match action {
        ActionState::Idle => 1,        // looping single frame
        ActionState::WalkForward => 1,  // looping
        ActionState::WalkBackward => 1, // looping
        ActionState::Jump => 40,        // jump arc
        ActionState::Crouch => 1,       // looping while held
        ActionState::LightAttack => 12, // 3 startup + 3 active + 6 recovery
        ActionState::HeavyAttack => 20, // 5 startup + 4 active + 11 recovery
        ActionState::SpecialAttack => 25,
        ActionState::CrouchLight => 10,
        ActionState::CrouchHeavy => 18,
        ActionState::AirLight => 10,
        ActionState::AirHeavy => 16,
        ActionState::Hitstun => 1,   // frame count driven by hitstun_frames
        ActionState::Blockstun => 1, // frame count driven by blockstun_frames
        ActionState::Knockdown => 30,
        ActionState::Getup => 15,
    }
}

/// Default standing hurtbox (local space, fixed-point raw values).
fn standing_hurtbox() -> HitboxSet {
    HitboxSet {
        hitboxes: [Aabb::NONE; 4],
        hitbox_count: 0,
        hurtboxes: [
            Aabb::new(-20_000, 0, 20_000, 90_000),
            Aabb::NONE,
            Aabb::NONE,
            Aabb::NONE,
        ],
        hurtbox_count: 1,
    }
}

/// Default crouching hurtbox.
fn crouching_hurtbox() -> HitboxSet {
    HitboxSet {
        hitboxes: [Aabb::NONE; 4],
        hitbox_count: 0,
        hurtboxes: [
            Aabb::new(-20_000, 0, 20_000, 50_000),
            Aabb::NONE,
            Aabb::NONE,
            Aabb::NONE,
        ],
        hurtbox_count: 1,
    }
}

/// Get frame data for a given action and frame number.
pub fn get_frame_data(action: ActionState, frame: u8) -> AnimationFrameData {
    match action {
        ActionState::LightAttack => light_attack_frame(frame),
        ActionState::HeavyAttack => heavy_attack_frame(frame),
        ActionState::SpecialAttack => special_attack_frame(frame),
        ActionState::CrouchLight => crouch_light_frame(frame),
        ActionState::CrouchHeavy => crouch_heavy_frame(frame),
        ActionState::AirLight => air_light_frame(frame),
        ActionState::AirHeavy => air_heavy_frame(frame),
        ActionState::Crouch => AnimationFrameData {
            boxes: crouching_hurtbox(),
            ..AnimationFrameData::default()
        },
        _ => AnimationFrameData {
            boxes: standing_hurtbox(),
            ..AnimationFrameData::default()
        },
    }
}

// Light attack: 3 startup, 3 active (frames 3-5), 6 recovery
fn light_attack_frame(frame: u8) -> AnimationFrameData {
    let mut data = AnimationFrameData {
        boxes: standing_hurtbox(),
        damage: 60,
        hitstun: 15,
        blockstun: 8,
        is_cancellable: (3..=5).contains(&frame),
        energy_gain: 5,
        guard_damage: 15,
        knockback_x: FixedPoint::raw(3_000),
        knockback_y: FixedPoint::ZERO,
    };
    if (3..=5).contains(&frame) {
        data.boxes.hitboxes[0] = Aabb::new(15_000, 40_000, 55_000, 65_000);
        data.boxes.hitbox_count = 1;
    }
    data
}

// Heavy attack: 5 startup, 4 active (frames 5-8), 11 recovery
fn heavy_attack_frame(frame: u8) -> AnimationFrameData {
    let mut data = AnimationFrameData {
        boxes: standing_hurtbox(),
        damage: 100,
        hitstun: 22,
        blockstun: 12,
        is_cancellable: false,
        energy_gain: 5,
        guard_damage: 25,
        knockback_x: FixedPoint::raw(6_000),
        knockback_y: FixedPoint::ZERO,
    };
    if (5..=8).contains(&frame) {
        data.boxes.hitboxes[0] = Aabb::new(10_000, 30_000, 65_000, 75_000);
        data.boxes.hitbox_count = 1;
    }
    data
}

// Special attack: 7 startup, 5 active (7-11), 13 recovery
fn special_attack_frame(frame: u8) -> AnimationFrameData {
    let mut data = AnimationFrameData {
        boxes: standing_hurtbox(),
        damage: 120,
        hitstun: 25,
        blockstun: 14,
        is_cancellable: false,
        energy_gain: 10,
        guard_damage: 30,
        knockback_x: FixedPoint::raw(8_000),
        knockback_y: FixedPoint::raw(4_000),
    };
    if (7..=11).contains(&frame) {
        data.boxes.hitboxes[0] = Aabb::new(5_000, 20_000, 70_000, 80_000);
        data.boxes.hitbox_count = 1;
    }
    data
}

// Crouch light: 2 startup, 3 active (2-4), 5 recovery
fn crouch_light_frame(frame: u8) -> AnimationFrameData {
    let mut data = AnimationFrameData {
        boxes: crouching_hurtbox(),
        damage: 40,
        hitstun: 12,
        blockstun: 6,
        is_cancellable: (2..=4).contains(&frame),
        energy_gain: 3,
        guard_damage: 10,
        knockback_x: FixedPoint::raw(2_000),
        knockback_y: FixedPoint::ZERO,
    };
    if (2..=4).contains(&frame) {
        data.boxes.hitboxes[0] = Aabb::new(10_000, 5_000, 55_000, 35_000);
        data.boxes.hitbox_count = 1;
    }
    data
}

// Crouch heavy: 4 startup, 4 active (4-7), 10 recovery
fn crouch_heavy_frame(frame: u8) -> AnimationFrameData {
    let mut data = AnimationFrameData {
        boxes: crouching_hurtbox(),
        damage: 80,
        hitstun: 18,
        blockstun: 10,
        is_cancellable: false,
        energy_gain: 5,
        guard_damage: 20,
        knockback_x: FixedPoint::raw(5_000),
        knockback_y: FixedPoint::raw(3_000),
    };
    if (4..=7).contains(&frame) {
        data.boxes.hitboxes[0] = Aabb::new(5_000, 0, 65_000, 40_000);
        data.boxes.hitbox_count = 1;
    }
    data
}

// Air light: 2 startup, 3 active (2-4), 5 recovery
fn air_light_frame(frame: u8) -> AnimationFrameData {
    let mut data = AnimationFrameData {
        boxes: standing_hurtbox(),
        damage: 50,
        hitstun: 13,
        blockstun: 7,
        is_cancellable: (2..=4).contains(&frame),
        energy_gain: 4,
        guard_damage: 12,
        knockback_x: FixedPoint::raw(2_500),
        knockback_y: FixedPoint::raw(-2_000),
    };
    if (2..=4).contains(&frame) {
        data.boxes.hitboxes[0] = Aabb::new(10_000, 50_000, 50_000, 80_000);
        data.boxes.hitbox_count = 1;
    }
    data
}

// Air heavy: 4 startup, 4 active (4-7), 8 recovery
fn air_heavy_frame(frame: u8) -> AnimationFrameData {
    let mut data = AnimationFrameData {
        boxes: standing_hurtbox(),
        damage: 90,
        hitstun: 20,
        blockstun: 11,
        is_cancellable: false,
        energy_gain: 5,
        guard_damage: 22,
        knockback_x: FixedPoint::raw(5_000),
        knockback_y: FixedPoint::raw(-4_000),
    };
    if (4..=7).contains(&frame) {
        data.boxes.hitboxes[0] = Aabb::new(5_000, 40_000, 60_000, 85_000);
        data.boxes.hitbox_count = 1;
    }
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_attack_active_frames() {
        for f in 0..12 {
            let data = get_frame_data(ActionState::LightAttack, f);
            if (3..=5).contains(&f) {
                assert_eq!(data.boxes.hitbox_count, 1, "frame {} should be active", f);
            } else {
                assert_eq!(
                    data.boxes.hitbox_count, 0,
                    "frame {} should be inactive",
                    f
                );
            }
        }
    }

    #[test]
    fn heavy_attack_damage() {
        let data = get_frame_data(ActionState::HeavyAttack, 6);
        assert_eq!(data.damage, 100);
        assert_eq!(data.hitstun, 22);
    }
}
