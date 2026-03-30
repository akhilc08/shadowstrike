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

/// Print the size of GameState in bytes for snapshot size measurement.
#[test]
fn test_snapshot_size_bytes() {
    let size = std::mem::size_of::<GameState>();
    println!("\n=== SNAPSHOT SIZE ===");
    println!("GameState size: {} bytes", size);
    println!("=====================\n");
    // Sanity: GameState should be reasonably small (under 1KB)
    assert!(size < 1024, "GameState unexpectedly large: {} bytes", size);
}

/// Measure serialized input packet size.
/// The network protocol sends: frame (u64 = 8 bytes) + input data ([u8; 8] = 8 bytes) = 16 bytes.
/// But actual per-player input is just Input(u8) = 1 byte.
#[test]
fn test_input_packet_size() {
    let input_size = std::mem::size_of::<Input>();
    println!("\n=== INPUT PACKET SIZE ===");
    println!("Input struct size: {} byte(s)", input_size);

    // The wire format from relay protocol: frame: u64 (8 bytes) + data: [u8; 8] (8 bytes)
    let wire_packet_size: usize = 8 + 8; // frame + data array
    println!("Wire packet size (binary): {} bytes", wire_packet_size);

    // Bandwidth calculation: packet_size * 60fps * 3x redundancy
    let bandwidth_per_second = wire_packet_size as f64 * 60.0 * 3.0;
    println!(
        "Bandwidth per player: {:.2} KB/s ({} bytes × 60fps × 3x redundancy)",
        bandwidth_per_second / 1024.0,
        wire_packet_size
    );
    println!("==========================\n");

    assert_eq!(input_size, 1, "Input should be exactly 1 byte");
}

/// Simulate rollback frequency at different artificial latencies.
/// Models realistic gameplay where players hold inputs for several frames
/// (input persistence ~85%) rather than random-mashing every frame.
#[test]
fn test_rollback_frequency_simulation() {
    println!("\n=== ROLLBACK FREQUENCY SIMULATION ===");

    let latencies_ms = [30, 60, 100, 150];
    let fps: usize = 60;

    for &latency_ms in &latencies_ms {
        // One-way latency in frames
        let one_way_frames = (latency_ms as f64 / 2.0 / (1000.0 / fps as f64)).ceil() as usize;

        let mut rng = Lcg::new(latency_ms as u32 * 31337);
        let total_frames: usize = 3600; // 60 seconds at 60fps

        // Generate realistic inputs: ~85% chance of repeating previous input
        let mut remote_inputs: Vec<u8> = Vec::with_capacity(total_frames);
        let mut current_input: u8 = 0;
        for _ in 0..total_frames {
            // 15% chance of changing input each frame (realistic gameplay)
            if rng.next() % 100 < 15 {
                current_input = rng.next_u8();
            }
            remote_inputs.push(current_input);
        }

        let mut rollback_count: usize = 0;
        let mut last_confirmed_input: u8 = 0;

        for frame in 0..total_frames {
            // Prediction: repeat last confirmed input
            let predicted = last_confirmed_input;

            // The actual input for this frame
            let actual = remote_inputs[frame];

            // Update confirmed input when it arrives (delayed by one_way_frames)
            if frame >= one_way_frames {
                last_confirmed_input = remote_inputs[frame - one_way_frames];
            }

            // Rollback needed if prediction was wrong
            if predicted != actual {
                rollback_count += 1;
            }
        }

        let rollback_pct = rollback_count as f64 / total_frames as f64 * 100.0;
        println!(
            "Latency: {}ms ({}f one-way) → {:.1}% frames rollback ({}/{})",
            latency_ms, one_way_frames, rollback_pct, rollback_count, total_frames
        );
    }

    println!("======================================\n");
}

/// Run 10,000 matches with random inputs using two independent GameState instances.
/// Verify CRC32 checksums match every frame — zero desyncs allowed.
#[test]
fn test_determinism_10k_matches() {
    let mut rng = Lcg::new(0xDEAD_BEEF);
    let mut total_frames: u64 = 0;
    let match_count = 10_000;

    for _ in 0..match_count {
        let e1 = rng.next_element();
        let e2 = rng.next_element();
        let mut state_a = GameState::new(e1, e2);
        let mut state_b = GameState::new(e1, e2);

        let num_frames = 60 + (rng.next() % 541) as usize; // 60-600 frames
        for _ in 0..num_frames {
            let p1 = Input(rng.next_u8());
            let p2 = Input(rng.next_u8());

            state_a.tick(p1, p2);
            state_b.tick(p1, p2);

            assert_eq!(
                state_a.checksum(),
                state_b.checksum(),
                "Desync at frame {} in match with {:?} vs {:?}",
                state_a.frame_number,
                e1,
                e2
            );
        }
        total_frames += num_frames as u64;
    }

    println!("\n=== DETERMINISM FUZZ RESULTS ===");
    println!(
        "Matches: {} | Total frames: {} | Desyncs: 0",
        match_count, total_frames
    );
    println!("=================================\n");
}
