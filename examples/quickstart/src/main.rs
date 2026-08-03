mod generated;

use generated::{SensorKind, SensorReading, TelemetryPacket};
use vexil_runtime::{BitReader, BitWriter, Pack, Unpack};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let packet = TelemetryPacket {
        device_id: 42,
        readings: vec![
            SensorReading {
                channel: 0,
                kind: SensorKind::Temperature,
                value: 2350,
                sequence: 1,
                delta_ts: -50,
                _unknown: Vec::new(),
            },
            SensorReading {
                channel: 1,
                kind: SensorKind::Humidity,
                value: 6500,
                sequence: 2,
                delta_ts: 0,
                _unknown: Vec::new(),
            },
        ],
        battery: 95,
        _unknown: Vec::new(),
    };

    let mut writer = BitWriter::new();
    packet.pack(&mut writer)?;
    let bytes = writer.finish();
    let hash = generated::SCHEMA_HASH
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let wire = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ");
    println!("schema: {hash}");
    println!("wire:   {wire}");

    let mut reader = BitReader::new(&bytes);
    let decoded = TelemetryPacket::unpack(&mut reader)?;

    assert_eq!(decoded.device_id, 42);
    assert_eq!(decoded.readings.len(), 2);
    assert_eq!(decoded.readings[0].channel, 0);
    assert_eq!(decoded.readings[0].value, 2350);
    assert_eq!(decoded.battery, 95);
    println!(
        "round trip: device {}, {} readings, battery {}%",
        decoded.device_id,
        decoded.readings.len(),
        decoded.battery
    );
    Ok(())
}
