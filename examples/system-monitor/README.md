# System Monitor — Live Vexil Delta Streaming Demo

Real-time PC resource monitoring over Vexil bitpack binary WebSocket
with `@delta` encoding for reduced wire size on steady-state frames.

**Rust backend** reads actual CPU and memory usage, delta-encodes via
the generated `SystemSnapshotEncoder`, and sends compact binary frames
over WebSocket every second.

**Browser dashboard** decodes the binary frame using the generated
`SystemSnapshotDecoder` (bundled via esbuild from `@vexil-lang/runtime`)
and renders live CPU bars, memory usage, and per-core utilization.

## @delta encoding

The `@delta` annotation on `SystemSnapshot` causes the code generator
to emit stateful encoder/decoder pairs. Numeric fields (`i64`, `u8`,
`u32`) are transmitted as deltas from the previous frame. Non-numeric
fields (`string`, `array`, `enum`) are sent in full each frame.

Wire size improvement:
- **First frame:** ~42 bytes (full snapshot, same as without `@delta`)
- **Steady state:** ~25-30 bytes (deltas are small, many zero)

Both encoder and decoder track previous field values. On reconnect the
decoder calls `reset()` to re-synchronize with the encoder's first
(full) frame.

## Building

Prerequisites: Node.js (for esbuild), Rust toolchain.

```bash
cd examples/system-monitor

# Install JS dependencies and bundle the TypeScript decoder
npm install
npm run bundle

# Build and run the Rust server
cargo run --release
```

Open http://127.0.0.1:3000 in your browser.

## Schema

See `schema/telemetry.vexil` — defines `SystemSnapshot` with `@delta`,
CPU usage, per-core array, memory stats, hostname, and a `CpuStatus` enum.

## How it works

1. Rust reads system metrics via `sysinfo` crate
2. Builds a `SystemSnapshot` struct (generated from schema by `vexilc`)
3. Delta-encodes with `SystemSnapshotEncoder` into compact binary
4. Sends as binary WebSocket frame via `axum`
5. Browser receives binary, delta-decodes with generated `SystemSnapshotDecoder`
6. Renders live dashboard with smooth CSS transitions

The `bundle.js` is embedded into the Rust binary via `include_str!` —
no filesystem access needed at runtime.

## Regenerating code

```bash
# Rust (server-side encoder)
cargo run -p vexilc -- codegen schema/telemetry.vexil --target rust > src/generated.rs

# TypeScript (browser decoder) + bundle
npm run build
```
