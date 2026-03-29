# Cross-Language Interop

The `examples/cross-language/` directory demonstrates Rust and Node.js exchanging binary data through Vexil-encoded `.vxb` files.

## How it works

1. A shared `.vexil` schema defines the data types
2. The Rust program encodes data to a `.vxb` binary file
3. The Node.js program reads the same `.vxb` file and decodes it
4. Both sides produce and consume byte-identical wire format

This works because all Vexil backends (Rust, TypeScript, Go) implement the same deterministic encoding rules, verified by compliance vectors.

## Running

```sh
cd examples/cross-language

# Build and run the Rust encoder
cd rust-device
cargo run

# Run the Node.js decoder
cd ../node-reader
npm install
npm start
```

## Source

[`examples/cross-language/`](https://github.com/vexil-lang/vexil/tree/main/examples/cross-language)
