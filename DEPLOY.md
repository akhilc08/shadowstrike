# Deploying ShadowStrike

## Relay Server (Fly.io)

The relay server handles WebSocket signaling and input relay for online multiplayer.

### Prerequisites

- [flyctl](https://fly.io/docs/hands-on/install-flyctl/) installed and authenticated (`fly auth login`)

### First-time setup

```bash
# Create the app (only needed once)
fly apps create shadowstrike-relay

# Deploy
fly deploy
```

### Subsequent deploys

```bash
fly deploy
```

### Verify

```bash
# Check status
fly status

# View logs
fly logs

# Check the relay is accepting WebSocket connections
# The relay listens on wss://shadowstrike-relay.fly.dev
```

### Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `9001` | TCP port the relay binds to (set in fly.toml) |

### Scaling

```bash
# Scale to multiple regions for lower latency
fly regions add lax ams nrt

# Scale machine count
fly scale count 2
```

## Client (Static Site)

The client is a static site (HTML + WASM). Serve the `web/` directory from any static host.

### Build WASM

```bash
# Install wasm-pack if needed
cargo install wasm-pack

# Build the client WASM bundle
cd crates/client
wasm-pack build --target web --out-dir ../../web/pkg
```

### Relay URL

The client auto-detects the relay URL:
- **localhost/127.0.0.1**: connects to `ws://localhost:9001` (local dev)
- **Any other host**: connects to `wss://shadowstrike-relay.fly.dev` (production)

To override, edit the `RELAY_WS` constant in `web/index.html`.

### Hosting options

- **GitHub Pages**: push `web/` to a `gh-pages` branch
- **Netlify/Vercel**: point build output to `web/`
- **Any static server**: `cd web && python3 -m http.server 8080`

## Local Development

```bash
# Terminal 1: Run relay locally
cargo run -p relay

# Terminal 2: Build WASM and serve
cd crates/client && wasm-pack build --target web --out-dir ../../web/pkg
cd ../../web && python3 -m http.server 8080

# Open http://localhost:8080
```
