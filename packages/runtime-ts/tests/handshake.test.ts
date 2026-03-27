import { describe, it, expect } from 'vitest';
import { SchemaHandshake } from '../src/handshake.js';

describe('SchemaHandshake', () => {
  it('encode/decode roundtrip', () => {
    const hash = new Uint8Array(32).fill(0xab);
    const hs = new SchemaHandshake(hash, '1.2.3');
    const bytes = hs.encode();
    const decoded = SchemaHandshake.decode(bytes);
    expect(Array.from(decoded.hash)).toEqual(Array.from(hash));
    expect(decoded.version).toBe('1.2.3');
  });

  it('check matching hashes', () => {
    const hash = new Uint8Array(32).fill(0x42);
    const local = new SchemaHandshake(hash, '1.0.0');
    const remote = new SchemaHandshake(hash, '1.0.0');
    expect(local.check(remote)).toEqual({ kind: 'match' });
  });

  it('check different hashes', () => {
    const local = new SchemaHandshake(new Uint8Array(32).fill(0x01), '1.0.0');
    const remote = new SchemaHandshake(
      new Uint8Array(32).fill(0x02),
      '1.1.0',
    );
    const result = local.check(remote);
    expect(result.kind).toBe('version_mismatch');
    if (result.kind === 'version_mismatch') {
      expect(result.localVersion).toBe('1.0.0');
      expect(result.remoteVersion).toBe('1.1.0');
    }
  });

  it('wire size is compact', () => {
    const hs = new SchemaHandshake(new Uint8Array(32), '1.0.0');
    const bytes = hs.encode();
    expect(bytes.length).toBe(38);
  });

  it('cross-language wire format', () => {
    const hash = new Uint8Array(32);
    hash[0] = 0x12;
    hash[1] = 0x34;
    const hs = new SchemaHandshake(hash, '2.0.0');
    const bytes = hs.encode();
    expect(bytes[0]).toBe(0x12);
    expect(bytes[1]).toBe(0x34);
    expect(bytes[32]).toBe(5); // LEB128 length of "2.0.0"
    expect(bytes[33]).toBe(0x32); // '2'
  });
});
