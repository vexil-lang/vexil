# Vexil FAQ

## What is Vexil for?

Vexil is for binary protocols where representation is part of the contract.
Use it when bit widths, integer encodings, deterministic bytes, and explicit
schema evolution matter enough that handwritten codecs become a liability.

Typical fits include device telemetry, simulation state, network messages,
compact files, and IPC between systems you control.

## How is it different from Protobuf, Cap'n Proto, or FlatBuffers?

Those tools make different trade-offs. Vexil's defining choice is to expose
wire representation directly in the schema: `u4` is four bits, `@varint` is
unsigned LEB128, and fields are packed in declared ordinal order.

Vexil also emits a canonical BLAKE3 schema hash and provides an explicit
compatibility checker. Its trade-off is ecosystem breadth: the maintained
targets are Rust, TypeScript, Go, and Python, with different verification depth
across them.

Choose the tool whose compatibility model, target support, and operational
constraints fit your protocol. Vexil is not intended as a universal replacement
for established serialization systems.

## Is Vexil production-ready?

Treat Vexil as an actively maintained 0.x toolchain. The repository has a
normative wire specification, conformance corpus, golden byte vectors, native
runtime tests, compatibility checks, and generated-code verification. Rust and
TypeScript have the broadest evidence; Go and Python run a representative shared
wire matrix.

Important limits remain:

- there is no independent implementation or external security audit;
- Go and Python coverage is not exhaustive for every schema or environment;
- the language specification remains a draft;
- transport, framing, discovery, and compression are application concerns.

Review the [support matrix](docs/book/src/getting-started/support-matrix.md) and
[limitations](docs/limitations-and-gaps.md), then test the exact schemas and
target versions you intend to ship.

## Which languages are supported?

Vexil generates Rust, TypeScript, Go, and Python. The compiler's public
`CodegenBackend` contract supports additional backends, but adding one requires
target API design, runtime behavior, naming rules, golden output, native
compilation, and wire evidence. It is a maintained compatibility surface, not a
small adapter exercise.

## Why not use packed structs or C bitfields?

Packed memory layout is not a portable wire contract. Bitfield ordering,
alignment, padding, integer representation, and language interoperability can
vary by compiler or platform. Vexil defines these choices independently of an
in-memory representation and generates the corresponding codecs.

## How does schema evolution work?

Field and variant ordinals are durable identities. The language specification
classifies changes as compatible or breaking, and `vexilc compat` applies those
rules to two compiled schemas:

```sh
vexilc compat old.vexil new.vexil
```

Append-only fields, typed tombstones, non-exhaustive variants, version
constraints, and length-bounded union payloads provide the mechanics. They do
not remove the need to test rolling upgrades and application behavior.

Follow the [project-evolution example](examples/project-evolution/) for a
runnable compatible and breaking comparison.

## Does Vexil define a network protocol?

No. Vexil defines typed bytes. Your application still owns transport, message
framing, authentication, retries, discovery, compression, and resource policy.
The generated schema hash can be used in a handshake, but it is not a complete
negotiation or security protocol.

For schema-driven files, `vexil-store` provides human-readable `.vx` values and
binary `.vxb` containers with a schema-aware header.

## What does `@delta` do?

`@delta` generates a stateful encoder and decoder. Numeric fields are sent as
deltas from the previous frame; other fields keep their normal encoding. Both
sides must share state, and reconnecting requires a reset followed by a new base
frame.

Delta encoding is not compression. See the [live-telemetry example](examples/live-telemetry/)
for the full Rust-to-browser path.

## What are the fixed-point types?

`fixed32` uses a Q16.16 representation and `fixed64` uses Q32.32. Their wire
representation is an explicitly sized signed integer, which is useful when a
protocol must avoid an implementation-defined floating-point representation.

Vexil defines the stored representation and generated conversions. Applications
remain responsible for arithmetic policy, overflow handling, and rounding.
