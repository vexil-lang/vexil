# Python Runtime

The Python runtime provides bit-level I/O for Vexil-generated Python code.

## Install

```sh
python -m pip install vexil-runtime
```

It requires Python 3.10 or later.

To work from a repository checkout instead:

```sh
python -m pip install ./packages/runtime-py
```

## Compatibility

Generated Python and its runtime are exercised together against a
representative shared wire matrix. This is not exhaustive for every schema or
environment.

## Source

[`packages/runtime-py/`](https://github.com/vexil-lang/vexil/tree/main/packages/runtime-py)
