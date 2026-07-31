# Generated-code checks

This private npm workspace pins the native static checkers used for generated
Python and TypeScript goldens.

```console
npm ci
npm run check:python
npm run check:typescript
```

The Pyright configuration checks every generated Python golden in strict mode
against Python 3.10 and the in-repository `vexil_runtime` package. The
TypeScript configuration checks every generated TypeScript golden with strict,
no-emit, and unused-symbol diagnostics against the in-repository runtime.
