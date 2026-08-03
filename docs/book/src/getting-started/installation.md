# Installation

`vexilc` is the compiler and command-line entry point. The generated target
runtimes are installed separately by the applications that use them.

## Install from crates.io

With Rust installed:

```sh
cargo install vexilc
```

## Use a release binary

Tagged `vexilc` releases publish archives and installers for supported Linux,
macOS, and Windows targets on the repository's
[Releases page](https://github.com/vexil-lang/vexil/releases).

Use an asset from the exact release you selected; a local build does not prove
that a newer release artifact has been published.

## Build this checkout

The workspace minimum supported Rust version is 1.94.

```sh
git clone https://github.com/vexil-lang/vexil
cd vexil
cargo build --release --bin vexilc
```

The binary is written under `target/release/`.

## Verify

```sh
vexilc --version
vexilc --help
```

Then continue to [Your First Schema](./first-schema.md).

Target runtime installation belongs in [Generating Code](./generating-code.md)
and the target-specific runtime chapters. Check the [support matrix](./support-matrix.md)
before selecting a new cross-language combination.
