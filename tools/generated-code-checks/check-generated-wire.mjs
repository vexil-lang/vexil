/*
 * Generated Go/Python wire contract matrix.
 *
 * This deliberately runs generated source, rather than the hand-written
 * runtimes alone: each case writes a fresh schema from compliance/vectors,
 * invokes vexilc, then proves encode bytes, decode of the authoritative bytes,
 * and round-trip identity in both target languages.
 */
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const toolDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(toolDir, "../..");
const vectorsDir = join(repoRoot, "compliance", "vectors");
const runtimeGo = resolve(repoRoot, "packages/runtime-go").replaceAll("\\", "/");
const runtimePy = resolve(repoRoot, "packages/runtime-py");
const vexilc = join(repoRoot, "target", "debug", process.platform === "win32" ? "vexilc.exe" : "vexilc");
const python = process.env.PYTHON ?? (process.platform === "win32" ? "python" : "python3");

function run(command, args, cwd, label, env = process.env) {
  const result = spawnSync(command, args, { cwd, encoding: "utf8", env });
  if (result.error) {
    throw new Error(`${label}: could not start ${command}: ${result.error.message}`);
  }
  if (result.status !== 0) {
    throw new Error(`${label} failed (exit ${result.status})\n${result.stdout}${result.stderr}`);
  }
  return result;
}

function requireTool(command, label, args = ["--version"]) {
  try {
    run(command, args, repoRoot, label);
  } catch (error) {
    throw new Error(`${label} is required for generated wire conformance; it must not be skipped.\n${error.message}`);
  }
}

function vector(file, name) {
  const values = JSON.parse(readFileSync(join(vectorsDir, file), "utf8"));
  const found = values.find((item) => item.name === name);
  if (!found) throw new Error(`missing authoritative vector ${file}:${name}`);
  return found;
}

function hexBytes(hex) {
  return hex.match(/../g).map((byte) => `0x${byte}`).join(", ");
}

function namespacePackage(source) {
  const match = source.match(/^namespace\s+([\w.]+)/m);
  if (!match) throw new Error("generated wire vector does not declare a namespace");
  return match[1].split(".").at(-1);
}

function writeGoHarness(dir, source, expected, value, valueCheck = "reflect.DeepEqual(decoded, value)", scenario = "unnamed") {
  writeFileSync(join(dir, "schema.vexil"), source);
  run(vexilc, ["codegen", "schema.vexil", "--output", "generated.go", "--target", "go"], dir, `${scenario}: generate Go contract source`);
  const pkg = namespacePackage(source);
  writeFileSync(join(dir, "go.mod"), `module vexil-generated-wire\n\ngo 1.22\n\nrequire github.com/vexil-lang/vexil/packages/runtime-go v0.0.0\n\nreplace github.com/vexil-lang/vexil/packages/runtime-go => ${runtimeGo}\n`);
  writeFileSync(join(dir, "generated_test.go"), `package ${pkg}

import (
  "bytes"
  "reflect"
  "testing"
  vexil "github.com/vexil-lang/vexil/packages/runtime-go"
)

var _ = reflect.DeepEqual

func TestGeneratedWireContract(t *testing.T) {
  want := []byte{${hexBytes(expected)}}
  value := ${value}
  writer := vexil.NewBitWriter()
  if err := value.Pack(writer); err != nil { t.Fatal(err) }
  if got := writer.Finish(); !bytes.Equal(got, want) { t.Fatalf("${scenario} encode: got %x want %x", got, want) }
  decoded := &M{}
  if err := decoded.Unpack(vexil.NewBitReader(want)); err != nil { t.Fatalf("${scenario} decode: %v", err) }
  if !(${valueCheck}) { t.Fatalf("${scenario} decode: got %#v want %#v", decoded, value) }
  round := vexil.NewBitWriter()
  if err := decoded.Pack(round); err != nil { t.Fatal(err) }
  if got := round.Finish(); !bytes.Equal(got, want) { t.Fatalf("${scenario} roundtrip: got %x want %x", got, want) }
}
`);
  run("go", ["test", "./..."], dir, `${scenario}: Go generated wire contract`);
}

function writePythonHarness(dir, source, expected, value, check = "decoded == value", scenario = "unnamed") {
  writeFileSync(join(dir, "schema.vexil"), source);
  run(vexilc, ["codegen", "schema.vexil", "--output", "generated.py", "--target", "python"], dir, `${scenario}: generate Python contract source`);
  writeFileSync(join(dir, "run.py"), `from generated import *

want = bytes.fromhex("${expected}")
value = ${value}
got = value.encode()
assert got == want, f"${scenario} encode: got {got.hex()} want {want.hex()}"
decoded = M.decode(want)
assert ${check}, f"${scenario} decode: got {decoded!r} want {value!r}"
round = decoded.encode()
assert round == want, f"${scenario} roundtrip: got {round.hex()} want {want.hex()}"
`);
  run(python, ["run.py"], dir, `${scenario}: Python generated wire contract`, { ...process.env, PYTHONPATH: `${runtimePy}${process.platform === "win32" ? ";" : ":"}${dir}` });
}

function runBasic(label, file, name, goValue, pyValue, goCheck, pyCheck) {
  const item = vector(file, name);
  const root = mkdtempSync(join(tmpdir(), `vexil-wire-${label}-`));
  try {
    const go = join(root, "go");
    const py = join(root, "python");
    // Child directories are created by schema writes through the explicit mkdir
    // below, keeping all generated artifacts inside the disposable root.
    for (const dir of [go, py]) {
      mkdirSync(dir, { recursive: true });
    }
    writeGoHarness(go, item.schema, item.expected_bytes, goValue, goCheck, label);
    writePythonHarness(py, item.schema, item.expected_bytes, pyValue, pyCheck, label);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

function runDeltaReset() {
  const item = vector("delta.json", "delta_reset");
  const goSteps = item.frames.map((frame) => {
    if (frame.reset) return "encoder.Reset(); decoder.Reset()";
    return `{ value := uint32(${frame.value.v}); want := []byte{${hexBytes(frame.expected_bytes)}}; w:=vexil.NewBitWriter(); if err:=encoder.Pack(&M{V:value},w); err != nil { t.Fatal(err) }; got:=w.Finish(); if !bytes.Equal(got,want) {t.Fatalf("encode got %x want %x",got,want)}; decoded,err:=decoder.Unpack(vexil.NewBitReader(want)); if err != nil || decoded.V != value {t.Fatalf("decode got %#v err %v",decoded,err)} }`;
  }).join("\n  ");
  const pythonFrames = JSON.stringify(item.frames);
  const root = mkdtempSync(join(tmpdir(), "vexil-wire-delta-"));
  try {
    const go = join(root, "go"); const py = join(root, "python");
    mkdirSync(go, { recursive: true });
    mkdirSync(py, { recursive: true });
    writeFileSync(join(go, "schema.vexil"), item.schema);
    run(vexilc, ["codegen", "schema.vexil", "--output", "generated.go", "--target", "go"], go, "generate Go delta source");
    writeFileSync(join(go, "go.mod"), `module vexil-generated-delta\n\ngo 1.22\n\nrequire github.com/vexil-lang/vexil/packages/runtime-go v0.0.0\n\nreplace github.com/vexil-lang/vexil/packages/runtime-go => ${runtimeGo}\n`);
    writeFileSync(join(go, "generated_test.go"), `package delta
import ("bytes"; "testing"; vexil "github.com/vexil-lang/vexil/packages/runtime-go")
func TestGeneratedDeltaContract(t *testing.T) {
  encoder := NewMEncoder(); decoder := NewMDecoder()
  ${goSteps}
}
`);

    run("go", ["test", "./..."], go, "Go generated delta contract");
    writeFileSync(join(py, "schema.vexil"), item.schema);
    run(vexilc, ["codegen", "schema.vexil", "--output", "generated.py", "--target", "python"], py, "generate Python delta source");
    writeFileSync(join(py, "run.py"), `import json
from generated import M, MEncoder, MDecoder
frames = json.loads(r'''${pythonFrames}''')
encoder = MEncoder(); decoder = MDecoder()
for frame in frames:
    if frame.get("reset"):
        encoder.reset(); decoder.reset(); continue
    value = frame["value"]["v"]; expected = frame["expected_bytes"]
    data = encoder.encode(M(v=value)); assert data.hex() == expected, f"encode {data.hex()} != {expected}"
    decoded = decoder.decode(bytes.fromhex(expected)); assert decoded.v == value, f"decode {decoded.v} != {value}"
`);
    run(python, ["run.py"], py, "Python generated delta contract", { ...process.env, PYTHONPATH: `${runtimePy}${process.platform === "win32" ? ";" : ":"}${py}` });
  } finally { rmSync(root, { recursive: true, force: true }); }
}

function runOptionalEvolution() {
  const v1ToV2 = vector(
    "evolution.json",
    "v1_encode_v2_decode_appended_optional_field",
  );
  const v2ToV1 = vector(
    "evolution.json",
    "v2_encode_v1_decode_appended_optional_field",
  );
  const encodedV1 = v1ToV2.encoded_v1;
  const encodedV2 = v2ToV1.encoded_v2;
  const root = mkdtempSync(join(tmpdir(), "vexil-wire-evolution-"));
  try {
    for (const target of ["go", "python"]) {
      for (const version of ["v1", "v2"]) {
        const dir = join(root, target, version);
        mkdirSync(dir, { recursive: true });
        const source = version === "v1" ? v1ToV2.schema_v1 : v1ToV2.schema_v2;
        writeFileSync(join(dir, "schema.vexil"), source);
        const extension = target === "go" ? "go" : "py";
        run(
          vexilc,
          [
            "codegen",
            "schema.vexil",
            "--output",
            `generated.${extension}`,
            "--target",
            target,
          ],
          dir,
          `optional-evolution ${target} ${version}: generate`,
        );
        if (target === "go") {
          writeFileSync(join(dir, "go.mod"), `module vexil-generated-evolution-${version}\n\ngo 1.22\n\nrequire github.com/vexil-lang/vexil/packages/runtime-go v0.0.0\n\nreplace github.com/vexil-lang/vexil/packages/runtime-go => ${runtimeGo}\n`);
          const pkg = namespacePackage(source);
          const body =
            version === "v1"
              ? `value:=&M{X:42}; writer:=vexil.NewBitWriter(); if err:=value.Pack(writer); err != nil {t.Fatal(err)}; if got:=writer.Finish(); !bytes.Equal(got, []byte{${hexBytes(encodedV1)}}) {t.Fatalf("optional-evolution v1 encode got %x",got)}; decoded:=&M{}; if err:=decoded.Unpack(vexil.NewBitReader([]byte{${hexBytes(encodedV2)}})); err != nil || decoded.X != 42 {t.Fatalf("optional-evolution v2-to-v1 decode: %#v %v",decoded,err)}`
              : `present:=uint16(99); value:=&M{X:42,Y:&present}; writer:=vexil.NewBitWriter(); if err:=value.Pack(writer); err != nil {t.Fatal(err)}; if got:=writer.Finish(); !bytes.Equal(got, []byte{${hexBytes(encodedV2)}}) {t.Fatalf("optional-evolution v2 encode got %x",got)}; decoded:=&M{}; if err:=decoded.Unpack(vexil.NewBitReader([]byte{${hexBytes(encodedV1)}})); err != nil || decoded.X != 42 || decoded.Y != nil {t.Fatalf("optional-evolution v1-to-v2 decode: %#v %v",decoded,err)}`;
          writeFileSync(
            join(dir, "generated_test.go"),
            `package ${pkg}\nimport ("bytes"; "testing"; vexil "github.com/vexil-lang/vexil/packages/runtime-go")\nfunc TestOptionalEvolution(t *testing.T) { ${body} }\n`,
          );
          run("go", ["test", "./..."], dir, `optional-evolution ${version}: Go contract`);
        } else {
          const body =
            version === "v1"
              ? `value=M(x=42); got=value.encode(); assert got.hex() == '${encodedV1}', f'optional-evolution v1 encode {got.hex()}'; decoded=M.decode(bytes.fromhex('${encodedV2}')); assert decoded.x == 42, f'optional-evolution v2-to-v1 decode {decoded!r}'`
              : `value=M(x=42,y=99); got=value.encode(); assert got.hex() == '${encodedV2}', f'optional-evolution v2 encode {got.hex()}'; decoded=M.decode(bytes.fromhex('${encodedV1}')); assert decoded.x == 42 and decoded.y is None, f'optional-evolution v1-to-v2 decode {decoded!r}'`;
          writeFileSync(join(dir, "run.py"), `from generated import M\n${body}\n`);
          run(python, ["run.py"], dir, `optional-evolution ${version}: Python contract`, {
            ...process.env,
            PYTHONPATH: `${runtimePy}${process.platform === "win32" ? ";" : ":"}${dir}`,
          });
        }
      }
    }
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

function runTraitInvariance() {
  const root = mkdtempSync(join(tmpdir(), "vexil-wire-trait-"));
  const schemas = {
    plain: "namespace test.traitplain\nmessage Counter { value @0 : u32 }\n",
    trait: "namespace test.traitimpl\ntrait Adjustable { value @9 : u32 fn adjust(delta: u32) -> u32 }\nmessage Counter { value @0 : u32 }\nimpl Adjustable for Counter { fn adjust(delta: u32) -> u32 { let previous: u32 = self.value self.value = self.value + delta return previous } }\n",
  };
  try {
    for (const [name, source] of Object.entries(schemas)) {
      const dir = join(root, "go", name);
      mkdirSync(dir, { recursive: true });
      writeFileSync(join(dir, "schema.vexil"), source);
      run(vexilc, ["codegen", "schema.vexil", "--output", "generated.go", "--target", "go"], dir, "generate Go trait invariance source");
      writeFileSync(join(dir, "go.mod"), `module vexil-generated-trait-${name}\n\ngo 1.22\n\nrequire github.com/vexil-lang/vexil/packages/runtime-go v0.0.0\n\nreplace github.com/vexil-lang/vexil/packages/runtime-go => ${runtimeGo}\n`);
      const pkg = namespacePackage(source);
      const behavior = name === "plain"
        ? "counter := &Counter{Value: 8}"
        : "counter := &Counter{Value: 5}; if got := counter.Adjust(3); got != 5 || counter.Value != 8 { t.Fatalf(\"method state: got %d value %d\", got, counter.Value) }";
      writeFileSync(join(dir, "generated_test.go"), `package ${pkg}\nimport (\"bytes\"; \"testing\"; vexil \"github.com/vexil-lang/vexil/packages/runtime-go\")\nfunc TestTraitWireInvariance(t *testing.T) { ${behavior}; w:=vexil.NewBitWriter(); if err:=counter.Pack(w); err != nil {t.Fatal(err)}; if got:=w.Finish(); !bytes.Equal(got, []byte{8,0,0,0}) {t.Fatalf(\"wire got %x\",got)} }\n`);
      run("go", ["test", "./..."], dir, "Go trait wire invariance");
    }
    const dir = join(root, "python");
    mkdirSync(dir, { recursive: true });
    for (const [name, source] of Object.entries(schemas)) {
      writeFileSync(join(dir, `${name}.vexil`), source);
      run(vexilc, ["codegen", `${name}.vexil`, "--output", `${name}.py`, "--target", "python"], dir, "generate Python trait invariance source");
    }
    writeFileSync(join(dir, "run.py"), `import importlib.util, sys\ndef load(name):\n spec=importlib.util.spec_from_file_location(name, name + '.py'); mod=importlib.util.module_from_spec(spec); sys.modules[name] = mod; spec.loader.exec_module(mod); return mod\nplain=load('plain'); trait=load('trait')\np=plain.Counter(value=8).encode(); c=trait.Counter(value=5); assert c.adjust(3) == 5; t=c.encode(); assert p == t == bytes.fromhex('08000000'), f'{p.hex()} {t.hex()}'\n`);
    run(python, ["run.py"], dir, "Python trait wire invariance", { ...process.env, PYTHONPATH: `${runtimePy}${process.platform === "win32" ? ";" : ":"}${dir}` });
  } finally { rmSync(root, { recursive: true, force: true }); }
}

requireTool("go", "Go", ["version"]);
requireTool(python, "Python");
if (!existsSync(join(repoRoot, "Cargo.toml"))) throw new Error("repository root not found");
run("cargo", ["build", "-p", "vexilc"], repoRoot, "build vexilc");

// The selected vectors collectively cover primitives/LSB packing, aggregates,
// optionals, collections, encoding annotations, and the stateful delta reset.
runBasic("primitives", "messages.json", "mixed_bool_u16_string", "&M{Flag: true, Count: 42, Name: \"test\"}", "M(flag=True, count=42, name='test')");
runBasic("signed-primitive", "primitives.json", "i32_negative", "&M{V: -1}", "M(v=-1)");
runBasic("finite-float", "primitives.json", "f32_finite_one_point_five", "&M{V: 1.5}", "M(v=1.5)");
runBasic("bytes", "primitives.json", "bytes_three", "&M{V: []byte{0xde, 0xad, 0xbe}}", "M(v=bytes([0xde, 0xad, 0xbe]))");
runBasic("subbyte", "sub_byte.json", "u3_u5_packed", "&M{A: 5, B: 18}", "M(a=5, b=18)");
runBasic("nested-message", "messages.json", "nested_message_u16", "&M{Child: Child{Value: 7}}", "M(child=Child(value=7))");
runBasic("enum", "enums.json", "enum_second_variant", "&M{V: StatusInactive}", "M(v=Status(Status.INACTIVE))");
runBasic("union", "unions.json", "union_first_variant", "&M{V: &ShapeCircle{Radius: 1.5}}", "M(v=ShapeCircle(1.5))", "func() bool { circle, ok := decoded.V.(*ShapeCircle); return ok && circle.Radius == 1.5 }()", "type(decoded.v) is ShapeCircle and decoded.v.radius == value.v.radius");
{ const present = "uint32(42)"; runBasic("optional", "optionals.json", "optional_some_u32", `func() *M { v := ${present}; return &M{V: &v} }()`, "M(v=42)"); }
runBasic("optional-absent", "optionals.json", "optional_none", "&M{V: nil}", "M(v=None)");
runBasic("array", "arrays_maps.json", "array_three_u32", "&M{V: []uint32{1, 2, 3}}", "M(v=[1, 2, 3])");
runBasic("map", "arrays_maps.json", "map_two_string_entries_canonical_order", "&M{V: map[string]uint32{\"z\": 2, \"a\": 1}}", "M(v={'z': 2, 'a': 1})");
runBasic("set", "v1_types.json", "set_strings", "&M{Tags: map[string]struct{}{\"beta\": {}, \"alpha\": {}}}", "M(tags={'beta', 'alpha'})");
runBasic("fixed-array", "v1_types.json", "fixed_array_u8", "&M{Data: [4]uint8{1,2,3,4}}", "M(data=(1,2,3,4))");
runBasic("annotations", "annotations.json", "varint_and_zigzag", "&M{Count: 300, Delta: -5}", "M(count=300, delta=-5)");
runDeltaReset();
runOptionalEvolution();
runTraitInvariance();
process.stdout.write("Generated Go/Python wire contract matrix passed.\n");
