import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { BitWriter } from '../src/bit-writer.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const vectorsDir = join(__dirname, '../../../compliance/vectors');

function toHex(bytes: Uint8Array): string {
  return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('');
}

interface DeltaFrame {
  value?: Record<string, unknown>;
  expected_bytes?: string;
  reset?: boolean;
}

interface DeltaVector {
  name: string;
  frames: DeltaFrame[];
}

const vectors: DeltaVector[] = JSON.parse(
  readFileSync(join(vectorsDir, 'delta.json'), 'utf-8')
);

describe('delta compliance', () => {
  it('delta_u32_two_frames', () => {
    const v = vectors.find(v => v.name === 'delta_u32_two_frames')!;
    let prev = 0;
    for (const frame of v.frames) {
      if (frame.reset) { prev = 0; continue; }
      const val = frame.value!.v as number;
      const delta = (val - prev) >>> 0;
      const w = new BitWriter();
      w.writeU32(delta);
      w.flushToByteBoundary();
      expect(toHex(w.finish())).toBe(frame.expected_bytes);
      prev = val;
    }
  });

  it('delta_i64_three_frames', () => {
    const v = vectors.find(v => v.name === 'delta_i64_three_frames')!;
    let prev = 0n;
    for (const frame of v.frames) {
      if (frame.reset) { prev = 0n; continue; }
      const val = BigInt(frame.value!.v as number);
      const delta = val - prev;
      const w = new BitWriter();
      w.writeI64(delta);
      w.flushToByteBoundary();
      expect(toHex(w.finish())).toBe(frame.expected_bytes);
      prev = val;
    }
  });

  it('delta_mixed_message', () => {
    const v = vectors.find(v => v.name === 'delta_mixed_message')!;
    let prevTs = 0n;
    let prevCount = 0;
    for (const frame of v.frames) {
      if (frame.reset) { prevTs = 0n; prevCount = 0; continue; }
      const ts = BigInt(frame.value!.ts as number);
      const label = frame.value!.label as string;
      const count = frame.value!.count as number;
      const w = new BitWriter();
      w.writeI64(ts - prevTs);
      w.writeString(label);
      w.writeU32((count - prevCount) >>> 0);
      w.flushToByteBoundary();
      expect(toHex(w.finish())).toBe(frame.expected_bytes);
      prevTs = ts;
      prevCount = count;
    }
  });

  it('delta_reset', () => {
    const v = vectors.find(v => v.name === 'delta_reset')!;
    let prev = 0;
    for (const frame of v.frames) {
      if (frame.reset) { prev = 0; continue; }
      const val = frame.value!.v as number;
      const delta = (val - prev) >>> 0;
      const w = new BitWriter();
      w.writeU32(delta);
      w.flushToByteBoundary();
      expect(toHex(w.finish())).toBe(frame.expected_bytes);
      prev = val;
    }
  });
});
