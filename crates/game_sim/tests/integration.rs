use game_sim::*;
use game_sim::fixed::FixedPoint;
use game_sim::input::Input;
use game_sim::player::Element;
use game_sim::ring_buffer::RingBuffer;
use game_sim::collision::{AABB, overlaps};

#[test]
fn test_fixed_point_arithmetic() {
    let a = FixedPoint::from_int(3);
    let b = FixedPoint::from_int(2);

    // Basic operations
    assert_eq!((a + b).raw(), 5000);
    assert_eq!((a - b).raw(), 1000);
    assert_eq!((a * b).raw(), 6000);
    assert_eq!((a / b).raw(), 1500);
    assert_eq!((-a).raw(), -3000);

    // Identity
    assert_eq!((FixedPoint::ZERO + a).raw(), a.raw());
    assert_eq!((a * FixedPoint::ONE).raw(), a.raw());
    assert_eq!((a / FixedPoint::ONE).raw(), a.raw());

    // Fractional
    let half = FixedPoint(500);
    assert_eq!((half + half).raw(), FixedPoint::ONE.raw());
    assert_eq!((half * FixedPoint::from_int(6)).raw(), 3000);

    // Negative
    let neg = FixedPoint::from_int(-5);
    assert_eq!((neg + FixedPoint::from_int(3)).raw(), -2000);
    assert_eq!((neg * FixedPoint::from_int(-2)).raw(), 10000);

    // from_f32 / to_f32 roundtrip
    let fp = FixedPoint::from_f32(3.5);
    assert!((fp.to_f32() - 3.5).abs() < 0.01);

    // Edge: multiply by zero
    assert_eq!((a * FixedPoint::ZERO).raw(), 0);
}

#[test]
fn test_determinism() {
    // Run 1000 frames with scripted inputs twice, assert checksums match
    let scripted_inputs: Vec<(Input, Input)> = (0..1000)
        .map(|i| {
            let p1 = match i % 7 {
                0 => Input(0b00000010), // right
                1 => Input(0b00010000), // light
                2 => Input(0b00000100), // jump
                3 => Input(0b00100000), // heavy
                4 => Input(0b00000001), // left
                5 => Input(0b10000000), // block
                _ => Input(0),          // neutral
            };
            let p2 = match i % 5 {
                0 => Input(0b00000001), // left
                1 => Input(0b00010000), // light
                2 => Input(0b00000100), // jump
                3 => Input(0b00100000), // heavy
                _ => Input(0),
            };
            (p1, p2)
        })
        .collect();

    let mut checksums_a = Vec::new();
    let mut state_a = GameState::new(Element::Fire, Element::Ice);
    for &(p1, p2) in &scripted_inputs {
        state_a.tick(p1, p2);
        checksums_a.push(state_a.checksum());
    }

    let mut checksums_b = Vec::new();
    let mut state_b = GameState::new(Element::Fire, Element::Ice);
    for &(p1, p2) in &scripted_inputs {
        state_b.tick(p1, p2);
        checksums_b.push(state_b.checksum());
    }

    assert_eq!(checksums_a, checksums_b, "Simulation is not deterministic!");
}

#[test]
fn test_no_infinite_combo() {
    // Fuzz random inputs for 10000 frames, assert combo never exceeds 15
    let mut state = GameState::new(Element::Lightning, Element::DarkMagic);
    let mut rng: u32 = 12345; // simple LCG

    for _ in 0..10000 {
        rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
        let p1_input = Input((rng >> 16) as u8);
        rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
        let p2_input = Input((rng >> 16) as u8);

        state.tick(p1_input, p2_input);

        assert!(
            state.combo[0].hit_count <= 15,
            "P1 combo exceeded 15: {}",
            state.combo[0].hit_count
        );
        assert!(
            state.combo[1].hit_count <= 15,
            "P2 combo exceeded 15: {}",
            state.combo[1].hit_count
        );
    }
}

#[test]
fn test_health_bounds() {
    // Fuzz random inputs, assert health always in [0, 1000]
    let mut state = GameState::new(Element::Fire, Element::Ice);
    let mut rng: u32 = 67890;

    for _ in 0..10000 {
        rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
        let p1_input = Input((rng >> 16) as u8);
        rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
        let p2_input = Input((rng >> 16) as u8);

        state.tick(p1_input, p2_input);

        for i in 0..2 {
            assert!(
                state.players[i].health >= 0,
                "Player {} health below 0: {}",
                i,
                state.players[i].health
            );
            assert!(
                state.players[i].health <= game_sim::constants::MAX_HEALTH,
                "Player {} health above {}: {}",
                i,
                game_sim::constants::MAX_HEALTH,
                state.players[i].health
            );
        }
    }
}

#[test]
fn test_ring_buffer() {
    let mut buf: RingBuffer<u32, 8> = RingBuffer::new(0);

    // Write and read
    for i in 0..8u64 {
        buf.write(i, (i * 100) as u32);
    }
    for i in 0..8u64 {
        assert_eq!(buf.read(i), Some(&((i * 100) as u32)));
    }

    // Overwrite wraps around
    buf.write(8, 800);
    assert_eq!(buf.read(8), Some(&800));
    assert_eq!(buf.read(0), None); // slot 0 now holds frame 8

    // Read non-existent
    assert_eq!(buf.read(999), None);

    // Large frame numbers
    buf.write(1_000_000, 42);
    assert_eq!(buf.read(1_000_000), Some(&42));
}

#[test]
fn test_aabb_overlap() {
    // Overlapping
    let a = AABB::new(
        FixedPoint::from_int(0),
        FixedPoint::from_int(0),
        FixedPoint::from_int(10),
        FixedPoint::from_int(10),
    );
    let b = AABB::new(
        FixedPoint::from_int(5),
        FixedPoint::from_int(5),
        FixedPoint::from_int(10),
        FixedPoint::from_int(10),
    );
    assert!(overlaps(&a, &b));

    // Non-overlapping
    let c = AABB::new(
        FixedPoint::from_int(20),
        FixedPoint::from_int(20),
        FixedPoint::from_int(5),
        FixedPoint::from_int(5),
    );
    assert!(!overlaps(&a, &c));

    // Edge touching = not overlapping
    let d = AABB::new(
        FixedPoint::from_int(10),
        FixedPoint::from_int(0),
        FixedPoint::from_int(5),
        FixedPoint::from_int(5),
    );
    assert!(!overlaps(&a, &d));

    // Contained
    let inner = AABB::new(
        FixedPoint::from_int(2),
        FixedPoint::from_int(2),
        FixedPoint::from_int(3),
        FixedPoint::from_int(3),
    );
    assert!(overlaps(&a, &inner));
}

#[test]
fn test_snapshot_restore() {
    let mut state = GameState::new(Element::Fire, Element::Ice);
    // Advance a few frames
    for _ in 0..10 {
        state.tick(Input(0b00000010), Input(0b00000001));
    }
    let snap = state.save_snapshot();
    let checksum_before = state.checksum();

    // Advance more
    for _ in 0..10 {
        state.tick(Input(0b00010000), Input(0b00100000));
    }
    assert_ne!(state.checksum(), checksum_before);

    // Restore
    state.restore_snapshot(snap);
    assert_eq!(state.checksum(), checksum_before);
}
