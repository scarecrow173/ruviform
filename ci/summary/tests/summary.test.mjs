// summary.test.mjs — Issue #366 Full CI ci-summary verdict contract tests.
// The summary gate is a pure function of the `needs` context (+ the
// expected_sha dispatch contract): every cell of the result matrix is
// pinned here so a skipped/unknown/missing lane can never be silently
// treated as pass.

import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import assert from "node:assert/strict";
import { REQUIRED_JOBS, evaluate } from "../summary.mjs";

const SCRIPT = join(dirname(fileURLToPath(import.meta.url)), "..", "summary.mjs");

function needsWith(result, jobs = REQUIRED_JOBS) {
  const needs = {};
  for (const id of jobs) needs[id] = { result };
  return needs;
}

function rowOf(result, name) {
  return result.rows.find((r) => r.name === name);
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

test("all lanes success -> pass", () => {
  const r = evaluate({ needs: needsWith("success"), headSha: "abc123" });
  assert.equal(r.verdict, "pass");
  assert.equal(r.reasons.length, 0);
  for (const row of r.rows) assert.equal(row.verdict, "Pass");
});

test("lane failure/cancelled/skipped -> fail (never pass)", () => {
  for (const bad of ["failure", "cancelled", "skipped"]) {
    const needs = needsWith("success");
    needs["ui-verify"] = { result: bad };
    const r = evaluate({ needs, headSha: "abc123" });
    assert.equal(r.verdict, "fail", bad);
    assert.equal(rowOf(r, "ui-verify").verdict, "Fail");
  }
});

test("unknown result string -> fail closed", () => {
  const needs = needsWith("success");
  needs["bench"] = { result: "timed_out" };
  const r = evaluate({ needs, headSha: "abc123" });
  assert.equal(r.verdict, "fail");
  assert.equal(rowOf(r, "bench").note, "job-result-unknown");
});

test("missing result field -> fail closed", () => {
  const needs = needsWith("success");
  needs["rt-verify"] = {};
  const r = evaluate({ needs, headSha: "abc123" });
  assert.equal(r.verdict, "fail");
  assert.equal(rowOf(r, "rt-verify").result, "unknown");
});

test("required lane absent from needs -> fail (not_run / renamed)", () => {
  const needs = needsWith("success");
  delete needs["verify-macos"];
  const r = evaluate({ needs, headSha: "abc123" });
  assert.equal(r.verdict, "fail");
  const row = rowOf(r, "verify-macos");
  assert.equal(row.result, "missing");
  assert.equal(row.verdict, "Fail");
});

test("extra needs entry is evaluated with the same success rule", () => {
  const needs = needsWith("success");
  needs["future-lane"] = { result: "success" };
  assert.equal(evaluate({ needs, headSha: "abc123" }).verdict, "pass");
  needs["future-lane"] = { result: "failure" };
  const r = evaluate({ needs, headSha: "abc123" });
  assert.equal(r.verdict, "fail");
  assert.equal(rowOf(r, "future-lane").declared, false);
});

test("expected_sha: empty/absent -> no check", () => {
  for (const expectedSha of [null, "", "   "]) {
    const r = evaluate({ needs: needsWith("success"), expectedSha, headSha: "abc123" });
    assert.equal(r.verdict, "pass", JSON.stringify(expectedSha));
    assert.equal(r.expectedSha, null);
  }
});

test("expected_sha matching head -> pass", () => {
  const r = evaluate({ needs: needsWith("success"), expectedSha: "abc123", headSha: "abc123" });
  assert.equal(r.verdict, "pass");
  assert.equal(r.expectedSha, "abc123");
});

test("expected_sha mismatch -> fail (different revision is not verified)", () => {
  const r = evaluate({ needs: needsWith("success"), expectedSha: "deadbeef", headSha: "abc123" });
  assert.equal(r.verdict, "fail");
  assert.ok(r.reasons.some((reason) => reason.startsWith("expected-sha-mismatch")));
});

test("expected_sha set but head unknown -> fail closed", () => {
  const r = evaluate({ needs: needsWith("success"), expectedSha: "abc123", headSha: null });
  assert.equal(r.verdict, "fail");
  assert.ok(r.reasons.some((reason) => reason.startsWith("head-sha-unknown")));
});

test("CLI end-to-end: pass writes report + summary and exits 0", () => {
  const dir = mkdtempSync(join(tmpdir(), "ci-summary-"));
  const needsPath = join(dir, "needs.json");
  const reportPath = join(dir, "report.json");
  const summaryPath = join(dir, "summary.md");
  writeFileSync(needsPath, JSON.stringify(needsWith("success")));
  const out = execFileSync(
    process.execPath,
    [SCRIPT, "--needs", needsPath, "--report", reportPath, "--summary", summaryPath, "--head", "abc123", "--event", "workflow_dispatch", "--run-url", "https://example.test/run/1"],
    { encoding: "utf8" },
  );
  assert.match(out, /ci-summary verdict: PASS/);
  const report = JSON.parse(readFileSync(reportPath, "utf8"));
  assert.equal(report.kind, "ci-summary");
  assert.equal(report.schemaVersion, 1);
  assert.equal(report.verdict, "pass");
  assert.equal(report.headSha, "abc123");
  assert.equal(report.event, "workflow_dispatch");
  assert.equal(report.runUrl, "https://example.test/run/1");
  assert.equal(report.jobs.length, REQUIRED_JOBS.length);
  const summary = readFileSync(summaryPath, "utf8");
  assert.match(summary, /ci-summary verdict: PASS/);
  assert.match(summary, /abc123/);
});

test("CLI end-to-end: failure exits 1 and lists fail reasons", () => {
  const dir = mkdtempSync(join(tmpdir(), "ci-summary-"));
  const needsPath = join(dir, "needs.json");
  const reportPath = join(dir, "report.json");
  const needs = needsWith("success");
  needs["verify-macos"] = { result: "cancelled" };
  writeFileSync(needsPath, JSON.stringify(needs));
  let code = 0;
  let out = "";
  try {
    out = execFileSync(
      process.execPath,
      [SCRIPT, "--needs", needsPath, "--report", reportPath, "--head", "abc123"],
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
