# ShadowStrike

A browser-based 2D fighting game built in Rust/WASM with rollback netcode.

## Architecture

- **game_sim** — Deterministic game simulation using fixed-point math. Pure Rust, zero floating point in gameplay logic. Handles physics, hit detection, combo scaling, round management, and state snapshots for rollback.
- **client** — WASM frontend: Canvas2D rendering with skeletal animation, particle effects, input handling, rollback netcode manager, and WebSocket networking stubs.
- **relay** — Lightweight WebSocket relay server for matchmaking, room management, signaling, and input forwarding between peers.

### Key Design Decisions

- **Fixed-point arithmetic** (1/1000 pixel precision) ensures bit-perfect determinism across platforms.
- **Ring buffer snapshots** allow O(1) save/restore for rollback netcode with zero allocation.
- **CRC32 checksums** on game state enable desync detection between peers.
- **Combo scaling** with hitstun decay prevents infinite combos (max 15 hits).

## Running Locally

### Prerequisites

- Rust toolchain (stable)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- A static file server (e.g., `python3 -m http.server` or `npx serve`)

### Build the WASM Client

```bash
cd crates/client
wasm-pack build --target web --out-dir ../../pkg
```

### Serve

```bash
# From the repo root
python3 -m http.server 8080
# Open http://localhost:8080 in your browser
```

### Run the Relay Server

```bash
cargo run -p relay
# Listens on ws://0.0.0.0:9001 by default
```

## Running Tests

```bash
# All tests (game_sim, client, relay)
cargo test --workspace

# Game simulation tests only
cargo test -p game_sim

# Fuzz / property-based tests
cargo test -p game_sim --test fuzz_tests

# Integration tests
cargo test -p game_sim --test integration
cargo test -p relay --test integration
```

## Running Benchmarks

```bash
cargo bench -p game_sim
```

Benchmarks cover:
- Single tick simulation latency
- Snapshot save/restore cycle
- 8-frame rollback re-simulation

## Clippy & Linting

```bash
cargo clippy --workspace -- -D warnings
```

## Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Tick simulation | < 0.1ms | Single game tick at 60 FPS |
| Snapshot save/restore | < 0.01ms | Copy-based, zero allocation |
| 8-frame rollback | < 1ms | Restore + re-simulate 8 frames |
| Render frame | < 4ms | Canvas2D with skeletal animation + particles |
| Input latency | < 3 frames | Local input to screen update |
| Network rollback | 1-8 frames | Adaptive based on RTT |

## Project Structure

```
crates/
  game_sim/       # Deterministic simulation (no_std compatible)
    src/
      lib.rs        # GameState, tick loop, checksum, snapshot
      player.rs     # PlayerState, actions, hit/hurtboxes
      input.rs      # Input bitmask
      fixed.rs      # Fixed-point arithmetic
      collision.rs  # AABB hit detection
      combo.rs      # Combo state and scaling
      ring_buffer.rs # Generic ring buffer
      constants.rs  # Game tuning constants
    tests/
      integration.rs # Determinism, bounds, snapshot tests
      fuzz_tests.rs  # Property-based fuzz tests
    benches/
      sim_bench.rs   # Criterion benchmarks
  client/          # WASM frontend
    src/
      lib.rs          # ShadowStrike game loop, RollbackManager
      renderer.rs     # Canvas2D rendering
      animation.rs    # Skeletal animation system
      particles.rs    # Particle effects pool
      input_handler.rs # Keyboard input mapping
      networking.rs   # WebSocket/WebRTC networking stubs
  relay/           # WebSocket relay server
    src/
      main.rs       # Server entry point
      server.rs     # WebSocket server
      room.rs       # Room management
      protocol.rs   # Message protocol
```
