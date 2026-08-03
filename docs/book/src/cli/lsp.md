# lsp

Start Vexil's diagnostics language server over standard input and output.

> This command is available in current source builds. It is newer than the
> published `vexilc` 0.6.0 CLI and is not included in `cargo install vexilc`
> yet.

## Usage

```sh
vexilc lsp
```

Configure an editor or LSP client to launch `vexilc` with the single argument
`lsp` for Vexil documents. The process reserves stdout for Language Server
Protocol messages; operational errors are written to stderr.

## Supported workflow

The server advertises UTF-16 positions and full-document text synchronization
with open and close notifications. It compiles the editor's in-memory text on
every open or full change, so diagnostics do not require the file to be saved.
Published diagnostics include:

- an end-exclusive source range;
- error or warning severity;
- the compiler diagnostic code;
- `vexilc` as the source;
- the message and any attached notes or suggestions;
- the current document version after open or change.

Replacing invalid text with valid text clears the diagnostics. Closing a
document also clears them.

## Current boundary

This is a diagnostics-only, single-file server. It does not load imports or
projects, so imported names may remain unresolved in the editor even when a
saved project succeeds with `vexilc check --include` or `vexilc build`.

The server does not advertise incremental changes, completion, navigation,
references, rename, hover, formatting, code actions, workspace indexing, or an
editor extension. Unsupported requests receive the standard JSON-RPC
method-not-found response.
