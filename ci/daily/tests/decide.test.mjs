// decide.test.mjs — Issue #367 daily scheduler decision/dispatch contract tests.
// Every cell of the decision matrix is pinned here so a wrong-skip, a
// double-dispatch, a consumed-or-missed retry budget, or a wrong-sha
// verification record can never pass silently. All fixtures are offline —
// no real GitHub Actions calls, no real dispatches.

import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  FULL_CI_EVENTS,
  IN_FLIGHT_STATUSES,
  createFixtureClient,
  createGitHubClient,
  decide,
  execute,
  markdownSummary,
} from "../decide.mjs";

const SCRIPT = join(dirname(fileURLToPath(import.meta.url)), "..", "decide.mjs");

const SHA_A = "0123456789abcdef0123456789abcdef01234567";
const SHA_B = "89abcdef0123456789abcdef0123456789abcdef";
const SHA_C = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const BOT = "github-actions[bot]";

let runId = 1000;
function run(overrides = {}) {
  runId += 1;
  return {
    id: runId,
    event: "workflow_dispatch",
    head_branch: "main",
    head_sha: SHA_A,
    status: "completed",
    conclusion: "success",
    run_attempt: 1,
    created_at: "2026-11-05T00:00:00Z",
    updated_at: "2026-11-05T01:00:00Z",
    html_url: `https://example.test/runs/${runId}`,
    actor: { login: "someone" },
    triggering_actor: { login: "someone" },
    ...overrides,
  };
}

function decideOn(runs, overrides = {}) {
  return decide({ mainSha: SHA_A, runs, ...overrides });
}

async function executeFixture(fixture, overrides = {}) {
  const client = createFixtureClient(fixture);
  const result = await execute({
    client,
    repo: "o/r",
    workflow: "ci.yml",
    mainRef: "main",
    autoActors: [BOT],
    maxAutoDispatches: 2,
    maxPages: 10,
    maxRechecks: 2,
    confirmTimeoutMs: 100,
    confirmIntervalMs: 10,
    dryRun: false,
    sleep: async () => {},
    now: () => "2026-11-06T00:00:00.000Z",
    ...overrides,
  });
  return { ...result, calls: client.calls };
}

// --- decision core ---------------------------------------------------------

test("decide: latest completed success on main sha -> skip-verified", () => {
  const d = decideOn([run({ conclusion: "success" })]);
  assert.equal(d.decision, "skip-verified");
});

test("decide: sha changed (no attempt on current sha) -> dispatch", () => {
  const d = decideOn([run({ head_sha: SHA_B })]);
  assert.equal(d.decision, "dispatch");
  assert.equal(d.attempts.length, 0);
});

test("decide: empty history (retention expired) -> dispatch", () => {
  const d = decideOn([]);
  assert.equal(d.decision, "dispatch");
  assert.match(d.reason, /no-history/);
});

test("decide: any in-flight status on the sha -> suppress-in-flight", () => {
  for (const status of IN_FLIGHT_STATUSES) {
    const d = decideOn([run({ status, conclusion: null })]);
    assert.equal(d.decision, "suppress-in-flight", status);
  }
});

test("decide: in-flight older run suppresses even with newer failure", () => {
  const d = decideOn([
    run({ id: 10, conclusion: "failure", created_at: "2026-11-05T02:00:00Z" }),
    run({ id: 9, status: "in_progress", conclusion: null, created_at: "2026-11-05T01:00:00Z" }),
  ]);
  assert.equal(d.decision, "suppress-in-flight");
});

test("decide: unrecognised status on latest run -> hold-unknown", () => {
  const d = decideOn([run({ status: "mystery", conclusion: null })]);
  assert.equal(d.decision, "hold-unknown");
});

test("decide: non-success conclusions -> dispatch candidate", () => {
  for (const conclusion of ["failure", "cancelled", "timed_out", "action_required", "startup_failure", "stale", "skipped", "neutral"]) {
    const d = decideOn([run({ conclusion })]);
    assert.equal(d.decision, "dispatch", conclusion);
  }
});

test("decide: unrecognised conclusion on latest run -> hold-unknown, never dispatch", () => {
  // A completed run whose conclusion we cannot classify is unknown
  // state, not a retryable failure — dispatching on it could fire while
  // the SHA is in fact being verified.
  for (const conclusion of ["weird", "unknown_future_state", null]) {
    const d = decideOn([run({ conclusion })]);
    assert.equal(d.decision, "hold-unknown", conclusion);
  }
});

test("decide: pull_request / other branch / other sha runs are ignored", () => {
  const d = decideOn([
    run({ event: "pull_request", head_branch: "main" }),
    run({ event: "push", head_branch: "feature/x" }),
    run({ event: "push", head_sha: SHA_B }),
    run({ event: "workflow_dispatch", head_sha: "notasha" }),
  ]);
  assert.equal(d.attempts.length, 0);
  assert.equal(d.decision, "dispatch");
});

test("decide: push and schedule events count as valid Full CI runs", () => {
  for (const event of FULL_CI_EVENTS) {
    const d = decideOn([run({ event, conclusion: "success" })]);
    assert.equal(d.decision, "skip-verified", event);
  }
});

test("decide: unknown event does not contribute a verification", () => {
  const d = decideOn([run({ event: "repository_dispatch", conclusion: "success" })]);
  assert.equal(d.attempts.length, 0);
  assert.equal(d.decision, "dispatch");
});

test("decide: two bot dispatches on sha -> budget-exhausted", () => {
  const d = decideOn([
    run({ triggering_actor: { login: BOT }, conclusion: "failure", created_at: "2026-11-05T02:00:00Z" }),
    run({ triggering_actor: { login: BOT }, conclusion: "failure", created_at: "2026-11-05T01:00:00Z" }),
  ]);
  assert.equal(d.decision, "budget-exhausted");
  assert.equal(d.autoDispatches.length, 2);
});

test("decide: manual actor dispatch does not consume auto budget", () => {
  const d = decideOn([
    run({ triggering_actor: { login: "maintainer" }, conclusion: "failure" }),
    run({ triggering_actor: { login: "maintainer" }, conclusion: "failure" }),
  ]);
  assert.equal(d.decision, "dispatch");
  assert.equal(d.autoDispatches.length, 0);
});

test("decide: human rerun of bot dispatches does not refund the retry budget", () => {
  // API semantics: actor = original initiator, triggering_actor = latest
  // (re)run actor. A maintainer re-running two bot dispatches leaves
  // actor=github-actions[bot] — the dispatches still consumed budget.
  const rerun = () =>
    run({
      event: "workflow_dispatch",
      conclusion: "failure",
      run_attempt: 2,
      actor: { login: BOT },
      triggering_actor: { login: "maintainer" },
    });
  const d = decideOn([rerun(), rerun()]);
  assert.equal(d.autoDispatches.length, 2);
  assert.equal(d.decision, "budget-exhausted");
});

test("decide: unidentifiable-actor dispatch counts toward the budget", () => {
  // No actor provenance at all -> count on the safe side. Undercounting
  // lets automation retry forever; overcounting only dispatches less.
  const d = decideOn([
    run({ event: "workflow_dispatch", conclusion: "failure", actor: null, triggering_actor: null }),
    run({ event: "workflow_dispatch", conclusion: "failure", actor: null, triggering_actor: null }),
  ]);
  assert.equal(d.autoDispatches.length, 2);
  assert.equal(d.decision, "budget-exhausted");
});

test("decide: push runs never consume the dispatch budget", () => {
  const d = decideOn([
    run({ event: "push", triggering_actor: { login: BOT }, conclusion: "failure" }),
    run({ event: "push", triggering_actor: { login: BOT }, conclusion: "failure" }),
  ]);
  assert.equal(d.decision, "dispatch");
});

test("decide: budget counts only dispatches on the decision sha", () => {
  const d = decideOn([
    run({ head_sha: SHA_B, triggering_actor: { login: BOT }, conclusion: "failure" }),
    run({ head_sha: SHA_B, triggering_actor: { login: BOT }, conclusion: "failure" }),
  ]);
  assert.equal(d.decision, "dispatch");
  assert.equal(d.autoDispatches.length, 0);
});

test("decide: latest attempt by created_at wins; tie falls to higher id", () => {
  const d = decideOn([
    run({ id: 5, conclusion: "success", created_at: "2026-11-05T01:00:00Z" }),
    run({ id: 6, status: "in_progress", conclusion: null, created_at: "2026-11-05T01:00:00Z" }),
  ]);
  // Same created_at -> higher id is the newer run -> in-flight.
  assert.equal(d.latest.id, 6);
  assert.equal(d.decision, "suppress-in-flight");
});

test("decide: older run rerun-failed after newer success -> not skip-verified", () => {
  // Real API semantics: created_at stays at original creation while a
  // rerun moves run_started_at/updated_at. Ordering by created_at alone
  // would mask the rerun failure behind the newer success — a false
  // verified. Two fail-closed shapes:
  //  run_started_at moved -> the rerun is provably newest -> retry
  const rerunFailed = run({
    id: 5, conclusion: "failure", run_attempt: 2,
    created_at: "2026-11-01T00:00:00Z",
    run_started_at: "2026-11-06T00:00:00Z",
    updated_at: "2026-11-06T00:30:00Z",
  });
  const success = run({ id: 6, conclusion: "success", created_at: "2026-11-05T00:00:00Z", updated_at: "2026-11-05T01:00:00Z" });
  const d1 = decideOn([rerunFailed, success]);
  assert.equal(d1.latest.id, 5);
  assert.equal(d1.decision, "dispatch");
  //  only updated_at moved -> activity exists that produced no visible
  //  attempt evidence -> ordering ambiguous -> hold, never false verified
  const d2 = decideOn([
    { ...rerunFailed, run_started_at: undefined },
    success,
  ]);
  assert.equal(d2.decision, "hold-unknown");
  assert.match(d2.reason, /ambiguous/);
});

test("decide: success run contested by a tied non-success run -> hold-unknown", () => {
  // Tie at the top: the success run sorts first (higher id) but a
  // non-success run shares the newest attempt time — the ordering is
  // ambiguous, so no verified claim may be recorded.
  const d = decideOn([
    run({ id: 6, conclusion: "success", created_at: "2026-11-05T01:00:00Z" }),
    run({ id: 5, conclusion: "failure", created_at: "2026-11-05T01:00:00Z" }),
  ]);
  assert.equal(d.decision, "hold-unknown");
  assert.match(d.reason, /ambiguous/);
});

test("decide: tied failure on top -> bounded dispatch, not false verified", () => {
  // Reverse tie: the failure sorts first. Dispatch is a bounded retry —
  // never a success claim — and is therefore the safe resolution.
  const d = decideOn([
    run({ id: 5, conclusion: "success", created_at: "2026-11-05T01:00:00Z" }),
    run({ id: 6, conclusion: "failure", created_at: "2026-11-05T01:00:00Z" }),
  ]);
  assert.equal(d.decision, "dispatch");
});

test("decide: unorderable non-success run blocks a success claim", () => {
  const d = decideOn([
    run({ id: 5, conclusion: "success", created_at: "2026-11-05T01:00:00Z" }),
    run({ id: 6, conclusion: "failure", created_at: undefined, updated_at: undefined }),
  ]);
  assert.equal(d.decision, "hold-unknown");
});

test("decide: latestVerified reports newest success across shas", () => {
  const d = decideOn([
    run({ head_sha: SHA_B, conclusion: "success", created_at: "2026-11-04T00:00:00Z" }),
    run({ head_sha: SHA_C, conclusion: "success", created_at: "2026-11-05T00:00:00Z" }),
  ]);
  assert.equal(d.latestVerified.head_sha, SHA_C);
});

// --- orchestration ---------------------------------------------------------

test("execute: dispatch -> confirmed new workflow_dispatch run on expected sha", async () => {
  const existing = run({ conclusion: "failure" });
  const newRun = run({ id: 9999, status: "queued", conclusion: null, triggering_actor: { login: BOT } });
  const { outcome, report, calls } = await executeFixture({
    refReads: [SHA_A, SHA_A],
    listCalls: [
      [[existing]],
      [[existing, newRun]],
    ],
  });
  assert.equal(outcome, "dispatch-confirmed");
  assert.deepEqual(calls.dispatches, [{ ref: "main", inputs: { expected_sha: SHA_A } }]);
  assert.equal(report.dispatch.expectedSha, SHA_A);
  assert.equal(report.dispatch.confirmedRun.id, newRun.id);
  assert.equal(report.decisionSha, SHA_A);
  assert.equal(report.kind, "daily-ci-decision");
});

test("execute: dispatched run lands on a different sha -> dispatch-landed-mismatch", async () => {
  const { outcome, report, warnings } = await executeFixture({
    refReads: [SHA_A, SHA_A],
    listCalls: [
      [[]],
      [[run({ id: 8888, head_sha: SHA_B, status: "queued", conclusion: null })]],
    ],
  });
  assert.equal(outcome, "dispatch-landed-mismatch");
  assert.equal(report.dispatch.confirmedRun.headSha, SHA_B);
  assert.equal(warnings.length, 1);
  assert.match(warnings[0], /landed/);
});

test("execute: dispatch accepted but never observed -> dispatched-unconfirmed", async () => {
  const { outcome, warnings } = await executeFixture({
    refReads: [SHA_A, SHA_A],
    listCalls: [[[]], [[]], [[]]],
  });
  assert.equal(outcome, "dispatched-unconfirmed");
  assert.match(warnings[0], /not observed|eventual consistency/);
});

test("execute: main moved between decision and dispatch -> re-decides on new sha", async () => {
  const { outcome, calls } = await executeFixture({
    refReads: [SHA_A, SHA_B, SHA_B],
    listCalls: [
      [[]], // for SHA_A
      [[run({ head_sha: SHA_B, conclusion: "success" })]], // for SHA_B
    ],
  });
  assert.equal(outcome, "skip-verified");
  assert.equal(calls.dispatches.length, 0);
});

test("execute: main keeps moving past recheck bound -> main-moved, no dispatch", async () => {
  const { outcome, calls } = await executeFixture({
    refReads: [SHA_A, SHA_B, SHA_C, SHA_A],
    listCalls: [[[]], [[]], [[]]],
  });
  assert.equal(outcome, "main-moved");
  assert.equal(calls.dispatches.length, 0);
});

test("execute: dry-run decides but never dispatches", async () => {
  const { outcome, calls } = await executeFixture(
    { refReads: [SHA_A, SHA_A], listCalls: [[[]]] },
    { dryRun: true },
  );
  assert.equal(outcome, "dry-run");
  assert.equal(calls.dispatches.length, 0);
});

test("execute: API list failure -> error outcome, exit 1", async () => {
  const { outcome, exitCode } = await executeFixture({
    refReads: [SHA_A],
    listError: { message: "boom" },
  });
  assert.equal(outcome, "error");
  assert.equal(exitCode, 1);
});

test("execute: dispatch POST failure -> error outcome, exit 1", async () => {
  const { outcome, exitCode } = await executeFixture({
    refReads: [SHA_A, SHA_A],
    listCalls: [[[]]],
    dispatchCalls: [{ error: { status: 422, message: "unprocessable" } }],
  });
  assert.equal(outcome, "error");
  assert.equal(exitCode, 1);
});

test("execute: ref not a sha -> error outcome", async () => {
  const { outcome, exitCode } = await executeFixture({ refReads: ["garbage"] });
  assert.equal(outcome, "error");
  assert.equal(exitCode, 1);
});

test("execute: budget-exhausted completes at exit 0 with warning", async () => {
  const runs = [
    run({ triggering_actor: { login: BOT }, conclusion: "failure", created_at: "2026-11-05T02:00:00Z" }),
    run({ triggering_actor: { login: BOT }, conclusion: "failure", created_at: "2026-11-05T01:00:00Z" }),
  ];
  const { outcome, exitCode, warnings } = await executeFixture({
    refReads: [SHA_A],
    listCalls: [[runs]],
  });
  assert.equal(outcome, "budget-exhausted");
  assert.equal(exitCode, 0);
  assert.equal(warnings.length, 1);
});

test("execute: truncated history holds every verdict — no verified claim, no dispatch", async () => {
  // Unseen older pages could hide auto dispatches (budget undercount) or
  // a rerun on this SHA contesting a success claim — incomplete data
  // cannot certify any verdict. Even a visible success must not become
  // a recorded skip-verified.
  for (const runs of [[], [run({ conclusion: "success" })]]) {
    const { outcome, report, warnings, calls } = await executeFixture({
      refReads: [SHA_A],
      listCalls: [[runs]],
      truncated: true,
    });
    assert.equal(outcome, "history-truncated");
    assert.equal(report.historyTruncated, true);
    assert.equal(calls.dispatches.length, 0);
    assert.ok(warnings.some((w) => w.includes("truncated")));
  }
});

test("execute: malformed runs-list response -> error, no dispatch", async () => {
  // A 200 body without workflow_runs is malformed — not empty history.
  // Exercised through the real createGitHubClient response boundary.
  const client = createGitHubClient({
    token: "t",
    repo: "o/r",
    apiBase: "https://api.invalid",
    fetchImpl: async (url, init) => {
      if (init.method === "POST") return new Response(null, { status: 204 });
      if (url.includes("/git/ref/")) {
        return new Response(JSON.stringify({ object: { sha: SHA_A } }));
      }
      return new Response(JSON.stringify({}));
    },
  });
  const { outcome, exitCode } = await execute({
    client,
    repo: "o/r",
    workflow: "ci.yml",
    mainRef: "main",
    autoActors: [BOT],
    maxAutoDispatches: 2,
    maxPages: 10,
    maxRechecks: 2,
    confirmTimeoutMs: 100,
    confirmIntervalMs: 10,
    dryRun: false,
    sleep: async () => {},
    now: () => "2026-11-06T00:00:00.000Z",
  });
  assert.equal(outcome, "error");
  assert.equal(exitCode, 1);
});

test("execute: human dispatch racing ours is reported, not hidden", async () => {
  // A manual workflow_dispatch landing in the poll window is
  // indistinguishable from ours at this API surface — the honest result
  // is dispatch-confirmed naming the observed run (its actor/url are in
  // the report), never a silent merge. Documented limit, not a
  // guarantee that "our" run is the one that landed.
  const manual = run({ id: 9001, status: "queued", conclusion: null, actor: { login: "human" }, triggering_actor: { login: "human" } });
  const { outcome, report } = await executeFixture({
    refReads: [SHA_A, SHA_A],
    listCalls: [
      [[]],
      [[manual]],
    ],
  });
  assert.equal(outcome, "dispatch-confirmed");
  assert.equal(report.dispatch.confirmedRun.id, 9001);
});

test("execute: skip-verified records latestVerified for ops", async () => {
  const { outcome, report } = await executeFixture({
    refReads: [SHA_A],
    listCalls: [[
      [run({ conclusion: "success", updated_at: "2026-11-05T03:00:00Z" })],
    ]],
  });
  assert.equal(outcome, "skip-verified");
  assert.equal(report.latestVerified.headSha, SHA_A);
  assert.equal(report.latestVerified.updatedAt, "2026-11-05T03:00:00Z");
});

test("execute: a same-id rerun does not look like a new dispatch", async () => {
  // The in-flight run was already present pre-dispatch -> the new-run
  // detection must not count it; poll exhausts -> unconfirmed.
  const existing = run({ id: 4242, status: "in_progress", conclusion: null });
  const failure = run({ head_sha: SHA_B, conclusion: "failure" });
  const { outcome } = await executeFixture({
    refReads: [SHA_B, SHA_B],
    listCalls: [
      [[failure, existing]],
      [[failure, existing]],
      [[failure, existing]],
    ],
  });
  assert.equal(outcome, "dispatched-unconfirmed");
});

// --- reporting -------------------------------------------------------------

test("markdownSummary renders outcome, dispatch line, attempts table, warnings", () => {
  const report = {
    outcome: "dispatch-landed-mismatch",
    reason: "r",
    decisionSha: SHA_A,
    observedShas: [SHA_A],
    latestVerified: { headSha: SHA_B, updatedAt: "2026-11-04T00:00:00Z", url: "https://x/y" },
    autoDispatchCount: 1,
    maxAutoDispatches: 2,
    historyTruncated: false,
    dispatch: {
      requestedAt: "2026-11-06T00:00:00.000Z",
      expectedSha: SHA_A,
      ref: "main",
      confirmedRun: { id: 1, headSha: SHA_B, status: "queued", conclusion: null, runAttempt: 1, url: "u" },
    },
    attemptsOnDecisionSha: [
      { id: 7, event: "workflow_dispatch", status: "completed", conclusion: "failure", runAttempt: 2, createdAt: "t", actor: BOT, url: "u" },
    ],
    warnings: ["w1"],
  };
  const md = markdownSummary(report);
  assert.match(md, /daily-ci outcome: dispatch-landed-mismatch/);
  assert.match(md, /expected `0123456789ab`/);
  assert.match(md, /landed `89abcdef0123`/);
  assert.match(md, /\| run \| event \| status \| conclusion \| attempt \|/);
  assert.match(md, /w1/);
});

// --- CLI end-to-end (fixture mode, offline) ---------------------------------

function cliRun(fixture, extra = []) {
  const dir = mkdtempSync(join(tmpdir(), "daily-ci-"));
  const fixturePath = join(dir, "fixture.json");
  const reportPath = join(dir, "report.json");
  const summaryPath = join(dir, "summary.md");
  writeFileSync(fixturePath, JSON.stringify(fixture));
  const env = {
    ...process.env,
    GITHUB_REPOSITORY: "o/r",
    GITHUB_STEP_SUMMARY: summaryPath,
    DRY_RUN: "",
  };
  let code = 0;
  let stdout = "";
  try {
    stdout = execFileSync("node", [SCRIPT, "--fixture", fixturePath, "--report", reportPath, "--summary", summaryPath, ...extra], {
      env,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
  } catch (e) {
    code = e.status ?? 1;
    stdout = e.stdout ?? "";
  }
  return { code, stdout, reportPath, summaryPath };
}

test("cli: fixture dispatch-confirmed writes report + summary, exit 0", () => {
  const { code, stdout, reportPath, summaryPath } = cliRun({
    refReads: [SHA_A, SHA_A],
    listCalls: [[[]], [[run({ id: 7777, status: "queued", conclusion: null })]]],
  });
  assert.equal(code, 0);
  const report = JSON.parse(readFileSync(reportPath, "utf8"));
  assert.equal(report.kind, "daily-ci-decision");
  assert.equal(report.outcome, "dispatch-confirmed");
  assert.equal(report.dispatch.expectedSha, SHA_A);
  const summary = readFileSync(summaryPath, "utf8");
  assert.match(summary, /daily-ci outcome: dispatch-confirmed/);
  assert.match(stdout, /daily-ci outcome: dispatch-confirmed/);
});

test("cli: fixture error exits 1", () => {
  const { code } = cliRun({ refError: { message: "down" } });
  assert.equal(code, 1);
});

test("cli: missing token without fixture exits 2", () => {
  let code = 0;
  try {
    execFileSync("node", [SCRIPT, "--repo", "o/r"], {
      env: { ...process.env, GITHUB_TOKEN: "", GITHUB_REPOSITORY: "" },
      stdio: ["ignore", "pipe", "pipe"],
    });
  } catch (e) {
    code = e.status ?? 1;
  }
  assert.equal(code, 2);
});

test("cli: budget-exhausted emits ::warning:: annotation and exits 0", () => {
  const { code, stdout } = cliRun({
    refReads: [SHA_A],
    listCalls: [[[
      run({ triggering_actor: { login: BOT }, conclusion: "failure", created_at: "2026-11-05T02:00:00Z" }),
      run({ triggering_actor: { login: BOT }, conclusion: "failure", created_at: "2026-11-05T01:00:00Z" }),
    ]]],
  });
  assert.equal(code, 0);
  assert.match(stdout, /::warning::/);
});
