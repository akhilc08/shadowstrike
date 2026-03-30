use game_sim::player::Element;
use web_sys::CanvasRenderingContext2d;

const POOL_SIZE: usize = 2000;

#[derive(Debug, Clone, Copy)]
pub enum ParticleBehavior {
    Standard,
    GravityAffected,
    Spiral { angle: f32 },
    DecelerateToStop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectType {
    HitImpact,
    SwordTrail,
    IdleAmbient,
    SpecialActivation,
    WalkDust,
    BlockSpark,
    KnockdownSlam,
}

#[derive(Debug, Clone, Copy)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub lifetime: f32,
    pub max_lifetime: f32,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f32,
    pub size: f32,
    pub behavior: ParticleBehavior,
    pub active: bool,
}

impl Particle {
    fn dead() -> Self {
        Particle {
            x: 0.0, y: 0.0, vx: 0.0, vy: 0.0,
            lifetime: 0.0, max_lifetime: 0.0,
            r: 0, g: 0, b: 0, a: 0.0, size: 0.0,
            behavior: ParticleBehavior::Standard,
            active: false,
        }
    }
}

pub struct ParticlePool {
    particles: Vec<Particle>,
    free_list: Vec<usize>,
    active: Vec<usize>,
}

impl Default for ParticlePool {
    fn default() -> Self {
        Self::new()
    }
}

impl ParticlePool {
    pub fn new() -> Self {
        let particles = vec![Particle::dead(); POOL_SIZE];
        let free_list: Vec<usize> = (0..POOL_SIZE).rev().collect();
        ParticlePool {
            particles,
            free_list,
            active: Vec::with_capacity(512),
        }
    }

    fn alloc(&mut self) -> Option<usize> {
        let idx = self.free_list.pop()?;
        self.active.push(idx);
        Some(idx)
    }

    pub fn emit(&mut self, x: f32, y: f32, element: Element, effect: EffectType) {
        let count = match effect {
            EffectType::HitImpact => 24,
            EffectType::SwordTrail => 4,
            EffectType::IdleAmbient => 1,
            EffectType::SpecialActivation => 35,
            EffectType::WalkDust => 5,
            EffectType::BlockSpark => 12,
            EffectType::KnockdownSlam => 30,
        };

        for _ in 0..count {
            let idx = match self.alloc() {
                Some(i) => i,
                None => return,
            };

            let (r, g, b, behavior) = element_style(element, effect);
            let angle = pseudo_random_angle(x, y, idx as f32);
            let rng = pseudo_rand(idx as f32);
            let speed = match effect {
                EffectType::HitImpact => 2.0 + rng * 3.5,
                EffectType::SwordTrail => 0.3 + rng * 0.5,
                EffectType::IdleAmbient => 0.15 + rng * 0.25,
                EffectType::SpecialActivation => 3.0 + rng * 5.0,
                EffectType::WalkDust => 0.5 + rng * 1.0,
                EffectType::BlockSpark => 2.5 + rng * 2.0,
                EffectType::KnockdownSlam => 1.5 + rng * 3.0,
            };
            let max_life = match effect {
                EffectType::HitImpact => 12.0 + rng * 10.0,
                EffectType::SwordTrail => 6.0 + rng * 4.0,
                EffectType::IdleAmbient => 40.0 + rng * 30.0,
                EffectType::SpecialActivation => 20.0 + rng * 15.0,
                EffectType::WalkDust => 10.0 + rng * 5.0,
                EffectType::BlockSpark => 6.0 + rng * 6.0,
                EffectType::KnockdownSlam => 15.0 + rng * 10.0,
            };
            let size = match effect {
                EffectType::HitImpact => 2.0 + rng * 3.5,
                EffectType::SwordTrail => 1.0 + rng * 1.5,
                EffectType::IdleAmbient => 1.0 + rng * 2.5,
                EffectType::SpecialActivation => 3.0 + rng * 5.0,
                EffectType::WalkDust => 2.0 + rng * 2.0,
                EffectType::BlockSpark => 1.5 + rng * 2.0,
                EffectType::KnockdownSlam => 2.5 + rng * 3.0,
            };

            self.particles[idx] = Particle {
                x,
                y,
                vx: angle.cos() * speed,
                vy: angle.sin() * speed,
                lifetime: 0.0,
                max_lifetime: max_life,
                r, g, b,
                a: 1.0,
                size,
                behavior,
                active: true,
            };
        }
    }

    pub fn update(&mut self, _dt: f32) {
        let mut i = 0;
        while i < self.active.len() {
            let idx = self.active[i];
            let p = &mut self.particles[idx];
            p.lifetime += 1.0;
            if p.lifetime >= p.max_lifetime {
                p.active = false;
                self.free_list.push(idx);
                self.active.swap_remove(i);
                continue;
            }

            let life_ratio = p.lifetime / p.max_lifetime;
            p.a = 1.0 - life_ratio;

            match p.behavior {
                ParticleBehavior::Standard => {
                    p.x += p.vx;
                    p.y += p.vy;
                }
                ParticleBehavior::GravityAffected => {
                    p.x += p.vx;
                    p.y += p.vy;
                    p.vy += 0.15;
                }
                ParticleBehavior::Spiral { ref mut angle } => {
                    *angle += 0.2;
                    let radius = (1.0 - life_ratio) * 3.0;
                    p.x += angle.cos() * radius + p.vx * 0.3;
                    p.y += angle.sin() * radius + p.vy * 0.3;
                }
                ParticleBehavior::DecelerateToStop => {
                    p.x += p.vx;
                    p.y += p.vy;
                    p.vx *= 0.92;
                    p.vy *= 0.92;
                    p.vy += 0.05;
                }
            }

            i += 1;
        }
    }

    pub fn render(&self, ctx: &CanvasRenderingContext2d) {
        for &idx in &self.active {
            let p = &self.particles[idx];
            if !p.active {
                continue;
            }
            let color = format!("rgba({},{},{},{:.2})", p.r, p.g, p.b, p.a);
            ctx.set_fill_style_str(&color);
            // Small particles as squares (faster), larger ones as circles
            if p.size < 2.5 {
                ctx.fill_rect(
                    (p.x - p.size * 0.5) as f64,
                    (p.y - p.size * 0.5) as f64,
                    p.size as f64,
                    p.size as f64,
                );
            } else {
                ctx.begin_path();
                let _ = ctx.arc(
                    p.x as f64,
                    p.y as f64,
                    (p.size * 0.5) as f64,
                    0.0,
                    std::f64::consts::TAU,
                );
                ctx.fill();
            }
        }
    }
}

fn element_style(element: Element, effect: EffectType) -> (u8, u8, u8, ParticleBehavior) {
    // Block sparks and knockdown slams are always white/yellow
    if effect == EffectType::BlockSpark {
        return (255, 255, 200, ParticleBehavior::Standard);
    }
    if effect == EffectType::KnockdownSlam {
        return (200, 180, 150, ParticleBehavior::GravityAffected);
    }

    match element {
        Element::Fire => {
            let behavior = match effect {
                EffectType::WalkDust => ParticleBehavior::GravityAffected,
                EffectType::HitImpact => ParticleBehavior::GravityAffected,
                EffectType::SpecialActivation => ParticleBehavior::Standard,
                _ => ParticleBehavior::Standard,
            };
            // Vary fire colors: orange core, red/yellow edges
            (255, 140, 20, behavior)
        }
        Element::Lightning => {
            let behavior = match effect {
                EffectType::HitImpact | EffectType::SpecialActivation => ParticleBehavior::Standard,
                _ => ParticleBehavior::Standard,
            };
            (180, 210, 255, behavior)
        }
        Element::DarkMagic => {
            let behavior = match effect {
                EffectType::IdleAmbient | EffectType::SpecialActivation => {
                    ParticleBehavior::Spiral { angle: 0.0 }
                }
                EffectType::HitImpact => ParticleBehavior::Standard,
                _ => ParticleBehavior::Spiral { angle: 0.0 },
            };
            (160, 50, 220, behavior)
        }
        Element::Ice => {
            let behavior = match effect {
                EffectType::HitImpact => ParticleBehavior::GravityAffected,
                _ => ParticleBehavior::DecelerateToStop,
            };
            (150, 240, 255, behavior)
        }
    }
}

fn pseudo_rand(seed: f32) -> f32 {
    let x = (seed * 12.9898 + 78.233).sin() * 43758.546;
    x - x.floor()
}

fn pseudo_random_angle(x: f32, y: f32, seed: f32) -> f32 {
    pseudo_rand(x * 0.1 + y * 0.3 + seed) * std::f32::consts::TAU
}
