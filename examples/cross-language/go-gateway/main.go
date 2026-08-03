package main

import (
	"encoding/hex"
	"fmt"
	"os"

	telemetry "cross-language-go-gateway/telemetry"
	vexil "github.com/vexil-lang/vexil/packages/runtime-go"
)

//go:generate vexilc codegen ../schema/telemetry.vexil --target go --output telemetry/generated.go

func main() {
	if err := run(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func run() error {
	lat := 41.0082
	lon := 28.9784
	reading := telemetry.SensorReading{
		DeviceID: 7, Battery: 95, Signal: 12, Status: telemetry.DeviceStatusOnline,
		Temperature: 22.5, Humidity: 65.0, Label: "greenhouse",
		GpsLat: &lat, GpsLon: &lon,
	}
	w := vexil.NewBitWriter()
	if err := reading.Pack(w); err != nil {
		return fmt.Errorf("encode fixture: %w", err)
	}
	bytes := w.Finish()
	var decoded telemetry.SensorReading
	if err := decoded.Unpack(vexil.NewBitReader(bytes)); err != nil {
		return fmt.Errorf("decode fixture: %w", err)
	}
	if decoded.DeviceID != reading.DeviceID || decoded.Label != reading.Label {
		return fmt.Errorf("round trip mismatch")
	}
	fmt.Printf("schema=%s\n", hex.EncodeToString(telemetry.SchemaHash[:]))
	fmt.Printf("wire=%s\n", hex.EncodeToString(bytes))
	fmt.Println("round-trip=ok")
	return nil
}
