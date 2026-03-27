//! Hand-written message types mirroring VNP-representative workloads.
//!
//! These types exercise the `vexil_runtime` encode/decode primitives at
//! various message sizes: sub-byte packed (Envelope), mixed-width
//! (DrawText), and variable-length (OutputChunk).

use vexil_runtime::{BitReader, BitWriter, DecodeError, EncodeError};

// ---------------------------------------------------------------------------
// Envelope — small, sub-byte packed
// ---------------------------------------------------------------------------

/// A compact protocol envelope with sub-byte packed header fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Envelope {
    /// Protocol version (4 bits, 0..15).
    pub version: u8,
    /// Domain identifier (4 bits, 0..15).
    pub domain: u8,
    /// Message type tag (7 bits, 0..127).
    pub msg_type: u8,
    /// Session identifier.
    pub session_id: u32,
    /// Millisecond timestamp (48 bits).
    pub timestamp: u64,
    /// Optional message correlation id.
    pub msg_id: Option<u32>,
}

impl Envelope {
    pub fn encode(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        w.write_bits(self.version as u64, 4);
        w.write_bits(self.domain as u64, 4);
        w.write_bits(self.msg_type as u64, 7);
        w.write_u32(self.session_id);
        w.write_bits(self.timestamp, 48);
        match &self.msg_id {
            Some(id) => {
                w.write_bool(true);
                w.write_u32(*id);
            }
            None => {
                w.write_bool(false);
            }
        }
        Ok(())
    }

    pub fn decode(r: &mut BitReader) -> Result<Self, DecodeError> {
        let version = r.read_bits(4)? as u8;
        let domain = r.read_bits(4)? as u8;
        let msg_type = r.read_bits(7)? as u8;
        let session_id = r.read_u32()?;
        let timestamp = r.read_bits(48)?;
        let has_msg_id = r.read_bool()?;
        let msg_id = if has_msg_id {
            Some(r.read_u32()?)
        } else {
            None
        };
        Ok(Envelope {
            version,
            domain,
            msg_type,
            session_id,
            timestamp,
            msg_id,
        })
    }
}

// ---------------------------------------------------------------------------
// DrawText — medium, mixed widths
// ---------------------------------------------------------------------------

/// A terminal draw command with colour and style attributes.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawText {
    pub x: u16,
    pub y: u16,
    pub fg: [u8; 3],
    pub bg: [u8; 3],
    pub bold: bool,
    pub italic: bool,
    pub text: String,
}

impl DrawText {
    pub fn encode(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        w.write_u16(self.x);
        w.write_u16(self.y);
        for &b in &self.fg {
            w.write_u8(b);
        }
        for &b in &self.bg {
            w.write_u8(b);
        }
        w.write_bool(self.bold);
        w.write_bool(self.italic);
        w.write_string(&self.text);
        Ok(())
    }

    pub fn decode(r: &mut BitReader) -> Result<Self, DecodeError> {
        let x = r.read_u16()?;
        let y = r.read_u16()?;
        let fg = [r.read_u8()?, r.read_u8()?, r.read_u8()?];
        let bg = [r.read_u8()?, r.read_u8()?, r.read_u8()?];
        let bold = r.read_bool()?;
        let italic = r.read_bool()?;
        let text = r.read_string()?;
        Ok(DrawText {
            x,
            y,
            fg,
            bg,
            bold,
            italic,
            text,
        })
    }
}

// ---------------------------------------------------------------------------
// OutputChunk — large, variable-length
// ---------------------------------------------------------------------------

/// A variable-length output payload with optional metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputChunk {
    pub session_id: u32,
    pub pane_id: u16,
    pub sequence: u64,
    pub data: Vec<u8>,
    pub command_tag: Option<String>,
}

impl OutputChunk {
    pub fn encode(&self, w: &mut BitWriter) -> Result<(), EncodeError> {
        w.write_u32(self.session_id);
        w.write_u16(self.pane_id);
        w.write_u64(self.sequence);
        w.write_bytes(&self.data);
        match &self.command_tag {
            Some(tag) => {
                w.write_bool(true);
                w.write_string(tag);
            }
            None => {
                w.write_bool(false);
            }
        }
        Ok(())
    }

    pub fn decode(r: &mut BitReader) -> Result<Self, DecodeError> {
        let session_id = r.read_u32()?;
        let pane_id = r.read_u16()?;
        let sequence = r.read_u64()?;
        let data = r.read_bytes()?;
        let has_tag = r.read_bool()?;
        let command_tag = if has_tag {
            Some(r.read_string()?)
        } else {
            None
        };
        Ok(OutputChunk {
            session_id,
            pane_id,
            sequence,
            data,
            command_tag,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_roundtrip() {
        let original = Envelope {
            version: 1,
            domain: 3,
            msg_type: 42,
            session_id: 1,
            timestamp: 1_234_567_890_123,
            msg_id: Some(99),
        };
        let mut w = BitWriter::new();
        original.encode(&mut w).unwrap();
        let bytes = w.finish();
        let mut r = BitReader::new(&bytes);
        let decoded = Envelope::decode(&mut r).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn envelope_no_msg_id_roundtrip() {
        let original = Envelope {
            version: 15,
            domain: 0,
            msg_type: 127,
            session_id: u32::MAX,
            timestamp: (1u64 << 48) - 1,
            msg_id: None,
        };
        let mut w = BitWriter::new();
        original.encode(&mut w).unwrap();
        let bytes = w.finish();
        let mut r = BitReader::new(&bytes);
        let decoded = Envelope::decode(&mut r).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn draw_text_roundtrip() {
        let original = DrawText {
            x: 80,
            y: 24,
            fg: [255, 128, 0],
            bg: [0, 0, 0],
            bold: true,
            italic: false,
            text: "Hello, Vexil!".into(),
        };
        let mut w = BitWriter::new();
        original.encode(&mut w).unwrap();
        let bytes = w.finish();
        let mut r = BitReader::new(&bytes);
        let decoded = DrawText::decode(&mut r).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn output_chunk_roundtrip() {
        let original = OutputChunk {
            session_id: 42,
            pane_id: 7,
            sequence: 1000,
            data: vec![0xDE, 0xAD, 0xBE, 0xEF],
            command_tag: Some("ls -la".into()),
        };
        let mut w = BitWriter::new();
        original.encode(&mut w).unwrap();
        let bytes = w.finish();
        let mut r = BitReader::new(&bytes);
        let decoded = OutputChunk::decode(&mut r).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn output_chunk_no_tag_roundtrip() {
        let original = OutputChunk {
            session_id: 1,
            pane_id: 0,
            sequence: 0,
            data: vec![],
            command_tag: None,
        };
        let mut w = BitWriter::new();
        original.encode(&mut w).unwrap();
        let bytes = w.finish();
        let mut r = BitReader::new(&bytes);
        let decoded = OutputChunk::decode(&mut r).unwrap();
        assert_eq!(original, decoded);
    }
}
