# Python Runtime

The Python runtime provides bit-level I/O for Vexil-generated Python code.

## Source installation

The runtime is currently available from this repository; it does not make a
public PyPI publication claim. From a repository checkout, install it with:

```sh
python -m pip install ./packages/runtime-py
```

Or, from `packages/runtime-py` itself:

```sh
python -m pip install .
```

It requires Python 3.10 or later.

## Compatibility

The runtime exercises local byte-vector tests. Verify generated Python output
for the schemas and runtime versions you intend to ship before using it in a
cross-language protocol.

## Source

[`packages/runtime-py/`](https://github.com/vexil-lang/vexil/tree/main/packages/runtime-py)
