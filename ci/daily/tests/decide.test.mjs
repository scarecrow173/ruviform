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
import { deflateRawSync } from "node:zlib";
import {
  FULL_CI_EVENTS,
  IN_FLIGHT_STATUSES,
  buildStoredZip,
  classifyRunEvidence,
  createFixtureClient,
  createGitHubClient,
  decide,
  decideTuple,
  execute,
  executeTuple,
  jsonFromArtifactZip,
  markdownSummary,
  parseConsumerRepos,
  parseLsRemote,
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

test("decide: unidentifiable history records hold every verdict", () => {
  // Records lacking the identification metadata needed to prove them
  // on-target OR off-target may hide an attempt on this SHA in any
  // state — no dispatch, no skip-verified, whatever else is visible.
  const variants = [
    [{}], // completely empty record
    [{ head_branch: "main", event: "push" }], // branch+event match but no head_sha — could be our SHA
    [{ head_sha: SHA_A, event: "push" }], // on-SHA event run but no branch — could be on main
    [{ head_sha: "notasha", head_branch: "main", event: "push" }], // corrupt sha — cannot prove non-target
    [{ event: "schedule" }], // in-allowlist event, nothing else
    [null], // non-object record reaching the pure core
    [run({ event: "pull_request", head_branch: "main" }), {}], // mixed: one excluded, one unidentifiable
  ];
  for (const runs of variants) {
    const d = decideOn(runs);
    assert.equal(d.decision, "hold-unknown", JSON.stringify(runs));
    assert.ok(d.unidentifiable.length >= 1, JSON.stringify(runs));
  }
});

test("decide: unidentifiable record cannot hide behind a visible success", () => {
  // A provable success run plus a record we cannot place — the success
  // claim must not be recorded while the unverified state may be hidden.
  const d = decideOn([run({ conclusion: "success" }), {}]);
  assert.equal(d.decision, "hold-unknown");
});

test("decide: clearly non-target records are still excluded normally", () => {
  // Only records that PROVE exclusion drop out — pull_request event,
  // other branch, other valid sha.
  const d = decideOn([
    run({ event: "pull_request", head_branch: "main" }),
    run({ head_branch: "feature" }),
    run({ head_sha: SHA_B }),
    run({ head_sha: "0123456789abcdef0123456789abcdef01234599" }),
  ]);
  assert.equal(d.unidentifiable.length, 0);
  assert.equal(d.decision, "dispatch");
});

test("decide: pull_request / other branch / other sha runs are ignored", () => {
  const d = decideOn([
    run({ event: "pull_request", head_branch: "main" }),
    run({ event: "push", head_branch: "feature/x" }),
    run({ event: "push", head_sha: SHA_B }),
    run({ event: "workflow_dispatch", head_sha: SHA_B }),
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

test("decide: one-sided or malformed actor provenance still counts toward the budget", () => {
  // Only BOTH sides identifying as non-auto actors proves a manual
  // origin. An unidentifiable side — null, empty login, non-object —
  // cannot disprove automation, so the dispatch still counts.
  const variants = [
    { actor: null, triggering_actor: { login: "maintainer" } }, // the coordinator's rerun case
    { actor: { login: "maintainer" }, triggering_actor: null },
    { actor: {}, triggering_actor: { login: "maintainer" } },
    { actor: { login: "" }, triggering_actor: { login: "maintainer" } },
    { actor: "weird", triggering_actor: { login: "maintainer" } },
    { actor: null }, // triggering_actor left at human default
    { actor: undefined, triggering_actor: undefined }, // both fields absent entirely
  ];
  for (const provenance of variants) {
    const runs = [
      run({ event: "workflow_dispatch", conclusion: "failure", ...provenance }),
      run({ event: "workflow_dispatch", conclusion: "failure", ...provenance }),
    ];
    const d = decideOn(runs);
    assert.equal(d.autoDispatches.length, 2, JSON.stringify(provenance));
    assert.equal(d.decision, "budget-exhausted", JSON.stringify(provenance));
  }
});

test("decide: both-sides-human dispatch stays manual", () => {
  const d = decideOn([
    run({ event: "workflow_dispatch", conclusion: "failure", actor: { login: "alice" }, triggering_actor: { login: "bob" } }),
    run({ event: "workflow_dispatch", conclusion: "failure", actor: { login: "alice" }, triggering_actor: { login: "bob" } }),
  ]);
  assert.equal(d.autoDispatches.length, 0);
  assert.equal(d.decision, "dispatch");
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

test("execute: malformed workflow_runs entries -> error, no dispatch", async () => {
  // Records missing the identification/state fields every real run
  // carries fail the list-response schema — malformed response, never
  // "ignorable run" and never empty history.
  const malformedEntries = [
    [{}],
    [{ id: 1 }],
    [{ id: 1, head_sha: SHA_A, head_branch: "main", event: "push" }], // status missing
    [{ id: 1, head_sha: SHA_A, head_branch: "main", status: "completed" }], // event missing
    [null],
  ];
  for (const entries of malformedEntries) {
    let dispatches = 0;
    const client = createGitHubClient({
      token: "t",
      repo: "o/r",
      apiBase: "https://api.invalid",
      fetchImpl: async (url, init) => {
        if (init.method === "POST") { dispatches += 1; return new Response(null, { status: 204 }); }
        if (url.includes("/git/ref/")) {
          return new Response(JSON.stringify({ object: { sha: SHA_A } }));
        }
        return new Response(JSON.stringify({ workflow_runs: entries }));
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
    assert.equal(outcome, "error", JSON.stringify(entries));
    assert.equal(exitCode, 1);
    assert.equal(dispatches, 0);
  }
});

test("execute: schema-valid but unidentifiable entries -> hold, no dispatch", async () => {
  // Fields exist but carry no provable identity: a record that passes
  // the wire schema yet cannot be proven on-target or off-target.
  // The decision core must hold rather than dispatch or skip.
  const client = createGitHubClient({
    token: "t",
    repo: "o/r",
    apiBase: "https://api.invalid",
    fetchImpl: async (url, init) => {
      if (init.method === "POST") return new Response(null, { status: 204 });
      if (url.includes("/git/ref/")) {
        return new Response(JSON.stringify({ object: { sha: SHA_A } }));
      }
      return new Response(
        JSON.stringify({
          workflow_runs: [
            { id: 1, head_sha: "notasha", head_branch: "main", event: "push", status: "completed" },
          ],
        }),
      );
    },
  });
  const { outcome, report } = await execute({
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
  assert.equal(outcome, "hold-unknown");
  assert.equal(report.unidentifiableCount, 1);
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

// --- tuple mode (Issue #294) -------------------------------------------------
//
// The tuple under test is
//   (ruviform main SHA, audiolens main SHA, synvelis main SHA,
//    "consumer-verify-tuple/v1")
// Run records do not expose dispatch inputs, so the tuple a run verified is
// recovered from consumer-evidence-<name>-attempt-<n> artifacts. These tests
// pin the fail-closed matrix: verified requires success + all consumers pass
// + exact tuple match; unverifiable evidence never verifies and counts
// conservatively toward the retry budget.

const AL_SHA = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const SV_SHA = "cccccccccccccccccccccccccccccccccccccccc";
const AL_REPO = "o/audio-lens";
const SV_REPO = "o/synvelis";
const CONTRACT = "consumer-verify-tuple/v1";
const CONSUMERS = [
  { name: "audiolens", repo: AL_REPO },
  { name: "synvelis", repo: SV_REPO },
];
const CANDIDATE = {
  ruviformSha: SHA_A,
  consumers: [
    { name: "audiolens", repo: AL_REPO, sha: AL_SHA },
    { name: "synvelis", repo: SV_REPO, sha: SV_SHA },
  ],
  contract: CONTRACT,
};

function evidenceFor(runId, consumer, { ruviformRev = SHA_A, consumerSha, repo, status = "pass", contract = CONTRACT } = {}) {
  const nameRepo = consumer === "audiolens" ? AL_REPO : SV_REPO;
  const sha = consumerSha ?? (consumer === "audiolens" ? AL_SHA : SV_SHA);
  return {
    schemaVersion: 1,
    kind: "consumer-verify-evidence",
    contract,
    run: { runId: String(runId), runAttempt: 1, event: "workflow_dispatch" },
    candidate: { ruviformRev },
    consumer: { name: consumer, repo: repo ?? nameRepo, resolvedSha: sha, expectedSha: sha },
    tuple: { ruviformRev, consumerResolvedSha: sha, contract },
    status,
    statusReason: status === "pass" ? null : "test",
  };
}

function coveringPair(runId, opts = {}) {
  return new Map([
    ["audiolens", evidenceFor(runId, "audiolens", opts.al ?? {})],
    ["synvelis", evidenceFor(runId, "synvelis", opts.sv ?? {})],
  ]);
}

function decideTupleOn({ covering = [], inFlight = [], unverifiable = [], candidate = CANDIDATE, ...rest } = {}) {
  return decideTuple({
    candidate,
    covering,
    inFlight,
    unverifiable,
    autoActors: [BOT],
    maxAutoDispatches: 2,
    ...rest,
  });
}

async function executeTupleFixture(fixture, overrides = {}) {
  const client = createFixtureClient(fixture);
  const result = await executeTuple({
    client,
    repo: "o/r",
    workflow: "consumer-verify.yml",
    mainRef: "main",
    mode: "tuple",
    consumers: CONSUMERS.map((c) => ({ ...c })),
    tupleContract: CONTRACT,
    autoActors: [BOT],
    maxAutoDispatches: 2,
    maxPages: 10,
    maxRechecks: 2,
    maxEvidenceRuns: 60,
    confirmTimeoutMs: 100,
    confirmIntervalMs: 10,
    dryRun: false,
    sleep: async () => {},
    now: () => "2026-11-06T00:00:00.000Z",
    ...overrides,
  });
  return { ...result, calls: client.calls };
}

// decideTuple decision matrix ----------------------------------------------

test("decideTuple: verified covering run (success + all consumers pass) -> skip-verified", () => {
  const r = run({ conclusion: "success" });
  const d = decideTupleOn({ covering: [{ run: r, consumerStatus: { audiolens: "pass", synvelis: "pass" } }] });
  assert.equal(d.decision, "skip-verified");
  assert.equal(d.latestVerified.run.id, r.id);
});

test("decideTuple: success run with a non-pass consumer is NOT verified -> dispatch", () => {
  for (const status of ["not_run", "unsupported", "fail"]) {
    const r = run({ conclusion: "success" });
    const d = decideTupleOn({
      covering: [{ run: r, consumerStatus: { audiolens: "pass", synvelis: status } }],
    });
    assert.equal(d.decision, "dispatch", status);
  }
});

test("decideTuple: non-success covering run -> retry path", () => {
  const r = run({ conclusion: "failure" });
  const d = decideTupleOn({ covering: [{ run: r, consumerStatus: { audiolens: "fail", synvelis: "fail" } }] });
  assert.equal(d.decision, "dispatch");
});

test("decideTuple: any in-flight run suppresses even with verified covering", () => {
  const d = decideTupleOn({
    covering: [{ run: run({ conclusion: "success" }), consumerStatus: { audiolens: "pass", synvelis: "pass" } }],
    inFlight: [run({ status: "in_progress", conclusion: null })],
  });
  assert.equal(d.decision, "suppress-in-flight");
});

test("decideTuple: in-flight suppresses with no covering history at all", () => {
  const d = decideTupleOn({ inFlight: [run({ status: "queued", conclusion: null })] });
  assert.equal(d.decision, "suppress-in-flight");
});

test("decideTuple: unknown state on latest covering run -> hold-unknown", () => {
  const d = decideTupleOn({
    covering: [{ run: run({ status: "mystery", conclusion: null }), consumerStatus: { audiolens: "pass", synvelis: "pass" } }],
  });
  assert.equal(d.decision, "hold-unknown");
});

test("decideTuple: verified latest contested by tied non-verified covering -> hold-unknown", () => {
  const d = decideTupleOn({
    covering: [
      { run: run({ id: 6, conclusion: "success", created_at: "2026-11-05T01:00:00Z" }), consumerStatus: { audiolens: "pass", synvelis: "pass" } },
      { run: run({ id: 5, conclusion: "failure", created_at: "2026-11-05T01:00:00Z" }), consumerStatus: { audiolens: "fail", synvelis: "fail" } },
    ],
  });
  assert.equal(d.decision, "hold-unknown");
});

test("decideTuple: unverifiable run newer than verified covering -> hold-unknown", () => {
  const d = decideTupleOn({
    covering: [{ run: run({ id: 5, conclusion: "success", created_at: "2026-11-05T01:00:00Z" }), consumerStatus: { audiolens: "pass", synvelis: "pass" } }],
    unverifiable: [{ run: run({ id: 6, conclusion: "success", created_at: "2026-11-05T02:00:00Z" }) }],
  });
  assert.equal(d.decision, "hold-unknown");
});

test("decideTuple: unverifiable run OLDER than verified covering does not block skip", () => {
  const d = decideTupleOn({
    covering: [{ run: run({ id: 6, conclusion: "success", created_at: "2026-11-05T02:00:00Z" }), consumerStatus: { audiolens: "pass", synvelis: "pass" } }],
    unverifiable: [{ run: run({ id: 5, conclusion: "success", created_at: "2026-11-05T01:00:00Z" }) }],
  });
  assert.equal(d.decision, "skip-verified");
});

test("decideTuple: unverifiable-only history -> dispatch (never blocks retry)", () => {
  const d = decideTupleOn({
    unverifiable: [
      { run: run({ conclusion: "success", actor: { login: "human" }, triggering_actor: { login: "human" } }) },
    ],
  });
  assert.equal(d.decision, "dispatch");
});

test("decideTuple: covering auto dispatches + unverifiable auto dispatches share one budget", () => {
  const auto = { actor: { login: BOT }, triggering_actor: { login: BOT } };
  const d = decideTupleOn({
    covering: [
      { run: run({ conclusion: "failure", created_at: "2026-11-05T02:00:00Z", ...auto }), consumerStatus: { audiolens: "fail", synvelis: "fail" } },
    ],
    unverifiable: [{ run: run({ conclusion: "failure", created_at: "2026-11-05T01:00:00Z", ...auto }) }],
  });
  assert.equal(d.autoOnTuple, 2);
  assert.equal(d.decision, "budget-exhausted");
});

test("decideTuple: unverifiable AUTO dispatches alone exhaust the budget", () => {
  const auto = { actor: { login: BOT }, triggering_actor: { login: BOT } };
  const d = decideTupleOn({
    unverifiable: [
      { run: run({ conclusion: "failure", created_at: "2026-11-05T02:00:00Z", ...auto }) },
      { run: run({ conclusion: "failure", created_at: "2026-11-05T01:00:00Z", ...auto }) },
    ],
  });
  assert.equal(d.decision, "budget-exhausted");
});

test("decideTuple: unverifiable MANUAL dispatches do not consume budget", () => {
  const d = decideTupleOn({
    unverifiable: [
      { run: run({ conclusion: "failure", actor: { login: "alice" }, triggering_actor: { login: "bob" } }) },
      { run: run({ conclusion: "failure", actor: { login: "alice" }, triggering_actor: { login: "bob" } }) },
    ],
  });
  assert.equal(d.decision, "dispatch");
});

test("decideTuple: empty everything -> dispatch", () => {
  const d = decideTupleOn({});
  assert.equal(d.decision, "dispatch");
  assert.match(d.reason, /no-covering-history/);
});

// classifyRunEvidence --------------------------------------------------------

test("classifyRunEvidence: exact tuple match -> covering with consumer statuses", () => {
  const r = run({ id: 42 });
  const cls = classifyRunEvidence({
    run: r,
    evidences: coveringPair(42),
    candidate: CANDIDATE,
    contract: CONTRACT,
  });
  assert.equal(cls.kind, "covering");
  assert.deepEqual(cls.consumerStatus, { audiolens: "pass", synvelis: "pass" });
});

test("classifyRunEvidence: any missing consumer evidence -> unverifiable", () => {
  const r = run({ id: 42 });
  for (const drop of ["audiolens", "synvelis"]) {
    const evs = coveringPair(42);
    evs.set(drop, null);
    const cls = classifyRunEvidence({ run: r, evidences: evs, candidate: CANDIDATE, contract: CONTRACT });
    assert.equal(cls.kind, "unverifiable", drop);
  }
});

test("classifyRunEvidence: consumer sha mismatch -> non-target (proven different tuple)", () => {
  const r = run({ id: 42 });
  const evs = coveringPair(42, { al: { consumerSha: SHA_B } });
  const cls = classifyRunEvidence({ run: r, evidences: evs, candidate: CANDIDATE, contract: CONTRACT });
  assert.equal(cls.kind, "non-target");
});

test("classifyRunEvidence: ruviform rev mismatch -> non-target", () => {
  const r = run({ id: 42 });
  const evs = coveringPair(42, { al: { ruviformRev: SHA_B }, sv: { ruviformRev: SHA_B } });
  const cls = classifyRunEvidence({ run: r, evidences: evs, candidate: CANDIDATE, contract: CONTRACT });
  assert.equal(cls.kind, "non-target");
});

test("classifyRunEvidence: contract version mismatch -> non-target", () => {
  const r = run({ id: 42 });
  const evs = coveringPair(42, { al: { contract: "consumer-verify-tuple/v2" }, sv: { contract: "consumer-verify-tuple/v2" } });
  const cls = classifyRunEvidence({ run: r, evidences: evs, candidate: CANDIDATE, contract: CONTRACT });
  assert.equal(cls.kind, "non-target");
});

test("classifyRunEvidence: inconsistent ruviformRev across consumers -> unverifiable", () => {
  const r = run({ id: 42 });
  const evs = coveringPair(42, { sv: { ruviformRev: SHA_B } });
  const cls = classifyRunEvidence({ run: r, evidences: evs, candidate: CANDIDATE, contract: CONTRACT });
  assert.equal(cls.kind, "unverifiable");
});

test("classifyRunEvidence: runId mismatch -> unverifiable", () => {
  const r = run({ id: 42 });
  const evs = new Map([
    ["audiolens", evidenceFor(999, "audiolens")],
    ["synvelis", evidenceFor(42, "synvelis")],
  ]);
  const cls = classifyRunEvidence({ run: r, evidences: evs, candidate: CANDIDATE, contract: CONTRACT });
  assert.equal(cls.kind, "unverifiable");
});

test("classifyRunEvidence: wrong kind / missing tuple / bad status -> unverifiable", () => {
  const r = run({ id: 42 });
  const variants = [
    { ...evidenceFor(42, "audiolens"), kind: "other" },
    { ...evidenceFor(42, "audiolens"), tuple: null },
    { ...evidenceFor(42, "audiolens"), status: "" },
    { ...evidenceFor(42, "audiolens"), consumer: { name: "synvelis", repo: SV_REPO } },
    { ...evidenceFor(42, "audiolens"), tuple: { ruviformRev: "xyz", consumerResolvedSha: AL_SHA, contract: CONTRACT } },
  ];
  for (const bad of variants) {
    const evs = coveringPair(42);
    evs.set("audiolens", bad);
    const cls = classifyRunEvidence({ run: r, evidences: evs, candidate: CANDIDATE, contract: CONTRACT });
    assert.equal(cls.kind, "unverifiable", JSON.stringify(bad).slice(0, 80));
  }
});

// jsonFromArtifactZip ---------------------------------------------------------

function buildDeflateZip(entries) {
  // Same layout as buildStoredZip but method 8 + deflateRawSync payloads.
  const locals = [];
  const centrals = [];
  let offset = 0;
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  const crc = (buf) => {
    let c = 0xffffffff;
    for (const byte of buf) c = table[(c ^ byte) & 0xff] ^ (c >>> 8);
    return (c ^ 0xffffffff) >>> 0;
  };
  for (const e of entries) {
    const name = Buffer.from(e.name, "utf8");
    const raw = Buffer.isBuffer(e.data) ? e.data : Buffer.from(e.data);
    const data = deflateRawSync(raw);
    const c = crc(raw);
    const local = Buffer.alloc(30);
    local.writeUInt32LE(0x04034b50, 0);
    local.writeUInt16LE(20, 4);
    local.writeUInt16LE(8, 8);
    local.writeUInt32LE(c, 14);
    local.writeUInt32LE(data.length, 18);
    local.writeUInt32LE(raw.length, 22);
    local.writeUInt16LE(name.length, 26);
    locals.push(local, name, data);
    const central = Buffer.alloc(46);
    central.writeUInt32LE(0x02014b50, 0);
    central.writeUInt16LE(20, 4);
    central.writeUInt16LE(20, 6);
    central.writeUInt16LE(8, 10);
    central.writeUInt32LE(c, 16);
    central.writeUInt32LE(data.length, 20);
    central.writeUInt32LE(raw.length, 24);
    central.writeUInt16LE(name.length, 28);
    central.writeUInt32LE(offset, 42);
    centrals.push(central, name);
    offset += 30 + name.length + data.length;
  }
  const cd = Buffer.concat(centrals);
  const eocd = Buffer.alloc(22);
  eocd.writeUInt32LE(0x06054b50, 0);
  eocd.writeUInt16LE(entries.length, 8);
  eocd.writeUInt16LE(entries.length, 10);
  eocd.writeUInt32LE(cd.length, 12);
  eocd.writeUInt32LE(offset, 16);
  return Buffer.concat([...locals, cd, eocd]);
}

test("jsonFromArtifactZip: stored-method zip round-trips JSON", () => {
  const zip = buildStoredZip([{ name: "consumer-evidence-audiolens.json", data: Buffer.from(JSON.stringify({ a: 1 })) }]);
  assert.deepEqual(jsonFromArtifactZip(zip), { a: 1 });
});

test("jsonFromArtifactZip: deflate-method zip round-trips JSON", () => {
  const zip = buildDeflateZip([{ name: "evidence.json", data: Buffer.from(JSON.stringify({ b: [1, 2, 3] })) }]);
  assert.deepEqual(jsonFromArtifactZip(zip), { b: [1, 2, 3] });
});

test("jsonFromArtifactZip: picks the .json entry among several", () => {
  const zip = buildStoredZip([
    { name: "readme.txt", data: Buffer.from("not json") },
    { name: "data.json", data: Buffer.from(JSON.stringify({ ok: true })) },
  ]);
  assert.deepEqual(jsonFromArtifactZip(zip), { ok: true });
});

test("jsonFromArtifactZip: non-zip / bad JSON / empty archive -> ApiError", () => {
  assert.throws(() => jsonFromArtifactZip(Buffer.from("not a zip")), /artifact zip/);
  assert.throws(() => jsonFromArtifactZip(buildStoredZip([{ name: "x.json", data: Buffer.from("{bad") }])), /not valid JSON/);
  assert.throws(() => jsonFromArtifactZip(buildStoredZip([])), /empty archive/);
});

// parseLsRemote ---------------------------------------------------------------

test("parseLsRemote: exactly one matching ref line -> sha", () => {
  assert.equal(parseLsRemote(`${AL_SHA}\trefs/heads/main\n`, "main"), AL_SHA);
});

test("parseLsRemote: empty / multiple / wrong-ref / non-sha -> ApiError", () => {
  for (const out of ["", "\n", `${AL_SHA}\trefs/heads/main\n${SV_SHA}\trefs/heads/main\n`, `${AL_SHA}\trefs/heads/dev\n`, `zzz\trefs/heads/main\n`]) {
    assert.throws(() => parseLsRemote(out, "main"), /ls-remote/, JSON.stringify(out));
  }
});

// parseConsumerRepos ----------------------------------------------------------

test("parseConsumerRepos: name=owner/repo list preserves order, rejects malformed", () => {
  assert.deepEqual(parseConsumerRepos("a=o/r1,b=o/r2"), [
    { name: "a", repo: "o/r1" },
    { name: "b", repo: "o/r2" },
  ]);
  for (const bad of ["a", "=o/r", "A=o/r", "a=o", "a=o/r/s", "a==o/r"]) {
    assert.throws(() => parseConsumerRepos(bad), /consumer-repo|consumer/, bad);
  }
});

// executeTuple orchestration ---------------------------------------------------

test("executeTuple: verified covering run -> skip-verified, no dispatch", async () => {
  const r = run({ conclusion: "success" });
  const { outcome, report, calls } = await executeTupleFixture({
    refReads: [SHA_A, SHA_A],
    repoRefReads: { [AL_REPO]: [AL_SHA, AL_SHA], [SV_REPO]: [SV_SHA, SV_SHA] },
    listCalls: [[r]],
    runArtifacts: { [String(r.id)]: { "1": { audiolens: evidenceFor(r.id, "audiolens"), synvelis: evidenceFor(r.id, "synvelis") } } },
  });
  assert.equal(outcome, "skip-verified");
  assert.equal(calls.dispatches.length, 0);
  assert.equal(report.kind, "daily-ci-tuple-decision");
  assert.equal(report.tuple.ruviformSha, SHA_A);
  assert.equal(report.coveringAttemptsOnDecisionTuple.length, 1);
  assert.equal(report.coveringAttemptsOnDecisionTuple[0].consumerStatus.audiolens, "pass");
  assert.equal(report.latestVerified.runId, r.id);
});

test("executeTuple: no history -> dispatch-confirmed with full tuple inputs", async () => {
  const landed = run({ id: 7777, status: "queued", conclusion: null, triggering_actor: { login: BOT } });
  const { outcome, report, calls } = await executeTupleFixture({
    refReads: [SHA_A, SHA_A],
    repoRefReads: { [AL_REPO]: [AL_SHA, AL_SHA], [SV_REPO]: [SV_SHA, SV_SHA] },
    listCalls: [[[]], [[landed]]],
  });
  assert.equal(outcome, "dispatch-confirmed");
  assert.deepEqual(calls.dispatches, [
    {
      ref: "main",
      inputs: {
        ruviform_rev: SHA_A,
        audiolens_ref: "main",
        audiolens_expected_sha: AL_SHA,
        synvelis_ref: "main",
        synvelis_expected_sha: SV_SHA,
      },
    },
  ]);
  assert.equal(report.dispatch.confirmedRun.id, landed.id);
  assert.equal(report.observedShas.length, 6); // 3 resolutions x (initial + recheck)
});

test("executeTuple: in-flight run suppresses dispatch", async () => {
  const { outcome, calls } = await executeTupleFixture({
    refReads: [SHA_A],
    repoRefReads: { [AL_REPO]: [AL_SHA], [SV_REPO]: [SV_SHA] },
    listCalls: [[run({ status: "in_progress", conclusion: null })]],
  });
  assert.equal(outcome, "suppress-in-flight");
  assert.equal(calls.dispatches.length, 0);
});

test("executeTuple: missing evidence artifact -> unverifiable, manual run -> dispatch", async () => {
  const r = run({ conclusion: "success", actor: { login: "alice" }, triggering_actor: { login: "alice" } });
  const landed = run({ status: "queued", conclusion: null });
  const { outcome, report, calls } = await executeTupleFixture({
    refReads: [SHA_A, SHA_A],
    repoRefReads: { [AL_REPO]: [AL_SHA, AL_SHA], [SV_REPO]: [SV_SHA, SV_SHA] },
    listCalls: [[r], [landed]],
    runArtifacts: { [String(r.id)]: { "1": { audiolens: "__missing__", synvelis: evidenceFor(r.id, "synvelis") } } },
  });
  assert.equal(outcome, "dispatch-confirmed");
  assert.equal(report.unverifiableCount, 1);
  assert.equal(calls.dispatches.length, 1);
});

test("executeTuple: expired + malformed + badzip evidence all unverifiable", async () => {
  for (const [consumer, marker] of [["audiolens", "__expired__"], ["audiolens", "__malformed__"], ["synvelis", "__badzip__"]]) {
    const r = run({ conclusion: "success", actor: { login: "alice" }, triggering_actor: { login: "alice" } });
    const landed = run({ status: "queued", conclusion: null });
    const artifacts = { audiolens: evidenceFor(r.id, "audiolens"), synvelis: evidenceFor(r.id, "synvelis") };
    artifacts[consumer] = marker;
    const { outcome, report } = await executeTupleFixture({
      refReads: [SHA_A, SHA_A],
      repoRefReads: { [AL_REPO]: [AL_SHA, AL_SHA], [SV_REPO]: [SV_SHA, SV_SHA] },
      listCalls: [[r], [landed]],
      runArtifacts: { [String(r.id)]: { "1": artifacts } },
    });
    assert.equal(outcome, "dispatch-confirmed", `${consumer}=${marker}`);
    assert.equal(report.unverifiableCount, 1, `${consumer}=${marker}`);
  }
});

test("executeTuple: proven different tuple -> non-target, dispatch proceeds", async () => {
  const r = run({ conclusion: "success", actor: { login: "alice" }, triggering_actor: { login: "alice" } });
  const landed = run({ status: "queued", conclusion: null });
  const { outcome, report } = await executeTupleFixture({
    refReads: [SHA_A, SHA_A],
    repoRefReads: { [AL_REPO]: [AL_SHA, AL_SHA], [SV_REPO]: [SV_SHA, SV_SHA] },
    listCalls: [[r], [landed]],
    runArtifacts: {
      [String(r.id)]: {
        "1": {
          audiolens: evidenceFor(r.id, "audiolens", { consumerSha: SHA_B }),
          synvelis: evidenceFor(r.id, "synvelis"),
        },
      },
    },
  });
  assert.equal(outcome, "dispatch-confirmed");
  assert.equal(report.unverifiableCount, 0);
  assert.equal(report.coveringAttemptsOnDecisionTuple.length, 0);
});

test("executeTuple: covering failure with 2 auto dispatches -> budget-exhausted", async () => {
  const auto = { actor: { login: BOT }, triggering_actor: { login: BOT } };
  const r1 = run({ id: 11, conclusion: "failure", created_at: "2026-11-05T02:00:00Z", ...auto });
  const r2 = run({ id: 10, conclusion: "failure", created_at: "2026-11-05T01:00:00Z", ...auto });
  const failEv = (id) => ({
    audiolens: evidenceFor(id, "audiolens", { status: "fail" }),
    synvelis: evidenceFor(id, "synvelis", { status: "fail" }),
  });
  const { outcome, exitCode, warnings, calls } = await executeTupleFixture({
    refReads: [SHA_A],
    repoRefReads: { [AL_REPO]: [AL_SHA], [SV_REPO]: [SV_SHA] },
    listCalls: [[r1, r2]],
    runArtifacts: { "11": { "1": failEv(11) }, "10": { "1": failEv(10) } },
  });
  assert.equal(outcome, "budget-exhausted");
  assert.equal(exitCode, 0);
  assert.equal(calls.dispatches.length, 0);
  assert.ok(warnings.length >= 1);
});

test("executeTuple: consumer SHA moves during recheck -> re-evaluates, then dispatches new tuple", async () => {
  // First resolution AL=AL_SHA; recheck resolves AL=AL_SHA2 -> new candidate,
  // history re-scanned (still empty) -> dispatch with NEW expected sha.
  const AL_SHA2 = "dddddddddddddddddddddddddddddddddddddddd";
  const landed = run({ id: 8888, status: "queued", conclusion: null, triggering_actor: { login: BOT } });
  const { outcome, calls } = await executeTupleFixture({
    refReads: [SHA_A, SHA_A],
    repoRefReads: { [AL_REPO]: [AL_SHA, AL_SHA2, AL_SHA2], [SV_REPO]: [SV_SHA, SV_SHA, SV_SHA] },
    listCalls: [[[]], [[]], [[landed]]],
  });
  assert.equal(outcome, "dispatch-confirmed");
  assert.equal(calls.dispatches[0].inputs.audiolens_expected_sha, AL_SHA2);
});

test("executeTuple: tuple keeps moving past recheck bound -> candidate-moved, no dispatch", async () => {
  const { outcome, calls } = await executeTupleFixture({
    refReads: [SHA_A, SHA_A, SHA_A, SHA_A],
    repoRefReads: {
      [AL_REPO]: [AL_SHA, "dddddddddddddddddddddddddddddddddddddddd", "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee", "ffffffffffffffffffffffffffffffffffffffff"],
      [SV_REPO]: [SV_SHA, SV_SHA, SV_SHA, SV_SHA],
    },
    listCalls: [[[]], [[]], [[]], [[]]],
  });
  assert.equal(outcome, "candidate-moved");
  assert.equal(calls.dispatches.length, 0);
});

test("executeTuple: consumer SHA resolution failure -> error, exit 1, no dispatch", async () => {
  const { outcome, exitCode, calls } = await executeTupleFixture({
    refReads: [SHA_A],
    repoRefErrors: { [AL_REPO]: { message: "ls-remote failed" } },
    repoRefReads: { [SV_REPO]: [SV_SHA] },
    listCalls: [[[]]],
  });
  assert.equal(outcome, "error");
  assert.equal(exitCode, 1);
  assert.equal(calls.dispatches.length, 0);
});

test("executeTuple: truncated history -> history-truncated, no dispatch", async () => {
  const { outcome, report, calls } = await executeTupleFixture({
    refReads: [SHA_A],
    repoRefReads: { [AL_REPO]: [AL_SHA], [SV_REPO]: [SV_SHA] },
    listCalls: [[run({ conclusion: "success" })]],
    truncated: true,
  });
  assert.equal(outcome, "history-truncated");
  assert.equal(report.historyTruncated, true);
  assert.equal(calls.dispatches.length, 0);
});

test("executeTuple: terminal runs exceed evidence bound -> evidence-truncated", async () => {
  const runs = Array.from({ length: 3 }, (_, i) => run({ id: 100 + i, conclusion: "failure" }));
  const { outcome, report, calls } = await executeTupleFixture({
    refReads: [SHA_A],
    repoRefReads: { [AL_REPO]: [AL_SHA], [SV_REPO]: [SV_SHA] },
    listCalls: [[runs]],
  }, { maxEvidenceRuns: 2 });
  assert.equal(outcome, "evidence-truncated");
  assert.equal(report.evidenceTruncated, true);
  assert.equal(calls.dispatches.length, 0);
});

test("executeTuple: dispatched run lands on different head_sha -> dispatch-landed-mismatch", async () => {
  const { outcome, warnings } = await executeTupleFixture({
    refReads: [SHA_A, SHA_A],
    repoRefReads: { [AL_REPO]: [AL_SHA, AL_SHA], [SV_REPO]: [SV_SHA, SV_SHA] },
    listCalls: [[[]], [[run({ id: 9999, head_sha: SHA_B, status: "queued", conclusion: null })]]],
  });
  assert.equal(outcome, "dispatch-landed-mismatch");
  assert.ok(warnings.some((w) => w.includes("landed")));
});

test("executeTuple: dispatch accepted but unobserved -> dispatched-unconfirmed", async () => {
  const { outcome } = await executeTupleFixture({
    refReads: [SHA_A, SHA_A],
    repoRefReads: { [AL_REPO]: [AL_SHA, AL_SHA], [SV_REPO]: [SV_SHA, SV_SHA] },
    listCalls: [[[]], [[]], [[]]],
  });
  assert.equal(outcome, "dispatched-unconfirmed");
});

test("executeTuple: dry-run decides without dispatching", async () => {
  const { outcome, calls } = await executeTupleFixture(
    {
      refReads: [SHA_A, SHA_A],
      repoRefReads: { [AL_REPO]: [AL_SHA, AL_SHA], [SV_REPO]: [SV_SHA, SV_SHA] },
      listCalls: [[[]]],
    },
    { dryRun: true },
  );
  assert.equal(outcome, "dry-run");
  assert.equal(calls.dispatches.length, 0);
});

test("executeTuple: markdownSummary renders tuple section", async () => {
  const r = run({ conclusion: "success" });
  const { report } = await executeTupleFixture({
    refReads: [SHA_A],
    repoRefReads: { [AL_REPO]: [AL_SHA], [SV_REPO]: [SV_SHA] },
    listCalls: [[r]],
    runArtifacts: { [String(r.id)]: { "1": { audiolens: evidenceFor(r.id, "audiolens"), synvelis: evidenceFor(r.id, "synvelis") } } },
  });
  const md = markdownSummary(report);
  assert.match(md, /daily-ci-tuple outcome: skip-verified/);
  assert.match(md, /contract `consumer-verify-tuple\/v1`/);
  assert.match(md, /audiolens=pass/);
});
