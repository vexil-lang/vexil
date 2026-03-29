# Real-Time Dashboard

The `examples/system-monitor/` directory demonstrates a real-time system monitoring dashboard using Vexil for the wire protocol between a Rust backend and a browser frontend.

## Architecture

- **Rust backend** collects system metrics (CPU, memory, disk) and encodes them as Vexil messages
- **WebSocket** carries the binary Vexil-encoded frames to the browser
- **Browser frontend** uses the TypeScript runtime to decode and display metrics in real time

## Why Vexil?

Compared to sending JSON over WebSocket:

- Smaller payloads (bit-packed fields, varint encoding)
- Deterministic encoding (no key ordering ambiguity)
- Type safety on both ends from the same schema
- No serialization library needed -- the generated code is the serializer

## Source

[`examples/system-monitor/`](https://github.com/vexil-lang/vexil/tree/main/examples/system-monitor)
