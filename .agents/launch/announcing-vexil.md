# Introducing Vexil: A Schema Language Where Encoding Is Part of the Type

Most schema languages describe the *shape* of your data. Vexil describes the *encoding* too.

When you write `u4` in Vexil, that field occupies exactly 4 bits on the wire. When you annotate a field with `@varint`, it encodes as LEB128. When you mark a message `@delta`, the generated code transmits field-level diffs instead of full values. The schema isn't just a shape contract — it's the wire contract.

I've been building Vexil for the past year, and today I'm releasing it publicly: a typed schema definition language with a Rust compiler, three code generation backends (Rust, TypeScript, Go), a CLI, a formal specification, and a conformance corpus. It's MIT/Apache-2.0, pre-1.0, and ready for early adopters.

## The problem

If you've ever built a binary protocol that crosses language boundaries, you've lived this:

1. Define the protocol in a document or a header file
2. Hand-write an encoder in Rust
3. Hand-write a decoder in TypeScript
4. Spend a week debugging why byte 14 is off by one because you packed a 4-bit field on the wrong boundary
5. Add Go to the stack. Repeat step 4.

Protocol Buffers solves cross-language serialization, but it doesn't give you sub-byte types. Your `u4` becomes a `uint32` on the wire — 28 wasted bits. For sensor networks, embedded telemetry, or bandwidth-constrained links, that waste adds up.

Cap'n Proto and FlatBuffers optimize for zero-copy access, but their alignment padding trades bandwidth for decode speed. If your constraint is wire size, not decode latency, they're optimizing the wrong axis.

And none of them hash the schema. If your sender compiles against version 3 and your receiver against version 2, you get silent data corruption. No error. Just wrong values.

## What Vexil looks like

Here's a schema for an IoT sensor packet:

```vexil
namespace sensor.packet

enum SensorKind : u8 {
    Temperature @0
    Humidity    @1
    Pressure    @2
    Light       @3
}

message SensorReading {
    channel  @0 : u4              # exactly 4 bits
    kind     @1 : SensorKind
    value    @2 : u16
    sequence @3 : u32 @varint     # LEB128 variable-length
    delta_ts @4 : i32 @zigzag    # ZigZag signed encoding
}

message TelemetryPacket {
    device_id @0 : u32
    readings  @1 : array<SensorReading>
    battery   @2 : u7             # 7 bits, values 0..127
}
```

Run `vexilc codegen sensor.vexil --output sensor.rs` and you get a Rust module with `SensorReading`, `TelemetryPacket`, and their `Pack`/`Unpack` implementations. Run it again with `--target typescript` and you get TypeScript encode/decode functions that produce *identical bytes*.

```rust
use vexil_runtime::{BitWriter, BitReader, Pack, Unpack};

let reading = SensorReading {
    channel: 0, kind: SensorKind::Temperature,
    value: 2350, sequence: 1, delta_ts: -50,
};

let mut w = BitWriter::new();
reading.pack(&mut w).unwrap();
let bytes = w.finish();   // compact, bit-packed

let mut r = BitReader::new(&bytes);
let decoded = SensorReading::unpack(&mut r).unwrap();
assert_eq!(decoded.value, 2350);
```

```typescript
import { BitWriter, BitReader } from '@vexil-lang/runtime';

const w = new BitWriter();
encodeSensorReading({
  channel: 0, kind: 'Temperature',
  value: 2350, sequence: 1, delta_ts: -50,
}, w);
const bytes = w.finish();  // identical bytes as Rust

const r = new BitReader(bytes);
const decoded = decodeSensorReading(r);
// decoded.value === 2350
```

Same schema. Same bytes. Different languages.

## Three things that set Vexil apart

### 1. Sub-byte types are first-class

Vexil supports integer types from `u1` to `u63` and `i2` to `i63`. A `u4` occupies exactly 4 bits on the wire, packed LSB-first with its neighbors. No other schema language does this.

This matters for embedded protocols, CAN bus replacements, sensor telemetry — anywhere you're packing data into constrained bandwidth. In Protobuf, your 4-bit channel ID becomes a 32-bit varint. In Vexil, it's 4 bits.

### 2. BLAKE3 schema hash catches mismatches before corruption

Every Vexil schema has a canonical text form. The compiler hashes it with BLAKE3 and embeds the hash as a compile-time constant in the generated code.

When a sender and receiver connect, they can compare schema hashes. If they don't match, you know *before decoding any data* that the schemas have diverged. No silent corruption. No mysterious field shifts.

This is particularly valuable in systems with independent deploy cycles — firmware devices in the field, microservices with staggered rollouts, or any architecture where "both sides updated at the same time" is a fantasy.

### 3. Union multiplexing and bandwidth efficiency

Vexil unions let you multiplex different message types over a single connection with minimal overhead — the union tag is as small as 3 bits. Combined with sub-byte types and tight packing, this adds up.

To show what this looks like in practice, I built **[vexmon](https://github.com/vexil-lang/vexmon)** — a real-time system monitor that streams CPU, memory, disk, network, and process data from a Rust backend to a browser dashboard over WebSocket using Vexil's binary format.

```vexil
union TelemetryFrame {
    Cpu     @0 : CpuSnapshot       // sent every 1s, ~14 bytes
    Memory  @1 : MemorySnapshot    // sent every 1s, ~42 bytes
    Disk    @2 : DiskInfo          // sent every 5s
    Network @3 : NetworkInfo       // sent every 2s
    Process @4 : ProcessInfo       // sent every 5s
    System  @5 : SystemInfo        // sent once on connect
}

message CpuSnapshot {
    overall   @0 : u8             // 0-100%, exactly 8 bits
    per_core  @1 : array<u8>
    frequency @2 : u16            // MHz
}
```

The result: **~300 bytes/second** of wire traffic for a full system monitor. The JSON equivalent would be ~3,600 B/s. That's a **92% bandwidth reduction** — and the dashboard shows the savings live in the header bar.

Install it with `cargo install vexmon`, open `http://127.0.0.1:3000`, and watch the wire stats in real time. The schema handshake uses BLAKE3 hash verification — if the frontend and backend schemas don't match, you see an error before any data is decoded.

## Deterministic encoding

Vexil's encoding is fully deterministic: the same input always produces the same bytes. This is a property that Protobuf explicitly does not guarantee (map iteration order is undefined) and that Cap'n Proto sacrifices for alignment.

Deterministic encoding enables:
- **Content addressing** — hash the encoded bytes to get a stable content ID
- **Deduplication** — identical messages produce identical bytes
- **Replay detection** — compare encoded frames by equality
- **Audit trails** — byte-identical encoding means reproducible verification

## The comparison

| | Vexil | Protobuf | Cap'n Proto | FlatBuffers |
|---|:---:|:---:|:---:|:---:|
| Sub-byte types (`u1`..`u63`) | Yes | -- | -- | -- |
| Encoding annotations in schema | Yes | -- | -- | -- |
| Schema hash (mismatch detection) | BLAKE3 | -- | -- | -- |
| Deterministic encoding | Yes | No (maps) | No (padding) | No (vtables) |
| Zero-copy decode | No | No | Yes | Yes |
| Self-describing wire format | No | Optional | No | Optional |
| Language targets | 3 | Many | Many | Many |
| Schema evolution | Partial | Yes | Yes | Yes |

Vexil wins on wire compactness and determinism. Protobuf wins on ecosystem breadth. Cap'n Proto and FlatBuffers win on decode latency. Pick the one that matches your constraint.

## See it in action

The fastest way to understand Vexil is to run **vexmon**:

```sh
cargo install vexmon
vexmon
# Open http://127.0.0.1:3000
```

You'll see a live dashboard monitoring your system — CPU per core, memory with swap/cache breakdown, disk usage per mount, network throughput per interface, and a searchable/sortable process table. All streamed over a single WebSocket at ~300 bytes/second.

The wire stats are displayed live in the header: average throughput, peak, JSON equivalent, and savings percentage. It's a proof-of-concept that doubles as a useful tool.

The full source (~1,800 lines including generated code) is at [github.com/vexil-lang/vexmon](https://github.com/vexil-lang/vexmon). The schema is [95 lines](https://github.com/vexil-lang/vexmon/blob/main/schema/telemetry.vexil). Everything else is generated.

## What's in the box

The [v0.5.0 release](https://github.com/vexil-lang/vexil/releases) includes:

- **`vexilc`** — CLI compiler with `check`, `codegen`, `build`, `watch`, `hash`, `compat`, and `init` subcommands. Pre-built binaries for Linux, macOS, and Windows.
- **Rust code generation** — structs, enums, encode/decode with the `vexil-runtime` crate
- **TypeScript code generation** — interfaces, encode/decode with the `@vexil-lang/runtime` npm package
- **Go code generation** — structs, encode/decode with the Go runtime package
- **Language specification** — normative spec with 14 sections, formal PEG grammar
- **Conformance corpus** — 83 test files (27 valid, 56 invalid) that any implementation must handle correctly
- **Compliance vectors** — published golden byte sequences verified across all three language backends
- **[vexmon](https://github.com/vexil-lang/vexmon)** — flagship demo: real-time system monitor dashboard (Rust + TypeScript, ~300 B/s wire traffic)
- **4 more examples** — sensor packet, command protocol, cross-language interop, multi-file project
- **30-page documentation book** — getting started, language guide, CLI reference, runtime APIs

Install the compiler:

```sh
cargo install vexilc
```

Or grab a pre-built binary from [GitHub Releases](https://github.com/vexil-lang/vexil/releases).

## What Vexil is NOT

Being honest about limitations matters more than marketing:

- **Not 1.0 yet.** The wire format may change before v1.0. The schema hash provides a safety net — you'll know immediately if something changed — but if you need wire format stability guarantees today, wait.
- **Not a Protobuf replacement for most teams.** If you need Java, Python, C, Swift, or any of the dozen other languages Protobuf targets, Vexil isn't there yet. The `CodegenBackend` trait makes adding backends straightforward, and contributions are welcome, but today it's Rust, TypeScript, and Go.
- **No zero-copy decode.** If your bottleneck is decode latency rather than wire size, Cap'n Proto or FlatBuffers are better choices.
- **No self-describing format.** Both sides must have the schema. If you need schema-less consumers, Vexil is the wrong tool.

## Who should try Vexil

- You're building a binary protocol for embedded/IoT devices and need sub-byte field packing
- You have a Rust backend and TypeScript frontend (or Go services) sharing a wire protocol
- You want deterministic encoding for content addressing or audit trails
- You're streaming real-time data and want compact delta-encoded frames
- You're tired of hand-rolling encoders that diverge across languages

## What's next

The roadmap includes:
- **LSP support** — editor tooling with diagnostics, completions, and hover info (design spec complete)
- **Package registry** — schema sharing and dependency management
- **More language backends** — Python and C are the most requested
- **Formal schema evolution rules** — field deprecation, backward compatibility guarantees

## Try it

```sh
cargo install vexilc
vexilc init hello.vexil --namespace hello.world
vexilc check hello.vexil
vexilc codegen hello.vexil --output hello.rs
```

The [getting-started guide](https://github.com/vexil-lang/vexil/blob/main/docs/book/src/getting-started/first-schema.md) walks through your first schema in 5 minutes.

GitHub: [github.com/vexil-lang/vexil](https://github.com/vexil-lang/vexil)
Docs: [docs.rs/vexil-lang](https://docs.rs/vexil-lang)
npm: [@vexil-lang/runtime](https://www.npmjs.com/package/@vexil-lang/runtime)

I'd love feedback — especially from anyone working on bandwidth-constrained protocols or cross-language binary formats. File an issue, open a discussion, or just try it and tell me what breaks.
