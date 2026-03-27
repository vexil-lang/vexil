const MAX_RECURSION_DEPTH = 64;
const MAX_BYTES_LENGTH = 1 << 26; // 64 MiB
const MAX_LENGTH_PREFIX_BYTES = 4;

const textDecoder = new TextDecoder('utf-8', { fatal: true });

/**
 * A cursor over a byte buffer that reads fields LSB-first at the bit level.
 *
 * Created with `new BitReader(data)`, consumed with `read*` methods. Tracks
 * a byte position and a sub-byte bit offset, plus a recursion depth counter
 * for safely decoding recursive types.
 *
 * Sub-byte reads pull individual bits from the current byte. Multi-byte reads
 * (e.g. `readU16`) first align to the next byte boundary, then interpret the
 * bytes as little-endian.
 */
export class BitReader {
  private data: Uint8Array;
  private bytePos = 0;
  private bitOffset = 0;
  private recursionDepth = 0;

  constructor(data: Uint8Array) {
    this.data = data;
  }

  /**
   * Read `count` bits LSB-first into a number.
   */
  readBits(count: number): number {
    let result = 0;
    for (let i = 0; i < count; i++) {
      if (this.bytePos >= this.data.length) {
        throw new Error('Unexpected end of data');
      }
      const bit = (this.data[this.bytePos] >>> this.bitOffset) & 1;
      result |= bit << i;
      this.bitOffset++;
      if (this.bitOffset === 8) {
        this.bytePos++;
        this.bitOffset = 0;
      }
    }
    return result;
  }

  /**
   * Read a single bit as boolean.
   */
  readBool(): boolean {
    return this.readBits(1) !== 0;
  }

  /**
   * Advance to the next byte boundary, discarding any remaining bits
   * in the current byte.
   */
  flushToByteBoundary(): void {
    if (this.bitOffset > 0) {
      this.bytePos++;
      this.bitOffset = 0;
    }
  }

  /**
   * Number of remaining bytes from current position.
   */
  remaining(): number {
    return Math.max(0, this.data.length - this.bytePos);
  }

  /**
   * Read a u8, aligning to a byte boundary first.
   */
  readU8(): number {
    this.flushToByteBoundary();
    if (this.remaining() < 1) {
      throw new Error('Unexpected end of data');
    }
    const v = this.data[this.bytePos];
    this.bytePos++;
    return v;
  }

  /**
   * Read a little-endian u16, aligning to a byte boundary first.
   */
  readU16(): number {
    this.flushToByteBoundary();
    if (this.remaining() < 2) {
      throw new Error('Unexpected end of data');
    }
    const v =
      this.data[this.bytePos] |
      (this.data[this.bytePos + 1] << 8);
    this.bytePos += 2;
    return v;
  }

  /**
   * Read a little-endian u32, aligning to a byte boundary first.
   */
  readU32(): number {
    this.flushToByteBoundary();
    if (this.remaining() < 4) {
      throw new Error('Unexpected end of data');
    }
    const dv = new DataView(
      this.data.buffer,
      this.data.byteOffset + this.bytePos,
      4,
    );
    const v = dv.getUint32(0, true);
    this.bytePos += 4;
    return v;
  }

  /**
   * Read a little-endian u64, aligning to a byte boundary first.
   */
  readU64(): bigint {
    this.flushToByteBoundary();
    if (this.remaining() < 8) {
      throw new Error('Unexpected end of data');
    }
    const dv = new DataView(
      this.data.buffer,
      this.data.byteOffset + this.bytePos,
      8,
    );
    const v = dv.getBigUint64(0, true);
    this.bytePos += 8;
    return v;
  }

  /**
   * Read an i8, aligning to a byte boundary first.
   */
  readI8(): number {
    this.flushToByteBoundary();
    if (this.remaining() < 1) {
      throw new Error('Unexpected end of data');
    }
    const v = this.data[this.bytePos];
    this.bytePos++;
    // Sign extend from 8 bits
    return (v << 24) >> 24;
  }

  /**
   * Read a little-endian i16, aligning to a byte boundary first.
   */
  readI16(): number {
    this.flushToByteBoundary();
    if (this.remaining() < 2) {
      throw new Error('Unexpected end of data');
    }
    const dv = new DataView(
      this.data.buffer,
      this.data.byteOffset + this.bytePos,
      2,
    );
    const v = dv.getInt16(0, true);
    this.bytePos += 2;
    return v;
  }

  /**
   * Read a little-endian i32, aligning to a byte boundary first.
   */
  readI32(): number {
    this.flushToByteBoundary();
    if (this.remaining() < 4) {
      throw new Error('Unexpected end of data');
    }
    const dv = new DataView(
      this.data.buffer,
      this.data.byteOffset + this.bytePos,
      4,
    );
    const v = dv.getInt32(0, true);
    this.bytePos += 4;
    return v;
  }

  /**
   * Read a little-endian i64, aligning to a byte boundary first.
   */
  readI64(): bigint {
    this.flushToByteBoundary();
    if (this.remaining() < 8) {
      throw new Error('Unexpected end of data');
    }
    const dv = new DataView(
      this.data.buffer,
      this.data.byteOffset + this.bytePos,
      8,
    );
    const v = dv.getBigInt64(0, true);
    this.bytePos += 8;
    return v;
  }

  /**
   * Read a little-endian f32, aligning to a byte boundary first.
   */
  readF32(): number {
    this.flushToByteBoundary();
    if (this.remaining() < 4) {
      throw new Error('Unexpected end of data');
    }
    const dv = new DataView(
      this.data.buffer,
      this.data.byteOffset + this.bytePos,
      4,
    );
    const v = dv.getFloat32(0, true);
    this.bytePos += 4;
    return v;
  }

  /**
   * Read a little-endian f64, aligning to a byte boundary first.
   */
  readF64(): number {
    this.flushToByteBoundary();
    if (this.remaining() < 8) {
      throw new Error('Unexpected end of data');
    }
    const dv = new DataView(
      this.data.buffer,
      this.data.byteOffset + this.bytePos,
      8,
    );
    const v = dv.getFloat64(0, true);
    this.bytePos += 8;
    return v;
  }

  /**
   * Read a LEB128-encoded unsigned integer.
   */
  readLeb128(): number {
    this.flushToByteBoundary();
    let result = 0;
    let shift = 0;
    for (let i = 0; i < MAX_LENGTH_PREFIX_BYTES; i++) {
      if (this.bytePos >= this.data.length) {
        throw new Error('Unexpected end of data');
      }
      const byte = this.data[this.bytePos];
      this.bytePos++;
      result |= (byte & 0x7f) << shift;
      shift += 7;
      if ((byte & 0x80) === 0) {
        // Reject overlong: if not first byte and byte is 0
        if (i > 0 && byte === 0) {
          throw new Error('Invalid varint: overlong encoding');
        }
        return result;
      }
    }
    throw new Error('Invalid varint: exceeds maximum length');
  }

  /**
   * Read a LEB128-encoded unsigned 64-bit integer as bigint.
   */
  readLeb12864(): bigint {
    this.flushToByteBoundary();
    let result = 0n;
    let shift = 0n;
    for (let i = 0; i < 10; i++) {
      if (this.bytePos >= this.data.length) {
        throw new Error('Unexpected end of data');
      }
      const byte = this.data[this.bytePos];
      this.bytePos++;
      result |= BigInt(byte & 0x7f) << shift;
      shift += 7n;
      if ((byte & 0x80) === 0) {
        if (i > 0 && byte === 0) {
          throw new Error('Invalid varint: overlong encoding');
        }
        return result;
      }
    }
    throw new Error('Invalid varint: exceeds maximum length');
  }

  /**
   * Read a ZigZag + LEB128 encoded signed integer (up to 32-bit).
   */
  readZigZag(typeBits: number): number {
    void typeBits; // type_bits not needed for decode
    const raw = this.readLeb128();
    return (raw >>> 1) ^ -(raw & 1);
  }

  /**
   * Read a ZigZag + LEB128 encoded signed 64-bit integer as bigint.
   */
  readZigZag64(): bigint {
    const raw = this.readLeb12864();
    return (raw >> 1n) ^ -(raw & 1n);
  }

  /**
   * Read a length-prefixed UTF-8 string.
   */
  readString(): string {
    this.flushToByteBoundary();
    const len = this.readLeb128();
    if (len > MAX_BYTES_LENGTH) {
      throw new Error(`String length ${len} exceeds limit ${MAX_BYTES_LENGTH}`);
    }
    if (this.remaining() < len) {
      throw new Error('Unexpected end of data');
    }
    const bytes = this.data.subarray(this.bytePos, this.bytePos + len);
    this.bytePos += len;
    return textDecoder.decode(bytes);
  }

  /**
   * Read a length-prefixed byte array.
   */
  readBytes(): Uint8Array {
    this.flushToByteBoundary();
    const len = this.readLeb128();
    if (len > MAX_BYTES_LENGTH) {
      throw new Error(`Bytes length ${len} exceeds limit ${MAX_BYTES_LENGTH}`);
    }
    if (this.remaining() < len) {
      throw new Error('Unexpected end of data');
    }
    const bytes = this.data.slice(this.bytePos, this.bytePos + len);
    this.bytePos += len;
    return bytes;
  }

  /**
   * Read exactly `count` raw bytes with no length prefix.
   */
  readRawBytes(count: number): Uint8Array {
    this.flushToByteBoundary();
    if (this.remaining() < count) {
      throw new Error('Unexpected end of data');
    }
    const bytes = this.data.slice(this.bytePos, this.bytePos + count);
    this.bytePos += count;
    return bytes;
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
}
