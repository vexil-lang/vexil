# TypeScript Runtime

The `@vexil-lang/runtime` npm package provides the bit reader and writer used by
Vexil-generated TypeScript code.

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
w.writeLeb128(12345);      // write an unsigned LEB128 integer
const bytes = w.finish();  // flush and return Uint8Array
```

### BitReader

```typescript
import { BitReader } from '@vexil-lang/runtime';

const r = new BitReader(bytes);
const nibble = r.readBits(4);   // read 4 bits
const byte = r.readU8();        // read a full byte
const value = r.readLeb128();   // read an unsigned LEB128 integer
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

Generated TypeScript has broad coverage against the same byte vectors as the
Rust reference implementation. See the [support matrix](../getting-started/support-matrix.md)
for the exact project-level claim.

## Source

[`packages/runtime-ts/`](https://github.com/vexil-lang/vexil/tree/main/packages/runtime-ts)
