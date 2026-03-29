import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { BitWriter } from '../src/bit-writer.js';
import { BitReader } from '../src/bit-reader.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const vectorsDir = join(__dirname, '..', '..', '..', 'compliance', 'vectors');

/** Convert hex string to Uint8Array. */
function hexToBytes(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substring(i, i + 2), 16);
  }
  return bytes;
}

/** Convert Uint8Array to hex string. */
function toHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

interface Vector {
  name: string;
  schema: string;
  type: string;
  value: Record<string, unknown>;
  expected_bytes: string;
  notes?: string;
}

/**
 * Encode a single field value using a BitWriter based on the schema type info.
 * This parses the schema string minimally to determine field types and order.
 */
function encodeValue(
  schema: string,
  value: Record<string, unknown>,
): Uint8Array {
  const fields = parseFields(schema);
  const w = new BitWriter();

  for (const field of fields) {
    const v = value[field.name];
    writeField(w, field.type, v);
  }

  return w.finish();
}

/**
 * Decode fields from bytes using a BitReader based on the schema.
 */
function decodeValue(
  schema: string,
  bytes: Uint8Array,
): Record<string, unknown> {
  const fields = parseFields(schema);
  const r = new BitReader(bytes);
  const result: Record<string, unknown> = {};

  for (const field of fields) {
    result[field.name] = readField(r, field.type);
  }

  return result;
}

interface FieldDef {
  name: string;
  type: string;
}

/**
 * Minimal schema parser: extracts field names and types from a message definition.
 * Handles schemas like: "namespace test.prim\nmessage M { v @0 : bool }"
 */
function parseFields(schema: string): FieldDef[] {
  // Find message body between { }
  const braceStart = schema.indexOf('{');
  const braceEnd = schema.lastIndexOf('}');
  if (braceStart === -1 || braceEnd === -1) return [];

  const body = schema.substring(braceStart + 1, braceEnd).trim();
  if (body.length === 0) return [];

  const fields: FieldDef[] = [];
  // Match: name @N : type
  const fieldRegex = /(\w+)\s+@\d+\s*:\s*(\w+)/g;
  let match;
  while ((match = fieldRegex.exec(body)) !== null) {
    fields.push({ name: match[1], type: match[2] });
  }

  return fields;
}

function writeField(
  w: BitWriter,
  type: string,
  value: unknown,
): void {
  // Handle sub-byte types like u1, u3, u5, u6
  const subByteMatch = type.match(/^u(\d+)$/);
  if (subByteMatch) {
    const bits = parseInt(subByteMatch[1], 10);
    if (bits < 8 && bits >= 1) {
      w.writeBits(value as number, bits);
      return;
    }
  }

  switch (type) {
    case 'bool':
      w.writeBool(value as boolean);
      break;
    case 'u8':
      w.writeU8(value as number);
      break;
    case 'u16':
      w.writeU16(value as number);
      break;
    case 'u32':
      w.writeU32(value as number);
      break;
    case 'u64':
      w.writeU64(BigInt(value as number));
      break;
    case 'i8':
      w.writeI8(value as number);
      break;
    case 'i16':
      w.writeI16(value as number);
      break;
    case 'i32':
      w.writeI32(value as number);
      break;
    case 'i64':
      w.writeI64(BigInt(value as number));
      break;
    case 'f32': {
      const v = value as number | string;
      if (v === 'NaN' || (typeof v === 'string' && v === 'NaN')) {
        w.writeF32(NaN);
      } else if (typeof v === 'string') {
        w.writeF32(parseFloat(v));
      } else {
        w.writeF32(Math.fround(v));
      }
      break;
    }
    case 'f64': {
      const v = value as number | string;
      if (v === 'NaN' || (typeof v === 'string' && v === 'NaN')) {
        w.writeF64(NaN);
      } else if (typeof v === 'string' && v === '-0.0') {
        w.writeF64(-0.0);
      } else if (typeof v === 'string') {
        w.writeF64(parseFloat(v));
      } else {
        w.writeF64(v);
      }
      break;
    }
    case 'string':
      w.writeString(value as string);
      break;
    default:
      throw new Error(`Unsupported type: ${type}`);
  }
}

function readField(r: BitReader, type: string): unknown {
  // Handle sub-byte types like u1, u3, u5, u6
  const subByteMatch = type.match(/^u(\d+)$/);
  if (subByteMatch) {
    const bits = parseInt(subByteMatch[1], 10);
    if (bits < 8 && bits >= 1) {
      return r.readBits(bits);
    }
  }

  switch (type) {
    case 'bool':
      return r.readBool();
    case 'u8':
      return r.readU8();
    case 'u16':
      return r.readU16();
    case 'u32':
      return r.readU32();
    case 'u64':
      return r.readU64();
    case 'i8':
      return r.readI8();
    case 'i16':
      return r.readI16();
    case 'i32':
      return r.readI32();
    case 'i64':
      return r.readI64();
    case 'f32':
      return r.readF32();
    case 'f64':
      return r.readF64();
    case 'string':
      return r.readString();
    default:
      throw new Error(`Unsupported type: ${type}`);
  }
}

/**
 * Compare decoded value against expected, handling special cases
 * like NaN, -0.0, and bigint.
 */
function valuesMatch(
  actual: unknown,
  expected: unknown,
  type: string,
): boolean {
  if (type === 'f32' || type === 'f64') {
    if (expected === 'NaN') {
      return Number.isNaN(actual as number);
    }
    if (expected === '-0.0') {
      return Object.is(actual, -0);
    }
    if (typeof expected === 'string') {
      return (actual as number) === parseFloat(expected);
    }
  }
  if (type === 'u64' || type === 'i64') {
    return (actual as bigint) === BigInt(expected as number);
  }
  return actual === expected;
}

describe('Compliance: primitives.json', () => {
  const vectors: Vector[] = JSON.parse(
    readFileSync(join(vectorsDir, 'primitives.json'), 'utf-8'),
  );

  for (const vec of vectors) {
    describe(vec.name, () => {
      it('encode matches expected bytes', () => {
        const encoded = encodeValue(vec.schema, vec.value);
        expect(toHex(encoded)).toBe(vec.expected_bytes);
      });

      it('decode matches expected value', () => {
        const bytes = hexToBytes(vec.expected_bytes);
        const decoded = decodeValue(vec.schema, bytes);
        const fields = parseFields(vec.schema);
        for (const field of fields) {
          expect(
            valuesMatch(decoded[field.name], vec.value[field.name], field.type),
          ).toBe(true);
        }
      });
    });
  }
});

describe('Compliance: sub_byte.json', () => {
  const vectors: Vector[] = JSON.parse(
    readFileSync(join(vectorsDir, 'sub_byte.json'), 'utf-8'),
  );

  for (const vec of vectors) {
    describe(vec.name, () => {
      it('encode matches expected bytes', () => {
        const encoded = encodeValue(vec.schema, vec.value);
        expect(toHex(encoded)).toBe(vec.expected_bytes);
      });

      it('decode matches expected value', () => {
        const bytes = hexToBytes(vec.expected_bytes);
        const decoded = decodeValue(vec.schema, bytes);
        const fields = parseFields(vec.schema);
        for (const field of fields) {
          expect(
            valuesMatch(decoded[field.name], vec.value[field.name], field.type),
          ).toBe(true);
        }
      });
    });
  }
});

describe('Compliance: messages.json', () => {
  const vectors: Vector[] = JSON.parse(
    readFileSync(join(vectorsDir, 'messages.json'), 'utf-8'),
  );

  for (const vec of vectors) {
    describe(vec.name, () => {
      it('encode matches expected bytes', () => {
        const encoded = encodeValue(vec.schema, vec.value);
        expect(toHex(encoded)).toBe(vec.expected_bytes);
      });

      // For messages with fields, also test decode
      if (Object.keys(vec.value).length > 0) {
        it('decode matches expected value', () => {
          const bytes = hexToBytes(vec.expected_bytes);
          const decoded = decodeValue(vec.schema, bytes);
          const fields = parseFields(vec.schema);
          for (const field of fields) {
            expect(
              valuesMatch(
                decoded[field.name],
                vec.value[field.name],
                field.type,
              ),
            ).toBe(true);
          }
        });
      }
    });
  }
});

describe('Compliance: optionals.json', () => {
  interface OptionalVector {
    name: string;
    schema: string;
    type: string;
    value: Record<string, unknown>;
    expected_bytes: string;
    notes?: string;
  }

  const vectors: OptionalVector[] = JSON.parse(
    readFileSync(join(vectorsDir, 'optionals.json'), 'utf-8'),
  );

  for (const vec of vectors) {
    describe(vec.name, () => {
      it('encode matches expected bytes', () => {
        const w = new BitWriter();
        const v = vec.value.v;
        if (v === null) {
          w.writeBool(false);
        } else {
          w.writeBool(true);
          w.flushToByteBoundary();
          w.writeU32(v as number);
        }
        expect(toHex(w.finish())).toBe(vec.expected_bytes);
      });

      it('decode matches expected value', () => {
        const r = new BitReader(hexToBytes(vec.expected_bytes));
        const present = r.readBool();
        if (!present) {
          expect(vec.value.v).toBeNull();
        } else {
          r.flushToByteBoundary();
          const val = r.readU32();
          expect(val).toBe(vec.value.v);
        }
      });
    });
  }
});

describe('Compliance: enums.json', () => {
  interface EnumVector {
    name: string;
    schema: string;
    type: string;
    value: Record<string, unknown>;
    expected_bytes: string;
    notes?: string;
  }

  const vectors: EnumVector[] = JSON.parse(
    readFileSync(join(vectorsDir, 'enums.json'), 'utf-8'),
  );

  const variantMap: Record<string, number> = { Active: 0, Inactive: 1 };
  const indexToVariant = ['Active', 'Inactive'];

  for (const vec of vectors) {
    describe(vec.name, () => {
      it('encode matches expected bytes', () => {
        const w = new BitWriter();
        const variant = vec.value.v as string;
        w.writeBits(variantMap[variant], 1);
        expect(toHex(w.finish())).toBe(vec.expected_bytes);
      });

      it('decode matches expected value', () => {
        const r = new BitReader(hexToBytes(vec.expected_bytes));
        const idx = r.readBits(1);
        expect(indexToVariant[idx]).toBe(vec.value.v);
      });
    });
  }
});

describe('Compliance: unions.json', () => {
  interface UnionVector {
    name: string;
    schema: string;
    type: string;
    value: Record<string, unknown>;
    expected_bytes: string;
    notes?: string;
  }

  const vectors: UnionVector[] = JSON.parse(
    readFileSync(join(vectorsDir, 'unions.json'), 'utf-8'),
  );

  for (const vec of vectors) {
    describe(vec.name, () => {
      it('encode matches expected bytes', () => {
        const w = new BitWriter();
        const unionVal = vec.value.v as Record<string, unknown>;
        const variant = unionVal.variant as string;
        const discriminant = variant === 'Circle' ? 0 : 1;

        const pw = new BitWriter();
        if (variant === 'Circle') {
          pw.writeF32(Math.fround(unionVal.radius as number));
        } else {
          pw.writeF32(Math.fround(unionVal.w as number));
          pw.writeF32(Math.fround(unionVal.h as number));
        }
        const payload = pw.finish();

        w.writeLeb128(discriminant);
        w.writeLeb128(payload.length);
        w.writeRawBytes(payload);

        expect(toHex(w.finish())).toBe(vec.expected_bytes);
      });

      it('decode matches expected value', () => {
        const r = new BitReader(hexToBytes(vec.expected_bytes));
        const discriminant = r.readLeb128();
        const payloadLen = r.readLeb128();
        const payloadBytes = r.readRawBytes(payloadLen);
        const pr = new BitReader(payloadBytes);

        const unionVal = vec.value.v as Record<string, unknown>;
        if (discriminant === 0) {
          const radius = pr.readF32();
          expect(radius).toBeCloseTo(unionVal.radius as number, 5);
        } else {
          const w = pr.readF32();
          const h = pr.readF32();
          expect(w).toBeCloseTo(unionVal.w as number, 5);
          expect(h).toBeCloseTo(unionVal.h as number, 5);
        }
      });
    });
  }
});

describe('Compliance: arrays_maps.json', () => {
  interface ArrayMapVector {
    name: string;
    schema: string;
    type: string;
    value: Record<string, unknown>;
    expected_bytes: string;
    notes?: string;
  }

  const vectors: ArrayMapVector[] = JSON.parse(
    readFileSync(join(vectorsDir, 'arrays_maps.json'), 'utf-8'),
  );

  for (const vec of vectors) {
    describe(vec.name, () => {
      it('encode matches expected bytes', () => {
        const w = new BitWriter();
        const v = vec.value.v;

        if (Array.isArray(v)) {
          w.writeLeb128(v.length);
          for (const elem of v) {
            w.writeU32(elem as number);
          }
        } else if (typeof v === 'object' && v !== null) {
          const entries = Object.entries(v as Record<string, unknown>);
          w.writeLeb128(entries.length);
          for (const [key, val] of entries) {
            w.writeString(key);
            w.writeU32(val as number);
          }
        }

        expect(toHex(w.finish())).toBe(vec.expected_bytes);
      });

      it('decode matches expected value', () => {
        const r = new BitReader(hexToBytes(vec.expected_bytes));
        const v = vec.value.v;

        if (Array.isArray(v)) {
          const count = r.readLeb128();
          expect(count).toBe(v.length);
          for (let i = 0; i < count; i++) {
            expect(r.readU32()).toBe(v[i]);
          }
        } else if (typeof v === 'object' && v !== null) {
          const expected = v as Record<string, unknown>;
          const count = r.readLeb128();
          expect(count).toBe(Object.keys(expected).length);
          for (let i = 0; i < count; i++) {
            const key = r.readString();
            const val = r.readU32();
            expect(expected[key]).toBe(val);
          }
        }
      });
    });
  }
});

describe('Compliance: evolution.json', () => {
  interface EvolutionVector {
    name: string;
    schema_v1: string;
    schema_v2: string;
    type: string;
    value_v1?: Record<string, unknown>;
    value_v2?: Record<string, unknown>;
    encoded_v1?: string;
    encoded_v2?: string;
    decoded_as_v1?: Record<string, unknown>;
    decoded_as_v2?: Record<string, unknown>;
    notes?: string;
  }

  const vectors: EvolutionVector[] = JSON.parse(
    readFileSync(join(vectorsDir, 'evolution.json'), 'utf-8'),
  );

  it('v1 encode produces expected bytes', () => {
    const vec = vectors.find(
      (v) => v.name === 'v1_encode_v2_decode_appended_field',
    )!;
    const w = new BitWriter();
    w.writeU32(vec.value_v1!.x as number);
    expect(toHex(w.finish())).toBe(vec.encoded_v1);
  });

  it('v1 bytes decoded as v2 fills default for missing field', () => {
    const vec = vectors.find(
      (v) => v.name === 'v1_encode_v2_decode_appended_field',
    )!;
    const r = new BitReader(hexToBytes(vec.encoded_v1!));
    const x = r.readU32();
    const y = r.remaining() >= 2 ? r.readU16() : 0;
    expect(x).toBe(vec.decoded_as_v2!.x);
    expect(y).toBe(vec.decoded_as_v2!.y);
  });

  it('v2 encode produces expected bytes', () => {
    const vec = vectors.find(
      (v) => v.name === 'v2_encode_v1_decode_trailing_ignored',
    )!;
    const w = new BitWriter();
    w.writeU32(vec.value_v2!.x as number);
    w.writeU16(vec.value_v2!.y as number);
    expect(toHex(w.finish())).toBe(vec.encoded_v2);
  });

  it('v2 bytes decoded as v1 ignores trailing bytes', () => {
    const vec = vectors.find(
      (v) => v.name === 'v2_encode_v1_decode_trailing_ignored',
    )!;
    const r = new BitReader(hexToBytes(vec.encoded_v2!));
    const x = r.readU32();
    expect(x).toBe(vec.decoded_as_v1!.x);
    expect(r.remaining()).toBeGreaterThan(0);
  });
});
