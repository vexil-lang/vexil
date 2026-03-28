package vexil

import (
	"errors"
	"fmt"
)

// MaxRecursionDepth is the maximum nesting depth for recursive types.
const MaxRecursionDepth = 64

// MaxBytesLength is the maximum allowed byte length for strings and byte arrays (64 MiB).
const MaxBytesLength = 1 << 26

// MaxCollectionCount is the maximum allowed element count for collections (16 M items).
const MaxCollectionCount = 1 << 24

// MaxLengthPrefixBytes limits the LEB128 length prefix to at most 4 bytes.
const MaxLengthPrefixBytes = 4

// Sentinel errors.
var (
	ErrUnexpectedEOF       = errors.New("unexpected end of input")
	ErrInvalidUTF8         = errors.New("invalid UTF-8 in string field")
	ErrInvalidVarint       = errors.New("invalid or overlong varint encoding")
	ErrRecursionLimit      = errors.New("recursive type nesting exceeded 64 levels")
	ErrSchemaMismatch      = errors.New("schema hash mismatch")
	ErrValueOutOfRange     = errors.New("value does not fit in declared bit width")
	ErrLimitExceeded       = errors.New("length exceeds limit")
	ErrUnknownEnumVariant  = errors.New("unknown enum variant")
	ErrUnknownUnionVariant = errors.New("unknown union variant")
)

// EncodeError represents an error during encoding (packing).
type EncodeError struct {
	Field   string
	Message string
	Err     error
}

func (e *EncodeError) Error() string {
	if e.Field != "" {
		return fmt.Sprintf("encode field %q: %s", e.Field, e.Message)
	}
	return fmt.Sprintf("encode: %s", e.Message)
}

func (e *EncodeError) Unwrap() error {
	return e.Err
}

// DecodeError represents an error during decoding (unpacking).
type DecodeError struct {
	Field   string
	Message string
	Err     error
}

func (e *DecodeError) Error() string {
	if e.Field != "" {
		return fmt.Sprintf("decode field %q: %s", e.Field, e.Message)
	}
	return fmt.Sprintf("decode: %s", e.Message)
}

func (e *DecodeError) Unwrap() error {
	return e.Err
}
