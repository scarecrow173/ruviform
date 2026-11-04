#!/usr/bin/env node
// classify.mjs — Issue #333 (CI-OPT-06) impact classifier CLI.
//
// Computes the expected verification-lane set for a change and writes a
// decision JSON (artifact + GITHUB_OUTPUT). Shadow mode: nothing consumes
// the decision for skipping — it is evidence for ci-gate and the Stage B
// false-negative observation window.
//
// Usage:
//   node ci/impact/classify.mjs [--event <name>] [--base <sha>] [--head <sha>]
//                               [--files <path>] [--all] [--out <path>]
//                               [--repo-root <dir>]
//
// Canonical contract: docs/designs/issue-333-impact-based-ci.md

import { appendFileSync, readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import { FULL_EVENTS, LANE_IDS } from "./map.mjs";
import {
  buildContext,
  classifyPath,
  gitDiffFiles,
  resolveLanes,
  toPosixPath,
} from "./lib.mjs";

function parseArgs(argv) {
  const args = { event: "local", base: null, head: null, files: null, all: false, out: null, repoRoot: "." };
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    const next = () => argv[++i];
    if (a === "--event") args.event = next();
    else if (a === "--base") args.base = next();
    else if (a === "--head") args.head = next();
    else if (a === "--files") args.files = next();
    else if (a === "--all") args.all = true;
    else if (a === "--out") args.out = next();
    else if (a === "--repo-root") args.repoRoot = next();
    else if (a === "--help" || a === "-h") {
      console.log("Usage: node ci/impact/classify.mjs [--event <name>] [--base <sha>] [--head <sha>] [--files <path>] [--all] [--out <path>] [--repo-root <dir>]");
      process.exit(0);
    } else {
      console.error(`error: unknown argument ${a}`);
      process.exit(2);
    }
  }
  return args;
}

function changedFiles(args) {
  if (args.all) return { files: [], source: "all" };
  if (args.files != null) {
    const files = readFileSync(args.files, "utf8")
      .split("\n")
      .map((l) => l.trim())
      .filter(Boolean)
      .map(toPosixPath);
    return { files, source: "files-arg" };
  }
  if (args.base && args.head) {
    return { files: gitDiffFiles(args.repoRoot, args.base, args.head), source: "git-diff" };
  }
  return { files: [], source: "none" };
}

export function main(argv = process.argv, env = process.env) {
  const args = parseArgs(argv);
  const impactDisabled = env.CI_IMPACT_DISABLE === "1";
  const ctx = buildContext(args.repoRoot);

  const { files, source } = changedFiles(args);
  const classifiedFiles = files.map((path) => ({ path, ...classifyPath(path, ctx) }));
  const resolved = resolveLanes(classifiedFiles, ctx);

  const decisionReasons = [...resolved.fullReasons];
  let decision = resolved.decision;
  if (FULL_EVENTS.has(args.event)) {
    decision = "full";
    decisionReasons.push(`event:${args.event}`);
  }
  if (args.all) {
    decision = "full";
    decisionReasons.push("explicit-all");
  }
  if (files.length === 0 && source !== "all") {
    decision = "full";
    decisionReasons.push(source === "none" ? "no-diff-provided" : "empty-diff");
  }
  if (impactDisabled) {
    decision = "full";
    decisionReasons.push("impact-disabled:CI_IMPACT_DISABLE=1");
  }
  for (const p of ctx.problems) decisionReasons.push(`context-problem:${p}`);
  if (decision === "full") {
    for (const id of LANE_IDS) resolved.lanes[id].expected = true;
  }

  const output = {
    schemaVersion: 1,
    event: args.event,
    base: args.base,
    head: args.head,
    decision,
    decisionReasons,
    impactDisabled,
    fileSource: source,
    files: classifiedFiles.map((f) => ({ path: f.path, classes: f.classes })),
    cratesTouched: [...(resolved.touched ?? [])].sort(),
    cratesImpacted: [...(resolved.impacted ?? [])].sort(),
    uiPackagesTouched: [...(resolved.uiTouched ?? [])].sort(),
    lanes: resolved.lanes,
  };

  const json = JSON.stringify(output, null, 2);
  if (args.out) writeFileSync(args.out, json + "\n");
  process.stdout.write(json + "\n");

  // Job outputs consumed by ci-gate (and later by lane `if:` conditions in
  // the enforcement stage). Per-lane keys use underscores — hyphens are not
  // valid in GitHub expression dot access.
  const ghOut = env.GITHUB_OUTPUT;
  if (ghOut) {
    const expected = Object.fromEntries(LANE_IDS.map((id) => [id, output.lanes[id].expected]));
    const lines = [
      `decision=${output.decision}`,
      `expected=${JSON.stringify(expected)}`,
      ...LANE_IDS.map((id) => `lane_${id.replaceAll("-", "_")}=${output.lanes[id].expected}`),
    ];
    appendFileSync(ghOut, lines.join("\n") + "\n");
  }

  if (env.GITHUB_STEP_SUMMARY) {
    const rows = LANE_IDS.map((id) => {
      const l = output.lanes[id];
      return `| ${id} | ${l.expected} | ${l.oses.join(", ")} | ${l.features.join(", ") || "-"} | ${l.reasons.join("; ") || "-"} |`;
    });
    appendFileSync(
      env.GITHUB_STEP_SUMMARY,
      [
        `## Impact classification (${output.decision})`,
        "",
        `- event: \`${output.event}\`  base: \`${output.base}\`  head: \`${output.head}\``,
        `- files: ${output.files.length}  crates touched: ${output.cratesTouched.join(", ") || "-"}  impacted: ${output.cratesImpacted.join(", ") || "-"}`,
        `- reasons: ${output.decisionReasons.join("; ") || "-"}`,
        "",
        "| lane | expected | oses | features | reasons |",
        "| --- | --- | --- | --- | --- |",
        ...rows,
        "",
      ].join("\n"),
    );
  }

  return output;
}

const isMain =
  process.argv[1] != null &&
  import.meta.url === pathToFileURL(process.argv[1]).href;
if (isMain) {
  main();
}
