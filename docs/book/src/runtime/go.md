# Go Runtime

The Go runtime provides BitWriter and BitReader for Vexil-generated Go code.

## Installation

The Go runtime is available as the versioned module `github.com/vexil-lang/vexil/packages/runtime-go@v0.1.1`. It requires Go 1.22 or later.

```sh
go get github.com/vexil-lang/vexil/packages/runtime-go@v0.1.1
```

Generated Go and its runtime are exercised together against a representative
shared wire matrix. This is not exhaustive for every schema or environment.

## Core types

### BitWriter

```go
import vexil "github.com/vexil-lang/vexil/packages/runtime-go"

w := vexil.NewBitWriter()
w.WriteBits(0b1010, 4)    // write 4 bits
w.WriteU8(255)             // write a full byte
w.WriteVarint(12345)       // write LEB128-encoded integer
bytes := w.Finish()        // flush and return byte slice
```

### BitReader

```go
r := vexil.NewBitReader(bytes)
nibble := r.ReadBits(4)    // read 4 bits
b := r.ReadU8()            // read a full byte
value := r.ReadVarint()    // read LEB128-encoded integer
```

## Generated code usage

Generated Go structs implement `Pack` and `Unpack` methods:

```go
// Encode
w := vexil.NewBitWriter()
myMessage.Pack(w)
bytes := w.Finish()

// Decode
r := vexil.NewBitReader(bytes)
var decoded MyMessage
decoded.Unpack(r)
```

## Source

[`packages/runtime-go/`](https://github.com/vexil-lang/vexil/tree/main/packages/runtime-go)
