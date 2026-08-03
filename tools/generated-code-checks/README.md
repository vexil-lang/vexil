# Generated-code checks

This npm workspace pins the static checkers used for generated TypeScript and
Python fixtures.

```console
npm ci
npm run check:typescript
npm run check:python
```

Pyright checks every generated Python fixture in strict mode against Python
3.10 and the in-repository `vexil_runtime` package. TypeScript uses strict,
no-emit, and unused-symbol checks against the in-repository runtime.

These checks complement each generator's Rust golden tests and native runtime
tests. They detect invalid generated source; they do not by themselves prove
wire compatibility.
