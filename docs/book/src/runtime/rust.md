# Rust Runtime

The `vexil-runtime` crate provides the runtime support needed by Vexil-generated Rust code.

## Installation

```toml
[dependencies]
vexil-runtime = "0.5"
```

## Core types

### BitWriter

Encodes data into a byte buffer with LSB-first bit packing.

```rust
use vexil_runtime::BitWriter;

let mut w = BitWriter::new();
w.write_bits(0b1010, 4);  // write 4 bits
w.write_u8(255);           // write a full byte
w.write_varint(12345);     // write LEB128-encoded integer
let bytes = w.finish();    // flush and return the byte buffer
```

### BitReader

Decodes data from a byte buffer with LSB-first bit packing.

```rust
use vexil_runtime::BitReader;

let mut r = BitReader::new(&bytes);
let nibble = r.read_bits(4)?;   // read 4 bits
let byte = r.read_u8()?;        // read a full byte
let value = r.read_varint()?;   // read LEB128-encoded integer
```

### Pack and Unpack traits

Generated message types implement `Pack` and `Unpack`:

```rust
use vexil_runtime::{BitWriter, BitReader, Pack, Unpack};

// Encode
let mut w = BitWriter::new();
my_message.pack(&mut w)?;
let bytes = w.finish();

// Decode
let mut r = BitReader::new(&bytes);
let decoded = MyMessage::unpack(&mut r)?;
```

## API documentation

Full API documentation is available on [docs.rs/vexil-runtime](https://docs.rs/vexil-runtime).

## Source

[`crates/vexil-runtime/`](https://github.com/vexil-lang/vexil/tree/main/crates/vexil-runtime)
