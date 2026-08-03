from generated import DeviceStatus, SCHEMA_HASH, SensorReading


def main() -> None:
    reading = SensorReading(
        device_id=7,
        battery=95,
        signal=12,
        status=DeviceStatus(DeviceStatus.ONLINE),
        temperature=22.5,
        humidity=65.0,
        label="greenhouse",
        gps_lat=41.0082,
        gps_lon=28.9784,
    )
    wire = reading.encode()
    if SensorReading.decode(wire) != reading:
        raise RuntimeError("round trip mismatch")
    print(f"schema={bytes(SCHEMA_HASH).hex()}")
    print(f"wire={wire.hex()}")
    print("round-trip=ok")


if __name__ == "__main__":
    main()
