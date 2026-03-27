# Vexil Language Specification

Version: 0.1.0-draft-r2
Date: 2026-03-25
Status: Draft (post-review revision)
Authors: Orix Systems
License: CC-BY 4.0 (this document) / Apache-2.0 (reference implementation)

## Abstract

Vexil (Validated Exchange Language) is a typed schema definition language with
first-class encoding semantics. It describes the shape, constraints, and wire
encoding of data crossing system boundaries, and generates correct, deterministic
code for multiple target languages from a single source of truth.

Vexil is not a general-purpose programming language. It has no execution model,
no runtime, and no Turing-complete features. It is purely declarative.

The key words "MUST", "MUST NOT", "SHOULD", "SHOULD NOT", and "MAY" in this
document are to be interpreted as described in RFC 2119. An implementation that
does not satisfy a MUST or MUST NOT requirement is non-conformant. An
implementation that does not satisfy a SHOULD requirement is conformant but not
recommended.

---

## §1  Language Definition

### 1.1  What Vexil is

Vexil is a schema definition language (SDL) in the tradition of Protocol Buffers,
Cap'n Proto, and Flatbuffers, distinguished by two properties:

**Encoding semantics are part of the type system.** The type `u4` means exactly
4 bits on the wire — not "an integer that fits in 4 bits." The annotation
`@varint` on a `u64` field changes the wire encoding to unsigned LEB128. The
annotation `@delta` on a numeric field instructs the encoder to write the
difference from the previous value. The schema is the wire contract, not just
the shape contract.

**The schema is the single source of truth.** The canonical form of each schema
has a deterministic BLAKE3 hash. That hash is embedded in generated code as a
compile-time constant and optionally in wire envelopes. A mismatch between the
schema a sender compiled against and the schema a receiver compiled against is
detectable at runtime, before any data corruption occurs.

### 1.2  What Vexil is not

Vexil intentionally excludes:

- Control flow (loops, conditionals, branching)
- Functions or procedures
- Variable binding or assignment
- A runtime or execution environment
- Service or RPC interface definitions (reserved for a future version)

Implementations MUST NOT accept syntax or semantics for any of the above.
Constraint expressions (e.g. `where x > 0`) are declarative — they describe
valid states and generate validation code in target languages, but are not
evaluated by Vexil itself.

### 1.3  Conformance

A conformant Vexil implementation:

1. MUST accept all programs in the valid corpus defined by this specification.
2. MUST reject all programs in the invalid corpus with structured errors (§1.4).
3. MUST produce wire-compatible output: a conformant encoder and conformant
   decoder for the same schema MUST round-trip all valid values identically.
4. MUST produce output satisfying the codegen contract (§12) for at least one
   normative target language. The normative target language for this version
   is Rust.

### 1.4  Error model

Each MUST NOT or invalid condition in this specification defines a distinct error
class. Implementations:

- MUST report each error with: schema file name, line, column, error class
  identifier, and a human-readable description.
- MUST continue analysis after recoverable errors (type errors, semantic errors)
  to report all errors in a single compiler pass. Implementations MUST NOT
  require repeated compilation to discover all errors.
- MUST NOT continue past parse errors in the affected file.
- SHOULD suggest corrective actions for common errors.

---

## §2  Concepts

### 2.1  Schema

A schema is a single `.vexil` file. It MUST contain, in order:

1. Exactly one `namespace` declaration.
2. Zero or more `import` declarations (MUST precede all type declarations).
3. Zero or more type declarations.

A schema containing no type declarations MUST be accepted as valid.

### 2.2  Namespace

A namespace is a dot-separated path identifying a schema (e.g.
`vexil.handshake`). Each component MUST match `[a-z][a-z0-9_]*`. A namespace
MUST contain at least one component.

A **compilation unit** is the complete set of schemas provided as input to a
single compiler invocation. Two schemas with the same namespace MUST NOT be
loaded in the same compilation unit; the compiler MUST reject this as an error.

The namespace prefix `vexil` is reserved. User schemas MUST NOT declare
namespaces beginning with `vexil.`. The following namespaces are reserved for
future use by the specification:

- `vexil.std` — standard library types
- `vexil.transport` — transport framing types
- `vexil.crypto` — field-level encryption types

These reserved namespaces are **provisional** — their contents, semantics, and
even existence are subject to change in future specification versions. A
conformant implementation is NOT required to provide or support any reserved
namespace. The only normative requirement is that user schemas MUST NOT declare
namespaces under the `vexil.` prefix; implementations MUST reject such schemas.

### 2.3  Declaration kinds

| Keyword   | Purpose                                              |
|-----------|------------------------------------------------------|
| `message` | Structured record for wire transmission              |
| `enum`    | Named integer discriminants, no associated data      |
| `flags`   | Named bit positions, combinable via bitwise OR       |
| `union`   | Discriminated sum type with per-variant fields       |
| `newtype` | Single-value wrapper around another type             |
| `config`  | Structured record with defaults; NOT wire-encoded    |

Declaration names MUST match `[A-Z][A-Za-z0-9]*`. Leading or trailing
underscores are not permitted. Declaration names MUST be unique within a schema.

### 2.4  Fields and ordinals

A field is a named, typed, ordinally-identified member of a `message`, `union`
variant, or `newtype`. Field names MUST match `[a-z][a-z0-9_]*`. Field names
MUST be unique within their declaration or variant.

Every field MUST carry an ordinal — a non-negative integer assigned with `@N`
syntax:

```vexil
message Point {
    x @0 : i32
    y @1 : i32
}
```

Ordinals are the stable **wire identity** of a field. Field names MAY be renamed
without affecting wire compatibility. Ordinals MUST NOT be reused within a
declaration, even after a field is removed (see `@removed`, §13.3). Wire
encoding order is ascending ordinal order, regardless of source declaration
order. Ordinals MUST NOT exceed 65535.

`config` fields do not carry ordinals and are not wire-encoded.

In the grammar, `@` followed by one or more digits is an ordinal. `@` followed
by a letter begins an annotation name. The two are syntactically disjoint.

### 2.5  Types

A type is one of: primitive (§3.1), sub-byte (§3.2), semantic (§3.3),
parameterized (§3.4), or user-defined (§3.5).

### 2.6  Annotations

An annotation is metadata attached to a declaration, field, or schema element.
Annotation syntax:

```
@name
@name("string literal")
@name(42)
@name(0xFF)
@name(Identifier)
@name(key: value, key2: "value")
@name(positional_value, named_key: value)
```

Annotation argument types: decimal integer, hexadecimal integer (`0x` prefix),
string literal (`"..."`), identifier (`[A-Za-z][A-Za-z0-9_]*`), boolean
(`true`/`false`).

Multiple annotations on the same element are permitted. The same annotation MUST
NOT appear more than once on the same element unless the annotation definition
explicitly permits repetition. `@doc` is the only standard annotation that
permits repetition (§13.2).

Unknown annotations — annotations not defined in this specification or by the
active backend — MUST NOT cause a parse or type error. Code generation backends
MAY process unknown annotations to produce backend-specific output.

### 2.7  Imports

An import makes declarations from another schema available in the current schema.

```vexil
import vexil.types                               # wildcard
import { SessionId, PaneId } from vexil.types    # named
import vexil.types as T                          # aliased (use as T.SessionId)
import vexil.types @ ^1.0.0                      # version-constrained

# Combined forms:
import vexil.types @ ^1.0.0 as T                # version-constrained + aliased
import { SessionId } from vexil.types @ ^1.0.0  # named + version-constrained
```

All four forms MAY include a version constraint (`@ constraint`). The aliased
form and the named form MAY be combined with a version constraint. Named and
aliased forms MUST NOT be combined with each other.

Imports MUST be placed after the `namespace` declaration and before any type
declarations. Direct and transitive circular imports MUST be rejected. A circular
import exists when the transitive closure of a schema's imports includes the
schema itself.

If two wildcard-imported schemas export the same name, the compiler MUST emit an
error requiring explicit disambiguation. Named imports and aliased imports take
precedence over wildcard imports.

---

## §3  Type System

### 3.1  Primitive types

| Type        | Rust    | Bits | Wire encoding                              |
|-------------|---------|------|--------------------------------------------|
| `bool`      | `bool`  |  1   | 1 bit; 0 = false, 1 = true                |
| `u8`        | `u8`    |  8   | unsigned, little-endian                    |
| `u16`       | `u16`   | 16   | unsigned, little-endian                    |
| `u32`       | `u32`   | 32   | unsigned, little-endian                    |
| `u64`       | `u64`   | 64   | unsigned, little-endian                    |
| `i8`        | `i8`    |  8   | two's complement, little-endian            |
| `i16`       | `i16`   | 16   | two's complement, little-endian            |
| `i32`       | `i32`   | 32   | two's complement, little-endian            |
| `i64`       | `i64`   | 64   | two's complement, little-endian            |
| `f32`       | `f32`   | 32   | IEEE 754 binary32, little-endian           |
| `f64`       | `f64`   | 64   | IEEE 754 binary64, little-endian           |
| `void`      | `()`    |  0   | no bits written                            |

Integer types MUST be encoded in little-endian byte order. Signed integers MUST
use two's complement representation. Float types MUST use IEEE 754
representation in little-endian byte order. NaN values MUST be normalized to
the canonical quiet NaN for the type during encoding (IEEE 754 qNaN: `0x7FC00000`
for f32, `0x7FF8000000000000` for f64). Negative zero (`−0.0`) MUST be preserved.

`bool` participates in sub-byte packing identically to `u1`. A `bool` field
occupies exactly 1 bit in the packed bit stream.

`void` encodes to zero bits. It is valid as the `T` in `result<T, E>`, as the
`E` in `result<T, E>`, and as a union variant with no payload fields.

### 3.2  Sub-byte types

`uN` — unsigned integer in exactly N bits. Valid N: 1 ≤ N ≤ 64, N ∉ {8,16,32,64}.
Valid value range: 0 to 2^N − 1.

`iN` — signed integer in exactly N bits, two's complement. Valid N: 2 ≤ N ≤ 64,
N ∉ {8,16,32,64}. Valid value range: −2^(N−1) to 2^(N−1) − 1.

**Bit packing rule:** Sub-byte fields are packed LSB-first, in ascending ordinal
order. The first field occupies the least-significant bits of the first byte.
Fields cross byte boundaries without padding or alignment.

Worked example — three fields in a message:

```vexil
message Header {
    kind   @0 : u3
    status @1 : u5
    extra  @2 : u6
}
```

Wire layout (14 bits → 2 bytes):

```
Byte 0:  [ status[4] status[3] status[2] status[1] status[0] kind[2] kind[1] kind[0] ]
           bit7       bit6      bit5      bit4      bit3     bit2    bit1    bit0

Byte 1:  [    0        0     extra[5] extra[4] extra[3] extra[2] extra[1] extra[0] ]
          bit7     bit6     bit5     bit4     bit3    bit2     bit1    bit0
```

Bits 7–6 of byte 1 are padding (see below).

**Padding:** A message MUST be padded to a byte boundary at its end. Padding
bits MUST be zero on encode. A decoder MUST ignore padding bits.

An encoder MUST write exactly N bits for a `uN`/`iN` field. A decoder MUST
read exactly N bits. A decoder MUST zero-extend `uN` values and sign-extend
`iN` values when promoting to a wider register type.

`@varint` and `@zigzag` MUST NOT be applied to sub-byte types.

### 3.3  Semantic types

| Type        | Rust           | Wire encoding                                            |
|-------------|----------------|----------------------------------------------------------|
| `string`    | `String`       | unsigned LEB128 byte count + UTF-8 bytes                 |
| `bytes`     | `Vec<u8>`      | unsigned LEB128 byte count + raw bytes                   |
| `rgb`       | `(u8,u8,u8)`   | 8 bits R, 8 bits G, 8 bits B (24 bits total)             |
| `uuid`      | `[u8; 16]`     | 128 bits, big-endian byte order                          |
| `timestamp` | `i64`          | i64 microseconds since Unix epoch (1970-01-01T00:00:00Z) |
| `hash`      | `[u8; 32]`     | 256 bits, opaque raw bytes                               |

`string` MUST be valid UTF-8. A decoder MUST reject a `string` field containing
invalid UTF-8.

`timestamp` MUST use the Unix epoch regardless of host platform. On Windows,
callers MUST convert from the Windows FILETIME epoch (1601-01-01T00:00:00Z)
before encoding.

`hash` is an opaque 256-bit value. No hash algorithm is implied or required by
the type. Callers determine the algorithm by convention.

`string` and `bytes` fields: a decoder MUST reject any field whose LEB128 length
prefix exceeds 67,108,864 (2^26, 64 MiB).

`rgb` is included in the core type system for v0.1. It will be moved to a
standard library in a future version. Implementations MUST support it.

### 3.4  Parameterized types

| Type            | Wire encoding                                                      |
|-----------------|--------------------------------------------------------------------|
| `optional<T>`   | 1-bit presence flag (packed); if 1, T follows; if 0, nothing       |
| `array<T>`      | unsigned LEB128 count, then each element in order                  |
| `map<K, V>`     | unsigned LEB128 pair count, then alternating K, V pairs            |
| `result<T, E>`  | 1-bit discriminant (packed); 0 = Ok (T follows); 1 = Err (E)      |

**`optional<T>`:** The 1-bit presence flag participates in sub-byte packing with
surrounding fields. When the flag is 0, no bits are written for the absent value
and sub-byte packing of subsequent fields continues from the immediately following
bit. When the flag is 1 and T is a byte-aligned type (`message`, `union`,
`string`, `bytes`, semantic types, or multi-byte integers), the sub-byte stream
is flushed to a byte boundary before writing T, then resumes at the next bit
boundary after T. When the flag is 1 and T is a sub-byte or 1-bit type, T's
bits are written immediately after the presence flag with no alignment gap.

**`array<T>`:** Element count MUST NOT exceed 16,777,216 (2^24). A decoder MUST
reject arrays exceeding this limit.

**`map<K, V>`:** Keys MUST be sorted in ascending order as follows:

- Integer key types (`bool`, `u8`–`u64`, `i8`–`i64`, `uN`, `iN`, `enum`,
  `flags`): ascending numeric order (signed types use signed comparison).
- `string`, `bytes`: ascending lexicographic order of the UTF-8 / raw byte
  sequences.
- `uuid`: ascending lexicographic order of the 16-byte big-endian representation.

K MUST be one of: `bool`, `u8`–`u64`, `i8`–`i64`, `uN`, `iN`, `string`,
`bytes`, `uuid`, or a user-defined `enum` or `flags` type. K MUST NOT be
`optional<T>`, `array<T>`, `map<K,V>`, `result<T,E>`, `void`, `f32`, `f64`,
or any `message`, `union`, `newtype`, or `config` type. Pair count MUST NOT
exceed 16,777,216.

**`result<T, E>`:** The 1-bit discriminant participates in sub-byte packing.
`T` MAY be `void`; when T is `void`, the Ok path writes only the discriminant
(0 additional bits). `E` MAY be `void`; when E is `void`, the Err path writes
only the discriminant. `E` MUST follow the same type restrictions as `T`; it
MUST NOT be a `config` type. `E` MAY be any other type including `string`,
`message`, `enum`, or parameterized types.

### 3.5  User-defined types

A field MAY reference any declaration in the same schema or any declaration
available via import. Forward references within the same schema are permitted.
A field MUST NOT reference a `config` declaration.

**Recursive types:** A declaration MAY transitively reference itself. The type
checker MUST detect all recursive references. Backends MUST generate appropriate
heap indirection for recursive field references (e.g. `Box<T>` in Rust, pointer
in C). Recursive references have no effect on wire encoding — the referenced
type is written inline.

---

## §4  Declaration Kinds

### 4.1  message

A `message` is a structured record encoded in ascending ordinal order of its
fields.

```vexil
@type(0x01) @revision(1) @domain(Handshake)
message Hello {
    client_kind @0 : ClientKind
    vnp_version @1 : u8
    session_id  @2 : optional<SessionId>
    pane_id     @3 : optional<PaneId>
}
```

Wire encoding: fields written in ascending ordinal order, followed by zero-bit
padding to the next byte boundary.

A `message` with no fields is valid. Its wire encoding is a single `0x00` byte.
(Zero content bits, padded to a byte boundary, produces one zero byte. The
explicit `0x00` is the result of applying the padding rule to an empty field
sequence; it is not a special case.)

Field ordinals within a message MUST be unique. Gaps between ordinals are
permitted and used for tombstoned fields.

### 4.2  enum

An `enum` defines named integer discriminants with no associated data.

```vexil
@non_exhaustive
enum ClientKind {
    Renderer @0
    Agent    @1
    Plugin   @2
    Shell    @3
}
```

The `@N` ordinal is the wire discriminant value. By default, the wire
representation uses `ceil(log2(max_ordinal + 1))` bits, with a minimum of 1 bit.
For `@non_exhaustive` enums, the minimum representation is 8 bits to accommodate
future variants.

An `enum` MAY specify an explicit backing type to override the automatic width:

```vexil
enum HardwareStatus : u16 {
    Ok     @0
    Warn   @1
    Fault  @2
}
```

Valid backing types are `u8`, `u16`, `u32`, `u64`. The explicit backing type
MUST be wide enough to hold all defined ordinal values; the compiler MUST reject
a backing type that is too narrow. When a backing type is specified, the enum
uses exactly that many bits on the wire regardless of the actual variant count.
This is useful for hardware-mapped protocols and FFI boundaries where the width
is externally fixed.

An `enum` field participates in sub-byte packing identically to a `uN` field
where N is the wire bit width computed above.

For exhaustive enums, a decoder MUST reject unknown discriminant values as
malformed. For `@non_exhaustive` enums, a decoder MUST preserve unknown values
as a numeric unknown variant and MUST NOT reject them.

Enum ordinal values MUST be unique within the enum. Enum ordinals MUST NOT
exceed 65535.

> **`@N` semantics differ between `enum` and `flags`:** In an `enum`, `@N`
> is the discriminant value itself (the number written on the wire). In a
> `flags`, `@N` is the bit position (the wire value is `1 << N`). These are
> not interchangeable. An `enum` variant `Foo @3` has wire value 3. A `flags`
> bit `Foo @3` has wire value 8.

### 4.3  flags

A `flags` declaration defines named bit positions combinable via bitwise OR.

```vexil
flags Permissions {
    Read    @0
    Write   @1
    Execute @2
    Delete  @3
}
```

The `@N` ordinal is the **bit position** (not the bit value). `Read @0` occupies
bit 0 (wire value 1), `Write @1` occupies bit 1 (wire value 2), etc.

Wire representation: the minimum power-of-2 byte width accommodating all defined
bit positions:

| Max bit position | Wire width |
|---|---|
| 0–7 | u8 (8 bits) |
| 8–15 | u16 (16 bits) |
| 16–31 | u32 (32 bits) |
| 32–63 | u64 (64 bits) |

The `flags` wire value MUST be encoded in little-endian byte order (inheriting
the rule from §3.1 for `u16`–`u64`). Bit position N occupies bit N of the
little-endian integer, i.e. byte `N / 8`, bit `N % 8`.

Bit positions MUST NOT exceed 63. A decoder MUST preserve unknown bit positions
(bits set that correspond to no defined flag) and MUST NOT reject them.

A `flags` field with wire width u8 participates in sub-byte packing identically
to a `u8` field. Multi-byte `flags` (u16/u32/u64) MUST be byte-aligned; the
enclosing message sub-byte stream is flushed to a byte boundary before writing
and after reading a multi-byte `flags` field.

### 4.4  union

A `union` is a discriminated (tagged) sum type. Exactly one variant is active
at a time. It is NOT a C-style untagged union.

```vexil
@non_exhaustive
union Color {
    Ansi  @0 { code @0 : u8 }
    Rgb   @1 { r @0 : u8  g @1 : u8  b @2 : u8 }
    Reset @2 {}
}
```

Variant names MUST match `[A-Z][A-Za-z0-9]*`. Variant ordinals MUST be unique
within the union. Variant fields follow the same rules as `message` fields:
each field MUST carry an ordinal (`@N`), ordinals are unique within the variant,
wire encoding is ascending ordinal order, and LSB-first sub-byte packing applies.

Wire encoding:
1. If the enclosing message's sub-byte packing is mid-byte, flush padding to the
   next byte boundary before writing the discriminant.
2. The active variant's ordinal, encoded as unsigned LEB128 (byte-aligned).
3. The encoded byte length of the variant's fields, encoded as unsigned LEB128.
4. The active variant's fields in ascending ordinal order, with sub-byte
   packing and byte-boundary padding at the end.

The byte length prefix in step 3 enables decoders to skip unknown variants
without knowing their field types.

Empty variants (no fields, e.g. `Reset @2 {}`) encode the discriminant (step 2)
and a zero byte length (step 3). No field bytes follow.

For exhaustive unions, a decoder MUST reject unknown discriminant values. For
`@non_exhaustive` unions, a decoder MUST read the byte length prefix and skip
exactly that many bytes, then continue decoding.

Variant ordinals MUST NOT exceed 65535.

### 4.5  newtype

A `newtype` wraps exactly one type under a distinct named type. The wire
encoding is identical to the inner type.

```vexil
newtype SessionId : u64
newtype PaneId    : u64
newtype ExitCode  : i32
```

`newtype` declarations MUST NOT carry ordinals. The inner type MUST NOT be a
`config` type or another `newtype`. The compiler MUST reject `newtype` over
`newtype`; there is no implicit flattening.

### 4.6  config

A `config` declaration defines a structured record with default values. Config
records are NOT wire-encoded.

```vexil
config ServerConfig {
    max_sessions : u32   = 1024
    timeout_ms   : u64   = 30000
    enable_tls   : bool  = false
    log_level    : string = "info"
}
```

Config fields MUST carry a default value. Config field types MUST be one of:
primitive types, semantic types, `enum` types, `flags` types, `optional<T>`,
`array<T>`, or another `config` type (where T is any of the preceding). `map<K,V>`
and `result<T,E>` are NOT valid config field types. Config fields MUST NOT carry
ordinals or encoding annotations (`@varint`, `@zigzag`, `@delta`).

Default values for `array<T>` config fields MUST be written as `[]` (empty
array) or `[v1, v2, ...]` with literal values matching the element type.

Default values for `optional<T>` config fields MUST be written as `none` (the
absent value) or as a literal value of type T (implying presence).

Backends MUST generate a struct or class for `config` declarations with
default-initialized fields. Backends MUST NOT generate `Pack`/`Unpack` or
serialization methods for `config` declarations. Backends MUST generate a
`Default` impl (or equivalent) for `config` types.

---

## §5  Annotation System

### 5.1  Grammar

```
annotation       = "@" ident ( "(" annotation-args? ")" )?
ordinal          = "@" [0-9]+
ident            = [a-z][a-z0-9_]*
upper-ident      = [A-Z][A-Za-z0-9]*
annotation-args  = annotation-arg ( "," annotation-arg )*
annotation-arg   = ( ident ":" )? annotation-value
annotation-value = dec-int | hex-int | string-lit | upper-ident | ident | bool-lit
dec-int          = [0-9]+
hex-int          = "0x" [0-9a-fA-F]+
string-lit       = '"' ( [^"\\] | escape-seq )* '"'
escape-seq       = "\\" ( '"' | "\\" | "n" | "t" | "r" )
bool-lit         = "true" | "false"
```

`ident` (lowercase-only) is used for annotation names and named argument keys.
`upper-ident` (uppercase-leading) is used for identifier argument values that
reference declaration names (e.g. `@domain(Handshake)`). The `bool-lit` keywords
`true` and `false` take precedence over `ident` in the parser.

String literals support the escape sequences `\"`, `\\`, `\n`, `\t`, `\r`. All
other escape sequences MUST be rejected as a parse error. String literal content
MUST be valid UTF-8 after escape processing.

An `ordinal` token (`@` followed by one or more digits) is syntactically
distinct from an `annotation` token (`@` followed by a letter). A parser MUST
NOT treat an ordinal as an annotation.

### 5.2  Placement

| Element | Valid annotation positions |
|---|---|
| Schema | Before `namespace` declaration |
| Declaration | Before declaration keyword |
| Field | Before or after field ordinal; OR on the preceding line |
| Enum/union variant | Before variant name |
| Tombstone (`@removed`) | Standalone statement inside a declaration body (not on a field) |

`@removed` is syntactically a tombstone statement, not a field annotation. It
appears as a standalone line within a `message` or `union` variant body. The
grammar for a declaration body is:

```
decl-body-item = field | tombstone
tombstone      = "@removed" "(" dec-int "," tombstone-args ")"
tombstone-args = tombstone-arg ( "," tombstone-arg )*
tombstone-arg  = ident ":" annotation-value
```

Tombstone arguments use named form only. `reason` MUST be provided.
`since` is OPTIONAL.

### 5.3  Duplication

An annotation MUST NOT appear more than once on the same element unless the
annotation's definition explicitly permits repetition. `@doc` is the only
standard annotation that MAY appear multiple times on the same element.

---

## §6  Import System

### 6.1  Resolution order

1. The compiler loads all directly imported schemas.
2. Transitive imports are resolved in topological dependency order.
3. Within a schema, all type declarations are resolved after all imports are
   loaded.
4. Local declarations shadow wildcard imports.
5. Named and aliased imports shadow wildcard imports.
6. Conflicting wildcard imports (same name exported by two schemas) MUST be
   reported as an error.

### 6.2  Circular import detection

The compiler MUST compute the transitive closure of the import graph. If any
schema appears in its own transitive closure, all schemas in the cycle MUST be
reported as errors.

### 6.3  Version constraints

The constraint `@ ^1.0.0` follows SemVer caret semantics: accepts any version
≥ 1.0.0 and < 2.0.0. The constraint argument MUST be a valid SemVer 2.0.0
string (major.minor.patch required). The shorthand `^1.0` (missing patch) MUST
be rejected as a parse error. The compiler MUST reject imports where the imported
schema's `@version` does not satisfy the constraint. If the imported schema has
no `@version` annotation, the compiler MUST emit a warning and MUST NOT treat
this as an error; the import proceeds as if unconstrained.

---

## §7  Canonical Form

The canonical form of a schema is a deterministic UTF-8 byte sequence used as
input to the schema hash function (§8). Two schemas with identical semantics
MUST produce identical canonical forms.

### 7.1  Algorithm

1. **Comments:** Strip all line comments (`#` to end of line).
2. **Whitespace:** Normalize all whitespace sequences (spaces, tabs, newlines)
   to a single space character. Strip leading and trailing whitespace from the
   result.
3. **Namespace:** Write `namespace ` + namespace path.
4. **Imports:** Write import declarations in ascending lexicographic order of
   their resolved namespace path.
5. **Declarations:** Write declarations in source order. Within each declaration:
   a. Write declaration kind keyword and name.
   b. Write annotations in ascending lexicographic order of annotation name.
      Annotation arguments maintain their source order.
   c. Write fields in ascending ordinal order.
   d. Write each field's annotations in ascending lexicographic order.
6. **Transitive closure:** The canonical form of schema A MUST include the
   canonical forms of all schemas A imports (directly and transitively),
   concatenated before A's own content. The ordering of dependency canonical
   forms MUST use Kahn's algorithm with lexicographic tie-breaking by namespace
   path (ascending). This guarantees a unique linearization for any valid DAG.

The canonical form is computed over the resolved schema graph, not the raw
source text. Two schemas that import different versions of the same dependency
will produce different canonical forms even if their own declarations are
identical.

For repeated `@doc` annotations (the only standard annotation permitting
repetition), instances are written in source order within the
lexicographically-sorted annotation block.

---

## §8  Schema Hash

### 8.1  Computation

The schema hash is the BLAKE3 hash of the schema's canonical form (§7):

```
schema_hash(S) = BLAKE3(canonical_form(S))
```

The hash is 256 bits (32 bytes). BLAKE3 is chosen for speed, security margin,
and the availability of pure-Rust implementations without system dependencies.

### 8.2  Generated constants

Every backend MUST emit the schema hash and version as compile-time constants:

```rust
// Rust example (generated)
pub const SCHEMA_HASH: [u8; 32] = [0xab, 0xcd, /* ... */];
pub const SCHEMA_VERSION: &str = "1.2.0";
```

### 8.3  Runtime verification

If an incoming wire envelope contains a schema hash field, a decoder MAY compare
it to `SCHEMA_HASH`. If schema hash verification is performed and the hashes
differ, the decoder MUST emit
`DecodeError::SchemaMismatch { local: [u8;32], remote: [u8;32] }` and MUST NOT
silently continue decoding.

The schema hash is NOT a cryptographic authenticity guarantee — BLAKE3 without a
key is not a MAC. For authenticity, callers MUST apply a separate signature (see
§14.2).

---

## §9  Schema Versioning

### 9.1  Declaring a version

A schema SHOULD declare its version before the `namespace` declaration:

```vexil
@version("1.2.0")
namespace vexil.handshake
```

Versions MUST follow Semantic Versioning 2.0.0. If no `@version` annotation is
present, the schema version is treated as `"0.0.0"`.

### 9.2  Compatibility model

- **Major** version increments signal breaking changes (§10).
- **Minor** version increments signal compatible additions.
- **Patch** version increments signal documentation or annotation-only changes
  with no wire or semantic effect.

---

## §10  Breaking Change Rules

A change is **compatible** if a decoder compiled against the new schema can
decode messages encoded with the old schema, and vice versa.

| Change | Classification |
|---|---|
| Add a field with a new, previously-unused ordinal | Compatible — minor |
| Add a variant to a `@non_exhaustive` enum or union | Compatible — minor |
| Add a bit position to a `flags` declaration | Compatible — minor |
| Add a new declaration | Compatible — minor |
| Mark a field `@deprecated` | Compatible — patch |
| Add `@since`, `@doc`, or other doc-only annotation | Compatible — patch |
| Rename a field (ordinal unchanged) | Compatible — patch |
| Remove a field with an `@removed` tombstone in place | **Breaking** — major |
| Remove a field **without** an `@removed` tombstone | **Breaking** — major |
| Change a field's type | **Breaking** — major |
| Change a field's ordinal | **Breaking** — major |
| Change an enum variant's ordinal | **Breaking** — major |
| Change a union variant's ordinal | **Breaking** — major |
| Remove an enum variant | **Breaking** — major |
| Remove a union variant | **Breaking** — major |
| Add or remove `@varint`, `@zigzag`, or `@delta` | **Breaking** — major |
| Change `@non_exhaustive` to exhaustive | **Breaking** — major |
| Change a `flags` bit position ordinal | **Breaking** — major |
| Change namespace | **Breaking** — major |
| Reorder fields in source (ordinals unchanged) | No change |

---

## §11  Encoding Edge Cases

These rules are normative.  A conformant implementation MUST handle each
case exactly as specified.  Behaviour on violation is "decode error" or
"encode error" unless stated otherwise.

### 11.1  Empty optionals

An `optional<T>` with no value encodes as a single `0` bit.  No payload
follows.  An `optional<T>` with a value encodes as a `1` bit followed by
`T`'s encoding.

For nested optionals (`optional<optional<T>>`):
- None → `0` (1 bit)
- Some(None) → `1 0` (2 bits)
- Some(Some(v)) → `1 1` followed by v's encoding

### 11.2  Zero-length payloads

A message with zero fields encodes as zero bytes (empty payload).
A union variant with no fields encodes as: discriminant (LEB128) +
length `0` (LEB128).

### 11.3  Maximum recursion depth

Recursive types (self-referencing messages via `optional` or `array`)
have a maximum nesting depth of **64** at encode and decode time.

- Encoder: returns `EncodeError::RecursionLimitExceeded`.
- Decoder: returns `DecodeError::RecursionLimitExceeded`.

Implementations MUST NOT rely on stack overflow for enforcement.

### 11.4  Trailing bytes

When a decoder has consumed all declared fields of a message, any
remaining bytes in the payload are **ignored**.  This enables forward
compatibility — a v2 encoder may append new fields that a v1 decoder
simply skips.

Decoders MUST NOT reject messages with trailing bytes after the last
known field.  Decoders MUST NOT interpret trailing bytes.

### 11.5  Sub-byte boundary at message end

After encoding all fields, the encoder calls `flush_to_byte_boundary()`.
Padding bits MUST be zero.  The decoder calls `flush_to_byte_boundary()`
after reading all known fields, before checking for trailing bytes.

### 11.6  Union discriminant overflow

If a decoder encounters a union discriminant value that does not match
any known variant:

- If the union is `@non_exhaustive`: skip the length-prefixed payload.
  The application receives an opaque discriminant + raw bytes.
- If the union is exhaustive: return `DecodeError::UnknownUnionVariant`.

The length-prefixed payload enables skipping unknown variants without
knowing their structure.

### 11.7  NaN canonicalization

All `f32` NaN values encode as `0x7FC00000` (canonical quiet NaN).
All `f64` NaN values encode as `0x7FF8000000000000` (canonical quiet NaN).

Signaling NaN, negative NaN, and NaN with payload are all mapped to
the canonical quiet NaN **before** encoding.  This ensures bit-identical
encoding for any NaN input.

### 11.8  Negative zero

`-0.0` is preserved on the wire (distinct from `+0.0`).  IEEE 754
defines `-0.0 == +0.0`, but their bit patterns differ.  Vexil preserves
the bit pattern for deterministic encoding.

### 11.9  String encoding errors

String fields use UTF-8.  An encoder receiving non-UTF-8 data returns
`EncodeError::InvalidUtf8`.  A decoder encountering invalid UTF-8 in a
string field returns `DecodeError::InvalidUtf8`.

`bytes` fields have no encoding restriction.

### 11.10  Schema evolution compatibility rules

**Adding a field** (new ordinal, appended in declaration order):
- v1 encoder → v2 decoder: v2 decoder reads known fields, new field gets
  its default value (zero / empty / None depending on type).
- v2 encoder → v1 decoder: v1 decoder reads its known fields, ignores
  trailing bytes (§11.4).

**Adding a variant** to a `@non_exhaustive` union:
- v2 encoder → v1 decoder: v1 decoder reads discriminant, does not
  recognise it, skips length-prefixed payload (§11.6).
- v1 encoder → v2 decoder: works unchanged (old variants still valid).

**Deprecating a field** (marking `@deprecated`):
- No wire change.  `@deprecated` is a source-level annotation only — the
  field is still encoded and decoded normally.  The ordinal remains
  reserved and MUST NOT be reused.

**Changing a required field to `optional<T>`** (**BREAKING**):
- This changes the wire encoding (a 1-bit presence flag is inserted
  before `T`'s encoding).  A v1 decoder reading v2-encoded data would
  misinterpret the presence flag as part of the field value.
- This is classified as a breaking change in §10.

---

## §12  Codegen Contract

### 12.1  Required output (all backends)

For each schema, every conformant backend MUST generate:

1. A named type for each declaration:
   - `message` → struct with fields in ordinal order
   - `enum` → enum with named variants
   - `flags` → bitflag type with named bit constants
   - `union` → tagged union / discriminated enum
   - `newtype` → single-field wrapper struct
   - `config` → struct with default-initialized fields
2. Wire serialization (Pack/Unpack or equivalent) for: `message`, `enum`,
   `flags`, `union`, `newtype`. NOT for `config`.
3. `SCHEMA_HASH: [u8; 32]` constant.
4. `SCHEMA_VERSION: &str` constant.

### 12.2  Rust backend (normative)

The Rust backend MUST additionally:

- Derive `Debug`, `Clone`, `PartialEq` on all generated types.
- Apply `#[non_exhaustive]` to `@non_exhaustive` enums and unions.
- Apply `#[deprecated(since = "...", note = "...")]` to `@deprecated` fields.
- Derive or implement `Default` for `config` types.
- Generate doc comments from `@doc` annotations and `///` schema comments.
- Emit `SCHEMA_HASH` and `SCHEMA_VERSION` as `pub const` items.
- Emit `@removed` tombstones as `// REMOVED @N (since vX.Y.Z): reason` comments
  and populate the `REMOVED_ORDINALS` constant (see §13.3).

### 12.3  All backends (prohibitions)

Backends MUST NOT:

- Silently truncate or widen values beyond their declared type range.
- Generate fields for `@removed` tombstones (tombstones appear as comments and
  the `REMOVED_ORDINALS` constant only).
- Ignore `@deprecated` annotations on fields.

---

## §13  Standard Annotations

### 13.1  Schema-level

**`@version("semver")`**
Declares the schema's semantic version (SemVer 2.0.0 string). MUST appear
before the `namespace` declaration if present. MUST NOT appear more than once
per schema.

### 13.2  Declaration-level

**`@non_exhaustive`**
Valid on `enum` and `union`. Instructs decoders not to reject unknown variants.
For enums, enforces a minimum 8-bit wire representation. For unions, the decoder
MUST skip unknown variant payloads rather than failing.

**`@doc("text")`**
Human-readable documentation string. MAY appear multiple times on the same
element; each instance is appended. Backends MUST emit this as a doc comment in
the target language (Rustdoc, JSDoc, docstring, etc.).

**`@deprecated(since: "version", reason: "text")`**
Marks a declaration as deprecated. `reason` MUST be provided. `since` is
OPTIONAL. Backends MUST emit a deprecation marker in generated code. All
usages of a deprecated type SHOULD produce a compiler warning.

### 13.3  Field lifecycle annotations

**`@since("version")`**
Marks the schema version in which this field was introduced. Backends MUST
include this in generated documentation. No wire effect.

**`@deprecated(since: "version", reason: "text")`**
Marks a field as deprecated. `reason` MUST be provided. `since` is OPTIONAL.
The field remains on the wire; decoders continue to read it. Backends MUST emit
a deprecation marker in generated code (e.g. `#[deprecated]` in Rust,
`@deprecated` in JSDoc).

**`@removed(ordinal, since: "version", reason: "text")`**
Tombstones a previously-used ordinal. This is a standalone statement within a
declaration body (see §5.2 grammar for `tombstone`), not a field annotation.

```vexil
message User {
    display_name @0 : string
        @since("1.0.0")

    @removed(1, since: "1.1.0", reason: "Replaced by display_name for i18n")

    email @2 : string
        @since("1.0.0")
        @deprecated(since: "1.2.0", reason: "Collected server-side")
}
```

`ordinal` MUST be provided (the previously-used field ordinal). `reason` MUST
be provided. `since` is OPTIONAL; if omitted, no version is recorded.

**Purpose:** `@removed` prevents ordinal reuse and serves as historical
documentation. It does NOT make removal a wire-compatible operation. Since Vexil
is not self-describing, a decoder cannot skip a removed field's bytes without
knowing the original type. Removing a field is a **breaking** wire change (see
§10). The tombstone ensures the compiler rejects any future reuse of the ordinal.

The backend MUST emit the tombstone list as generated constants so decoders can
distinguish a tombstoned ordinal from an unrecognized one:

```rust
// Rust (generated)
pub const REMOVED_ORDINALS: &[(u16, &str, &str)] = &[
    (1, "1.1.0", "Replaced by display_name for i18n"),
];
// REMOVED @1 (since 1.1.0): Replaced by display_name for i18n
```

A decoder encountering a tombstoned ordinal MUST emit:
```
DecodeError::RemovedField { ordinal: N, removed_in: "1.1.0", reason: "..." }
```
where `removed_in` is an empty string if `since` was not specified.

### 13.4  Encoding annotations (field-level)

**`@varint`**
Valid on: `u8`, `u16`, `u32`, `u64`.
Encodes the field as unsigned LEB128. A `u8`/`u16`/`u32`/`u64` varint MUST
terminate within 2/3/5/10 bytes respectively; a decoder MUST reject longer
sequences. MUST NOT be combined with `@zigzag`. MAY be combined with `@delta`.
MUST NOT be applied to sub-byte types.

**`@zigzag`**
Valid on: `i8`, `i16`, `i32`, `i64`.
Applies ZigZag mapping then unsigned LEB128. Mapping: `(n << 1) ^ (n >> (bits−1))`
where `>>` is an **arithmetic** (sign-replicating) right shift. This produces
`−1` (all ones) for negative `n` and `0` for non-negative `n`. Results:
0→0, −1→1, 1→2, −2→3, and so on. MUST NOT be combined with `@varint`. MAY be
combined with `@delta`. MUST NOT be applied to sub-byte types.

For `@zigzag` on `i8`, `i16`, `i32`, `i64`, the maximum LEB128 byte counts are
2, 3, 5, and 10 bytes respectively. A decoder MUST reject longer sequences.

**`@delta`**
Valid on: `u8`–`u64`, `i8`–`i64`, `uN`, `iN`, `f32`, `f64`.
Encodes the difference from the previous value of this field. The encoder
computes `delta = current − previous` and writes `delta` using the field's
base wire encoding (or via `@varint`/`@zigzag` if those annotations are also
present). The decoder reads the encoded delta value, adds it to the running
previous value to recover `current`, then updates the running previous value.

Composition with `@varint`: compute the unsigned delta, then LEB128-encode it.
For `uN`/`u8`–`u64` fields, the delta is unsigned (wrap on overflow).
Composition with `@zigzag`: compute the signed delta, then apply ZigZag mapping,
then LEB128-encode. The delta of two `iN` values MAY be negative.

The initial previous value is `0` for integer and sub-byte types, and `+0.0`
(positive zero) for `f32`/`f64`.

A **stream context** is the lifetime of a single encoder or decoder object
instance. The running previous values for all `@delta` fields are reset to their
initial values when the encoder/decoder is constructed or explicitly reset.
Stream context boundaries MUST be established by the enclosing protocol; Vexil
does not define connection-level reset semantics.

Backends MUST generate a stateful encoder/decoder structure that tracks the
running previous value per `@delta` field. The byte-count limits for
`@varint`/`@zigzag` apply to the encoded delta value, not the raw field value.

### 13.5  Validation annotations (field-level)

**`@limit(N)`**
Valid on: `string`, `bytes`, `array<T>`, `map<K, V>`.
Declares the maximum element count (for `array`/`map`) or byte length (for
`string`/`bytes`) that this field may hold. N MUST be a positive integer. N MUST
NOT exceed the global limit for its category (16,777,216 for `array`/`map`;
67,108,864 for `string`/`bytes`).

```vexil
message ChatMessage {
    body    @0 : string   @limit(4096)
    tags    @1 : array<string> @limit(20)
    headers @2 : map<string, string> @limit(64)
}
```

A decoder MUST reject any field whose decoded count or byte length exceeds the
`@limit` value. An encoder MUST reject values exceeding the limit before
encoding. Backends MUST generate validation logic that enforces the limit at
both encode and decode time.

`@limit` is a schema-level constraint, not a transport-level one. It narrows
the global limits defined in §14.1. If `@limit(N)` and the global limit both
apply, the smaller of the two governs.

### 13.6  Wire protocol annotations (message/declaration-level)

**`@type(0xNN)`**
Assigns a wire type discriminant for message dispatch. Value MUST be a
hexadecimal integer in the range `0x00`–`0xFF` (u8). Used with `@domain` for
message routing in the enclosing protocol (e.g. VNP).

**`@domain(Identifier)`**
Assigns a routing domain identifier. The identifier MUST be an unquoted
identifier matching `[A-Z][A-Za-z0-9]*`.

**`@revision(N)`**
Declares the wire revision of this message as a u8. Distinct from the
schema-level `@version("semver")`. Used for per-message versioning within a
protocol that may carry multiple revisions simultaneously.

---

## §14  Security Considerations

### 14.1  Malformed input

Vexil decoders process untrusted data from the network. A conformant decoder
MUST enforce all of the following limits:

- **Message size:** Reject any message whose total encoded byte length exceeds a
  configurable maximum (default: 1 MiB, 1,048,576 bytes).
- **LEB128 varint overflow:** Reject any varint field encoded in more bytes than
  its type allows (u8: 2, u16: 3, u32: 5, u64: 10, i8/zigzag: 2, i16: 3,
  i32: 5, i64: 10). A decoder MUST NOT silently wrap or truncate varint values.
  A decoder MUST also reject overlong LEB128 encodings (trailing zero-value
  continuation bytes that do not alter the decoded value).
- **String and bytes length:** Reject any `string` or `bytes` field whose LEB128
  length prefix exceeds 67,108,864 bytes (64 MiB). The length prefix itself
  MUST NOT exceed 4 LEB128 bytes (sufficient for values up to 2^28).
- **Array and map counts:** Reject any `array<T>` or `map<K,V>` whose LEB128
  count prefix exceeds 16,777,216 (16M elements). The count prefix itself MUST
  NOT exceed 4 LEB128 bytes.
- **UTF-8 validity:** Reject any `string` field that is not valid UTF-8.
- **Recursive depth:** Reject any message containing recursive type references
  nested more than 64 levels deep.

Decoders MUST NOT allocate memory proportional to an untrusted length prefix
before validating that the underlying data is available. Allocating based on an
attacker-controlled length field without bounding is a denial-of-service vector.

### 14.2  Schema hash authenticity

The schema hash (§8) is a content hash (BLAKE3 without a key). It detects
accidental schema version skew. It does NOT prevent an attacker who controls the
network from forging a valid-looking message for a different schema version.
If message authenticity is required, callers MUST apply a separate signature
(e.g. HMAC-BLAKE3 or Ed25519) over the full wire payload.

### 14.3  Code generation safety

The Vexil compiler generates code that is compiled into production systems. The
compiler MUST NOT execute any code derived from schema content. Schema
annotations are data; the compiler MUST parse and validate them, not evaluate
them as expressions.

Unknown annotations MUST be stored as inert data and passed to backends. Backends
MUST validate unknown annotation arguments before acting on them. Annotation
arguments MUST NOT cause shell execution, file system access outside the declared
output directory, or network access during code generation.

### 14.4  Dependency supply chain

Schema imports reference external schemas by namespace path. The compiler MUST
resolve imports from a declared, bounded set of schema paths (e.g. a workspace
directory). The compiler MUST NOT resolve imports by fetching from the network
at compile time unless the user explicitly enables network resolution. Enabling
network resolution SHOULD require explicit opt-in and display the resolved
namespace and version before fetching.

---

## Appendix A  Standard Transport Header (non-normative)

This appendix describes a recommended transport framing for multiplexing Vexil
messages over a byte stream. It is non-normative; protocols MAY define their own
framing. VNP (Vexil Native Protocol) uses this layout.

### A.1  Header layout

```
Byte  Field          Type    Description
───── ────────────── ─────── ────────────────────────────────────────────
0     domain         u8      Routing domain (@domain ordinal)
1     type           u8      Message type (@type value)
2     revision       u8      Wire revision (@revision value)
3     flags          u8      Transport flags (bit 0: has_hash)
4–7   payload_len    u32 LE  Byte length of the payload that follows
8–39  schema_hash    [u8;32] BLAKE3 schema hash (present only if flags.has_hash)
*     payload        bytes   The Vexil-encoded message
```

When `flags.has_hash` is 0, the header is 8 bytes and `schema_hash` is omitted.
When `flags.has_hash` is 1, the header is 40 bytes.

### A.2  Stream framing

Messages are concatenated on the byte stream with no separator. The `payload_len`
field enables a reader to skip unrecognized message types. A reader that does not
recognize a `(domain, type, revision)` triple MUST skip `payload_len` bytes and
continue reading the next header.

### A.3  Relationship to `@delta` stream context

A `@delta` stream context (§13.4) spans the lifetime of the transport connection
or until an explicit reset. A transport MAY define a reset mechanism (e.g. a
reserved `type` value) that resets all `@delta` running values to their initial
states.

---

## Appendix B  Future Work (non-normative)

> **This appendix is entirely non-normative.** Everything below represents
> possible directions for future specification versions. Items may be added,
> redesigned, deferred indefinitely, or dropped entirely. No version of Vexil
> is required to implement any item listed here. Conformance is determined
> solely by the normative sections above.

The following features are under consideration:

- **`vexil.std` standard library:** Common types including `Duration` (i64
  nanoseconds), `Decimal` (fixed-point), `IpAddr` (v4/v6), `Url`, `SemVer`.
- **Constraint expressions:** A subset of boolean logic (`==`, `!=`, `<`, `>`,
  `<=`, `>=`, `&&`, `||`, `!`, field references) for generating validation code.
- **Service definitions:** RPC interface declarations with request/response
  message pairing.
- **`vexil.transport` namespace:** Normative transport framing (promoting
  Appendix A).
- **`encrypted<T>` parameterized type and `vexil.crypto` namespace:**
  Field-level encryption as a core type, not a transport annotation. Vexil-encoded
  data may be stored at rest (disk, database, logs, archives), making field-level
  encryption a schema concern. Early design direction: `[key_id: LEB128]
  [ciphertext_len: LEB128][ciphertext: bytes]` wire encoding — decoders without
  keys skip by length, decoders with keys decrypt to the inner `T`. Algorithm
  selection, key management, and rotation policy would be defined in
  `vexil.crypto`. This design is preliminary and subject to significant revision.
