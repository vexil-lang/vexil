mod generated;

use generated::{SensorKind, SensorReading, TelemetryPacket};
use vexil_runtime::{BitReader, BitWriter, Pack, Unpack};

fn main() {
    // Build a telemetry packet
    let packet = TelemetryPacket {
        device_id: 42,
        readings: vec![
            SensorReading {
                channel: 0,
                kind: SensorKind::Temperature,
                value: 2350,      // 23.50°C
                sequence: 1,
                delta_ts: -50,    // 50ms before previous
            },
            SensorReading {
                channel: 1,
                kind: SensorKind::Humidity,
                value: 6500,      // 65.00%
                sequence: 2,
                delta_ts: 0,
            },
        ],
        battery: 95,  // 95% — fits in 7 bits (0..127)
    };

    // Encode
    let mut writer = BitWriter::new();
    packet.pack(&mut writer).expect("encode failed");
    let bytes = writer.finish();
    println!("Encoded {} bytes", bytes.len());

    // Decode
    let mut reader = BitReader::new(&bytes);
    let decoded = TelemetryPacket::unpack(&mut reader).expect("decode failed");

    // Verify roundtrip
    assert_eq!(decoded.device_id, 42);
    assert_eq!(decoded.readings.len(), 2);
    assert_eq!(decoded.readings[0].channel, 0);
    assert_eq!(decoded.readings[0].value, 2350);
    assert_eq!(decoded.battery, 95);
    println!("Roundtrip OK: {} readings, device {}", decoded.readings.len(), decoded.device_id);
}
