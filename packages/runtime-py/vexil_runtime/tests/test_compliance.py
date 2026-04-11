"""Compliance tests for Python runtime against compliance vectors."""

from __future__ import annotations

import json
import math
import os
import struct
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))

from vexil_runtime import BitWriter, BitReader

VECTORS_DIR = Path(__file__).parent.parent.parent.parent.parent / "compliance" / "vectors"


def hex_to_bytes(h: str) -> bytes:
    return bytes.fromhex(h)


def to_hex(b: bytes) -> str:
    return b.hex()


def extract_field_names(schema: str) -> list[str]:
    names = []
    brace_start = schema.find("{")
    brace_end = schema.rfind("}")
    if brace_start == -1 or brace_end == -1:
        return names
    body = schema[brace_start + 1 : brace_end]
    tokens = body.split()
    for i, tok in enumerate(tokens):
        if len(tok) >= 2 and tok[0] == "@":
            is_ordinal = all(c.isdigit() for c in tok[1:])
            if is_ordinal and i > 0:
                names.append(tokens[i - 1])
    return names


def extract_field_type(schema: str, field_name: str) -> str:
    brace_start = schema.find("{")
    brace_end = schema.rfind("}")
    if brace_start == -1 or brace_end == -1:
        return ""
    body = schema[brace_start + 1 : brace_end]
    tokens = body.split()
    for i, tok in enumerate(tokens):
        if tok == field_name and i + 2 < len(tokens) and tokens[i + 1].startswith("@"):
            for j in range(i + 2, len(tokens)):
                if tokens[j] == ":":
                    return tokens[j + 1] if j + 1 < len(tokens) else ""
    return ""


def encode_field(w: BitWriter, schema: str, field_name: str, value) -> None:
    field_type = extract_field_type(schema, field_name)
    if field_type == "bool":
        w.write_bool(bool(value))
    elif field_type == "u8":
        w.write_u8(int(value))
    elif field_type == "u16":
        w.write_u16(int(value))
    elif field_type == "u32":
        w.write_u32(int(value))
    elif field_type == "u64":
        w.write_u64(int(value))
    elif field_type == "i8":
        w.write_i8(int(value))
    elif field_type == "i16":
        w.write_i16(int(value))
    elif field_type == "i32":
        w.write_i32(int(value))
    elif field_type == "i64":
        w.write_i64(int(value))
    elif field_type == "f32":
        if isinstance(value, str):
            if value == "NaN":
                w.write_f32(float("nan"))
            else:
                w.write_f32(float(value))
        else:
            w.write_f32(float(value))
    elif field_type == "f64":
        if isinstance(value, str):
            if value == "NaN":
                w.write_f64(float("nan"))
            elif value == "-0.0":
                w.write_f64(-0.0)
            else:
                w.write_f64(float(value))
        else:
            w.write_f64(float(value))
    elif field_type == "string":
        w.write_string(str(value))
    elif field_type.startswith("u") and field_type[1:].isdigit():
        bits = int(field_type[1:])
        if bits < 8:
            w.write_bits(int(value), bits)
    else:
        raise ValueError(f"unsupported field type: {field_type}")


def encode_value(schema: str, value: dict) -> bytes:
    w = BitWriter()
    fields = extract_field_names(schema)
    for fn in fields:
        if fn in value:
            encode_field(w, schema, fn, value[fn])
    return w.finish()


def load_vectors(filename: str) -> list:
    with open(VECTORS_DIR / filename) as f:
        return json.load(f)


class TestCompliancePrimitives:
    vectors = load_vectors("primitives.json")

    @pytest.mark.parametrize("vec", vectors, ids=[v["name"] for v in vectors])
    def test_encode_matches_expected_bytes(self, vec) -> None:
        encoded = encode_value(vec["schema"], vec["value"])
        assert to_hex(encoded) == vec["expected_bytes"]


class TestComplianceSubByte:
    vectors = load_vectors("sub_byte.json")

    @pytest.mark.parametrize("vec", vectors, ids=[v["name"] for v in vectors])
    def test_encode_matches_expected_bytes(self, vec) -> None:
        encoded = encode_value(vec["schema"], vec["value"])
        assert to_hex(encoded) == vec["expected_bytes"]


class TestComplianceMessages:
    vectors = load_vectors("messages.json")

    @pytest.mark.parametrize("vec", vectors, ids=[v["name"] for v in vectors])
    def test_encode_matches_expected_bytes(self, vec) -> None:
        encoded = encode_value(vec["schema"], vec["value"])
        assert to_hex(encoded) == vec["expected_bytes"]


class TestComplianceOptionals:
    vectors = load_vectors("optionals.json")

    @pytest.mark.parametrize("vec", vectors, ids=[v["name"] for v in vectors])
    def test_encode_matches_expected_bytes(self, vec) -> None:
        w = BitWriter()
        v = vec["value"]["v"]
        if v is None:
            w.write_bool(False)
        else:
            w.write_bool(True)
            w.flush_to_byte_boundary()
            w.write_u32(int(v))
        assert to_hex(w.finish()) == vec["expected_bytes"]


class TestComplianceEnums:
    vectors = load_vectors("enums.json")

    @pytest.mark.parametrize("vec", vectors, ids=[v["name"] for v in vectors])
    def test_encode_matches_expected_bytes(self, vec) -> None:
        w = BitWriter()
        variant = vec["value"]["v"]
        discriminant = 0 if variant == "Active" else 1
        w.write_bits(discriminant, 1)
        assert to_hex(w.finish()) == vec["expected_bytes"]


class TestComplianceUnions:
    vectors = load_vectors("unions.json")

    @pytest.mark.parametrize("vec", vectors, ids=[v["name"] for v in vectors])
    def test_encode_matches_expected_bytes(self, vec) -> None:
        w = BitWriter()
        union_val = vec["value"]["v"]
        variant = union_val["variant"]
        discriminant = 0 if variant == "Circle" else 1

        payload_w = BitWriter()
        if variant == "Circle":
            payload_w.write_f32(float(union_val["radius"]))
        else:
            payload_w.write_f32(float(union_val["w"]))
            payload_w.write_f32(float(union_val["h"]))
        payload = payload_w.finish()

        w.write_leb128(discriminant)
        w.write_leb128(len(payload))
        w.write_raw_bytes(payload, len(payload))
        assert to_hex(w.finish()) == vec["expected_bytes"]


class TestComplianceArraysMaps:
    vectors = load_vectors("arrays_maps.json")

    @pytest.mark.parametrize("vec", vectors, ids=[v["name"] for v in vectors])
    def test_encode_matches_expected_bytes(self, vec) -> None:
        w = BitWriter()
        v = vec["value"]["v"]

        if isinstance(v, list):
            w.write_leb128(len(v))
            for elem in v:
                w.write_u32(int(elem))
        elif isinstance(v, dict):
            entries = list(v.items())
            w.write_leb128(len(entries))
            for key, val in entries:
                w.write_string(key)
                w.write_u32(int(val))

        assert to_hex(w.finish()) == vec["expected_bytes"]


# Note: v1_types.json is excluded as it contains invalid JSON (unquoted hex in array values)


class TestComplianceEvolution:
    vectors = load_vectors("evolution.json")

    def test_v1_encode_produces_expected_bytes(self) -> None:
        vec = next(v for v in self.vectors if v["name"] == "v1_encode_v2_decode_appended_field")
        w = BitWriter()
        w.write_u32(int(vec["value_v1"]["x"]))
        assert to_hex(w.finish()) == vec["encoded_v1"]

    def test_v1_bytes_decoded_as_v2_fills_default(self) -> None:
        vec = next(v for v in self.vectors if v["name"] == "v1_encode_v2_decode_appended_field")
        r = BitReader(hex_to_bytes(vec["encoded_v1"]))
        x = r.read_u32()
        assert x == vec["decoded_as_v2"]["x"]

    def test_v2_encode_produces_expected_bytes(self) -> None:
        vec = next(v for v in self.vectors if v["name"] == "v2_encode_v1_decode_trailing_ignored")
        w = BitWriter()
        w.write_u32(int(vec["value_v2"]["x"]))
        w.write_u16(int(vec["value_v2"]["y"]))
        assert to_hex(w.finish()) == vec["encoded_v2"]

    def test_v2_bytes_decoded_as_v1_ignores_trailing(self) -> None:
        vec = next(v for v in self.vectors if v["name"] == "v2_encode_v1_decode_trailing_ignored")
        r = BitReader(hex_to_bytes(vec["encoded_v2"]))
        x = r.read_u32()
        assert x == vec["decoded_as_v1"]["x"]
        assert r.remaining() > 0


def parse_delta_fields(schema: str) -> list[dict]:
    fields = []
    lines = schema.split("\n")
    next_is_delta = False
    for line in lines:
        line = line.strip()
        if line == "@delta":
            next_is_delta = True
            continue
        at_idx = line.find(" @")
        if at_idx == -1:
            next_is_delta = False
            continue
        name = line[:at_idx].strip()
        if "{" in name or "}" in name:
            next_is_delta = False
            continue
        colon_idx = line[at_idx:].find(":")
        if colon_idx == -1:
            next_is_delta = False
            continue
        type_str = line[at_idx + colon_idx + 1 :].strip().rstrip(" }")
        fields.append({"name": name, "type": type_str, "is_delta": next_is_delta})
        next_is_delta = False
    return fields


class TestComplianceDelta:
    vectors = load_vectors("delta.json")

    @pytest.mark.parametrize("vec", vectors, ids=[v["name"] for v in vectors])
    def test_delta_frames(self, vec) -> None:
        frames = vec["frames"]
        fields = parse_delta_fields(vec["schema"])
        prev_values: dict[str, int] = {f["name"]: 0 for f in fields if f["is_delta"]}

        for frame in frames:
            if frame.get("reset"):
                for k in prev_values:
                    prev_values[k] = 0
                continue

            w = BitWriter()
            for f in fields:
                fname = f["name"]
                if fname not in frame["value"]:
                    continue
                if f["is_delta"]:
                    current_val = int(frame["value"][fname])
                    delta = current_val - prev_values[fname]
                    prev_values[fname] = current_val
                    if f["type"] == "u32":
                        w.write_u32(delta)
                    elif f["type"] == "u64":
                        w.write_u64(delta)
                    elif f["type"] == "i32":
                        w.write_i32(delta)
                    elif f["type"] == "i64":
                        w.write_i64(delta)
                else:
                    encode_field(w, vec["schema"], fname, frame["value"][fname])

            got = w.finish()
            want = hex_to_bytes(frame["expected_bytes"])
            assert got == want, f"got {got.hex()}, want {want.hex()}"
