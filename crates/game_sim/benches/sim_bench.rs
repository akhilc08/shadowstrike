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

fn bench_snapshot_save_restore(c: &mut Criterion) {
    let mut state = GameState::new(Element::Lightning, Element::DarkMagic);
    for _ in 0..120 {
        state.tick(Input(0b00000010), Input(0b00010001));
    }

    c.bench_function("snapshot_save_restore", |b| {
        b.iter(|| {
            let snap = black_box(&state).save_snapshot();
            let mut restored = GameState::new(Element::Lightning, Element::DarkMagic);
            restored.restore_snapshot(black_box(snap));
            black_box(restored);
        })
    });
}

fn bench_rollback_8_frames(c: &mut Criterion) {
    c.bench_function("rollback_8_frames", |b| {
        b.iter_batched(
            || {
                // Setup: create a state advanced 100 frames, save snapshot at frame 92
                let mut state = GameState::new(Element::Fire, Element::Ice);
                let inputs: Vec<(Input, Input)> = (0..100)
                    .map(|i| {
                        let p1 = Input(((i * 7 + 3) % 256) as u8);
                        let p2 = Input(((i * 13 + 5) % 256) as u8);
                        (p1, p2)
                    })
                    .collect();
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
                // Rollback: restore to frame 92, re-simulate 8 frames
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

criterion_group!(
    benches,
    bench_tick_simulation,
    bench_snapshot_save_restore,
    bench_rollback_8_frames,
);
criterion_main!(benches);
