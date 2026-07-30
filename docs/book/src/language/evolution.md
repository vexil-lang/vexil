# Schema Evolution

Vexil supports safe schema evolution -- adding fields, deprecating fields, and detecting breaking changes.

## Compatible changes

These changes are safe (v1 and v2 can interoperate):

| Change | Classification |
|--------|---------------|
| Add a field with a new ordinal | Minor |
| Add a variant to `@non_exhaustive` enum/union | Minor |
| Mark a field `@deprecated` | Patch |
| Rename a field (ordinal unchanged) | Patch |

## Breaking changes

These changes require all peers to upgrade simultaneously:

| Change | Why |
|--------|-----|
| Remove a field | Wire layout changes |
| Change a field's type | Wire encoding differs |
| Change a field's ordinal | Wire order changes |
| Add/remove `@varint`, `@zigzag`, `@delta` | Encoding differs |

## Detecting breaking changes

```sh
vexilc compat v1/schema.vexil v2/schema.vexil
```

Output:

```
  ✓ field "flags" added at @2           compatible (minor)
  ✗ field "timeout" type u32 → optional<u32>  BREAKING (major)

Result: BREAKING — requires major version bump
```

JSON output for CI integration:

```sh
vexilc compat v1.vexil v2.vexil --format json
```

The `compat` command exits with code 0 for compatible changes and code 1 for breaking changes, making it suitable for CI gates.

## Forward compatibility

When a v1 decoder receives v2-encoded data with extra fields, it reads its known fields and ignores the trailing bytes. Decoding succeeds rather than failing on the unrecognised data, which is what makes appending a field a compatible change.

The ignored bytes are discarded, not retained -- a v1 decoder cannot re-encode the v2 data it did not understand. See [unknown fields](./messages.md#unknown-fields) for what generated messages do and do not carry.

## Typed tombstones

When removing a field, use `@removed` with the original type to enable decode-and-discard:

```vexil
message Config {
    name       @0 : string
    @removed(1, reason: "migrated to timeout_ms") : u32
    timeout_ms @2 : u64
}
```

The tombstone tells the decoder exactly how many bytes to skip for ordinal 1, even though the field no longer exists in the current schema.

See the [language specification](https://github.com/vexil-lang/vexil/blob/main/spec/language.md) for the full normative reference.
