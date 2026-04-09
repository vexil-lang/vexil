# Social Launch Content

---

## Twitter/X Thread

### Tweet 1 (Hook)
```
I've been building a schema language where `u4` means exactly 4 bits on the wire.

Today I'm releasing Vexil — define your binary protocol once, generate Rust, TypeScript, and Go code that produces identical bytes.

Open source. MIT/Apache-2.0.

github.com/vexil-lang/vexil

Thread -->
```

### Tweet 2 (Problem)
```
The problem: Protobuf doesn't do sub-byte types. Your 4-bit channel ID becomes a 32-bit varint. Cap'n Proto wastes bandwidth on alignment padding.

If your constraint is wire size — embedded, IoT, streaming — you end up hand-rolling bit packing in every language. And the encoders diverge.
```

### Tweet 3 (Solution)
```
Vexil puts encoding in the type system:

  channel @0 : u4           # 4 bits
  sequence @1 : u32 @varint # LEB128
  delta_ts @2 : i32 @zigzag # ZigZag signed

Run `vexilc codegen` → get Rust structs with Pack/Unpack, TypeScript encode/decode, or Go structs. Same bytes.
```

### Tweet 4 (Schema hash)
```
Every schema gets a BLAKE3 hash, embedded in generated code at compile time.

If sender and receiver have different schemas, you know before decoding — not after silent corruption.

No more "why is field 7 shifted by 3 bits in production."
```

### Tweet 5 (vexmon demo)
```
To prove it works, I built vexmon — a real-time system monitor dashboard.

Rust backend streams CPU, memory, disk, network, and processes to a browser over WebSocket using Vexil binary format.

~300 bytes/second. JSON equivalent: ~3,600 B/s. 92% savings.

cargo install vexmon

github.com/vexil-lang/vexmon
```

### Tweet 6 (Honest limitations)
```
What Vexil doesn't do:

- Not 1.0 yet (wire format may change)
- 3 language targets (Rust, TS, Go) — not Protobuf's dozen
- No zero-copy decode
- No self-describing format

If those are your constraints, use Protobuf or Cap'n Proto. They're great tools.
```

### Tweet 7 (CTA)
```
What's in the repo:

- Formal spec + PEG grammar
- 83-file conformance corpus
- Cross-language compliance vectors
- CLI with check, codegen, build, watch, hash, compat
- 5 working examples
- 30-page docs

cargo install vexilc

Feedback welcome: github.com/vexil-lang/vexil
```

---

## Reddit: r/rust

### Title
```
[ANN] Vexil — a schema language with sub-byte types and first-class encoding semantics (Rust + TypeScript + Go codegen)
```

### Body
```
I've been working on Vexil, a typed schema definition language where encoding is part of the type system. The type `u4` means exactly 4 bits on the wire. The annotation `@varint` means LEB128. The annotation `@delta` generates stateful encoder/decoder pairs for streaming.

**Why I built it:** I kept hand-rolling binary encoders across Rust and TypeScript and getting bitten by bit-alignment bugs. Protobuf doesn't support sub-byte types. Cap'n Proto wastes bandwidth on alignment. I wanted a schema language where the encoding is declarative and the generated code is provably identical across languages.

**What's different:**

- Sub-byte types: `u1`..`u63`, packed LSB-first
- BLAKE3 schema hash: embedded in generated code, catches sender/receiver mismatch before corruption
- Deterministic encoding: same input → same bytes (Protobuf can't guarantee this)
- Delta encoding: `@delta` on a message generates stateful codecs for streaming
- Compliance vectors: published JSON golden bytes, verified identical across Rust, TypeScript, and Go

**See it in action — vexmon:**

`cargo install vexmon && vexmon` — opens a real-time system monitor at localhost:3000. CPU per-core, memory, disk, network, top 50 processes — all streamed over one WebSocket at ~300 B/s (vs ~3,600 B/s JSON). Wire stats displayed live. Source: https://github.com/vexil-lang/vexmon

**What's in the release:**

- `vexilc` CLI (check, codegen, build, watch, hash, compat) — pre-built binaries on GitHub Releases
- Rust codegen with `vexil-runtime` (Pack/Unpack traits, BitWriter/BitReader)
- TypeScript codegen with `@vexil-lang/runtime` on npm
- Go codegen with Go runtime
- Formal language spec (14 sections + PEG grammar)
- 83-file conformance corpus, 436+ tests
- vexmon showcase + 4 more examples

**Honest limitations:**

- Pre-1.0 — wire format may change (schema hash is the safety net)
- 3 language backends — Rust, TypeScript, Go. More planned.
- No zero-copy decode — optimized for wire size, not decode latency
- Partial schema evolution — field appending works, formal rules coming

GitHub: https://github.com/vexil-lang/vexil
Crate: https://crates.io/crates/vexilc

I'd especially love feedback from anyone in the embedded/IoT space or anyone dealing with cross-language binary protocols. What did I get wrong? What's missing?

`cargo install vexilc` to try it.
```

---

## Reddit: r/programming

### Title
```
Vexil: A schema language where `u4` means exactly 4 bits on the wire — cross-language codegen for Rust, TypeScript, and Go
```

### Body
```
Vexil is a typed schema definition language with first-class encoding semantics. Instead of just describing the shape of data (like Protobuf), it describes the encoding too — sub-byte types, varint annotations, union multiplexing, and a BLAKE3 schema hash for mismatch detection.

Quick comparison with Protobuf/Cap'n Proto/FlatBuffers: https://github.com/vexil-lang/vexil#comparison

The key differentiator is sub-byte types. When you write `u4` in Vexil, the field is exactly 4 bits on the wire, packed LSB-first. No other schema language supports this. It matters for embedded protocols, sensor networks, and bandwidth-constrained links where every bit counts.

The compiler generates Rust, TypeScript, and Go code from a single schema. All three produce byte-identical output, verified by published compliance vectors (JSON golden bytes in the repo).

**Try it live:** `cargo install vexmon && vexmon` — a real-time system monitor dashboard that streams CPU, memory, disk, network, and processes at ~300 B/s over WebSocket (vs ~3,600 B/s JSON). Source: https://github.com/vexil-lang/vexmon

It's pre-1.0, open source (MIT/Apache-2.0), and looking for early adopter feedback.

GitHub: https://github.com/vexil-lang/vexil
```

---

## Reddit: r/embedded

### Title
```
Vexil — schema language with sub-byte types (u1..u63) for bandwidth-constrained binary protocols
```

### Body
```
I've been building Vexil, a schema language designed for the kinds of binary protocols common in embedded systems — where you need 4-bit field IDs, 7-bit battery levels, and tight bit packing.

The type `u4` means exactly 4 bits on the wire. `u7` means 7 bits. The compiler packs them LSB-first and generates Rust code with `Pack`/`Unpack` traits. You can also generate TypeScript (for dashboards) and Go (for gateways) from the same schema — all three produce identical bytes, verified by compliance vectors.

Other features relevant to embedded:
- `@varint` / `@zigzag` annotations for variable-length encoding
- Union multiplexing with 3-bit tags — different message types at different intervals over one connection
- BLAKE3 schema hash — embedded in generated code, detects firmware/server schema mismatch at connection time
- Deterministic encoding — same data always produces same bytes
- No runtime allocation in the Rust codegen path

To see the wire efficiency in action: `cargo install vexmon` — a system monitor dashboard that streams full telemetry at ~300 B/s (92% smaller than JSON). Source: https://github.com/vexil-lang/vexmon

Example sensor schema:

    message SensorReading {
        channel  @0 : u4
        kind     @1 : SensorKind
        value    @2 : u16
        sequence @3 : u32 @varint
        delta_ts @4 : i32 @zigzag
    }

GitHub: https://github.com/vexil-lang/vexil

Pre-1.0, MIT/Apache-2.0. I'd love feedback from anyone who's currently hand-rolling binary encoders for embedded protocols.
```

---

## LinkedIn Post (if applicable)

```
I've been working on something for the past year and I'm releasing it today.

Vexil is a schema language for binary protocols — the kind where every bit matters.

Most serialization formats describe the *shape* of data. Vexil describes the *encoding* too. The type `u4` means exactly 4 bits on the wire. The annotation `@delta` generates stateful encoder/decoder pairs that transmit field-level diffs for streaming.

Define your protocol once. Generate Rust, TypeScript, and Go code that produces identical bytes — verified by published compliance vectors.

It's open source, pre-1.0, and built for teams working on:
- Embedded/IoT protocols
- Cross-language wire formats
- Real-time streaming with bandwidth constraints
- Deterministic encoding for content addressing

To see it in action: cargo install vexmon — a system monitor that streams full telemetry at ~300 B/s (92% less than JSON). github.com/vexil-lang/vexmon

Language + compiler: github.com/vexil-lang/vexil

Feedback welcome, especially from anyone hand-rolling binary encoders today.
```

---

## Posting Sequence

| Day | Channel | Action |
|-----|---------|--------|
| Launch Day, AM | Twitter/X | Post full thread |
| Launch Day, AM | Hacker News | Submit Show HN + first comment |
| Launch Day, +2h | r/rust | Post announcement |
| Launch Day, +4h | r/programming | Post announcement |
| Launch Day +1 | r/embedded | Post with embedded-specific angle |
| Launch Day +1 | LinkedIn | Post for professional network |
| Launch Day +2 | This Week in Rust | Submit for next newsletter |
| Launch Day +3 | Dev.to | Cross-post blog with TypeScript angle |
