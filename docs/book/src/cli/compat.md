# compat

Compare two schema versions and detect breaking changes.

## Usage

```sh
vexilc compat <old.vexil> <new.vexil> [--format <human|json>]
```

## Options

| Option | Default | Description |
|--------|---------|-------------|
| `--format <format>` | `human` | Output format: `human` or `json` |

## Example

```sh
$ vexilc compat v1/sensor.vexil v2/sensor.vexil
  ✓ field "flags" added at @2           compatible (minor)
  ✗ field "timeout" type u32 → u64      BREAKING (major)

Result: BREAKING — requires major version bump
```

### JSON output

```sh
$ vexilc compat v1.vexil v2.vexil --format json
```

```json
{
  "changes": [
    {
      "kind": "field_added",
      "declaration": "SensorReading",
      "field": "flags",
      "detail": "field \"flags\" added at @2",
      "classification": "minor"
    }
  ],
  "result": "compatible",
  "suggested_bump": "minor"
}
```

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Compatible changes only |
| 1 | Breaking changes detected |
| 2 | Schema compilation error |

## Detected changes

The compat checker detects field additions, removals, type changes, ordinal changes, renames, deprecations, encoding changes, variant additions/removals, declaration additions/removals, namespace changes, and flags bit changes.
