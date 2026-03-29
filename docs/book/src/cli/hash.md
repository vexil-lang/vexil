# hash

Print the BLAKE3 hash of a compiled schema.

## Usage

```sh
vexilc hash <file.vexil>
```

## Example

```sh
$ vexilc hash sensor.vexil
a1b2c3d4e5f67890...  sensor.vexil
```

## How it works

The hash is computed from the **canonical form** of the schema, not the raw source text. This means:

- Whitespace differences don't affect the hash
- Comment differences don't affect the hash
- Reordering declarations (without changing semantics) may or may not affect the hash, depending on the canonical form rules

Two schemas that describe the same types with the same encoding produce the same hash. This enables:

- Schema identity verification at connection time
- Content addressing for cached compilations
- Detecting when a schema has actually changed vs. just been reformatted
