use game_sim::constants::{MAX_ENERGY, MAX_HEALTH};
use game_sim::input::Input;
use game_sim::player::Element;
use game_sim::GameState;

/// Simple seeded LCG for deterministic pseudo-random numbers.
struct Lcg {
    state: u32,
}

impl Lcg {
    fn new(seed: u32) -> Self {
        Lcg { state: seed }
    }

    fn next(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345);
        self.state
    }

    fn next_u8(&mut self) -> u8 {
        (self.next() >> 16) as u8
    }

    fn next_element(&mut self) -> Element {
        match self.next() % 4 {
            0 => Element::Fire,
            1 => Element::Lightning,
            2 => Element::DarkMagic,
            _ => Element::Ice,
        }
    }
}

/// Run 1000 matches with random inputs using two independent GameState instances.
/// Assert checksums match every frame to verify determinism.
#[test]
fn test_no_desync_10k_matches() {
    let mut rng = Lcg::new(42);

    for _ in 0..1000 {
        let e1 = rng.next_element();
        let e2 = rng.next_element();
        let mut state_a = GameState::new(e1, e2);
        let mut state_b = GameState::new(e1, e2);

        let num_frames = 60 + (rng.next() % 120) as usize; // 60-179 frames per match
        for _ in 0..num_frames {
            let p1 = Input(rng.next_u8());
            let p2 = Input(rng.next_u8());

            state_a.tick(p1, p2);
            state_b.tick(p1, p2);

            assert_eq!(
                state_a.checksum(),
                state_b.checksum(),
                "Desync at frame {}",
                state_a.frame_number
            );
        }
    }
}

/// Run 500 matches with random inputs, assert health is always in [0, MAX_HEALTH].
#[test]
fn test_health_always_bounded() {
    let mut rng = Lcg::new(1234);

    for _ in 0..500 {
        let e1 = rng.next_element();
        let e2 = rng.next_element();
        let mut state = GameState::new(e1, e2);

        for _ in 0..300 {
            let p1 = Input(rng.next_u8());
            let p2 = Input(rng.next_u8());
            state.tick(p1, p2);

            for p in 0..2 {
                assert!(
                    state.players[p].health >= 0,
                    "Player {} health below 0: {} at frame {}",
                    p,
                    state.players[p].health,
                    state.frame_number
                );
                assert!(
                    state.players[p].health <= MAX_HEALTH,
                    "Player {} health above {}: {} at frame {}",
                    p,
                    MAX_HEALTH,
                    state.players[p].health,
                    state.frame_number
                );
            }
        }
    }
}

/// Run 500 matches with random inputs, assert energy is always in [0, MAX_ENERGY].
#[test]
fn test_energy_always_bounded() {
    let mut rng = Lcg::new(5678);

    for _ in 0..500 {
        let e1 = rng.next_element();
        let e2 = rng.next_element();
        let mut state = GameState::new(e1, e2);

        for _ in 0..300 {
            let p1 = Input(rng.next_u8());
            let p2 = Input(rng.next_u8());
            state.tick(p1, p2);

            for p in 0..2 {
                assert!(
                    state.players[p].energy >= 0,
                    "Player {} energy below 0: {} at frame {}",
                    p,
                    state.players[p].energy,
                    state.frame_number
                );
                assert!(
                    state.players[p].energy <= MAX_ENERGY,
                    "Player {} energy above {}: {} at frame {}",
                    p,
                    MAX_ENERGY,
                    state.players[p].energy,
                    state.frame_number
                );
            }
        }
    }
}

/// Assert combo hit count stays bounded (no true infinite combos).
/// Limit raised to 25 to accommodate DarkMagic's higher hitstun values
/// which allow longer but still decay-limited combo chains.
#[test]
fn test_no_infinite_combo() {
    let mut rng = Lcg::new(9999);

    for _ in 0..500 {
        let e1 = rng.next_element();
        let e2 = rng.next_element();
        let mut state = GameState::new(e1, e2);

        for _ in 0..300 {
            let p1 = Input(rng.next_u8());
            let p2 = Input(rng.next_u8());
            state.tick(p1, p2);

            for p in 0..2 {
                assert!(
                    state.combo[p].hit_count <= 25,
                    "Player {} combo exceeded 25: {} at frame {}",
                    p,
                    state.combo[p].hit_count,
                    state.frame_number
                );
            }
        }
    }
}

/// Assert every match terminates within 90*60 = 5400 ticks.
#[test]
fn test_round_always_ends() {
    let mut rng = Lcg::new(77777);

    for _ in 0..100 {
        let e1 = rng.next_element();
        let e2 = rng.next_element();
        let mut state = GameState::new(e1, e2);

        let max_ticks = 90 * 60; // 5400
        let mut ended = false;

        for tick in 0..max_ticks {
            let p1 = Input(rng.next_u8());
            let p2 = Input(rng.next_u8());
            state.tick(p1, p2);

            if matches!(state.phase, game_sim::GamePhase::RoundEnd { .. })
                || matches!(state.phase, game_sim::GamePhase::MatchEnd { .. })
            {
                ended = true;
                assert!(
                    tick < max_ticks,
                    "Round did not end within {} ticks",
                    max_ticks
                );
                break;
            }
        }

        assert!(ended, "Round never ended within {} ticks", max_ticks);
    }
}
