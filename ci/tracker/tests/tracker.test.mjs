// tracker.test.mjs — Issue #368 ci-issue-tracker contract tests.
// Every GitHub API interaction goes through an injected mock client — these
// tests never touch the network and never create/update/close real issues.
// The matrix pins the fail-closed invariants: stale/ambiguous completions
// cannot overwrite newer state, old/divergent/uncovering successes cannot
// close newer failures, cancelled/unknown is neither success nor failure,
// authoritative run-history reconciliation defeats dropped/out-of-order
// notifications, and API failures propagate instead of being swallowed.

import { test } from "node:test";
import assert from "node:assert/strict";
import {
  ALLOWED_EVENTS,
  classifyHistoryRecord,
  classifyRun,
  compareRunOrder,
  defaultScope,
  issueTitle,
  mergeMarkerStates,
  parseMarker,
  parseTrackerArgs,
  plan,
  renderBody,
  renderMarker,
  run,
  sanitizeConclusion,
  sanitizeSha,
  sanitizeText,
  sanitizeUrl,
  scopeMatches,
} from "../tracker.mjs";

const REPO = "scarecrow173/ruviform";
const SHA_A = "0123456789abcdef0123456789abcdef01234567";
const SHA_B = "89abcdef0123456789abcdef0123456789abcdef";
const SHA_C = "abcdef0123456789abcdef0123456789abcdef01";
const SHA_D = "def0123456789abcdef0123456789abcdef01234";
const RUN_URL = "https://github.com/scarecrow173/ruviform/actions/runs/123";
const runUrl = (id) => `https://github.com/scarecrow173/ruviform/actions/runs/${id}`;

const T1 = "2026-11-06T00:00:00Z";
const T2 = "2026-11-06T00:01:00Z";
const T3 = "2026-11-06T00:02:00Z";
const T4 = "2026-11-06T00:03:00Z";

const scope = defaultScope(REPO);

function payload(overrides = {}) {
  const wr = {
    id: 100,
    name: "ci",
    path: ".github/workflows/ci.yml",
    head_sha: SHA_A,
    head_branch: "main",
    event: "push",
    conclusion: "failure",
    html_url: RUN_URL,
    run_attempt: 1,
    updated_at: T1,
    status: "completed",
    head_repository: { full_name: REPO },
    ...(overrides.workflow_run ?? {}),
  };
  return { action: "completed", workflow_run: wr, ...rest(overrides) };
}
function rest(o) {
  const { workflow_run: _ignored, ...r } = o;
  return r;
}

// Raw runs-list entry (authoritative history fixture) — mirrors the shape
// GET /actions/workflows/{file}/runs returns, including workflow identity.
function histRun(id, sha, conclusion, updated_at, { attempt = 1, event = "push", status = "completed", branch = "main", repo = REPO, wfname = "ci", wfpath = ".github/workflows/ci.yml" } = {}) {
  return {
    id,
    head_sha: sha,
    head_branch: branch,
    event,
    conclusion,
    html_url: runUrl(id),
    run_attempt: attempt,
    updated_at,
    run_started_at: updated_at,
    status,
    path: wfpath,
    name: wfname,
    repository: { full_name: repo },
    head_repository: { full_name: repo, fork: false },
  };
}

function managedIssue(number, state, scopeOverrides = {}) {
  const s = { ...scope, ...scopeOverrides };
  const marker = JSON.parse(renderMarker(s, state).slice(`<!-- ci-issue-tracker:`.length, -4));
  return { number, body: renderMarker(s, state), marker };
}

// Marker state fixture in the current tuple shape.
function failureState(overrides = {}) {
  return {
    latest: { runId: 100, attempt: 1, updatedAt: T1 },
    failure: {
      sha: SHA_A,
      runId: 100,
      attempt: 1,
      updatedAt: T1,
      runUrl: RUN_URL,
      conclusion: "failure",
      firstSha: SHA_A,
      firstAt: T1,
      lastAt: T1,
      jobs: [],
    },
    history: [{ kind: "failure", runId: 100, attempt: 1, updatedAt: T1, sha: SHA_A, conclusion: "failure", at: T1 }],
    operationalNote: null,
    ...overrides,
  };
}

// Internal candidate shape used to drive plan() directly.
function cand(kind, { id, attempt = 1, updatedAt = T1, sha = SHA_A, conclusion = "failure", url = null } = {}, ambiguous = false) {
  return {
    kind,
    ambiguous,
    run: { id, sha, url: url ?? runUrl(id), conclusion, event: "push", attempt, updatedAt },
  };
}

// Records every call; per-method responses are configurable.
function mockClient({
  issues = [],
  compare = "ahead",
  compareMap = {},
  jobs = [],
  history = [],
  branchSha = SHA_C,
  failOn = null,
} = {}) {
  const calls = [];
  const maybeFail = (name) => {
    if (failOn === name) throw new Error(`mock ${name} failure`);
    calls.push(name);
  };
  return {
    calls,
    ensureLabel: async (name, color) => maybeFail("ensureLabel"),
    listOpenIssues: async (label) => {
      maybeFail("listOpenIssues");
      return issues;
    },
    getRunHistory: async ({ workflowFile, branch }) => {
      maybeFail("getRunHistory");
      return history;
    },
    getRunJobs: async (id) => {
      maybeFail("getRunJobs");
      return jobs;
    },
    compareCommits: async (b, h) => {
      maybeFail("compareCommits");
      return compareMap[`${b}...${h}`] ?? compare;
    },
    getBranchHead: async (b) => {
      maybeFail("getBranchHead");
      return { sha: branchSha };
    },
    createIssue: async ({ title, body, labels }) => {
      maybeFail("createIssue");
      calls.push(`createIssue:${title}`);
      return { number: 500 };
    },
    updateIssue: async (n, { title, body }) => {
      maybeFail("updateIssue");
      calls.push(`updateIssue:#${n}`);
      return { number: n };
    },
    addComment: async (n, body) => {
      maybeFail("addComment");
      calls.push(`addComment:#${n}`);
      return {};
    },
    closeIssue: async (n) => {
      maybeFail("closeIssue");
      calls.push(`closeIssue:#${n}`);
      return {};
    },
  };
}

// ------------------------------------------------------- classifyRun ----

test("classifyRun accepts a valid main push run", () => {
  const c = classifyRun(payload(), scope);
  assert.equal(c.kind, "failure");
  assert.equal(c.run.id, 100);
  assert.equal(c.run.sha, SHA_A);
  assert.equal(c.run.url, RUN_URL);
  assert.equal(c.run.updatedAt, T1);
});

test("classifyRun accepts workflow_dispatch (daily scheduler / manual)", () => {
  const c = classifyRun(payload({ workflow_run: { event: "workflow_dispatch" } }), scope);
  assert.equal(c.kind, "failure");
});

test("classifyRun skips non-main branches and non-repo runs", () => {
  assert.equal(classifyRun(payload({ workflow_run: { head_branch: "feature-x" } }), scope).kind, "skip");
  assert.equal(
    classifyRun(payload({ workflow_run: { head_repository: { full_name: "fork/ruviform" } } }), scope).kind,
    "skip",
  );
});

test("classifyRun skips non-allowed events and non-completed actions", () => {
  assert.equal(classifyRun(payload({ workflow_run: { event: "pull_request" } }), scope).kind, "skip");
  assert.equal(classifyRun(payload({ workflow_run: { event: "merge_group" } }), scope).kind, "skip");
  assert.equal(classifyRun(payload({ workflow_run: { event: "schedule" } }), scope).kind, "skip");
  assert.equal(classifyRun(payload({ action: "requested" }), scope).kind, "skip");
  assert.equal(classifyRun({}, scope).kind, "skip");
});

test("classifyRun rejects malformed run metadata", () => {
  assert.equal(classifyRun(payload({ workflow_run: { id: "abc" } }), scope).kind, "skip");
  assert.equal(classifyRun(payload({ workflow_run: { head_sha: "not-a-sha" } }), scope).kind, "skip");
  assert.equal(classifyRun(payload({ workflow_run: { id: null } }), scope).kind, "skip");
  assert.equal(classifyRun(payload({ workflow_run: { run_attempt: null } }), scope).kind, "skip");
});

test("classifyRun requires status=completed on a completed action", () => {
  // A missing or contradicting status is a malformed payload, never
  // silently treated as completed evidence.
  assert.equal(classifyRun(payload({ workflow_run: { status: "in_progress" } }), scope).kind, "skip");
  assert.equal(classifyRun(payload({ workflow_run: { status: "queued" } }), scope).kind, "skip");
  assert.equal(classifyRun(payload({ workflow_run: { status: null } }), scope).kind, "skip");
  assert.equal(classifyRun(payload({ workflow_run: { status: undefined } }), scope).kind, "skip");
  assert.equal(classifyRun(payload({ workflow_run: { status: "completed" } }), scope).kind, "failure");
});

test("classifyRun conclusion matrix", () => {
  const k = (conclusion) => classifyRun(payload({ workflow_run: { conclusion } }), scope).kind;
  assert.equal(k("success"), "success");
  assert.equal(k("failure"), "failure");
  assert.equal(k("timed_out"), "failure");
  assert.equal(k("startup_failure"), "failure");
  for (const c of ["cancelled", "skipped", "neutral", "stale", "action_required", null, "weird"]) {
    assert.equal(k(c), "operational", c);
  }
});

test("ALLOWED_EVENTS is the explicit push+workflow_dispatch allowlist", () => {
  assert.deepEqual(ALLOWED_EVENTS, ["push", "workflow_dispatch"]);
});

// ---------------------------------------------------------- ordering -----

test("compareRunOrder: same run id orders strictly by attempt", () => {
  const ref = { runId: 100, attempt: 1, updatedAt: T1 };
  assert.equal(compareRunOrder({ id: 100, attempt: 2, updatedAt: T1 }, ref), "newer");
  assert.equal(compareRunOrder({ id: 100, attempt: 1, updatedAt: T1 }, ref), "same");
  assert.equal(compareRunOrder({ id: 100, attempt: 1, updatedAt: T4 }, ref), "same");
});

test("compareRunOrder: different run ids order by updated_at (old-run rerun lands newer)", () => {
  const ref = { runId: 300, attempt: 1, updatedAt: T3 };
  // run 100's second attempt completed AFTER run 300 — (id,attempt) alone
  // would call this stale; updated_at proves it newer.
  assert.equal(compareRunOrder({ id: 100, attempt: 2, updatedAt: T4 }, ref), "newer");
  assert.equal(compareRunOrder({ id: 100, attempt: 2, updatedAt: T2 }, ref), "stale");
});

test("compareRunOrder: missing/equal timestamps on different ids are ambiguous, never success", () => {
  const ref = { runId: 300, attempt: 1, updatedAt: null };
  assert.equal(compareRunOrder({ id: 200, attempt: 1, updatedAt: T2 }, ref), "ambiguous");
  const ref2 = { runId: 300, attempt: 1, updatedAt: T2 };
  assert.equal(compareRunOrder({ id: 200, attempt: 1, updatedAt: T2 }, ref2), "ambiguous");
});

// ---------------------------------------------------------------- plan ---

test("failure + no open issue → create", () => {
  const c = classifyRun(payload(), scope);
  const actions = plan({
    classified: c,
    canonical: null,
    newest: cand("failure", { id: 100, updatedAt: T1 }),
    newestFailure: cand("failure", { id: 100, updatedAt: T1 }),
    jobs: [],
  });
  const create = actions.find((a) => a.type === "create");
  assert.ok(create);
  assert.equal(create.state.failure.sha, SHA_A);
  assert.deepEqual(create.state.latest, { runId: 100, attempt: 1, updatedAt: T1 });
  assert.equal(create.state.history.length, 1);
});

test("failure + open issue + newer evidence → update preserving first detection", () => {
  const canonical = managedIssue(42, failureState());
  const mergedState = canonical.marker.state;
  const nf = cand("failure", { id: 150, updatedAt: T2, sha: SHA_B });
  const actions = plan({
    canonical,
    mergedState,
    newest: nf,
    newestFailure: nf,
    jobs: [{ name: "verify", conclusion: "failure" }],
  });
  const update = actions.find((a) => a.type === "update");
  assert.ok(update);
  assert.equal(update.issue, 42);
  assert.equal(update.state.failure.sha, SHA_B);
  assert.equal(update.state.failure.runId, 150);
  assert.equal(update.state.failure.firstSha, SHA_A, "first detection sha preserved");
  assert.deepEqual(update.state.latest, { runId: 150, attempt: 1, updatedAt: T2 });
  assert.equal(update.state.history.at(-1).runId, 150);
});

test("failure + open issue + stale/ambiguous evidence → noop", () => {
  const canonical = managedIssue(42, failureState());
  const mergedState = canonical.marker.state;
  const stale = cand("failure", { id: 50, updatedAt: T1, sha: SHA_A });
  stale.run.updatedAt = "2026-11-05T00:00:00Z"; // provably older than stored T1
  const actions = plan({ canonical, mergedState, newest: stale, newestFailure: stale, jobs: [] });
  assert.ok(actions.every((a) => a.type === "noop"));
  assert.match(actions[0].reason, /stale-failure/);
});

test("success + open issue + all guards → close", () => {
  for (const cmp of ["identical", "ahead"]) {
    const canonical = managedIssue(42, failureState());
    const s = cand("success", { id: 300, updatedAt: T3, sha: SHA_B, conclusion: "success" });
    const nf = cand("failure", { id: 100, updatedAt: T1 });
    const actions = plan({
      canonical,
      mergedState: canonical.marker.state,
      newest: s,
      newestFailure: nf,
      coveringSuccess: s,
      coverage: cmp,
      mainCoverage: "ahead",
    });
    const close = actions.find((a) => a.type === "close-recovered");
    assert.ok(close, cmp);
    assert.equal(close.issue, 42);
    assert.equal(close.recovery.sha, SHA_B);
    assert.equal(close.recovery.attempt, 1);
  }
});

test("success + open issue + behind/diverged coverage → no close", () => {
  for (const cmp of ["behind", "diverged"]) {
    const canonical = managedIssue(42, failureState());
    const s = cand("success", { id: 300, updatedAt: T3, sha: SHA_B, conclusion: "success" });
    const nf = cand("failure", { id: 100, updatedAt: T1 });
    const actions = plan({
      canonical,
      mergedState: canonical.marker.state,
      newest: s,
      newestFailure: nf,
      coveringSuccess: s,
      coverage: cmp,
      mainCoverage: "ahead",
    });
    assert.ok(!actions.some((a) => a.type === "close-recovered"), cmp);
    assert.ok(actions.some((a) => a.type === "update"), cmp);
  }
});

test("success + open issue + success not covering current main tip → no close", () => {
  const canonical = managedIssue(42, failureState());
  const s = cand("success", { id: 300, updatedAt: T3, sha: SHA_B, conclusion: "success" });
  const nf = cand("failure", { id: 100, updatedAt: T1 });
  const actions = plan({
    canonical,
    mergedState: canonical.marker.state,
    newest: s,
    newestFailure: nf,
    coveringSuccess: s,
    coverage: "ahead",
    mainCoverage: "behind", // main moved past the success — tip unverified
  });
  assert.ok(!actions.some((a) => a.type === "close-recovered"));
  const update = actions.find((a) => a.type === "update");
  assert.ok(update);
  assert.match(update.state.operationalNote, /compare-main/);
});

test("success + open issue + ambiguous ordering → hold, never close", () => {
  const canonical = managedIssue(42, failureState());
  const s = cand("success", { id: 300, updatedAt: T3, sha: SHA_B, conclusion: "success" }, true);
  const actions = plan({
    canonical,
    mergedState: canonical.marker.state,
    newest: s,
    newestFailure: cand("failure", { id: 100, updatedAt: T1 }),
    coverage: "ahead",
    mainCoverage: "ahead",
  });
  assert.ok(!actions.some((a) => a.type === "close-recovered"));
  const update = actions.find((a) => a.type === "update");
  assert.ok(update);
  assert.match(update.state.operationalNote, /ambiguous/);
});

test("success + no open issue → noop", () => {
  const s = cand("success", { id: 300, updatedAt: T3, sha: SHA_B, conclusion: "success" });
  assert.ok(plan({ canonical: null, newest: s }).every((a) => a.type === "noop"));
});

test("operational + open issue + newer run → note update, never close/create", () => {
  const canonical = managedIssue(42, failureState());
  const op = cand("operational", { id: 300, updatedAt: T3, conclusion: "cancelled" });
  const actions = plan({
    canonical,
    mergedState: canonical.marker.state,
    newest: op,
    newestFailure: cand("failure", { id: 100, updatedAt: T1 }),
  });
  const update = actions.find((a) => a.type === "update");
  assert.ok(update);
  assert.deepEqual(update.state.latest, { runId: 300, attempt: 1, updatedAt: T3 });
  assert.match(update.state.operationalNote, /cancelled/);
  assert.equal(update.state.failure.sha, SHA_A, "failure state untouched");
  assert.ok(!actions.some((a) => a.type === "create" || a.type === "close-recovered"));
});

test("operational + stale run → noop", () => {
  const canonical = managedIssue(42, failureState({
    latest: { runId: 500, attempt: 1, updatedAt: T4 },
  }));
  const op = cand("operational", { id: 300, updatedAt: T3, conclusion: "cancelled" });
  assert.ok(
    plan({
      canonical,
      mergedState: canonical.marker.state,
      newest: op,
      newestFailure: cand("failure", { id: 100, updatedAt: T1 }),
    }).every((a) => a.type === "noop"),
  );
});

// ------------------------------------------------------- marker/sanitize -

test("marker roundtrip preserves scope and state", () => {
  const state = failureState();
  const body = renderBody(scope, state);
  const parsed = parseMarker(body);
  assert.ok(parsed);
  assert.ok(scopeMatches(parsed.scope, scope));
  assert.equal(parsed.state.failure.sha, SHA_A);
  assert.deepEqual(parsed.state.latest, { runId: 100, attempt: 1, updatedAt: T1 });
});

test("parseMarker accepts legacy flat latestRunId/latestAttempt markers", () => {
  const legacy = failureState();
  legacy.latestRunId = 100;
  legacy.latestAttempt = 2;
  delete legacy.latest;
  const parsed = parseMarker(renderMarker(scope, legacy));
  assert.ok(parsed);
  assert.deepEqual(parsed.state.latest, { runId: 100, attempt: 2, updatedAt: null });
});

test("parseMarker rejects missing/broken/foreign markers", () => {
  assert.equal(parseMarker("no marker here"), null);
  assert.equal(parseMarker("<!-- ci-issue-tracker:{broken -->"), null);
  assert.equal(parseMarker(renderMarker(scope, failureState()).replace('"v":1', '"v":2')), null);
  const foreign = parseMarker(
    renderMarker({ ...scope, branch: "release" }, failureState()),
  );
  assert.ok(foreign);
  assert.ok(!scopeMatches(foreign.scope, scope));
});

test("mergeMarkerStates adopts the newest failure wholesale and unions histories", () => {
  const older = failureState();
  const newer = failureState({
    latest: { runId: 400, attempt: 1, updatedAt: T4 },
    failure: { ...failureState().failure, sha: SHA_D, runId: 400, updatedAt: T4 },
    history: [{ kind: "failure", runId: 400, attempt: 1, updatedAt: T4, sha: SHA_D, conclusion: "failure", at: T4 }],
  });
  const merged = mergeMarkerStates([older, newer]);
  assert.equal(merged.failure.runId, 400);
  assert.equal(merged.failure.sha, SHA_D);
  assert.deepEqual(merged.latest, { runId: 400, attempt: 1, updatedAt: T4 });
  assert.equal(merged.history.length, 2);
  // order-independent
  const merged2 = mergeMarkerStates([newer, older]);
  assert.equal(merged2.failure.runId, 400);
});

test("sanitize helpers bound all API-derived values", () => {
  assert.equal(sanitizeSha(SHA_A), SHA_A);
  assert.equal(sanitizeSha("zzz"), null);
  assert.equal(sanitizeUrl(RUN_URL), RUN_URL);
  assert.equal(sanitizeUrl("https://evil.example.com/x"), null);
  assert.equal(sanitizeUrl("javascript:alert(1)"), null);
  assert.equal(sanitizeText("a|b`c\n d\x00e"), "abc de");
  assert.equal(sanitizeConclusion("failure"), "failure");
  assert.equal(sanitizeConclusion("bogus"), "unknown");
  const long = "x".repeat(200);
  assert.ok(sanitizeText(long).length <= 101);
});

test("job name injection cannot break the issue table", () => {
  const state = failureState();
  // sanitizeText output as produced by run()'s jobs mapping.
  state.failure.jobs = [
    { name: sanitizeText("verify | [x](https://evil) `code`\nnewline"), conclusion: "failure" },
  ];
  const body = renderBody(scope, state);
  const tableLine = body.split("\n").find((l) => l.startsWith("| verify"));
  assert.ok(tableLine, "sanitized job row exists");
  assert.ok(!tableLine.includes("`") && !tableLine.includes("\r"), tableLine);
  const parsed = parseMarker(body);
  assert.ok(parsed, "marker still parseable after hostile job name");
});

test("marker JSON cannot be broken out of by --> sequences in state", () => {
  const state = failureState();
  state.operationalNote = "note --> injected <b>html</b>";
  const body = renderBody(scope, state);
  const markerLine = body.split("\n")[0];
  assert.ok(markerLine.startsWith("<!--") && markerLine.endsWith("-->"), markerLine);
  // inside the comment payload there is no `-->` that could terminate it early.
  assert.ok(!markerLine.slice(4, -3).includes("-->"));
  const parsed = parseMarker(body);
  assert.equal(parsed.state.operationalNote, "note --> injected <b>html</b>");
});

// ------------------------------------------------------------------ run ---

test("run: skip events make zero API calls", async () => {
  const client = mockClient();
  const res = await run({ payload: payload({ workflow_run: { head_branch: "pr-1" } }), client, scope });
  assert.equal(res.classified.kind, "skip");
  assert.deepEqual(client.calls, []);
});

test("run: first failure creates one issue via ensureLabel→list→history→jobs→create", async () => {
  const client = mockClient({ issues: [], jobs: [{ name: "verify", conclusion: "failure" }] });
  const res = await run({ payload: payload(), client, scope });
  assert.ok(res.applied.some((a) => a.type === "create" && a.issue === 500));
  const order = client.calls.join(",");
  assert.ok(order.startsWith("ensureLabel,listOpenIssues,getRunHistory"));
  assert.ok(client.calls.includes("getRunJobs"));
  assert.ok(!client.calls.includes("compareCommits"), "no compare needed on lone failure");
});

test("run: foreign labeled issue is untouched and a fresh issue is created", async () => {
  const foreign = { number: 9, body: "human issue with the label", pull_request: undefined };
  const client = mockClient({ issues: [foreign] });
  const res = await run({ payload: payload(), client, scope });
  assert.ok(res.applied.some((a) => a.type === "create"));
  assert.ok(!client.calls.some((c) => c.includes("#9")), "foreign issue #9 never modified");
});

test("run: success closes covered issue via both compares", async () => {
  const issue = managedIssue(42, failureState());
  const success = histRun(300, SHA_B, "success", T3);
  const client = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [success, histRun(100, SHA_A, "failure", T1)],
    branchSha: SHA_B,
  });
  const res = await run({
    payload: payload({ workflow_run: { id: 300, conclusion: "success", head_sha: SHA_B, updated_at: T3 } }),
    client,
    scope,
  });
  assert.ok(res.applied.some((a) => a.type === "close-recovered" && a.issue === 42));
  assert.equal(client.calls.filter((c) => c === "compareCommits").length, 2, "failure-coverage + main-coverage");
  assert.ok(client.calls.includes("closeIssue:#42"));
  assert.ok(!client.calls.includes("getRunJobs"), "no jobs fetch needed on success");
});

test("run: behind success does not close", async () => {
  const issue = managedIssue(42, failureState());
  const client = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [histRun(300, SHA_B, "success", T3), histRun(100, SHA_A, "failure", T1)],
    compare: "behind",
    branchSha: SHA_B,
  });
  const res = await run({
    payload: payload({ workflow_run: { id: 300, conclusion: "success", head_sha: SHA_A, updated_at: T3 } }),
    client,
    scope,
  });
  assert.ok(!res.applied.some((a) => a.type === "close-recovered"));
  assert.ok(!client.calls.includes("closeIssue:#42"));
});

test("run: API failures propagate instead of being swallowed", async () => {
  for (const failOn of [
    "ensureLabel",
    "listOpenIssues",
    "getRunHistory",
    "getRunJobs",
    "updateIssue",
    "compareCommits",
  ]) {
    const issue = managedIssue(42, failureState());
    const client = mockClient({
      issues: failOn === "updateIssue" || failOn === "compareCommits" ? [{ number: 42, body: issue.body }] : [],
      history: [histRun(300, SHA_B, failOn === "compareCommits" ? "success" : "failure", T3)],
      compare: "ahead",
      branchSha: SHA_B,
      failOn,
    });
    const p =
      failOn === "compareCommits"
        ? payload({ workflow_run: { id: 300, conclusion: "success", head_sha: SHA_B, updated_at: T3 } })
        : payload({ workflow_run: { id: 300, updated_at: T3 } });
    await assert.rejects(() => run({ payload: p, client, scope }), new RegExp(failOn), failOn);
  }
});

// ------------------------------------------- coordinator regressions -----

// (1) Stale success must not close when the newest execution is
// cancelled/operational — even if compare says the old success covers the
// recorded failure's revision.
test("run: stale success does not close when a newer cancelled run exists", async () => {
  const stored = failureState({
    latest: { runId: 300, attempt: 1, updatedAt: T3 },
    operationalNote: "cancelled",
  });
  const issue = managedIssue(42, stored);
  const client = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [
      histRun(300, SHA_C, "cancelled", T3),
      histRun(200, SHA_B, "success", T2),
      histRun(100, SHA_A, "failure", T1),
    ],
    compare: "ahead", // compare(A→B)=ahead — the old buggy trigger
    branchSha: SHA_C,
  });
  const res = await run({
    payload: payload({ workflow_run: { id: 200, conclusion: "success", head_sha: SHA_B, updated_at: T2 } }),
    client,
    scope,
  });
  assert.ok(!res.applied.some((a) => a.type === "close-recovered"), "newest evidence is cancelled — no close");
  assert.ok(!client.calls.includes("closeIssue:#42"), "issue stays open");
  assert.equal(res.effective.kind, "operational");
});

// (1b) A success that does not cover the CURRENT main tip cannot close —
// main is verified, not merely the old failed revision.
test("run: success behind current main tip does not close", async () => {
  const issue = managedIssue(42, failureState());
  const client = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [histRun(200, SHA_B, "success", T2), histRun(100, SHA_A, "failure", T1)],
    compareMap: { [`${SHA_A}...${SHA_B}`]: "ahead", [`${SHA_C}...${SHA_B}`]: "behind" },
    branchSha: SHA_C, // main moved to C after the success
  });
  const res = await run({
    payload: payload({ workflow_run: { id: 200, conclusion: "success", head_sha: SHA_B, updated_at: T2 } }),
    client,
    scope,
  });
  assert.ok(!res.applied.some((a) => a.type === "close-recovered"));
  assert.ok(!client.calls.includes("closeIssue:#42"));
});

// (1c) When the branch head cannot be fetched, a close-candidate run fails
// the job instead of silently closing.
test("run: unavailable branch head on a close candidate propagates, never closes", async () => {
  const issue = managedIssue(42, failureState());
  const client = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [histRun(200, SHA_B, "success", T2), histRun(100, SHA_A, "failure", T1)],
    failOn: "getBranchHead",
  });
  await assert.rejects(
    run({
      payload: payload({ workflow_run: { id: 200, conclusion: "success", head_sha: SHA_B, updated_at: T2 } }),
      client,
      scope,
    }),
    /getBranchHead/,
  );
  assert.ok(!client.calls.includes("closeIssue:#42"));
});

// (2) A delayed old failure event must not resurrect an issue that a newer
// covering success already resolved — arrival order is not truth.
test("run: delayed stale failure does not create when a newer success covers it", async () => {
  const client = mockClient({
    issues: [],
    history: [histRun(200, SHA_B, "success", T2), histRun(100, SHA_A, "failure", T1)],
    compare: "ahead",
  });
  const res = await run({ payload: payload(), client, scope }); // failure run 100 @T1
  assert.ok(res.applied.every((a) => a.type === "noop"));
  assert.ok(!client.calls.some((c) => c.startsWith("createIssue")), "stale failure does not file");
});

test("run: delayed stale failure still creates when the newer success does not cover it", async () => {
  const client = mockClient({
    issues: [],
    history: [histRun(200, SHA_B, "success", T2), histRun(100, SHA_A, "failure", T1)],
    compareMap: { [`${SHA_A}...${SHA_B}`]: "diverged" },
  });
  const res = await run({ payload: payload(), client, scope });
  assert.ok(res.applied.some((a) => a.type === "create"));
});

test("run: missing history evidence is fail-closed — the event itself still files", async () => {
  const client = mockClient({ issues: [], history: [] });
  const res = await run({ payload: payload(), client, scope });
  assert.ok(res.applied.some((a) => a.type === "create"));
});

// (3) Duplicate consolidation must merge the newest marker state FIRST —
// the newer failure held by a duplicate can never be lost to an old success.
test("run: duplicate holding the newest failure survives consolidation — no close", async () => {
  const low = managedIssue(10, failureState()); // failure run 100 @T1
  const high = managedIssue(
    77,
    failureState({
      latest: { runId: 400, attempt: 1, updatedAt: T4 },
      failure: { ...failureState().failure, runId: 400, sha: SHA_D, updatedAt: T4, runUrl: runUrl(400) },
      history: [{ kind: "failure", runId: 400, attempt: 1, updatedAt: T4, sha: SHA_D, conclusion: "failure", at: T4 }],
    }),
  );
  const client = mockClient({
    issues: [
      { number: 10, body: low.body },
      { number: 77, body: high.body },
    ],
    history: [
      histRun(400, SHA_D, "failure", T4),
      histRun(200, SHA_B, "success", T2),
      histRun(100, SHA_A, "failure", T1),
    ],
    compare: "ahead",
  });
  const res = await run({
    payload: payload({ workflow_run: { id: 200, conclusion: "success", head_sha: SHA_B, updated_at: T2 } }),
    client,
    scope,
  });
  assert.ok(!res.applied.some((a) => a.type === "close-recovered"), "old success never closes newer failure");
  assert.ok(!client.calls.includes("closeIssue:#10"), "canonical stays open");
  assert.ok(client.calls.includes("closeIssue:#77"), "duplicate consolidated");
  const write = res.applied.find((a) => (a.type === "merge-state" || a.type === "update") && a.issue === 10);
  assert.ok(write, "canonical adopts merged newest state");
  assert.equal(write.state.failure.runId, 400, "newest failure preserved");
});

// (4) An OLD run's rerun can complete after newer runs — (id,attempt) alone
// under-orders it. updated_at-aware ordering must treat it as new evidence.
test("run: old run rerun completing later updates instead of going stale", async () => {
  const stored = failureState({
    latest: { runId: 300, attempt: 1, updatedAt: T3 },
  });
  const issue = managedIssue(42, stored);
  const client = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [
      histRun(100, SHA_A, "failure", T4, { attempt: 2 }), // old run's new attempt lands last
      histRun(300, SHA_B, "success", T3),
    ],
    jobs: [{ name: "verify", conclusion: "failure" }],
  });
  const res = await run({
    payload: payload({
      workflow_run: { id: 100, run_attempt: 2, conclusion: "failure", head_sha: SHA_A, updated_at: T4 },
    }),
    client,
    scope,
  });
  const update = res.applied.find((a) => a.type === "update" && a.issue === 42);
  assert.ok(update, "rerun treated as newest evidence, not stale");
  assert.equal(update.state.failure.runId, 100);
  assert.equal(update.state.failure.attempt, 2);
  assert.deepEqual(update.state.latest, { runId: 100, attempt: 2, updatedAt: T4 });
});

test("run: ambiguous-order success holds — never closes", async () => {
  const issue = managedIssue(42, failureState());
  const client = mockClient({
    issues: [{ number: 42, body: issue.body }],
    // equal updated_at on different runs → order unprovable
    history: [histRun(400, SHA_C, "failure", T2), histRun(200, SHA_B, "success", T2), histRun(100, SHA_A, "failure", T1)],
    compare: "ahead",
  });
  const res = await run({
    payload: payload({ workflow_run: { id: 200, conclusion: "success", head_sha: SHA_B, updated_at: T2 } }),
    client,
    scope,
  });
  assert.ok(!res.applied.some((a) => a.type === "close-recovered"));
  assert.ok(!client.calls.includes("closeIssue:#42"));
});

// (5) Pending replacement drops notifications: a queued workflow_run event
// can be superseded before the tracker sees it. History reconciliation must
// recover the dropped failure even when the incoming event is operational.
test("run: dropped failure notification is recovered from authoritative history", async () => {
  const issue = managedIssue(42, failureState());
  const client = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [
      histRun(400, SHA_D, "failure", T4), // event dropped by pending replacement
      histRun(300, SHA_C, "cancelled", T3),
      histRun(100, SHA_A, "failure", T1),
    ],
    jobs: [{ name: "verify", conclusion: "failure" }],
  });
  const res = await run({
    payload: payload({ workflow_run: { id: 300, conclusion: "cancelled", head_sha: SHA_C, updated_at: T3 } }),
    client,
    scope,
  });
  const update = res.applied.find((a) => a.type === "update" && a.issue === 42);
  assert.ok(update, "dropped failure recovered");
  assert.equal(update.state.failure.runId, 400);
  assert.deepEqual(update.state.latest, { runId: 400, attempt: 1, updatedAt: T4 });
});

// (6) Ordering keys are atomic tuples — never field-wise maxima. A stored
// latest of (run 300, attempt 5) must not synthesize (run 400, attempt 5).
test("run: latest is adopted as one atomic tuple, never maxed per-field", async () => {
  const issue = managedIssue(
    42,
    failureState({ latest: { runId: 300, attempt: 5, updatedAt: T3 } }),
  );
  const client = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [histRun(400, SHA_D, "failure", T4), histRun(300, SHA_B, "failure", T3, { attempt: 5 }), histRun(100, SHA_A, "failure", T1)],
    jobs: [],
  });
  const res = await run({
    payload: payload({ workflow_run: { id: 400, head_sha: SHA_D, updated_at: T4 } }),
    client,
    scope,
  });
  const update = res.applied.find((a) => a.type === "update" && a.issue === 42);
  assert.ok(update);
  assert.deepEqual(
    update.state.latest,
    { runId: 400, attempt: 1, updatedAt: T4 },
    "tuple adopted wholesale from the actual run — attempt is not carried over",
  );
});

// History runs must match this repo's workflow scope: foreign-repo or
// wrong-branch entries cannot become evidence.
test("run: out-of-scope history runs are ignored as evidence", async () => {
  const client = mockClient({
    issues: [],
    history: [
      histRun(400, SHA_D, "failure", T4, { branch: "release" }),
      histRun(300, SHA_C, "failure", T3, { repo: "fork/ruviform" }),
      histRun(200, SHA_B, "success", T2, { event: "pull_request" }),
    ],
  });
  const res = await run({ payload: payload(), client, scope });
  assert.ok(res.applied.some((a) => a.type === "create"), "only the in-scope incoming run counts");
});

// (7) A NEWER run that has not completed yet (queued/in_progress/
// requested/waiting/pending/unknown status) is live evidence — its
// completion lies in the future, so an older success can never close.
test("run: newer in_progress run in history blocks close", async () => {
  const issue = managedIssue(42, failureState());
  const client = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [
      histRun(300, SHA_C, null, T3, { status: "in_progress" }),
      histRun(200, SHA_B, "success", T2),
      histRun(100, SHA_A, "failure", T1),
    ],
    compare: "ahead",
    branchSha: SHA_A, // even with main == the failed sha, no close
  });
  const res = await run({
    payload: payload({ workflow_run: { id: 200, conclusion: "success", head_sha: SHA_B, updated_at: T2 } }),
    client,
    scope,
  });
  assert.ok(!res.applied.some((a) => a.type === "close-recovered"), "live run blocks close");
  assert.ok(!client.calls.includes("closeIssue:#42"), "issue stays open");
  assert.equal(res.effective.kind, "pending");
});

test("run: every non-completed status in history blocks close", async () => {
  const issue = managedIssue(42, failureState());
  for (const status of ["queued", "in_progress", "requested", "waiting", "pending", "made_up_status"]) {
    const client = mockClient({
      issues: [{ number: 42, body: issue.body }],
      history: [
        histRun(300, SHA_C, null, T3, { status }),
        histRun(200, SHA_B, "success", T2),
        histRun(100, SHA_A, "failure", T1),
      ],
      compare: "ahead",
      branchSha: SHA_C,
    });
    const res = await run({
      payload: payload({ workflow_run: { id: 200, conclusion: "success", head_sha: SHA_B, updated_at: T2 } }),
      client,
      scope,
    });
    assert.ok(!res.applied.some((a) => a.type === "close-recovered"), status);
    assert.ok(!client.calls.includes("closeIssue:#42"), status);
  }
});

test("run: an in_progress rerun of the failed run itself prevents close", async () => {
  const issue = managedIssue(42, failureState());
  const client = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [
      histRun(100, SHA_A, null, T4, { attempt: 2, status: "in_progress" }), // rerun still live
      histRun(200, SHA_B, "success", T2),
    ],
    compare: "ahead",
    branchSha: SHA_A,
  });
  const res = await run({
    payload: payload({ workflow_run: { id: 200, conclusion: "success", head_sha: SHA_B, updated_at: T2 } }),
    client,
    scope,
  });
  assert.ok(!res.applied.some((a) => a.type === "close-recovered"));
  assert.ok(!client.calls.includes("closeIssue:#42"));
  const update = res.applied.find((a) => a.type === "update" && a.issue === 42);
  assert.ok(update, "live rerun is recorded, not closed");
  assert.equal(update.state.failure.runId, 100, "recorded failure preserved");
  assert.deepEqual(update.state.latest, { runId: 100, attempt: 2, updatedAt: T4 });
});

// (8) A history record that cannot be classified — missing scope fields,
// missing ordering fields, or a bare malformed object — is incomplete
// evidence: it blocks every close and never becomes success.
test("run: malformed history record holds — never closes", async () => {
  const issue = managedIssue(42, failureState());
  const client = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [histRun(200, SHA_B, "success", T2), histRun(100, SHA_A, "failure", T1), {}],
    compare: "ahead",
    branchSha: SHA_C,
  });
  const res = await run({
    payload: payload({ workflow_run: { id: 200, conclusion: "success", head_sha: SHA_B, updated_at: T2 } }),
    client,
    scope,
  });
  assert.ok(!res.applied.some((a) => a.type === "close-recovered"), "unclassifiable record blocks close");
  assert.ok(!client.calls.includes("closeIssue:#42"));
  const update = res.applied.find((a) => a.type === "update" && a.issue === 42);
  assert.ok(update, "held success is recorded, not closed");
  assert.equal(update.state.failure.runId, 100);
});

test("run: every missing-essential-field history record holds close", async () => {
  const issue = managedIssue(42, failureState());
  const variants = [
    (r) => ({ ...r, id: undefined }),            // no run id
    (r) => ({ ...r, updated_at: undefined }),    // no ordering timestamp
    (r) => ({ ...r, run_attempt: undefined }),   // no attempt
    (r) => ({ ...r, head_branch: undefined }),   // branch unverifiable
    (r) => ({ ...r, head_repository: undefined }),// repo unverifiable
    (r) => ({ ...r, event: undefined }),         // event unverifiable
    (r) => ({ ...r, status: undefined }),        // completed? unverifiable
    (r) => ({ ...r, path: undefined, name: undefined }), // workflow unverifiable
    () => null,
    () => "garbage",
  ];
  for (const make of variants) {
    const client = mockClient({
      issues: [{ number: 42, body: issue.body }],
      history: [
        make(histRun(300, SHA_C, null, T3)),
        histRun(200, SHA_B, "success", T2),
        histRun(100, SHA_A, "failure", T1),
      ],
      compare: "ahead",
      branchSha: SHA_C,
    });
    const res = await run({
      payload: payload({ workflow_run: { id: 200, conclusion: "success", head_sha: SHA_B, updated_at: T2 } }),
      client,
      scope,
    });
    assert.ok(!res.applied.some((a) => a.type === "close-recovered"), JSON.stringify(make(histRun(300, SHA_C, null, T3))));
    assert.ok(!client.calls.includes("closeIssue:#42"));
  }
});

test("run: non-array history response throws — missing evidence is fail-closed", async () => {
  const issue = managedIssue(42, failureState());
  const client = mockClient({ issues: [{ number: 42, body: issue.body }], history: null });
  await assert.rejects(
    run({
      payload: payload({ workflow_run: { id: 200, conclusion: "success", head_sha: SHA_B, updated_at: T2 } }),
      client,
      scope,
    }),
    /run history unavailable/,
  );
  assert.ok(!client.calls.includes("closeIssue:#42"));
});

// Unclassifiable evidence also prevents resolved-suppression: it could
// hide a newer failure, so the real failure evidence still files.
test("run: unclassifiable history prevents stale-failure suppression — issue files", async () => {
  const client = mockClient({
    issues: [],
    history: [histRun(200, SHA_B, "success", T2), histRun(100, SHA_A, "failure", T1), {}],
    compare: "ahead",
  });
  const res = await run({ payload: payload(), client, scope }); // delayed failure 100
  assert.ok(res.applied.some((a) => a.type === "create"), "incomplete evidence cannot prove resolution");
});

// A live pending run does NOT undo proven coverage — the stale failure
// stays suppressed and the pending run reconciles on its own completion
// event.
test("run: pending run does not resurrect a resolved stale failure", async () => {
  const client = mockClient({
    issues: [],
    history: [
      histRun(300, SHA_C, null, T3, { status: "queued" }),
      histRun(200, SHA_B, "success", T2),
      histRun(100, SHA_A, "failure", T1),
    ],
    compare: "ahead",
  });
  const res = await run({ payload: payload(), client, scope });
  assert.ok(res.applied.every((a) => a.type === "noop"));
  assert.ok(!client.calls.some((c) => c.startsWith("createIssue")));
});

// An unknown conclusion on a COMPLETED run is operational — never success.
test("run: unknown conclusion on a completed run blocks close", async () => {
  const issue = managedIssue(42, failureState());
  const client = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [
      histRun(300, SHA_C, "inconclusive", T3),
      histRun(200, SHA_B, "success", T2),
      histRun(100, SHA_A, "failure", T1),
    ],
    compare: "ahead",
    branchSha: SHA_C,
  });
  const res = await run({
    payload: payload({ workflow_run: { id: 200, conclusion: "success", head_sha: SHA_B, updated_at: T2 } }),
    client,
    scope,
  });
  assert.ok(!res.applied.some((a) => a.type === "close-recovered"));
  assert.ok(!client.calls.includes("closeIssue:#42"));
  assert.equal(res.effective.kind, "operational");
});

// Clearly out-of-scope records stay excluded — they never hold a valid
// close back (the fail-closed net must not over-block).
test("run: definite out-of-scope records do not block a valid close", async () => {
  const issue = managedIssue(42, failureState());
  const client = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [
      histRun(400, SHA_D, null, T4, { status: "in_progress", branch: "release" }),
      histRun(401, SHA_D, null, T4, { repo: "fork/ruviform" }),
      histRun(402, SHA_D, "success", T4, { event: "pull_request" }),
      { ...histRun(403, SHA_D, "success", T4), path: ".github/workflows/other.yml" },
      histRun(200, SHA_B, "success", T2),
      histRun(100, SHA_A, "failure", T1),
    ],
    compare: "ahead",
    branchSha: SHA_C,
  });
  const res = await run({
    payload: payload({ workflow_run: { id: 200, conclusion: "success", head_sha: SHA_B, updated_at: T2 } }),
    client,
    scope,
  });
  assert.ok(res.applied.some((a) => a.type === "close-recovered"), "excluded records leave the close intact");
  assert.ok(client.calls.includes("closeIssue:#42"));
});

// (9) Attempt-level evidence: the runs list carries only each run's LATEST
// attempt, so per-id merging + per-id marker suppression can erase the
// recorded failure attempt (newestFailure = null → compare-failure:
// unavailable → the issue can never auto-close). The recorded failure
// attempt must survive as distinct evidence so a same-id rerun success
// can close its own failed attempt (R6 rerun recovery).
test("run: same-id rerun success closes the recorded failure attempt", async () => {
  const issue = managedIssue(42, failureState()); // failure run 100 attempt 1 @ SHA_A
  const client = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [histRun(100, SHA_A, "success", T4, { attempt: 2 })], // list shows latest attempt only
    compareMap: { [`${SHA_A}...${SHA_A}`]: "identical" },
    branchSha: SHA_A,
  });
  const res = await run({
    payload: payload({ workflow_run: { id: 100, run_attempt: 2, conclusion: "success", head_sha: SHA_A, updated_at: T4 } }),
    client,
    scope,
  });
  assert.ok(res.applied.some((a) => a.type === "close-recovered"), "rerun success closes its own failed attempt");
  assert.ok(client.calls.includes("closeIssue:#42"));
});

// A rerun on a different sha still requires commit coverage of the failed
// revision — different-sha close is allowed only when compare covers.
test("run: same-id rerun on a different sha closes only with coverage", async () => {
  const issue = managedIssue(42, failureState());
  const covered = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [histRun(100, SHA_D, "success", T4, { attempt: 2 })],
    compareMap: { [`${SHA_A}...${SHA_D}`]: "ahead", [`${SHA_D}...${SHA_D}`]: "identical" },
    branchSha: SHA_D,
  });
  const res = await run({
    payload: payload({ workflow_run: { id: 100, run_attempt: 2, conclusion: "success", head_sha: SHA_D, updated_at: T4 } }),
    client: covered,
    scope,
  });
  assert.ok(res.applied.some((a) => a.type === "close-recovered"), "covered different-sha rerun closes");

  const uncovered = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [histRun(100, SHA_D, "success", T4, { attempt: 2 })],
    compareMap: { [`${SHA_A}...${SHA_D}`]: "diverged" },
    branchSha: SHA_D,
  });
  const res2 = await run({
    payload: payload({ workflow_run: { id: 100, run_attempt: 2, conclusion: "success", head_sha: SHA_D, updated_at: T4 } }),
    client: uncovered,
    scope,
  });
  assert.ok(!res2.applied.some((a) => a.type === "close-recovered"), "diverged sha never closes");
  assert.ok(!uncovered.calls.includes("closeIssue:#42"));
});

// Same-id rerun that fails again updates the recorded attempt; cancelled /
// in-progress reruns hold — the recorded failure attempt is never closed
// by them and never lost.
test("run: same-id rerun failure updates the recorded attempt", async () => {
  const issue = managedIssue(42, failureState());
  const client = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [histRun(100, SHA_A, "failure", T4, { attempt: 2 })],
    jobs: [{ name: "verify", conclusion: "failure" }],
  });
  const res = await run({
    payload: payload({ workflow_run: { id: 100, run_attempt: 2, conclusion: "failure", head_sha: SHA_A, updated_at: T4 } }),
    client,
    scope,
  });
  const update = res.applied.find((a) => a.type === "update" && a.issue === 42);
  assert.ok(update);
  assert.equal(update.state.failure.runId, 100);
  assert.equal(update.state.failure.attempt, 2);
});

test("run: same-id rerun cancelled holds the recorded failure attempt", async () => {
  const issue = managedIssue(42, failureState());
  const client = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [histRun(100, SHA_A, "cancelled", T4, { attempt: 2 })],
    compare: "identical",
    branchSha: SHA_A,
  });
  const res = await run({
    payload: payload({ workflow_run: { id: 100, run_attempt: 2, conclusion: "cancelled", head_sha: SHA_A, updated_at: T4 } }),
    client,
    scope,
  });
  assert.ok(!res.applied.some((a) => a.type === "close-recovered"));
  assert.ok(!client.calls.includes("closeIssue:#42"));
  const update = res.applied.find((a) => a.type === "update" && a.issue === 42);
  assert.ok(update);
  assert.equal(update.state.failure.runId, 100, "failure attempt preserved");
});

test("run: same-id rerun in progress holds the recorded failure attempt", async () => {
  const issue = managedIssue(42, failureState());
  const client = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [histRun(100, SHA_A, null, T4, { attempt: 2, status: "in_progress" })],
    compare: "identical",
    branchSha: SHA_A,
  });
  const res = await run({
    payload: payload({ workflow_run: { id: 200, conclusion: "success", head_sha: SHA_B, updated_at: T2 } }),
    client,
    scope,
  });
  assert.ok(!res.applied.some((a) => a.type === "close-recovered"));
  assert.ok(!client.calls.includes("closeIssue:#42"));
});

// A success on a DIFFERENT, older run can never close the recorded
// failure attempt — attempt-level preservation must not weaken ordering.
test("run: older different-run success does not close the recorded attempt", async () => {
  const issue = managedIssue(42, failureState());
  const client = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [histRun(50, SHA_B, "success", "2026-11-04T00:00:00Z")],
    compare: "ahead",
    branchSha: SHA_C,
  });
  const res = await run({
    payload: payload({ workflow_run: { id: 50, conclusion: "success", head_sha: SHA_B, updated_at: "2026-11-04T00:00:00Z" } }),
    client,
    scope,
  });
  assert.ok(!res.applied.some((a) => a.type === "close-recovered"));
  assert.ok(!client.calls.includes("closeIssue:#42"));
});

// Attempt preservation is per (id, attempt) — an unrelated newer failure
// run still dominates a same-id success.
test("run: same-id success does not close while a newer different-run failure exists", async () => {
  const issue = managedIssue(42, failureState());
  const client = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [
      histRun(100, SHA_A, "success", T4, { attempt: 2 }),
      histRun(300, SHA_B, "failure", T3),
    ],
    compareMap: { [`${SHA_B}...${SHA_A}`]: "behind" },
    branchSha: SHA_B,
  });
  const res = await run({
    payload: payload({ workflow_run: { id: 100, run_attempt: 2, conclusion: "success", head_sha: SHA_A, updated_at: T4 } }),
    client,
    scope,
  });
  assert.ok(!res.applied.some((a) => a.type === "close-recovered"));
  assert.ok(!client.calls.includes("closeIssue:#42"));
  const update = res.applied.find((a) => a.type === "update" && a.issue === 42);
  assert.equal(update?.state.failure.runId, 300, "newest failure is the unrelated run");
});

// ------------------------------------------------------- reconcile mode ---
// Issue #368 notification-gap fill: GITHUB_TOKEN-dispatched `ci` completions
// fire no workflow_run event, so a second entry runs the SAME evaluator with
// no event payload. `run({payload: null, reconcile: true})` must reconcile
// purely from authoritative history + marker state — identical fail-closed
// contract, no event candidate.

// The #375 incident shape: tracker opened the issue on a real failure, the
// scheduler's bot dispatch then succeeded with no event — reconcile closes
// it from authoritative history alone.
test("reconcile: bot success in history closes the open failure issue", async () => {
  const issue = managedIssue(42, failureState());
  const client = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [histRun(200, SHA_B, "success", T2, { event: "workflow_dispatch" }), histRun(100, SHA_A, "failure", T1)],
    compare: "ahead",
    branchSha: SHA_B,
  });
  const res = await run({ payload: null, reconcile: true, client, scope });
  assert.equal(res.classified.kind, "reconcile");
  assert.ok(res.applied.some((a) => a.type === "close-recovered" && a.issue === 42));
  assert.ok(client.calls.includes("closeIssue:#42"));
});

// The gap is symmetric: a bot-dispatched FAILURE also fires no event —
// reconcile must file it even though no payload ever arrived.
test("reconcile: unresolved bot failure in history creates an issue", async () => {
  const client = mockClient({
    issues: [],
    history: [histRun(300, SHA_C, "failure", T3, { event: "workflow_dispatch" })],
    jobs: [{ name: "verify", conclusion: "failure" }],
  });
  const res = await run({ payload: null, reconcile: true, client, scope });
  assert.equal(res.classified.kind, "reconcile");
  assert.ok(res.applied.some((a) => a.type === "create" && a.issue === 500));
  assert.ok(client.calls.includes("getRunJobs"));
});

test("reconcile: success-only history with no issue is a noop", async () => {
  const client = mockClient({
    issues: [],
    history: [histRun(200, SHA_B, "success", T2, { event: "workflow_dispatch" })],
  });
  const res = await run({ payload: null, reconcile: true, client, scope });
  assert.equal(res.classified.kind, "reconcile");
  assert.ok(res.applied.every((a) => a.type === "noop"));
  assert.ok(!client.calls.some((c) => c.startsWith("createIssue")));
});

// Reconcile must not smuggle in an event candidate — a payload-less run has
// no `status:"completed"` self-evidence, only history + marker.
test("reconcile: live pending run in history still blocks close", async () => {
  const issue = managedIssue(42, failureState());
  const client = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [
      histRun(400, SHA_C, null, T4, { status: "in_progress", event: "workflow_dispatch" }),
      histRun(200, SHA_B, "success", T2),
      histRun(100, SHA_A, "failure", T1),
    ],
    compare: "ahead",
    branchSha: SHA_B,
  });
  const res = await run({ payload: null, reconcile: true, client, scope });
  assert.ok(!res.applied.some((a) => a.type === "close-recovered"));
  assert.ok(!client.calls.includes("closeIssue:#42"));
});

test("reconcile: unclassifiable history record blocks close", async () => {
  const issue = managedIssue(42, failureState());
  const client = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [{ garbage: true }, histRun(200, SHA_B, "success", T2), histRun(100, SHA_A, "failure", T1)],
    compare: "ahead",
    branchSha: SHA_B,
  });
  const res = await run({ payload: null, reconcile: true, client, scope });
  assert.ok(!res.applied.some((a) => a.type === "close-recovered"));
  assert.ok(!client.calls.includes("closeIssue:#42"));
});

test("reconcile: history API failure propagates", async () => {
  const client = mockClient({ failOn: "getRunHistory" });
  await assert.rejects(run({ payload: null, reconcile: true, client, scope }), /getRunHistory/);
});

test("reconcile: unavailable branch head on a close candidate propagates", async () => {
  const issue = managedIssue(42, failureState());
  const client = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [histRun(200, SHA_B, "success", T2), histRun(100, SHA_A, "failure", T1)],
    compare: "ahead",
    failOn: "getBranchHead",
  });
  await assert.rejects(run({ payload: null, reconcile: true, client, scope }), /getBranchHead/);
  assert.ok(!client.calls.includes("closeIssue:#42"));
});

// ------------------------------------------------- parseTrackerArgs -------
// Mode separation contract: --reconcile never consults the event path;
// ambient env paths are ignored, an explicit --event is a contradiction.

test("parseTrackerArgs: --reconcile ignores ambient event paths", () => {
  const r = parseTrackerArgs(["node", "tracker.mjs", "--reconcile"], {
    TRACKER_EVENT_PATH: "/tmp/event.json",
    GITHUB_EVENT_PATH: "/tmp/github-event.json",
    GITHUB_REPOSITORY: REPO,
  });
  assert.equal(r.ok, true);
  assert.equal(r.args.mode, "reconcile");
  assert.equal(r.args.event, null);
});

test("parseTrackerArgs: --reconcile with explicit --event is a usage error", () => {
  const r = parseTrackerArgs(["node", "tracker.mjs", "--reconcile", "--event", "/tmp/e.json"], {
    GITHUB_REPOSITORY: REPO,
  });
  assert.equal(r.ok, false);
});

test("parseTrackerArgs: default mode reads TRACKER_EVENT_PATH", () => {
  const r = parseTrackerArgs(["node", "tracker.mjs"], {
    TRACKER_EVENT_PATH: "/tmp/event.json",
    GITHUB_REPOSITORY: REPO,
  });
  assert.equal(r.ok, true);
  assert.equal(r.args.mode, "event");
  assert.equal(r.args.event, "/tmp/event.json");
});

test("parseTrackerArgs: no flag and no event path is a usage error", () => {
  const r = parseTrackerArgs(["node", "tracker.mjs"], { GITHUB_REPOSITORY: REPO });
  assert.equal(r.ok, false);
});

test("reconcile: passing an event payload is rejected (no cross-mode interpretation)", async () => {
  const client = mockClient({ history: [] });
  await assert.rejects(
    run({ payload: payload(), reconcile: true, client, scope }),
    /does not accept an event payload/,
  );
});

test("reconcile: result carries mode", async () => {
  const client = mockClient({ history: [] });
  const res = await run({ payload: null, reconcile: true, client, scope });
  assert.equal(res.mode, "reconcile");
  const ev = await run({ payload: payload(), client: mockClient({ history: [] }), scope });
  assert.equal(ev.mode, "event");
});

// Event mode boundary: a file that parses but is not a webhook object is
// corrupt input, not a valid skip — it must error rather than silently noop.
test("event mode: non-object payload is a hard error, not a skip", async () => {
  for (const bad of [null, "completed", 42, []]) {
    await assert.rejects(
      run({ payload: bad, client: mockClient({ history: [] }), scope }),
      /requires a JSON object event payload/,
    );
  }
});

// ------------------------------------------- consumer-verify scope (#294) --
//
// The tracker now watches TWO workflows. These tests pin the scope
// isolation contract: a `consumer-verify` completion is evidence ONLY for
// the consumer-verify scope (repo × workflow × branch marker + title), a
// `ci` payload can never feed it, and vice versa. consumer-verify is
// dispatch-only, so its allowed events collapse to workflow_dispatch in
// practice — the shared ALLOWED_EVENTS allowlist still applies unchanged.

const cvScope = {
  ...defaultScope(REPO),
  workflow: "consumer-verify",
  workflowFile: "consumer-verify.yml",
};

function cvPayload(overrides = {}) {
  return payload({
    workflow_run: {
      name: "consumer-verify",
      path: ".github/workflows/consumer-verify.yml",
      event: "workflow_dispatch",
      ...(overrides.workflow_run ?? {}),
    },
    ...rest(overrides),
  });
}

test("issueTitle: ci keeps the historical title, consumer-verify gets its own", () => {
  assert.equal(issueTitle(scope), "[CI failure][main] Full CI failed");
  assert.equal(issueTitle(cvScope), "[CI failure][main] consumer-verify failed");
  // Unknown workflow names fall back to the sanitized workflow id — never
  // to another scope's title.
  assert.equal(
    issueTitle({ ...scope, workflow: "mystery-ci", branch: "main" }),
    "[CI failure][main] mystery-ci failed",
  );
});

test("classifyRun: consumer-verify scope accepts its own dispatch run", () => {
  const c = classifyRun(cvPayload(), cvScope);
  assert.equal(c.kind, "failure");
  assert.equal(c.run.id, 100);
});

test("classifyRun: cross-scope payloads are skipped, never evidence", () => {
  // A consumer-verify completion must not feed the ci scope…
  assert.equal(classifyRun(cvPayload(), scope).kind, "skip");
  // …and a ci completion must not feed the consumer-verify scope.
  assert.equal(classifyRun(payload(), cvScope).kind, "skip");
});

test("classifyRun: workflow identity fields must match the scope", () => {
  // Same workflow name but a different file path → skip.
  assert.equal(
    classifyRun(
      payload({ workflow_run: { path: ".github/workflows/ci-copy.yml" } }),
      scope,
    ).kind,
    "skip",
  );
  // Same file path but a renamed workflow → skip.
  assert.equal(
    classifyRun(payload({ workflow_run: { name: "ci-renamed" } }), scope).kind,
    "skip",
  );
  // No identity fields at all → malformed, skip (reconcile heals).
  assert.equal(
    classifyRun(
      payload({ workflow_run: { name: null, path: null } }),
      scope,
    ).kind,
    "skip",
  );
  // A matching name alone is sufficient identity (path may be absent).
  assert.equal(
    classifyRun(payload({ workflow_run: { path: null } }), scope).kind,
    "failure",
  );
});

test("classifyHistoryRecord: consumer-verify history is in-scope only for its own scope", () => {
  const cv = histRun(200, SHA_B, "failure", T2, {
    event: "workflow_dispatch",
    wfname: "consumer-verify",
    wfpath: ".github/workflows/consumer-verify.yml",
  });
  // In scope for consumer-verify…
  const inScope = classifyHistoryRecord(cv, cvScope);
  assert.equal(inScope.disposition, "in");
  // …out of scope for ci (proven exclusion, not unknown).
  assert.equal(classifyHistoryRecord(cv, scope).disposition, "out");
  // And ci history is out of scope for consumer-verify.
  assert.equal(
    classifyHistoryRecord(histRun(100, SHA_A, "failure", T1), cvScope).disposition,
    "out",
  );
});

test("run: consumer-verify failure creates its own scoped issue", async () => {
  const client = mockClient({
    history: [
      histRun(200, SHA_B, "failure", T2, {
        event: "workflow_dispatch",
        wfname: "consumer-verify",
        wfpath: ".github/workflows/consumer-verify.yml",
      }),
    ],
    jobs: [{ name: "audiolens", conclusion: "failure" }],
  });
  const res = await run({ payload: cvPayload(), client, scope: cvScope });
  const create = res.applied.find((a) => a.type === "create");
  assert.ok(create, "consumer-verify failure must file an issue");
  assert.ok(client.calls.includes("createIssue:[CI failure][main] consumer-verify failed"));
  // The marker scope pins the workflow so ci-scope issues stay foreign.
  assert.equal(create.state.failure.runId, 200);
});

test("run: a ci-scoped managed issue is foreign to the consumer-verify scope", async () => {
  // An open issue carrying the ci marker must not be adopted, updated, or
  // closed by a consumer-verify evaluation — scopes never share issues.
  const ciIssue = managedIssue(42, failureState());
  const client = mockClient({
    issues: [{ number: 42, body: ciIssue.body }],
    history: [
      histRun(200, SHA_B, "failure", T2, {
        event: "workflow_dispatch",
        wfname: "consumer-verify",
        wfpath: ".github/workflows/consumer-verify.yml",
      }),
    ],
    jobs: [{ name: "synvelis", conclusion: "failure" }],
  });
  const res = await run({ payload: cvPayload(), client, scope: cvScope });
  assert.ok(res.applied.some((a) => a.type === "create"), "new scoped issue expected");
  assert.ok(!client.calls.some((c) => c.startsWith("updateIssue:#42")));
  assert.ok(!client.calls.some((c) => c.startsWith("closeIssue:#42")));
});

test("run: consumer-verify cancelled run is operational — no issue", async () => {
  // `cancelled` is neither pass nor fail: it must not create an issue and
  // must not count as recovery either.
  const client = mockClient({ history: [] });
  const res = await run({
    payload: cvPayload({ workflow_run: { conclusion: "cancelled" } }),
    client,
    scope: cvScope,
  });
  assert.ok(!res.applied.some((a) => a.type === "create" || a.type === "update"));
  assert.ok(!client.calls.some((c) => c.startsWith("createIssue:")));
});

test("run: consumer-verify reconcile recovers a dropped dispatch notification", async () => {
  // daily-ci dispatches consumer-verify with GITHUB_TOKEN → no
  // workflow_run event fires. Reconcile must still file the failure from
  // authoritative history alone.
  const client = mockClient({
    history: [
      histRun(200, SHA_B, "failure", T2, {
        event: "workflow_dispatch",
        wfname: "consumer-verify",
        wfpath: ".github/workflows/consumer-verify.yml",
      }),
    ],
    jobs: [{ name: "audiolens", conclusion: "failure" }],
  });
  const res = await run({ payload: null, reconcile: true, client, scope: cvScope });
  assert.equal(res.mode, "reconcile");
  assert.ok(res.applied.some((a) => a.type === "create"));
  assert.ok(client.calls.includes("createIssue:[CI failure][main] consumer-verify failed"));
});

test("run: consumer-verify success newer than failure closes the scoped issue", async () => {
  const cvState = failureState({
    latest: { runId: 200, attempt: 1, updatedAt: T2 },
    failure: {
      ...failureState().failure,
      sha: SHA_B,
      runId: 200,
      attempt: 1,
      updatedAt: T2,
      runUrl: runUrl(200),
      firstSha: SHA_B,
      firstAt: T2,
      lastAt: T2,
    },
    history: [{ kind: "failure", runId: 200, attempt: 1, updatedAt: T2, sha: SHA_B, conclusion: "failure", at: T2 }],
  });
  const cvIssueScope = { workflow: "consumer-verify", workflowFile: "consumer-verify.yml" };
  const issue = managedIssue(42, cvState, cvIssueScope);
  const client = mockClient({
    issues: [{ number: 42, body: issue.body }],
    history: [
      histRun(201, SHA_C, "success", T3, {
        event: "workflow_dispatch",
        wfname: "consumer-verify",
        wfpath: ".github/workflows/consumer-verify.yml",
      }),
      histRun(200, SHA_B, "failure", T2, {
        event: "workflow_dispatch",
        wfname: "consumer-verify",
        wfpath: ".github/workflows/consumer-verify.yml",
      }),
    ],
    compare: "ahead",
    branchSha: SHA_C,
  });
  const res = await run({
    payload: cvPayload({ workflow_run: { id: 201, conclusion: "success", head_sha: SHA_C, updated_at: T3 } }),
    client,
    scope: cvScope,
  });
  const close = res.applied.find((a) => a.type === "close-recovered" && a.issue === 42);
  assert.ok(close, "newer covering success must close the consumer-verify issue");
  assert.ok(client.calls.includes("closeIssue:#42"));
});
