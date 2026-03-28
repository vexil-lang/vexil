package vexil

// Packer is implemented by types that can encode themselves to wire format.
type Packer interface {
	Pack(w *BitWriter) error
}

// Unpacker is implemented by types that can decode themselves from wire format.
type Unpacker interface {
	Unpack(r *BitReader) error
}
