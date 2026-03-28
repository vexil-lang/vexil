package main

import (
	"encoding/binary"
	"fmt"
	"os"
	"path/filepath"

	vexil "github.com/vexil-lang/vexil/packages/runtime-go"
)

//go:generate vexilc codegen ../schema/telemetry.vexil --target go --output generated.go

func main() {
	dataDir := filepath.Join("..", "data")

	// --- Read sensor readings from Rust ---
	readingsPath := filepath.Join(dataDir, "readings.bin")
	if data, err := os.ReadFile(readingsPath); err == nil {
		count := binary.LittleEndian.Uint32(data[0:4])
		offset := 4
		fmt.Printf("[Go] Reading %d sensor readings from Rust:\n\n", count)

		for i := uint32(0); i < count; i++ {
			length := binary.LittleEndian.Uint32(data[offset : offset+4])
			offset += 4
			msgBytes := data[offset : offset+int(length)]
			offset += int(length)

			var reading SensorReading
			r := vexil.NewBitReader(msgBytes)
			if err := reading.Unpack(r); err != nil {
				fmt.Printf("  decode error: %v\n", err)
				continue
			}

			fmt.Printf("  Reading #%d:\n", i+1)
			fmt.Printf("    Device ID: %d\n", reading.DeviceID)
			fmt.Printf("    Battery: %d%%\n", reading.Battery)
			fmt.Printf("    Signal: %d/15\n", reading.Signal)
			fmt.Printf("    Status: %d\n", reading.Status)
			fmt.Printf("    Temperature: %.1f\u00b0C\n", reading.Temperature)
			fmt.Printf("    Humidity: %.1f%%\n", reading.Humidity)
			fmt.Printf("    Label: %q\n", reading.Label)
			if reading.GpsLat != nil {
				fmt.Printf("    GPS: %.4f, %.4f\n", *reading.GpsLat, *reading.GpsLon)
			} else {
				fmt.Printf("    GPS: not available\n")
			}
			fmt.Println()
		}
	} else {
		fmt.Println("[Go] No readings.bin found -- run the Rust device first.")
	}

	// --- Write config from Go ---
	config := DeviceConfig{
		DeviceID:       1,
		ReportInterval: 15,
		HighTempAlert:  40.0,
		LowTempAlert:   -10.0,
		Label:          "Configured from Go Gateway",
	}

	w := vexil.NewBitWriter()
	if err := config.Pack(w); err != nil {
		fmt.Printf("[Go] encode error: %v\n", err)
		return
	}
	configBytes := w.Finish()

	configPath := filepath.Join(dataDir, "config_from_go.bin")
	os.MkdirAll(dataDir, 0755)
	if err := os.WriteFile(configPath, configBytes, 0644); err != nil {
		fmt.Printf("[Go] write error: %v\n", err)
		return
	}
	fmt.Printf("[Go] Wrote config to %s\n", configPath)
	fmt.Printf("  device_id: %d\n", config.DeviceID)
	fmt.Printf("  report_interval: %ds\n", config.ReportInterval)
	fmt.Printf("  temp alerts: %.1f\u00b0C - %.1f\u00b0C\n", config.LowTempAlert, config.HighTempAlert)
	fmt.Printf("  label: %q\n", config.Label)

	// --- Try to read config from Node ---
	nodeConfigPath := filepath.Join(dataDir, "config.bin")
	if data, err := os.ReadFile(nodeConfigPath); err == nil {
		var nodeConfig DeviceConfig
		r := vexil.NewBitReader(data)
		if err := nodeConfig.Unpack(r); err != nil {
			fmt.Printf("\n[Go] Failed to decode Node config: %v\n", err)
		} else {
			fmt.Printf("\n[Go] Received config from Node:\n")
			fmt.Printf("  device_id: %d\n", nodeConfig.DeviceID)
			fmt.Printf("  report_interval: %ds\n", nodeConfig.ReportInterval)
			fmt.Printf("  temp alerts: %.1f\u00b0C - %.1f\u00b0C\n", nodeConfig.LowTempAlert, nodeConfig.HighTempAlert)
			fmt.Printf("  label: %q\n", nodeConfig.Label)
		}
	}
}
