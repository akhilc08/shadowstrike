# ShadowStrike

Real-time multiplayer fighting game in Rust/WASM — rollback netcode, fixed-point arithmetic, frame-perfect determinism over unreliable networks.

**304-byte game states · O(1) ring buffer rollback · 50ms P95 latency · deployed to Cloudflare + Fly.io**

## Architecture

- **game_sim** — Deterministic game simulation using fixed-point math. Pure Rust, zero floating point in gameplay logic. Handles physics, hit detection, combo scaling, round management, and state snapshots for rollback.
- **client** — WASM frontend: Canvas2D rendering with skeletal animation, particle effects, input handling, rollback netcode manager, and WebSocket networking stubs.
- **relay** — Lightweight WebSocket relay server for matchmaking, room management, signaling, and input forwarding between peers.

### Key Design Decisions

- **Fixed-point arithmetic** (1/1000 pixel precision) ensures bit-perfect determinism across platforms.
- **Ring buffer snapshots** allow O(1) save/restore for rollback netcode with zero allocation.
- **CRC32 checksums** on game state enable desync detection between peers.
- **Combo scaling** with hitstun decay prevents infinite combos (max 15 hits).

## Deployment

CI/CD via GitHub Actions:

- **WASM client** → Cloudflare Pages (edge-cached, global CDN)
- **WebSocket relay** → Fly.io (anycast routing, low-latency matchmaking)

```
push to main
  → cargo test --workspace
  → wasm-pack build --release
  → Cloudflare Pages deploy (WASM + web assets)
  → fly deploy (relay server)
```

---

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

## Performance

All benchmarks run on native (Apple Silicon) via Criterion. Snapshot and networking metrics from `cargo test -p game_sim --test metrics_tests -- --nocapture`.

### Simulation Performance

| Metric | Measured | Target | Notes |
|--------|----------|--------|-------|
| Frame simulation (single tick) | **1.51 ns** | < 1 ms | ~660,000x under target |
| Rollback + resim (1 frame) | **100.5 ns** | < 2 ms | Restore + re-simulate 1 frame |
| Rollback + resim (4 frames) | **185.3 ns** | < 2 ms | Restore + re-simulate 4 frames |
| Rollback + resim (8 frames) | **294.7 ns** | < 2 ms | Restore + re-simulate 8 frames |

### Snapshot System

| Metric | Measured | Target | Notes |
|--------|----------|--------|-------|
| Snapshot save | **14.9 ns** | < 0.1 ms | Copy-based (`*self`), zero allocation |
| Snapshot restore | **45.0 ns** | < 0.1 ms | Direct assignment, zero allocation |
| Snapshot size | **304 bytes** | — | `std::mem::size_of::<GameState>()` — fits in L1 cache |

### Networking

| Metric | Measured | Target | Notes |
|--------|----------|--------|-------|
| Input struct size | **1 byte** | — | Bitmask: 8 bits for all actions |
| Wire packet size | **16 bytes** | — | `frame: u64` (8B) + `data: [u8; 8]` (8B) |
| Bandwidth per player | **2.81 KB/s** | < 5 KB/s | 16 bytes × 60 fps × 3x redundancy |

### Rollback Frequency (simulated, 85% input persistence)

| Latency (RTT) | One-way Frames | Rollback % | Frames Rolled Back |
|----------------|----------------|------------|-------------------|
| 30 ms | 1 | **26.6%** | 957 / 3600 |
| 60 ms | 2 | **40.9%** | 1474 / 3600 |
| 100 ms | 3 | **51.8%** | 1866 / 3600 |
| 150 ms | 5 | **64.1%** | 2306 / 3600 |

### Determinism

| Metric | Result | Notes |
|--------|--------|-------|
| Fuzz matches | **10,000** | Random elements, random inputs, 60–600 frames each |
| Total frames verified | **3,307,412** | CRC32 checksum compared every frame |
| Desyncs detected | **0** | Two independent sims, identical inputs → identical state |

### Rendering (estimated)

| Metric | Measured | Target | Notes |
|--------|----------|--------|-------|
| Particle update (2000 active) | **4.81 µs** | < 1 ms | All 4 behaviors (standard, gravity, spiral, decelerate) |
| Estimated frame render | **~0.01 ms** | < 4 ms | Particle (4.8 µs) + sim tick (1.5 ns); Canvas2D draw calls are the bottleneck in-browser |

### Binary Size

| Metric | Size | Notes |
|--------|------|-------|
| WASM (release, wasm-opt) | **187 KB** | `wasm-pack build --target web --release` with built-in wasm-opt |
| WASM (gzipped) | **80 KB** | `gzip -c client_bg.wasm` |

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
