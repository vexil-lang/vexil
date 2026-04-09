# Marketing Context — Vexil

> Auto-drafted from codebase on 2026-03-29. Review and correct before using with other marketing skills.

---

## 1. Product Overview

**One-line:** A typed schema definition language with first-class encoding semantics.

**What it does:** Vexil describes both the shape and the wire encoding of data crossing system boundaries. You define a schema once — including sub-byte types like `u4`, encoding annotations like `@varint` and `@delta` — and generate byte-identical code for Rust, TypeScript, and Go. Each schema produces a deterministic BLAKE3 hash so sender/receiver mismatches are detectable before data corruption.

**Product category:** Schema definition language / binary serialization framework (same shelf as Protocol Buffers, Cap'n Proto, FlatBuffers)

**Product type:** Open-source developer tool (MIT OR Apache-2.0)

**Business model:** Open-source, free. No paid tier currently. Revenue model TBD.

---

## 2. Target Audience

**Primary segments:**
- Embedded/IoT engineers building bandwidth-constrained protocols (sensor networks, automotive, industrial)
- Systems engineers defining cross-language wire protocols (Rust backend + TypeScript frontend + Go gateway)
- Infrastructure teams replacing hand-rolled binary encoders with a schema-driven approach
- Real-time streaming developers (WebSocket, UDP) who need compact delta-encoded frames

**Secondary segments:**
- Game networking engineers (bandwidth optimization, deterministic encoding)
- Financial/trading systems (deterministic encoding for content addressing, audit trails)
- Blockchain/crypto developers (deterministic serialization for hashing)

**Decision-makers:** Staff/senior engineers, tech leads, protocol architects

**Primary use case:** Define a compact binary protocol once, generate type-safe encode/decode code for multiple languages, with schema hashing to catch version mismatches.

**Jobs to be done:**
1. "I need to define a binary protocol that works across Rust and TypeScript without hand-rolling encoders in each language"
2. "I need sub-byte field packing for bandwidth-constrained links and I'm tired of writing bitwise operations by hand"
3. "I need deterministic encoding so the same data always produces the same bytes — for content addressing, replay detection, or compliance"

---

## 3. Personas

### Protocol Architect (Champion + User)
- **Cares about:** Wire efficiency, correctness, cross-language interop
- **Challenge:** Hand-rolling binary encoders per language is error-prone and diverges over time
- **Value promise:** Define once, generate everywhere, verify with compliance vectors

### Embedded/IoT Lead (Decision Maker + User)
- **Cares about:** Bandwidth, deterministic behavior, no runtime overhead
- **Challenge:** Protobuf is too heavy for constrained links; custom bitpacking is fragile
- **Value promise:** Sub-byte types and LSB-first packing — the schema IS the wire format

### Full-Stack Tech Lead (Champion)
- **Cares about:** Rust + TypeScript interop, development velocity, correctness
- **Challenge:** Keeping binary serialization in sync across backend and frontend
- **Value promise:** Same schema generates both languages; compliance vectors prove byte-identical output

---

## 4. Problems & Pain Points

**Core challenge:** Defining compact binary protocols that work correctly across multiple languages.

**Why current solutions fall short:**
- **Protobuf:** No sub-byte types, non-deterministic encoding (maps), self-describing overhead
- **Cap'n Proto / FlatBuffers:** No sub-byte types, no schema-level encoding control, padding wastes bandwidth
- **Hand-rolled bitpacking:** Works for one language/platform; diverges across teams, no formal verification
- **`#[repr(packed)]` / C bitfields:** Platform-specific, single-language, no cross-language story

**Cost of the problem:**
- Time: Hand-rolling encoders in 2-3 languages for every protocol change
- Bugs: Subtle byte-order and bit-alignment mismatches that only surface in production
- Bandwidth: Wasted bits from formats that don't support sub-byte fields

**Emotional tension:** "We're one off-by-one bitshift away from a production incident, and nobody wants to touch the encoder code."

---

## 5. Competitive Landscape

### Direct competitors (same solution, same problem)
- **Protocol Buffers** — Dominant schema language, but no sub-byte types, non-deterministic maps, self-describing overhead
- **Cap'n Proto** — Zero-copy focus, but no bit-level packing, alignment padding wastes bandwidth
- **FlatBuffers** — Zero-copy, but vtable overhead, no deterministic encoding

### Secondary competitors (different solution, same problem)
- **ASN.1 / CBOR / MessagePack** — Self-describing formats; heavier on the wire, not schema-first
- **Custom bitpacking code** — The incumbent; works but doesn't scale across languages or teams

### Indirect competitors (conflicting approach)
- **JSON / YAML over HTTP** — "Just use REST"; works until bandwidth or latency matters
- **gRPC** — Builds on Protobuf; brings the same limitations plus HTTP/2 overhead

---

## 6. Differentiation

**Key differentiators:**
1. **Encoding is part of the type system** — `u4` means exactly 4 bits. `@varint` means LEB128. The schema IS the wire contract.
2. **Sub-byte types** — `u1`..`u63` and `i2`..`i63` — no other schema language offers this
3. **Deterministic encoding** — Same input always produces the same bytes. Enables content addressing, deduplication, replay detection.
4. **BLAKE3 schema hash** — Compiled into generated code. Sender/receiver mismatch is detected before data corruption.
5. **Delta encoding** — `@delta` annotation generates stateful encoder/decoder pairs for streaming (25-30 byte steady-state frames vs 42 byte full frames in the system-monitor example)
6. **Cross-language compliance vectors** — Not just "we support multiple languages" but "we prove byte-identical output with published golden vectors"

**Why that's better:** You stop writing encoding logic and start declaring encoding intent. The compiler handles correctness; the compliance vectors prove it.

---

## 7. Objections & Anti-Personas

### Top objections
1. **"It's pre-1.0, the wire format might change"** → Schema hash detects mismatches immediately. Pin your vexilc version for stability. Migration tooling is planned.
2. **"Only 3 language targets — Protobuf has dozens"** → If you need Java/Python/C today, Vexil isn't ready for you. If your stack is Rust + TypeScript (+ Go), Vexil is purpose-built.
3. **"No zero-copy decode"** → Correct. If zero-copy is your primary requirement, Cap'n Proto is the right tool. Vexil optimizes for wire compactness and determinism, not decode latency.

### Anti-personas (NOT a good fit)
- Teams that need 10+ language targets today
- Systems requiring zero-copy decode performance
- Projects that need self-describing wire format (schema-less consumers)
- Organizations that cannot tolerate any wire format changes before v1.0

---

## 8. Switching Dynamics (Four Forces)

**Push (away from current solution):**
- Hand-rolled encoders diverge across languages
- Protobuf wastes bandwidth for bit-level protocols
- No schema hash = silent data corruption on version mismatch

**Pull (toward Vexil):**
- Define once, generate Rust + TypeScript + Go
- Sub-byte types express intent directly
- BLAKE3 hash catches mismatches at connect time
- Delta encoding reduces streaming bandwidth 30-40%

**Habit (keeps them stuck):**
- Existing Protobuf schemas and generated code
- Team familiarity with current tooling
- "It works well enough"

**Anxiety (about switching):**
- Pre-1.0 stability concerns
- Smaller ecosystem and community
- "What if the project is abandoned?"

---

## 9. Customer Language

**How they describe the problem:**
- "We're hand-rolling binary encoders in three languages and they keep diverging"
- "Protobuf is too bloated for our sensor network"
- "We need bit-level packing but want something more maintainable than raw bitshifts"
- "Our encoding bugs only show up in production because there's no schema hash"

**How they'd describe Vexil:**
- "It's like Protobuf but for bit-packed protocols"
- "Schema language where encoding is part of the type"
- "Define your wire format, get Rust and TypeScript code that produces identical bytes"

**Words TO use:**
- Schema, wire format, encoding, bit-packed, compact, deterministic
- Cross-language, byte-identical, compliance vectors
- Define once, generate everywhere
- Schema hash, mismatch detection

**Words to AVOID:**
- "Serialization library" (it's a language + compiler, not a library)
- "Lightweight Protobuf" (undersells the differentiation)
- "Simple" (the problem space is inherently complex; claim clarity, not simplicity)
- "Enterprise-grade" (too corporate for the dev tool audience)
- "Revolutionary" / "game-changing" (let the tech speak)

**Glossary:**
- **Wire format** — The exact byte layout of encoded data
- **LSB-first** — Least-significant-bit first packing order
- **LEB128 / varint** — Variable-length integer encoding
- **ZigZag** — Signed integer encoding that maps to unsigned for efficient varint
- **Delta encoding** — Transmitting differences from previous frame instead of absolute values
- **Schema hash** — BLAKE3 hash of the canonical schema form
- **Compliance vectors** — Published golden byte sequences that all implementations must match
- **Conformance corpus** — 83 test files (valid + invalid) that any implementation must handle correctly

---

## 10. Brand Voice

**Tone:** Technical, precise, confident but honest about limitations

**Communication style:** Direct. Lead with what it does, not what it promises. Show code, not adjectives.

**Brand personality:** Rigorous, transparent, engineering-first, pragmatic, quietly ambitious

**Voice DO's:**
- Show code examples — they're the best marketing
- State limitations honestly (pre-1.0, 3 backends, no zero-copy)
- Use precise technical language (the audience knows what LEB128 is)
- Let the comparison table do the persuading
- Credit the problem space as hard — "this is genuinely difficult, and here's how we approach it"

**Voice DON'Ts:**
- Don't hype ("revolutionary", "blazingly fast", "the future of")
- Don't minimize limitations or hand-wave pre-1.0 status
- Don't use corporate marketing language ("synergy", "leverage", "best-in-class")
- Don't talk down to the audience — they're senior engineers
- Don't claim features you don't have yet (LSP, package registry)

---

## 11. Style Guide

- **Capitalization:** "Vexil" always capitalized. CLI tool is `vexilc` (lowercase, monospace). File extension is `.vexil`.
- **Code in prose:** Use backtick monospace for types (`u4`), annotations (`@varint`), CLI commands (`vexilc check`), file names
- **Numbers:** Spell out one through nine; use digits for 10+. Exception: always use digits for bit widths and byte counts.
- **Lists:** Prefer bullet points over numbered lists unless order matters
- **Competitor references:** Always factual. "Protobuf does not support sub-byte types" not "Protobuf fails at sub-byte types"

---

## 12. Proof Points

**Key metrics:**
- 436+ tests across the workspace
- 83-file conformance corpus (27 valid, 56 invalid)
- Cross-language compliance vectors: Rust, TypeScript, and Go produce byte-identical output
- Delta encoding: 25-30 byte steady-state frames vs 42 byte full frames (system-monitor example)
- Published on crates.io (`vexilc`, `vexil-lang`, `vexil-runtime`) and npm (`@vexil-lang/runtime`)

**Proof of rigor:**
- Formal PEG grammar derived from normative spec
- BLAKE3 schema hashing with canonical form
- No `unwrap()` in production code
- MIT OR Apache-2.0 dual license
- Governance document with RFC process for wire format changes

**Notable assets:**
- Working system-monitor demo: Rust server → WebSocket → browser dashboard with live delta-encoded metrics
- Cross-language example: Rust device → binary file → Node.js dashboard + Go gateway
- 30-page documentation book

---

## 13. Content & SEO Context

**Target keyword clusters:**

| Cluster | Keywords |
|---------|----------|
| Core | vexil schema language, vexil lang, binary schema definition |
| Problem | binary protocol definition, cross-language serialization, bit-packed encoding |
| Comparison | protobuf alternative, protobuf vs, schema language comparison, binary serialization comparison |
| Use case | IoT protocol schema, embedded binary protocol, sensor data encoding, compact wire format |
| Technical | deterministic encoding, schema hashing, sub-byte types, LSB bit packing, LEB128 varint |
| Delta | delta encoding streaming, binary diff encoding, compact websocket binary |

**Content tone:** Technical blog posts with code examples. 1,200-2,000 words. Always include a runnable schema + generated code.

---

## 14. Goals

**Primary goal:** Establish Vexil as the go-to schema language for bandwidth-constrained, cross-language binary protocols.

**Key conversion actions:**
1. `cargo install vexilc` (primary)
2. Star the GitHub repo
3. Try the getting-started guide
4. Join the community (when it exists)

**Current metrics:** No public marketing yet. This is day zero.

---

## Status

- [x] Product overview — 🟢 verified from README + spec
- [x] Target audience — 🟡 inferred from use cases and examples; needs user validation
- [x] Personas — 🟡 inferred; no real customer interviews yet
- [x] Problems & pain points — 🟢 verified from FAQ + comparison table
- [x] Competitive landscape — 🟢 verified from README comparison + FAQ
- [x] Differentiation — 🟢 verified from spec + code
- [x] Objections — 🟡 inferred from FAQ + limitations doc
- [x] Switching dynamics — 🟡 inferred; needs real user feedback
- [x] Customer language — 🔴 assumed; no real customer quotes yet
- [x] Brand voice — 🟡 inferred from existing README/FAQ tone
- [x] Style guide — 🟡 inferred from codebase conventions
- [x] Proof points — 🟢 verified from test suite + examples
- [x] Content & SEO — 🔴 assumed; no search data yet
- [x] Goals — 🟡 inferred; needs user confirmation
