import { describe, it, expect } from 'vitest';
import { BitWriter } from '../src/bit-writer.js';

/** Convert Uint8Array to hex string for easy comparison. */
function toHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

describe('BitWriter', () => {
  describe('writeBits', () => {
    it('writes a single true bit', () => {
      const w = new BitWriter();
      w.writeBool(true);
      expect(toHex(w.finish())).toBe('01');
    });

    it('writes a single false bit', () => {
      const w = new BitWriter();
      w.writeBool(false);
      expect(toHex(w.finish())).toBe('00');
    });

    it('packs bits LSB-first within byte', () => {
      const w = new BitWriter();
      w.writeBits(5, 3); // 101
      w.writeBits(19, 5); // 10011
      // LSB-first: byte = 10011_101 = 0x9D
      expect(toHex(w.finish())).toBe('9d');
    });

    it('crosses byte boundary', () => {
      const w = new BitWriter();
      w.writeBits(5, 3);
      w.writeBits(19, 5);
      w.writeBits(42, 6); // 101010
      // Byte 0: 0x9D, Byte 1: 00_101010 = 0x2A
      expect(toHex(w.finish())).toBe('9d2a');
    });
  });

  describe('flushToByteBoundary', () => {
    it('pads with zeros', () => {
      const w = new BitWriter();
      w.writeBits(0b101, 3);
      w.flushToByteBoundary();
      w.writeBits(0xff, 8);
      expect(toHex(w.finish())).toBe('05ff');
    });

    it('empty writer produces zero byte', () => {
      const w = new BitWriter();
      w.flushToByteBoundary();
      expect(toHex(w.finish())).toBe('00');
    });
  });

  describe('multi-byte writes', () => {
    it('writeU8 flushes first', () => {
      const w = new BitWriter();
      w.writeBool(true);
      w.writeU8(0xab);
      expect(toHex(w.finish())).toBe('01ab');
    });

    it('writeU16 little-endian', () => {
      const w = new BitWriter();
      w.writeU16(0x0102);
      expect(toHex(w.finish())).toBe('0201');
    });

    it('writeU32 little-endian', () => {
      const w = new BitWriter();
      w.writeU32(0x01020304);
      expect(toHex(w.finish())).toBe('04030201');
    });

    it('writeU64 little-endian', () => {
      const w = new BitWriter();
      w.writeU64(BigInt('0x0102030405060708'));
      expect(toHex(w.finish())).toBe('0807060504030201');
    });

    it('writeI16 negative', () => {
      const w = new BitWriter();
      w.writeI16(-1);
      expect(toHex(w.finish())).toBe('ffff');
    });

    it('writeI32 negative -1', () => {
      const w = new BitWriter();
      w.writeI32(-1);
      expect(toHex(w.finish())).toBe('ffffffff');
    });

    it('writeI64 negative', () => {
      const w = new BitWriter();
      w.writeI64(BigInt(-1));
      expect(toHex(w.finish())).toBe('ffffffffffffffff');
    });
  });

  describe('floating point', () => {
    it('writeF32 NaN canonicalized', () => {
      const w = new BitWriter();
      w.writeF32(NaN);
      expect(toHex(w.finish())).toBe('0000c07f');
    });

    it('writeF32 negative zero preserved', () => {
      const w = new BitWriter();
      w.writeF32(-0.0);
      expect(toHex(w.finish())).toBe('00000080');
    });

    it('writeF64 NaN canonicalized', () => {
      const w = new BitWriter();
      w.writeF64(NaN);
      expect(toHex(w.finish())).toBe('000000000000f87f');
    });

    it('writeF64 negative zero', () => {
      const w = new BitWriter();
      w.writeF64(-0.0);
      expect(toHex(w.finish())).toBe('0000000000000080');
    });

    it('writeF32 pi', () => {
      const w = new BitWriter();
      w.writeF32(Math.fround(Math.PI));
      // PI as f32 = 0x40490FDB, LE = DB 0F 49 40
      expect(toHex(w.finish())).toBe('db0f4940');
    });
  });

  describe('LEB128', () => {
    it('encodes 0', () => {
      const w = new BitWriter();
      w.writeLeb128(0);
      expect(toHex(w.finish())).toBe('00');
    });

    it('encodes 127', () => {
      const w = new BitWriter();
      w.writeLeb128(127);
      expect(toHex(w.finish())).toBe('7f');
    });

    it('encodes 128', () => {
      const w = new BitWriter();
      w.writeLeb128(128);
      expect(toHex(w.finish())).toBe('8001');
    });

    it('encodes 300', () => {
      const w = new BitWriter();
      w.writeLeb128(300);
      expect(toHex(w.finish())).toBe('ac02');
    });
  });

  describe('string and bytes', () => {
    it('writeString "hi"', () => {
      const w = new BitWriter();
      w.writeString('hi');
      expect(toHex(w.finish())).toBe('026869');
    });

    it('writeString "hello"', () => {
      const w = new BitWriter();
      w.writeString('hello');
      expect(toHex(w.finish())).toBe('0568656c6c6f');
    });

    it('writeString empty', () => {
      const w = new BitWriter();
      w.writeString('');
      expect(toHex(w.finish())).toBe('00');
    });

    it('writeBytes', () => {
      const w = new BitWriter();
      w.writeBytes(new Uint8Array([0xde, 0xad]));
      expect(toHex(w.finish())).toBe('02dead');
    });

    it('writeRawBytes', () => {
      const w = new BitWriter();
      w.writeRawBytes(new Uint8Array([0xca, 0xfe]));
      expect(toHex(w.finish())).toBe('cafe');
    });
  });

  describe('empty finish', () => {
    it('produces zero byte', () => {
      const w = new BitWriter();
      expect(toHex(w.finish())).toBe('00');
    });
  });

  describe('writeLeb12864', () => {
    it('encodes 0n', () => {
      const w = new BitWriter();
      w.writeLeb12864(0n);
      expect(toHex(w.finish())).toBe('00');
    });

    it('encodes 300n', () => {
      const w = new BitWriter();
      w.writeLeb12864(300n);
      // 300 = 0x12C → LEB128: ac 02
      expect(toHex(w.finish())).toBe('ac02');
    });
  });

  describe('writeZigZag', () => {
    it('encodes 0 as 0', () => {
      const w = new BitWriter();
      w.writeZigZag(0, 32);
      expect(toHex(w.finish())).toBe('00');
    });

    it('encodes -1 as 1', () => {
      const w = new BitWriter();
      w.writeZigZag(-1, 32);
      expect(toHex(w.finish())).toBe('01');
    });

    it('encodes 1 as 2', () => {
      const w = new BitWriter();
      w.writeZigZag(1, 32);
      expect(toHex(w.finish())).toBe('02');
    });

    it('encodes -2 as 3', () => {
      const w = new BitWriter();
      w.writeZigZag(-2, 32);
      expect(toHex(w.finish())).toBe('03');
    });
  });

  describe('writeZigZag64', () => {
    it('encodes 0n as 0', () => {
      const w = new BitWriter();
      w.writeZigZag64(0n);
      expect(toHex(w.finish())).toBe('00');
    });

    it('encodes -1n as 1', () => {
      const w = new BitWriter();
      w.writeZigZag64(-1n);
      expect(toHex(w.finish())).toBe('01');
    });

    it('encodes 1n as 2', () => {
      const w = new BitWriter();
      w.writeZigZag64(1n);
      expect(toHex(w.finish())).toBe('02');
    });
  });

  describe('recursion depth', () => {
    it('allows up to 64 levels', () => {
      const w = new BitWriter();
      for (let i = 0; i < 64; i++) {
        w.enterNested();
      }
      // Should not throw
    });

    it('throws at 65 levels', () => {
      const w = new BitWriter();
      for (let i = 0; i < 64; i++) {
        w.enterNested();
      }
      expect(() => w.enterNested()).toThrow('Recursion limit exceeded');
    });

    it('leave allows re-entry', () => {
      const w = new BitWriter();
      for (let i = 0; i < 64; i++) {
        w.enterNested();
      }
      w.leaveNested();
      w.enterNested(); // Should not throw
    });
  });
});
