#!/usr/bin/env node
// gate.mjs — Issue #333 (CI-OPT-06) aggregated fail-closed required gate.
//
// Pure evaluation: expected lane set (impact decision) x actual job results
// (`needs` context) -> per-lane verdict + overall pass/fail. The same code
// is correct in shadow mode (all lanes run; a false/failure row surfaces a
// false-negative signal) and in the later enforcement stage (skipped
// not-expected lanes become NotApplicable).
//
// Usage:
//   NEEDS_JSON='{...}' node ci/impact/gate.mjs [--expected <decision.json|expected.json>]
//       [--summary <path>] [--report <path>] [--base <sha>] [--head <sha>]
// or
//   node ci/impact/gate.mjs --needs <needs.json> ...
//
// Verdict table (docs/designs/issue-333-impact-based-ci.md):
//   expected=true  + success            -> Pass
//   expected=true  + failure|cancelled  -> Fail
//   expected=true  + skipped            -> Fail (expected lane never ran)
//   expected=false + skipped            -> NotApplicable
//   expected=false + success            -> Pass (over-executed; audited)
//   expected=false + failure|cancelled  -> Fail (executed lane results are
//                                           always honored — shadow-mode
//                                           false-negative signal)
//   needs.<lane> missing or result
//     unknown                           -> Fail (fail-closed)
//   impact job failed / expected absent -> classification-error:
//                                          expected=all + overall Fail

import { appendFileSync, readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import { LANE_IDS, LANES } from "./map.mjs";

function parseJsonText(text, what) {
  try {
    return JSON.parse(text);
  } catch (e) {
    throw new Error(`invalid JSON for ${what}: ${e.message}`);
  }
}

export function evaluate({ needs, expected, laneIds = LANE_IDS }) {
  const impact = needs?.impact ?? null;
  const impactOk = impact?.result === "success" && expected != null;

  // classification-error: expectation cannot be proven -> degrade to
  // "everything expected" for the row table and fail overall.
  const expMap = {};
  for (const id of laneIds) {
    // Missing expected info for a lane means it cannot be proven
    // unaffected -> it was expected (fail-closed default true).
    expMap[id] = expected?.[id] !== false;
  }

  const rows = laneIds.map((lane) => {
    const need = needs?.[lane] ?? null;
    const result = need?.result ?? "missing";
    const expectedLane = expMap[lane];
    let verdict;
    let note;
    if (!["success", "failure", "cancelled", "skipped"].includes(result)) {
      verdict = "Fail";
      note = `lane-result-unknown:${result}`;
    } else if (expectedLane && result === "success") {
      verdict = "Pass";
      note = "";
    } else if (expectedLane && (result === "failure" || result === "cancelled")) {
      verdict = "Fail";
      note = `expected-lane-${result}`;
    } else if (expectedLane && result === "skipped") {
      verdict = "Fail";
      note = "expected-lane-skipped";
    } else if (!expectedLane && result === "skipped") {
      verdict = "NotApplicable";
      note = "classified-not-needed";
    } else if (!expectedLane && result === "success") {
      verdict = "Pass";
      note = "over-executed";
    } else {
      // !expected && (failure | cancelled): the lane ran and failed — the
      // classifier marked it unaffected yet it produced a real failure
      // signal. In shadow mode this IS the false-negative detector.
      verdict = "Fail";
      note = `not-expected-lane-${result}`;
    }
    return {
      lane,
      oses: LANES[lane]?.oses ?? [],
      contexts: LANES[lane]?.contexts ?? [lane],
      expected: expectedLane,
      result,
      verdict,
      note,
    };
  });

  const problems = [];
  if (!impact) problems.push("impact-job-missing");
  else if (impact.result !== "success") problems.push(`impact-job-${impact.result}`);
  else if (expected == null) problems.push("impact-expected-missing");

  const fail = rows.some((r) => r.verdict === "Fail") || problems.length > 0;
  return {
    verdict: fail ? "fail" : "pass",
    classificationOk: impactOk,
    classificationProblems: problems,
    rows,
  };
}

function markdownTable(evalResult, meta) {
  const lines = [
    `## ci-gate verdict: ${evalResult.verdict.toUpperCase()}`,
    "",
    `- event: \`${meta.event ?? "?"}\`  base: \`${meta.base ?? "?"}\`  head: \`${meta.head ?? "?"}\``,
    `- classification: ${evalResult.classificationOk ? "ok" : `ERROR (${evalResult.classificationProblems.join(", ")})`}`,
    "",
    "| lane | expected | result | verdict | note |",
    "| --- | --- | --- | --- | --- |",
    ...evalResult.rows.map(
      (r) => `| ${r.lane} | ${r.expected} | ${r.result} | ${r.verdict} | ${r.note || "-"} |`,
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
      console.log("Usage: node ci/impact/gate.mjs [--needs <file>] [--expected <file>] [--summary <file>] [--report <file>] [--base <sha>] [--head <sha>] [--event <name>]  (needs JSON also read from $NEEDS_JSON)");
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

  // Expected set precedence: explicit --expected file (decision JSON or a
  // bare {lane: bool} map) > needs.impact.outputs.expected > absent.
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

  const result = evaluate({ needs, expected });
  const meta = { base: args.base ?? needs?.impact?.outputs?.base ?? null, head: args.head ?? needs?.impact?.outputs?.head ?? null, event: args.event };

  const report = {
    schemaVersion: 1,
    event: meta.event,
    base: meta.base,
    head: meta.head,
    verdict: result.verdict,
    classificationOk: result.classificationOk,
    classificationProblems: result.classificationProblems,
    lanes: result.rows,
  };
  if (args.report) writeFileSync(args.report, JSON.stringify(report, null, 2) + "\n");

  const table = markdownTable(result, meta);
  process.stdout.write(table + "\n");
  if (args.summary) appendFileSync(args.summary, table + "\n");
  else if (env.GITHUB_STEP_SUMMARY) appendFileSync(env.GITHUB_STEP_SUMMARY, table + "\n");

  process.exit(result.verdict === "fail" ? 1 : 0);
}

const isMain =
  process.argv[1] != null &&
  import.meta.url === pathToFileURL(process.argv[1]).href;
if (isMain) {
  main();
}
