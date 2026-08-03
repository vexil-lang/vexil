# Cross-Language Interop: One Fixture, Four Targets

Rust, TypeScript, Go, and Python generate one `SensorReading`, encode it, decode
it, and report the schema hash and wire bytes. The repository runner compares
the four results byte for byte.

This is a focused interoperability example, not proof for every schema shape or
deployment environment.

## Prerequisites

- Rust 1.94 or later
- Node.js 22.12 or later
- Go 1.22 or later
- Python 3.10 or later

## Run

From the repository root:

```sh
python scripts/examples.py check cross-language
```

Expected result:

```text
cross-language: Rust, TypeScript, Go, and Python agree
```

## What is compared

Every target uses [`schema/telemetry.vexil`](./schema/telemetry.vexil) and the
same fixture: sub-byte battery and signal values, an enum, floating-point
readings, a label, and optional coordinates. Each program prints three
machine-readable lines:

```text
schema=<64 hexadecimal characters>
wire=<hexadecimal payload>
round-trip=ok
```

The runner requires identical schema and wire values across all four targets.

## Regenerate

```sh
python scripts/examples.py regenerate cross-language
python scripts/examples.py check cross-language
```

Continue with [live telemetry](../live-telemetry/) for a stateful browser stream.
