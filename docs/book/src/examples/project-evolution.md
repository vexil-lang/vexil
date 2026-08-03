# Project Evolution

Real protocols rarely remain in one file or one version. This example combines
two workflows:

- `vexilc build` resolves imports and emits a Rust module tree;
- `vexilc compat` distinguishes an append-only field from a breaking field-type
  change.

Run the complete path from the repository root:

```sh
python scripts/examples.py check project-evolution
```

The command expects the compatible comparison to succeed and the breaking
comparison to exit with status 1. A failure path is part of the example rather
than something the guide asks you to imagine.

Read the guided [example README](https://github.com/vexil-lang/vexil/tree/main/examples/project-evolution)
for the schema layout and expected reports.

Next: [Cross-Language Interop](./cross-language.md).
