<h1 align="center">Vexil</h1>
<p align="center"><em>Exact binary protocols, defined as types.</em></p>

<p align="center">
  <a href="https://github.com/vexil-lang/vexil/actions/workflows/ci.yml"><img src="https://github.com/vexil-lang/vexil/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://vexil-lang.github.io/vexil/"><img src="https://img.shields.io/badge/docs-mdBook-0f766e" alt="Documentation"></a>
  <a href="https://crates.io/crates/vexilc"><img src="https://img.shields.io/crates/v/vexilc" alt="vexilc on crates.io"></a>
  <img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue" alt="License: MIT OR Apache-2.0">
</p>

Vexil is a schema language and toolchain for compact, deterministic binary
protocols. A schema defines both the data model and its representation on the
wire: `u4` is four bits, `@varint` selects unsigned LEB128, and fields are packed
LSB-first without platform-dependent padding.

Generate Rust, TypeScript, Go, or Python codecs from the same contract. Every
generated target carries the schema's canonical BLAKE3 hash, while `vexilc
compat` makes compatible and breaking schema changes explicit.

> Vexil is being actively revived through a focused 0.x stabilization release.
> Start with the [support matrix](docs/book/src/getting-started/support-matrix.md)
> and [current limitations](docs/limitations-and-gaps.md), especially for new
> cross-language deployments.

## See the contract

```vexil
namespace sensor.packet

enum SensorKind : u8 {
    Temperature @0
    Humidity    @1
    Pressure    @2
}

message SensorReading {
    channel  @0 : u4
    kind     @1 : SensorKind
    value    @2 : u16
    sequence @3 : u32 @varint
    delta_ts @4 : i32 @zigzag
}
```

That schema fixes details which are often left to handwritten codecs:

- `channel` occupies exactly four bits;
- `kind` uses its declared ordinal;
- `sequence` uses unsigned LEB128;
- `delta_ts` uses ZigZag followed by LEB128;
- the canonical schema hash is independent of comments and formatting.

The wire is deliberately not self-describing. Both peers compile the schema
they intend to use and can compare its hash before exchanging application data.

## Start in five minutes

Install the compiler:

```sh
cargo install vexilc
```

Or build this checkout with Rust 1.94 or later:

```sh
cargo build --release --bin vexilc
```

Check a schema and generate a Rust codec:

```sh
vexilc check telemetry.vexil
vexilc codegen telemetry.vexil --target rust --output telemetry.rs
```

For a runnable first project:

```sh
cargo run --manifest-path examples/quickstart/Cargo.toml
```

The [quickstart guide](examples/quickstart/) explains the schema, generated
source, exact bytes, and round trip.

## Why Vexil

### Wire choices are reviewable

Bit widths, field ordinals, integer encodings, collection bounds, and evolution
annotations live in the schema instead of being scattered across encoders.

### Output is deterministic

Canonical map and set ordering, defined scalar encodings, and padding rules make
the same value produce the same bytes for a given schema.

### Drift is visible

Generated schema hashes identify the exact contract. `vexilc compat` classifies
schema changes and reports the required SemVer level before a protocol ships.

## Choose a generated target

| Target | Runtime | Current evidence |
| --- | --- | --- |
| Rust | `vexil-runtime` | Broad compile, golden, Clippy, and byte-vector coverage |
| TypeScript | `@vexil-lang/runtime` | Native build/tests and broad byte-vector coverage |
| Go | `packages/runtime-go` | Native execution over a representative shared wire matrix |
| Python | [`vexil-runtime`](https://pypi.org/project/vexil-runtime/) | Static and native execution over a representative shared wire matrix |

“Representative” is intentional. Go and Python do not yet have the same breadth
of generated-code evidence as Rust and TypeScript. Verify the schemas and target
combinations used by your application.

Read [Generating Code](docs/book/src/getting-started/generating-code.md) for
target-specific commands and [Compatibility and Limitations](docs/book/src/getting-started/compatibility.md)
for the adoption boundary.

## Follow the examples

1. [Quickstart](examples/quickstart/) — check, generate, encode, and decode one schema.
2. [Project evolution](examples/project-evolution/) — imports and compatible versus breaking changes.
3. [Cross-language interop](examples/cross-language/) — one fixture encoded by four generated targets.
4. [Live telemetry](examples/live-telemetry/) — stateful delta frames from Rust to a browser.

Run the complete checked path with:

```sh
python scripts/examples.py check all
```

## Documentation

- [Documentation book](https://vexil-lang.github.io/vexil/)
- [Language specification](spec/language.md)
- [Binary wire-format specification](spec/wire-format.md)
- [CLI reference](docs/book/src/cli/overview.md)
- [Support matrix](docs/book/src/getting-started/support-matrix.md)
- [FAQ](FAQ.md)
- [Changelog](CHANGELOG.md)

The specifications are normative. Guides and examples explain the contract but
do not replace it.

## Contributing

Bug reports, focused documentation corrections, corpus cases, and code changes
are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull
request. Language or wire changes follow the lightweight RFC process in
[GOVERNANCE.md](GOVERNANCE.md).

## License

Licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your
option.
