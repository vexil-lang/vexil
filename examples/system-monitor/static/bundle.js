// ts/generated.ts
var SCHEMA_HASH = new Uint8Array([209, 153, 1, 111, 193, 65, 90, 56, 37, 129, 177, 44, 104, 246, 21, 59, 13, 132, 208, 234, 66, 44, 107, 83, 228, 119, 125, 26, 97, 173, 2, 146]);
var CpuStatus = {
  Normal: "Normal",
  Degraded: "Degraded",
  Critical: "Critical"
};
function encodeCpuStatus(v, w) {
  let disc;
  switch (v) {
    case "Normal":
      disc = 0;
      break;
    case "Degraded":
      disc = 1;
      break;
    case "Critical":
      disc = 2;
      break;
    default:
      throw new Error(`Unknown CpuStatus variant: ${v}`);
  }
  w.writeBits(disc, 2);
}
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
function encodeSystemSnapshot(v, w) {
  w.writeZigZag64(v.timestamp_ms);
  w.writeString(v.hostname);
  w.writeLeb128(v.cpu_usage);
  w.writeLeb128(v.cpu_count);
  w.writeLeb128(v.per_core_usage.length);
  for (const item of v.per_core_usage) {
    w.writeU8(item);
  }
  w.writeLeb128(v.memory_used_mb);
  w.writeLeb128(v.memory_total_mb);
  w.enterNested();
  encodeCpuStatus(v.cpu_status, w);
  w.leaveNested();
  w.flushToByteBoundary();
}
function decodeSystemSnapshot(r) {
  const timestamp_ms = r.readZigZag64();
  const hostname = r.readString();
  const cpu_usage = r.readLeb128();
  const cpu_count = r.readLeb128();
  const per_core_usage_len = r.readLeb128();
  const per_core_usage = [];
  for (let i = 0; i < per_core_usage_len; i++) {
    const per_core_usage_item = r.readU8();
    per_core_usage.push(per_core_usage_item);
  }
  const memory_used_mb = r.readLeb128();
  const memory_total_mb = r.readLeb128();
  r.enterNested();
  const cpu_status = decodeCpuStatus(r);
  r.leaveNested();
  r.flushToByteBoundary();
  return { timestamp_ms, hostname, cpu_usage, cpu_count, per_core_usage, memory_used_mb, memory_total_mb, cpu_status };
}
var SystemSnapshotEncoder = class {
  prevtimestampMs = 0n;
  prevcpuUsage = 0;
  prevcpuCount = 0;
  prevmemoryUsedMb = 0;
  prevmemoryTotalMb = 0;
  encode(v, w) {
    const delta_timestamp_ms = v.timestamp_ms - this.prevtimestampMs;
    w.writeZigZag64(delta_timestamp_ms);
    this.prevtimestampMs = v.timestamp_ms;
    w.writeString(v.hostname);
    const delta_cpu_usage = v.cpu_usage - this.prevcpuUsage & 255;
    w.writeLeb128(delta_cpu_usage);
    this.prevcpuUsage = v.cpu_usage;
    const delta_cpu_count = v.cpu_count - this.prevcpuCount & 255;
    w.writeLeb128(delta_cpu_count);
    this.prevcpuCount = v.cpu_count;
    w.writeLeb128(v.per_core_usage.length);
    for (const item of v.per_core_usage) {
      w.writeU8(item);
    }
    const delta_memory_used_mb = v.memory_used_mb - this.prevmemoryUsedMb >>> 0;
    w.writeLeb128(delta_memory_used_mb);
    this.prevmemoryUsedMb = v.memory_used_mb;
    const delta_memory_total_mb = v.memory_total_mb - this.prevmemoryTotalMb >>> 0;
    w.writeLeb128(delta_memory_total_mb);
    this.prevmemoryTotalMb = v.memory_total_mb;
    w.enterNested();
    encodeCpuStatus(v.cpu_status, w);
    w.leaveNested();
    w.flushToByteBoundary();
  }
  reset() {
    this.prevtimestampMs = 0n;
    this.prevcpuUsage = 0;
    this.prevcpuCount = 0;
    this.prevmemoryUsedMb = 0;
    this.prevmemoryTotalMb = 0;
  }
};
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
  CpuStatus,
  SCHEMA_HASH,
  SystemSnapshotDecoder,
  SystemSnapshotEncoder,
  decodeCpuStatus,
  decodeSystemSnapshot,
  encodeCpuStatus,
  encodeSystemSnapshot
};
