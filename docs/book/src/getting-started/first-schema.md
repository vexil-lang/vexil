# Your First Schema

Create `hello.vexil`:

```vexil
namespace hello

message Greeting {
    priority @0 : u3
    name     @1 : string @limit(64)
    count    @2 : u32 @varint
}
```

Or ask the CLI for a starting file:

```sh
vexilc init hello
```

## Check the contract

```sh
vexilc check hello.vexil
```

On success, `vexilc` prints the canonical BLAKE3 schema hash and exits with
status 0. On failure, it reports the source span and a diagnostic explaining the
rejected contract.

## Read the schema as wire instructions

- `namespace hello` gives declarations a stable namespace.
- `priority @0 : u3` assigns ordinal 0 and exactly three wire bits.
- `name @1 : string @limit(64)` uses a length-prefixed UTF-8 string with an
  application-visible bound.
- `count @2 : u32 @varint` uses unsigned LEB128 instead of fixed-width `u32`.

Ordinals are durable wire identities. Reordering source lines does not reorder
the encoded fields.

## Inspect the hash

```sh
vexilc hash hello.vexil
```

Comments and formatting do not affect the hash. A change to the compiled
contract does.

Next: [Generating Code](./generating-code.md), or run the complete
[Quickstart](../examples/quickstart.md).
