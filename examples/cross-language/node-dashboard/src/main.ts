import { BitReader, BitWriter } from '@vexil-lang/runtime';
import {
  decodeSensorReading,
  encodeSensorReading,
  SCHEMA_HASH,
  type SensorReading,
} from './generated.js';

const reading: SensorReading = {
  device_id: 7,
  battery: 95,
  signal: 12,
  status: 'Online',
  temperature: 22.5,
  humidity: 65.0,
  label: 'greenhouse',
  gps_lat: 41.0082,
  gps_lon: 28.9784,
  _unknown: new Uint8Array(),
};

const w = new BitWriter();
encodeSensorReading(reading, w);
const bytes = w.finish();
const decoded = decodeSensorReading(new BitReader(bytes));
if (JSON.stringify(decoded) !== JSON.stringify(reading)) {
  throw new Error('round trip mismatch');
}

const hex = (value: Uint8Array): string =>
  Array.from(value, (byte) => byte.toString(16).padStart(2, '0')).join('');

console.log(`schema=${hex(SCHEMA_HASH)}`);
console.log(`wire=${hex(bytes)}`);
console.log('round-trip=ok');
