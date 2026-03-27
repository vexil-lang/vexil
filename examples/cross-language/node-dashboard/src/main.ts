import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { BitReader, BitWriter } from '@vexil/runtime';
import {
  decodeSensorReading,
  encodeDeviceConfig,
  type SensorReading,
  type DeviceConfig,
} from './generated.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const dataDir = join(__dirname, '../../data');

// Ensure data directory exists
mkdirSync(dataDir, { recursive: true });

// --- Decode sensor readings from Rust ---
const readingsPath = join(dataDir, 'readings.bin');
if (existsSync(readingsPath)) {
  const buf = readFileSync(readingsPath);
  const data = new Uint8Array(buf);
  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);

  let offset = 0;
  const count = view.getUint32(offset, true);
  offset += 4;

  console.log(`[Node] Reading ${count} sensor readings from Rust:\n`);

  for (let i = 0; i < count; i++) {
    const len = view.getUint32(offset, true);
    offset += 4;
    const msgBytes = data.slice(offset, offset + len);
    offset += len;

    const r = new BitReader(msgBytes);
    const reading = decodeSensorReading(r);

    console.log(`  Reading #${i + 1}:`);
    console.log(`    Device ID: ${reading.device_id}`);
    console.log(`    Battery: ${reading.battery}%`);
    console.log(`    Signal: ${reading.signal}/15`);
    console.log(`    Status: ${reading.status}`);
    console.log(`    Temperature: ${reading.temperature.toFixed(1)}\u00B0C`);
    console.log(`    Humidity: ${reading.humidity.toFixed(1)}%`);
    console.log(`    Label: "${reading.label}"`);
    if (reading.gps_lat !== null && reading.gps_lon !== null) {
      console.log(
        `    GPS: ${reading.gps_lat.toFixed(4)}, ${reading.gps_lon.toFixed(4)}`,
      );
    } else {
      console.log(`    GPS: not available`);
    }
    console.log();
  }
} else {
  console.log('[Node] No readings.bin found -- run the Rust device first.');
}

// --- Encode a config command back to Rust ---
const config: DeviceConfig = {
  device_id: 1,
  report_interval: 30,
  high_temp_alert: 35.0,
  low_temp_alert: 5.0,
  label: 'Updated from Node Dashboard',
};

const w = new BitWriter();
encodeDeviceConfig(config, w);
const configBytes = w.finish();

const configPath = join(dataDir, 'config.bin');
writeFileSync(configPath, configBytes);
console.log(`[Node] Wrote config to ${configPath}`);
console.log(`  device_id: ${config.device_id}`);
console.log(`  report_interval: ${config.report_interval}s`);
console.log(
  `  temp alerts: ${config.low_temp_alert}\u00B0C - ${config.high_temp_alert}\u00B0C`,
);
console.log(`  label: "${config.label}"`);
