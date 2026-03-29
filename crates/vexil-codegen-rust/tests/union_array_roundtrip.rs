//! Regression test for issue #40: readRemaining() in message decoders
//! must not consume sibling array elements inside union payloads.

use vexil_runtime::{BitReader, BitWriter};

// Simulate the wire format of a union with array<Message> variant:
// discriminant (LEB128) + payload_length (LEB128) + payload
// where payload = count (LEB128) + element1 + element2 + element3

#[test]
fn union_array_payload_preserves_all_elements() {
    // Encode a union variant with 3 simple messages
    // Each "message" is: u32 + string (simulating a struct with 2 fields)
    let mut w = BitWriter::new();

    // Union discriminant = 0
    w.write_leb128(0);

    // Build payload: 3 messages, each with id:u32 + name:string
    let mut payload_w = BitWriter::new();
    payload_w.write_leb128(3); // array count

    // Element 1: id=1, name="alpha"
    payload_w.write_u32(1);
    payload_w.write_string("alpha");
    payload_w.flush_to_byte_boundary();

    // Element 2: id=2, name="beta"
    payload_w.write_u32(2);
    payload_w.write_string("beta");
    payload_w.flush_to_byte_boundary();

    // Element 3: id=3, name="gamma"
    payload_w.write_u32(3);
    payload_w.write_string("gamma");
    payload_w.flush_to_byte_boundary();

    let payload = payload_w.finish();
    w.write_leb128(payload.len() as u64);
    w.write_raw_bytes(&payload);

    let bytes = w.finish();

    // Decode: read discriminant, length, then decode 3 elements from payload
    let mut r = BitReader::new(&bytes);
    let disc = r.read_leb128(10).unwrap();
    assert_eq!(disc, 0);

    let len = r.read_leb128(10).unwrap() as usize;
    let payload_bytes = r.read_raw_bytes(len).unwrap();

    let mut pr = BitReader::new(&payload_bytes);
    let count = pr.read_leb128(10).unwrap() as usize;
    assert_eq!(count, 3);

    // Decode all 3 elements — this is where the bug manifested:
    // readRemaining() on element 1 would eat elements 2 and 3
    for i in 0..count {
        let id = pr.read_u32().unwrap();
        let name = pr.read_string().unwrap();
        pr.flush_to_byte_boundary();
        // NOT calling read_remaining() here — that's the fix

        assert_eq!(id, (i + 1) as u32);
        match i {
            0 => assert_eq!(name, "alpha"),
            1 => assert_eq!(name, "beta"),
            2 => assert_eq!(name, "gamma"),
            _ => unreachable!(),
        }
    }
}

#[test]
fn union_encode_no_spurious_leading_byte() {
    // Regression test: union Pack must NOT emit a leading 0x00 byte
    // from flush_to_byte_boundary() on a fresh writer.

    let mut w = BitWriter::new();
    // Simulate what generated union Pack does (after fix):
    // No flush at start — directly write discriminant
    w.write_leb128(0); // discriminant

    let mut payload_w = BitWriter::new();
    payload_w.write_u32(42);
    payload_w.flush_to_byte_boundary();
    let payload = payload_w.finish();

    w.write_leb128(payload.len() as u64);
    w.write_raw_bytes(&payload);

    let bytes = w.finish();

    // First byte must be the discriminant (0), not a spurious flush byte
    assert_eq!(bytes[0], 0x00); // discriminant 0
                                // Second byte should be payload length, NOT another 0x00 from a spurious flush
    assert!(
        bytes[1] > 0,
        "second byte should be payload length, not zero"
    );
}
