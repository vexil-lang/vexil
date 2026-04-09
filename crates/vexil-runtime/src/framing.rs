use crate::error::DecodeError;
use crate::leb128;

/// Writes length-prefixed frames to a byte stream.
///
/// Each frame is: [LEB128 length][payload bytes]
/// This enables streaming — you can send frames one at a time
/// over a TCP connection, WebSocket, etc.
///
/// # Example
///
/// ```
/// use vexil_runtime::{BitWriter, FrameWriter};
///
/// let mut fw = FrameWriter::new();
///
/// // Encode first message
/// let mut w = BitWriter::new();
/// w.write_u32(42);
/// let bytes = w.finish();
/// fw.write_frame(&bytes);
///
/// // Encode second message
/// let mut w = BitWriter::new();
/// w.write_string("hello");
/// let bytes = w.finish();
/// fw.write_frame(&bytes);
///
/// let stream = fw.finish();
/// // stream = [1, 42, 0, 0, 0, 6, h, e, l, l, o]
/// //           ^len=1  ^payload    ^len=6  ^payload
/// ```
pub struct FrameWriter {
    buf: Vec<u8>,
}

impl FrameWriter {
    /// Create a new empty frame writer.
    pub fn new() -> Self {
        Self {
            buf: Vec::with_capacity(256),
        }
    }

    /// Write a single frame. The `payload` is typically the output
    /// of `BitWriter::finish()`.
    pub fn write_frame(&mut self, payload: &[u8]) {
        // Write length as LEB128
        leb128::encode(&mut self.buf, payload.len() as u64);
        // Write payload
        self.buf.extend_from_slice(payload);
    }

    /// Get a reference to the accumulated bytes without consuming.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    /// Finish and return the complete byte stream.
    pub fn finish(self) -> Vec<u8> {
        self.buf
    }

    /// Reset for reuse, keeping the allocation.
    pub fn reset(&mut self) {
        self.buf.clear();
    }
}

impl Default for FrameWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Reads length-prefixed frames from a byte stream.
///
/// Yields individual message payloads that can be decoded
/// with `BitReader`.
///
/// # Example
///
/// ```
/// use vexil_runtime::{BitReader, FrameReader};
///
/// let stream: &[u8] = &[
///     4, 0x2A, 0x00, 0x00, 0x00,  // frame 1: len=4, u32(42) LE
///     6, 0x05, 0x68, 0x65, 0x6C, 0x6C, 0x6F, // frame 2: len=6, string "hello"
/// ];
/// let mut fr = FrameReader::new(stream);
///
/// let frame1 = fr.read_frame().unwrap().unwrap();
/// let mut r = BitReader::new(frame1);
/// assert_eq!(r.read_u32().unwrap(), 42);
///
/// let frame2 = fr.read_frame().unwrap().unwrap();
/// let mut r = BitReader::new(frame2);
/// assert_eq!(r.read_string().unwrap(), "hello");
///
/// assert!(fr.read_frame().is_none()); // EOF
/// ```
pub struct FrameReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> FrameReader<'a> {
    /// Create a new frame reader over the given byte stream.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Read the next frame. Returns the payload bytes, or `None` at EOF.
    pub fn read_frame(&mut self) -> Option<Result<&'a [u8], DecodeError>> {
        if self.pos >= self.data.len() {
            return None;
        }

        let (len, consumed) = match leb128::decode(&self.data[self.pos..], 10) {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };
        self.pos += consumed;

        let len = len as usize;
        if self.pos + len > self.data.len() {
            return Some(Err(DecodeError::UnexpectedEof));
        }

        let payload = &self.data[self.pos..self.pos + len];
        self.pos += len;
        Some(Ok(payload))
    }

    /// Returns true if there are no more frames.
    pub fn is_empty(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Returns remaining bytes (partial frame or trailing data).
    pub fn remaining(&self) -> &'a [u8] {
        &self.data[self.pos..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BitReader, BitWriter};

    #[test]
    fn roundtrip_single_frame() {
        let mut w = BitWriter::new();
        w.write_u32(42);
        let payload = w.finish();

        let mut fw = FrameWriter::new();
        fw.write_frame(&payload);
        let stream = fw.finish();

        let mut fr = FrameReader::new(&stream);
        let frame = fr.read_frame().unwrap().unwrap();
        let mut r = BitReader::new(frame);
        assert_eq!(r.read_u32().unwrap(), 42);
        assert!(fr.read_frame().is_none());
    }

    #[test]
    fn roundtrip_multiple_frames() {
        let mut fw = FrameWriter::new();

        let mut w = BitWriter::new();
        w.write_u32(1);
        fw.write_frame(&w.finish());

        let mut w = BitWriter::new();
        w.write_string("hello");
        fw.write_frame(&w.finish());

        let mut w = BitWriter::new();
        w.write_u64(999);
        fw.write_frame(&w.finish());

        let stream = fw.finish();
        let mut fr = FrameReader::new(&stream);

        let f1 = fr.read_frame().unwrap().unwrap();
        assert_eq!(BitReader::new(f1).read_u32().unwrap(), 1);

        let f2 = fr.read_frame().unwrap().unwrap();
        assert_eq!(BitReader::new(f2).read_string().unwrap(), "hello");

        let f3 = fr.read_frame().unwrap().unwrap();
        assert_eq!(BitReader::new(f3).read_u64().unwrap(), 999);

        assert!(fr.read_frame().is_none());
    }

    #[test]
    fn writer_reset_reuse() {
        let mut fw = FrameWriter::new();

        let mut w = BitWriter::new();
        w.write_u32(1);
        fw.write_frame(&w.finish());

        let bytes1 = fw.finish();

        let mut fw = FrameWriter::new();
        let mut w = BitWriter::new();
        w.write_u32(1);
        fw.write_frame(&w.finish());
        let bytes2 = fw.finish();

        assert_eq!(bytes1, bytes2);
    }

    #[test]
    fn empty_stream() {
        let mut fr = FrameReader::new(&[]);
        assert!(fr.read_frame().is_none());
        assert!(fr.is_empty());
    }

    #[test]
    fn corrupted_length() {
        // Truncated LEB128
        let stream = [0x80]; // continuation byte with no follow-up
        let mut fr = FrameReader::new(&stream);
        match fr.read_frame() {
            Some(Err(_)) => {} // expected
            other => panic!("expected error, got {:?}", other),
        }
    }

    #[test]
    fn truncated_payload() {
        // Length says 10 bytes but only 3 available
        let stream = [10, 0x01, 0x02, 0x03];
        let mut fr = FrameReader::new(&stream);
        match fr.read_frame() {
            Some(Err(DecodeError::UnexpectedEof)) => {} // expected
            other => panic!("expected EOF error, got {:?}", other),
        }
    }
}
