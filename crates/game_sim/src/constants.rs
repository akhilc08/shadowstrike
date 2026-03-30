use crate::fixed::FixedPoint;

pub const STAGE_WIDTH: i32 = 1200;
pub const STAGE_HEIGHT: i32 = 600;
pub const GROUND_Y: i32 = 500;

/// 4 pixels/frame
pub const PLAYER_SPEED: FixedPoint = FixedPoint(4000);
/// -12 pixels/frame (upward)
pub const JUMP_VELOCITY: FixedPoint = FixedPoint(-12000);
/// 0.5 pixels/frame^2
pub const GRAVITY: FixedPoint = FixedPoint(500);

pub const MAX_HEALTH: i32 = 1000;
pub const MAX_ENERGY: i32 = 100;
pub const ROUND_TIME_SECONDS: i32 = 90;
pub const TICKS_PER_SECOND: i32 = 60;
pub const BUFFER_SIZE: usize = 8;
pub const INPUT_BUFFER_SIZE: usize = 120;
