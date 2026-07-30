import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const toolDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(toolDir, "../..");
const pyright = join(toolDir, "node_modules", "pyright", "index.js");
const python =
  process.env.PYTHON ??
  (process.platform === "win32" ? "python" : "python3");

function runPyright(project) {
  return spawnSync(process.execPath, [pyright, "--project", project], {
    cwd: toolDir,
    encoding: "utf8",
  });
}

const positive = runPyright(join(toolDir, "pyrightconfig.json"));
if (positive.status !== 0) {
  process.stderr.write(positive.stdout);
  process.stderr.write(positive.stderr);
  process.exit(positive.status ?? 1);
}

const temp = mkdtempSync(join(tmpdir(), "vexil-pyright-negative-"));
try {
  const golden = resolve(
    repoRoot,
    "crates/vexil-codegen-py/tests/golden/049_trait_function_portable_body.py",
  );
  const negative = join(temp, "negative_trait.py");
  writeFileSync(
    negative,
    `${readFileSync(golden, "utf8")}

if TYPE_CHECKING:
    class _BadCounter:
        value: int

    _bad_counter: Adjustable[int] = _BadCounter()
`,
  );
  const config = join(temp, "pyrightconfig.json");
  writeFileSync(
    config,
    JSON.stringify(
      {
        include: [negative],
        extraPaths: [resolve(repoRoot, "packages/runtime-py")],
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
  if (result.status === 0) {
    process.stderr.write(
      "Pyright accepted an intentionally nonconforming generated trait implementation.\n",
    );
    process.exit(1);
  }
  if (!result.stdout.includes("Adjustable") || !result.stdout.includes("not assignable")) {
    process.stderr.write(result.stdout);
    process.stderr.write(result.stderr);
    process.stderr.write("Pyright failed for an unexpected reason.\n");
    process.exit(1);
  }

  const runtime = spawnSync(
    python,
    [
      "-c",
      `import sys
sys.path.insert(0, ${JSON.stringify(temp)})
from negative_trait import Counter
counter = Counter(5)
assert counter.adjust(3) == 5
assert counter.value == 8
assert counter.encode() == bytes.fromhex("08000000")
counter.reset()
assert counter.value == 0
`,
    ],
    {
      cwd: temp,
      encoding: "utf8",
      env: {
        ...process.env,
        PYTHONPATH: resolve(repoRoot, "packages/runtime-py"),
      },
    },
  );
  if (runtime.status !== 0) {
    process.stderr.write(runtime.stdout);
    process.stderr.write(runtime.stderr);
    process.exit(runtime.status ?? 1);
  }
} finally {
  rmSync(temp, { recursive: true, force: true });
}
