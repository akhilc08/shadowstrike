# ShadowStrike

A browser-based 2D fighting game built in Rust/WASM with rollback netcode.

## Architecture

- **game_sim** — Deterministic game simulation using fixed-point math. Pure Rust, no_std compatible, zero floating point in gameplay logic.
- **client** — WASM frontend: rendering, input handling, audio, and rollback netcode client.
- **relay** — Lightweight WebSocket relay server for matchmaking and packet forwarding.

## Building

```bash
cargo build
cargo test -p game_sim
```
