package vexil

// SchemaHandshake holds schema identity for connection-time negotiation.
type SchemaHandshake struct {
	Hash    [32]byte
	Version string
}

// HandshakeResult represents the result of comparing two handshakes.
type HandshakeResult struct {
	Match          bool
	LocalVersion   string
	RemoteVersion  string
	LocalHash      [32]byte
	RemoteHash     [32]byte
}

// NewSchemaHandshake creates a new SchemaHandshake.
func NewSchemaHandshake(hash [32]byte, version string) *SchemaHandshake {
	return &SchemaHandshake{Hash: hash, Version: version}
}

// Encode serializes the handshake to wire format: 32 raw hash bytes + LEB128-prefixed version string.
func (h *SchemaHandshake) Encode() []byte {
	w := NewBitWriter()
	w.WriteRawBytes(h.Hash[:])
	w.WriteString(h.Version)
	return w.Finish()
}

// DecodeSchemaHandshake deserializes a handshake from wire format.
func DecodeSchemaHandshake(data []byte) (*SchemaHandshake, error) {
	r := NewBitReader(data)
	hashBytes, err := r.ReadRawBytes(32)
	if err != nil {
		return nil, err
	}
	var hash [32]byte
	copy(hash[:], hashBytes)
	version, err := r.ReadString()
	if err != nil {
		return nil, err
	}
	return &SchemaHandshake{Hash: hash, Version: version}, nil
}

// Check compares this handshake with a remote one.
func (h *SchemaHandshake) Check(remote *SchemaHandshake) HandshakeResult {
	if h.Hash == remote.Hash {
		return HandshakeResult{Match: true}
	}
	return HandshakeResult{
		Match:         false,
		LocalVersion:  h.Version,
		RemoteVersion: remote.Version,
		LocalHash:     h.Hash,
		RemoteHash:    remote.Hash,
	}
}
