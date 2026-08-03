import { describe, expect, it } from 'vitest';
import { BitReader, BitWriter } from '../src/index.js';
import {
  decodeEvent,
  encodeEvent,
  type Event,
} from '../../../crates/vexil-codegen-ts/tests/golden/050_non_exhaustive_union_unknown_collision.js';

describe('generated non-exhaustive union unknown values', () => {
  it('preserves discriminant and payload bytes', () => {
    const value: Event = {
      tag: '__vexil_unknown',
      discriminant: 9,
      data: Uint8Array.from([0xde, 0xad]),
    };
    const writer = new BitWriter();
    encodeEvent(value, writer);
    const bytes = writer.finish();
    expect(Array.from(bytes)).toEqual([0x09, 0x02, 0xde, 0xad]);

    const decoded = decodeEvent(new BitReader(bytes));
    expect(decoded).toEqual(value);

    const roundtrip = new BitWriter();
    encodeEvent(decoded, roundtrip);
    expect(Array.from(roundtrip.finish())).toEqual(Array.from(bytes));
  });

  it('rejects over-limit payload lengths before allocation', () => {
    const encoded = Uint8Array.from([0x09, 0x81, 0x80, 0x80, 0x20]);
    expect(() => decodeEvent(new BitReader(encoded))).toThrow(/exceeds limit/);
  });
});
