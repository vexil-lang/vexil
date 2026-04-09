use crate::error::VxbError;

/// Current `.vxb` binary file format version number.
pub const FORMAT_VERSION: u8 = 1;

/// Magic bytes identifying the binary file type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Magic {
    /// `.vxb` — binary data file (`b"VXB\0"`).
    Vxb,
    /// `.vxc` — binary compiled schema file (`b"VXC\0"`).
    Vxc,
    /// `.vxbp` — binary data pack file (`b"VXBP"`).
    Vxbp,
    /// `.vxcp` — binary compiled schema pack file (`b"VXCP"`).
    Vxcp,
}

impl Magic {
    /// Returns the 4-byte magic signature for this variant.
    pub fn bytes(&self) -> [u8; 4] {
        match self {
            Magic::Vxb => *b"VXB\0",
            Magic::Vxc => *b"VXC\0",
            Magic::Vxbp => *b"VXBP",
            Magic::Vxcp => *b"VXCP",
        }
    }

    /// Attempts to parse a 4-byte magic signature into a `Magic` variant.
    /// Returns `None` if the bytes do not match any known signature.
    pub fn from_bytes(bytes: &[u8; 4]) -> Option<Magic> {
        match bytes {
            b"VXB\0" => Some(Magic::Vxb),
            b"VXC\0" => Some(Magic::Vxc),
            b"VXBP" => Some(Magic::Vxbp),
            b"VXCP" => Some(Magic::Vxcp),
            _ => None,
        }
    }
}

/// Header for `.vxb` binary files.
///
/// Parsed from the first bytes of a binary file, containing the magic
/// signature, format version, compression flag, schema hash, namespace,
/// and schema version.
#[derive(Debug, Clone, PartialEq)]
pub struct VxbHeader {
    /// The magic bytes identifying the file type.
    pub magic: Magic,
    /// Binary format version (must equal [`FORMAT_VERSION`]).
    pub format_version: u8,
    /// Whether the payload is compressed.
    pub compressed: bool,
    /// BLAKE3 hash of the compiled schema used to encode this file.
    pub schema_hash: [u8; 32],
    /// Schema namespace (dot-separated).
    pub namespace: String,
    /// Schema version string.
    pub schema_version: String,
}

/// Write a VxbHeader into a buffer.
pub fn write_header(header: &VxbHeader, buf: &mut Vec<u8>) {
    // Magic (4 bytes)
    buf.extend_from_slice(&header.magic.bytes());
    // Format version (1 byte)
    buf.push(header.format_version);
    // Flags (1 byte): bit 0 = compressed
    buf.push(if header.compressed { 1 } else { 0 });
    // Schema hash (32 bytes)
    buf.extend_from_slice(&header.schema_hash);
    // Namespace (LEB128 length + UTF-8)
    write_leb128_string(&header.namespace, buf);
    // Schema version (LEB128 length + UTF-8)
    write_leb128_string(&header.schema_version, buf);
}

/// Read a VxbHeader from a byte slice.
/// Returns (header, bytes_consumed).
pub fn read_header(data: &[u8]) -> Result<(VxbHeader, usize), VxbError> {
    if data.len() < 38 {
        return Err(VxbError::BadMagic);
    }

    // Magic
    let magic_bytes: [u8; 4] = [data[0], data[1], data[2], data[3]];
    let magic = Magic::from_bytes(&magic_bytes).ok_or(VxbError::BadMagic)?;

    // Format version
    let format_version = data[4];
    if format_version != FORMAT_VERSION {
        return Err(VxbError::UnsupportedVersion {
            version: format_version,
            max_supported: FORMAT_VERSION,
        });
    }

    // Flags
    let flags = data[5];
    let compressed = flags & 1 != 0;

    // Schema hash
    let mut schema_hash = [0u8; 32];
    schema_hash.copy_from_slice(&data[6..38]);

    let mut pos = 38;

    // Namespace
    let (namespace, ns_len) = read_leb128_string(&data[pos..])?;
    pos += ns_len;

    // Schema version
    let (schema_version, sv_len) = read_leb128_string(&data[pos..])?;
    pos += sv_len;

    Ok((
        VxbHeader {
            magic,
            format_version,
            compressed,
            schema_hash,
            namespace,
            schema_version,
        },
        pos,
    ))
}

fn write_leb128_string(s: &str, buf: &mut Vec<u8>) {
    let bytes = s.as_bytes();
    let mut len = bytes.len() as u64;
    loop {
        let mut byte = (len & 0x7f) as u8;
        len >>= 7;
        if len != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if len == 0 {
            break;
        }
    }
    buf.extend_from_slice(bytes);
}

fn read_leb128_string(data: &[u8]) -> Result<(String, usize), VxbError> {
    let mut len: u64 = 0;
    let mut shift = 0u32;
    let mut pos = 0;
    loop {
        if pos >= data.len() {
            return Err(VxbError::Io {
                message: "unexpected end of data reading LEB128 length".to_string(),
            });
        }
        let byte = data[pos];
        pos += 1;
        len |= ((byte & 0x7f) as u64) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            break;
        }
        if shift >= 35 {
            return Err(VxbError::Io {
                message: "LEB128 length overflow".to_string(),
            });
        }
    }
    // Reject implausibly large strings in header fields (namespace, version).
    // The largest realistic namespace is well under 1 KiB; 65535 is generous.
    const MAX_HEADER_STRING: u64 = 65535;
    if len > MAX_HEADER_STRING {
        return Err(VxbError::Io {
            message: format!("header string length {len} exceeds maximum {MAX_HEADER_STRING}"),
        });
    }
    let str_len = len as usize;
    if pos + str_len > data.len() {
        return Err(VxbError::Io {
            message: "unexpected end of data reading string".to_string(),
        });
    }
    let s = std::str::from_utf8(&data[pos..pos + str_len])
        .map_err(|e| VxbError::Io {
            message: e.to_string(),
        })?
        .to_string();
    Ok((s, pos + str_len))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_header() -> VxbHeader {
        VxbHeader {
            magic: Magic::Vxb,
            format_version: FORMAT_VERSION,
            compressed: false,
            schema_hash: [0u8; 32],
            namespace: "test.simple".to_string(),
            schema_version: "0.1.0".to_string(),
        }
    }

    #[test]
    fn header_roundtrip() {
        let h = sample_header();
        let mut buf = Vec::new();
        write_header(&h, &mut buf);
        let (decoded, consumed) = read_header(&buf).unwrap();
        assert_eq!(h, decoded);
        assert_eq!(consumed, buf.len());
    }

    #[test]
    fn header_compressed() {
        let h = VxbHeader {
            compressed: true,
            ..sample_header()
        };
        let mut buf = Vec::new();
        write_header(&h, &mut buf);
        let (decoded, _) = read_header(&buf).unwrap();
        assert!(decoded.compressed);
    }

    #[test]
    fn all_magic_variants() {
        for magic in [Magic::Vxb, Magic::Vxc, Magic::Vxbp, Magic::Vxcp] {
            let h = VxbHeader {
                magic,
                ..sample_header()
            };
            let mut buf = Vec::new();
            write_header(&h, &mut buf);
            let (decoded, _) = read_header(&buf).unwrap();
            assert_eq!(decoded.magic, magic);
        }
    }

    #[test]
    fn bad_magic_rejected() {
        let data = b"NOPE\x01\x00" as &[u8];
        let mut full = data.to_vec();
        full.extend_from_slice(&[0u8; 40]); // pad to > 38 bytes
        let result = read_header(&full);
        assert!(matches!(result, Err(VxbError::BadMagic)));
    }
}
