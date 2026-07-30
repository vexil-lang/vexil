import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const toolDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(toolDir, "../..");
const pyright = join(toolDir, "node_modules", "pyright", "index.js");
const python =
  process.env.PYTHON ??
  (process.platform === "win32" ? "python" : "python3");
const runtimePath = resolve(repoRoot, "packages/runtime-py");
const goldenDir = resolve(repoRoot, "crates/vexil-codegen-py/tests/golden");

const syntaxCheck = spawnSync(
  python,
  [
    "-c",
    "import ast, pathlib, sys\nfor value in sys.argv[1:]:\n    path = pathlib.Path(value)\n    ast.parse(path.read_text(encoding='utf-8'), filename=str(path))",
    join(goldenDir, "037_fixed_array.py"),
  ],
  { encoding: "utf8" },
);
if (syntaxCheck.status !== 0) {
  process.stderr.write("A generated Python golden is not valid Python syntax.\n");
  process.stderr.write(syntaxCheck.stdout);
  process.stderr.write(syntaxCheck.stderr);
  process.exit(syntaxCheck.status ?? 1);
}

function runPyright(project, json = false) {
  return spawnSync(
    process.execPath,
    [pyright, "--project", project, ...(json ? ["--outputjson"] : [])],
    { cwd: toolDir, encoding: "utf8" },
  );
}

function fail(result) {
  process.stderr.write(result.stdout);
  process.stderr.write(result.stderr);
  process.exit(result.status ?? 1);
}

function writeProject(temp, sources) {
  const config = join(temp, "pyrightconfig.json");
  writeFileSync(
    config,
    JSON.stringify(
      {
        include: sources.map((source) => relative(temp, source).replaceAll("\\", "/")),
        extraPaths: [relative(temp, runtimePath).replaceAll("\\", "/")],
        pythonVersion: "3.10",
        reportPrivateUsage: "none",
        reportUnusedImport: "none",
        typeCheckingMode: "strict",
      },
      null,
      2,
    ),
  );
  return config;
}

function assertPositive(label, sources) {
  const temp = mkdtempSync(join(tmpdir(), "vexil-pyright-positive-"));
  try {
    const result = runPyright(writeProject(temp, sources));
    if (result.status !== 0) {
      process.stderr.write(`Pyright rejected ${label}.\n`);
      fail(result);
    }
  } finally {
    rmSync(temp, { recursive: true, force: true });
  }
}

function assertProtocolAssignmentFailure(label, source, expectedProtocol) {
  const temp = mkdtempSync(join(tmpdir(), "vexil-pyright-negative-"));
  try {
    const negative = join(temp, `${label}.py`);
    writeFileSync(negative, source);
    const result = runPyright(writeProject(temp, [negative]), true);
    if (result.status === 0) {
      process.stderr.write(
        `Pyright accepted intentionally nonconforming ${label} trait implementation.\n`,
      );
      process.exit(1);
    }

    let output;
    try {
      output = JSON.parse(result.stdout);
    } catch {
      process.stderr.write("Pyright did not produce JSON diagnostics.\n");
      fail(result);
    }
    const assignmentDiagnostic = output.generalDiagnostics?.find(
      (diagnostic) =>
        diagnostic.rule === "reportAssignmentType" &&
        diagnostic.message.includes(expectedProtocol) &&
        diagnostic.message.includes("not assignable"),
    );
    if (!assignmentDiagnostic) {
      process.stderr.write(result.stdout);
      process.stderr.write(result.stderr);
      process.stderr.write(
        `Pyright rejected ${label}, but not with the expected ${expectedProtocol} protocol-assignment diagnostic.\n`,
      );
      process.exit(1);
    }
  } finally {
    rmSync(temp, { recursive: true, force: true });
  }
}

function buildImportedGenericTraitProject(temp, aliased = false) {
  const schemas = join(temp, "schemas");
  const traits = join(schemas, "imported", "traits.vexil");
  const consumer = join(schemas, "imported", "consumer.vexil");
  const output = join(temp, "generated");
  mkdirSync(dirname(traits), { recursive: true });
  writeFileSync(
    traits,
    `namespace imported.traits

trait Tagged<T> {
    tag @0 : T
}
`,
  );
  writeFileSync(
    consumer,
    `namespace imported.consumer

${aliased ? "import imported.traits as Contracts" : "import { Tagged } from imported.traits"}

message Event {
    tag @0 : u64
}

impl ${aliased ? "Contracts.Tagged" : "Tagged"}<u64> for Event { }
`,
  );

  const result = spawnSync(
    "cargo",
    [
      "run",
      "--quiet",
      "-p",
      "vexilc",
      "--",
      "build",
      consumer,
      "--include",
      schemas,
      "--output",
      output,
      "--target",
      "python",
    ],
    { cwd: repoRoot, encoding: "utf8" },
  );
  if (result.status !== 0) {
    process.stderr.write("vexilc could not generate the directly imported generic trait project.\n");
    fail(result);
  }
  return output;
}

const genericField = join(goldenDir, "045_generic_trait.py");
const nestedGeneric = join(goldenDir, "048_generic_trait_nested.py");
const functionBearing = join(goldenDir, "049_trait_function_portable_body.py");
const functionSignature = join(goldenDir, "047_trait_function_codegen_deferred.py");

for (const [label, source] of [
  ["generic trait field output", genericField],
  ["nested generic trait field output", nestedGeneric],
  ["function-bearing trait output", functionBearing],
  ["function-only trait output", functionSignature],
]) {
  const output = readFileSync(source, "utf8");
  if (!output.includes("Protocol") || output.includes(".register(")) {
    throw new Error(`${label} does not retain the Protocol-only conformance model`);
  }
  assertPositive(label, [source]);
}

const importedTemp = mkdtempSync(join(tmpdir(), "vexil-pyright-imported-"));
try {
  const generated = buildImportedGenericTraitProject(importedTemp);
  const consumer = join(generated, "imported", "consumer.py");
  const consumerOutput = readFileSync(consumer, "utf8");
  if (
    !consumerOutput.includes("from imported.traits import Tagged") ||
    !consumerOutput.includes("-> Tagged[int]") ||
    consumerOutput.includes(".register(")
  ) {
    throw new Error("directly imported generic trait output lost its static protocol proof");
  }
  const temp = mkdtempSync(join(tmpdir(), "vexil-pyright-imported-project-"));
  try {
    const config = join(temp, "pyrightconfig.json");
    writeFileSync(
      config,
      JSON.stringify(
        {
          include: [relative(temp, generated).replaceAll("\\", "/")],
          extraPaths: [runtimePath, generated].map((path) =>
            relative(temp, path).replaceAll("\\", "/"),
          ),
          pythonVersion: "3.10",
          reportPrivateUsage: "none",
          reportUnusedImport: "none",
          typeCheckingMode: "strict",
        },
        null,
        2,
      ),
    );
    const result = runPyright(config);
    if (result.status !== 0) {
      process.stderr.write("Pyright rejected the directly imported generic trait project.\n");
      fail(result);
    }
  } finally {
    rmSync(temp, { recursive: true, force: true });
  }
} finally {
  rmSync(importedTemp, { recursive: true, force: true });
}

const aliasedTemp = mkdtempSync(join(tmpdir(), "vexil-pyright-aliased-"));
try {
  const generated = buildImportedGenericTraitProject(aliasedTemp, true);
  const consumer = join(generated, "imported", "consumer.py");
  const consumerOutput = readFileSync(consumer, "utf8");
  if (
    !consumerOutput.includes("from imported.traits import Tagged") ||
    !consumerOutput.includes("-> Tagged[int]") ||
    consumerOutput.includes("Contracts.Tagged") ||
    consumerOutput.includes(".register(")
  ) {
    throw new Error("aliased generic trait output lost its resolved static protocol proof");
  }
  const config = join(aliasedTemp, "aliased-pyrightconfig.json");
  writeFileSync(
    config,
    JSON.stringify(
      {
        include: [relative(aliasedTemp, generated).replaceAll("\\", "/")],
        extraPaths: [runtimePath, generated].map((path) =>
          relative(aliasedTemp, path).replaceAll("\\", "/"),
        ),
        pythonVersion: "3.10",
        reportPrivateUsage: "none",
        reportUnusedImport: "none",
        typeCheckingMode: "strict",
      },
      null,
      2,
    ),
  );
  const result = runPyright(config);
  if (result.status !== 0) {
    process.stderr.write("Pyright rejected the aliased generic trait project.\n");
    fail(result);
  }
} finally {
  rmSync(aliasedTemp, { recursive: true, force: true });
}

assertProtocolAssignmentFailure(
  "wrong_generic_field",
  `${readFileSync(genericField, "utf8")}

if TYPE_CHECKING:
    class _BadEvent:
        tag: str
        label: str

    _bad_event: Tagged[int] = _BadEvent()
`,
  "Tagged[int]",
);

assertProtocolAssignmentFailure(
  "wrong_nested_generic_field",
  `${readFileSync(nestedGeneric, "utf8")}

if TYPE_CHECKING:
    class _BadEventList:
        items: list[str]

    _bad_event_list: Container[int] = _BadEventList()
`,
  "Container[int]",
);

assertProtocolAssignmentFailure(
  "missing_trait_functions",
  `${readFileSync(functionBearing, "utf8")}

if TYPE_CHECKING:
    class _MissingFunctions:
        value: int

    _missing_functions: Adjustable[int] = _MissingFunctions()
`,
  "Adjustable[int]",
);

assertProtocolAssignmentFailure(
  "wrong_trait_function",
  `${readFileSync(functionBearing, "utf8")}

if TYPE_CHECKING:
    class _WrongFunction:
        value: int

        def adjust(self, delta: str) -> int:
            return 0

        def reset(self) -> None:
            pass

        def snapshot(self) -> Counter:
            return Counter(0)

    _wrong_function: Adjustable[int] = _WrongFunction()
`,
  "Adjustable[int]",
);

const runtimeTemp = mkdtempSync(join(tmpdir(), "vexil-python-runtime-"));
try {
  const generated = join(runtimeTemp, "trait_methods.py");
  writeFileSync(generated, readFileSync(functionBearing, "utf8"));
  writeFileSync(
    join(runtimeTemp, "fixed_arrays.py"),
    readFileSync(join(goldenDir, "037_fixed_array.py"), "utf8"),
  );
  writeFileSync(
    join(runtimeTemp, "newtype_fields.py"),
    readFileSync(join(goldenDir, "030_newtype_map_key.py"), "utf8"),
  );
  const runtime = spawnSync(
    python,
    [
      "-c",
      `import sys
sys.path.insert(0, ${JSON.stringify(runtimeTemp)})
from trait_methods import Counter
from fixed_arrays import Nested
from newtype_fields import UserId, UserProfile
counter = Counter(5)
assert counter.adjust(3) == 5
assert counter.value == 8
assert counter.encode() == bytes.fromhex("08000000")
counter.reset()
assert counter.value == 0
nested = Nested(a=((1, 2, 3, 4), (5, 6, 7, 8), (9, 10, 11, 12)))
nested_bytes = nested.encode()
assert nested_bytes == bytes(range(1, 13))
assert Nested.decode(nested_bytes) == nested
profile = UserProfile(id=UserId(7), friends={}, tags={})
profile_bytes = profile.encode()
assert profile_bytes == bytes.fromhex("070000000000")
decoded_profile = UserProfile.decode(profile_bytes)
assert decoded_profile.id == UserId(7)
assert decoded_profile.friends == {} and decoded_profile.tags == {}
`,
    ],
    {
      cwd: runtimeTemp,
      encoding: "utf8",
      env: { ...process.env, PYTHONPATH: runtimePath },
    },
  );
  if (runtime.status !== 0) {
    process.stderr.write(
      "Generated Python runtime import, methods, fixed arrays, newtypes, or encoding regressed.\n",
    );
    fail(runtime);
  }
} finally {
  rmSync(runtimeTemp, { recursive: true, force: true });
}
