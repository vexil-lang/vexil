# Schema Evolution

Vexil makes schema changes explicit and classifies whether existing message
values can still be decoded safely. Because message values are not internally
length-delimited, adding a field is a breaking change.

## Compatible changes

These changes are safe (v1 and v2 can interoperate):

| Change | Classification |
|--------|---------------|
| Add a variant to `@non_exhaustive` enum/union | Minor |
| Add a new declaration | Minor |
| Mark a field `@deprecated` | Patch |
| Rename a field (ordinal unchanged) | Patch |

## Breaking changes

These changes require all peers to upgrade simultaneously:

| Change | Why |
|--------|-----|
| Add a field | Nested and aggregate message values have no old-schema boundary |
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
  ✗ field "flags" added at @2           BREAKING (major)
  ✗ field "timeout" type u32 → optional<u32>  BREAKING (major)

Result: BREAKING — requires major version bump
```

JSON output for CI integration:

```sh
vexilc compat v1.vexil v2.vexil --format json
```

The `compat` command exits with code 0 for compatible changes and code 1 for breaking changes, making it suitable for CI gates.

## Why appending is breaking

A bounded top-level reader may be able to stop after its known fields, but that
does not make the schema change generally compatible. A nested message is
encoded directly beside its parent's following fields. A newer nested decoder
cannot tell whether the next bytes contain its appended field or the parent's
next field. Arrays of inline messages have the same problem between elements.

Do not treat end of input as a general evolution marker. It would turn some
truncated required fields into defaults. Add a new declaration and migrate
explicitly, or coordinate a major-version transition for the changed message.

## Typed tombstones

When removing a field, use `@removed` to reserve its ordinal and document why
it disappeared. You can retain the original type as historical metadata:

```text
message Config {
    name       @0 : string
    @removed(1, reason: "migrated to timeout_ms") : u32
    timeout_ms @2 : u64
}
```

The type after the tombstone is metadata only. Generated codecs do not read or
write bytes for it, and changing it does not change the schema hash. Removing a
field still changes the wire layout and remains a breaking change; the
tombstone prevents accidental ordinal reuse rather than making old and new
payloads interoperable.

See the [language specification](https://github.com/vexil-lang/vexil/blob/main/spec/language.md) for the full normative reference.
