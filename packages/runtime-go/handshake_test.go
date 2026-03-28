package vexil

import (
	"testing"
)

func TestHandshakeEncodeDecodeRoundtrip(t *testing.T) {
	var hash [32]byte
	for i := range hash {
		hash[i] = 0xAB
	}
	hs := NewSchemaHandshake(hash, "1.2.3")
	data := hs.Encode()
	decoded, err := DecodeSchemaHandshake(data)
	assertNoError(t, err)
	if decoded.Hash != hash {
		t.Fatal("hash mismatch")
	}
	if decoded.Version != "1.2.3" {
		t.Fatalf("version mismatch: got %q", decoded.Version)
	}
}

func TestHandshakeCheckMatchingHashes(t *testing.T) {
	var hash [32]byte
	for i := range hash {
		hash[i] = 0x42
	}
	local := NewSchemaHandshake(hash, "1.0.0")
	remote := NewSchemaHandshake(hash, "1.0.0")
	result := local.Check(remote)
	if !result.Match {
		t.Fatal("expected match")
	}
}

func TestHandshakeCheckDifferentHashes(t *testing.T) {
	var h1, h2 [32]byte
	for i := range h1 {
		h1[i] = 0x01
		h2[i] = 0x02
	}
	local := NewSchemaHandshake(h1, "1.0.0")
	remote := NewSchemaHandshake(h2, "1.1.0")
	result := local.Check(remote)
	if result.Match {
		t.Fatal("expected mismatch")
	}
	if result.LocalVersion != "1.0.0" {
		t.Fatalf("local version: got %q", result.LocalVersion)
	}
	if result.RemoteVersion != "1.1.0" {
		t.Fatalf("remote version: got %q", result.RemoteVersion)
	}
}

func TestHandshakeWireSizeCompact(t *testing.T) {
	var hash [32]byte
	hs := NewSchemaHandshake(hash, "1.0.0")
	data := hs.Encode()
	// 32 (hash) + 1 (LEB128 len) + 5 (version) = 38
	if len(data) != 38 {
		t.Fatalf("got %d bytes, want 38", len(data))
	}
}
