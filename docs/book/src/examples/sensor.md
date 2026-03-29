# Sensor Telemetry

The `examples/sensor-packet/` directory demonstrates a basic Vexil schema for sensor telemetry data.

## Schema

```vexil
namespace sensor.packet

enum SensorKind : u8 {
    Temperature @0
    Humidity    @1
    Pressure    @2
}

message SensorReading {
    channel  @0 : u4
    kind     @1 : SensorKind
    value    @2 : u16
    sequence @3 : u32 @varint
}
```

This schema packs a sensor reading into a compact binary format:

- `channel` uses only 4 bits (supports 16 channels)
- `kind` uses 8 bits for the enum discriminant
- `value` is a fixed 16-bit reading
- `sequence` uses variable-length encoding (small sequence numbers take fewer bytes)

## Running

```sh
# Generate Rust code
vexilc codegen examples/sensor-packet/sensor.vexil --target rust

# Generate TypeScript code
vexilc codegen examples/sensor-packet/sensor.vexil --target typescript
```

## Source

[`examples/sensor-packet/`](https://github.com/vexil-lang/vexil/tree/main/examples/sensor-packet)
