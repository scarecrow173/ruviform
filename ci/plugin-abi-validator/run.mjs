#!/usr/bin/env node
// run.mjs — Issue #630 RF-VTEST-03 external ABI validator runner (opt-in).
//
// Drives the two authoritative external validators against the real
// test-plugin artifacts built by this workspace:
//
//   * clap-validator (free-audio/clap-validator, pinned release binary)
//     against `ruviform_plugin_test_clap.dll` — one run per plugin id.
//   * Steinberg VST3 SDK `validator` (built from the pinned SDK source
//     revision) against a staged `ruviform-plugin-test-vst3.vst3`
//     bundle wrapping `ruviform_plugin_test_vst3.dll`.
//
// This lane is opt-in and NOT a CI-required gate: it exists so the
// suite's ABI conformance is proven by tools independent of the
// production adapters (POLICIES.md "独立Validator/Reference Host").
// The intended mise task registration (Coordinator-owned) is:
//
//   [tasks.test-plugin-abi-validator]
//   description = "Run external ABI validators (clap-validator + VST3 SDK validator) against the test-plugin suite — opt-in, NOT a CI-required gate"
//   run = "node ci/plugin-abi-validator/run.mjs"
//
// Tool resolution (external tools are never workspace dependencies):
//   --clap-validator <path>   | env CLAP_VALIDATOR
//   --vst3-validator <path>   | env VST3_VALIDATOR
//   --validator-dir <dir>     | env RUVIFORM_ABI_VALIDATOR_DIR
//       conventional layout: <dir>/clap-validator[.exe] and
//       <dir>/vst3-validator[.exe]
//   --clap-dll <path>         | env CLAP_TEST-dll — default
//       <repo>/target/debug/ruviform_plugin_test_clap.dll
//   --vst3-dll <path>         | env VST3_TEST_DLL — default
//       <repo>/target/debug/ruviform_plugin_test_vst3.dll
//   --no-build                skip `cargo build` of both fixture crates
//   --evidence-dir <dir>      default <repo>/evidence/rf630/abi-validator
//
// Fail-closed contract: a missing validator binary, missing artifact,
// failed build, validator crash or any failed/crashed test case yields
// a non-zero exit and an honest NotRun/Failed verdict in run.json —
// the lane never reports success it did not measure.
//
// Verdict vocabulary follows POLICIES.md: Passed / Failed / NotRun /
// Unsupported. `Unsupported` is reserved for surfaces the fixture
// intentionally does not implement (recorded with a reason), never
// for unrun checks.

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");

// The suite's CLAP plugins, one clap-validator run each so a crash or
// failure in one fixture cannot mask the others. `test_fault` arms
// fatal fault modes (abort/exit/hang) through its stepped parameter —
// param-mutating validator tests would crash the sandboxed child by
// design, so its canonical run excludes the `param-set-*`/`param-fuzz-*`
// cases. The unfiltered run is still executed as an *informational*
// leg: its abnormal termination is recorded as Unsupported evidence of
// the intentional-crash surface, not as a plugin defect.
export const CLAP_PLUGINS = [
  { id: "org.ruviform-test.clap-test-gain", name: "test_gain" },
  {
    id: "org.ruviform-test.clap-test-fault",
    name: "test_fault",
    exclude: "param-(set|fuzz)",
    excludeReason:
      "test_fault's stepped Fault parameter intentionally arms abort/exit/hang modes; " +
      "param-mutating validator cases would crash the sandboxed child by design. " +
      "The fatal surface is covered by the L1 out-of-process fault lane instead.",
  },
  { id: "org.ruviform-test.clap-test-stateful", name: "test_stateful" },
  { id: "org.ruviform-test.clap-test-instrument", name: "test_instrument" },
  { id: "org.ruviform-test.clap-test-editor", name: "test_editor" },
];

const USAGE = `Usage: node ci/plugin-abi-validator/run.mjs [options]

Options:
  --clap-validator <path>   clap-validator executable
  --vst3-validator <path>   Steinberg VST3 SDK validator executable
  --validator-dir <dir>     directory containing clap-validator[.exe] and vst3-validator[.exe]
  --clap-dll <path>         ruviform_plugin_test_clap.dll (default: <repo>/target/debug/...)
  --vst3-dll <path>         ruviform_plugin_test_vst3.dll (default: <repo>/target/debug/...)
  --no-build                skip building both fixture crates with cargo
  --evidence-dir <dir>      evidence output directory
                            (default: <repo>/evidence/rf630/abi-validator)
  -h, --help                show this help
`;

export function parseArgs(argv) {
  const opts = {
    clapValidator: process.env.CLAP_VALIDATOR || null,
    vst3Validator: process.env.VST3_VALIDATOR || null,
    validatorDir: process.env.RUVIFORM_ABI_VALIDATOR_DIR || null,
    clapDll: process.env.CLAP_TEST_DLL || null,
    vst3Dll: process.env.VST3_TEST_DLL || null,
    build: true,
    evidenceDir: null,
    help: false,
  };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    const next = () => {
      const v = argv[++i];
      if (v === undefined) throw new Error(`missing value for ${arg}`);
      return v;
    };
    switch (arg) {
      case "--clap-validator":
        opts.clapValidator = next();
        break;
      case "--vst3-validator":
        opts.vst3Validator = next();
        break;
      case "--validator-dir":
        opts.validatorDir = next();
        break;
      case "--clap-dll":
        opts.clapDll = next();
        break;
      case "--vst3-dll":
        opts.vst3Dll = next();
        break;
      case "--no-build":
        opts.build = false;
        break;
      case "--evidence-dir":
        opts.evidenceDir = next();
        break;
      case "-h":
      case "--help":
        opts.help = true;
        break;
      default:
        throw new Error(`unknown argument: ${arg}`);
    }
  }
  return opts;
}

function executableCandidates(dir, base) {
  return process.platform === "win32"
    ? [join(dir, `${base}.exe`), join(dir, base)]
    : [join(dir, base)];
}

// Resolves validator binaries. Explicit paths win; --validator-dir
// supplies the conventional <dir>/clap-validator[.exe] and
// <dir>/vst3-validator[.exe] layout. Returns { clap, vst3 } paths or
// null per missing tool — never guesses.
export function resolveValidators(opts) {
  let clap = opts.clapValidator;
  let vst3 = opts.vst3Validator;
  if ((!clap || !vst3) && opts.validatorDir) {
    const dir = resolve(opts.validatorDir);
    if (!clap) {
      clap = executableCandidates(dir, "clap-validator").find(existsSync) || null;
    }
    if (!vst3) {
      vst3 = executableCandidates(dir, "vst3-validator").find(existsSync) || null;
    }
  }
  return {
    clap: clap && existsSync(clap) ? resolve(clap) : null,
    vst3: vst3 && existsSync(vst3) ? resolve(vst3) : null,
  };
}

export function sha256File(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function gitHead() {
  const r = spawnSync("git", ["rev-parse", "HEAD"], { cwd: REPO_ROOT, encoding: "utf8" });
  return r.status === 0 ? r.stdout.trim() : null;
}

// Normalizes a finished clap-validator invocation into a leg verdict.
// Pure — the contract tests drive it with canned process results.
export function normalizeClapLeg({ proc, plugin, extraArgs, validator, dll, durationMs }) {
  const args = ["validate", "--json", ...extraArgs, "--plugin-id", plugin.id, dll];
  const leg = {
    kind: "clap-validator",
    plugin: plugin.name,
    pluginId: plugin.id,
    command: [validator, ...args].join(" "),
    durationMs,
    exitCode: proc.status ?? null,
    signal: proc.signal || null,
    counts: null,
    failedCases: [],
    warnings: [],
    skipped: 0,
    status: null,
  };
  const raw = `${proc.stdout || ""}\n${proc.stderr || ""}`;
  leg.rawOutput = raw;

  let parsed = null;
  try {
    const json = JSON.parse(proc.stdout || "");
    parsed = json.results || json;
  } catch {
    // No JSON — validator crashed or was killed before reporting.
  }

  if (Array.isArray(parsed)) {
    const counts = { success: 0, skipped: 0, warning: 0, failed: 0, crashed: 0 };
    for (const item of parsed) {
      const code = item?.status?.code;
      if (code in counts) counts[code] += 1;
      if (code === "failed" || code === "crashed") {
        leg.failedCases.push({
          test: item.test?.["plugin-instance"]?.test || item.test?.["plugin-library"]?.test || "?",
          code,
          details: item.status?.details || null,
        });
      } else if (code === "warning") {
        leg.warnings.push(item.status?.details || "(no details)");
      }
    }
    leg.counts = counts;
    leg.skipped = counts.skipped;
  }

  if (proc.error) {
    leg.status = "Failed";
    leg.details = `spawn error: ${proc.error.message}`;
  } else if (proc.status !== 0 && !leg.counts) {
    // Non-zero exit without a JSON report — the validator itself
    // terminated abnormally (e.g. sandbox panic). That is a tool-side
    // limitation, not a measured plugin failure.
    leg.status = "Unsupported";
    leg.details = `validator exited ${proc.status} without a JSON report (tool-side termination)`;
  } else if (leg.counts && (leg.counts.failed > 0 || leg.counts.crashed > 0)) {
    leg.status = "Failed";
    leg.details = `${leg.counts.failed} failed, ${leg.counts.crashed} crashed test case(s)`;
  } else if (leg.counts) {
    leg.status = "Passed";
    leg.details =
      `${leg.counts.success} passed, ${leg.counts.skipped} skipped, ` +
      `${leg.counts.warning} warning(s), 0 failed`;
  } else {
    leg.status = "Failed";
    leg.details = "unparseable validator output";
  }
  return leg;
}

// Runs clap-validator for one plugin id and returns a normalized leg
// result. `extraArgs` carries the declared per-plugin scope (e.g. the
// fault fixture's param exclusion).
export function runClapLeg({ validator, dll, plugin, extraArgs = [] }) {
  const args = ["validate", "--json", ...extraArgs, "--plugin-id", plugin.id, dll];
  const started = Date.now();
  const proc = spawnSync(validator, args, {
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
    timeout: 300_000,
  });
  return normalizeClapLeg({
    proc,
    plugin,
    extraArgs,
    validator,
    dll,
    durationMs: Date.now() - started,
  });
}

// Normalizes a finished VST3 SDK validator invocation. Pure.
export function normalizeVst3Leg({ proc, validator, bundle, durationMs }) {
  const args = [bundle];
  const raw = `${proc.stdout || ""}\n${proc.stderr || ""}`;
  const leg = {
    kind: "vst3-sdk-validator",
    plugin: "vst3 suite bundle",
    pluginId: null,
    command: [validator, ...args].join(" "),
    durationMs,
    exitCode: proc.status ?? null,
    signal: proc.signal || null,
    counts: null,
    failedCases: [],
    warnings: [],
    skipped: 0,
    status: null,
    rawOutput: raw,
  };

  const match = raw.match(/Result:\s*(\d+)\s+tests passed,\s*(\d+)\s+tests failed/);
  if (match) {
    leg.counts = { passed: Number(match[1]), failed: Number(match[2]) };
  }

  if (proc.error) {
    leg.status = "Failed";
    leg.details = `spawn error: ${proc.error.message}`;
  } else if (!match) {
    leg.status = proc.status === 0 ? "Failed" : "Unsupported";
    leg.details = `no result summary in validator output (exit ${proc.status})`;
  } else if (leg.counts.failed > 0 || proc.status !== 0) {
    leg.status = "Failed";
    leg.details = `${leg.counts.passed} passed, ${leg.counts.failed} failed (exit ${proc.status})`;
  } else {
    leg.status = "Passed";
    leg.details = `${leg.counts.passed} passed, 0 failed`;
  }
  return leg;
}

// Runs the Steinberg SDK validator against a staged .vst3 bundle.
export function runVst3Leg({ validator, bundle }) {
  const started = Date.now();
  const proc = spawnSync(validator, [bundle], {
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
    timeout: 600_000,
  });
  return normalizeVst3Leg({
    proc,
    validator,
    bundle,
    durationMs: Date.now() - started,
  });
}

// Stages the VST3 cdylib as a spec-compliant Windows bundle:
// <name>.vst3/Contents/x86_64-win/<name>.vst3 — the inner binary name
// must match the bundle folder name.
export function stageVst3Bundle(dllPath, stageRoot) {
  const bundleName = "ruviform-plugin-test-vst3.vst3";
  const bundleDir = join(stageRoot, bundleName);
  const innerDir = join(bundleDir, "Contents", "x86_64-win");
  mkdirSync(innerDir, { recursive: true });
  const innerPath = join(innerDir, bundleName);
  copyFileSync(dllPath, innerPath);
  return { bundleDir, innerPath };
}

// Canonical legs: every CLAP plugin (fault scoped) + the VST3 bundle.
// Informational legs (fault unfiltered) never affect the exit code.
export function canonicalLegs(legs) {
  return legs.filter((l) => !l.informational);
}

export function evaluate(legs) {
  const canon = canonicalLegs(legs);
  const failed = canon.filter((l) => l.status === "Failed");
  const notRun = canon.filter((l) => l.status === "NotRun");
  const unsupported = canon.filter((l) => l.status === "Unsupported");
  const passed = canon.filter((l) => l.status === "Passed");
  const ok = failed.length === 0 && notRun.length === 0 && unsupported.length === 0;
  return {
    verdict: ok ? "Passed" : failed.length > 0 ? "Failed" : notRun.length > 0 ? "NotRun" : "Unsupported",
    exitCode: ok ? 0 : 1,
    summary: {
      total: canon.length,
      passed: passed.length,
      failed: failed.length,
      notRun: notRun.length,
      unsupported: unsupported.length,
    },
  };
}

function notRunLeg(kind, name, pluginId, reason) {
  return {
    kind,
    plugin: name,
    pluginId,
    status: "NotRun",
    details: reason,
    counts: null,
    failedCases: [],
    warnings: [],
    skipped: 0,
    exitCode: null,
  };
}

function buildFixtureArtifacts(opts) {
  if (!opts.build) return { ran: false, status: "skipped (--no-build)" };
  const args = [
    "build",
    "-p",
    "ruviform-plugin-test-clap",
    "-p",
    "ruviform-plugin-test-vst3",
  ];
  const proc = spawnSync("cargo", args, {
    cwd: REPO_ROOT,
    encoding: "utf8",
    maxBuffer: 16 * 1024 * 1024,
    timeout: 600_000,
  });
  return {
    ran: true,
    command: ["cargo", ...args].join(" "),
    exitCode: proc.status,
    ok: proc.status === 0 && !proc.error,
    tail: `${proc.stdout || ""}\n${proc.stderr || ""}`.trim().split("\n").slice(-20).join("\n"),
  };
}

export function run(opts) {
  const startedAt = new Date().toISOString();
  const evidenceDir = resolve(opts.evidenceDir || join(REPO_ROOT, "evidence", "rf630", "abi-validator"));
  mkdirSync(evidenceDir, { recursive: true });

  const tools = resolveValidators(opts);
  const clapDll = resolve(opts.clapDll || join(REPO_ROOT, "target", "debug", "ruviform_plugin_test_clap.dll"));
  const vst3Dll = resolve(opts.vst3Dll || join(REPO_ROOT, "target", "debug", "ruviform_plugin_test_vst3.dll"));

  const build = buildFixtureArtifacts(opts);

  const legs = [];

  // -- CLAP legs ---------------------------------------------------------
  if (!tools.clap) {
    for (const plugin of CLAP_PLUGINS) {
      legs.push(
        notRunLeg(
          "clap-validator",
          plugin.name,
          plugin.id,
          "clap-validator binary not found — provision the pinned release " +
            "(see docs/specs/issue-630-abi-validator.md) or pass --clap-validator / CLAP_VALIDATOR",
        ),
      );
    }
  } else if (!existsSync(clapDll)) {
    for (const plugin of CLAP_PLUGINS) {
      legs.push(
        notRunLeg(
          "clap-validator",
          plugin.name,
          plugin.id,
          `artifact missing: ${clapDll} — run without --no-build or pass --clap-dll`,
        ),
      );
    }
  } else if (build.ran && !build.ok) {
    for (const plugin of CLAP_PLUGINS) {
      legs.push(
        notRunLeg(
          "clap-validator",
          plugin.name,
          plugin.id,
          `fixture build failed (exit ${build.exitCode}) — see build.tail in run.json`,
        ),
      );
    }
  } else {
    for (const plugin of CLAP_PLUGINS) {
      const extraArgs = plugin.exclude ? ["--exclude", plugin.exclude] : [];
      const leg = runClapLeg({
        validator: tools.clap,
        dll: clapDll,
        plugin,
        extraArgs,
      });
      if (plugin.exclude) leg.exclusion = { pattern: plugin.exclude, reason: plugin.excludeReason };
      legs.push(leg);
      writeFileSync(join(evidenceDir, `clap-${plugin.name}.log`), leg.rawOutput || "");
      delete leg.rawOutput;
    }
    // Informational unfiltered fault run — records the intentional
    // crash surface without affecting the verdict.
    const fault = CLAP_PLUGINS.find((p) => p.exclude);
    if (fault) {
      const leg = runClapLeg({ validator: tools.clap, dll: clapDll, plugin: fault });
      leg.informational = true;
      leg.details =
        `unfiltered run: ${leg.status} — ${leg.details || ""} ` +
        `(expected: fatal fault modes are armed by param-mutating cases)`;
      legs.push(leg);
      writeFileSync(join(evidenceDir, `clap-${fault.name}-unfiltered.log`), leg.rawOutput || "");
      delete leg.rawOutput;
    }
  }

  // -- VST3 leg ----------------------------------------------------------
  if (!tools.vst3) {
    legs.push(
      notRunLeg(
        "vst3-sdk-validator",
        "vst3 suite bundle",
        null,
        "VST3 SDK validator binary not found — build `validator` from the " +
          "pinned SDK source revision (see docs/specs/issue-630-abi-validator.md) " +
          "or pass --vst3-validator / VST3_VALIDATOR",
      ),
    );
  } else if (!existsSync(vst3Dll)) {
    legs.push(
      notRunLeg(
        "vst3-sdk-validator",
        "vst3 suite bundle",
        null,
        `artifact missing: ${vst3Dll} — run without --no-build or pass --vst3-dll`,
      ),
    );
  } else if (build.ran && !build.ok) {
    legs.push(
      notRunLeg(
        "vst3-sdk-validator",
        "vst3 suite bundle",
        null,
        `fixture build failed (exit ${build.exitCode}) — see build.tail in run.json`,
      ),
    );
  } else {
    const stageRoot = mkdtempSync(join(tmpdir(), "rf630-vst3-"));
    try {
      const { bundleDir, innerPath } = stageVst3Bundle(vst3Dll, stageRoot);
      const leg = runVst3Leg({ validator: tools.vst3, bundle: bundleDir });
      leg.bundle = bundleDir;
      leg.innerBinary = innerPath;
      legs.push(leg);
      writeFileSync(join(evidenceDir, "vst3-validator.log"), leg.rawOutput || "");
      delete leg.rawOutput;
    } finally {
      rmSync(stageRoot, { recursive: true, force: true });
    }
  }

  const evaluation = evaluate(legs);

  const report = {
    schema: "rf630-abi-validator-run/1",
    startedAt,
    finishedAt: new Date().toISOString(),
    repo: {
      root: REPO_ROOT,
      head: gitHead(),
      platform: `${process.platform}/${process.arch}`,
      osRelease: null,
      node: process.version,
    },
    tools: {
      clapValidator: tools.clap ? { path: tools.clap, sha256: sha256File(tools.clap) } : null,
      vst3Validator: tools.vst3 ? { path: tools.vst3, sha256: sha256File(tools.vst3) } : null,
    },
    artifacts: {
      clapDll: existsSync(clapDll) ? { path: clapDll, sha256: sha256File(clapDll) } : { path: clapDll, missing: true },
      vst3Dll: existsSync(vst3Dll) ? { path: vst3Dll, sha256: sha256File(vst3Dll) } : { path: vst3Dll, missing: true },
    },
    build,
    legs,
    evaluation,
  };

  try {
    report.repo.osRelease = spawnSync("uname", ["-sr"], { encoding: "utf8" }).stdout?.trim() || null;
  } catch {
    /* os-release is best-effort evidence */
  }

  writeFileSync(join(evidenceDir, "run.json"), `${JSON.stringify(report, null, 2)}\n`);
  return report;
}

const isMain = process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  let opts;
  try {
    opts = parseArgs(process.argv.slice(2));
  } catch (err) {
    console.error(`error: ${err.message}\n`);
    console.error(USAGE);
    process.exit(2);
  }
  if (opts.help) {
    console.log(USAGE);
    process.exit(0);
  }
  const report = run(opts);
  for (const leg of report.legs) {
    const tag = leg.informational ? "info " : "     ";
    console.log(`${tag}[${leg.status}] ${leg.kind}: ${leg.plugin} — ${leg.details || ""}`);
  }
  console.log(`verdict: ${report.evaluation.verdict} (exit ${report.evaluation.exitCode})`);
  console.log(`evidence: ${join(report.evaluation && opts.evidenceDir ? resolve(opts.evidenceDir) : join(REPO_ROOT, "evidence", "rf630", "abi-validator"), "run.json")}`);
  process.exit(report.evaluation.exitCode);
}
