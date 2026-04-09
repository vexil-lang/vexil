# Hacker News Submission

## Title Options (pick one)

**Option A (recommended):**
> Show HN: Vexil -- Schema language where u4 means exactly 4 bits on the wire

**Option B:**
> Show HN: Vexil -- Define binary protocols once, generate Rust/TypeScript/Go with identical bytes

**Option C:**
> Show HN: Vexil -- Typed schema language with sub-byte types and BLAKE3 schema hashing

## URL

`https://github.com/vexil-lang/vexil`

(Or link directly to the blog post if published on a standalone URL)

---

## First Comment (post immediately after submission)

Hi HN, I built Vexil because I kept hand-rolling binary encoders across Rust and TypeScript and getting bitten by subtle bit-alignment bugs.

Most schema languages describe the shape of data — "this field is a 32-bit integer." Vexil describes the encoding too — "this field is 4 bits, LSB-first, packed with its neighbors." The type `u4` means exactly 4 bits on the wire. The annotation `@varint` switches to LEB128. Union tags can be as small as 3 bits.

**What makes it different from Protobuf:**

- Sub-byte types (`u1`..`u63`) — your 4-bit channel ID is actually 4 bits, not a 32-bit varint
- Schema hashing — BLAKE3 hash of the canonical schema is embedded in generated code. Sender/receiver mismatch is detected before decoding, not after corruption
- Deterministic encoding — same input always produces same bytes (Protobuf can't guarantee this due to map iteration order)
- Measurable wire efficiency — the vexmon demo streams a full system monitor at ~300 B/s vs ~3,600 B/s JSON equivalent (92% savings)

**To see it in action:** `cargo install vexmon && vexmon` — opens a real-time system monitor dashboard at localhost:3000 that shows CPU (per-core), memory, disk, network, and top 50 processes, all streamed over a single WebSocket using Vexil's binary format. The wire stats are displayed live in the header. Source + schema: https://github.com/vexil-lang/vexmon

**What it doesn't do (honest trade-offs):**

- Pre-1.0 — wire format may change. Schema hash is the safety net.
- Three backends (Rust, TypeScript, Go) — Protobuf has dozens. The `CodegenBackend` trait makes adding new ones straightforward.
- No zero-copy — if decode latency is your constraint, Cap'n Proto is a better choice.
- No self-describing format — both sides need the schema.

**What's in the repo:**

- Formal language spec (14 sections + PEG grammar)
- 83-file conformance corpus (27 valid, 56 invalid)
- Cross-language compliance vectors (JSON golden bytes verified across Rust, TS, Go)
- vexmon showcase + 4 more examples
- 30-page documentation book

The spec came first, then the grammar, then the corpus, then the implementation. I wanted something I could prove correct, not just something that works on the happy path.

`cargo install vexilc` to try the compiler, or `cargo install vexmon` to see a working app. Happy to answer questions about the wire format design, the union multiplexing approach, or why I decided to build this instead of extending Protobuf.

---

## Timing Notes

- Best submission times for HN: weekday mornings US Eastern (9-11 AM ET)
- Tuesday through Thursday tend to perform best
- Avoid Fridays and weekends for technical content
- Be online and responsive for the first 2 hours after posting — HN rewards active, thoughtful engagement

## Anticipated Questions & Responses

**"Why not just extend Protobuf?"**
> Protobuf's wire format is fundamentally varint-based. You can't retrofit sub-byte bit packing into it without breaking backward compatibility. The encoding semantics need to be in the type system from the start, not bolted on.

**"How does this compare to ASN.1 PER?"**
> ASN.1 PER does support bit-level packing, but ASN.1's type system is enormous and the tooling ecosystem is fragmented. Vexil is deliberately smaller in scope — six declaration kinds, one wire format, deterministic encoding. The tradeoff is expressiveness for simplicity.

**"What about schema evolution?"**
> Partial today. Decoders tolerate trailing bytes, so you can append fields. The `@removed` annotation prevents ordinal reuse. The BLAKE3 hash detects any divergence. Formal evolution rules (backward/forward compatibility guarantees) are planned but not shipped.

**"Why BLAKE3 and not SHA-256?"**
> Speed. BLAKE3 is an order of magnitude faster than SHA-256 for small inputs, and schema canonical forms are small. The hash is computed at compile time, so it doesn't affect runtime performance, but faster hashing means faster CI. It's also a single dependency (the `blake3` crate), not an OS-level crypto library.

**"Is this production-ready?"**
> It's pre-1.0, so the wire format isn't frozen. But the implementation has 436+ tests, an 83-file conformance corpus, cross-language compliance vectors, and no `unwrap()` in production code. If you pin your `vexilc` version and use the schema hash to detect changes, you can use it in production with eyes open about the stability guarantee.
