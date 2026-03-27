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
