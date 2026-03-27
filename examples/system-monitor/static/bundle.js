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
   * Read all remaining bytes from the current position to the end.
   * Flushes to byte boundary first. Returns an empty Uint8Array if no bytes remain.
   */
  readRemaining() {
    this.flushToByteBoundary();
    if (this.bytePos >= this.data.length) {
      return new Uint8Array(0);
    }
    const result = this.data.slice(this.bytePos);
    this.bytePos = this.data.length;
    return result;
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

// ../../packages/runtime-ts/dist/bit-writer.js
var MAX_RECURSION_DEPTH2 = 64;
var textEncoder = new TextEncoder();
var BitWriter = class {
  buf = [];
  currentByte = 0;
  bitOffset = 0;
  recursionDepth = 0;
  /**
   * Internal: align to a byte boundary without the "empty = zero byte" rule.
   * Used before multi-byte writes to ensure alignment.
   */
  align() {
    if (this.bitOffset > 0) {
      this.buf.push(this.currentByte);
      this.currentByte = 0;
      this.bitOffset = 0;
    }
  }
  /**
   * Write `count` bits from `value`, LSB first.
   */
  writeBits(value, count) {
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
  writeBool(v) {
    this.writeBits(v ? 1 : 0, 1);
  }
  /**
   * Flush any partial byte to the buffer.
   *
   * Special case per spec section 4.1: if nothing has been written at all
   * (bitOffset == 0 AND buf is empty), push a zero byte anyway.
   * If bitOffset == 0 and buf is non-empty, this is a no-op.
   */
  flushToByteBoundary() {
    if (this.bitOffset === 0) {
      if (this.buf.length === 0) {
        this.buf.push(0);
      }
    } else {
      this.buf.push(this.currentByte);
      this.currentByte = 0;
      this.bitOffset = 0;
    }
  }
  /**
   * Write a u8, aligning to a byte boundary first.
   */
  writeU8(v) {
    this.align();
    this.buf.push(v & 255);
  }
  /**
   * Write a u16 in little-endian byte order, aligning first.
   */
  writeU16(v) {
    this.align();
    this.buf.push(v & 255);
    this.buf.push(v >>> 8 & 255);
  }
  /**
   * Write a u32 in little-endian byte order, aligning first.
   */
  writeU32(v) {
    this.align();
    this.buf.push(v & 255);
    this.buf.push(v >>> 8 & 255);
    this.buf.push(v >>> 16 & 255);
    this.buf.push(v >>> 24 & 255);
  }
  /**
   * Write a u64 in little-endian byte order, aligning first.
   */
  writeU64(v) {
    this.align();
    const mask = BigInt(255);
    for (let i = 0; i < 8; i++) {
      this.buf.push(Number(v >> BigInt(i * 8) & mask));
    }
  }
  /**
   * Write an i8, aligning to a byte boundary first.
   */
  writeI8(v) {
    this.align();
    this.buf.push(v & 255);
  }
  /**
   * Write an i16 in little-endian byte order, aligning first.
   */
  writeI16(v) {
    this.align();
    const ab = new ArrayBuffer(2);
    new DataView(ab).setInt16(0, v, true);
    const bytes = new Uint8Array(ab);
    this.buf.push(bytes[0], bytes[1]);
  }
  /**
   * Write an i32 in little-endian byte order, aligning first.
   */
  writeI32(v) {
    this.align();
    const ab = new ArrayBuffer(4);
    new DataView(ab).setInt32(0, v, true);
    const bytes = new Uint8Array(ab);
    this.buf.push(bytes[0], bytes[1], bytes[2], bytes[3]);
  }
  /**
   * Write an i64 in little-endian byte order, aligning first.
   */
  writeI64(v) {
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
  writeF32(v) {
    this.align();
    const ab = new ArrayBuffer(4);
    const dv = new DataView(ab);
    if (Number.isNaN(v)) {
      dv.setUint32(0, 2143289344, true);
    } else {
      dv.setFloat32(0, v, true);
    }
    const bytes = new Uint8Array(ab);
    this.buf.push(bytes[0], bytes[1], bytes[2], bytes[3]);
  }
  /**
   * Write an f64, canonicalizing NaN to 0x7FF8000000000000.
   */
  writeF64(v) {
    this.align();
    const ab = new ArrayBuffer(8);
    const dv = new DataView(ab);
    if (Number.isNaN(v)) {
      dv.setUint32(0, 0, true);
      dv.setUint32(4, 2146959360, true);
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
  writeLeb128(value) {
    this.align();
    let v = value;
    do {
      let byte = v & 127;
      v >>>= 7;
      if (v !== 0) {
        byte |= 128;
      }
      this.buf.push(byte);
    } while (v !== 0);
  }
  /**
   * Write a LEB128-encoded unsigned 64-bit integer (bigint).
   */
  writeLeb12864(value) {
    this.align();
    let v = value < 0n ? value + (1n << 64n) : value;
    do {
      let byte = Number(v & 0x7fn);
      v >>= 7n;
      if (v !== 0n) {
        byte |= 128;
      }
      this.buf.push(byte);
    } while (v !== 0n);
  }
  /**
   * Write a ZigZag + LEB128 encoded signed integer (up to 32-bit).
   */
  writeZigZag(value, typeBits) {
    const zigzag = value << 1 ^ value >> typeBits - 1;
    this.writeLeb128(zigzag >>> 0);
  }
  /**
   * Write a ZigZag + LEB128 encoded signed 64-bit integer (bigint).
   */
  writeZigZag64(value) {
    const zigzag = value << 1n ^ value >> 63n;
    this.writeLeb12864(zigzag < 0n ? zigzag + (1n << 64n) : zigzag);
  }
  /**
   * Write a UTF-8 string with a LEB128 length prefix.
   */
  writeString(s) {
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
  writeBytes(data) {
    this.align();
    this.writeLeb128Internal(data.length);
    for (let i = 0; i < data.length; i++) {
      this.buf.push(data[i]);
    }
  }
  /**
   * Write raw bytes with no length prefix, aligning first.
   */
  writeRawBytes(data) {
    this.align();
    for (let i = 0; i < data.length; i++) {
      this.buf.push(data[i]);
    }
  }
  /**
   * Increment recursion depth; throws if limit exceeded.
   */
  enterNested() {
    this.recursionDepth++;
    if (this.recursionDepth > MAX_RECURSION_DEPTH2) {
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
  /**
   * Flush any partial byte and return the finished buffer.
   */
  finish() {
    this.flushToByteBoundary();
    return new Uint8Array(this.buf);
  }
  /**
   * Internal LEB128 encode that does NOT call align() — used by writeString/writeBytes
   * which have already aligned.
   */
  writeLeb128Internal(value) {
    let v = value;
    do {
      let byte = v & 127;
      v >>>= 7;
      if (v !== 0) {
        byte |= 128;
      }
      this.buf.push(byte);
    } while (v !== 0);
  }
};

// ../../packages/runtime-ts/dist/handshake.js
var SchemaHandshake = class _SchemaHandshake {
  hash;
  version;
  constructor(hash, version) {
    this.hash = hash;
    this.version = version;
  }
  encode() {
    const w = new BitWriter();
    w.writeRawBytes(this.hash);
    w.writeString(this.version);
    return w.finish();
  }
  static decode(bytes) {
    const r = new BitReader(bytes);
    const hash = r.readRawBytes(32);
    const version = r.readString();
    return new _SchemaHandshake(hash, version);
  }
  check(remote) {
    const match_ = this.hash.length === remote.hash.length && this.hash.every((b, i) => b === remote.hash[i]);
    if (match_) {
      return { kind: "match" };
    }
    return {
      kind: "version_mismatch",
      localVersion: this.version,
      remoteVersion: remote.version,
      localHash: this.hash,
      remoteHash: remote.hash
    };
  }
};

// ts/generated.ts
var SCHEMA_HASH = new Uint8Array([209, 153, 1, 111, 193, 65, 90, 56, 37, 129, 177, 44, 104, 246, 21, 59, 13, 132, 208, 234, 66, 44, 107, 83, 228, 119, 125, 26, 97, 173, 2, 146]);
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
    const delta_timestamp_ms = r.readZigZag64();
    const timestamp_ms = this.prevtimestampMs + delta_timestamp_ms;
    this.prevtimestampMs = timestamp_ms;
    const hostname = r.readString();
    const delta_cpu_usage = r.readLeb128();
    const cpu_usage = this.prevcpuUsage + delta_cpu_usage & 255;
    this.prevcpuUsage = cpu_usage;
    const delta_cpu_count = r.readLeb128();
    const cpu_count = this.prevcpuCount + delta_cpu_count & 255;
    this.prevcpuCount = cpu_count;
    const per_core_usage_len = r.readLeb128();
    const per_core_usage = [];
    for (let i = 0; i < per_core_usage_len; i++) {
      const per_core_usage_item = r.readU8();
      per_core_usage.push(per_core_usage_item);
    }
    const delta_memory_used_mb = r.readLeb128();
    const memory_used_mb = this.prevmemoryUsedMb + delta_memory_used_mb >>> 0;
    this.prevmemoryUsedMb = memory_used_mb;
    const delta_memory_total_mb = r.readLeb128();
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
  SCHEMA_HASH,
  SchemaHandshake,
  SystemSnapshotDecoder
};
