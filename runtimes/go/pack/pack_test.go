package pack

import (
	"bytes"
	"testing"

	"github.com/vexil-lang/vexil-runtime/bitio"
)

// TestType implements both Packer and Unpacker interfaces
type TestType struct {
	Value uint32
	Flag  bool
}

func (t *TestType) Pack(w *bitio.Writer) error {
	if err := w.WriteU32(t.Value); err != nil {
		return err
	}
	if err := w.WriteBool(t.Flag); err != nil {
		return err
	}
	return w.Flush()
}

func (t *TestType) Unpack(r *bitio.Reader) error {
	v, err := r.ReadU32()
	if err != nil {
		return err
	}
	t.Value = v

	f, err := r.ReadBool()
	if err != nil {
		return err
	}
	t.Flag = f
	return nil
}

// Verify interface implementations
var _ Packer = (*TestType)(nil)
var _ Unpacker = (*TestType)(nil)

func TestPackUnpack(t *testing.T) {
	original := &TestType{Value: 0xDEADBEEF, Flag: true}

	var buf bytes.Buffer
	w := bitio.NewWriter(&buf)

	if err := original.Pack(w); err != nil {
		t.Fatalf("Pack failed: %v", err)
	}

	result := &TestType{}
	r := bitio.NewReader(&buf)
	if err := result.Unpack(r); err != nil {
		t.Fatalf("Unpack failed: %v", err)
	}

	if result.Value != original.Value {
		t.Errorf("Value: got 0x%x, want 0x%x", result.Value, original.Value)
	}
	if result.Flag != original.Flag {
		t.Errorf("Flag: got %v, want %v", result.Flag, original.Flag)
	}
}
