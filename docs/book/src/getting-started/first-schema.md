# Your First Schema

Create a file called `hello.vexil`:

```vexil
namespace hello

message Greeting {
    name    @0 : string
    message @1 : string
    count   @2 : u32
}
```

Or use `vexilc init`:

```sh
vexilc init hello
# Creates hello.vexil
```

## Check for errors

```sh
vexilc check hello.vexil
```

If the schema is valid, vexilc prints the schema hash and exits with code 0. Errors show source spans:

```
Error: unknown type `strin`
   ╭─[ hello.vexil:3:18 ]
   │
 3 │     name    @0 : strin
   │                  ──┬──
   │                    ╰── UnknownType
───╯
```

## Understand the schema

- `namespace hello` -- every schema needs a namespace, keeps types from colliding
- `message Greeting` -- a struct-like type with named, ordered fields
- `@0`, `@1`, `@2` -- ordinals determine wire order, not source order. Swap lines around, the wire format stays the same
- `: string`, `: u32` -- field types determine encoding and size

## Schema hash

Every schema has a deterministic BLAKE3 hash:

```sh
vexilc hash hello.vexil
# a1b2c3d4...  hello.vexil
```

Two schemas with identical content produce identical hashes regardless of whitespace or comments. The hash is computed from the canonical form, not the raw source text.
