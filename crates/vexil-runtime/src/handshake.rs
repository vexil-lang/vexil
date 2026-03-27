//! Schema handshake helpers for connection-time identity checking.

use crate::{BitReader, BitWriter, DecodeError};

/// Schema identity for connection-time negotiation.
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaHandshake {
    pub hash: [u8; 32],
    pub version: String,
}

/// Result of comparing two handshakes.
#[derive(Debug, Clone, PartialEq)]
pub enum HandshakeResult {
    Match,
    VersionMismatch {
        local_version: String,
        remote_version: String,
        local_hash: [u8; 32],
        remote_hash: [u8; 32],
    },
}

impl SchemaHandshake {
    pub fn new(hash: [u8; 32], version: &str) -> Self {
        Self {
            hash,
            version: version.to_string(),
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut w = BitWriter::new();
        w.write_raw_bytes(&self.hash);
        w.write_string(&self.version);
        w.finish()
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = BitReader::new(bytes);
        let hash_bytes = r.read_raw_bytes(32)?;
        let hash: [u8; 32] = hash_bytes
            .try_into()
            .map_err(|_| DecodeError::UnexpectedEof)?;
        let version = r.read_string()?;
        Ok(Self { hash, version })
    }

    pub fn check(&self, remote: &SchemaHandshake) -> HandshakeResult {
        if self.hash == remote.hash {
            HandshakeResult::Match
        } else {
            HandshakeResult::VersionMismatch {
                local_version: self.version.clone(),
                remote_version: remote.version.clone(),
                local_hash: self.hash,
                remote_hash: remote.hash,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip() {
        let hash = [0xABu8; 32];
        let hs = SchemaHandshake::new(hash, "1.2.3");
        let bytes = hs.encode();
        let decoded = SchemaHandshake::decode(&bytes).unwrap();
        assert_eq!(decoded.hash, hash);
        assert_eq!(decoded.version, "1.2.3");
    }

    #[test]
    fn check_matching_hashes() {
        let hash = [0x42u8; 32];
        let local = SchemaHandshake::new(hash, "1.0.0");
        let remote = SchemaHandshake::new(hash, "1.0.0");
        assert_eq!(local.check(&remote), HandshakeResult::Match);
    }

    #[test]
    fn check_different_hashes() {
        let local = SchemaHandshake::new([0x01; 32], "1.0.0");
        let remote = SchemaHandshake::new([0x02; 32], "1.1.0");
        match local.check(&remote) {
            HandshakeResult::VersionMismatch {
                local_version,
                remote_version,
                ..
            } => {
                assert_eq!(local_version, "1.0.0");
                assert_eq!(remote_version, "1.1.0");
            }
            _ => panic!("expected VersionMismatch"),
        }
    }

    #[test]
    fn wire_size_is_compact() {
        let hs = SchemaHandshake::new([0; 32], "1.0.0");
        let bytes = hs.encode();
        // 32 (hash) + 1 (LEB128 len) + 5 (version) = 38
        assert_eq!(bytes.len(), 38);
    }
}
