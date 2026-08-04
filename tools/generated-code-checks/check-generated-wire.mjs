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

function validateCoverageTable() {
  const coverage = JSON.parse(readFileSync(join(toolDir, "wire-coverage.json"), "utf8"));
  if (!Array.isArray(coverage) || coverage.length === 0) {
    throw new Error("generated wire coverage table must contain scenarios");
  }
  for (const item of coverage) {
    if (!item.scenario || !item.authority || !["covered", "blocked"].includes(item.status)) {
      throw new Error(`invalid generated wire coverage row: ${JSON.stringify(item)}`);
    }
    if (item.status === "blocked" && !item.reason) {
      throw new Error(`blocked generated wire coverage row needs a reason: ${item.scenario}`);
    }
  }
}

function hexBytes(hex) {
  return hex.match(/../g).map((byte) => `0x${byte}`).join(", ");
}

function namespacePackage(source) {
  const match = source.match(/^namespace\s+([\w.]+)/m);
  if (!match) throw new Error("generated wire vector does not declare a namespace");
  return match[1].split(".").at(-1);
}

function writeGoHarness(dir, source, expected, value, valueCheck = "reflect.DeepEqual(decoded, value)", scenario = "unnamed", typeName = "M") {
  writeFileSync(join(dir, "schema.vexil"), source);
  run(vexilc, ["codegen", "schema.vexil", "--output", "generated.go", "--target", "go"], dir, `${scenario}: generate Go contract source`);
  const pkg = namespacePackage(source);
  writeFileSync(join(dir, "go.mod"), `module vexil-generated-wire\n\ngo 1.22\n\nrequire github.com/vexil-lang/vexil/packages/runtime-go v0.0.0\n\nreplace github.com/vexil-lang/vexil/packages/runtime-go => ${runtimeGo}\n`);
  writeFileSync(join(dir, "generated_test.go"), `package ${pkg}

import (
  "bytes"
  "math"
  "reflect"
  "testing"
  vexil "github.com/vexil-lang/vexil/packages/runtime-go"
)

var _ = reflect.DeepEqual
var _ = math.IsNaN

func TestGeneratedWireContract(t *testing.T) {
  want := []byte{${hexBytes(expected)}}
  value := ${value}
  writer := vexil.NewBitWriter()
  if err := value.Pack(writer); err != nil { t.Fatal(err) }
  if got := writer.Finish(); !bytes.Equal(got, want) { t.Fatalf("${scenario} encode: got %x want %x", got, want) }
  decoded := &${typeName}{}
  if err := decoded.Unpack(vexil.NewBitReader(want)); err != nil { t.Fatalf("${scenario} decode: %v", err) }
  if !(${valueCheck}) { t.Fatalf("${scenario} decode: got %#v want %#v", decoded, value) }
  round := vexil.NewBitWriter()
  if err := decoded.Pack(round); err != nil { t.Fatal(err) }
  if got := round.Finish(); !bytes.Equal(got, want) { t.Fatalf("${scenario} roundtrip: got %x want %x", got, want) }
}
`);
  run("go", ["test", "./..."], dir, `${scenario}: Go generated wire contract`);
}

function writePythonHarness(dir, source, expected, value, check = "decoded == value", scenario = "unnamed", typeName = "M") {
  writeFileSync(join(dir, "schema.vexil"), source);
  run(vexilc, ["codegen", "schema.vexil", "--output", "generated.py", "--target", "python"], dir, `${scenario}: generate Python contract source`);
  writeFileSync(join(dir, "run.py"), `from generated import *

want = bytes.fromhex("${expected}")
value = ${value}
got = value.encode()
assert got == want, f"${scenario} encode: got {got.hex()} want {want.hex()}"
decoded = ${typeName}.decode(want)
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
    writeGoHarness(go, item.schema, item.expected_bytes, goValue, goCheck, label, item.type);
    writePythonHarness(py, item.schema, item.expected_bytes, pyValue, pyCheck, label, item.type);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

function goFieldName(name) {
  return name.split("_").map((part) => part[0].toUpperCase() + part.slice(1)).join("");
}

function goLiteral(value) {
  return typeof value === "string" ? JSON.stringify(value) : String(value);
}

function runDeltaVectors() {
  const items = JSON.parse(readFileSync(join(vectorsDir, "delta.json"), "utf8"));
  const root = mkdtempSync(join(tmpdir(), "vexil-wire-delta-"));
  try {
    for (const item of items) {
      const goSteps = item.frames.map((frame, index) => {
        if (frame.reset) return "encoder.Reset(); roundEncoder.Reset(); decoder.Reset()";
        const fields = Object.entries(frame.value)
          .map(([name, value]) => `${goFieldName(name)}: ${goLiteral(value)}`)
          .join(", ");
        const mismatches = Object.entries(frame.value)
          .map(([name, value]) => `decoded.${goFieldName(name)} != ${goLiteral(value)}`)
          .join(" || ");
        return `{ value := &M{${fields}}; want := []byte{${hexBytes(frame.expected_bytes)}}; w:=vexil.NewBitWriter(); if err:=encoder.Pack(value,w); err != nil { t.Fatal(err) }; got:=w.Finish(); if !bytes.Equal(got,want) {t.Fatalf("${item.name} frame ${index} encode got %x want %x",got,want)}; decoded,err:=decoder.Unpack(vexil.NewBitReader(want)); if err != nil || ${mismatches} {t.Fatalf("${item.name} frame ${index} decode got %#v err %v",decoded,err)}; round:=vexil.NewBitWriter(); if err:=roundEncoder.Pack(decoded,round); err != nil {t.Fatal(err)}; if got:=round.Finish(); !bytes.Equal(got,want) {t.Fatalf("${item.name} frame ${index} re-encode got %x want %x",got,want)} }`;
      }).join("\n  ");
      const pythonFrames = JSON.stringify(item.frames);
      const go = join(root, "go", item.name);
      const py = join(root, "python", item.name);
      mkdirSync(go, { recursive: true });
      mkdirSync(py, { recursive: true });
      writeFileSync(join(go, "schema.vexil"), item.schema);
      run(vexilc, ["codegen", "schema.vexil", "--output", "generated.go", "--target", "go"], go, `${item.name}: generate Go delta source`);
      writeFileSync(join(go, "go.mod"), `module vexil-generated-delta-${item.name.replaceAll("_", "-")}\n\ngo 1.22\n\nrequire github.com/vexil-lang/vexil/packages/runtime-go v0.0.0\n\nreplace github.com/vexil-lang/vexil/packages/runtime-go => ${runtimeGo}\n`);
      writeFileSync(join(go, "generated_test.go"), `package ${namespacePackage(item.schema)}
import ("bytes"; "testing"; vexil "github.com/vexil-lang/vexil/packages/runtime-go")
func TestGeneratedDeltaContract(t *testing.T) {
  encoder := NewMEncoder(); roundEncoder := NewMEncoder(); decoder := NewMDecoder()
  ${goSteps}
}
`);
      run("go", ["test", "./..."], go, `${item.name}: Go generated delta contract`);

      writeFileSync(join(py, "schema.vexil"), item.schema);
      run(vexilc, ["codegen", "schema.vexil", "--output", "generated.py", "--target", "python"], py, `${item.name}: generate Python delta source`);
      writeFileSync(join(py, "run.py"), `import json
from generated import M, MEncoder, MDecoder
frames = json.loads(r'''${pythonFrames}''')
encoder = MEncoder(); round_encoder = MEncoder(); decoder = MDecoder()
for index, frame in enumerate(frames):
    if frame.get("reset"):
        encoder.reset(); round_encoder.reset(); decoder.reset(); continue
    value = M(**frame["value"]); expected = bytes.fromhex(frame["expected_bytes"])
    data = encoder.encode(value); assert data == expected, f"${item.name} frame {index} encode {data.hex()} != {expected.hex()}"
    decoded = decoder.decode(expected)
    actual = {name: getattr(decoded, name) for name in frame["value"]}
    assert actual == frame["value"], f"${item.name} frame {index} decode {actual!r} != {frame['value']!r}"
    round_data = round_encoder.encode(decoded)
    assert round_data == expected, f"${item.name} frame {index} re-encode {round_data.hex()} != {expected.hex()}"
`);
      run(python, ["run.py"], py, `${item.name}: Python generated delta contract`, { ...process.env, PYTHONPATH: `${runtimePy}${process.platform === "win32" ? ";" : ":"}${py}` });
    }
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
              : `present:=uint16(99); value:=&M{X:42,Y:&present}; writer:=vexil.NewBitWriter(); if err:=value.Pack(writer); err != nil {t.Fatal(err)}; if got:=writer.Finish(); !bytes.Equal(got, []byte{${hexBytes(encodedV2)}}) {t.Fatalf("optional-evolution v2 encode got %x",got)}; decoded:=&M{}; if err:=decoded.Unpack(vexil.NewBitReader([]byte{${hexBytes(encodedV1)}})); err != vexil.ErrUnexpectedEOF {t.Fatalf("optional-evolution v1-to-v2 decode: got %v, want %v",err,vexil.ErrUnexpectedEOF)}`;
          writeFileSync(
            join(dir, "generated_test.go"),
            `package ${pkg}\nimport ("bytes"; "testing"; vexil "github.com/vexil-lang/vexil/packages/runtime-go")\nfunc TestOptionalEvolution(t *testing.T) { ${body} }\n`,
          );
          run("go", ["test", "./..."], dir, `optional-evolution ${version}: Go contract`);
        } else {
          const body =
            version === "v1"
              ? `value=M(x=42); got=value.encode(); assert got.hex() == '${encodedV1}', f'optional-evolution v1 encode {got.hex()}'; decoded=M.decode(bytes.fromhex('${encodedV2}')); assert decoded.x == 42, f'optional-evolution v2-to-v1 decode {decoded!r}'`
              : `value=M(x=42,y=99); got=value.encode(); assert got.hex() == '${encodedV2}', f'optional-evolution v2 encode {got.hex()}';\ntry:\n    M.decode(bytes.fromhex('${encodedV1}'))\nexcept DecodeError:\n    pass\nelse:\n    raise AssertionError('optional-evolution v1-to-v2 decode accepted missing optional field')`;
          writeFileSync(join(dir, "run.py"), `from generated import M\nfrom vexil_runtime import DecodeError\n${body}\n`);
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

function runNestedOptionalConstraint() {
  const item = vector("generated_wire.json", "nested_optional_constrained_u16");
  const root = mkdtempSync(join(tmpdir(), "vexil-wire-nested-constraint-"));
  try {
    const go = join(root, "go");
    const py = join(root, "python");
    mkdirSync(go, { recursive: true });
    mkdirSync(py, { recursive: true });

    writeFileSync(join(go, "schema.vexil"), item.schema);
    run(vexilc, ["codegen", "schema.vexil", "--output", "generated.go", "--target", "go"], go, "nested-optional-constraint: generate Go source");
    writeFileSync(join(go, "go.mod"), `module vexil-generated-nested-constraint\n\ngo 1.22\n\nrequire github.com/vexil-lang/vexil/packages/runtime-go v0.0.0\n\nreplace github.com/vexil-lang/vexil/packages/runtime-go => ${runtimeGo}\n`);
    writeFileSync(join(go, "generated_test.go"), `package ${namespacePackage(item.schema)}

import (
  "bytes"
  "testing"
  vexil "github.com/vexil-lang/vexil/packages/runtime-go"
)

func nested(value uint16) **uint16 { inner := &value; return &inner }
func nestedNone() **uint16 { var inner *uint16; return &inner }
func encodeNested(value **uint16) ([]byte, error) {
  writer := vexil.NewBitWriter()
  if err := (&M{V: value}).Pack(writer); err != nil { return nil, err }
  return writer.Finish(), nil
}

func TestNestedOptionalConstraint(t *testing.T) {
  want := []byte{${hexBytes(item.expected_bytes)}}
  got, err := encodeNested(nested(258))
  if err != nil || !bytes.Equal(got, want) { t.Fatalf("nested constraint valid encode: got %x err %v want %x", got, err, want) }
  decoded := &M{}
  if err := decoded.Unpack(vexil.NewBitReader(want)); err != nil || decoded.V == nil || *decoded.V == nil || **decoded.V != 258 {
    t.Fatalf("nested constraint valid decode: got %#v err %v", decoded.V, err)
  }
  if got, err := encodeNested(nil); err != nil || !bytes.Equal(got, []byte{0}) { t.Fatalf("nested constraint outer none: got %x err %v", got, err) }
  if got, err := encodeNested(nestedNone()); err != nil || !bytes.Equal(got, []byte{1}) { t.Fatalf("nested constraint inner none: got %x err %v", got, err) }
  if _, err := encodeNested(nested(0)); err == nil { t.Fatal("nested constraint invalid encode: expected constraint error") }
  if err := (&M{}).Unpack(vexil.NewBitReader([]byte{3, 0, 0})); err == nil { t.Fatal("nested constraint invalid decode: expected constraint error") }
  outerNone := &M{}
  if err := outerNone.Unpack(vexil.NewBitReader([]byte{0})); err != nil || outerNone.V != nil { t.Fatalf("nested constraint outer-none decode: %#v %v", outerNone.V, err) }
  innerNone := &M{}
  if err := innerNone.Unpack(vexil.NewBitReader([]byte{1})); err != nil || innerNone.V == nil || *innerNone.V != nil { t.Fatalf("nested constraint inner-none decode: %#v %v", innerNone.V, err) }
}
`);
    run("go", ["test", "./..."], go, "nested-optional-constraint: Go generated contract");

    writeFileSync(join(py, "schema.vexil"), item.schema);
    run(vexilc, ["codegen", "schema.vexil", "--output", "generated.py", "--target", "python"], py, "nested-optional-constraint: generate Python source");
    writeFileSync(join(py, "run.py"), `from generated import M

want = bytes.fromhex("${item.expected_bytes}")
value = M(v=(258,))
assert value.encode() == want, f"nested constraint valid encode: {value.encode().hex()} != {want.hex()}"
assert M.decode(want) == value, f"nested constraint valid decode: {M.decode(want)!r} != {value!r}"
assert M(v=None).encode() == bytes([0]), "nested constraint outer none"
assert M(v=(None,)).encode() == bytes([1]), "nested constraint inner none"
assert M.decode(bytes([0])).v is None, "nested constraint outer-none decode"
assert M.decode(bytes([1])).v == (None,), "nested constraint inner-none decode"
try:
    M(v=(0,)).encode()
except ValueError:
    pass
else:
    raise AssertionError("nested constraint invalid encode: expected constraint error")
try:
    M.decode(bytes([3, 0, 0]))
except ValueError:
    pass
else:
    raise AssertionError("nested constraint invalid decode: expected constraint error")
`);
    run(python, ["run.py"], py, "nested-optional-constraint: Python generated contract", {
      ...process.env,
      PYTHONPATH: `${runtimePy}${process.platform === "win32" ? ";" : ":"}${py}`,
    });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

function runFailurePaths() {
  const root = mkdtempSync(join(tmpdir(), "vexil-wire-failures-"));
  const cases = {
    truncated: "namespace test.truncated\nmessage M { value @0 : u32 }\n",
    recursion: "namespace test.recursion_limit\nmessage Node { next @0 : optional<Node> }\n",
  };
  try {
    for (const [name, source] of Object.entries(cases)) {
      const go = join(root, "go", name);
      const py = join(root, "python", name);
      mkdirSync(go, { recursive: true });
      mkdirSync(py, { recursive: true });
      writeFileSync(join(go, "schema.vexil"), source);
      writeFileSync(join(py, "schema.vexil"), source);
      run(vexilc, ["codegen", "schema.vexil", "--output", "generated.go", "--target", "go"], go, `${name}: generate Go source`);
      run(vexilc, ["codegen", "schema.vexil", "--output", "generated.py", "--target", "python"], py, `${name}: generate Python source`);
      writeFileSync(join(go, "go.mod"), `module vexil-generated-${name}\n\ngo 1.22\n\nrequire github.com/vexil-lang/vexil/packages/runtime-go v0.0.0\n\nreplace github.com/vexil-lang/vexil/packages/runtime-go => ${runtimeGo}\n`);
      if (name === "truncated") {
        writeFileSync(join(go, "generated_test.go"), `package truncated
import ("testing"; vexil "github.com/vexil-lang/vexil/packages/runtime-go")
func TestTruncatedGeneratedDecode(t *testing.T) { if err := (&M{}).Unpack(vexil.NewBitReader([]byte{1, 2})); err == nil { t.Fatal("truncated-u32: expected decode error") } }
`);
        writeFileSync(join(py, "run.py"), `from generated import M
from vexil_runtime import DecodeError
try:
    M.decode(bytes([1, 2]))
except DecodeError:
    pass
else:
    raise AssertionError("truncated-u32: expected decode error")
`);
      } else {
        writeFileSync(join(go, "generated_test.go"), `package recursion_limit
import ("bytes"; "testing"; vexil "github.com/vexil-lang/vexil/packages/runtime-go")
func chain(count int) *Node { var next *Node; for i := 0; i < count; i++ { next = &Node{Next: next} }; return next }
func encodeRoot(value *Node) error { w := vexil.NewBitWriter(); if err := w.EnterRecursive(); err != nil { return err }; defer w.LeaveRecursive(); return value.Pack(w) }
func decodeRoot(data []byte) error { r := vexil.NewBitReader(data); if err := r.EnterRecursive(); err != nil { return err }; defer r.LeaveRecursive(); return (&Node{}).Unpack(r) }
func TestGeneratedRecursionLimit(t *testing.T) {
  if err := encodeRoot(chain(64)); err != nil { t.Fatalf("depth-64 encode: %v", err) }
  if err := encodeRoot(chain(65)); err == nil { t.Fatal("depth-65 encode: expected recursion error") }
  if err := decodeRoot(append(bytes.Repeat([]byte{1}, 63), 0)); err != nil { t.Fatalf("depth-64 decode: %v", err) }
  if err := decodeRoot(append(bytes.Repeat([]byte{1}, 64), 0)); err == nil { t.Fatal("depth-65 decode: expected recursion error") }
}
`);
        writeFileSync(join(py, "run.py"), `from generated import Node
def chain(count: int) -> Node:
    value = None
    for _ in range(count): value = Node(next=value)
    assert value is not None
    return value
chain(64).encode()
try:
    chain(65).encode()
except Exception as error:
    assert "nesting exceeded 64" in str(error), f"depth-65 encode: {error}"
else:
    raise AssertionError("depth-65 encode: expected recursion error")
Node.decode(bytes([1] * 63 + [0]))
try:
    Node.decode(bytes([1] * 64 + [0]))
except Exception as error:
    assert "nesting exceeded 64" in str(error), f"depth-65 decode: {error}"
else:
    raise AssertionError("depth-65 decode: expected recursion error")
`);
      }
      run("go", ["test", "./..."], go, `${name}: Go generated failure contract`);
      run(python, ["run.py"], py, `${name}: Python generated failure contract`, {
        ...process.env,
        PYTHONPATH: `${runtimePy}${process.platform === "win32" ? ";" : ":"}${py}`,
      });
    }
  } finally { rmSync(root, { recursive: true, force: true }); }
}

requireTool("go", "Go", ["version"]);
requireTool(python, "Python");
validateCoverageTable();
if (!existsSync(join(repoRoot, "Cargo.toml"))) throw new Error("repository root not found");
run("cargo", ["build", "-p", "vexilc"], repoRoot, "build vexilc");

// The selected vectors collectively cover primitives/LSB packing, aggregates,
// optionals, collections, encoding annotations, and the stateful delta reset.
runBasic("primitives", "messages.json", "mixed_bool_u16_string", "&M{Flag: true, Count: 42, Name: \"test\"}", "M(flag=True, count=42, name='test')");
runBasic("bool-false", "primitives.json", "bool_false", "&M{V: false}", "M(v=False)");
runBasic("bool-true", "primitives.json", "bool_true", "&M{V: true}", "M(v=True)");
runBasic("u8-zero", "primitives.json", "u8_zero", "&M{V: 0}", "M(v=0)");
runBasic("u8-max", "primitives.json", "u8_max", "&M{V: 255}", "M(v=255)");
runBasic("u16-le", "primitives.json", "u16_le", "&M{V: 258}", "M(v=258)");
runBasic("u32-le", "primitives.json", "u32_le", "&M{V: 305419896}", "M(v=305419896)");
runBasic("signed-primitive", "primitives.json", "i32_negative", "&M{V: -1}", "M(v=-1)");
runBasic("nan-canonical", "primitives.json", "f32_nan_canonical", "&M{V: float32(math.NaN())}", "M(v=float('nan'))", "math.IsNaN(float64(decoded.V))", "__import__('math').isnan(decoded.v)");
runBasic("negative-zero", "primitives.json", "f64_negative_zero", "&M{V: math.Copysign(0, -1)}", "M(v=-0.0)", "math.Signbit(decoded.V)", "__import__('math').copysign(1.0, decoded.v) < 0");
runBasic("string-hello", "primitives.json", "string_hello", "&M{V: \"hello\"}", "M(v='hello')");
runBasic("string-empty", "primitives.json", "string_empty", "&M{V: \"\"}", "M(v='')");
runBasic("finite-float", "primitives.json", "f32_finite_one_point_five", "&M{V: 1.5}", "M(v=1.5)");
runBasic("bytes", "primitives.json", "bytes_three", "&M{V: []byte{0xde, 0xad, 0xbe}}", "M(v=bytes([0xde, 0xad, 0xbe]))");
runBasic("subbyte", "sub_byte.json", "u3_u5_packed", "&M{A: 5, B: 18}", "M(a=5, b=18)");
runBasic("subbyte-cross-byte", "sub_byte.json", "u3_u5_u6_cross_byte", "&M{A: 7, B: 31, C: 63}", "M(a=7, b=31, c=63)");
runBasic("subbyte-u1", "sub_byte.json", "u1_one", "&M{V: 1}", "M(v=1)");
runBasic("empty-message", "messages.json", "empty_message", "&Empty{}", "Empty()");
runBasic("two-u32-fields", "messages.json", "two_u32_fields", "&M{X: 1, Y: 2}", "M(x=1, y=2)");
runBasic("nested-message", "messages.json", "nested_message_u16", "&M{Child: Child{Value: 7}}", "M(child=Child(value=7))");
runBasic("enum-first", "enums.json", "enum_first_variant", "&M{V: StatusActive}", "M(v=Status(Status.ACTIVE))");
runBasic("enum", "enums.json", "enum_second_variant", "&M{V: StatusInactive}", "M(v=Status(Status.INACTIVE))");
runBasic("union", "unions.json", "union_first_variant", "&M{V: &ShapeCircle{Radius: 1.5}}", "M(v=ShapeCircle(1.5))", "func() bool { circle, ok := decoded.V.(*ShapeCircle); return ok && circle.Radius == 1.5 }()", "type(decoded.v) is ShapeCircle and decoded.v.radius == value.v.radius");
{ const present = "uint32(42)"; runBasic("optional", "optionals.json", "optional_some_u32", `func() *M { v := ${present}; return &M{V: &v} }()`, "M(v=42)"); }
runBasic("optional-absent", "optionals.json", "optional_none", "&M{V: nil}", "M(v=None)");
runBasic("array-empty", "arrays_maps.json", "array_empty", "&M{V: []uint32{}}", "M(v=[])");
runBasic("array", "arrays_maps.json", "array_three_u32", "&M{V: []uint32{1, 2, 3}}", "M(v=[1, 2, 3])");
runBasic("map-one", "arrays_maps.json", "map_one_entry", "&M{V: map[string]uint32{\"key\": 42}}", "M(v={'key': 42})");
runBasic("map", "arrays_maps.json", "map_two_string_entries_canonical_order", "&M{V: map[string]uint32{\"z\": 2, \"a\": 1}}", "M(v={'z': 2, 'a': 1})");
runBasic("set-empty", "v1_types.json", "set_empty", "&M{Tags: map[string]struct{}{}}", "M(tags=set())");
runBasic("set", "v1_types.json", "set_strings", "&M{Tags: map[string]struct{}{\"beta\": {}, \"alpha\": {}}}", "M(tags={'beta', 'alpha'})");
runBasic("fixed-array", "v1_types.json", "fixed_array_u8", "&M{Data: [4]uint8{1,2,3,4}}", "M(data=(1,2,3,4))");
runBasic("fixed32-zero", "v1_types.json", "fixed32_zero", "&M{V: 0}", "M(v=0)");
runBasic("fixed32-one", "v1_types.json", "fixed32_one", "&M{V: 65536}", "M(v=65536)");
runBasic("fixed64-zero", "v1_types.json", "fixed64_zero", "&M{V: 0}", "M(v=0)");
runBasic("vec3-f32", "v1_types.json", "vec3_f32", "&M{Pos: [3]float32{1,2,3}}", "M(pos=(1.0,2.0,3.0))");
runBasic("vec2-fixed64", "v1_types.json", "vec2_fixed64", "&M{Pos: [2]int64{0,0}}", "M(pos=(0,0))");
runBasic("bits-inline", "v1_types.json", "bits_inline", "&M{Perms: 3}", "M(perms=3)");
runBasic("newtype-u32", "generated_wire.json", "newtype_u32", "&M{V: UserId(16909060)}", "M(v=UserId(16909060))");
runBasic("alias-u16", "generated_wire.json", "alias_u16", "&M{V: 258}", "M(v=258)");
runBasic(
  "nested-optional-some-none-u16",
  "generated_wire.json",
  "nested_optional_some_none_u16",
  "func() *M { var inner *uint16; return &M{V: &inner} }()",
  "M(v=(None,))",
);
runBasic(
  "nested-optional-some-none-u16-tail-bool",
  "generated_wire.json",
  "nested_optional_some_none_u16_tail_bool",
  "func() *M { var inner *uint16; return &M{V: &inner, Tail: true} }()",
  "M(v=(None,), tail=True)",
);
runBasic(
  "nested-optional-some-u16",
  "generated_wire.json",
  "nested_optional_some_u16",
  "func() *M { scalar := uint16(258); inner := &scalar; return &M{V: &inner} }()",
  "M(v=(258,))",
);
runBasic("unknown-enum", "generated_wire.json", "non_exhaustive_enum_unknown", "&M{V: Status(7)}", "M(v=Status(7))");
runBasic("unknown-flags", "generated_wire.json", "flags_unknown_bits", "&M{V: Permissions(128)}", "M(v=Permissions(128))");
runBasic("map-i16-order", "generated_wire.json", "map_i16_canonical_order", "&M{V: map[int16]uint8{2: 22, -1: 11}}", "M(v={2: 22, -1: 11})");
runBasic("set-u16-order", "generated_wire.json", "set_u16_canonical_order", "&M{V: map[uint16]struct{}{2: {}, 1: {}}}", "M(v={2, 1})");
runBasic("nested-optional-map-set", "generated_wire.json", "nested_optional_map_set", "func() *M { value := map[uint8]map[uint16]struct{}{2: {2: {}, 1: {}}}; return &M{V: &value} }()", "M(v={2: {2, 1}})");
runBasic("annotations", "annotations.json", "varint_and_zigzag", "&M{Count: 300, Delta: -5}", "M(count=300, delta=-5)");
runBasic(
  "result-ok-u8",
  "results.json",
  "result_ok_u8",
  "func() *M { value := uint8(42); result := &M{}; result.Value.Ok = &value; return result }()",
  "M(value=(True, 42))",
);
runBasic(
  "result-err-string",
  "results.json",
  "result_err_string",
  "func() *M { value := \"oops\"; result := &M{}; result.Value.Err = &value; return result }()",
  "M(value=(False, 'oops'))",
);
runBasic(
  "result-ok-void",
  "results.json",
  "result_ok_void",
  "func() *M { value := struct{}{}; result := &M{}; result.Value.Ok = &value; return result }()",
  "M(value=(True, None))",
);
runBasic(
  "result-packed-bool-adjacency",
  "results.json",
  "result_packed_bool_adjacency",
  "func() *M { value := true; result := &M{Tail: true}; result.Value.Ok = &value; return result }()",
  "M(value=(True, True), tail=True)",
);
runBasic(
  "unknown-union-roundtrip",
  "unions.json",
  "non_exhaustive_union_unknown_roundtrip",
  "&M{V: &Event__VexilUnknown{Discriminant: 9, Data: []byte{0xde, 0xad}}}",
  "M(v=Event__VexilUnknown(9, bytes([0xde, 0xad])))",
  "func() bool { unknown, ok := decoded.V.(*Event__VexilUnknown); return ok && unknown.Discriminant == 9 && bytes.Equal(unknown.Data, []byte{0xde, 0xad}) }()",
  "type(decoded.v) is Event__VexilUnknown and decoded.v.discriminant == 9 and decoded.v.data == bytes([0xde, 0xad])",
);
runDeltaVectors();
runOptionalEvolution();
runTraitInvariance();
runNestedOptionalConstraint();
runFailurePaths();
process.stdout.write("Generated Go/Python wire contract matrix passed.\n");
