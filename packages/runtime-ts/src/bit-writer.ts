const MAX_RECURSION_DEPTH = 64;

const textEncoder = new TextEncoder();

/**
 * A byte-buffer builder that packs fields LSB-first at the bit level.
 *
 * Created with `new BitWriter()`, written to with `write*` methods, and
 * finalized with `finish()` which flushes any partial byte and returns
 * the completed buffer.
 *
 * Sub-byte fields are accumulated in a single byte; once 8 bits are filled
 * the byte is flushed. Multi-byte writes (e.g. `writeU16`) first align to
 * a byte boundary, then append little-endian bytes directly.
 */
export class BitWriter {
  private buf: number[] = [];
  private currentByte = 0;
  private bitOffset = 0;
  private recursionDepth = 0;

  /**
   * Internal: align to a byte boundary without the "empty = zero byte" rule.
   * Used before multi-byte writes to ensure alignment.
   */
  private align(): void {
    if (this.bitOffset > 0) {
      this.buf.push(this.currentByte);
      this.currentByte = 0;
      this.bitOffset = 0;
    }
  }

  /**
   * Write `count` bits from `value`, LSB first.
   */
  writeBits(value: number, count: number): void {
    let v = value;
    for (let i = 0; i < count; i++) {
      const bit = v & 1;
      this.currentByte |= bit << this.bitOffset;
      this.bitOffset++;
      if (this.bitOffset === 8) {
        this.buf.push(this.currentByte);
        this.currentByte = 0;
        this.bitOffset = 0;
      }
      v >>>= 1;
    }
  }

  /**
   * Write a single boolean as 1 bit.
   */
  writeBool(v: boolean): void {
    this.writeBits(v ? 1 : 0, 1);
  }

  /**
   * Flush any partial byte to the buffer.
   *
   * Special case per spec section 4.1: if nothing has been written at all
   * (bitOffset == 0 AND buf is empty), push a zero byte anyway.
   * If bitOffset == 0 and buf is non-empty, this is a no-op.
   */
  flushToByteBoundary(): void {
    if (this.bitOffset === 0) {
      if (this.buf.length === 0) {
        this.buf.push(0x00);
      }
      // else: already aligned and something was written - no-op
    } else {
      this.buf.push(this.currentByte);
      this.currentByte = 0;
      this.bitOffset = 0;
    }
  }

  /**
   * Write a u8, aligning to a byte boundary first.
   */
  writeU8(v: number): void {
    this.align();
    this.buf.push(v & 0xff);
  }

  /**
   * Write a u16 in little-endian byte order, aligning first.
   */
  writeU16(v: number): void {
    this.align();
    this.buf.push(v & 0xff);
    this.buf.push((v >>> 8) & 0xff);
  }

  /**
   * Write a u32 in little-endian byte order, aligning first.
   */
  writeU32(v: number): void {
    this.align();
    this.buf.push(v & 0xff);
    this.buf.push((v >>> 8) & 0xff);
    this.buf.push((v >>> 16) & 0xff);
    this.buf.push((v >>> 24) & 0xff);
  }

  /**
   * Write a u64 in little-endian byte order, aligning first.
   */
  writeU64(v: bigint): void {
    this.align();
    const mask = BigInt(0xff);
    for (let i = 0; i < 8; i++) {
      this.buf.push(Number((v >> BigInt(i * 8)) & mask));
    }
  }

  /**
   * Write an i8, aligning to a byte boundary first.
   */
  writeI8(v: number): void {
    this.align();
    this.buf.push(v & 0xff);
  }

  /**
   * Write an i16 in little-endian byte order, aligning first.
   */
  writeI16(v: number): void {
    this.align();
    // Use DataView for correct two's complement encoding
    const ab = new ArrayBuffer(2);
    new DataView(ab).setInt16(0, v, true);
    const bytes = new Uint8Array(ab);
    this.buf.push(bytes[0], bytes[1]);
  }

  /**
   * Write an i32 in little-endian byte order, aligning first.
   */
  writeI32(v: number): void {
    this.align();
    const ab = new ArrayBuffer(4);
    new DataView(ab).setInt32(0, v, true);
    const bytes = new Uint8Array(ab);
    this.buf.push(bytes[0], bytes[1], bytes[2], bytes[3]);
  }

  /**
   * Write an i64 in little-endian byte order, aligning first.
   */
  writeI64(v: bigint): void {
    this.align();
    const ab = new ArrayBuffer(8);
    new DataView(ab).setBigInt64(0, v, true);
    const bytes = new Uint8Array(ab);
    for (let i = 0; i < 8; i++) {
      this.buf.push(bytes[i]);
    }
  }

  /**
   * Write an f32, canonicalizing NaN to 0x7FC00000.
   */
  writeF32(v: number): void {
    this.align();
    const ab = new ArrayBuffer(4);
    const dv = new DataView(ab);
    if (Number.isNaN(v)) {
      dv.setUint32(0, 0x7fc00000, true);
    } else {
      dv.setFloat32(0, v, true);
    }
    const bytes = new Uint8Array(ab);
    this.buf.push(bytes[0], bytes[1], bytes[2], bytes[3]);
  }

  /**
   * Write an f64, canonicalizing NaN to 0x7FF8000000000000.
   */
  writeF64(v: number): void {
    this.align();
    const ab = new ArrayBuffer(8);
    const dv = new DataView(ab);
    if (Number.isNaN(v)) {
      // Canonical qNaN: 0x7FF8000000000000
      dv.setUint32(0, 0x00000000, true); // low 4 bytes LE
      dv.setUint32(4, 0x7ff80000, true); // high 4 bytes LE
    } else {
      dv.setFloat64(0, v, true);
    }
    const bytes = new Uint8Array(ab);
    for (let i = 0; i < 8; i++) {
      this.buf.push(bytes[i]);
    }
  }

  /**
   * Write a LEB128-encoded unsigned integer.
   */
  writeLeb128(value: number): void {
    this.align();
    let v = value;
    do {
      let byte = v & 0x7f;
      v >>>= 7;
      if (v !== 0) {
        byte |= 0x80;
      }
      this.buf.push(byte);
    } while (v !== 0);
  }

  /**
   * Write a LEB128-encoded unsigned 64-bit integer (bigint).
   */
  writeLeb12864(value: bigint): void {
    this.align();
    let v = value < 0n ? value + (1n << 64n) : value; // treat as unsigned
    do {
      let byte = Number(v & 0x7fn);
      v >>= 7n;
      if (v !== 0n) {
        byte |= 0x80;
      }
      this.buf.push(byte);
    } while (v !== 0n);
  }

  /**
   * Write a ZigZag + LEB128 encoded signed integer (up to 32-bit).
   */
  writeZigZag(value: number, typeBits: number): void {
    // ZigZag encode: (n << 1) ^ (n >> (bits - 1))
    const zigzag = (value << 1) ^ (value >> (typeBits - 1));
    this.writeLeb128(zigzag >>> 0); // treat as unsigned
  }

  /**
   * Write a ZigZag + LEB128 encoded signed 64-bit integer (bigint).
   */
  writeZigZag64(value: bigint): void {
    const zigzag = (value << 1n) ^ (value >> 63n);
    this.writeLeb12864(zigzag < 0n ? zigzag + (1n << 64n) : zigzag);
  }

  /**
   * Write a UTF-8 string with a LEB128 length prefix.
   */
  writeString(s: string): void {
    this.align();
    const encoded = textEncoder.encode(s);
    this.writeLeb128Internal(encoded.length);
    for (let i = 0; i < encoded.length; i++) {
      this.buf.push(encoded[i]);
    }
  }

  /**
   * Write a byte array with a LEB128 length prefix.
   */
  writeBytes(data: Uint8Array): void {
    this.align();
    this.writeLeb128Internal(data.length);
    for (let i = 0; i < data.length; i++) {
      this.buf.push(data[i]);
    }
  }

  /**
   * Write raw bytes with no length prefix, aligning first.
   */
  writeRawBytes(data: Uint8Array | number[]): void {
    this.align();
    for (let i = 0; i < data.length; i++) {
      this.buf.push(data[i]);
    }
  }

  /**
   * Increment recursion depth; throws if limit exceeded.
   */
  enterNested(): void {
    this.recursionDepth++;
    if (this.recursionDepth > MAX_RECURSION_DEPTH) {
      throw new Error('Recursion limit exceeded');
    }
  }

  /**
   * Decrement recursion depth.
   */
  leaveNested(): void {
    if (this.recursionDepth > 0) {
      this.recursionDepth--;
    }
  }

  /**
   * Flush any partial byte and return the finished buffer.
   */
  finish(): Uint8Array {
    this.flushToByteBoundary();
    return new Uint8Array(this.buf);
  }

  /**
   * Internal LEB128 encode that does NOT call align() — used by writeString/writeBytes
   * which have already aligned.
   */
  private writeLeb128Internal(value: number): void {
    let v = value;
    do {
      let byte = v & 0x7f;
      v >>>= 7;
      if (v !== 0) {
        byte |= 0x80;
      }
      this.buf.push(byte);
    } while (v !== 0);
  }
}
