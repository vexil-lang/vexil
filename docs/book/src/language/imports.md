# Imports

Vexil supports multi-file schemas with explicit imports. This allows you to split large schemas into reusable modules.

## Basic imports

```vexil
import common.types

namespace myapp.protocol

message Request {
    id     @0 : common.types.RequestId
    action @1 : string
}
```

The imported namespace must be resolvable via the include paths passed to `vexilc build`.

## Project compilation

When using imports, use `vexilc build` instead of `vexilc codegen`:

```sh
vexilc build root.vexil --include ./schemas --output ./generated --target rust
```

The compiler:

1. Parses the root file and discovers imports
2. Resolves each import against the include directories
3. Compiles all schemas in topological order (dependencies first)
4. Generates code for each schema with proper cross-file references

## Diamond dependencies

If A imports B and C, and both B and C import D, the compiler deduplicates D. Each type is compiled exactly once, and generated code references the canonical location.

## Generated imports

Each target language handles cross-file references idiomatically:

- **Rust**: `use` statements referencing sibling modules
- **TypeScript**: relative `import` statements with barrel `index.ts` files
- **Go**: standard package imports

See the [language specification](https://github.com/vexil-lang/vexil/blob/main/spec/vexil-spec.md) for the full normative reference.
