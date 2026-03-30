use game_sim::Element;
use wasm_bindgen::JsValue;
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
            EffectType::HitImpact => 20,
            EffectType::SwordTrail => 3,
            EffectType::IdleAmbient => 1,
            EffectType::SpecialActivation => 30,
            EffectType::WalkDust => 5,
        };

        for _ in 0..count {
            let idx = match self.alloc() {
                Some(i) => i,
                None => return,
            };

            let (r, g, b, behavior) = element_style(element, effect);
            let angle = pseudo_random_angle(x, y, idx as f32);
            let speed = match effect {
                EffectType::HitImpact => 2.0 + pseudo_rand(idx as f32) * 3.0,
                EffectType::SwordTrail => 0.3 + pseudo_rand(idx as f32) * 0.5,
                EffectType::IdleAmbient => 0.2 + pseudo_rand(idx as f32) * 0.3,
                EffectType::SpecialActivation => 3.0 + pseudo_rand(idx as f32) * 4.0,
                EffectType::WalkDust => 0.5 + pseudo_rand(idx as f32) * 1.0,
            };
            let max_life = match effect {
                EffectType::HitImpact => 15.0 + pseudo_rand(idx as f32) * 10.0,
                EffectType::SwordTrail => 8.0 + pseudo_rand(idx as f32) * 4.0,
                EffectType::IdleAmbient => 30.0 + pseudo_rand(idx as f32) * 20.0,
                EffectType::SpecialActivation => 20.0 + pseudo_rand(idx as f32) * 15.0,
                EffectType::WalkDust => 10.0 + pseudo_rand(idx as f32) * 5.0,
            };
            let size = match effect {
                EffectType::HitImpact => 2.0 + pseudo_rand(idx as f32) * 3.0,
                EffectType::SwordTrail => 1.0 + pseudo_rand(idx as f32),
                EffectType::IdleAmbient => 1.0 + pseudo_rand(idx as f32) * 2.0,
                EffectType::SpecialActivation => 3.0 + pseudo_rand(idx as f32) * 4.0,
                EffectType::WalkDust => 2.0 + pseudo_rand(idx as f32) * 2.0,
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
            ctx.set_fill_style(&JsValue::from_str(&color));
            ctx.fill_rect(
                (p.x - p.size * 0.5) as f64,
                (p.y - p.size * 0.5) as f64,
                p.size as f64,
                p.size as f64,
            );
        }
    }
}

fn element_style(element: Element, effect: EffectType) -> (u8, u8, u8, ParticleBehavior) {
    match element {
        Element::Fire => {
            let behavior = match effect {
                EffectType::WalkDust => ParticleBehavior::GravityAffected,
                _ => ParticleBehavior::Standard,
            };
            (255, 140, 20, behavior) // orange
        }
        Element::Lightning => {
            let behavior = match effect {
                EffectType::HitImpact | EffectType::SpecialActivation => ParticleBehavior::Standard,
                _ => ParticleBehavior::Standard,
            };
            (180, 210, 255, behavior) // blue-white
        }
        Element::DarkMagic => {
            let angle = 0.0_f32;
            (160, 50, 220, ParticleBehavior::Spiral { angle }) // purple
        }
        Element::Ice => {
            (150, 240, 255, ParticleBehavior::DecelerateToStop) // cyan
        }
    }
}

/// Simple deterministic pseudo-random in [0, 1).
fn pseudo_rand(seed: f32) -> f32 {
    let x = (seed * 12.9898 + 78.233).sin() * 43758.5453;
    x - x.floor()
}

fn pseudo_random_angle(x: f32, y: f32, seed: f32) -> f32 {
    pseudo_rand(x * 0.1 + y * 0.3 + seed) * std::f32::consts::TAU
}
