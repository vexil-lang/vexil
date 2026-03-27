#[path = "generated.rs"]
mod generated;

use generated::*;
use vexil_runtime::{BitReader, BitWriter, Pack, Unpack};
use std::fs;
use std::path::Path;

fn main() {
    let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../data");
    fs::create_dir_all(&data_dir).unwrap();

    // --- Encode sensor readings ---
    let readings = vec![
        SensorReading {
            device_id: 1,
            battery: 95,
            signal: 12,
            status: DeviceStatus::Online,
            temperature: 22.5,
            humidity: 65.0,
            label: "Living Room".to_string(),
            gps_lat: Some(37.7749),
            gps_lon: Some(-122.4194),
        },
        SensorReading {
            device_id: 2,
            battery: 30,
            signal: 5,
            status: DeviceStatus::Degraded,
            temperature: -3.2,
            humidity: 89.1,
            label: "Rooftop".to_string(),
            gps_lat: None,
            gps_lon: None,
        },
        SensorReading {
            device_id: 3,
            battery: 100,
            signal: 15,
            status: DeviceStatus::Online,
            temperature: 36.6,
            humidity: 45.5,
            label: "Server Room".to_string(),
            gps_lat: Some(40.7128),
            gps_lon: Some(-74.0060),
        },
    ];

    // Write readings to binary file with simple framing:
    // u32 LE count, then for each: u32 LE byte_length + bytes
    let readings_path = data_dir.join("readings.bin");
    let mut file_buf: Vec<u8> = Vec::new();
    file_buf.extend_from_slice(&(readings.len() as u32).to_le_bytes());

    for (i, reading) in readings.iter().enumerate() {
        let mut w = BitWriter::new();
        reading.pack(&mut w).unwrap();
        let bytes = w.finish();
        file_buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        file_buf.extend_from_slice(&bytes);
        println!(
            "[Rust] Encoded reading #{}: device={}, temp={:.1}\u{00B0}C, battery={}%, label=\"{}\"",
            i + 1,
            reading.device_id,
            reading.temperature,
            reading.battery,
            reading.label
        );
    }

    fs::write(&readings_path, &file_buf).unwrap();
    println!(
        "[Rust] Wrote {} readings to {}",
        readings.len(),
        readings_path.display()
    );

    // --- Try to read config from Node ---
    let config_path = data_dir.join("config.bin");
    if config_path.exists() {
        let config_bytes = fs::read(&config_path).unwrap();
        let mut r = BitReader::new(&config_bytes);
        match DeviceConfig::unpack(&mut r) {
            Ok(config) => {
                println!("\n[Rust] Received config from Node:");
                println!("  device_id: {}", config.device_id);
                println!("  report_interval: {}s", config.report_interval);
                println!(
                    "  temp alerts: {:.1}\u{00B0}C - {:.1}\u{00B0}C",
                    config.low_temp_alert, config.high_temp_alert
                );
                println!("  label: \"{}\"", config.label);
            }
            Err(e) => eprintln!("[Rust] Failed to decode config: {e}"),
        }
    } else {
        println!("\n[Rust] No config.bin found -- run the Node dashboard to generate one.");
    }
}
