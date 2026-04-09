# vexil_runtime

Vexil runtime for Python - binary serialization support for the Vexil language.

## Installation

```bash
pip install vexil_runtime
```

## Usage

```python
from vexil_runtime import BitWriter, BitReader, pack, unpack

# Basic BitWriter/BitReader usage
writer = BitWriter()
writer.write_u8(42)
writer.write_u16(1000)
writer.write_bool(True)
data = writer.finish()

reader = BitReader(data)
value8 = reader.read_u8()      # 42
value16 = reader.read_u16()    # 1000
flag = reader.read_bool()      # True
```

## API

- `BitWriter`: Write bits and bytes to a binary buffer
- `BitReader`: Read bits and bytes from a binary buffer
- `Pack` / `Unpack`: Protocols for serializable types
- `pack(obj)`: Pack an object to bytes
- `unpack(cls, data)`: Unpack bytes to an object

## License

MIT
