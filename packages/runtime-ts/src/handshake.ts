import { BitReader } from './bit-reader.js';
import { BitWriter } from './bit-writer.js';

export type HandshakeResult =
  | { kind: 'match' }
  | {
      kind: 'version_mismatch';
      localVersion: string;
      remoteVersion: string;
      localHash: Uint8Array;
      remoteHash: Uint8Array;
    };

export class SchemaHandshake {
  constructor(
    public readonly hash: Uint8Array,
    public readonly version: string,
  ) {}

  /** Encode this handshake (hash + version) into a wire-format byte array. */
  encode(): Uint8Array {
    const w = new BitWriter();
    w.writeRawBytes(this.hash);
    w.writeString(this.version);
    return w.finish();
  }

  /** Decode a wire-format byte array into a SchemaHandshake. */
  static decode(bytes: Uint8Array): SchemaHandshake {
    const r = new BitReader(bytes);
    const hash = r.readRawBytes(32);
    const version = r.readString();
    return new SchemaHandshake(hash, version);
  }

  /** Compare this handshake against a remote one, returning match or mismatch details. */
  check(remote: SchemaHandshake): HandshakeResult {
    const match_ =
      this.hash.length === remote.hash.length &&
      this.hash.every((b, i) => b === remote.hash[i]);
    if (match_) {
      return { kind: 'match' };
    }
    return {
      kind: 'version_mismatch',
      localVersion: this.version,
      remoteVersion: remote.version,
      localHash: this.hash,
      remoteHash: remote.hash,
    };
  }
}
