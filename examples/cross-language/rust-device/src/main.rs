#[path = "generated.rs"]
mod generated;

use generated::*;
use vexil_runtime::{BitReader, BitWriter, Pack, Unpack};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let reading = SensorReading {
        device_id: 7,
        battery: 95,
        signal: 12,
        status: DeviceStatus::Online,
        temperature: 22.5,
        humidity: 65.0,
        label: "greenhouse".to_string(),
        gps_lat: Some(41.0082),
        gps_lon: Some(28.9784),
        _unknown: Vec::new(),
    };
    let mut writer = BitWriter::new();
    reading.pack(&mut writer)?;
    let bytes = writer.finish();
    let mut reader = BitReader::new(&bytes);
    assert_eq!(SensorReading::unpack(&mut reader)?, reading);

    println!("schema={}", hex(&SCHEMA_HASH));
    println!("wire={}", hex(&bytes));
    println!("round-trip=ok");
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
