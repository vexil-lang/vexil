# TypeScript Runtime

The `@vexil-lang/runtime` npm package provides BitWriter and BitReader for Vexil-generated TypeScript code. It produces byte-identical output to the Rust runtime, verified by compliance vectors.

## Installation

```sh
npm install @vexil-lang/runtime
```

Zero dependencies.

## Core types

### BitWriter

```typescript
import { BitWriter } from '@vexil-lang/runtime';

const w = new BitWriter();
w.writeBits(0b1010, 4);   // write 4 bits
w.writeU8(255);            // write a full byte
w.writeVarint(12345n);     // write LEB128-encoded integer (BigInt)
const bytes = w.finish();  // flush and return Uint8Array
```

### BitReader

```typescript
import { BitReader } from '@vexil-lang/runtime';

const r = new BitReader(bytes);
const nibble = r.readBits(4);   // read 4 bits
const byte = r.readU8();        // read a full byte
const value = r.readVarint();   // read LEB128-encoded integer
```

## Generated code usage

```typescript
import { BitWriter, BitReader } from '@vexil-lang/runtime';
import { encodeMyMessage, decodeMyMessage } from './generated/my_message';

// Encode
const w = new BitWriter();
encodeMyMessage(myData, w);
const bytes = w.finish();

// Decode
const r = new BitReader(bytes);
const decoded = decodeMyMessage(r);
```

## Compliance

The TypeScript runtime is tested against the same compliance vectors as the Rust runtime. Both must produce identical bytes for every test case.

## Source

[`packages/runtime-ts/`](https://github.com/vexil-lang/vexil/tree/main/packages/runtime-ts)
