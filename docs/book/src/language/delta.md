# Delta Encoding

Delta encoding is an annotation that instructs the encoder to write differences between consecutive values instead of absolute values. This is useful for time-series data where consecutive readings are close together.

```vexil
message TimeSeries {
    timestamps @0 : array<u64 @delta>
    values     @1 : array<i32 @delta @zigzag>
}
```

## How it works

With `@delta`, the encoder writes:
1. The first value as-is
2. Each subsequent value as `current - previous`

The decoder reverses the process, accumulating deltas to reconstruct absolute values.

## When to use delta encoding

Delta encoding is most effective when:

- Values increase monotonically (timestamps, sequence numbers)
- Consecutive values are close together (sensor readings, coordinates)
- Combined with `@varint` or `@zigzag` -- small deltas compress to fewer bytes

## Combining annotations

Delta encoding composes with other encoding annotations:

```vexil
message GpsTrack {
    timestamps @0 : array<u64 @delta @varint>    # monotonic, small deltas
    latitudes  @1 : array<i32 @delta @zigzag>    # signed deltas near zero
    longitudes @2 : array<i32 @delta @zigzag>
}
```

> **Note:** Delta encoding support is currently specified but implementation may vary by backend. Check the [limitations document](https://github.com/vexil-lang/vexil/blob/main/docs/limitations-and-gaps.md) for current status.

See the [language specification](https://github.com/vexil-lang/vexil/blob/main/spec/vexil-spec.md) for the full normative reference.
