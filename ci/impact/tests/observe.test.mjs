// observe.test.mjs — Issue #346 (Issue #333/#326 rev 2026-11-05) macOS
// observe-report status tests. The report is a pure function of (expected
// set, *-macos needs results): every status in the vocabulary is pinned
// here so runner starvation can never be silently recorded as
// NotApplicable, and a real macOS failure always lands as Fail evidence
// for release/Consumer acceptance gating. Covers the agreed scenario
// matrix: (a) macOS long-queued/cancelled, (b) macOS failure,
// (d) all success, (e) unclassifiable — (c) Linux/Windows failure is the
// required-lane side pinned in gate.test.mjs.

import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import assert from "node:assert/strict";
import { LANE_IDS, OBSERVE_LANE_IDS } from "../map.mjs";
import { evaluateObserve } from "../observe.mjs";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");

const ALL_TRUE = Object.fromEntries(LANE_IDS.map((id) => [id, true]));
const ALL_FALSE = Object.fromEntries(LANE_IDS.map((id) => [id, false]));

function needsWith(result, legs = OBSERVE_LANE_IDS, expected = ALL_TRUE) {
  const needs = { impact: { result: "success", outputs: { expected: JSON.stringify(expected) } } };
  for (const id of legs) needs[id] = { result };
  return needs;
}

function rowOf(result, job) {
  return result.rows.find((r) => r.job === job);
}

test("(d) all observe legs succeed -> all Pass + verdict pass", () => {
  const r = evaluateObserve({ needs: needsWith("success"), expected: ALL_TRUE });
  assert.equal(r.verdict, "pass");
  assert.equal(r.classificationOk, true);
  for (const row of r.rows) assert.equal(row.status, "Pass");
});

test("(a) expected leg cancelled (queue starvation) -> NotRun, never NotApplicable, verdict attention", () => {
  const needs = needsWith("success");
  needs["ui-verify-macos"] = { result: "cancelled" };
  const r = evaluateObserve({ needs, expected: ALL_TRUE });
  const row = rowOf(r, "ui-verify-macos");
  assert.equal(row.status, "NotRun");
  assert.equal(r.verdict, "attention");
});

test("cancelled stays NotRun even when the lane was not expected", () => {
  const needs = needsWith("success", OBSERVE_LANE_IDS, { ...ALL_TRUE, "ui-verify": false });
  needs["ui-verify-macos"] = { result: "cancelled" };
  const r = evaluateObserve({ needs, expected: { ...ALL_TRUE, "ui-verify": false } });
  assert.equal(rowOf(r, "ui-verify-macos").status, "NotRun");
});

test("(b) expected leg failure -> Fail + verdict attention", () => {
  const needs = needsWith("success");
  needs["verify-macos"] = { result: "failure" };
  const r = evaluateObserve({ needs, expected: ALL_TRUE });
  assert.equal(rowOf(r, "verify-macos").status, "Fail");
  assert.equal(r.verdict, "attention");
});

test("not-expected leg skipped -> NotApplicable (the only path to it)", () => {
  const expected = { ...ALL_TRUE, "verify": false };
  const needs = needsWith("success", OBSERVE_LANE_IDS, expected);
  needs["verify-macos"] = { result: "skipped" };
  const r = evaluateObserve({ needs, expected });
  const row = rowOf(r, "verify-macos");
  assert.equal(row.status, "NotApplicable");
  assert.equal(row.note, "classified-not-needed");
  // a proven-unaffected skip is not an anomaly
  assert.equal(r.verdict, "pass");
});

test("expected leg skipped -> Fail (wiring anomaly, evidence kept)", () => {
  const needs = needsWith("success");
  needs["verify-macos"] = { result: "skipped" };
  const r = evaluateObserve({ needs, expected: ALL_TRUE });
  assert.equal(rowOf(r, "verify-macos").status, "Fail");
  assert.equal(r.verdict, "attention");
});

test("not-expected leg that ran and passed -> Pass (over-executed)", () => {
  const r = evaluateObserve({ needs: needsWith("success", OBSERVE_LANE_IDS, ALL_FALSE), expected: ALL_FALSE });
  for (const row of r.rows) {
    assert.equal(row.status, "Pass");
    assert.equal(row.note, "over-executed");
  }
  assert.equal(r.verdict, "pass");
});

test("not-expected leg that ran and FAILED -> Fail (false-negative signal)", () => {
  const needs = needsWith("success", OBSERVE_LANE_IDS, ALL_FALSE);
  needs["l1-real-verify-macos"] = { result: "failure" };
  const r = evaluateObserve({ needs, expected: ALL_FALSE });
  const row = rowOf(r, "l1-real-verify-macos");
  assert.equal(row.status, "Fail");
  assert.equal(row.note, "not-expected-lane-failed");
  assert.equal(r.verdict, "attention");
});

test("expected leg missing/unknown result -> Fail (expected evidence absent)", () => {
  const needs = needsWith("success");
  delete needs["verify-macos"];
  needs["ui-verify-macos"] = { result: "timed_out" };
  const r = evaluateObserve({ needs, expected: ALL_TRUE });
  assert.equal(rowOf(r, "verify-macos").status, "Fail");
  assert.equal(rowOf(r, "ui-verify-macos").status, "Fail");
  assert.equal(r.verdict, "attention");
});

test("(e) unclassifiable: impact failed -> expectation undetermined, unrun legs Pending", () => {
  const needs = needsWith("success");
  needs.impact = { result: "failure" };
  needs["verify-macos"] = { result: "skipped" };
  needs["ui-verify-macos"] = { result: "cancelled" };
  const r = evaluateObserve({ needs, expected: null });
  assert.equal(r.classificationOk, false);
  assert.ok(r.classificationProblems.some((p) => p.includes("impact")));
  // expectation undetermined: ran legs still honored, unrun legs Pending
  assert.equal(rowOf(r, "l1-real-verify-macos").status, "Pass");
  assert.equal(rowOf(r, "verify-macos").status, "Pending");
  assert.equal(rowOf(r, "ui-verify-macos").status, "NotRun");
  assert.equal(r.verdict, "attention");
});

test("(e) unclassifiable: impact success but expected absent -> skipped legs Pending, never NotApplicable", () => {
  const needs = needsWith("success");
  needs["verify-macos"] = { result: "skipped" };
  const r = evaluateObserve({ needs, expected: null });
  assert.equal(r.classificationOk, false);
  assert.ok(r.classificationProblems.includes("impact-expected-missing"));
  assert.equal(rowOf(r, "verify-macos").status, "Pending");
});

test("missing leg with undetermined expectation -> Pending", () => {
  const needs = { impact: { result: "failure" } };
  const r = evaluateObserve({ needs, expected: null });
  for (const row of r.rows) assert.equal(row.status, "Pending");
});

test("CLI: writes report+summary and exits 0 even on attention verdict (non-blocking)", () => {
  const dir = mkdtempSync(join(tmpdir(), "observe-"));
  const reportPath = join(dir, "macos-observe-report.json");
  const summaryPath = join(dir, "summary.md");
  const needs = needsWith("success");
  needs["verify-macos"] = { result: "failure" };
  const stdout = execFileSync(
    process.execPath,
    [join(ROOT, "ci/impact/observe.mjs"), "--report", reportPath, "--summary", summaryPath],
    {
      env: { ...process.env, NEEDS_JSON: JSON.stringify(needs), GITHUB_STEP_SUMMARY: "" },
      encoding: "utf8",
    },
  );
  // exit 0 reached (execFileSync throws on non-zero)
  assert.match(stdout, /macos-report verdict: ATTENTION/);
  const report = JSON.parse(readFileSync(reportPath, "utf8"));
  assert.equal(report.kind, "macos-observe");
  assert.equal(report.verdict, "attention");
  assert.match(readFileSync(summaryPath, "utf8"), /verify-macos \| verify \| macos-latest \| true \| failure \| Fail/);
});
