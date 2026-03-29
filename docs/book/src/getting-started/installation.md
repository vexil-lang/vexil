# Installation

## Pre-built binaries

Download from the [Releases page](https://github.com/vexil-lang/vexil/releases). Binaries are available for:

- Linux x86-64
- Linux ARM64
- macOS Apple Silicon
- macOS Intel
- Windows x86-64

### Shell installer (Linux/macOS)

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/vexil-lang/vexil/releases/latest/download/vexilc-installer.sh | sh
```

### PowerShell installer (Windows)

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/vexil-lang/vexil/releases/latest/download/vexilc-installer.ps1 | iex"
```

## From crates.io

```sh
cargo install vexilc
```

Requires Rust 1.94 or later.

## From source

```sh
git clone https://github.com/vexil-lang/vexil
cd vexil
cargo build --release --bin vexilc
```

The binary will be at `target/release/vexilc`.

## Verify

```sh
vexilc --version
```
