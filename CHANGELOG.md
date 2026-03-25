# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Full compiler pipeline: lexer, parser, AST, lowering pass, IR, and type checker
- `vexil_lang::parse()` — parses and semantically validates a `.vexil` source string
- `vexil_lang::compile()` — full pipeline through IR and type checking
- IR type definitions and `ErrorClass` variants covering all 56 invalid corpus cases
- Type checker: wire size computation and recursive type detection
- Lowering pass: AST → IR name resolution and `compile()` API
- Semantic validation — all 74 corpus tests passing (18 valid, 56 invalid)
- `vexilc` CLI with [ariadne](https://github.com/zesterer/ariadne) error rendering
- Formal PEG grammar (`spec/vexil-grammar.peg`) derived from the language specification
- 74-file conformance test corpus: `corpus/valid/` (18 files) and `corpus/invalid/` (56 files)

[Unreleased]: https://github.com/vexil-lang/vexil/commits/main
