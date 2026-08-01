import {
  cpSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const toolDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(toolDir, "../..");
const tsc = resolve(repoRoot, "packages/runtime-ts/node_modules/typescript/bin/tsc");
const golden = resolve(
  repoRoot,
  "crates/vexil-codegen-ts/tests/golden/049_trait_function_portable_body.ts",
);
const nestedOptionalTailGolden = resolve(
  repoRoot,
  "crates/vexil-codegen-ts/tests/golden/nested_optional_tail.ts",
);
const runtimePackage = resolve(repoRoot, "packages/runtime-ts");

function runTsc(project) {
  return spawnSync(process.execPath, [tsc, "--project", project], {
    cwd: toolDir,
    encoding: "utf8",
  });
}

function installRuntime(temp) {
  const destination = join(temp, "node_modules", "@vexil-lang", "runtime");
  mkdirSync(destination, { recursive: true });
  cpSync(join(runtimePackage, "dist"), join(destination, "dist"), {
    recursive: true,
  });
  cpSync(join(runtimePackage, "package.json"), join(destination, "package.json"));
}

function writeProject(temp, source, emit = false) {
  const generated = join(temp, "generated.ts");
  writeFileSync(generated, source);
  const config = join(temp, "tsconfig.json");
  writeFileSync(
    config,
    JSON.stringify(
      {
        compilerOptions: {
          module: "NodeNext",
          moduleResolution: "NodeNext",
          noEmit: !emit,
          outDir: emit ? join(temp, "out") : undefined,
          skipLibCheck: true,
          strict: true,
          target: "ES2022",
        },
        files: [generated],
      },
      null,
      2,
    ),
  );
  return config;
}

function runGeneratedContract(goldenPath, contractSource, label) {
  const temp = mkdtempSync(join(tmpdir(), `vexil-ts-${label}-`));
  try {
    installRuntime(temp);
    const source = readFileSync(goldenPath, "utf8");
    const emitted = runTsc(writeProject(temp, source, true));
    if (emitted.status !== 0) {
      process.stderr.write(emitted.stdout);
      process.stderr.write(emitted.stderr);
      process.stderr.write(`${label}: TypeScript compilation failed.\n`);
      process.exit(emitted.status ?? 1);
    }
    const contract = join(temp, "contract.mjs");
    writeFileSync(contract, contractSource);
    const runtime = spawnSync(process.execPath, [contract], {
      cwd: temp,
      encoding: "utf8",
    });
    if (runtime.status !== 0) {
      process.stderr.write(runtime.stdout);
      process.stderr.write(runtime.stderr);
      process.stderr.write(`${label}: generated runtime contract failed.\n`);
      process.exit(runtime.status ?? 1);
    }
  } finally {
    rmSync(temp, { recursive: true, force: true });
  }
}

function checkAliasedTraitProject() {
  const temp = mkdtempSync(join(tmpdir(), "vexil-tsc-alias-project-"));
  try {
    installRuntime(temp);
    const project = resolve(repoRoot, "corpus/projects/trait_alias");
    const generated = join(temp, "generated");
    const build = spawnSync(
      "cargo",
      [
        "run",
        "--quiet",
        "-p",
        "vexilc",
        "--",
        "build",
        join(project, "app/root.vexil"),
        "--include",
        project,
        "--output",
        generated,
        "--target",
        "typescript",
      ],
      { cwd: repoRoot, encoding: "utf8" },
    );
    if (build.status !== 0) {
      process.stderr.write(build.stdout);
      process.stderr.write(build.stderr);
      process.stderr.write("vexilc could not generate the aliased TypeScript trait project.\n");
      process.exit(build.status ?? 1);
    }
    const config = join(temp, "alias-tsconfig.json");
    writeFileSync(
      config,
      JSON.stringify(
        {
          compilerOptions: {
            module: "ESNext",
            moduleResolution: "Bundler",
            noEmit: true,
            skipLibCheck: true,
            strict: true,
            target: "ES2022",
          },
          include: [join(generated, "**/*.ts")],
        },
        null,
        2,
      ),
    );
    const result = runTsc(config);
    if (result.status !== 0) {
      process.stderr.write(result.stdout);
      process.stderr.write(result.stderr);
      process.stderr.write("TypeScript rejected the aliased generated trait project.\n");
      process.exit(result.status ?? 1);
    }
  } finally {
    rmSync(temp, { recursive: true, force: true });
  }
}

const temp = mkdtempSync(join(tmpdir(), "vexil-tsc-check-"));
try {
  installRuntime(temp);
  const source = readFileSync(golden, "utf8");
  const positive = runTsc(writeProject(temp, source));
  if (positive.status !== 0) {
    process.stderr.write(positive.stdout);
    process.stderr.write(positive.stderr);
    process.exit(positive.status ?? 1);
  }

  const negativeSource = `${source}
const _bad: Adjustable<number> = { value: 0 };
void _bad;
`;
  const negative = runTsc(writeProject(temp, negativeSource));
  if (negative.status === 0) {
    process.stderr.write(
      "TypeScript accepted an intentionally nonconforming generated trait implementation.\n",
    );
    process.exit(1);
  }
  if (
    !negative.stdout.includes("Adjustable<number>") ||
    !negative.stdout.includes("missing the following properties")
  ) {
    process.stderr.write(negative.stdout);
    process.stderr.write(negative.stderr);
    process.stderr.write("TypeScript failed for an unexpected reason.\n");
    process.exit(1);
  }

  const emitted = runTsc(writeProject(temp, source, true));
  if (emitted.status !== 0) {
    process.stderr.write(emitted.stdout);
    process.stderr.write(emitted.stderr);
    process.exit(emitted.status ?? 1);
  }
  const contract = join(temp, "contract.mjs");
  writeFileSync(
    contract,
    `import { BitWriter } from '@vexil-lang/runtime';
import { createCounter, encodeCounter } from './out/generated.js';

const counter = createCounter({ value: 5, _unknown: new Uint8Array(0) });
if (counter.adjust(3) !== 5 || counter.value !== 8) {
  throw new Error('generated TypeScript trait method behavior mismatch');
}
const writer = new BitWriter();
encodeCounter(counter, writer);
const bytes = writer.finish();
if (Buffer.from(bytes).toString('hex') !== '08000000') {
  throw new Error(\`generated TypeScript wire bytes mismatch: \${Buffer.from(bytes).toString('hex')}\`);
}
counter.reset();
if (counter.value !== 0) {
  throw new Error('generated TypeScript reset behavior mismatch');
}
`,
  );
  const runtime = spawnSync(process.execPath, [contract], {
    cwd: temp,
    encoding: "utf8",
  });
  if (runtime.status !== 0) {
    process.stderr.write(runtime.stdout);
    process.stderr.write(runtime.stderr);
    process.exit(runtime.status ?? 1);
  }
  runGeneratedContract(
    nestedOptionalTailGolden,
    `import { BitReader, BitWriter } from '@vexil-lang/runtime';
import { decodeM, encodeM } from './out/generated.js';

function encode(v, tail) {
  const writer = new BitWriter();
  encodeM({ v, tail, _unknown: new Uint8Array(0) }, writer);
  return writer.finish();
}
function hex(bytes) { return Buffer.from(bytes).toString('hex'); }

const innerNoneBytes = encode([null], true);
if (hex(innerNoneBytes) !== '05') throw new Error(\`inner-none tail packing: \${hex(innerNoneBytes)}\`);
const innerNone = decodeM(new BitReader(Uint8Array.from([0x05])));
if (!Array.isArray(innerNone.v) || innerNone.v[0] !== null || !innerNone.tail) {
  throw new Error('inner-none tail decode');
}
if (hex(encode(innerNone.v, innerNone.tail)) !== '05') throw new Error('inner-none tail round trip');
if (hex(encode(null, true)) !== '02') throw new Error('outer-none tail packing');
`,
    "nested-optional-tail",
  );
  checkAliasedTraitProject();
} finally {
  rmSync(temp, { recursive: true, force: true });
}
