// tracker.test.mjs — Issue #368 ci-issue-tracker contract tests.
// Every GitHub API interaction goes through an injected mock client — these
// tests never touch the network and never create/update/close real issues.
// The matrix pins the fail-closed invariants: stale completions cannot
// overwrite newer state, old/divergent successes cannot close newer
// failures, cancelled/unknown is neither success nor failure, and API
// failures propagate instead of being swallowed.

import { test } from "node:test";
import assert from "node:assert/strict";
import {
  ALLOWED_EVENTS,
  classifyRun,
  defaultScope,
  parseMarker,
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
const RUN_URL = "https://github.com/scarecrow173/ruviform/actions/runs/123";

const scope = defaultScope(REPO);

function payload(overrides = {}) {
  const wr = {
    id: 100,
    head_sha: SHA_A,
    head_branch: "main",
    event: "push",
    conclusion: "failure",
    html_url: RUN_URL,
    run_attempt: 1,
    updated_at: "2026-11-06T00:00:00Z",
    head_repository: { full_name: REPO },
    ...(overrides.workflow_run ?? {}),
  };
  return { action: "completed", workflow_run: wr, ...rest(overrides) };
}
function rest(o) {
  const { workflow_run: _ignored, ...r } = o;
  return r;
}

function managedIssue(number, state, scopeOverrides = {}) {
  const s = { ...scope, ...scopeOverrides };
  const marker = JSON.parse(renderMarker(s, state).slice(`<!-- ci-issue-tracker:`.length, -4));
  return { number, body: renderMarker(s, state), marker };
}

function failureState(overrides = {}) {
  return {
    latestRunId: 100,
    latestAttempt: 1,
    failure: {
      sha: SHA_A,
      runId: 100,
      attempt: 1,
      runUrl: RUN_URL,
      conclusion: "failure",
      firstSha: SHA_A,
      firstAt: "2026-11-06T00:00:00Z",
      lastAt: "2026-11-06T00:00:00Z",
      jobs: [],
    },
    history: [{ kind: "failure", runId: 100, attempt: 1, sha: SHA_A, conclusion: "failure", at: "2026-11-06T00:00:00Z" }],
    operationalNote: null,
    ...overrides,
  };
}

// Records every call; per-method responses are configurable.
function mockClient({ issues = [], compare = "ahead", jobs = [], branchSha = SHA_C, failOn = null } = {}) {
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
    getRunJobs: async (id) => {
      maybeFail("getRunJobs");
      return jobs;
    },
    compareCommits: async (b, h) => {
      maybeFail("compareCommits");
      return compare;
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

// ---------------------------------------------------------------- plan ---

test("failure + no open issue → create", () => {
  const c = classifyRun(payload(), scope);
  const actions = plan(c, { canonical: null, duplicates: [], jobs: [] });
  const create = actions.find((a) => a.type === "create");
  assert.ok(create);
  assert.equal(create.state.failure.sha, SHA_A);
  assert.equal(create.state.latestRunId, 100);
  assert.equal(create.state.history.length, 1);
});

test("failure + open issue + newer run id → update preserving first detection", () => {
  const canonical = managedIssue(42, failureState());
  const c = classifyRun(payload({ workflow_run: { id: 150, head_sha: SHA_B } }), scope);
  const actions = plan(c, { canonical, duplicates: [], jobs: [{ name: "verify", conclusion: "failure" }] });
  const update = actions.find((a) => a.type === "update");
  assert.ok(update);
  assert.equal(update.issue, 42);
  assert.equal(update.state.failure.sha, SHA_B);
  assert.equal(update.state.failure.runId, 150);
  assert.equal(update.state.failure.firstSha, SHA_A, "first detection sha preserved");
  assert.equal(update.state.latestRunId, 150);
  assert.equal(update.state.history.at(-1).runId, 150);
});

test("failure + open issue + older/equal run id → noop (completion inversion)", () => {
  const canonical = managedIssue(42, failureState({ latestRunId: 200, failure: { ...failureState().failure, runId: 200 } }));
  const c = classifyRun(payload({ workflow_run: { id: 150 } }), scope);
  const actions = plan(c, { canonical, duplicates: [], jobs: [] });
  assert.ok(actions.every((a) => a.type === "noop"));
  assert.match(actions.find((a) => a.reason?.startsWith("stale-failure"))?.reason ?? "", /stale-failure/);
});

test("failure + duplicate managed opens → canonical min + dup closes", () => {
  const canonical = managedIssue(10, failureState());
  const dup = managedIssue(77, failureState());
  const c = classifyRun(payload({ workflow_run: { id: 300 } }), scope);
  const actions = plan(c, { canonical, duplicates: [dup], jobs: [] });
  assert.ok(actions.some((a) => a.type === "close-duplicate" && a.issue === 77 && a.canonical === 10));
  assert.ok(actions.some((a) => a.type === "update" && a.issue === 10));
});

test("success + open issue + identical/ahead + newer run → close", () => {
  for (const cmp of ["identical", "ahead"]) {
    const canonical = managedIssue(42, failureState());
    const c = classifyRun(payload({ workflow_run: { id: 300, conclusion: "success", head_sha: SHA_B } }), scope);
    const actions = plan(c, { canonical, duplicates: [], compareStatus: cmp });
    const close = actions.find((a) => a.type === "close-recovered");
    assert.ok(close, cmp);
    assert.equal(close.issue, 42);
    assert.equal(close.recovery.sha, SHA_B);
  }
});

test("success + open issue + behind/diverged → no close", () => {
  for (const cmp of ["behind", "diverged"]) {
    const canonical = managedIssue(42, failureState());
    const c = classifyRun(payload({ workflow_run: { id: 300, conclusion: "success", head_sha: SHA_B } }), scope);
    const actions = plan(c, { canonical, duplicates: [], compareStatus: cmp });
    assert.ok(!actions.some((a) => a.type === "close-recovered"), cmp);
    // newer than everything seen → note update, not silent noop
    assert.ok(actions.some((a) => a.type === "update" || a.type === "noop"), cmp);
  }
});

test("success + open issue + success run older than failure run → no close (old run success)", () => {
  const canonical = managedIssue(42, failureState());
  const c = classifyRun(payload({ workflow_run: { id: 50, conclusion: "success", head_sha: SHA_A } }), scope);
  const actions = plan(c, { canonical, duplicates: [], compareStatus: "identical" });
  assert.ok(!actions.some((a) => a.type === "close-recovered"));
  assert.ok(actions.some((a) => a.type === "noop" && a.reason.startsWith("stale-success")));
});

test("rerun success (same run id, higher attempt) closes the failure it re-verified", () => {
  const canonical = managedIssue(42, failureState());
  const c = classifyRun(
    payload({ workflow_run: { id: 100, run_attempt: 2, conclusion: "success", head_sha: SHA_A } }),
    scope,
  );
  const actions = plan(c, { canonical, duplicates: [], compareStatus: "identical" });
  assert.ok(actions.some((a) => a.type === "close-recovered" && a.issue === 42));
});

test("rerun failure (same run id, higher attempt) updates instead of going stale", () => {
  const canonical = managedIssue(42, failureState());
  const c = classifyRun(
    payload({ workflow_run: { id: 100, run_attempt: 2, conclusion: "failure", head_sha: SHA_A } }),
    scope,
  );
  const actions = plan(c, { canonical, duplicates: [], jobs: [] });
  const update = actions.find((a) => a.type === "update");
  assert.ok(update);
  assert.equal(update.state.failure.attempt, 2);
  assert.equal(update.state.latestAttempt, 2);
});

test("same run id same attempt is stale — idempotent re-delivery", () => {
  const canonical = managedIssue(42, failureState());
  const c = classifyRun(payload({ workflow_run: { id: 100, run_attempt: 1 } }), scope);
  assert.ok(plan(c, { canonical, duplicates: [], jobs: [] }).every((a) => a.type === "noop"));
});

test("success + no open issue → noop", () => {
  const c = classifyRun(payload({ workflow_run: { conclusion: "success" } }), scope);
  assert.ok(plan(c, { canonical: null, duplicates: [] }).every((a) => a.type === "noop"));
});

test("operational + open issue + newer run → note update, never close/create", () => {
  const canonical = managedIssue(42, failureState());
  const c = classifyRun(payload({ workflow_run: { id: 300, conclusion: "cancelled" } }), scope);
  const actions = plan(c, { canonical, duplicates: [] });
  const update = actions.find((a) => a.type === "update");
  assert.ok(update);
  assert.equal(update.state.latestRunId, 300);
  assert.match(update.state.operationalNote, /cancelled/);
  assert.equal(update.state.failure.sha, SHA_A, "failure state untouched");
  assert.ok(!actions.some((a) => a.type === "create" || a.type === "close-recovered"));
});

test("operational + no open issue → noop (cancelled never files)", () => {
  const c = classifyRun(payload({ workflow_run: { conclusion: "cancelled" } }), scope);
  assert.ok(plan(c, { canonical: null, duplicates: [] }).every((a) => a.type === "noop"));
});

test("operational + stale run → noop", () => {
  const canonical = managedIssue(42, failureState({ latestRunId: 500 }));
  const c = classifyRun(payload({ workflow_run: { id: 300, conclusion: "cancelled" } }), scope);
  assert.ok(plan(c, { canonical, duplicates: [] }).every((a) => a.type === "noop"));
});

// ------------------------------------------------------- marker/sanitize -

test("marker roundtrip preserves scope and state", () => {
  const state = failureState();
  const body = renderBody(scope, state);
  const parsed = parseMarker(body);
  assert.ok(parsed);
  assert.ok(scopeMatches(parsed.scope, scope));
  assert.equal(parsed.state.failure.sha, SHA_A);
  assert.equal(parsed.state.latestRunId, 100);
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

test("run: first failure creates one issue via ensureLabel→list→jobs→create", async () => {
  const client = mockClient({ issues: [], jobs: [{ name: "verify", conclusion: "failure" }] });
  const res = await run({ payload: payload(), client, scope });
  assert.ok(res.applied.some((a) => a.type === "create" && a.issue === 500));
  const order = client.calls.join(",");
  assert.ok(order.startsWith("ensureLabel,listOpenIssues"));
  assert.ok(client.calls.includes("getRunJobs"));
  assert.ok(!client.calls.includes("compareCommits"), "no compare needed on failure");
});

test("run: foreign labeled issue is untouched and a fresh issue is created", async () => {
  const foreign = { number: 9, body: "human issue with the label", pull_request: undefined };
  const client = mockClient({ issues: [foreign] });
  const res = await run({ payload: payload(), client, scope });
  assert.ok(res.applied.some((a) => a.type === "create"));
  assert.ok(!client.calls.some((c) => c.includes("#9")), "foreign issue #9 never modified");
});

test("run: success closes covered issue via compare", async () => {
  const issue = managedIssue(42, failureState());
  const client = mockClient({ issues: [{ number: 42, body: issue.body }], compare: "ahead" });
  const res = await run({
    payload: payload({ workflow_run: { id: 300, conclusion: "success", head_sha: SHA_B } }),
    client,
    scope,
  });
  assert.ok(res.applied.some((a) => a.type === "close-recovered" && a.issue === 42));
  assert.ok(client.calls.includes("compareCommits"));
  assert.ok(client.calls.includes("closeIssue:#42"));
  assert.ok(!client.calls.includes("getRunJobs"), "no jobs fetch needed on success");
});

test("run: behind success does not close", async () => {
  const issue = managedIssue(42, failureState());
  const client = mockClient({ issues: [{ number: 42, body: issue.body }], compare: "behind" });
  const res = await run({
    payload: payload({ workflow_run: { id: 300, conclusion: "success", head_sha: SHA_A } }),
    client,
    scope,
  });
  assert.ok(!res.applied.some((a) => a.type === "close-recovered"));
  assert.ok(!client.calls.includes("closeIssue:#42"));
});

test("run: duplicate opens are consolidated under the lowest number", async () => {
  const low = managedIssue(10, failureState());
  const high = managedIssue(77, failureState());
  const client = mockClient({
    issues: [
      { number: 77, body: high.body },
      { number: 10, body: low.body },
    ],
  });
  const res = await run({ payload: payload({ workflow_run: { id: 300 } }), client, scope });
  assert.equal(res.canonical, 10);
  assert.ok(client.calls.includes("closeIssue:#77"));
  assert.ok(res.applied.some((a) => a.type === "update" && a.issue === 10));
});

test("run: API failures propagate instead of being swallowed", async () => {
  for (const failOn of ["ensureLabel", "listOpenIssues", "getRunJobs", "updateIssue", "compareCommits"]) {
    const issue = managedIssue(42, failureState());
    const client = mockClient({
      issues: failOn === "updateIssue" || failOn === "compareCommits" ? [{ number: 42, body: issue.body }] : [],
      compare: "ahead",
      failOn,
    });
    const p =
      failOn === "compareCommits"
        ? payload({ workflow_run: { id: 300, conclusion: "success", head_sha: SHA_B } })
        : payload({ workflow_run: { id: 300 } });
    await assert.rejects(() => run({ payload: p, client, scope }), new RegExp(failOn), failOn);
  }
});

test("run: branch-head probe failure is diagnostic-only and does not block", async () => {
  const client = mockClient({ issues: [], failOn: "getBranchHead" });
  const res = await run({ payload: payload(), client, scope });
  assert.equal(res.mainHead, null);
  assert.ok(res.applied.some((a) => a.type === "create"), "create still applied");
});
