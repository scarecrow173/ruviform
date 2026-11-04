// gate.test.mjs — Issue #333 (CI-OPT-06) aggregated gate verdict tests.
// The gate is a pure function of (expected set, needs results): every cell
// of the expected x actual matrix is pinned here so a skipped/unknown lane
// can never be silently treated as pass.

import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import assert from "node:assert/strict";
import { LANE_IDS } from "../map.mjs";
import { evaluate } from "../gate.mjs";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");

const ALL_TRUE = Object.fromEntries(LANE_IDS.map((id) => [id, true]));
const ALL_FALSE = Object.fromEntries(LANE_IDS.map((id) => [id, false]));

function needsWith(result, lanes = LANE_IDS) {
  const needs = { impact: { result: "success", outputs: { expected: JSON.stringify(ALL_TRUE) } } };
  for (const id of lanes) needs[id] = { result };
  return needs;
}

function rowOf(result, lane) {
  return result.rows.find((r) => r.lane === lane);
}

test("all expected lanes succeed -> pass", () => {
  const r = evaluate({ needs: needsWith("success"), expected: ALL_TRUE });
  assert.equal(r.verdict, "pass");
  assert.equal(r.classificationOk, true);
  for (const row of r.rows) assert.equal(row.verdict, "Pass");
});

test("expected lane failure/cancelled/skipped -> fail", () => {
  for (const bad of ["failure", "cancelled", "skipped"]) {
    const needs = needsWith("success");
    needs["rt-verify"] = { result: bad };
    const r = evaluate({ needs, expected: ALL_TRUE });
    assert.equal(r.verdict, "fail", bad);
    assert.equal(rowOf(r, "rt-verify").verdict, "Fail");
  }
});

test("expected-lane skipped is NEVER pass (required semantics)", () => {
  const needs = needsWith("skipped");
  const r = evaluate({ needs, expected: ALL_TRUE });
  assert.equal(r.verdict, "fail");
  for (const row of r.rows) assert.equal(row.verdict, "Fail");
});

test("not-expected lane skipped -> NotApplicable", () => {
  const needs = needsWith("success");
  needs["icc-verify"] = { result: "skipped" };
  const r = evaluate({ needs, expected: { ...ALL_TRUE, "icc-verify": false } });
  assert.equal(r.verdict, "pass");
  assert.equal(rowOf(r, "icc-verify").verdict, "NotApplicable");
});

test("not-expected lane that ran and passed -> Pass (over-executed)", () => {
  const r = evaluate({ needs: needsWith("success"), expected: ALL_FALSE });
  assert.equal(r.verdict, "pass");
  for (const row of r.rows) {
    assert.equal(row.verdict, "Pass");
    assert.equal(row.note, "over-executed");
  }
});

test("not-expected lane that ran and FAILED -> Fail (false-negative signal)", () => {
  const needs = needsWith("success");
  needs["bench"] = { result: "failure" };
  const r = evaluate({ needs, expected: { ...ALL_TRUE, bench: false } });
  assert.equal(r.verdict, "fail");
  const row = rowOf(r, "bench");
  assert.equal(row.verdict, "Fail");
  assert.match(row.note, /not-expected-lane-failure/);
});

test("not-expected lane cancelled -> Fail", () => {
  const needs = needsWith("success");
  needs["ui-verify"] = { result: "cancelled" };
  const r = evaluate({ needs, expected: { ...ALL_TRUE, "ui-verify": false } });
  assert.equal(r.verdict, "fail");
});

test("missing lane result -> Fail (fail-closed)", () => {
  const needs = needsWith("success");
  delete needs["bench"];
  const r = evaluate({ needs, expected: ALL_TRUE });
  assert.equal(r.verdict, "fail");
  assert.equal(rowOf(r, "bench").verdict, "Fail");
});

test("unrecognized lane result -> Fail", () => {
  const needs = needsWith("success");
  needs["verify"] = { result: "timed_out" };
  const r = evaluate({ needs, expected: ALL_TRUE });
  assert.equal(r.verdict, "fail");
  assert.match(rowOf(r, "verify").note, /lane-result-unknown/);
});

test("impact job failed -> classification error + fail", () => {
  const needs = needsWith("success");
  needs.impact = { result: "failure" };
  const r = evaluate({ needs, expected: ALL_TRUE });
  assert.equal(r.verdict, "fail");
  assert.equal(r.classificationOk, false);
  assert.ok(r.classificationProblems.some((p) => p.includes("impact")));
});

test("impact success but expected output absent -> classification error + fail", () => {
  const needs = needsWith("success");
  const r = evaluate({ needs, expected: null });
  assert.equal(r.verdict, "fail");
  assert.equal(r.classificationOk, false);
  assert.ok(r.classificationProblems.includes("impact-expected-missing"));
});

test("expected map missing a lane -> lane treated as expected (fail-closed)", () => {
  const partial = { verify: true }; // other lanes absent -> expected
  const needs = needsWith("success");
  needs["bench"] = { result: "skipped" };
  const r = evaluate({ needs, expected: partial });
  assert.equal(r.verdict, "fail");
  assert.equal(rowOf(r, "bench").expected, true);
});

test("Issue #333/#326 rev: *-macos observe legs never affect the gate (scenarios a/b pass)", () => {
  for (const bad of ["cancelled", "failure", "skipped"]) {
    const needs = needsWith("success");
    needs["verify-macos"] = { result: bad };
    needs["ui-verify-macos"] = { result: bad };
    needs["l1-real-verify-macos"] = { result: bad };
    const r = evaluate({ needs, expected: ALL_TRUE });
    assert.equal(r.verdict, "pass", `macOS leg ${bad} must not fail the required gate`);
    assert.ok(r.rows.every((row) => !row.lane.endsWith("-macos")), "observe legs must not appear in gate rows");
  }
});

test("required Linux/Windows leg failure still fails the gate (scenario c)", () => {
  const needs = needsWith("success");
  needs["verify"] = { result: "failure" };
  const r = evaluate({ needs, expected: ALL_TRUE });
  assert.equal(r.verdict, "fail");
});

test("CLI: reads NEEDS_JSON env, writes report+summary, exit code mirrors verdict", () => {
  const dir = mkdtempSync(join(tmpdir(), "gate-"));
  const reportPath = join(dir, "gate-report.json");
  const summaryPath = join(dir, "summary.md");
  const needs = needsWith("success");
  needs["icc-verify"] = { result: "skipped" };
  const env = {
    ...process.env,
    NEEDS_JSON: JSON.stringify(needs),
    GITHUB_STEP_SUMMARY: summaryPath,
  };
  let code = 0;
  let stdout = "";
  try {
    stdout = execFileSync(
      process.execPath,
      [join(ROOT, "ci/impact/gate.mjs"), "--report", reportPath],
      { env, encoding: "utf8" },
    );
  } catch (e) {
    code = e.status;
    stdout = e.stdout ?? "";
  }
  // icc skipped with expected(from needs.impact.outputs.expected)=ALL true
  // -> expected+skipped -> Fail.
  assert.equal(code, 1);
  const report = JSON.parse(readFileSync(reportPath, "utf8"));
  assert.equal(report.verdict, "fail");
  const summary = readFileSync(summaryPath, "utf8");
  assert.match(summary, /ci-gate verdict: FAIL/);
  assert.match(summary, /icc-verify \| true \| skipped \| Fail/);
});

test("CLI: pass exits 0", () => {
  const stdout = execFileSync(
    process.execPath,
    [join(ROOT, "ci/impact/gate.mjs")],
    { env: { ...process.env, NEEDS_JSON: JSON.stringify(needsWith("success")) }, encoding: "utf8" },
  );
  assert.match(stdout, /ci-gate verdict: PASS/);
});
