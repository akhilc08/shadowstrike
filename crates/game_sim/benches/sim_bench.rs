use criterion::{black_box, criterion_group, criterion_main, Criterion};
use game_sim::input::Input;
use game_sim::player::Element;
use game_sim::GameState;

fn bench_tick_simulation(c: &mut Criterion) {
    let mut state = GameState::new(Element::Fire, Element::Ice);
    // Warm up to a mid-game state
    for _ in 0..60 {
        state.tick(Input(0b00000010), Input(0b00000001));
    }

    c.bench_function("tick_simulation", |b| {
        b.iter(|| {
            state.tick(black_box(Input(0b00010010)), black_box(Input(0b00100001)));
        })
    });
}

fn bench_snapshot_save(c: &mut Criterion) {
    let mut state = GameState::new(Element::Lightning, Element::DarkMagic);
    for _ in 0..120 {
        state.tick(Input(0b00000010), Input(0b00010001));
    }

    c.bench_function("snapshot_save", |b| {
        b.iter(|| {
            let snap = black_box(&state).save_snapshot();
            black_box(snap);
        })
    });
}

fn bench_snapshot_restore(c: &mut Criterion) {
    let mut state = GameState::new(Element::Lightning, Element::DarkMagic);
    for _ in 0..120 {
        state.tick(Input(0b00000010), Input(0b00010001));
    }
    let snap = state.save_snapshot();

    c.bench_function("snapshot_restore", |b| {
        b.iter(|| {
            let mut target = GameState::new(Element::Lightning, Element::DarkMagic);
            target.restore_snapshot(black_box(snap));
            black_box(target);
        })
    });
}

fn make_inputs(n: usize) -> Vec<(Input, Input)> {
    (0..n)
        .map(|i| {
            let p1 = Input(((i * 7 + 3) % 256) as u8);
            let p2 = Input(((i * 13 + 5) % 256) as u8);
            (p1, p2)
        })
        .collect()
}

fn bench_rollback_1_frame(c: &mut Criterion) {
    c.bench_function("rollback_1_frame", |b| {
        b.iter_batched(
            || {
                let mut state = GameState::new(Element::Fire, Element::Ice);
                let inputs = make_inputs(100);
                for i in 0..99 {
                    state.tick(inputs[i].0, inputs[i].1);
                }
                let snapshot = state.save_snapshot();
                state.tick(inputs[99].0, inputs[99].1);
                (state, snapshot, inputs)
            },
            |(mut state, snapshot, inputs)| {
                state.restore_snapshot(black_box(snapshot));
                state.tick(black_box(inputs[99].0), black_box(inputs[99].1));
                black_box(state);
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_rollback_4_frames(c: &mut Criterion) {
    c.bench_function("rollback_4_frames", |b| {
        b.iter_batched(
            || {
                let mut state = GameState::new(Element::Fire, Element::Ice);
                let inputs = make_inputs(100);
                for i in 0..96 {
                    state.tick(inputs[i].0, inputs[i].1);
                }
                let snapshot = state.save_snapshot();
                for i in 96..100 {
                    state.tick(inputs[i].0, inputs[i].1);
                }
                (state, snapshot, inputs)
            },
            |(mut state, snapshot, inputs)| {
                state.restore_snapshot(black_box(snapshot));
                for i in 96..100 {
                    state.tick(black_box(inputs[i].0), black_box(inputs[i].1));
                }
                black_box(state);
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_rollback_8_frames(c: &mut Criterion) {
    c.bench_function("rollback_8_frames", |b| {
        b.iter_batched(
            || {
                let mut state = GameState::new(Element::Fire, Element::Ice);
                let inputs = make_inputs(100);
                for i in 0..92 {
                    state.tick(inputs[i].0, inputs[i].1);
                }
                let snapshot = state.save_snapshot();
                for i in 92..100 {
                    state.tick(inputs[i].0, inputs[i].1);
                }
                (state, snapshot, inputs)
            },
            |(mut state, snapshot, inputs)| {
                state.restore_snapshot(black_box(snapshot));
                for i in 92..100 {
                    state.tick(black_box(inputs[i].0), black_box(inputs[i].1));
                }
                black_box(state);
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

/// Particle system benchmark — standalone reimplementation of particle update
/// logic without web_sys dependencies. Simulates 2000 active particles.
fn bench_particle_update_2000(c: &mut Criterion) {
    #[derive(Clone, Copy)]
    enum Behavior {
        Standard,
        GravityAffected,
        Spiral { angle: f32 },
        DecelerateToStop,
    }

    #[derive(Clone, Copy)]
    struct Particle {
        x: f32,
        y: f32,
        vx: f32,
        vy: f32,
        lifetime: f32,
        max_lifetime: f32,
        a: f32,
        behavior: Behavior,
        active: bool,
    }

    fn pseudo_rand(seed: f32) -> f32 {
        let x = (seed * 12.9898 + 78.233).sin() * 43758.546;
        x - x.floor()
    }

    // Create 2000 active particles with varied behaviors
    let make_pool = || -> Vec<Particle> {
        (0..2000)
            .map(|i| {
                let rng = pseudo_rand(i as f32);
                let angle = rng * std::f32::consts::TAU;
                let speed = 1.0 + rng * 4.0;
                let behavior = match i % 4 {
                    0 => Behavior::Standard,
                    1 => Behavior::GravityAffected,
                    2 => Behavior::Spiral { angle: 0.0 },
                    _ => Behavior::DecelerateToStop,
                };
                Particle {
                    x: 600.0 + rng * 100.0,
                    y: 300.0 + rng * 100.0,
                    vx: angle.cos() * speed,
                    vy: angle.sin() * speed,
                    lifetime: rng * 10.0,
                    max_lifetime: 20.0 + rng * 30.0,
                    a: 1.0,
                    behavior,
                    active: true,
                }
            })
            .collect()
    };

    c.bench_function("particle_update_2000", |b| {
        b.iter_batched(
            make_pool,
            |mut particles| {
                for p in particles.iter_mut() {
                    if !p.active {
                        continue;
                    }
                    p.lifetime += 1.0;
                    if p.lifetime >= p.max_lifetime {
                        p.active = false;
                        continue;
                    }
                    let life_ratio = p.lifetime / p.max_lifetime;
                    p.a = 1.0 - life_ratio;

                    match &mut p.behavior {
                        Behavior::Standard => {
                            p.x += p.vx;
                            p.y += p.vy;
                        }
                        Behavior::GravityAffected => {
                            p.x += p.vx;
                            p.y += p.vy;
                            p.vy += 0.15;
                        }
                        Behavior::Spiral { angle } => {
                            *angle += 0.2;
                            let radius = (1.0 - life_ratio) * 3.0;
                            p.x += angle.cos() * radius + p.vx * 0.3;
                            p.y += angle.sin() * radius + p.vy * 0.3;
                        }
                        Behavior::DecelerateToStop => {
                            p.x += p.vx;
                            p.y += p.vy;
                            p.vx *= 0.92;
                            p.vy *= 0.92;
                            p.vy += 0.05;
                        }
                    }
                }
                black_box(particles);
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

criterion_group!(
    benches,
    bench_tick_simulation,
    bench_snapshot_save,
    bench_snapshot_restore,
    bench_rollback_1_frame,
    bench_rollback_4_frames,
    bench_rollback_8_frames,
    bench_particle_update_2000,
);
criterion_main!(benches);
