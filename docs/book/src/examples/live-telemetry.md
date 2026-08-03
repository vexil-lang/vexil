# Live Telemetry

The flagship example streams local CPU and memory samples from Rust to a browser
using generated stateful delta codecs.

The path has four contract boundaries:

1. the schema marks `SystemSnapshot` with `@delta`;
2. the Rust encoder retains its previous numeric values;
3. the TypeScript decoder applies the matching state transitions;
4. reconnect resets the decoder before a new base frame.

Verify the codecs and reset path without binding a port:

```sh
python scripts/examples.py check live-telemetry
```

To run the dashboard, install its locked Node dependencies and start the Rust
service as described in the guided [example README](https://github.com/vexil-lang/vexil/tree/main/examples/live-telemetry).

Delta encoding is stateful representation, not general compression. Frame size
depends on the sampled values and number of CPU cores.
