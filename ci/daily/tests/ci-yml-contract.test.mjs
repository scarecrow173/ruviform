// Issue #326 / #365 rule 1 contract test: after the PR/push CI retirement,
// NO workflow file in .github/workflows/ may declare pull_request,
// pull_request_target, or push triggers — a PR or main push must never
// start the heavy Full CI or any PR-auxiliary action again. This guard is
// deliberately textual (stdlib only, no yaml dep): it extracts each file's
// `on:` block and inspects ONLY the direct children of `on` — map keys
// (`  push:`) and sequence items (`  - push`) at the first indented level —
// plus any scalar/list/flow value on the `on:` line itself. Nested keys
// (e.g. a workflow_dispatch input named `push`) are never treated as
// triggers, and comment lines/tails are stripped so a `# pull_request:`
// mention can neither satisfy nor violate the contract.
//
// It also guards the contracts that must SURVIVE the removal:
//   - ci.yml's direct-child triggers are exactly {workflow_dispatch} —
//     an allowlist, so `schedule:`/`merge_group:`/`workflow_call:` or any
//     other trigger that would bypass the daily decide+dispatch contract
//     fails loudly, not just the three retired names
//   - ci.yml keeps the `expected_sha` input (daily-ci's dispatch contract)
//   - ci-issue-tracker.yml keeps `workflow_run` (the event entry)

import { test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const WORKFLOWS_DIR = fileURLToPath(
  new URL("../../../.github/workflows/", import.meta.url),
);

function workflowFiles() {
  return readdirSync(WORKFLOWS_DIR)
    .filter((f) => f.endsWith(".yml") || f.endsWith(".yaml"))
    .sort();
}

function readWorkflow(name) {
  return readFileSync(join(WORKFLOWS_DIR, name), "utf8").replace(/^﻿/, ""); // strip UTF-8 BOM
}

const stripComment = (line) => line.replace(/(^|\s)#.*$/, "");
const unquote = (s) => s.trim().replace(/^['"]|['"]$/g, "");

// Locate the top-level `on:` key (bare, single-, or double-quoted) and the
// exclusive end of its block: the next top-level key. Returns the block's
// raw lines, the inline value on the `on:` line, or null when absent.
export function onBlock(yamlText) {
  const lines = yamlText.split("\n");
  const start = lines.findIndex((l) => /^\s*['"]?on['"]?\s*:/.test(l));
  if (start < 0) return null;
  const end = lines.findIndex(
    (l, i) =>
      i > start && /^['"]?[A-Za-z][\w-]*['"]?\s*:/.test(l) && !/^\s/.test(l),
  );
  return {
    first: stripComment(lines[start]),
    body: lines.slice(start + 1, end < 0 ? lines.length : end),
  };
}

// Direct children of `on`: either map keys `  <name>:` or block-sequence
// items `  - <name>` at the minimum indentation present in the block
// (deeper lines — branch filters, input declarations — are never children).
// The `on:` line's own inline value covers `on: push`, `on: [push, fork]`,
// `on: {push: …}`, and quoted scalars like `on: 'push'`.
// Split a flow collection's contents on top-level commas only — nested
// `[`/`{`/`(` groups stay inside their item so inner keys are never
// mistaken for triggers.
function topLevelItems(inner) {
  const parts = [];
  let depth = 0;
  let seg = "";
  for (const ch of inner) {
    if ("[{(".includes(ch)) depth++;
    if ("}])".includes(ch)) depth--;
    if (ch === "," && depth === 0) {
      parts.push(seg);
      seg = "";
      continue;
    }
    seg += ch;
  }
  if (seg.trim()) parts.push(seg);
  return parts;
}

const KEY_RE = /^\s*([\w-]+|'[^']+'|"[^"]+")\s*:/;

export function directTriggers(block) {
  if (!block) return null;
  const triggers = [];
  let inline = block.first.replace(/^\s*['"]?on['"]?\s*:\s*/, "");
  // A multi-line flow form (`on: [\n push,\n fork]`) continues into the
  // body — join stripped lines until brackets balance.
  const opens = (s) => (s.match(/[[{]/g) ?? []).length;
  const closes = (s) => (s.match(/[\]}]/g) ?? []).length;
  if (opens(inline) > closes(inline)) {
    for (const l of block.body) {
      inline += " " + stripComment(l).trim();
      if (opens(inline) <= closes(inline)) break;
    }
  }
  if (inline && inline !== "|" && inline !== ">") {
    if (inline.startsWith("[") || inline.startsWith("{")) {
      const inner = inline.slice(1, inline.lastIndexOf(inline[0] === "[" ? "]" : "}"));
      for (const item of topLevelItems(inner)) {
        const k = item.match(KEY_RE);
        triggers.push(unquote(k ? k[1] : item));
      }
    } else {
      triggers.push(unquote(inline));
    }
  }
  const content = block.body
    .map(stripComment)
    .filter((l) => /\S/.test(l));
  // Direct children sit at the block's minimum indentation — including
  // indent 0: YAML permits block-sequence items at the parent key's column
  // (`on:`\n`- push` is legal and triggers a real run).
  const indents = content.map((l) => l.match(/^ */)[0].length);
  const min = indents.length ? Math.min(...indents) : Infinity;
  for (const l of content) {
    if (l.match(/^ */)[0].length !== min) continue;
    const seq = l.match(/^\s*-\s*(\S+)/);
    if (seq) {
      triggers.push(unquote(seq[1]));
      continue;
    }
    const key = l.match(KEY_RE);
    if (key) triggers.push(unquote(key[1]));
  }
  return triggers;
}

const BANNED = ["pull_request", "pull_request_target", "push"];

test("every workflow file declares an `on:` block (no vacuous scan)", () => {
  for (const file of workflowFiles()) {
    assert.notEqual(
      onBlock(readWorkflow(file)),
      null,
      `${file} has no top-level on: block`,
    );
  }
});

for (const trigger of BANNED) {
  test(`contract: no workflow declares a \`${trigger}\` trigger`, () => {
    for (const file of workflowFiles()) {
      const triggers = directTriggers(onBlock(readWorkflow(file))) ?? [];
      assert.ok(
        !triggers.includes(trigger),
        `${file} declares ${trigger} trigger`,
      );
    }
  });
}

test("contract: ci.yml direct triggers are exactly {workflow_dispatch}", () => {
  const triggers = new Set(directTriggers(onBlock(readWorkflow("ci.yml"))));
  assert.deepEqual(
    triggers,
    new Set(["workflow_dispatch"]),
    "ci.yml must stay dispatch-only — any other trigger bypasses the daily decide+dispatch contract",
  );
});

test("contract: ci.yml keeps the expected_sha dispatch input", () => {
  const block = onBlock(readWorkflow("ci.yml"));
  const body = block.body.map(stripComment);
  const wd = body.findIndex((l) => /^\s*workflow_dispatch\s*:/.test(l));
  const inputs = body.findIndex(
    (l, i) => i > wd && /^\s*inputs\s*:/.test(l),
  );
  assert.ok(inputs >= 0, "ci.yml lost the workflow_dispatch inputs block");
  assert.ok(
    body.slice(inputs).some((l) => /^\s*expected_sha\s*:/.test(l)),
    "ci.yml lost the expected_sha input (daily dispatch contract)",
  );
});

test("contract: ci-issue-tracker.yml keeps the workflow_run entry", () => {
  const triggers = directTriggers(onBlock(readWorkflow("ci-issue-tracker.yml")));
  assert.ok(triggers.includes("workflow_run"), "tracker lost workflow_run");
});

// The scan itself must not be a tautology: every legal `on:` shape that
// could smuggle a banned trigger back is exercised against a fixture, and
// comment/nested decoys prove the direct-child boundary holds.
test("self-check: block-map, block-sequence, inline and flow forms detected", () => {
  const blockMap = directTriggers(
    onBlock(["on:", "  pull_request:", "    branches: [main]", "  # push: decoy", "jobs: {}"].join("\n")),
  );
  assert.deepEqual(blockMap, ["pull_request"]);

  const blockSeq = directTriggers(onBlock("on:\n  - push\n  - workflow_dispatch\njobs: {}"));
  assert.deepEqual(blockSeq, ["push", "workflow_dispatch"]);

  const inlineScalar = directTriggers(onBlock("on: push\njobs: {}"));
  assert.deepEqual(inlineScalar, ["push"]);
  const quotedScalar = directTriggers(onBlock("on: 'push'\njobs: {}"));
  assert.deepEqual(quotedScalar, ["push"]);

  const flowList = directTriggers(onBlock("on: [push, workflow_dispatch]\njobs: {}"));
  assert.deepEqual(flowList, ["push", "workflow_dispatch"]);
  const flowMap = directTriggers(onBlock("on: {push: {branches: [main]}}\njobs: {}"));
  assert.deepEqual(flowMap, ["push"]);

  const quotedKey = directTriggers(onBlock('on:\n  "push":\njobs: {}'));
  assert.deepEqual(quotedKey, ["push"]);

  const colZeroSeq = directTriggers(onBlock("on:\n- push\njobs: {}"));
  assert.deepEqual(colZeroSeq, ["push"], "YAML allows block seq at parent indent");

  const multiLineFlow = directTriggers(
    onBlock("on: [\n  push,\n  workflow_dispatch]\njobs: {}"),
  );
  assert.deepEqual(multiLineFlow, ["push", "workflow_dispatch"]);
});

test("self-check: nested keys and comments are not triggers", () => {
  const nestedInput = directTriggers(
    onBlock(
      [
        "on:",
        "  workflow_dispatch:",
        "    inputs:",
        "      push: # legal input name, not a trigger",
        "        type: boolean",
        "jobs: {}",
      ].join("\n"),
    ),
  );
  assert.deepEqual(nestedInput, ["workflow_dispatch"]);

  const seqFilter = directTriggers(
    onBlock(
      ["on:", "  push:", "    branches:", "      - not-a-trigger", "jobs: {}"].join("\n"),
    ),
  );
  assert.deepEqual(seqFilter, ["push"]);
});
