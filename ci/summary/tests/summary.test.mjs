// summary.test.mjs — Issue #366 Full CI ci-summary verdict contract tests.
// The summary gate is a pure function of the `needs` context (+ the
// expected_sha dispatch contract + measured checkout-sha evidence): every
// cell of the result matrix is pinned here so a skipped/unknown/missing
// lane — or a checkout that cannot be proven to be the expected revision —
// can never be silently treated as pass.

import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import assert from "node:assert/strict";
import { REQUIRED_CHECKOUT_LEGS, REQUIRED_JOBS, evaluate } from "../summary.mjs";

const SCRIPT = join(dirname(fileURLToPath(import.meta.url)), "..", "summary.mjs");

const SHA_A = "0123456789abcdef0123456789abcdef01234567";
const SHA_B = "89abcdef0123456789abcdef0123456789abcdef";

function needsWith(result, jobs = REQUIRED_JOBS) {
  const needs = {};
  for (const id of jobs) needs[id] = { result };
  return needs;
}

// A consistent run: every declared checkout leg measured the same sha at
// attempt 1 (single-attempt evidence, the common case).
function checkoutEvidenceWith(sha, legs = REQUIRED_CHECKOUT_LEGS, attempt = 1) {
  const legMap = {};
  for (const leg of legs) legMap[leg] = { sha, attempt, divergent: false };
  return { legs: legMap, malformed: [] };
}

function passingInput(overrides = {}) {
  return {
    needs: needsWith("success"),
    headSha: SHA_A,
    eventSha: SHA_A,
    checkoutEvidence: checkoutEvidenceWith(SHA_A),
    ...overrides,
  };
}

function rowOf(result, name) {
  return result.rows.find((r) => r.name === name);
}

function shaRowOf(result, name) {
  return result.shaRows.find((r) => r.name === name);
}

test("REQUIRED_JOBS covers every Full CI lane incl. macOS legs and io-structured", () => {
  for (const job of [
    "verify",
    "verify-macos",
    "rt-verify",
    "icc-verify",
    "compression-verify",
    "io-structured-verify",
    "ui-verify",
    "ui-verify-macos",
    "bench",
    "l1-real-verify",
    "l1-real-verify-macos",
  ]) {
    assert.ok(REQUIRED_JOBS.includes(job), job);
  }
});

test("REQUIRED_CHECKOUT_LEGS expands matrix lanes into per-OS legs", () => {
  for (const leg of [
    "verify-ubuntu-latest",
    "verify-windows-latest",
    "verify-macos",
    "rt-verify",
    "icc-verify",
    "compression-verify",
    "io-structured-verify",
    "ui-verify-ubuntu-latest",
    "ui-verify-windows-latest",
    "ui-verify-macos",
    "bench",
    "l1-real-verify-ubuntu-latest",
    "l1-real-verify-windows-latest",
    "l1-real-verify-macos",
  ]) {
    assert.ok(REQUIRED_CHECKOUT_LEGS.includes(leg), leg);
  }
});

test("all lanes success + consistent checkout evidence -> pass", () => {
  const r = evaluate(passingInput());
  assert.equal(r.verdict, "pass");
  assert.equal(r.reasons.length, 0);
  assert.equal(r.commonSha, SHA_A);
  for (const row of r.rows) assert.equal(row.verdict, "Pass");
  for (const row of r.shaRows) assert.equal(row.verdict, "Pass");
});

test("lane failure/cancelled/skipped -> fail (never pass)", () => {
  for (const bad of ["failure", "cancelled", "skipped"]) {
    const needs = needsWith("success");
    needs["ui-verify"] = { result: bad };
    const r = evaluate(passingInput({ needs }));
    assert.equal(r.verdict, "fail", bad);
    assert.equal(rowOf(r, "ui-verify").verdict, "Fail");
  }
});

test("unknown result string -> fail closed", () => {
  const needs = needsWith("success");
  needs["bench"] = { result: "timed_out" };
  const r = evaluate(passingInput({ needs }));
  assert.equal(r.verdict, "fail");
  assert.equal(rowOf(r, "bench").note, "job-result-unknown");
});

test("missing result field -> fail closed", () => {
  const needs = needsWith("success");
  needs["rt-verify"] = {};
  const r = evaluate(passingInput({ needs }));
  assert.equal(r.verdict, "fail");
  assert.equal(rowOf(r, "rt-verify").result, "unknown");
});

test("required lane absent from needs -> fail (not_run / renamed)", () => {
  const needs = needsWith("success");
  delete needs["verify-macos"];
  const r = evaluate(passingInput({ needs }));
  assert.equal(r.verdict, "fail");
  const row = rowOf(r, "verify-macos");
  assert.equal(row.result, "missing");
  assert.equal(row.verdict, "Fail");
});

test("extra needs entry is evaluated with the same success rule", () => {
  const needs = needsWith("success");
  needs["future-lane"] = { result: "success" };
  assert.equal(evaluate(passingInput({ needs })).verdict, "pass");
  needs["future-lane"] = { result: "failure" };
  const r = evaluate(passingInput({ needs }));
  assert.equal(r.verdict, "fail");
  assert.equal(rowOf(r, "future-lane").declared, false);
});

test("checkout-sha evidence missing for a declared leg -> fail closed", () => {
  const checkoutEvidence = checkoutEvidenceWith(SHA_A);
  delete checkoutEvidence.legs["verify-windows-latest"];
  const r = evaluate(passingInput({ checkoutEvidence }));
  assert.equal(r.verdict, "fail");
  const row = shaRowOf(r, "verify-windows-latest");
  assert.equal(row.verdict, "Fail");
  assert.equal(row.note, "checkout-sha-missing");
  assert.ok(r.reasons.some((reason) => reason.includes("verify-windows-latest:checkout-sha-missing")));
});

test("checkout-sha evidence absent entirely -> fail closed (not assumed)", () => {
  const r = evaluate(passingInput({ checkoutEvidence: null }));
  assert.equal(r.verdict, "fail");
  for (const leg of REQUIRED_CHECKOUT_LEGS) {
    assert.ok(r.reasons.includes(`${leg}:checkout-sha-missing`), leg);
  }
});

test("malformed checkout-sha file content -> fail closed", () => {
  const checkoutEvidence = checkoutEvidenceWith(SHA_A);
  checkoutEvidence.legs["bench"] = { sha: "not-a-sha", attempt: 1, divergent: false };
  const r = evaluate(passingInput({ checkoutEvidence }));
  assert.equal(r.verdict, "fail");
  assert.equal(shaRowOf(r, "bench").note, "checkout-sha-invalid");
});

test("one lane leg checked out a different revision -> fail", () => {
  const checkoutEvidence = checkoutEvidenceWith(SHA_A);
  checkoutEvidence.legs["ui-verify-macos"] = { sha: SHA_B, attempt: 1, divergent: false };
  const r = evaluate(passingInput({ checkoutEvidence }));
  assert.equal(r.verdict, "fail");
  assert.ok(r.reasons.some((reason) => reason.startsWith("lane-checkout-mismatch")));
});

test("re-run failed jobs: legs at mixed attempts with same sha -> pass, latest attempt selected", () => {
  // Rerunning only failed jobs leaves successful legs at attempt 1 while
  // the rerun leg uploads attempt 2 — needs.*.result reflects the latest
  // attempt of each job, so the evidence set is legitimately mixed.
  const checkoutEvidence = checkoutEvidenceWith(SHA_A);
  checkoutEvidence.legs["ui-verify-macos"] = { sha: SHA_A, attempt: 2, divergent: false };
  const r = evaluate(passingInput({ checkoutEvidence }));
  assert.equal(r.verdict, "pass");
  assert.equal(shaRowOf(r, "ui-verify-macos").attempt, 2);
});

test("re-run all jobs: every leg at attempt 2 with same sha -> pass", () => {
  const r = evaluate(passingInput({ checkoutEvidence: checkoutEvidenceWith(SHA_A, REQUIRED_CHECKOUT_LEGS, 2) }));
  assert.equal(r.verdict, "pass");
  for (const row of r.shaRows) assert.equal(row.attempt, 2);
});

test("same leg measured different checkouts across attempts -> fail (divergence)", () => {
  // Pinned checkout makes per-attempt divergence impossible in theory —
  // if it ever happens the leg's evidence cannot define the verified
  // revision, so the verdict must fail closed.
  const checkoutEvidence = checkoutEvidenceWith(SHA_A);
  checkoutEvidence.legs["bench"] = { sha: SHA_B, attempt: 2, divergent: true };
  const r = evaluate(passingInput({ checkoutEvidence }));
  assert.equal(r.verdict, "fail");
  const row = shaRowOf(r, "bench");
  assert.equal(row.note, "checkout-sha-attempt-divergence");
  assert.equal(row.sha, SHA_B);
  assert.ok(r.reasons.includes("bench:checkout-sha-attempt-divergence"));
});

test("evidence file that does not match the attempt naming scheme -> fail (malformed)", () => {
  // e.g. a legacy `checkout-sha-<leg>.txt` artifact — anything shipped via
  // the checkout-sha-* pattern that cannot be attributed to a leg+attempt
  // is unverifiable evidence and must not be silently ignored.
  const checkoutEvidence = checkoutEvidenceWith(SHA_A);
  checkoutEvidence.malformed.push("checkout-sha-verify-ubuntu-latest.txt");
  const r = evaluate(passingInput({ checkoutEvidence }));
  assert.equal(r.verdict, "fail");
  assert.ok(r.reasons.includes("checkout-sha-verify-ubuntu-latest.txt:checkout-sha-malformed"));
});

test("summary job checkout disagrees with lanes -> fail", () => {
  const r = evaluate(passingInput({ headSha: SHA_B }));
  assert.equal(r.verdict, "fail");
  assert.ok(r.reasons.some((reason) => reason.startsWith("summary-checkout-mismatch")));
});

test("event sha (github.sha) disagrees with measured checkouts -> fail", () => {
  const r = evaluate(passingInput({ eventSha: SHA_B }));
  assert.equal(r.verdict, "fail");
  assert.ok(r.reasons.some((reason) => reason.startsWith("event-checkout-mismatch")));
});

test("summary checkout sha unmeasured -> fail closed", () => {
  for (const headSha of [null, "", "not-a-sha"]) {
    const r = evaluate(passingInput({ headSha }));
    assert.equal(r.verdict, "fail", JSON.stringify(headSha));
    assert.ok(r.reasons.includes("ci-summary-checkout:checkout-sha-missing"));
  }
});

test("extra undeclared checkout leg is evaluated with the same rule", () => {
  const checkoutEvidence = checkoutEvidenceWith(SHA_A);
  checkoutEvidence.legs["future-leg"] = { sha: SHA_A, attempt: 1, divergent: false };
  const ok = evaluate(passingInput({ checkoutEvidence }));
  assert.equal(ok.verdict, "pass");
  assert.equal(shaRowOf(ok, "future-leg").declared, false);
  checkoutEvidence.legs["future-leg"] = { sha: SHA_B, attempt: 1, divergent: false };
  const bad = evaluate(passingInput({ checkoutEvidence }));
  assert.equal(bad.verdict, "fail");
});

test("expected_sha: empty/absent -> no check", () => {
  for (const expectedSha of [null, "", "   "]) {
    const r = evaluate(passingInput({ expectedSha }));
    assert.equal(r.verdict, "pass", JSON.stringify(expectedSha));
    assert.equal(r.expectedSha, null);
  }
});

test("expected_sha matching the measured common checkout -> pass", () => {
  const r = evaluate(passingInput({ expectedSha: SHA_A }));
  assert.equal(r.verdict, "pass");
  assert.equal(r.expectedSha, SHA_A);
});

test("expected_sha matching github.sha but not the measured checkout -> fail", () => {
  // Regression for the coordinator finding: the dispatch contract is about
  // what the lanes actually checked out, not the event sha. Here the run
  // was triggered for SHA_B but every lane measured SHA_A.
  const r = evaluate(passingInput({ expectedSha: SHA_B, eventSha: SHA_B }));
  assert.equal(r.verdict, "fail");
  assert.ok(r.reasons.some((reason) => reason.startsWith("expected-sha-mismatch")));
  assert.ok(r.reasons.some((reason) => reason.startsWith("event-checkout-mismatch")));
});

test("expected_sha differs from the measured common checkout -> fail", () => {
  const r = evaluate(passingInput({ expectedSha: SHA_B }));
  assert.equal(r.verdict, "fail");
  assert.ok(r.reasons.some((reason) => reason.startsWith("expected-sha-mismatch")));
});

test("expected_sha set but checkout evidence incomplete -> fail closed", () => {
  const r = evaluate(passingInput({ expectedSha: SHA_A, checkoutEvidence: null }));
  assert.equal(r.verdict, "fail");
  assert.ok(r.reasons.some((reason) => reason.startsWith("expected-sha-unverifiable")));
});

test("CLI end-to-end: pass writes report + summary and exits 0", () => {
  const dir = mkdtempSync(join(tmpdir(), "ci-summary-"));
  const needsPath = join(dir, "needs.json");
  const reportPath = join(dir, "report.json");
  const summaryPath = join(dir, "summary.md");
  const shaDir = join(dir, "checkout-shas");
  mkdirSync(shaDir);
  for (const leg of REQUIRED_CHECKOUT_LEGS) {
    writeFileSync(join(shaDir, `checkout-sha-${leg}-attempt-1.txt`), `${SHA_A}\n`);
  }
  writeFileSync(needsPath, JSON.stringify(needsWith("success")));
  const out = execFileSync(
    process.execPath,
    [SCRIPT, "--needs", needsPath, "--checkout-dir", shaDir, "--report", reportPath, "--summary", summaryPath, "--head", SHA_A, "--event-sha", SHA_A, "--event", "workflow_dispatch", "--run-url", "https://example.test/run/1", "--workflow-ref", "owner/repo/.github/workflows/ci.yml@refs/heads/main", "--workflow-sha", SHA_A],
    { encoding: "utf8" },
  );
  assert.match(out, /ci-summary verdict: PASS/);
  const report = JSON.parse(readFileSync(reportPath, "utf8"));
  assert.equal(report.kind, "ci-summary");
  assert.equal(report.schemaVersion, 2);
  assert.equal(report.verdict, "pass");
  assert.equal(report.headSha, SHA_A);
  assert.equal(report.eventSha, SHA_A);
  assert.equal(report.commonSha, SHA_A);
  assert.equal(report.workflowRef, "owner/repo/.github/workflows/ci.yml@refs/heads/main");
  assert.equal(report.workflowSha, SHA_A);
  assert.equal(report.event, "workflow_dispatch");
  assert.equal(report.runUrl, "https://example.test/run/1");
  assert.equal(report.jobs.length, REQUIRED_JOBS.length);
  assert.equal(report.checkoutShas.length, REQUIRED_CHECKOUT_LEGS.length);
  const summary = readFileSync(summaryPath, "utf8");
  assert.match(summary, /ci-summary verdict: PASS/);
  assert.match(summary, new RegExp(SHA_A));
});

test("CLI end-to-end: failure exits 1 and lists fail reasons", () => {
  const dir = mkdtempSync(join(tmpdir(), "ci-summary-"));
  const needsPath = join(dir, "needs.json");
  const reportPath = join(dir, "report.json");
  const shaDir = join(dir, "checkout-shas");
  mkdirSync(shaDir);
  for (const leg of REQUIRED_CHECKOUT_LEGS) {
    writeFileSync(join(shaDir, `checkout-sha-${leg}-attempt-1.txt`), `${SHA_A}\n`);
  }
  const needs = needsWith("success");
  needs["verify-macos"] = { result: "cancelled" };
  writeFileSync(needsPath, JSON.stringify(needs));
  let code = 0;
  let out = "";
  try {
    out = execFileSync(
      process.execPath,
      [SCRIPT, "--needs", needsPath, "--checkout-dir", shaDir, "--report", reportPath, "--head", SHA_A, "--event-sha", SHA_A],
      { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
    );
  } catch (e) {
    code = e.status;
    out = e.stdout;
  }
  assert.equal(code, 1);
  assert.match(out, /ci-summary verdict: FAIL/);
  assert.match(out, /verify-macos/);
  const report = JSON.parse(readFileSync(reportPath, "utf8"));
  assert.equal(report.verdict, "fail");
  assert.ok(report.reasons.some((reason) => reason.includes("verify-macos")));
});

test("CLI end-to-end: rerun evidence — latest attempt per leg is selected, legacy names fail closed", () => {
  // Mixed-attempt evidence (as produced by "re-run failed jobs"): one leg
  // re-ran at attempt 2 while others stayed at attempt 1. A leftover file
  // that does not carry the attempt suffix must fail closed rather than
  // be silently ignored or ambiguously overwrite a newer measurement.
  const dir = mkdtempSync(join(tmpdir(), "ci-summary-"));
  const needsPath = join(dir, "needs.json");
  const reportPath = join(dir, "report.json");
  const shaDir = join(dir, "checkout-shas");
  mkdirSync(shaDir);
  for (const leg of REQUIRED_CHECKOUT_LEGS) {
    writeFileSync(join(shaDir, `checkout-sha-${leg}-attempt-1.txt`), `${SHA_A}\n`);
  }
  writeFileSync(join(shaDir, "checkout-sha-rt-verify-attempt-2.txt"), `${SHA_A}\n`);
  writeFileSync(needsPath, JSON.stringify(needsWith("success")));
  const out = execFileSync(
    process.execPath,
    [SCRIPT, "--needs", needsPath, "--checkout-dir", shaDir, "--report", reportPath, "--head", SHA_A, "--event-sha", SHA_A],
    { encoding: "utf8" },
  );
  assert.match(out, /ci-summary verdict: PASS/);
  const report = JSON.parse(readFileSync(reportPath, "utf8"));
  assert.equal(report.verdict, "pass");
  assert.equal(report.checkoutShas.find((r) => r.name === "rt-verify").attempt, 2);
  assert.match(out, /\| rt-verify \| true \| 2 \|/);

  // Same leg diverging across attempts fails closed (unprovable revision).
  writeFileSync(join(shaDir, "checkout-sha-rt-verify-attempt-3.txt"), `${SHA_B}\n`);
  let code = 0;
  let failOut = "";
  try {
    failOut = execFileSync(
      process.execPath,
      [SCRIPT, "--needs", needsPath, "--checkout-dir", shaDir, "--head", SHA_A, "--event-sha", SHA_A],
      { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
    );
  } catch (e) {
    code = e.status;
    failOut = e.stdout;
  }
  assert.equal(code, 1);
  assert.match(failOut, /checkout-sha-attempt-divergence/);
});

test("CLI end-to-end: missing checkout dir -> fail closed, not crash", () => {
  const dir = mkdtempSync(join(tmpdir(), "ci-summary-"));
  const needsPath = join(dir, "needs.json");
  writeFileSync(needsPath, JSON.stringify(needsWith("success")));
  let code = 0;
  let out = "";
  try {
    out = execFileSync(
      process.execPath,
      [SCRIPT, "--needs", needsPath, "--checkout-dir", join(dir, "no-such-dir"), "--head", SHA_A, "--event-sha", SHA_A],
      { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
    );
  } catch (e) {
    code = e.status;
    out = e.stdout;
  }
  assert.equal(code, 1);
  assert.match(out, /checkout-sha-missing/);
});

test("CLI: missing needs JSON input -> exit 2", () => {
  let code = 0;
  try {
    execFileSync(process.execPath, [SCRIPT], { encoding: "utf8", env: { ...process.env, NEEDS_JSON: undefined }, stdio: ["ignore", "pipe", "pipe"] });
  } catch (e) {
    code = e.status;
  }
  assert.equal(code, 2);
});

test("CLI: malformed needs JSON -> nonzero exit", () => {
  const dir = mkdtempSync(join(tmpdir(), "ci-summary-"));
  const needsPath = join(dir, "needs.json");
  writeFileSync(needsPath, "{ not json");
  let code = 0;
  try {
    execFileSync(process.execPath, [SCRIPT, "--needs", needsPath], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
  } catch (e) {
    code = e.status;
  }
  assert.notEqual(code, 0);
});
