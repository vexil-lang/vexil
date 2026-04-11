package vexil

import (
	"encoding/hex"
	"encoding/json"
	"math"
	"os"
	"strings"
	"testing"
)

// primitiveVector represents a compliance test vector for primitive types.
type primitiveVector struct {
	Name          string                 `json:"name"`
	Schema        string                 `json:"schema"`
	Type          string                 `json:"type"`
	Value         map[string]interface{} `json:"value"`
	ExpectedBytes string                 `json:"expected_bytes"`
	Notes         string                 `json:"notes"`
}

// subByteVector is the same structure as primitiveVector.
type subByteVector = primitiveVector

// messageVector is the same structure as primitiveVector.
type messageVector = primitiveVector

func loadVectors[T any](t *testing.T, filename string) []T {
	t.Helper()
	data, err := os.ReadFile("../../compliance/vectors/" + filename)
	if err != nil {
		t.Fatalf("failed to read %s: %v", filename, err)
	}
	var vectors []T
	if err := json.Unmarshal(data, &vectors); err != nil {
		t.Fatalf("failed to parse %s: %v", filename, err)
	}
	return vectors
}

func hexToBytes(t *testing.T, h string) []byte {
	t.Helper()
	b, err := hex.DecodeString(h)
	if err != nil {
		t.Fatalf("invalid hex %q: %v", h, err)
	}
	return b
}

// encodeValue writes a single value to the BitWriter based on the field schema type.
func encodeField(t *testing.T, w *BitWriter, schema string, fieldName string, value interface{}) {
	t.Helper()
	fieldType := extractFieldType(schema, fieldName)
	switch fieldType {
	case "bool":
		w.WriteBool(value.(bool))
	case "u8":
		w.WriteU8(uint8(toFloat64(value)))
	case "u16":
		w.WriteU16(uint16(toFloat64(value)))
	case "u32":
		w.WriteU32(uint32(toFloat64(value)))
	case "u64":
		w.WriteU64(uint64(toFloat64(value)))
	case "i8":
		w.WriteI8(int8(toFloat64(value)))
	case "i16":
		w.WriteI16(int16(toFloat64(value)))
	case "i32":
		w.WriteI32(int32(toFloat64(value)))
	case "i64":
		w.WriteI64(int64(toFloat64(value)))
	case "f32":
		s, ok := value.(string)
		if ok && s == "NaN" {
			w.WriteF32(float32(math.NaN()))
		} else if ok && s == "-0.0" {
			w.WriteF32(math.Float32frombits(0x80000000))
		} else {
			w.WriteF32(float32(toFloat64(value)))
		}
	case "f64":
		s, ok := value.(string)
		if ok && s == "NaN" {
			w.WriteF64(math.NaN())
		} else if ok && s == "-0.0" {
			w.WriteF64(math.Float64frombits(0x8000000000000000))
		} else {
			w.WriteF64(toFloat64(value))
		}
	case "string":
		w.WriteString(value.(string))
	default:
		// Handle sub-byte types like u1, u3, u5, u6
		if len(fieldType) >= 2 && fieldType[0] == 'u' {
			bits := uint8(0)
			for _, c := range fieldType[1:] {
				bits = bits*10 + uint8(c-'0')
			}
			w.WriteBits(uint64(toFloat64(value)), bits)
		} else {
			t.Fatalf("unsupported field type %q", fieldType)
		}
	}
}

func toFloat64(v interface{}) float64 {
	switch val := v.(type) {
	case float64:
		return val
	case int:
		return float64(val)
	case int64:
		return float64(val)
	default:
		return 0
	}
}

// extractFieldType extracts the type of a field from a simple schema string.
// Handles schemas like "message M { v @0 : bool  x @1 : u32 }"
func extractFieldType(schema string, fieldName string) string {
	// Tokenize the body content
	braceStart := strings.Index(schema, "{")
	braceEnd := strings.LastIndex(schema, "}")
	if braceStart == -1 || braceEnd == -1 {
		return ""
	}
	body := schema[braceStart+1 : braceEnd]
	tokens := strings.Fields(body)
	// Find: fieldName @N : type
	for i := 0; i < len(tokens)-3; i++ {
		if tokens[i] == fieldName && len(tokens[i+1]) >= 2 && tokens[i+1][0] == '@' && tokens[i+2] == ":" {
			return tokens[i+3]
		}
	}
	return ""
}

// extractFieldNames extracts field names in order from a schema message definition.
// Handles both multi-line and single-line schemas like "message M { v @0 : bool }"
func extractFieldNames(schema string) []string {
	var names []string
	// Use regex-like approach: find all "name @N : type" patterns
	// First, extract content between { and }
	braceStart := strings.Index(schema, "{")
	braceEnd := strings.LastIndex(schema, "}")
	if braceStart == -1 || braceEnd == -1 || braceEnd <= braceStart {
		return names
	}
	body := schema[braceStart+1 : braceEnd]
	// Split by whitespace and find field definitions
	// Fields look like: name @N : type
	// We split on multiple spaces to handle inline definitions
	tokens := strings.Fields(body)
	for i := 0; i < len(tokens); i++ {
		// Look for @N pattern (starts with @, followed by digits)
		if len(tokens[i]) >= 2 && tokens[i][0] == '@' {
			isOrdinal := true
			for _, c := range tokens[i][1:] {
				if c < '0' || c > '9' {
					isOrdinal = false
					break
				}
			}
			if isOrdinal && i > 0 {
				// Previous token is the field name
				name := tokens[i-1]
				names = append(names, name)
			}
		}
	}
	return names
}

func TestCompliancePrimitives(t *testing.T) {
	vectors := loadVectors[primitiveVector](t, "primitives.json")
	for _, v := range vectors {
		t.Run(v.Name, func(t *testing.T) {
			w := NewBitWriter()
			fields := extractFieldNames(v.Schema)
			for _, fn := range fields {
				val, ok := v.Value[fn]
				if !ok {
					continue
				}
				encodeField(t, w, v.Schema, fn, val)
			}
			got := w.Finish()
			want := hexToBytes(t, v.ExpectedBytes)
			if !bytesEqual(got, want) {
				t.Fatalf("got %X, want %X", got, want)
			}
		})
	}
}

func TestComplianceSubByte(t *testing.T) {
	vectors := loadVectors[subByteVector](t, "sub_byte.json")
	for _, v := range vectors {
		t.Run(v.Name, func(t *testing.T) {
			w := NewBitWriter()
			fields := extractFieldNames(v.Schema)
			for _, fn := range fields {
				val, ok := v.Value[fn]
				if !ok {
					continue
				}
				encodeField(t, w, v.Schema, fn, val)
			}
			got := w.Finish()
			want := hexToBytes(t, v.ExpectedBytes)
			if !bytesEqual(got, want) {
				t.Fatalf("got %X, want %X", got, want)
			}
		})
	}
}

func TestComplianceMessages(t *testing.T) {
	vectors := loadVectors[messageVector](t, "messages.json")
	for _, v := range vectors {
		t.Run(v.Name, func(t *testing.T) {
			w := NewBitWriter()
			fields := extractFieldNames(v.Schema)
			if len(fields) == 0 {
				// Empty message
				got := w.Finish()
				want := hexToBytes(t, v.ExpectedBytes)
				if !bytesEqual(got, want) {
					t.Fatalf("got %X, want %X", got, want)
				}
				return
			}
			for _, fn := range fields {
				val, ok := v.Value[fn]
				if !ok {
					continue
				}
				encodeField(t, w, v.Schema, fn, val)
			}
			got := w.Finish()
			want := hexToBytes(t, v.ExpectedBytes)
			if !bytesEqual(got, want) {
				t.Fatalf("got %X, want %X", got, want)
			}
		})
	}
}

func TestComplianceOptionals(t *testing.T) {
	vectors := loadVectors[primitiveVector](t, "optionals.json")
	for _, v := range vectors {
		t.Run(v.Name, func(t *testing.T) {
			w := NewBitWriter()
			val := v.Value["v"]
			if val == nil {
				w.WriteBool(false)
			} else {
				w.WriteBool(true)
				w.FlushToByteBoundary()
				w.WriteU32(uint32(toFloat64(val)))
			}
			got := w.Finish()
			want := hexToBytes(t, v.ExpectedBytes)
			if !bytesEqual(got, want) {
				t.Fatalf("got %X, want %X", got, want)
			}
		})
	}
}

func TestComplianceEnums(t *testing.T) {
	vectors := loadVectors[primitiveVector](t, "enums.json")
	variantMap := map[string]int{"Active": 0, "Inactive": 1}
	for _, v := range vectors {
		t.Run(v.Name, func(t *testing.T) {
			w := NewBitWriter()
			variant := v.Value["v"].(string)
			discriminant := variantMap[variant]
			w.WriteBits(uint64(discriminant), 1)
			got := w.Finish()
			want := hexToBytes(t, v.ExpectedBytes)
			if !bytesEqual(got, want) {
				t.Fatalf("got %X, want %X", got, want)
			}
		})
	}
}

func TestComplianceUnions(t *testing.T) {
	vectors := loadVectors[primitiveVector](t, "unions.json")
	for _, v := range vectors {
		t.Run(v.Name, func(t *testing.T) {
			w := NewBitWriter()
			unionVal := v.Value["v"].(map[string]interface{})
			variant := unionVal["variant"].(string)
			discriminant := 0
			if variant == "Rect" {
				discriminant = 1
			}

			pw := NewBitWriter()
			if variant == "Circle" {
				pw.WriteF32(float32(toFloat64(unionVal["radius"])))
			} else {
				pw.WriteF32(float32(toFloat64(unionVal["w"])))
				pw.WriteF32(float32(toFloat64(unionVal["h"])))
			}
			payload := pw.Finish()

			w.WriteLeb128(uint64(discriminant))
			w.WriteLeb128(uint64(len(payload)))
			w.WriteRawBytes(payload)
			got := w.Finish()
			want := hexToBytes(t, v.ExpectedBytes)
			if !bytesEqual(got, want) {
				t.Fatalf("got %X, want %X", got, want)
			}
		})
	}
}

func TestComplianceArraysMaps(t *testing.T) {
	vectors := loadVectors[primitiveVector](t, "arrays_maps.json")
	for _, v := range vectors {
		t.Run(v.Name, func(t *testing.T) {
			w := NewBitWriter()
			val := v.Value["v"]
			if arr, ok := val.([]interface{}); ok {
				w.WriteLeb128(uint64(len(arr)))
				for _, elem := range arr {
					w.WriteU32(uint32(toFloat64(elem)))
				}
			} else if m, ok := val.(map[string]interface{}); ok {
				entries := len(m)
				w.WriteLeb128(uint64(entries))
				for k, val := range m {
					w.WriteString(k)
					w.WriteU32(uint32(toFloat64(val)))
				}
			}
			got := w.Finish()
			want := hexToBytes(t, v.ExpectedBytes)
			if !bytesEqual(got, want) {
				t.Fatalf("got %X, want %X", got, want)
			}
		})
	}
}

func TestComplianceEvolution(t *testing.T) {
	data, err := os.ReadFile("../../compliance/vectors/evolution.json")
	if err != nil {
		t.Fatalf("failed to read evolution.json: %v", err)
	}
	var vectors []struct {
		Name        string                 `json:"name"`
		SchemaV1    string                 `json:"schema_v1"`
		SchemaV2    string                 `json:"schema_v2"`
		Type        string                 `json:"type"`
		ValueV1     map[string]interface{} `json:"value_v1"`
		ValueV2     map[string]interface{} `json:"value_v2"`
		EncodedV1   string                 `json:"encoded_v1"`
		EncodedV2   string                 `json:"encoded_v2"`
		DecodedAsV1 map[string]interface{} `json:"decoded_as_v1"`
		DecodedAsV2 map[string]interface{} `json:"decoded_as_v2"`
		Notes       string                 `json:"notes"`
	}
	if err := json.Unmarshal(data, &vectors); err != nil {
		t.Fatalf("failed to parse evolution.json: %v", err)
	}

	for _, v := range vectors {
		t.Run(v.Name, func(t *testing.T) {
			if v.Name == "v1_encode_v2_decode_appended_field" {
				w := NewBitWriter()
				w.WriteU32(uint32(toFloat64(v.ValueV1["x"])))
				got := w.Finish()
				want := hexToBytes(t, v.EncodedV1)
				if !bytesEqual(got, want) {
					t.Fatalf("v1 encode: got %X, want %X", got, want)
				}
			} else if v.Name == "v2_encode_v1_decode_trailing_ignored" {
				w := NewBitWriter()
				w.WriteU32(uint32(toFloat64(v.ValueV2["x"])))
				w.WriteU16(uint16(toFloat64(v.ValueV2["y"])))
				got := w.Finish()
				want := hexToBytes(t, v.EncodedV2)
				if !bytesEqual(got, want) {
					t.Fatalf("v2 encode: got %X, want %X", got, want)
				}
			}
		})
	}
}
