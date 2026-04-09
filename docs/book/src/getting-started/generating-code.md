# Generating Code

## Single file

```sh
# Rust (default target)
vexilc codegen hello.vexil --target rust --output hello.rs

# TypeScript
vexilc codegen hello.vexil --target typescript --output hello.ts

# Go
vexilc codegen hello.vexil --target go --output hello.go

# Python
vexilc codegen hello.vexil --target python --output hello.py
```

Default target is `rust`. Output goes to stdout if `--output` is omitted.

## Multi-file project

For schemas with imports, use the `build` subcommand:

```sh
vexilc build root.vexil --include ./schemas --output ./generated --target rust
```

This resolves all imports, compiles in topological order, and generates one file per namespace.

## Watch mode

Auto-rebuild on save:

```sh
vexilc watch root.vexil --include ./schemas --output ./generated --target typescript
```

Changes to any `.vexil` file in the watched directories trigger a rebuild with 200ms debounce.

## Using generated code

### Rust

Add `vexil-runtime` to your `Cargo.toml`:

```toml
[dependencies]
vexil-runtime = "0.5"
```

```rust
use vexil_runtime::{BitWriter, BitReader, Pack, Unpack};

let greeting = Greeting {
    name: "world".to_string(),
    message: "hello".to_string(),
    count: 42,
    _unknown: Vec::new(),
};

// Encode
let mut w = BitWriter::new();
greeting.pack(&mut w).unwrap();
let bytes = w.finish();

// Decode
let mut r = BitReader::new(&bytes);
let decoded = Greeting::unpack(&mut r).unwrap();
```

### TypeScript

Install `@vexil-lang/runtime`:

```sh
npm install @vexil-lang/runtime
```

```typescript
import { BitWriter, BitReader } from '@vexil-lang/runtime';
import { encodeGreeting, decodeGreeting } from './hello';

const w = new BitWriter();
encodeGreeting(
  { name: 'world', message: 'hello', count: 42, _unknown: new Uint8Array(0) },
  w,
);
const bytes = w.finish();

const r = new BitReader(bytes);
const decoded = decodeGreeting(r);
```

### Go

```go
import vexil "github.com/vexil-lang/vexil/packages/runtime-go"

greeting := &Greeting{
    Name:    "world",
    Message: "hello",
    Count:   42,
}

w := vexil.NewBitWriter()
greeting.Pack(w)
bytes := w.Finish()

r := vexil.NewBitReader(bytes)
var decoded Greeting
decoded.Unpack(r)
```
