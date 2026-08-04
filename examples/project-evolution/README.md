# Project Evolution: Imports and Compatibility

This example grows a protocol without losing control of its wire contract. It
combines a multi-file project with explicit compatible and breaking schema
changes.

## What you will build

- shared types imported into a task protocol;
- a generated Rust module tree;
- a compatible declaration addition that suggests a minor version bump;
- a field-type change that `vexilc compat` rejects as breaking.

## Prerequisites

- Rust 1.94 or later
- Python 3.10 or later for the repository example runner

## Run

From the repository root:

```sh
python scripts/examples.py check project-evolution
```

The check regenerates the project into a temporary directory, compares it with
the tracked modules, runs the Rust round trip, accepts the compatible change,
and confirms that the breaking comparison exits with status 1.

## Walkthrough

1. [`schemas/project/types.vexil`](./schemas/project/types.vexil) owns reusable
   priorities, permissions, timestamps, and task state.
2. [`schemas/project/messages.vexil`](./schemas/project/messages.vexil) imports
   those declarations and defines the application messages.
3. [`evolution/v1.vexil`](./evolution/v1.vexil) and
   [`evolution/compatible.vexil`](./evolution/compatible.vexil) add a declaration without
   changing the existing message wire contract.
4. [`evolution/breaking.vexil`](./evolution/breaking.vexil) changes an existing
   field's type and demonstrates the failure path.

## Regenerate

```sh
python scripts/examples.py regenerate project-evolution
python scripts/examples.py check project-evolution
```

Continue with [cross-language interop](../cross-language/) to verify one schema
against four generated targets.
