// run.test.mjs — contract tests for ci/plugin-abi-validator/run.mjs.
//
// The lane is opt-in and fail-closed: these tests pin the verdict
// vocabulary (Passed/Failed/NotRun/Unsupported), the tool/artifact
// resolution rules, the per-plugin CLAP scoping, and the guarantee
// that missing prerequisites can never produce a pass.

import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import assert from "node:assert/strict";

import {
  CLAP_PLUGINS,
  canonicalLegs,
  evaluate,
  normalizeClapLeg,
  normalizeVst3Leg,
  parseArgs,
  resolveValidators,
  run,
  stageVst3Bundle,
} from "../run.mjs";

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url));

function clapJson(counts) {
  const statuses = [];
  for (const [code, n] of Object.entries(counts)) {
    for (let i = 0; i < n; i++) {
      statuses.push({
        test: { "plugin-instance": { test: `case-${statuses.length}`, path: "x.dll" } },
        status: { code, details: null },
        duration: { secs: 0, nanos: 0 },
      });
    }
  }
  return JSON.stringify({ results: statuses });
}

function clapProc({ status = 0, stdout = "", stderr = "", error = null } = {}) {
  return { status, signal: null, stdout, stderr, error };
}

// -- parseArgs -------------------------------------------------------------

test("parseArgs reads explicit tool paths and flags", () => {
  const opts = parseArgs([
    "--clap-validator", "c:/tools/clap-validator.exe",
    "--vst3-validator", "c:/tools/vst3-validator.exe",
    "--validator-dir", "c:/tools",
    "--clap-dll", "c:/dlls/clap.dll",
    "--vst3-dll", "c:/dlls/vst3.dll",
    "--no-build",
    "--evidence-dir", "c:/ev",
  ]);
  assert.equal(opts.clapValidator, "c:/tools/clap-validator.exe");
  assert.equal(opts.vst3Validator, "c:/tools/vst3-validator.exe");
  assert.equal(opts.validatorDir, "c:/tools");
  assert.equal(opts.clapDll, "c:/dlls/clap.dll");
  assert.equal(opts.vst3Dll, "c:/dlls/vst3.dll");
  assert.equal(opts.build, false);
  assert.equal(opts.evidenceDir, "c:/ev");
});

test("parseArgs reads the production CLAP options", () => {
  const opts = parseArgs([
    "--clap-production",
    "--clap-production-dll", "c:/dlls/prod-clap.dll",
  ]);
  assert.equal(opts.clapProduction, true);
  assert.equal(opts.clapProductionDll, "c:/dlls/prod-clap.dll");
  const off = parseArgs([]);
  assert.equal(off.clapProduction, false);
  assert.equal(off.clapProductionDll, null);
});

test("parseArgs rejects unknown arguments", () => {
  assert.throws(() => parseArgs(["--bogus"]), /unknown argument/);
  assert.throws(() => parseArgs(["--clap-validator"]), /missing value/);
});

// -- resolveValidators -----------------------------------------------------

test("resolveValidators returns null for absent tools — never guesses", () => {
  const dir = mkdtempSync(join(tmpdir(), "rf630-noval-"));
  const resolved = resolveValidators({ validatorDir: dir });
  assert.equal(resolved.clap, null);
  assert.equal(resolved.vst3, null);
});

test("resolveValidators honors the conventional validator-dir layout", () => {
  const dir = mkdtempSync(join(tmpdir(), "rf630-val-"));
  const clapPath = join(dir, process.platform === "win32" ? "clap-validator.exe" : "clap-validator");
  const vst3Path = join(dir, process.platform === "win32" ? "vst3-validator.exe" : "vst3-validator");
  writeFileSync(clapPath, "fake");
  writeFileSync(vst3Path, "fake");
  const resolved = resolveValidators({ validatorDir: dir });
  assert.equal(resolved.clap, clapPath);
  assert.equal(resolved.vst3, vst3Path);
});

test("resolveValidators prefers explicit paths over validator-dir", () => {
  const dir = mkdtempSync(join(tmpdir(), "rf630-val-"));
  const explicit = join(dir, "explicit.exe");
  writeFileSync(explicit, "fake");
  const resolved = resolveValidators({
    validatorDir: dir,
    clapValidator: explicit,
    vst3Validator: explicit,
  });
  assert.equal(resolved.clap, explicit);
  assert.equal(resolved.vst3, explicit);
});

// -- normalizeClapLeg ------------------------------------------------------

test("normalizeClapLeg: clean JSON report is Passed with honest counts", () => {
  const leg = normalizeClapLeg({
    proc: clapProc({ stdout: clapJson({ success: 26, skipped: 17, warning: 1 }) }),
    plugin: CLAP_PLUGINS[0],
    extraArgs: [],
    validator: "clap-validator.exe",
    dll: "x.dll",
    durationMs: 1,
  });
  assert.equal(leg.status, "Passed");
  assert.deepEqual(leg.counts, { success: 26, skipped: 17, warning: 1, failed: 0, crashed: 0 });
  assert.equal(leg.skipped, 17);
});

test("normalizeClapLeg: failed or crashed cases are Failed with details", () => {
  for (const code of ["failed", "crashed"]) {
    const leg = normalizeClapLeg({
      proc: clapProc({
        status: 1,
        stdout: JSON.stringify({
          results: [
            {
              test: { "plugin-instance": { test: "bad-case", path: "x.dll" } },
              status: { code, details: "boom" },
              duration: { secs: 0, nanos: 0 },
            },
          ],
        }),
      }),
      plugin: CLAP_PLUGINS[0],
      extraArgs: [],
      validator: "v",
      dll: "d",
      durationMs: 0,
    });
    assert.equal(leg.status, "Failed", code);
    assert.equal(leg.failedCases.length, 1);
    assert.equal(leg.failedCases[0].test, "bad-case");
  }
});

test("normalizeClapLeg: nonzero exit without JSON is Unsupported (tool-side), not Failed", () => {
  const leg = normalizeClapLeg({
    proc: clapProc({ status: 101, stdout: "", stderr: "thread 'main' panicked" }),
    plugin: CLAP_PLUGINS[1],
    extraArgs: [],
    validator: "v",
    dll: "d",
    durationMs: 0,
  });
  assert.equal(leg.status, "Unsupported");
  assert.match(leg.details, /tool-side termination/);
});

test("normalizeClapLeg: zero exit with unparseable output is Failed", () => {
  const leg = normalizeClapLeg({
    proc: clapProc({ status: 0, stdout: "not json" }),
    plugin: CLAP_PLUGINS[0],
    extraArgs: [],
    validator: "v",
    dll: "d",
    durationMs: 0,
  });
  assert.equal(leg.status, "Failed");
});

// -- normalizeVst3Leg ------------------------------------------------------

test("normalizeVst3Leg: result summary drives the verdict", () => {
  const ok = normalizeVst3Leg({
    proc: clapProc({ status: 0, stdout: "noise\nResult: 235 tests passed, 0 tests failed\n" }),
    validator: "v",
    bundle: "b.vst3",
    durationMs: 0,
  });
  assert.equal(ok.status, "Passed");
  assert.deepEqual(ok.counts, { passed: 235, failed: 0 });

  const bad = normalizeVst3Leg({
    proc: clapProc({ status: 0, stdout: "Result: 200 tests passed, 3 tests failed" }),
    validator: "v",
    bundle: "b.vst3",
    durationMs: 0,
  });
  assert.equal(bad.status, "Failed");

  const crash = normalizeVst3Leg({
    proc: clapProc({ status: 1, stdout: "segfault" }),
    validator: "v",
    bundle: "b.vst3",
    durationMs: 0,
  });
  assert.equal(crash.status, "Unsupported");
});

// -- stageVst3Bundle -------------------------------------------------------

test("stageVst3Bundle produces the spec layout with matching inner name", () => {
  const dir = mkdtempSync(join(tmpdir(), "rf630-stage-"));
  const dll = join(dir, "src.dll");
  writeFileSync(dll, "dll-bytes");
  const { bundleDir, innerPath } = stageVst3Bundle(dll, dir);
  assert.ok(bundleDir.endsWith("ruviform-plugin-test-vst3.vst3"));
  assert.ok(innerPath.includes("x86_64-win"));
  assert.ok(innerPath.endsWith("ruviform-plugin-test-vst3.vst3"));
  assert.equal(readFileSync(innerPath, "utf8"), "dll-bytes");
});

// -- evaluate ---------------------------------------------------------------

function leg(status, informational = false) {
  return { status, informational, kind: "k", plugin: "p" };
}

test("evaluate: all canonical legs Passed → verdict Passed, exit 0", () => {
  const ev = evaluate([leg("Passed"), leg("Passed")]);
  assert.equal(ev.verdict, "Passed");
  assert.equal(ev.exitCode, 0);
});

test("evaluate: any Failed → verdict Failed, exit 1", () => {
  const ev = evaluate([leg("Passed"), leg("Failed")]);
  assert.equal(ev.verdict, "Failed");
  assert.equal(ev.exitCode, 1);
});

test("evaluate: NotRun is fail-closed — never a pass", () => {
  const ev = evaluate([leg("Passed"), leg("NotRun")]);
  assert.equal(ev.verdict, "NotRun");
  assert.equal(ev.exitCode, 1);
});

test("evaluate: Unsupported canonical leg fails closed too", () => {
  const ev = evaluate([leg("Passed"), leg("Unsupported")]);
  assert.equal(ev.verdict, "Unsupported");
  assert.equal(ev.exitCode, 1);
});

test("evaluate: informational legs never affect the verdict", () => {
  const ev = evaluate([leg("Passed"), leg("Unsupported", true)]);
  assert.equal(ev.verdict, "Passed");
  assert.equal(ev.exitCode, 0);
  assert.equal(canonicalLegs([leg("Passed"), leg("Unsupported", true)]).length, 1);
});

// -- run() fail-closed integration ------------------------------------------

test("run() with no tools and no artifacts records NotRun and exits nonzero", () => {
  const evidenceDir = mkdtempSync(join(tmpdir(), "rf630-ev-"));
  const report = run({
    clapValidator: null,
    vst3Validator: null,
    validatorDir: null,
    clapDll: join(evidenceDir, "missing-clap.dll"),
    vst3Dll: join(evidenceDir, "missing-vst3.dll"),
    build: false,
    evidenceDir,
  });
  assert.equal(report.evaluation.verdict, "NotRun");
  assert.equal(report.evaluation.exitCode, 1);
  assert.equal(report.legs.length, CLAP_PLUGINS.length + 1);
  for (const l of report.legs) assert.equal(l.status, "NotRun");
  const written = JSON.parse(readFileSync(join(evidenceDir, "run.json"), "utf8"));
  assert.equal(written.evaluation.verdict, "NotRun");
  assert.equal(written.schema, "rf630-abi-validator-run/1");
});

test("run() records tool + artifact digests when present", () => {
  const dir = mkdtempSync(join(tmpdir(), "rf630-digest-"));
  const clap = join(dir, "clap-validator.exe");
  const vst3 = join(dir, "vst3-validator.exe");
  const dll = join(dir, "clap.dll");
  writeFileSync(clap, "clap-bytes");
  writeFileSync(vst3, "vst3-bytes");
  writeFileSync(dll, "dll-bytes");
  // A validator that is not a real executable would fail to spawn —
  // this leg proves resolution + digest recording happen before any
  // spawn verdict, and a spawn failure is Failed (not silently Pass).
  const report = run({
    clapValidator: clap,
    vst3Validator: vst3,
    validatorDir: null,
    clapDll: dll,
    vst3Dll: join(dir, "missing-vst3.dll"),
    build: false,
    evidenceDir: dir,
  });
  assert.equal(report.tools.clapValidator.sha256.length, 64);
  assert.equal(report.tools.vst3Validator.sha256.length, 64);
  assert.equal(report.artifacts.clapDll.sha256.length, 64);
  assert.equal(report.artifacts.vst3Dll.missing, true);
  // CLAP legs attempted spawn → Failed (bad executable), never Pass.
  for (const l of report.legs.filter((x) => x.kind === "clap-validator")) {
    assert.notEqual(l.status, "Passed");
    assert.notEqual(l.status, "NotRun");
  }
  // VST3 artifact missing → NotRun even though the tool resolved.
  const vst3Leg = report.legs.find((x) => x.kind === "vst3-sdk-validator");
  assert.equal(vst3Leg.status, "NotRun");
});

// -- production CLAP leg (Issue #732) -------------------------------------

test("run() without --clap-production never emits a production leg", () => {
  const dir = mkdtempSync(join(tmpdir(), "rf732-noprod-"));
  const report = run({
    clapValidator: null,
    vst3Validator: null,
    validatorDir: null,
    clapDll: join(dir, "missing-clap.dll"),
    vst3Dll: join(dir, "missing-vst3.dll"),
    build: false,
    evidenceDir: dir,
  });
  assert.equal(report.legs.some((l) => l.plugin === "production_gain"), false);
  assert.equal(report.artifacts.clapProductionDll, null);
});

test("run() --clap-production records an honest NotRun leg for a missing artifact", () => {
  const dir = mkdtempSync(join(tmpdir(), "rf732-prod-"));
  const clap = join(dir, "clap-validator.exe");
  writeFileSync(clap, "fake");
  const report = run({
    clapValidator: clap,
    vst3Validator: null,
    validatorDir: null,
    clapDll: join(dir, "missing-clap.dll"),
    clapProduction: true,
    clapProductionDll: join(dir, "missing-prod.dll"),
    vst3Dll: join(dir, "missing-vst3.dll"),
    build: false,
    evidenceDir: dir,
  });
  const prod = report.legs.find((l) => l.plugin === "production_gain");
  assert.ok(prod);
  assert.equal(prod.status, "NotRun");
  assert.match(prod.details, /missing-prod\.dll/);
  assert.match(prod.details, /--clap-production-dll/);
  assert.equal(prod.pluginId, "org.ruviform.clap-gain-poc");
  assert.equal(report.artifacts.clapProductionDll.missing, true);
  // Fail-closed: a NotRun canonical leg keeps the verdict non-Passed.
  assert.notEqual(report.evaluation.verdict, "Passed");
});
