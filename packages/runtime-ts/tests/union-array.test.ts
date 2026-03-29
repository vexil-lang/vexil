import { describe, it, expect } from 'vitest';
import { BitWriter, BitReader } from '../src/index.js';

describe('union array regression (#40)', () => {
  it('decodes all array elements without readRemaining eating siblings', () => {
    // Encode a union payload with 3 messages
    const w = new BitWriter();
    w.writeLeb128(0); // discriminant

    const pw = new BitWriter();
    pw.writeLeb128(3); // array count

    // 3 elements: u32 id + string name
    pw.writeU32(1);
    pw.writeString('alpha');
    pw.flushToByteBoundary();
    pw.writeU32(2);
    pw.writeString('beta');
    pw.flushToByteBoundary();
    pw.writeU32(3);
    pw.writeString('gamma');
    pw.flushToByteBoundary();

    const payload = pw.finish();
    w.writeLeb128(payload.length);
    w.writeRawBytes(payload);

    const bytes = w.finish();
    const r = new BitReader(bytes);

    const disc = r.readLeb128();
    expect(disc).toBe(0);

    const len = r.readLeb128();
    const payloadBytes = r.readRawBytes(len);
    const pr = new BitReader(payloadBytes);

    const count = pr.readLeb128();
    expect(count).toBe(3);

    const names = ['alpha', 'beta', 'gamma'];
    for (let i = 0; i < count; i++) {
      const id = pr.readU32();
      const name = pr.readString();
      pr.flushToByteBoundary();
      expect(id).toBe(i + 1);
      expect(name).toBe(names[i]);
    }
  });

  it('union frame has no spurious leading byte', () => {
    const w = new BitWriter();
    // Directly write discriminant — no flush at start
    w.writeLeb128(0);

    const pw = new BitWriter();
    pw.writeU32(42);
    pw.flushToByteBoundary();
    const payload = pw.finish();

    w.writeLeb128(payload.length);
    w.writeRawBytes(payload);

    const bytes = w.finish();
    expect(bytes[0]).toBe(0); // discriminant
    expect(bytes[1]).toBeGreaterThan(0); // payload length, not spurious zero
  });
});
