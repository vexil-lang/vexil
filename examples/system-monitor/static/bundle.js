// ../../packages/runtime-ts/dist/bit-reader.js
var MAX_RECURSION_DEPTH = 64;
var MAX_BYTES_LENGTH = 1 << 26;
var MAX_LENGTH_PREFIX_BYTES = 4;
var textDecoder = new TextDecoder("utf-8", { fatal: true });
var BitReader = class {
  data;
  bytePos = 0;
  bitOffset = 0;
  recursionDepth = 0;
  constructor(data) {
    this.data = data;
  }
  /**
   * Read `count` bits LSB-first into a number.
   */
  readBits(count) {
    let result = 0;
    for (let i = 0; i < count; i++) {
      if (this.bytePos >= this.data.length) {
        throw new Error("Unexpected end of data");
      }
      const bit = this.data[this.bytePos] >>> this.bitOffset & 1;
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
  readBool() {
    return this.readBits(1) !== 0;
  }
  /**
   * Advance to the next byte boundary, discarding any remaining bits
   * in the current byte.
   */
  flushToByteBoundary() {
    if (this.bitOffset > 0) {
      this.bytePos++;
      this.bitOffset = 0;
    }
  }
  /**
   * Number of remaining bytes from current position.
   */
  remaining() {
    return Math.max(0, this.data.length - this.bytePos);
  }
  /**
   * Read a u8, aligning to a byte boundary first.
   */
  readU8() {
    this.flushToByteBoundary();
    if (this.remaining() < 1) {
      throw new Error("Unexpected end of data");
    }
    const v = this.data[this.bytePos];
    this.bytePos++;
    return v;
  }
  /**
   * Read a little-endian u16, aligning to a byte boundary first.
   */
  readU16() {
    this.flushToByteBoundary();
    if (this.remaining() < 2) {
      throw new Error("Unexpected end of data");
    }
    const v = this.data[this.bytePos] | this.data[this.bytePos + 1] << 8;
    this.bytePos += 2;
    return v;
  }
  /**
   * Read a little-endian u32, aligning to a byte boundary first.
   */
  readU32() {
    this.flushToByteBoundary();
    if (this.remaining() < 4) {
      throw new Error("Unexpected end of data");
    }
    const dv = new DataView(this.data.buffer, this.data.byteOffset + this.bytePos, 4);
    const v = dv.getUint32(0, true);
    this.bytePos += 4;
    return v;
  }
  /**
   * Read a little-endian u64, aligning to a byte boundary first.
   */
  readU64() {
    this.flushToByteBoundary();
    if (this.remaining() < 8) {
      throw new Error("Unexpected end of data");
    }
    const dv = new DataView(this.data.buffer, this.data.byteOffset + this.bytePos, 8);
    const v = dv.getBigUint64(0, true);
    this.bytePos += 8;
    return v;
  }
  /**
   * Read an i8, aligning to a byte boundary first.
   */
  readI8() {
    this.flushToByteBoundary();
    if (this.remaining() < 1) {
      throw new Error("Unexpected end of data");
    }
    const v = this.data[this.bytePos];
    this.bytePos++;
    return v << 24 >> 24;
  }
  /**
   * Read a little-endian i16, aligning to a byte boundary first.
   */
  readI16() {
    this.flushToByteBoundary();
    if (this.remaining() < 2) {
      throw new Error("Unexpected end of data");
    }
    const dv = new DataView(this.data.buffer, this.data.byteOffset + this.bytePos, 2);
    const v = dv.getInt16(0, true);
    this.bytePos += 2;
    return v;
  }
  /**
   * Read a little-endian i32, aligning to a byte boundary first.
   */
  readI32() {
    this.flushToByteBoundary();
    if (this.remaining() < 4) {
      throw new Error("Unexpected end of data");
    }
    const dv = new DataView(this.data.buffer, this.data.byteOffset + this.bytePos, 4);
    const v = dv.getInt32(0, true);
    this.bytePos += 4;
    return v;
  }
  /**
   * Read a little-endian i64, aligning to a byte boundary first.
   */
  readI64() {
    this.flushToByteBoundary();
    if (this.remaining() < 8) {
      throw new Error("Unexpected end of data");
    }
    const dv = new DataView(this.data.buffer, this.data.byteOffset + this.bytePos, 8);
    const v = dv.getBigInt64(0, true);
    this.bytePos += 8;
    return v;
  }
  /**
   * Read a little-endian f32, aligning to a byte boundary first.
   */
  readF32() {
    this.flushToByteBoundary();
    if (this.remaining() < 4) {
      throw new Error("Unexpected end of data");
    }
    const dv = new DataView(this.data.buffer, this.data.byteOffset + this.bytePos, 4);
    const v = dv.getFloat32(0, true);
    this.bytePos += 4;
    return v;
  }
  /**
   * Read a little-endian f64, aligning to a byte boundary first.
   */
  readF64() {
    this.flushToByteBoundary();
    if (this.remaining() < 8) {
      throw new Error("Unexpected end of data");
    }
    const dv = new DataView(this.data.buffer, this.data.byteOffset + this.bytePos, 8);
    const v = dv.getFloat64(0, true);
    this.bytePos += 8;
    return v;
  }
  /**
   * Read a LEB128-encoded unsigned integer.
   */
  readLeb128() {
    this.flushToByteBoundary();
    let result = 0;
    let shift = 0;
    for (let i = 0; i < MAX_LENGTH_PREFIX_BYTES; i++) {
      if (this.bytePos >= this.data.length) {
        throw new Error("Unexpected end of data");
      }
      const byte = this.data[this.bytePos];
      this.bytePos++;
      result |= (byte & 127) << shift;
      shift += 7;
      if ((byte & 128) === 0) {
        if (i > 0 && byte === 0) {
          throw new Error("Invalid varint: overlong encoding");
        }
        return result;
      }
    }
    throw new Error("Invalid varint: exceeds maximum length");
  }
  /**
   * Read a LEB128-encoded unsigned 64-bit integer as bigint.
   */
  readLeb12864() {
    this.flushToByteBoundary();
    let result = 0n;
    let shift = 0n;
    for (let i = 0; i < 10; i++) {
      if (this.bytePos >= this.data.length) {
        throw new Error("Unexpected end of data");
      }
      const byte = this.data[this.bytePos];
      this.bytePos++;
      result |= BigInt(byte & 127) << shift;
      shift += 7n;
      if ((byte & 128) === 0) {
        if (i > 0 && byte === 0) {
          throw new Error("Invalid varint: overlong encoding");
        }
        return result;
      }
    }
    throw new Error("Invalid varint: exceeds maximum length");
  }
  /**
   * Read a ZigZag + LEB128 encoded signed integer (up to 32-bit).
   */
  readZigZag(typeBits) {
    void typeBits;
    const raw = this.readLeb128();
    return raw >>> 1 ^ -(raw & 1);
  }
  /**
   * Read a ZigZag + LEB128 encoded signed 64-bit integer as bigint.
   */
  readZigZag64() {
    const raw = this.readLeb12864();
    return raw >> 1n ^ -(raw & 1n);
  }
  /**
   * Read a length-prefixed UTF-8 string.
   */
  readString() {
    this.flushToByteBoundary();
    const len = this.readLeb128();
    if (len > MAX_BYTES_LENGTH) {
      throw new Error(`String length ${len} exceeds limit ${MAX_BYTES_LENGTH}`);
    }
    if (this.remaining() < len) {
      throw new Error("Unexpected end of data");
    }
    const bytes = this.data.subarray(this.bytePos, this.bytePos + len);
    this.bytePos += len;
    return textDecoder.decode(bytes);
  }
  /**
   * Read a length-prefixed byte array.
   */
  readBytes() {
    this.flushToByteBoundary();
    const len = this.readLeb128();
    if (len > MAX_BYTES_LENGTH) {
      throw new Error(`Bytes length ${len} exceeds limit ${MAX_BYTES_LENGTH}`);
    }
    if (this.remaining() < len) {
      throw new Error("Unexpected end of data");
    }
    const bytes = this.data.slice(this.bytePos, this.bytePos + len);
    this.bytePos += len;
    return bytes;
  }
  /**
   * Read exactly `count` raw bytes with no length prefix.
   */
  readRawBytes(count) {
    this.flushToByteBoundary();
    if (this.remaining() < count) {
      throw new Error("Unexpected end of data");
    }
    const bytes = this.data.slice(this.bytePos, this.bytePos + count);
    this.bytePos += count;
    return bytes;
  }
  /**
   * Increment recursion depth; throws if limit exceeded.
   */
  enterNested() {
    this.recursionDepth++;
    if (this.recursionDepth > MAX_RECURSION_DEPTH) {
      throw new Error("Recursion limit exceeded");
    }
  }
  /**
   * Decrement recursion depth.
   */
  leaveNested() {
    if (this.recursionDepth > 0) {
      this.recursionDepth--;
    }
  }
};

// ts/generated.ts
var SCHEMA_HASH = new Uint8Array([105, 242, 9, 37, 221, 85, 18, 138, 126, 108, 171, 113, 126, 192, 148, 214, 155, 63, 195, 239, 193, 176, 244, 147, 179, 103, 203, 32, 3, 121, 210, 89]);
function decodeCpuStatus(r) {
  const disc = r.readBits(2);
  switch (disc) {
    case 0:
      return "Normal";
    case 1:
      return "Degraded";
    case 2:
      return "Critical";
    default:
      throw new Error(`Unknown CpuStatus discriminant: ${disc}`);
  }
}
var SystemSnapshotDecoder = class {
  prevtimestampMs = 0n;
  prevcpuUsage = 0;
  prevcpuCount = 0;
  prevmemoryUsedMb = 0;
  prevmemoryTotalMb = 0;
  decode(r) {
    const delta_timestamp_ms = r.readI64();
    const timestamp_ms = this.prevtimestampMs + delta_timestamp_ms;
    this.prevtimestampMs = timestamp_ms;
    const hostname = r.readString();
    const delta_cpu_usage = r.readU8();
    const cpu_usage = this.prevcpuUsage + delta_cpu_usage & 255;
    this.prevcpuUsage = cpu_usage;
    const delta_cpu_count = r.readU8();
    const cpu_count = this.prevcpuCount + delta_cpu_count & 255;
    this.prevcpuCount = cpu_count;
    const per_core_usage_len = r.readLeb128();
    const per_core_usage = [];
    for (let i = 0; i < per_core_usage_len; i++) {
      const per_core_usage_item = r.readU8();
      per_core_usage.push(per_core_usage_item);
    }
    const delta_memory_used_mb = r.readU32();
    const memory_used_mb = this.prevmemoryUsedMb + delta_memory_used_mb >>> 0;
    this.prevmemoryUsedMb = memory_used_mb;
    const delta_memory_total_mb = r.readU32();
    const memory_total_mb = this.prevmemoryTotalMb + delta_memory_total_mb >>> 0;
    this.prevmemoryTotalMb = memory_total_mb;
    r.enterNested();
    const cpu_status = decodeCpuStatus(r);
    r.leaveNested();
    r.flushToByteBoundary();
    return { timestamp_ms, hostname, cpu_usage, cpu_count, per_core_usage, memory_used_mb, memory_total_mb, cpu_status };
  }
  reset() {
    this.prevtimestampMs = 0n;
    this.prevcpuUsage = 0;
    this.prevcpuCount = 0;
    this.prevmemoryUsedMb = 0;
    this.prevmemoryTotalMb = 0;
  }
};
export {
  BitReader,
  SystemSnapshotDecoder
};
