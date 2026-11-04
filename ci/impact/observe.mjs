#!/usr/bin/env node
// observe.mjs — Issue #346 (Issue #333/#326 rev 2026-11-05) non-blocking
// macOS observe report.
//
// Pure evaluation: expected lane set (impact decision) x actual `*-macos`
// job results (`needs` context) -> per-leg status row + informational
// verdict. Unlike gate.mjs this never gates a merge — the check itself
// exits 0. The JSON report + step summary are the artifact that
// release/Consumer acceptance gating binds to a candidate SHA.
//
// Usage mirrors gate.mjs:
//   NEEDS_JSON='{...}' node ci/impact/observe.mjs [--report <path>] ...
//
// Status vocabulary (per-OS row):
//   Pass           ran and succeeded (note "over-executed" when the lane
//                  was classified unaffected)
//   Fail           ran and failed, or an expected leg produced no usable
//                  result — real verification signal, surfaced for the
//                  release gate
//   NotRun         started/queued but produced no verdict (cancelled —
//                  queue starvation or manual cancel). Runner starvation
//                  is NEVER NotApplicable.
//   NotApplicable  classifier proved the parent lane unaffected and the
//                  leg was skipped/absent — the ONLY path to this status
//   Pending        expectation undetermined (classification error) and
//                  the leg produced no verdict — indeterminate, honest
//   Unsupported    reserved vocabulary; no current leg is unsupported
//
// Informational verdict: "attention" when any row is Fail or Pending, or
// an expected leg ended NotRun (an expected-but-unmeasured macOS leg is
// exactly what the release gate must not ignore); else "pass".

import { appendFileSync, readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import { LANE_IDS, OBSERVE_LANES, OBSERVE_LANE_IDS } from "./map.mjs";

function parseJsonText(text, what) {
  try {
    return JSON.parse(text);
  } catch (e) {
    throw new Error(`invalid JSON for ${what}: ${e.message}`);
  }
}

export function evaluateObserve({ needs, expected, legIds = OBSERVE_LANE_IDS }) {
  const impact = needs?.impact ?? null;
  const classificationOk = impact?.result === "success" && expected != null;

  const problems = [];
  if (!impact) problems.push("impact-job-missing");
  else if (impact.result !== "success") problems.push(`impact-job-${impact.result}`);
  else if (expected == null) problems.push("impact-expected-missing");

  const rows = legIds.map((jobId) => {
    const { lane, os } = OBSERVE_LANES[jobId] ?? { lane: jobId, os: "unknown" };
    const need = needs?.[jobId] ?? null;
    const result = need?.result ?? "missing";
    // expected stays undetermined (null) on classification error so rows
    // can become Pending instead of guessing a lane was required.
    const expectedLeg = classificationOk ? expected?.[lane] !== false : null;
    let status;
    let note = "";
    if (result === "success") {
      status = "Pass";
      if (expectedLeg === false) note = "over-executed";
      if (expectedLeg === null) note = "expectation-undetermined";
    } else if (result === "failure") {
      status = "Fail";
      if (expectedLeg === false) note = "not-expected-lane-failed";
      if (expectedLeg === null) note = "expectation-undetermined";
    } else if (result === "cancelled") {
      status = "NotRun";
      note = "cancelled (queue starvation / manual) — never NotApplicable";
    } else if (result === "skipped") {
      if (expectedLeg === false) {
        status = "NotApplicable";
        note = "classified-not-needed";
      } else if (expectedLeg === true) {
        status = "Fail";
        note = "expected-leg-skipped";
      } else {
        status = "Pending";
        note = "expectation-undetermined";
      }
    } else {
      // missing or unknown result (e.g. timed_out): honor fail-closed for
      // expected legs, NotApplicable only when proven unneeded.
      if (expectedLeg === true) {
        status = "Fail";
        note = `expected-leg-no-result:${result}`;
      } else if (expectedLeg === false) {
        status = "NotApplicable";
        note = `classified-not-needed (${result})`;
      } else {
        status = "Pending";
        note = `result-unknown:${result}`;
      }
    }
    return { job: jobId, lane, os, expected: expectedLeg, result, status, note };
  });

  const attention = rows.some(
    (r) =>
      r.status === "Fail" ||
      r.status === "Pending" ||
      (r.expected === true && r.status === "NotRun"),
  );
  return {
    verdict: attention ? "attention" : "pass",
    classificationOk,
    classificationProblems: problems,
    rows,
  };
}

function markdownTable(evalResult, meta) {
  const lines = [
    `## macos-report verdict: ${evalResult.verdict.toUpperCase()} (non-blocking)`,
    "",
    `- event: \`${meta.event ?? "?"}\`  base: \`${meta.base ?? "?"}\`  head: \`${meta.head ?? "?"}\``,
    `- classification: ${evalResult.classificationOk ? "ok" : `ERROR (${evalResult.classificationProblems.join(", ")})`}`,
    "- non-blocking by design (Issue #333/#326 rev 2026-11-05): statuses feed release/Consumer acceptance gating, never PR merge protection. Runner starvation is NotRun, never NotApplicable.",
    "",
    "| leg | lane | os | expected | result | status | note |",
    "| --- | --- | --- | --- | --- | --- | --- |",
    ...evalResult.rows.map(
      (r) =>
        `| ${r.job} | ${r.lane} | ${r.os} | ${r.expected ?? "?"} | ${r.result} | ${r.status} | ${r.note || "-"} |`,
    ),
    "",
  ];
  return lines.join("\n");
}

export function main(argv = process.argv, env = process.env) {
  const args = { needs: null, expected: null, summary: null, report: null, base: null, head: null, event: env.GITHUB_EVENT_NAME ?? null };
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    const next = () => argv[++i];
    if (a === "--needs") args.needs = next();
    else if (a === "--expected") args.expected = next();
    else if (a === "--summary") args.summary = next();
    else if (a === "--report") args.report = next();
    else if (a === "--base") args.base = next();
    else if (a === "--head") args.head = next();
    else if (a === "--event") args.event = next();
    else if (a === "--help" || a === "-h") {
      console.log("Usage: node ci/impact/observe.mjs [--needs <file>] [--expected <file>] [--summary <file>] [--report <file>] [--base <sha>] [--head <sha>] [--event <name>]  (needs JSON also read from $NEEDS_JSON; always exits 0)");
      process.exit(0);
    } else {
      console.error(`error: unknown argument ${a}`);
      process.exit(2);
    }
  }

  const needsText = args.needs != null ? readFileSync(args.needs, "utf8") : env.NEEDS_JSON;
  if (needsText == null) {
    console.error("error: no needs JSON (pass --needs <file> or set NEEDS_JSON)");
    process.exit(2);
  }
  const needs = parseJsonText(needsText, "needs");

  // Expected set precedence mirrors gate.mjs: explicit --expected file
  // (decision JSON or bare {lane: bool} map) > needs.impact.outputs.expected
  // > absent (undetermined -> Pending-capable rows).
  let expected = null;
  if (args.expected != null) {
    const parsed = parseJsonText(readFileSync(args.expected, "utf8"), "expected");
    expected = parsed.lanes
      ? Object.fromEntries(LANE_IDS.map((id) => [id, parsed.lanes[id]?.expected !== false]))
      : parsed;
  } else {
    const raw = needs?.impact?.outputs?.expected;
    if (typeof raw === "string" && raw.trim()) {
      const parsed = parseJsonText(raw, "needs.impact.outputs.expected");
      expected = Object.fromEntries(LANE_IDS.map((id) => [id, parsed[id] !== false]));
    } else if (raw && typeof raw === "object") {
      expected = Object.fromEntries(LANE_IDS.map((id) => [id, raw[id] !== false]));
    }
  }

  const result = evaluateObserve({ needs, expected });
  const meta = { base: args.base ?? needs?.impact?.outputs?.base ?? null, head: args.head ?? needs?.impact?.outputs?.head ?? null, event: args.event };

  const report = {
    schemaVersion: 1,
    kind: "macos-observe",
    event: meta.event,
    base: meta.base,
    head: meta.head,
    verdict: result.verdict,
    classificationOk: result.classificationOk,
    classificationProblems: result.classificationProblems,
    legs: result.rows,
  };
  if (args.report) writeFileSync(args.report, JSON.stringify(report, null, 2) + "\n");

  const table = markdownTable(result, meta);
  process.stdout.write(table + "\n");
  if (args.summary) appendFileSync(args.summary, table + "\n");
  else if (env.GITHUB_STEP_SUMMARY) appendFileSync(env.GITHUB_STEP_SUMMARY, table + "\n");

  // Non-blocking by definition: the check stays green; the recorded rows
  // carry the signal. Exit 0 unconditionally.
  process.exit(0);
}

const isMain =
  process.argv[1] != null &&
  import.meta.url === pathToFileURL(process.argv[1]).href;
if (isMain) {
  main();
}
