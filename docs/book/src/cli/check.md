# check

Validate a Vexil schema file and print its BLAKE3 hash.

## Usage

```sh
vexilc check <file.vexil>
```

## Example

```sh
$ vexilc check sensor.vexil
schema hash: a1b2c3d4e5f6...
```

If the schema has errors, they are printed with source spans and the command exits with code 1:

```
Error: unknown type `strin`
   ╭─[ sensor.vexil:4:18 ]
   │
 4 │     name    @0 : strin
   │                  ──┬──
   │                    ╰── UnknownType
───╯
```

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Schema is valid |
| 1 | Schema has errors |
