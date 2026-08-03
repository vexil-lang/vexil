# Live Telemetry: Stateful Delta Frames

This is the flagship example. A Rust service samples local CPU and memory data,
encodes it with a generated stateful delta codec, and streams binary WebSocket
frames to a browser decoder generated from the same schema.

## Prerequisites

- Rust 1.94 or later
- Node.js 22.12 or later

## Run

From the repository root:

```sh
cd examples/live-telemetry
npm ci
npm run bundle
cargo run --release
```

Open <http://127.0.0.1:3000>. The server binds only to the loopback interface.

## Verify without a browser

```sh
python scripts/examples.py check live-telemetry
```

The headless check regenerates both codecs, verifies the tracked browser bundle,
and exercises several frames plus an encoder reset. It does not bind a port or
read host metrics.

## How the path fits together

1. [`schema/telemetry.vexil`](./schema/telemetry.vexil) marks
   `SystemSnapshot` with `@delta`.
2. The Rust `SystemSnapshotEncoder` retains the previous frame.
3. The browser `SystemSnapshotDecoder` applies the matching state transition.
4. Both sides exchange the generated schema hash before telemetry starts.
5. Reconnect resets the decoder so the next full frame establishes a new base.

Delta encoding is stateful representation, not general-purpose compression.
The actual frame size depends on the sampled values and core count.

## Regenerate

```sh
python scripts/examples.py regenerate live-telemetry
python scripts/examples.py check live-telemetry
```

Return to the [example index](../) to compare prerequisites and outcomes.
