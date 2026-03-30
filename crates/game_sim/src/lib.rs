pub mod collision;
pub mod combo;
pub mod constants;
pub mod fixed;
pub mod input;
pub mod player;
pub mod ring_buffer;

use collision::{check_hit, overlaps, AABB};
use combo::ComboState;
use constants::*;
use fixed::FixedPoint;
use input::Input;
use player::{attack_damage, attack_hitstun, Element, PlayerAction, PlayerState};

pub const MAX_PROJECTILES: usize = 4;

#[derive(Debug, Clone, Copy)]
pub struct Projectile {
    pub active: bool,
    pub x: FixedPoint,
    pub y: FixedPoint,
    pub vx: FixedPoint,
    pub owner: usize,
    pub damage: i32,
    pub hitstun: i32,
    pub lifetime: i32,
    pub element: Element,
}

impl Projectile {
    pub fn empty() -> Self {
        Projectile {
            active: false,
            x: FixedPoint::ZERO,
            y: FixedPoint::ZERO,
            vx: FixedPoint::ZERO,
            owner: 0,
            damage: 0,
            hitstun: 0,
            lifetime: 0,
            element: Element::Fire,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GamePhase {
    Fighting,
    RoundEnd { winner: i32, countdown: i32 },
    MatchEnd { winner: i32 },
}

#[derive(Debug, Clone, Copy)]
pub struct GameState {
    pub players: [PlayerState; 2],
    pub combo: [ComboState; 2],
    pub projectiles: [Projectile; MAX_PROJECTILES],
    pub round_timer: i32,
    pub round_number: i32,
    pub round_scores: [i32; 2],
    pub frame_number: u64,
    pub phase: GamePhase,
    hit_this_frame: [bool; 2],
}

impl GameState {
    pub fn new(p1_element: Element, p2_element: Element) -> Self {
        GameState {
            players: [
                PlayerState::new(FixedPoint::from_int(300), p1_element),
                {
                    let mut p = PlayerState::new(FixedPoint::from_int(900), p2_element);
                    p.facing = -1;
                    p
                },
            ],
            combo: [ComboState::new(), ComboState::new()],
            projectiles: [Projectile::empty(); MAX_PROJECTILES],
            round_timer: ROUND_TIME_SECONDS * TICKS_PER_SECOND,
            round_number: 1,
            round_scores: [0, 0],
            frame_number: 0,
            phase: GamePhase::Fighting,
            hit_this_frame: [false; 2],
        }
    }

    pub fn tick(&mut self, p1_input: Input, p2_input: Input) {
        match self.phase {
            GamePhase::MatchEnd { .. } => return,
            GamePhase::RoundEnd { countdown, winner } => {
                if countdown <= 0 {
                    self.start_new_round();
                } else {
                    self.phase = GamePhase::RoundEnd {
                        winner,
                        countdown: countdown - 1,
                    };
                }
                self.frame_number += 1;
                return;
            }
            GamePhase::Fighting => {}
        }

        let inputs = [p1_input, p2_input];

        // Apply inputs
        let opp_x_0 = self.players[1].x;
        let opp_x_1 = self.players[0].x;
        self.players[0].apply_input(inputs[0], opp_x_0);
        self.players[1].apply_input(inputs[1], opp_x_1);

        // Physics
        self.players[0].tick_physics();
        self.players[1].tick_physics();

        // Hit detection
        self.hit_this_frame = [false; 2];

        // Spawn projectiles from Fireball action at the right frame
        for i in 0..2 {
            if self.players[i].action == PlayerAction::Fireball && self.players[i].action_frame == 8 {
                if let Some(slot) = self.projectiles.iter().position(|p| !p.active) {
                    let facing = self.players[i].facing as i32;
                    self.projectiles[slot] = Projectile {
                        active: true,
                        x: self.players[i].x + FixedPoint::from_int(30 * facing),
                        y: self.players[i].y - FixedPoint::from_int(50),
                        vx: FixedPoint::from_int(PROJECTILE_SPEED * facing),
                        owner: i,
                        damage: PROJECTILE_DAMAGE,
                        hitstun: PROJECTILE_HITSTUN,
                        lifetime: PROJECTILE_LIFETIME,
                        element: self.players[i].element,
                    };
                }
            }
        }

        // Tick projectiles
        for proj in &mut self.projectiles {
            if !proj.active {
                continue;
            }
            proj.x += proj.vx;
            proj.lifetime -= 1;
            if proj.lifetime <= 0 || proj.x < FixedPoint::ZERO || proj.x > FixedPoint::from_int(STAGE_WIDTH) {
                proj.active = false;
            }
        }

        // Projectile-vs-player collision
        for pi in 0..MAX_PROJECTILES {
            if !self.projectiles[pi].active {
                continue;
            }
            let defender_idx = 1 - self.projectiles[pi].owner;
            let proj_box = AABB::new(
                self.projectiles[pi].x - FixedPoint::from_int(10),
                self.projectiles[pi].y - FixedPoint::from_int(10),
                FixedPoint::from_int(20),
                FixedPoint::from_int(20),
            );
            let hurtboxes = self.players[defender_idx].get_hurtboxes();
            for hb in hurtboxes.iter().flatten() {
                if overlaps(&proj_box, hb) {
                    let element = self.projectiles[pi].element;
                    let damage = self.projectiles[pi].damage;
                    let hitstun = self.projectiles[pi].hitstun;
                    let attacker_idx = self.projectiles[pi].owner;
                    self.players[defender_idx].take_hit(damage, hitstun, element);
                    self.hit_this_frame[attacker_idx] = true;
                    self.players[attacker_idx].energy = (self.players[attacker_idx].energy + ENERGY_PER_HIT).min(MAX_ENERGY);
                    self.projectiles[pi].active = false;
                    break;
                }
            }
        }

        // Melee hit detection
        for attacker_idx in 0..2 {
            let defender_idx = 1 - attacker_idx;
            let hitboxes = self.players[attacker_idx].get_hitboxes();
            let hurtboxes = self.players[defender_idx].get_hurtboxes();

            if check_hit(&hitboxes, &hurtboxes) {
                let action = self.players[attacker_idx].action;
                let damage = attack_damage(&action);
                let base_hitstun = attack_hitstun(&action);

                // Apply combo scaling
                let scale = self.combo[attacker_idx].register_hit();
                let scaled_hitstun =
                    ((base_hitstun as i64 * scale.raw() as i64) / 1000) as i32;
                let scaled_hitstun = if scaled_hitstun < 4 { 4 } else { scaled_hitstun };

                let element = self.players[attacker_idx].element;
                self.players[defender_idx].take_hit(damage, scaled_hitstun, element);
                self.hit_this_frame[attacker_idx] = true;

                // Energy gain for attacker
                self.players[attacker_idx].energy =
                    (self.players[attacker_idx].energy + ENERGY_PER_HIT).min(MAX_ENERGY);
            }
        }

        // Reset combo if no hit this frame and defender is actionable
        for i in 0..2 {
            let defender = 1 - i;
            if !self.hit_this_frame[i] {
                let defender_action = &self.players[defender].action;
                if matches!(
                    defender_action,
                    PlayerAction::Idle
                        | PlayerAction::WalkForward
                        | PlayerAction::WalkBack
                        | PlayerAction::Jump
                        | PlayerAction::Crouch
                ) {
                    self.combo[i].reset();
                }
            }
        }

        // Timer
        self.round_timer -= 1;

        // Check round end
        let p1_dead = self.players[0].health <= 0;
        let p2_dead = self.players[1].health <= 0;
        let time_up = self.round_timer <= 0;

        if p1_dead || p2_dead || time_up {
            let winner = if p1_dead && p2_dead {
                0 // draw, no score
            } else if p2_dead {
                self.round_scores[0] += 1;
                0
            } else if p1_dead {
                self.round_scores[1] += 1;
                1
            } else {
                // time up — higher health wins
                if self.players[0].health >= self.players[1].health {
                    self.round_scores[0] += 1;
                    0
                } else {
                    self.round_scores[1] += 1;
                    1
                }
            };

            // Best of 3: first to 2 rounds
            if self.round_scores[0] >= 2 || self.round_scores[1] >= 2 {
                self.phase = GamePhase::MatchEnd { winner };
            } else {
                self.phase = GamePhase::RoundEnd {
                    winner,
                    countdown: 120,
                };
            }
        }

        self.frame_number += 1;
    }

    fn start_new_round(&mut self) {
        self.players[0] = PlayerState::new(FixedPoint::from_int(300), self.players[0].element);
        self.players[1] = {
            let mut p = PlayerState::new(FixedPoint::from_int(900), self.players[1].element);
            p.facing = -1;
            p
        };
        self.combo = [ComboState::new(), ComboState::new()];
        self.projectiles = [Projectile::empty(); MAX_PROJECTILES];
        self.round_timer = ROUND_TIME_SECONDS * TICKS_PER_SECOND;
        self.round_number += 1;
        self.phase = GamePhase::Fighting;
        self.hit_this_frame = [false; 2];
    }

    /// CRC32 checksum of entire game state for desync detection.
    pub fn checksum(&self) -> u32 {
        let mut crc: u32 = 0xFFFFFFFF;
        let mut feed = |val: i32| {
            let bytes = val.to_le_bytes();
            for &b in &bytes {
                let idx = ((crc ^ b as u32) & 0xFF) as usize;
                crc = CRC32_TABLE[idx] ^ (crc >> 8);
            }
        };

        for p in &self.players {
            feed(p.x.raw());
            feed(p.y.raw());
            feed(p.vx.raw());
            feed(p.vy.raw());
            feed(p.facing as i32);
            feed(p.health);
            feed(p.energy);
            feed(p.guard_meter);
            feed(p.action_frame);
        }
        for proj in &self.projectiles {
            feed(if proj.active { 1 } else { 0 });
            feed(proj.x.raw());
            feed(proj.y.raw());
            feed(proj.lifetime);
        }
        feed(self.round_timer);
        feed(self.frame_number as i32);

        crc ^ 0xFFFFFFFF
    }

    pub fn save_snapshot(&self) -> GameState {
        *self
    }

    pub fn restore_snapshot(&mut self, snapshot: GameState) {
        *self = snapshot;
    }
}

/// CRC32 lookup table (IEEE polynomial).
const CRC32_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut i = 0u32;
    while i < 256 {
        let mut crc = i;
        let mut j = 0;
        while j < 8 {
            if crc & 1 != 0 {
                crc = 0xEDB88320 ^ (crc >> 1);
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        table[i as usize] = crc;
        i += 1;
    }
    table
};
