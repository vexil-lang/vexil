// Package pack provides Pack/Unpack interfaces for Vexil types.
package pack

import "github.com/vexil-lang/vexil-runtime/bitio"

// Packer is the interface for types that can be packed.
type Packer interface {
	Pack(w *bitio.Writer) error
}

// Unpacker is the interface for types that can be unpacked.
type Unpacker interface {
	Unpack(r *bitio.Reader) error
}
