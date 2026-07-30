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

Generated Python and its runtime are exercised together against a
representative shared wire matrix. This is not exhaustive for every schema or
environment.

## Source

[`packages/runtime-py/`](https://github.com/vexil-lang/vexil/tree/main/packages/runtime-py)
