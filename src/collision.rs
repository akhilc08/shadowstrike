use crate::fixed_point::FixedPoint;
use serde::{Deserialize, Serialize};

/// Axis-aligned bounding box in fixed-point coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Aabb {
    pub x_min: FixedPoint,
    pub y_min: FixedPoint,
    pub x_max: FixedPoint,
    pub y_max: FixedPoint,
}

impl Aabb {
    pub const NONE: Self = Self {
        x_min: FixedPoint::ZERO,
        y_min: FixedPoint::ZERO,
        x_max: FixedPoint::ZERO,
        y_max: FixedPoint::ZERO,
    };

    pub const fn new(x_min: i32, y_min: i32, x_max: i32, y_max: i32) -> Self {
        Self {
            x_min: FixedPoint(x_min),
            y_min: FixedPoint(y_min),
            x_max: FixedPoint(x_max),
            y_max: FixedPoint(y_max),
        }
    }

    /// Check if two AABBs overlap.
    #[inline]
    pub fn overlaps(&self, other: &Aabb) -> bool {
        self.x_min < other.x_max
            && self.x_max > other.x_min
            && self.y_min < other.y_max
            && self.y_max > other.y_min
    }

    /// Translate to world space given character position and facing direction.
    pub fn to_world(&self, pos_x: FixedPoint, pos_y: FixedPoint, facing: i8) -> Self {
        if facing >= 0 {
            Self {
                x_min: FixedPoint(pos_x.0 + self.x_min.0),
                y_min: FixedPoint(pos_y.0 + self.y_min.0),
                x_max: FixedPoint(pos_x.0 + self.x_max.0),
                y_max: FixedPoint(pos_y.0 + self.y_max.0),
            }
        } else {
            // Mirror on x-axis
            Self {
                x_min: FixedPoint(pos_x.0 - self.x_max.0),
                y_min: FixedPoint(pos_y.0 + self.y_min.0),
                x_max: FixedPoint(pos_x.0 - self.x_min.0),
                y_max: FixedPoint(pos_y.0 + self.y_max.0),
            }
        }
    }
}

/// Up to 4 hitboxes + 4 hurtboxes per animation frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct HitboxSet {
    pub hitboxes: [Aabb; 4],
    pub hitbox_count: u8,
    pub hurtboxes: [Aabb; 4],
    pub hurtbox_count: u8,
}

/// Per-frame animation data including collision boxes.
#[derive(Clone, Copy, Debug, Default)]
pub struct AnimationFrameData {
    pub boxes: HitboxSet,
    pub damage: i32,
    pub hitstun: u8,
    pub blockstun: u8,
    pub is_cancellable: bool,
    pub energy_gain: i32,
    pub guard_damage: i32,
    pub knockback_x: FixedPoint,
    pub knockback_y: FixedPoint,
}

/// Attacker and defender spatial info for hit checking.
pub struct CombatantInfo {
    pub pos: FixedPoint,
    pub pos_y: FixedPoint,
    pub facing: i8,
}

/// Check if any hitbox in the attacker's set overlaps any hurtbox in the defender's set.
pub fn check_hit(
    attacker_boxes: &HitboxSet,
    attacker: &CombatantInfo,
    defender_boxes: &HitboxSet,
    defender: &CombatantInfo,
) -> bool {
    for i in 0..attacker_boxes.hitbox_count as usize {
        let hit = attacker_boxes.hitboxes[i].to_world(attacker.pos, attacker.pos_y, attacker.facing);
        for j in 0..defender_boxes.hurtbox_count as usize {
            let hurt =
                defender_boxes.hurtboxes[j].to_world(defender.pos, defender.pos_y, defender.facing);
            if hit.overlaps(&hurt) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aabb_overlap() {
        let a = Aabb::new(0, 0, 10_000, 10_000);
        let b = Aabb::new(5_000, 5_000, 15_000, 15_000);
        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));

        let c = Aabb::new(20_000, 0, 30_000, 10_000);
        assert!(!a.overlaps(&c));
    }

    #[test]
    fn aabb_no_overlap_edge() {
        // Touching edges should NOT overlap (strict inequality).
        let a = Aabb::new(0, 0, 10_000, 10_000);
        let b = Aabb::new(10_000, 0, 20_000, 10_000);
        assert!(!a.overlaps(&b));
    }

    #[test]
    fn world_transform_facing_right() {
        let local = Aabb::new(5_000, 0, 20_000, 30_000);
        let world = local.to_world(FixedPoint::from_pixels(100), FixedPoint::ZERO, 1);
        assert_eq!(world.x_min.0, 105_000);
        assert_eq!(world.x_max.0, 120_000);
    }

    #[test]
    fn world_transform_facing_left() {
        let local = Aabb::new(5_000, 0, 20_000, 30_000);
        let world = local.to_world(FixedPoint::from_pixels(100), FixedPoint::ZERO, -1);
        assert_eq!(world.x_min.0, 80_000);
        assert_eq!(world.x_max.0, 95_000);
    }
}
