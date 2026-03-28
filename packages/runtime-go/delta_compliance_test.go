package vexil

import (
	"encoding/hex"
	"encoding/json"
	"os"
	"strings"
	"testing"
)

// deltaVector represents a compliance test vector for delta encoding.
type deltaVector struct {
	Name   string       `json:"name"`
	Schema string       `json:"schema"`
	Type   string       `json:"type"`
	Frames []deltaFrame `json:"frames"`
	Notes  string       `json:"notes"`
}

type deltaFrame struct {
	Value         map[string]interface{} `json:"value"`
	ExpectedBytes string                 `json:"expected_bytes"`
	Reset         bool                   `json:"reset"`
}

// deltaFieldInfo holds metadata about a field in a delta schema.
type deltaFieldInfo struct {
	Name    string
	Type    string
	IsDelta bool
}

// parseDeltaFields extracts field info including @delta annotations from a schema.
func parseDeltaFields(schema string) []deltaFieldInfo {
	var fields []deltaFieldInfo
	lines := strings.Split(schema, "\n")
	nextIsDelta := false
	for _, line := range lines {
		line = strings.TrimSpace(line)
		if line == "@delta" {
			nextIsDelta = true
			continue
		}
		// Look for field definitions with @N : type pattern
		atIdx := strings.Index(line, " @")
		if atIdx == -1 {
			nextIsDelta = false
			continue
		}
		name := strings.TrimSpace(line[:atIdx])
		if strings.Contains(name, "{") || strings.Contains(name, "}") {
			nextIsDelta = false
			continue
		}
		colonIdx := strings.Index(line[atIdx:], ":")
		if colonIdx == -1 {
			nextIsDelta = false
			continue
		}
		typStr := strings.TrimSpace(line[atIdx+colonIdx+1:])
		typStr = strings.TrimRight(typStr, " }")
		fields = append(fields, deltaFieldInfo{
			Name:    name,
			Type:    typStr,
			IsDelta: nextIsDelta,
		})
		nextIsDelta = false
	}
	return fields
}

func TestComplianceDelta(t *testing.T) {
	data, err := os.ReadFile("../../compliance/vectors/delta.json")
	if err != nil {
		t.Fatalf("failed to read delta.json: %v", err)
	}
	var vectors []deltaVector
	if err := json.Unmarshal(data, &vectors); err != nil {
		t.Fatalf("failed to parse delta.json: %v", err)
	}

	for _, v := range vectors {
		t.Run(v.Name, func(t *testing.T) {
			fields := parseDeltaFields(v.Schema)

			// Track previous values for delta fields (start at 0)
			prevValues := make(map[string]int64)
			for _, f := range fields {
				if f.IsDelta {
					prevValues[f.Name] = 0
				}
			}

			for fi, frame := range v.Frames {
				if frame.Reset {
					// Reset all delta state to zero
					for k := range prevValues {
						prevValues[k] = 0
					}
					continue
				}

				w := NewBitWriter()
				for _, f := range fields {
					val, ok := frame.Value[f.Name]
					if !ok {
						continue
					}

					if f.IsDelta {
						currentVal := int64(toFloat64(val))
						prev := prevValues[f.Name]
						delta := currentVal - prev
						prevValues[f.Name] = currentVal

						// Write the delta value using the field type
						switch f.Type {
						case "u32":
							w.WriteU32(uint32(delta))
						case "u64":
							w.WriteU64(uint64(delta))
						case "i32":
							w.WriteI32(int32(delta))
						case "i64":
							w.WriteI64(delta)
						default:
							t.Fatalf("unsupported delta type %q", f.Type)
						}
					} else {
						encodeField(t, w, v.Schema, f.Name, val)
					}
				}

				got := w.Finish()
				want, err := hex.DecodeString(frame.ExpectedBytes)
				if err != nil {
					t.Fatalf("frame %d: invalid hex: %v", fi, err)
				}
				if !bytesEqual(got, want) {
					t.Fatalf("frame %d: got %X, want %X", fi, got, want)
				}
			}
		})
	}
}
