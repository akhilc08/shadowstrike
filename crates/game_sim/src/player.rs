use crate::collision::AABB;
use crate::constants::*;
use crate::fixed::FixedPoint;
use crate::input::Input;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerAction {
    Idle,
    WalkForward,
    WalkBack,
    Jump,
    Crouch,
    LightAttack1,
    LightAttack2,
    LightAttack3,
    HeavyAttack,
    Uppercut,
    AerialAttack,
    Block,
    Fireball,
    DashStrike,
    Hitstun { frames_remaining: i32 },
    Blockstun { frames_remaining: i32 },
    Knockdown { frames_remaining: i32 },
    Getup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Element {
    Fire,
    Lightning,
    DarkMagic,
    Ice,
}

#[derive(Debug, Clone, Copy)]
pub struct PlayerState {
    pub x: FixedPoint,
    pub y: FixedPoint,
    pub vx: FixedPoint,
    pub vy: FixedPoint,
    pub facing: i8,
    pub health: i32,
    pub energy: i32,
    pub guard_meter: i32,
    pub action: PlayerAction,
    pub action_frame: i32,
    pub is_grounded: bool,
    pub element: Element,
}

/// Duration (in frames) of each attack action.
fn action_duration(action: &PlayerAction) -> i32 {
    match action {
        PlayerAction::LightAttack1 => 12,
        PlayerAction::LightAttack2 => 14,
        PlayerAction::LightAttack3 => 18,
        PlayerAction::HeavyAttack => 24,
        PlayerAction::Uppercut => 20,
        PlayerAction::AerialAttack => 16,
        PlayerAction::Fireball => 22,
        PlayerAction::DashStrike => 18,
        PlayerAction::Getup => 20,
        _ => 0, // no fixed duration
    }
}

/// Damage dealt by an attack.
/// Tuned so full L1-L2-L3 chain = 85 dmg (~7% of 1200 HP), rewarding full combos.
/// Heavy/specials deal more but are slower and riskier.
pub fn attack_damage(action: &PlayerAction) -> i32 {
    match action {
        PlayerAction::LightAttack1 => 25,  // fast jab, low commitment
        PlayerAction::LightAttack2 => 30,  // chain follow-up
        PlayerAction::LightAttack3 => 40,  // chain finisher, slight knockback
        PlayerAction::HeavyAttack => 70,   // slow wind-up, causes knockdown
        PlayerAction::Uppercut => 60,      // anti-air launcher
        PlayerAction::AerialAttack => 50,  // air-to-air or jump-in
        PlayerAction::DashStrike => 55,    // gap closer, costs meter
        _ => 0,
    }
}

/// Base hitstun frames for an attack.
/// Light hitstun just long enough to chain into next light on hit.
/// Heavy/special hitstun gives advantage but doesn't guarantee follow-up without reads.
pub fn attack_hitstun(action: &PlayerAction) -> i32 {
    match action {
        PlayerAction::LightAttack1 => 10,  // tight link window to L2
        PlayerAction::LightAttack2 => 12,  // chains into L3 on hit
        PlayerAction::LightAttack3 => 14,  // slight advantage on hit, combo ender
        PlayerAction::HeavyAttack => 20,   // big advantage, can follow up
        PlayerAction::Uppercut => 18,      // launcher hitstun
        PlayerAction::AerialAttack => 15,  // air reset
        PlayerAction::DashStrike => 16,    // dash-in advantage
        _ => 0,
    }
}

/// Active frames window: (start_frame, end_frame) where hitbox is active.
fn active_frames(action: &PlayerAction) -> (i32, i32) {
    match action {
        PlayerAction::LightAttack1 => (3, 6),
        PlayerAction::LightAttack2 => (4, 8),
        PlayerAction::LightAttack3 => (5, 10),
        PlayerAction::HeavyAttack => (8, 14),
        PlayerAction::Uppercut => (5, 12),
        PlayerAction::AerialAttack => (4, 10),
        PlayerAction::DashStrike => (6, 12),
        _ => (0, 0),
    }
}

fn is_attack(action: &PlayerAction) -> bool {
    matches!(
        action,
        PlayerAction::LightAttack1
            | PlayerAction::LightAttack2
            | PlayerAction::LightAttack3
            | PlayerAction::HeavyAttack
            | PlayerAction::Uppercut
            | PlayerAction::AerialAttack
            | PlayerAction::DashStrike
            | PlayerAction::Fireball
    )
}

/// Energy cost for special moves — 30 energy limits spam to ~3 specials from full meter.
pub fn special_energy_cost(action: &PlayerAction) -> i32 {
    match action {
        PlayerAction::Fireball => SPECIAL_ENERGY_COST,
        PlayerAction::DashStrike => SPECIAL_ENERGY_COST,
        _ => 0,
    }
}

fn is_stunned(action: &PlayerAction) -> bool {
    matches!(
        action,
        PlayerAction::Hitstun { .. }
            | PlayerAction::Blockstun { .. }
            | PlayerAction::Knockdown { .. }
            | PlayerAction::Getup
    )
}

impl PlayerState {
    pub fn new(x: FixedPoint, element: Element) -> Self {
        PlayerState {
            x,
            y: FixedPoint::from_int(GROUND_Y),
            vx: FixedPoint::ZERO,
            vy: FixedPoint::ZERO,
            facing: 1,
            health: MAX_HEALTH,
            energy: MAX_ENERGY,
            guard_meter: 100,
            action: PlayerAction::Idle,
            action_frame: 0,
            is_grounded: true,
            element,
        }
    }

    pub fn apply_input(&mut self, input: Input, opponent_x: FixedPoint) {
        // Can't act during stun/knockdown
        if is_stunned(&self.action) {
            self.advance_stun();
            return;
        }

        // If in the middle of an attack, let it finish
        if is_attack(&self.action) {
            let dur = action_duration(&self.action);
            if self.action_frame < dur {
                self.action_frame += 1;
                return;
            }
            // Attack finished
            self.action = PlayerAction::Idle;
            self.action_frame = 0;
        }

        self.auto_face_opponent(opponent_x);

        // Block
        if input.is_block() && self.is_grounded {
            self.action = PlayerAction::Block;
            self.action_frame = 0;
            self.vx = FixedPoint::ZERO;
            return;
        }

        // Special move combinations (check before basic attacks)
        if input.is_special() && self.is_grounded {
            let forward = (input.is_right() && self.facing > 0) || (input.is_left() && self.facing < 0);
            if input.is_down() && self.energy >= special_energy_cost(&PlayerAction::Fireball) {
                // Down + Special = Fireball
                self.energy -= special_energy_cost(&PlayerAction::Fireball);
                self.action = PlayerAction::Fireball;
                self.action_frame = 0;
                self.vx = FixedPoint::ZERO;
                return;
            } else if forward && self.energy >= special_energy_cost(&PlayerAction::DashStrike) {
                // Forward + Special = Dash Strike
                self.energy -= special_energy_cost(&PlayerAction::DashStrike);
                self.action = PlayerAction::DashStrike;
                self.action_frame = 0;
                self.vx = FixedPoint(PLAYER_SPEED.0 * self.facing as i32 * DASH_STRIKE_SPEED_MULT);
                return;
            } else {
                // Special alone = Uppercut
                self.action = PlayerAction::Uppercut;
                self.action_frame = 0;
                self.vx = FixedPoint::ZERO;
                return;
            }
        }

        // Attacks
        if input.is_light() && self.is_grounded {
            self.action = PlayerAction::LightAttack1;
            self.action_frame = 0;
            self.vx = FixedPoint::ZERO;
            return;
        }
        if input.is_heavy() && self.is_grounded {
            self.action = PlayerAction::HeavyAttack;
            self.action_frame = 0;
            self.vx = FixedPoint::ZERO;
            return;
        }
        if input.is_light() && !self.is_grounded {
            self.action = PlayerAction::AerialAttack;
            self.action_frame = 0;
            return;
        }

        // Jump
        if input.is_up() && self.is_grounded {
            self.action = PlayerAction::Jump;
            self.action_frame = 0;
            self.vy = JUMP_VELOCITY;
            self.is_grounded = false;
            return;
        }

        // Crouch
        if input.is_down() && self.is_grounded {
            self.action = PlayerAction::Crouch;
            self.action_frame = 0;
            self.vx = FixedPoint::ZERO;
            return;
        }

        // Movement
        let move_dir = if input.is_right() {
            1
        } else if input.is_left() {
            -1
        } else {
            0
        };

        if move_dir != 0 && self.is_grounded {
            let forward = move_dir == self.facing as i32;
            self.action = if forward {
                PlayerAction::WalkForward
            } else {
                PlayerAction::WalkBack
            };
            self.vx = FixedPoint(PLAYER_SPEED.0 * move_dir);
        } else if self.is_grounded {
            self.action = PlayerAction::Idle;
            self.action_frame = 0;
            self.vx = FixedPoint::ZERO;
        }
    }

    fn advance_stun(&mut self) {
        match &mut self.action {
            PlayerAction::Hitstun { frames_remaining } => {
                *frames_remaining -= 1;
                if *frames_remaining <= 0 {
                    self.action = PlayerAction::Idle;
                    self.action_frame = 0;
                }
            }
            PlayerAction::Blockstun { frames_remaining } => {
                *frames_remaining -= 1;
                if *frames_remaining <= 0 {
                    self.action = PlayerAction::Idle;
                    self.action_frame = 0;
                }
            }
            PlayerAction::Knockdown { frames_remaining } => {
                *frames_remaining -= 1;
                if *frames_remaining <= 0 {
                    self.action = PlayerAction::Getup;
                    self.action_frame = 0;
                }
            }
            PlayerAction::Getup => {
                self.action_frame += 1;
                if self.action_frame >= action_duration(&PlayerAction::Getup) {
                    self.action = PlayerAction::Idle;
                    self.action_frame = 0;
                }
            }
            _ => {}
        }
    }

    pub fn tick_physics(&mut self) {
        // Dash strike forward movement during active frames
        if self.action == PlayerAction::DashStrike {
            let (start, end) = active_frames(&PlayerAction::DashStrike);
            if self.action_frame >= start && self.action_frame <= end {
                self.vx = FixedPoint(PLAYER_SPEED.0 * self.facing as i32 * DASH_STRIKE_SPEED_MULT);
            } else {
                self.vx = FixedPoint::ZERO;
            }
        }

        // Apply gravity if airborne
        if !self.is_grounded {
            self.vy += GRAVITY;
        }

        // Update position
        self.x += self.vx;
        self.y += self.vy;

        // Ground collision
        let ground = FixedPoint::from_int(GROUND_Y);
        if self.y >= ground {
            self.y = ground;
            self.vy = FixedPoint::ZERO;
            if !self.is_grounded {
                self.is_grounded = true;
                if self.action == PlayerAction::Jump || self.action == PlayerAction::AerialAttack {
                    self.action = PlayerAction::Idle;
                    self.action_frame = 0;
                }
            }
        }

        // Stage bounds
        let min_x = FixedPoint::ZERO;
        let max_x = FixedPoint::from_int(STAGE_WIDTH);
        if self.x < min_x {
            self.x = min_x;
        }
        if self.x > max_x {
            self.x = max_x;
        }
    }

    pub fn get_hurtboxes(&self) -> [Option<AABB>; 4] {
        let facing = self.facing as i32;
        let _ = facing;
        // Body hurtbox (always present unless knockdown)
        if matches!(self.action, PlayerAction::Knockdown { .. }) {
            return [None, None, None, None];
        }

        let body_w = FixedPoint::from_int(40);
        let body_h = if self.action == PlayerAction::Crouch {
            FixedPoint::from_int(50)
        } else {
            FixedPoint::from_int(90)
        };
        let body_x = self.x - FixedPoint::from_int(20);
        let body_y = self.y - body_h;

        let body = Some(AABB::new(body_x, body_y, body_w, body_h));

        // Head hurtbox (smaller, above body for standing)
        let head = if self.action != PlayerAction::Crouch {
            Some(AABB::new(
                self.x - FixedPoint::from_int(12),
                self.y - FixedPoint::from_int(100),
                FixedPoint::from_int(24),
                FixedPoint::from_int(20),
            ))
        } else {
            None
        };

        [body, head, None, None]
    }

    pub fn get_hitboxes(&self) -> [Option<AABB>; 4] {
        if !is_attack(&self.action) {
            return [None, None, None, None];
        }

        let (start, end) = active_frames(&self.action);
        if self.action_frame < start || self.action_frame > end {
            return [None, None, None, None];
        }

        let facing = self.facing as i32;

        let (offset_x, offset_y, w, h) = match self.action {
            PlayerAction::LightAttack1 => (30, -60, 35, 20),
            PlayerAction::LightAttack2 => (25, -50, 40, 25),
            PlayerAction::LightAttack3 => (20, -45, 50, 30),
            PlayerAction::HeavyAttack => (25, -55, 55, 35),
            PlayerAction::Uppercut => (15, -80, 40, 50),
            PlayerAction::AerialAttack => (20, -40, 45, 35),
            PlayerAction::DashStrike => (20, -55, 60, 40),
            _ => return [None, None, None, None],
        };

        let hb_x = if facing > 0 {
            self.x + FixedPoint::from_int(offset_x)
        } else {
            self.x - FixedPoint::from_int(offset_x + w)
        };

        let hitbox = Some(AABB::new(
            hb_x,
            self.y + FixedPoint::from_int(offset_y),
            FixedPoint::from_int(w),
            FixedPoint::from_int(h),
        ));

        [hitbox, None, None, None]
    }

    pub fn take_hit(&mut self, damage: i32, hitstun: i32, _element: Element) {
        self.health -= damage;
        if self.health < 0 {
            self.health = 0;
        }

        if self.action == PlayerAction::Block {
            // Chip damage: BLOCK_CHIP_PERCENT% of attack damage — blocking is safe but not free
            let chip = damage * BLOCK_CHIP_PERCENT / 100;
            self.health += damage - chip;
            if self.health > MAX_HEALTH {
                self.health = MAX_HEALTH;
            }
            self.guard_meter -= 10;
            // Blockstun: shorter than hitstun so defender can act sooner
            let blockstun = (hitstun as i64 * BLOCKSTUN_RATIO as i64 / 1000) as i32;
            self.action = PlayerAction::Blockstun {
                frames_remaining: blockstun.max(4),
            };
        } else if damage >= KNOCKDOWN_THRESHOLD && self.is_grounded {
            self.action = PlayerAction::Knockdown {
                frames_remaining: KNOCKDOWN_FRAMES,
            };
        } else {
            self.action = PlayerAction::Hitstun {
                frames_remaining: hitstun,
            };
        }
        self.action_frame = 0;
        self.vx = FixedPoint::ZERO;
    }

    pub fn auto_face_opponent(&mut self, opponent_x: FixedPoint) {
        if opponent_x > self.x {
            self.facing = 1;
        } else if opponent_x < self.x {
            self.facing = -1;
        }
    }
}
