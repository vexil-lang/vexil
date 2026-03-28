import { describe, it, expect } from 'vitest';
import { BitReader } from '../src/bit-reader.js';
import { BitWriter } from '../src/bit-writer.js';

describe('BitReader', () => {
  describe('readBits', () => {
    it('reads a single true bit', () => {
      const r = new BitReader(new Uint8Array([0x01]));
      expect(r.readBool()).toBe(true);
    });

    it('reads a single false bit', () => {
      const r = new BitReader(new Uint8Array([0x00]));
      expect(r.readBool()).toBe(false);
    });

    it('reads sub-byte fields LSB-first', () => {
      // 0x9D = 10011101 -> LSB-first: bits[0..3] = 101 = 5, bits[3..8] = 10011 = 19
      const r = new BitReader(new Uint8Array([0x9d]));
      expect(r.readBits(3)).toBe(5);
      expect(r.readBits(5)).toBe(19);
    });

    it('reads across byte boundary', () => {
      const r = new BitReader(new Uint8Array([0x9d, 0x2a]));
      expect(r.readBits(3)).toBe(5);
      expect(r.readBits(5)).toBe(19);
      expect(r.readBits(6)).toBe(42);
    });
  });

  describe('flushToByteBoundary', () => {
    it('skips remaining bits in current byte', () => {
      const w = new BitWriter();
      w.writeBits(0b101, 3);
      w.flushToByteBoundary();
      w.writeU8(0xab);
      const buf = w.finish();

      const r = new BitReader(buf);
      expect(r.readBits(3)).toBe(0b101);
      r.flushToByteBoundary();
      expect(r.readU8()).toBe(0xab);
    });
  });

  describe('multi-byte reads', () => {
    it('readU8', () => {
      const r = new BitReader(new Uint8Array([0xff]));
      expect(r.readU8()).toBe(255);
    });

    it('readU16 little-endian', () => {
      const r = new BitReader(new Uint8Array([0x02, 0x01]));
      expect(r.readU16()).toBe(258);
    });

    it('readU32 little-endian', () => {
      const r = new BitReader(new Uint8Array([0x78, 0x56, 0x34, 0x12]));
      expect(r.readU32()).toBe(0x12345678);
    });

    it('readU64 little-endian', () => {
      const r = new BitReader(
        new Uint8Array([0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01]),
      );
      expect(r.readU64()).toBe(BigInt('0x0102030405060708'));
    });

    it('readI8 negative', () => {
      const r = new BitReader(new Uint8Array([0xff]));
      expect(r.readI8()).toBe(-1);
    });

    it('readI16 negative', () => {
      const r = new BitReader(new Uint8Array([0xff, 0xff]));
      expect(r.readI16()).toBe(-1);
    });

    it('readI32 negative', () => {
      const r = new BitReader(new Uint8Array([0xff, 0xff, 0xff, 0xff]));
      expect(r.readI32()).toBe(-1);
    });

    it('readI64 negative', () => {
      const r = new BitReader(
        new Uint8Array([0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
      );
      expect(r.readI64()).toBe(BigInt(-1));
    });
  });

  describe('floating point', () => {
    it('readF32 NaN canonical', () => {
      const r = new BitReader(new Uint8Array([0x00, 0x00, 0xc0, 0x7f]));
      const v = r.readF32();
      expect(Number.isNaN(v)).toBe(true);
    });

    it('readF64 negative zero', () => {
      const r = new BitReader(
        new Uint8Array([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80]),
      );
      const v = r.readF64();
      expect(v).toBe(-0.0);
      expect(Object.is(v, -0)).toBe(true);
    });

    it('readF32 pi round-trip', () => {
      const w = new BitWriter();
      w.writeF32(Math.fround(Math.PI));
      const buf = w.finish();
      const r = new BitReader(buf);
      expect(r.readF32()).toBe(Math.fround(Math.PI));
    });

    it('readF64 NaN round-trip is canonical', () => {
      const w = new BitWriter();
      w.writeF64(NaN);
      const buf = w.finish();
      const r = new BitReader(buf);
      const v = r.readF64();
      expect(Number.isNaN(v)).toBe(true);
    });
  });

  describe('LEB128', () => {
    it('reads 0', () => {
      const r = new BitReader(new Uint8Array([0x00]));
      expect(r.readLeb128()).toBe(0);
    });

    it('reads 300', () => {
      const r = new BitReader(new Uint8Array([0xac, 0x02]));
      expect(r.readLeb128()).toBe(300);
    });

    it('round-trip 128', () => {
      const w = new BitWriter();
      w.writeLeb128(128);
      const r = new BitReader(w.finish());
      expect(r.readLeb128()).toBe(128);
    });
  });

  describe('string and bytes', () => {
    it('readString "hello"', () => {
      const r = new BitReader(
        new Uint8Array([0x05, 0x68, 0x65, 0x6c, 0x6c, 0x6f]),
      );
      expect(r.readString()).toBe('hello');
    });

    it('readString empty', () => {
      const r = new BitReader(new Uint8Array([0x00]));
      expect(r.readString()).toBe('');
    });

    it('readBytes', () => {
      const r = new BitReader(new Uint8Array([0x02, 0xde, 0xad]));
      expect(r.readBytes()).toEqual(new Uint8Array([0xde, 0xad]));
    });

    it('readRawBytes', () => {
      const r = new BitReader(new Uint8Array([0xca, 0xfe]));
      expect(r.readRawBytes(2)).toEqual(new Uint8Array([0xca, 0xfe]));
    });

    it('string round-trip', () => {
      const w = new BitWriter();
      w.writeString('hello');
      const r = new BitReader(w.finish());
      expect(r.readString()).toBe('hello');
    });
  });

  describe('error handling', () => {
    it('throws on unexpected EOF for readU8', () => {
      const r = new BitReader(new Uint8Array([]));
      expect(() => r.readU8()).toThrow('Unexpected end of data');
    });

    it('throws on unexpected EOF for readBits', () => {
      const r = new BitReader(new Uint8Array([]));
      expect(() => r.readBits(1)).toThrow('Unexpected end of data');
    });

    it('throws on unexpected EOF for readU16', () => {
      const r = new BitReader(new Uint8Array([0x01]));
      expect(() => r.readU16()).toThrow('Unexpected end of data');
    });
  });

  describe('remaining', () => {
    it('reports remaining bytes', () => {
      const r = new BitReader(new Uint8Array([0x01, 0x02, 0x03]));
      expect(r.remaining()).toBe(3);
      r.readU8();
      expect(r.remaining()).toBe(2);
    });
  });

  describe('readLeb12864', () => {
    it('round-trips 0n', () => {
      const w = new BitWriter();
      w.writeLeb12864(0n);
      const r = new BitReader(w.finish());
      expect(r.readLeb12864()).toBe(0n);
    });

    it('round-trips 300n', () => {
      const w = new BitWriter();
      w.writeLeb12864(300n);
      const r = new BitReader(w.finish());
      expect(r.readLeb12864()).toBe(300n);
    });

    it('round-trips large value', () => {
      const w = new BitWriter();
      const big = 1234567890123456789n;
      w.writeLeb12864(big);
      const r = new BitReader(w.finish());
      expect(r.readLeb12864()).toBe(big);
    });
  });

  describe('readZigZag', () => {
    it('decodes 0', () => {
      const w = new BitWriter();
      w.writeZigZag(0, 32);
      const r = new BitReader(w.finish());
      expect(r.readZigZag(32)).toBe(0);
    });

    it('decodes -1', () => {
      const w = new BitWriter();
      w.writeZigZag(-1, 32);
      const r = new BitReader(w.finish());
      expect(r.readZigZag(32)).toBe(-1);
    });

    it('decodes 1', () => {
      const w = new BitWriter();
      w.writeZigZag(1, 32);
      const r = new BitReader(w.finish());
      expect(r.readZigZag(32)).toBe(1);
    });

    it('round-trips -42', () => {
      const w = new BitWriter();
      w.writeZigZag(-42, 32);
      const r = new BitReader(w.finish());
      expect(r.readZigZag(32)).toBe(-42);
    });
  });

  describe('readZigZag64', () => {
    it('round-trips 0n', () => {
      const w = new BitWriter();
      w.writeZigZag64(0n);
      const r = new BitReader(w.finish());
      expect(r.readZigZag64()).toBe(0n);
    });

    it('round-trips -1n', () => {
      const w = new BitWriter();
      w.writeZigZag64(-1n);
      const r = new BitReader(w.finish());
      expect(r.readZigZag64()).toBe(-1n);
    });

    it('round-trips large negative', () => {
      const w = new BitWriter();
      w.writeZigZag64(-123456789n);
      const r = new BitReader(w.finish());
      expect(r.readZigZag64()).toBe(-123456789n);
    });
  });

  describe('recursion depth', () => {
    it('allows up to 64 levels', () => {
      const r = new BitReader(new Uint8Array([]));
      for (let i = 0; i < 64; i++) {
        r.enterNested();
      }
    });

    it('throws at 65 levels', () => {
      const r = new BitReader(new Uint8Array([]));
      for (let i = 0; i < 64; i++) {
        r.enterNested();
      }
      expect(() => r.enterNested()).toThrow('Recursion limit exceeded');
    });

    it('leave allows re-entry', () => {
      const r = new BitReader(new Uint8Array([]));
      for (let i = 0; i < 64; i++) {
        r.enterNested();
      }
      r.leaveNested();
      r.enterNested(); // Should not throw
    });
  });

  describe('round-trip with BitWriter', () => {
    it('round-trips sub-byte fields', () => {
      const w = new BitWriter();
      w.writeBits(5, 3);
      w.writeBits(19, 5);
      w.writeBits(42, 6);
      const buf = w.finish();
      const r = new BitReader(buf);
      expect(r.readBits(3)).toBe(5);
      expect(r.readBits(5)).toBe(19);
      expect(r.readBits(6)).toBe(42);
    });

    it('round-trips u16', () => {
      const w = new BitWriter();
      w.writeU16(0x1234);
      const r = new BitReader(w.finish());
      expect(r.readU16()).toBe(0x1234);
    });

    it('round-trips i32 negative', () => {
      const w = new BitWriter();
      w.writeI32(-42);
      const r = new BitReader(w.finish());
      expect(r.readI32()).toBe(-42);
    });

    it('round-trips mixed types', () => {
      const w = new BitWriter();
      w.writeBool(true);
      w.writeU16(42);
      w.writeString('test');
      const buf = w.finish();
      const r = new BitReader(buf);
      expect(r.readBool()).toBe(true);
      r.flushToByteBoundary();
      expect(r.readU16()).toBe(42);
      expect(r.readString()).toBe('test');
    });
  });

  describe('readRemaining', () => {
    it('reads remaining bytes after partial decode', () => {
      const r = new BitReader(new Uint8Array([0x2a, 0x00, 0x00, 0x00, 0x63, 0x00]));
      r.readU32();
      const remaining = r.readRemaining();
      expect(remaining.length).toBe(2);
      expect(remaining[0]).toBe(0x63);
      expect(remaining[1]).toBe(0x00);
    });

    it('returns empty when fully consumed', () => {
      const r = new BitReader(new Uint8Array([0x2a, 0x00, 0x00, 0x00]));
      r.readU32();
      const remaining = r.readRemaining();
      expect(remaining.length).toBe(0);
    });

    it('reads all bytes from start', () => {
      const r = new BitReader(new Uint8Array([0x01, 0x02, 0x03]));
      const remaining = r.readRemaining();
      expect(remaining).toEqual(new Uint8Array([0x01, 0x02, 0x03]));
    });
  });

  describe('trailing bytes tolerance', () => {
    it('does not reject trailing bytes', () => {
      // Simulate v2-encoded message read by v1 decoder
      const data = new Uint8Array([0x2a, 0x00, 0x00, 0x00, 0x63, 0x00]);
      const r = new BitReader(data);
      expect(r.readU32()).toBe(42);
      r.flushToByteBoundary();
      // Remaining bytes must not cause error
      expect(r.remaining()).toBe(2);
    });
  });
});
