# System Monitor — Live Vexil Wire Format Demo

Real-time PC resource monitoring over Vexil bitpack binary WebSocket.

**Rust backend** reads actual CPU and memory usage, encodes as a compact
Vexil binary frame, and sends it to the browser over WebSocket every second.

**Browser dashboard** decodes the binary frame using an inline JavaScript
BitReader (matching `@vexil/runtime`) and renders live CPU bars, memory
usage, and per-core utilization.

## Running

```bash
cd examples/system-monitor
cargo run --release
```

Open http://127.0.0.1:3000 in your browser.

## Schema

See `schema/telemetry.vexil` — defines `SystemSnapshot` with CPU usage,
per-core array, memory stats, hostname, and a `CpuStatus` enum.

## How it works

1. Rust reads system metrics via `sysinfo` crate
2. Builds a `SystemSnapshot` struct (generated from schema by `vexilc`)
3. Encodes with `vexil_runtime::Pack` into compact binary (Vexil bitpack)
4. Sends as binary WebSocket frame via `axum`
5. Browser receives binary, decodes with inline BitReader
6. Renders live dashboard with smooth CSS transitions

The wire frame is typically **30-60 bytes** for a full system snapshot
including per-core usage — compared to ~300+ bytes for equivalent JSON.

## Regenerating code

```bash
cargo run -p vexilc -- codegen schema/telemetry.vexil --target rust > src/generated.rs
```
