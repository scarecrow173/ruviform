#!/usr/bin/env node
// tracker.mjs — Issue #368 CI failure Issue Tracker.
//
// Watches `workflow_run` completions of the `ci` workflow and keeps at most
// ONE open failure issue per scope (repo × workflow × branch). It is the
// Issue-side counterpart of `ci/summary/summary.mjs`: where ci-summary
// fail-closes the run, the tracker records that a main Full CI failure
// exists and closes the issue only when a NEWER success covers the failed
// revision AND the current main tip.
//
// Trust model (Issue #368 implementation 2): this script runs from the
// default branch inside a privileged `workflow_run` job. It never checks
// out, downloads, or executes the completed run's code or artifacts — the
// only inputs are the workflow_run event payload and GitHub REST metadata
// (issues, jobs, run history, compare). Every externally-sourced value
// written into an issue body, comment, or the job summary is
// sanitized/allowlisted.
//
// Evidence model (post-review hardening): the incoming event is a TRIGGER,
// not the truth. GitHub Actions concurrency groups keep at most one running
// + one pending workflow per group, and a new pending replaces the queued
// one — notifications can be silently dropped between "run finished" and
// "tracker processed". So on every non-skip event the tracker fetches the
// authoritative run list for (workflow, branch, allowed events) and
// reconciles it with the incoming run and the marker state. Decisions are
// made against the NEWEST evidence, which heals dropped/out-of-order
// notifications without ever trusting them blindly.
//
// Ordering: a rerun keeps the same run.id with run_attempt incremented, and
// an OLD run's new attempt can complete AFTER a newer run. So (id, attempt)
// alone cannot order evidence across runs. `updated_at` is the completion
// order signal (`run_started_at` is attempt START time and unusable for
// completion ordering; actor/triggering_actor attribute who ran it, not
// when it finished). compareRunOrder: same id → attempt; different ids →
// updated_at; equal/missing timestamps on different ids → "ambiguous" and
// the caller decides (state advancement falls back to run.id; CLOSING
// never does — ambiguity holds the issue open).
//
// Close rule: only when the newest evidence is a success that is
// unambiguously newer than the newest recorded failure, `compare(failed
// sha → success sha)` is identical/ahead, AND `compare(current main head →
// success sha)` is identical/ahead (the success must cover the CURRENT
// main tip — main is verified, not merely the old failed revision). A
// cancelled/unknown/operational newest run, an ambiguous order, an
// unreachable main head, or a compare/API failure always keeps the issue
// open; API failures propagate and fail the job.
//
// The GitHub client is injectable: `run()` and `plan()` are fully
// deterministic under a mock client, and all tests are offline.
//
// Usage:
//   TRACKER_EVENT_PATH=event.json GH_TOKEN=*** node ci/tracker/tracker.mjs \
//       [--repo owner/name] [--branch main] [--workflow ci] [--label ci-failure] \
//       [--workflow-file ci.yml] [--report out.json] [--summary out.md]

import { execFileSync } from "node:child_process";
import { appendFileSync, readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

// ---------------------------------------------------------------- scope ---

export const TRACKER_LABEL = "ci-failure";
export const LABEL_COLOR = "d93f0b";
export const ISSUE_TITLE = "[CI failure][main] Full CI failed";
export const MARKER_TAG = "ci-issue-tracker";
export const HISTORY_LIMIT = 10;
export const HISTORY_PAGE_SIZE = 30;

// Events whose completed `ci` runs count as valid main Full CI executions:
// `push` (main) and `workflow_dispatch` (manual + the Issue #367 daily
// scheduler). `pull_request`/`merge_group`/`schedule`/anything else is
// skipped — the allowlist is deliberate so a new trigger added to ci.yml
// cannot silently become tracked evidence.
export const ALLOWED_EVENTS = ["push", "workflow_dispatch"];

// Conclusions that mean "the run finished without verifying" — the failure
// side of the tracker. `timed_out`/`startup_failure` never verified either.
// Everything else (`cancelled`, `skipped`, `neutral`, `stale`,
// `action_required`, null, unknown) is operational: not success, and not a
// reason to open a failure issue.
export const FAILURE_CONCLUSIONS = ["failure", "timed_out", "startup_failure"];
export const HONORED_CONCLUSIONS = [
  "success",
  "failure",
  "timed_out",
  "startup_failure",
  "cancelled",
  "skipped",
  "neutral",
  "stale",
  "action_required",
];

// ------------------------------------------------------------ sanitize ---

const SHA_RE = /^[0-9a-f]{40}$/;
const ISO_RE = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:?\d{2})$/;

export function sanitizeSha(v) {
  return typeof v === "string" && SHA_RE.test(v) ? v : null;
}

export function sanitizeUrl(v) {
  if (typeof v !== "string" || v.length > 500) return null;
  if (!v.startsWith("https://github.com/")) return null;
  // printable ASCII only — no control characters or whitespace tricks.
  if (!/^[\x21-\x7e]+$/.test(v)) return null;
  return v;
}

// Free-form text taken from API metadata (job names, branch names, OS
// labels). Strip control characters, `|` (table injection), and backticks;
// bound the length.
export function sanitizeText(v, max = 100) {
  if (typeof v !== "string") return "";
  const cleaned = v.replace(/[\x00-\x1f\x7f|`]/g, "").trim();
  return cleaned.length > max ? `${cleaned.slice(0, max)}…` : cleaned;
}

export function sanitizeConclusion(v) {
  return typeof v === "string" && HONORED_CONCLUSIONS.includes(v) ? v : "unknown";
}

export function sanitizeRunId(v) {
  return typeof v === "number" && Number.isSafeInteger(v) && v > 0 ? v : null;
}

function sanitizeIso(v) {
  return typeof v === "string" && ISO_RE.test(v) ? v : null;
}

// ----------------------------------------------------------- classify ----

export function defaultScope(repo) {
  return {
    repo,
    workflow: "ci",
    workflowFile: "ci.yml",
    branch: "main",
    allowedEvents: [...ALLOWED_EVENTS],
    label: TRACKER_LABEL,
  };
}

// success | failure | operational for any conclusion value.
export function kindOfConclusion(conclusion) {
  const c = sanitizeConclusion(conclusion);
  if (c === "success") return "success";
  if (FAILURE_CONCLUSIONS.includes(c)) return "failure";
  return "operational";
}

// Normalize a raw workflow_run-shaped record (event payload or runs-list
// item) into the internal run shape. Returns null on malformed metadata.
function normalizeRun(wr, scope) {
  if (wr == null || typeof wr !== "object") return null;
  const id = sanitizeRunId(wr.id);
  const sha = sanitizeSha(wr.head_sha);
  if (id == null || sha == null) return null;
  return {
    id,
    sha,
    url: sanitizeUrl(wr.html_url),
    conclusion: typeof wr.conclusion === "string" ? wr.conclusion : null,
    event: wr.event,
    attempt: sanitizeRunId(wr.run_attempt) ?? 1,
    updatedAt: sanitizeIso(wr.updated_at),
    startedAt: sanitizeIso(wr.run_started_at),
    branch: wr.head_branch,
    repo: wr.head_repository?.full_name ?? null,
    status: wr.status ?? null,
  };
}

// Classify a raw runs-list record into three dispositions:
//   "out"     — a PRESENT field proves exclusion (other repo, fork head,
//               other branch, disallowed event, other workflow file/name):
//               dropped, never evidence.
//   "in"      — every essential field is present and matches scope (repo,
//               branch, event, workflow identity, id, sha, attempt,
//               updated_at, status). A `completed` record is evidence
//               classified by conclusion; any OTHER status — queued,
//               in_progress, requested, waiting, pending, or an
//               unrecognized value — is a live run whose completion still
//               lies in the FUTURE, so it is `pending` evidence that can
//               never promote a success to a close.
//   "unknown" — scope or ordering cannot be proven (missing/unusable id,
//               sha, run_attempt, updated_at, status, repo, branch, event,
//               or workflow identity): fail-closed evidence — it blocks
//               every close and prevents resolved-suppression, and is
//               never itself a success, failure, or issue trigger.
export function classifyHistoryRecord(raw, scope) {
  const partial = {
    id: sanitizeRunId(raw?.id) ?? null,
    sha: sanitizeSha(raw?.head_sha) ?? null,
    attempt: sanitizeRunId(raw?.run_attempt) ?? null,
    updatedAt: sanitizeIso(raw?.updated_at) ?? null,
    conclusion: null,
  };
  const unknown = { disposition: "unknown", run: partial };
  if (raw == null || typeof raw !== "object") return unknown;
  const headRepo = raw.head_repository?.full_name ?? null;
  const parentRepo = raw.repository?.full_name ?? null;
  const path = typeof raw.path === "string" ? raw.path : null;
  const wname = typeof raw.name === "string" ? raw.name : null;
  if (
    (headRepo != null && headRepo !== scope.repo) ||
    (parentRepo != null && parentRepo !== scope.repo) ||
    (raw.head_branch != null && raw.head_branch !== scope.branch) ||
    (raw.event != null && !scope.allowedEvents.includes(raw.event)) ||
    (path != null && !(path === scope.workflowFile || path.endsWith(`/${scope.workflowFile}`))) ||
    (wname != null && wname !== scope.workflow)
  ) {
    return { disposition: "out" };
  }
  const provable =
    headRepo === scope.repo &&
    raw.head_repository?.fork !== true &&
    raw.head_branch === scope.branch &&
    scope.allowedEvents.includes(raw.event) &&
    (path != null || wname != null) &&
    typeof raw.status === "string" &&
    partial.id != null &&
    partial.sha != null &&
    partial.attempt != null &&
    partial.updatedAt != null;
  if (!provable) return unknown;
  const run = normalizeRun(raw, scope);
  if (run == null) return unknown;
  if (run.status !== "completed") return { disposition: "in", run: { ...run, pending: true } };
  return { disposition: "in", run };
}

// A completed run is a valid Full CI target only when it ran on this
// repository's `main` branch via an allowed event. Anything else — fork
// runs, PR auxiliary runs, other branches, non-completed actions — is a
// hard `skip` and must not touch issues.
export function classifyRun(payload, scope) {
  const wr = payload?.workflow_run;
  const skip = (reason) => ({ kind: "skip", reason });
  if (payload?.action !== "completed") return skip(`action:${payload?.action ?? "missing"}`);
  if (wr == null || typeof wr !== "object") return skip("no-workflow_run");
  if (wr.head_repository?.full_name !== scope.repo) {
    return skip(`repo:${sanitizeText(wr.head_repository?.full_name, 200) || "unknown"}`);
  }
  if (wr.head_branch !== scope.branch) {
    return skip(`branch:${sanitizeText(wr.head_branch) || "unknown"}`);
  }
  if (!scope.allowedEvents.includes(wr.event)) {
    return skip(`event:${sanitizeText(wr.event) || "unknown"}`);
  }
  // action === "completed" must agree with the run status — a missing or
  // contradicting status is a malformed payload, not evidence.
  if (wr.status !== "completed") {
    return skip(`status:${sanitizeText(wr.status) || "missing"}`);
  }
  const run = normalizeRun(wr, scope);
  if (run == null || sanitizeRunId(wr.run_attempt) == null) {
    return skip("malformed-run-metadata");
  }
  return { kind: kindOfConclusion(run.conclusion), run };
}

// ------------------------------------------------------------- marker ----

const MARKER_RE = new RegExp(`^\\s*<!--\\s*${MARKER_TAG}:([\\s\\S]*?)-->`);

// The stable identity: a JSON marker as the first HTML comment of the issue
// body, carrying scope + machine state. Issues carrying the label but no
// parseable marker (or a marker for a different scope) are foreign and must
// never be modified.
export function renderMarker(scope, state) {
  const json = JSON.stringify({
    v: 1,
    scope: { repo: scope.repo, workflow: scope.workflow, branch: scope.branch },
    state,
  })
    // Angle brackets only occur inside JSON string literals, so escaping
    // them keeps the JSON valid while making `-->` / `<!--` sequences
    // impossible — a hostile job name can never break out of the comment.
    .replace(/</g, "\\u003c")
    .replace(/>/g, "\\u003e");
  return `<!-- ${MARKER_TAG}:${json} -->`;
}

// A run reference is one atomic ordering tuple — never assembled by
// field-wise max (mixing fields from different runs fabricates a run that
// never existed).
function refOf(run) {
  if (run == null) return null;
  return {
    runId: run.id ?? run.runId,
    attempt: run.attempt ?? 1,
    updatedAt: run.updatedAt ?? null,
  };
}

// Backward-compatible marker state: `latest` is the canonical tuple form;
// legacy `latestRunId`/`latestAttempt` flat fields are mapped in (their
// updatedAt is unknown → cross-run order becomes ambiguous → conservative).
function normalizeState(state) {
  if (state == null || typeof state !== "object") return null;
  const failure = state.failure;
  if (
    failure == null ||
    !Number.isSafeInteger(failure.runId) ||
    !SHA_RE.test(failure.sha ?? "")
  ) {
    return null;
  }
  let latest = state.latest;
  if (!Number.isSafeInteger(latest?.runId)) {
    latest = Number.isSafeInteger(state.latestRunId)
      ? { runId: state.latestRunId, attempt: state.latestAttempt ?? 1, updatedAt: null }
      : null;
  }
  if (latest == null) return null;
  return {
    latest,
    failure: { ...failure, attempt: failure.attempt ?? 1, updatedAt: failure.updatedAt ?? null },
    history: Array.isArray(state.history) ? state.history : [],
    operationalNote: state.operationalNote ?? null,
  };
}

export function parseMarker(body) {
  if (typeof body !== "string") return null;
  const m = MARKER_RE.exec(body);
  if (m == null) return null;
  let parsed;
  try {
    parsed = JSON.parse(m[1]);
  } catch {
    return null;
  }
  if (parsed?.v !== 1 || parsed.scope == null || parsed.state == null) return null;
  const { scope, state } = parsed;
  if (
    typeof scope.repo !== "string" ||
    typeof scope.workflow !== "string" ||
    typeof scope.branch !== "string"
  ) {
    return null;
  }
  const normalized = normalizeState(state);
  if (normalized == null) return null;
  return { scope, state: normalized };
}

// ---------------------------------------------------------- ordering -----

// Ordering between a candidate run and a stored reference.
//   "newer"     — the candidate is provably later evidence
//   "stale"     — the candidate is provably older evidence
//   "same"      — identical (id, attempt)
//   "ambiguous" — different runs whose completion order cannot be proven
//                 (equal or missing updated_at): never treated as success.
//
// run: {id, attempt, updatedAt}; ref: {runId, attempt, updatedAt}.
// Same id → attempt is authoritative (attempts are strictly sequential).
// Different ids → updated_at is the completion-order signal; a rerun of an
// old run can land after a newer run, which (id, attempt) alone misses.
export function compareRunOrder(run, ref) {
  if (ref == null) return "newer";
  const rid = run.id ?? run.runId;
  const ra = run.attempt ?? 1;
  const refAttempt = ref.attempt ?? 1;
  if (rid === ref.runId) {
    if (ra > refAttempt) return "newer";
    if (ra < refAttempt) return "stale";
    return "same";
  }
  const a = run.updatedAt ?? null;
  const b = ref.updatedAt ?? null;
  if (a != null && b != null) {
    if (a > b) return "newer";
    if (a < b) return "stale";
    return "ambiguous"; // same-second completion on different runs
  }
  return "ambiguous";
}

// State advancement tolerates ambiguity by falling back to run-id
// monotonicity (a higher id was created later). CLOSE decisions never use
// this — they require strict "newer".
function orderForState(run, ref) {
  const o = compareRunOrder(run, ref);
  if (o !== "ambiguous") return o;
  const rid = run.id ?? run.runId;
  return rid > ref.runId ? "newer" : rid < ref.runId ? "stale" : "same";
}

// Strict close-order predicate: ambiguity is not newer.
export function isNewerRun(run, ref) {
  return compareRunOrder(run, ref) === "newer";
}

export function scopeMatches(markerScope, scope) {
  return (
    markerScope.repo === scope.repo &&
    markerScope.workflow === scope.workflow &&
    markerScope.branch === scope.branch
  );
}

// ---------------------------------------------------------- evidence -----

// Deterministic argmax key — picks a representative; NOT an ordering proof.
// Ordering proofs go through compareRunOrder and may answer "ambiguous".
function runSortKey(run) {
  return [run.updatedAt ?? "", run.id ?? run.runId, run.attempt ?? 1];
}

function compareSortKey(a, b) {
  const ka = runSortKey(a);
  const kb = runSortKey(b);
  for (let i = 0; i < 3; i++) {
    if (ka[i] < kb[i]) return -1;
    if (ka[i] > kb[i]) return 1;
  }
  return 0;
}

// Newest candidate by deterministic argmax, plus an `ambiguous` flag set
// when any same-rank competitor is a different run whose order vs the top
// cannot be proven. `ambiguous` never permits a success decision.
function pickNewest(candidates) {
  const list = candidates.filter((c) => c?.run != null);
  if (list.length === 0) return null;
  const sorted = [...list].sort((x, y) => compareSortKey(y.run, x.run));
  const top = sorted[0];
  const ambiguous = sorted
    .slice(1)
    .some((c) => c.run.id !== top.run.id && compareRunOrder(top.run, refOf(c.run)) === "ambiguous");
  return { kind: top.kind, run: top.run, ambiguous };
}

// Merge per-run latest-attempt records by run id: for each id keep the
// record with the highest attempt; for equal attempts prefer `preferred`
// (the incoming event — authoritative for the attempt it reports).
function mergeRunRecords(records, preferred = null) {
  const byId = new Map();
  for (const r of records) {
    if (r == null) continue;
    const cur = byId.get(r.id);
    if (cur == null || r.attempt > cur.attempt || (r.attempt === cur.attempt && r === preferred)) {
      byId.set(r.id, r);
    }
  }
  return [...byId.values()];
}

// Merge marker states across duplicate managed issues: adopt the marker
// whose `failure` is the newest wholesale (never field-wise mixed), take
// the newest `latest` the same way, and union histories by
// (kind, runId, attempt) sorted by `at`, capped.
export function mergeMarkerStates(states) {
  const normalized = states.map((s) => normalizeState(s)).filter(Boolean);
  if (normalized.length === 0) return null;
  const newestFailure = normalized
    .map((s) => s.failure)
    .sort((a, b) => compareSortKey(refOf(b), refOf(a)))[0];
  const newestLatest = normalized
    .map((s) => s.latest)
    .sort((a, b) => compareSortKey(b, a))[0];
  const owner = normalized.find((s) => s.failure === newestFailure);
  const seen = new Set();
  const history = normalized
    .flatMap((s) => s.history)
    .filter((h) => {
      const k = `${h.kind}:${h.runId}:${h.attempt ?? 1}`;
      if (seen.has(k)) return false;
      seen.add(k);
      return true;
    })
    .sort((a, b) => (a.at ?? "") < (b.at ?? "") ? -1 : (a.at ?? "") > (b.at ?? "") ? 1 : 0)
    .slice(-HISTORY_LIMIT);
  return { ...owner, latest: newestLatest, history };
}

// ------------------------------------------------------------ body -------

function jobRows(jobs) {
  if (!Array.isArray(jobs) || jobs.length === 0) return ["(no job data)"];
  return jobs.map(
    (j) => `| ${sanitizeText(j.name) || "?"} | ${sanitizeConclusion(j.conclusion)} |`,
  );
}

function historyRows(history) {
  return (history ?? []).map(
    (h) =>
      `| ${h.runId}a${h.attempt ?? 1} | \`${sanitizeSha(h.sha) ?? "?"}\` | ${sanitizeConclusion(h.conclusion)} | ${sanitizeText(h.kind)} | ${sanitizeIso(h.at) ?? "?"} |`,
  );
}

// Human-readable body. Everything dynamic is sanitized before it lands —
// the marker line carries the machine state and stays decoration-free.
export function renderBody(scope, state) {
  const f = state.failure;
  const lines = [
    renderMarker(scope, state),
    "",
    `## CI failure tracker — \`${sanitizeText(scope.branch)}\` × \`${sanitizeText(scope.workflow)}\``,
    "",
    "Managed by `.github/workflows/ci-issue-tracker.yml` (Issue #368).",
    "Do not edit the leading marker comment — it identifies this issue to the tracker.",
    "",
    "### Status: OPEN — unresolved Full CI failure",
    "",
    "| field | value |",
    "| --- | --- |",
    `| latest failure run | ${sanitizeUrl(f.runUrl) ?? `run id ${f.runId}`} (run id ${f.runId}a${f.attempt ?? 1}) |`,
    `| failed revision | \`${sanitizeSha(f.sha) ?? "?"}\` |`,
    `| conclusion | \`${sanitizeConclusion(f.conclusion)}\` |`,
    `| first detected | ${sanitizeIso(f.firstAt) ?? "?"} (revision \`${sanitizeSha(f.firstSha) ?? "?"}\`) |`,
    `| last detected | ${sanitizeIso(f.lastAt) ?? "?"} |`,
  ];
  if (state.operationalNote) {
    lines.push("", `> ${sanitizeText(state.operationalNote, 300)}`);
  }
  lines.push(
    "",
    "#### Jobs in latest failing run",
    "",
    "| job | conclusion |",
    "| --- | --- |",
    ...jobRows(f.jobs),
    "",
    "#### Recent observed runs",
    "",
    "| run | sha | conclusion | kind | at |",
    "| --- | --- | --- | --- | --- |",
    ...historyRows(state.history),
    "",
    "#### Triage",
    "",
    "1. Open the latest failing run and inspect the failing jobs.",
    "2. Land a fix on `main` or re-run `ci` via `workflow_dispatch`.",
    "3. This issue closes automatically only when a **newer** `ci` run succeeds",
    "   at a revision covering the failed one AND the current `main` tip. Old-run,",
    "   ambiguous-order, or divergent successes do not close it.",
    "",
    "#### Operational policy",
    "",
    "- `cancelled`/`skipped`/unknown conclusions are neither failure nor",
    "  success — they are recorded here as an operational note only.",
    "- The tracker never reopens manually closed issues; a failure after a",
    "  manual close opens a new issue.",
    "- While GitHub Actions itself is down the tracker cannot run — check the",
    "  last green `ci` run and re-dispatch manually after recovery.",
    "",
  );
  return lines.join("\n");
}

// --------------------------------------------------------------- plan ----

function historyPush(state, entry) {
  const history = [...(state?.history ?? []), entry];
  return history.slice(-HISTORY_LIMIT);
}

function runHistoryEntry(kind, run, src) {
  return {
    kind,
    runId: run.id,
    attempt: run.attempt,
    updatedAt: run.updatedAt ?? null,
    sha: run.sha,
    conclusion: run.conclusion,
    at: run.updatedAt,
    src: src ?? "event",
  };
}

function failureFrom(run, jobs, firstSha, firstAt) {
  return {
    sha: run.sha,
    runId: run.id,
    attempt: run.attempt,
    updatedAt: run.updatedAt ?? null,
    runUrl: run.url,
    conclusion: run.conclusion,
    firstSha: firstSha ?? run.sha,
    firstAt: firstAt ?? run.updatedAt,
    lastAt: run.updatedAt,
    jobs: jobs ?? [],
  };
}

function baseState(run, jobs, note = null) {
  return {
    latest: refOf(run),
    failure: failureFrom(run, jobs),
    history: [runHistoryEntry("failure", run)],
    operationalNote: note,
  };
}

// `plan` is pure: it receives the reconciled evidence (already merged from
// the incoming event + authoritative run history + marker states) and the
// coverage verdicts fetched by the caller, and returns the action list.
//   newest          — {kind, run, ambiguous}: newest overall evidence run
//   newestFailure   — {kind:"failure", run}: newest failure-typed evidence
//   coveringSuccess — {kind:"success", run}: newest success strictly newer
//                     than newestFailure (ordering proven, not ambiguous)
//   coverage        — compare(newestFailure.sha → coveringSuccess.sha)
//   mainCoverage    — compare(current main head → newest.run.sha); fetched
//                     only when a close is otherwise a candidate
//   resolved        — coveringSuccess != null && coverage covers &&
//                     evidence complete: the newest failure evidence is
//                     already verified fixed
//   evidencePending — a live (non-completed) run exists in evidence: its
//                     completion is in the future, so nothing may close
//   evidenceUnknown — an unclassifiable history record exists: the list is
//                     incomplete, so neither close nor resolved-suppression
//                     is allowed
export function plan({
  classified,
  canonical,
  duplicates = [],
  mergedState = null,
  newest = null,
  newestFailure = null,
  coveringSuccess = null,
  coverage = null,
  mainCoverage = null,
  jobs = null,
  evidencePending = false,
  evidenceUnknown = false,
}) {
  const actions = [];
  const stored = mergedState ?? canonical?.marker?.state ?? null;
  // Coverage resolves a failure only when the evidence is complete — an
  // unclassifiable history record could hide a newer failure.
  const resolved =
    coveringSuccess != null &&
    (coverage === "identical" || coverage === "ahead") &&
    !evidenceUnknown;

  // Duplicate consolidation first: merge-state (only when a duplicate
  // actually held newer state — otherwise the decision's own update covers
  // it), then close-duplicate per extra managed issue.
  const stateChanged =
    canonical != null &&
    mergedState != null &&
    canonical.marker != null &&
    JSON.stringify(canonical.marker.state) !== JSON.stringify(mergedState);
  const dupActions = duplicates.map((d) => ({
    type: "close-duplicate",
    issue: d.number,
    canonical: canonical?.number ?? null,
  }));

  const pushDecision = (decision) => {
    // merge-state is skipped when the decision itself updates the canonical
    // issue — that update already carries the merged/newest state.
    const decisionUpdatesCanonical =
      decision.type === "update" && decision.issue === canonical?.number;
    if (stateChanged && !decisionUpdatesCanonical) {
      actions.push({ type: "merge-state", issue: canonical.number, state: mergedState });
    }
    actions.push(...dupActions, decision);
    return actions;
  };

  if (newest == null || newest.run == null) {
    return pushDecision({ type: "noop", reason: "no-evidence" });
  }

  // ------------------------------------------------- no managed issue ----
  // A new issue is opened only on UNRESOLVED failure evidence: the newest
  // failure run not covered by a strictly-newer covering success. A stale
  // failure whose successor already verified a fix never files — event
  // delivery order cannot resurrect it. Operational/ambiguous evidence
  // alone never files.
  if (canonical == null) {
    if (newestFailure != null && !resolved) {
      const note =
        newest.kind === "failure"
          ? null
          : `Latest observed run ${newest.run.id ?? "?"} (${newest.kind}${newest.run.conclusion ? `, concluded \`${sanitizeConclusion(newest.run.conclusion)}\`` : ""}); issue opened on the newest failure evidence, reconciled from run history.`;
      const state = baseState(newestFailure.run, jobs);
      if (newest.run.id != null && orderForState(newest.run, refOf(newestFailure.run)) === "newer") {
        state.latest = refOf(newest.run);
      }
      state.operationalNote = note;
      return pushDecision({ type: "create", state });
    }
    return pushDecision({
      type: "noop",
      reason:
        newest.kind === "success"
          ? "success:no-unresolved-failure"
          : newest.kind === "pending"
            ? `pending-run:${newest.run.id}`
            : newest.kind === "unknown"
              ? "unclassified-history"
              : `operational:${newest.run.conclusion ?? "null"}`,
    });
  }

  // ------------------------------------------------------------- success -
  if (newest.kind === "success") {
    if (newest.ambiguous) {
      // A success that cannot be ordered against the rest of the evidence
      // is never a recovery — record it and keep everything open.
      if (stored != null && orderForState(newest.run, stored.latest) === "newer") {
        return pushDecision({
          type: "update",
          issue: canonical.number,
          state: {
            ...stored,
            latest: refOf(newest.run),
            history: historyPush(stored, { ...runHistoryEntry("success-ambiguous", newest.run) }),
            operationalNote: `Success run ${newest.run.id} has ambiguous ordering vs recorded evidence — held open pending a verifiable recovery.`,
          },
        });
      }
      return pushDecision({ type: "noop", reason: "success-ambiguous:held" });
    }
    // Close guard — every condition is required:
    //  1. strictly newer than the newest recorded failure (never ambiguous)
    //  2. not stale/ambiguous vs the newest observed run
    //  3. compare(failed sha → success sha) covers the failed revision
    //  4. compare(current main head → success sha) covers the CURRENT tip
    //  5. evidence is complete — no live (pending) run and no
    //     unclassifiable history record may coexist with the close
    const ordFailure = newestFailure != null
      ? compareRunOrder(newest.run, refOf(newestFailure.run))
      : "newer";
    const ordLatest = stored?.latest != null
      ? compareRunOrder(newest.run, stored.latest)
      : "newer";
    const coveredFailure = coverage === "identical" || coverage === "ahead";
    const coveredMain = mainCoverage === "identical" || mainCoverage === "ahead";
    if (
      ordFailure === "newer" &&
      (ordLatest === "newer" || ordLatest === "same") &&
      coveredFailure &&
      coveredMain &&
      !evidencePending &&
      !evidenceUnknown
    ) {
      return pushDecision({
        type: "close-recovered",
        issue: canonical.number,
        recovery: { sha: newest.run.sha, runId: newest.run.id, attempt: newest.run.attempt, runUrl: newest.run.url },
      });
    }
    const why = [];
    if (ordFailure !== "newer") why.push(`vs-failure:${ordFailure}`);
    if (ordLatest !== "newer" && ordLatest !== "same") why.push(`vs-latest:${ordLatest}`);
    if (!coveredFailure) why.push(`compare-failure:${coverage ?? "unavailable"}`);
    if (!coveredMain) why.push(`compare-main:${mainCoverage ?? "unavailable"}`);
    if (evidencePending) why.push("pending-run");
    if (evidenceUnknown) why.push("unclassified-history");
    if (ordLatest === "stale" && ordFailure !== "newer") {
      return pushDecision({ type: "noop", reason: `stale-success:${why.join(",")}` });
    }
    // Even when the success does not close, a newer failure in evidence
    // (dropped notification, merged duplicate state) is still recorded.
    const failureAlsoNewer =
      newestFailure != null && orderForState(newestFailure.run, stored?.failure) === "newer";
    const next = {
      ...stored,
      latest: refOf(newest.run),
      operationalNote: `Newest success run ${newest.run.id} did not close this issue (${why.join("; ")}) — held open.`,
    };
    if (failureAlsoNewer) {
      next.failure = failureFrom(
        newestFailure.run,
        jobs,
        stored?.failure?.firstSha,
        stored?.failure?.firstAt,
      );
      next.history = historyPush(
        historyPush(next, { ...runHistoryEntry("failure", newestFailure.run, "history") }),
        { ...runHistoryEntry("success-not-covering", newest.run) },
      );
    } else {
      next.history = historyPush(next, { ...runHistoryEntry("success-not-covering", newest.run) });
    }
    return pushDecision({ type: "update", issue: canonical.number, state: next });
  }

  // ------------------------------------------------------------- failure -
  if (newest.kind === "failure") {
    const f = newestFailure?.run ?? newest.run;
    if (orderForState(f, stored?.failure) === "newer") {
      return pushDecision({
        type: "update",
        issue: canonical.number,
        state: {
          latest: refOf(f),
          failure: failureFrom(f, jobs, stored?.failure?.firstSha, stored?.failure?.firstAt),
          history: historyPush(stored ?? {}, { ...runHistoryEntry("failure", f) }),
          operationalNote: null,
        },
      });
    }
    return pushDecision({
      type: "noop",
      reason: `stale-failure:run-${f.id}a${f.attempt}<=failure-${stored?.failure?.runId ?? "?"}a${stored?.failure?.attempt ?? 1}`,
    });
  }

  // --------------------------------------- operational/observed/pending --
  // cancelled/skipped/neutral/stale/action_required/unknown conclusions,
  // marker "observed" refs, live "pending" runs, and unclassifiable
  // "unknown" history records: never a success (no close) and never by
  // itself a reason for a new issue. A newer failure in evidence (e.g. one
  // whose notification was dropped) still updates the stored failure.
  const failureIsNewer =
    newestFailure != null && orderForState(newestFailure.run, stored?.failure) === "newer";
  const latestOrder = orderForState(newest.run, stored?.latest);
  const newestDesc =
    newest.kind === "pending"
      ? `is still in progress (status \`${sanitizeText(newest.run.status) || "non-completed"}\`) — its completion is in the future`
      : newest.kind === "unknown"
        ? "could not be classified from run-history metadata (incomplete record) — held open"
        : `concluded \`${sanitizeConclusion(newest.run.conclusion)}\` — verification pending retry; counted as neither failure nor success`;
  if (failureIsNewer || latestOrder === "newer") {
    const next = {
      ...stored,
      // An unclassifiable record may carry no run id — never let it
      // overwrite the recorded latest ref.
      latest: latestOrder === "newer" && newest.run.id != null ? refOf(newest.run) : stored?.latest,
      operationalNote: `Latest observed run ${newest.run.id ?? "?"} ${newestDesc}.`,
    };
    if (failureIsNewer) {
      next.failure = failureFrom(
        newestFailure.run,
        jobs,
        stored?.failure?.firstSha,
        stored?.failure?.firstAt,
      );
      next.history = historyPush(next, { ...runHistoryEntry("failure", newestFailure.run, "history") });
    }
    // Id-less unclassifiable records live only in the note — the marker
    // history must stay runId-valid.
    if (newest.run.id != null) {
      next.history = historyPush(next, {
        ...runHistoryEntry(newest.kind === "pending" ? "pending" : "operational", newest.run),
      });
    }
    return pushDecision({ type: "update", issue: canonical.number, state: next });
  }
  return pushDecision({ type: "noop", reason: `operational-stale:run-${newest.run.id ?? "?"}` });
}

// --------------------------------------------------------------- apply ---

async function applyActions(actions, scope, client) {
  const applied = [];
  for (const a of actions) {
    if (a.type === "noop") {
      applied.push({ ...a });
      continue;
    }
    if (a.type === "close-duplicate") {
      await client.addComment(
        a.issue,
        `Superseded by #${a.canonical} — one open CI failure issue per scope. ` +
          "Newest recorded failure state was merged into the canonical issue before closing.",
      );
      await client.closeIssue(a.issue);
      applied.push({ ...a });
      continue;
    }
    if (a.type === "create") {
      const created = await client.createIssue({
        title: ISSUE_TITLE,
        body: renderBody(scope, a.state),
        labels: [scope.label],
      });
      applied.push({ ...a, issue: created?.number ?? null });
      continue;
    }
    if (a.type === "update" || a.type === "merge-state") {
      await client.updateIssue(a.issue, {
        title: ISSUE_TITLE,
        body: renderBody(scope, a.state),
      });
      applied.push({ ...a });
      continue;
    }
    if (a.type === "close-recovered") {
      await client.addComment(
        a.issue,
        [
          `Recovered: newer \`ci\` run ${a.recovery.runId}a${a.recovery.attempt ?? 1} succeeded at revision`,
          `\`${a.recovery.sha}\` covering the failed revision and the current main tip.`,
          a.recovery.runUrl ? `Run: ${a.recovery.runUrl}` : null,
        ]
          .filter(Boolean)
          .join(" "),
      );
      await client.closeIssue(a.issue);
      applied.push({ ...a });
      continue;
    }
    throw new Error(`unknown action type ${a.type}`);
  }
  return applied;
}

// ----------------------------------------------------------------- run ---

// Orchestrator: classify the trigger → ensure label → list managed issues →
// fetch authoritative run history and reconcile it with the event and all
// marker states → fetch only what the decision needs (jobs for failures,
// compares for coverage candidates) → apply. Every API error propagates —
// the tracker job must fail and stay diagnosable rather than swallow a
// permission or outage problem, and incomplete evidence must never produce
// a close or a silent success.
export async function run({ payload, client, scope }) {
  const classified = classifyRun(payload, scope);
  const result = {
    scope,
    classified,
    effective: null,
    canonical: null,
    actions: [],
    applied: [],
    mainHead: null,
  };
  if (classified.kind === "skip") {
    result.actions = [{ type: "noop", reason: classified.reason }];
    result.applied = result.actions;
    return result;
  }

  await client.ensureLabel(scope.label, LABEL_COLOR);
  const listed = await client.listOpenIssues(scope.label);
  const managed = [];
  for (const issue of listed ?? []) {
    if (issue?.pull_request != null) continue;
    const marker = parseMarker(issue?.body ?? "");
    if (marker != null && scopeMatches(marker.scope, scope)) {
      managed.push({ number: issue.number, marker });
    }
  }
  managed.sort((a, b) => a.number - b.number);
  const canonical = managed[0] ?? null;
  const duplicates = managed.slice(1);
  result.canonical = canonical?.number ?? null;

  // Merge marker states across duplicates BEFORE any decision — closing a
  // duplicate that holds the newest failure must never lose it.
  const mergedState = mergeMarkerStates(managed.map((m) => m.marker.state));

  // Authoritative run history for this workflow/branch — the notification
  // may be a dropped-event replay, so the truth is the run list, not the
  // incoming payload alone. Failure propagates (history is required
  // evidence, and missing evidence is fail-closed).
  const historyRaw = await client.getRunHistory({
    workflowFile: scope.workflowFile,
    branch: scope.branch,
  });
  // History is required evidence — a non-array response is an API failure,
  // and missing evidence is fail-closed (propagates; nothing is written).
  if (!Array.isArray(historyRaw)) {
    throw new Error("run history unavailable — cannot reconcile evidence");
  }
  const historyParts = historyRaw.map((r) => classifyHistoryRecord(r, scope));
  // Records whose scope/ordering cannot be proven still count as evidence
  // of an incomplete list: they never become success but they block close
  // and resolved-suppression.
  const unknownEvidence = historyParts
    .filter((p) => p.disposition === "unknown")
    .map((p) => ({ kind: "unknown", run: { ...p.run, src: "history-unknown" } }));
  const historyRuns = mergeRunRecords(
    [
      ...historyParts.filter((p) => p.disposition === "in").map((p) => p.run),
      // The incoming run may not be listed yet (eventual consistency), or
      // the list may lag on its latest attempt — it is still authoritative
      // for itself, so it always participates in the per-id merge.
      classified.run,
    ].filter(Boolean),
    classified.run,
  );

  // Evidence: stored refs + incoming + history. A marker ref is added only
  // when the real record for that run id is absent — the authoritative
  // record carries the actual conclusion, so it always outranks the marker
  // ref for the same run.
  const toCand = (run, src) =>
    run == null
      ? null
      : { kind: run.pending ? "pending" : kindOfConclusion(run.conclusion), run: { ...run, src } };
  // Marker refs are suppressed per (run id, attempt): the authoritative
  // record outranks the marker for the exact attempt it describes. A
  // recorded FAILURE attempt is NOT dropped when the same run id now
  // shows a newer attempt — attempts are distinct evidence, and a rerun
  // success must still be able to close its own failed attempt.
  const realAttempts = new Set(historyRuns.map((r) => `${r.id}:${r.attempt ?? 1}`));
  // The `latest` observation is superseded whenever real history carries
  // the same run id at an equal-or-newer attempt.
  const latestIsCovered =
    mergedState != null &&
    historyRuns.some(
      (r) => r.id === mergedState.latest.runId && (r.attempt ?? 1) >= (mergedState.latest.attempt ?? 1),
    );
  const storedCandidates = [];
  if (mergedState != null) {
    if (!latestIsCovered) {
      storedCandidates.push({
        kind: "observed",
        run: { ...mergedState.latest, id: mergedState.latest.runId, src: "marker" },
      });
    }
    if (!realAttempts.has(`${mergedState.failure.runId}:${mergedState.failure.attempt ?? 1}`)) {
      storedCandidates.push({
        kind: "failure",
        run: { ...mergedState.failure, id: mergedState.failure.runId, src: "marker" },
      });
    }
  }
  const evidence = [
    ...storedCandidates,
    toCand(classified.run, "event"),
    ...historyRuns.map((r) => toCand(r, "history")),
    ...unknownEvidence,
  ].filter(Boolean);
  // Evidence is incomplete while any live run (pending — its completion is
  // still in the future) or any unclassifiable record exists: a close
  // requires the evidence to be complete AND to show a verifiable success.
  const hasPending = evidence.some((c) => c.kind === "pending");
  const hasUnknown = unknownEvidence.length > 0;
  const newest = pickNewest(evidence);
  const newestFailure = pickNewest(evidence.filter((c) => c.kind === "failure"));
  result.effective = newest;

  // main head is diagnostic on most paths but REQUIRED evidence for a
  // close: its absence must never produce a close.
  let headError = null;
  try {
    result.mainHead = sanitizeSha((await client.getBranchHead(scope.branch))?.sha ?? null);
  } catch (e) {
    headError = e;
    result.mainHead = null;
  }

  // coveringSuccess: the newest success evidence that is STRICTLY newer
  // than the newest failure (ordering proven — "ambiguous" never counts).
  // It feeds both the close path (when it is the newest overall run) and
  // the create-suppression path (a resolved stale failure must not file).
  // When the newest overall run is an ambiguous success it is excluded —
  // an unproven success never resolves anything.
  const stored = mergedState;
  let coveringSuccess = null;
  let coverage = null;
  if (newestFailure != null) {
    coveringSuccess = pickNewest(
      evidence.filter(
        (c) =>
          c.kind === "success" &&
          compareRunOrder(c.run, refOf(newestFailure.run)) === "newer" &&
          !(newest?.kind === "success" && newest.ambiguous && c.run.id === newest.run.id),
      ),
    );
    if (coveringSuccess != null) {
      coverage = await client.compareCommits(newestFailure.run.sha, coveringSuccess.run.sha);
    }
  }
  // resolved suppresses a create only when coverage is proven AND the
  // evidence is complete — an unclassifiable record could hide a newer
  // failure, so incomplete evidence falls back to recording the failure.
  const resolved =
    coveringSuccess != null &&
    (coverage === "identical" || coverage === "ahead") &&
    !hasUnknown;

  // A close is a candidate only when every ordering/coverage precondition
  // already holds; only then is the main-head compare fetched. Its absence
  // is fatal to the job — propagate rather than silently close.
  let mainCoverage = null;
  const maybeClose =
    canonical != null &&
    newest != null &&
    !newest.ambiguous &&
    !hasPending &&
    !hasUnknown &&
    newest.kind === "success" &&
    coveringSuccess != null &&
    coveringSuccess.run.id === newest.run.id &&
    (stored?.latest == null ||
      ["newer", "same"].includes(compareRunOrder(newest.run, stored.latest))) &&
    resolved;
  if (maybeClose) {
    if (headError) throw headError;
    if (result.mainHead == null) {
      throw new Error("branch head unavailable — cannot prove main-tip coverage for close");
    }
    mainCoverage = await client.compareCommits(result.mainHead, newest.run.sha);
  }

  // Jobs are needed only when failure state will be written: a create on
  // unresolved failure evidence, or an update where the newest failure
  // evidence is newer than the stored one (any newest-run kind — a
  // not-covering success still merges the newer failure into the issue).
  const willWriteFailure =
    newestFailure != null &&
    (canonical == null
      ? !resolved
      : orderForState(newestFailure.run, stored?.failure) === "newer");
  let jobs = null;
  if (willWriteFailure) {
    jobs = (await client.getRunJobs(newestFailure.run.id)).map((j) => ({
      // sanitize at the input boundary: these values end up both in the
      // rendered table and inside the marker JSON state.
      name: sanitizeText(j?.name),
      conclusion: sanitizeConclusion(j?.conclusion),
    }));
  }

  result.actions = plan({
    classified,
    canonical,
    duplicates,
    mergedState,
    newest,
    newestFailure,
    coveringSuccess,
    coverage,
    mainCoverage,
    jobs,
    evidencePending: hasPending,
    evidenceUnknown: hasUnknown,
  });
  result.applied = await applyActions(result.actions, scope, client);
  return result;
}

// ------------------------------------------------------------ gh client --

// Thin `gh api` wrapper — gh CLI is preinstalled on every GitHub-hosted
// runner and authenticates via GH_TOKEN. JSONL outputs (`--paginate --jq`)
// are parsed line-wise so pagination never needs JSON-array juggling.
export function makeGhClient({ repo, gh = "gh", execFile = execFileSync } = {}) {
  function api(args) {
    return execFile(gh, ["api", ...args], { encoding: "utf8" });
  }
  function apiJson(args) {
    return JSON.parse(api(args));
  }
  function apiJsonLines(args) {
    return api(args)
      .split("\n")
      .filter((l) => l.trim() !== "")
      .map((l) => JSON.parse(l));
  }
  return {
    async ensureLabel(name, color) {
      try {
        api([`repos/${repo}/labels/${encodeURIComponent(name)}`]);
        return;
      } catch {
        // missing label — create below; any other error surfaces at POST.
      }
      api([`repos/${repo}/labels`, "-f", `name=${name}`, "-f", `color=${color}`]);
    },
    async listOpenIssues(label) {
      return apiJsonLines([
        `repos/${repo}/issues?labels=${encodeURIComponent(label)}&state=open&per_page=100`,
        "--paginate",
        "--jq",
        ".[]",
      ]);
    },
    async getRunHistory({ workflowFile, branch }) {
      // Latest-attempt state per run for this workflow+branch. The file
      // name form of workflow_id is used so consumers keep a declarative
      // workflow mapping. `exclude_pull_requests` drops PR-triggered runs
      // at the API level; event/branch/repo are still re-checked locally.
      const runs = [];
      for (let page = 1; ; page += 1) {
        const out = apiJson([
          `repos/${repo}/actions/workflows/${encodeURIComponent(workflowFile)}/runs?branch=${encodeURIComponent(branch)}&exclude_pull_requests=true&per_page=${HISTORY_PAGE_SIZE}&page=${page}`,
        ]);
        if (!Array.isArray(out?.workflow_runs)) {
          throw new Error(`run history page ${page} returned a response without a workflow_runs array`);
        }
        runs.push(...out.workflow_runs);
        if (out.workflow_runs.length < HISTORY_PAGE_SIZE) return runs;
      }
    },
    async getRunJobs(runId) {
      return apiJsonLines([
        `repos/${repo}/actions/runs/${runId}/jobs?per_page=100`,
        "--paginate",
        "--jq",
        ".jobs[]",
      ]);
    },
    async compareCommits(base, head) {
      const out = apiJson([`repos/${repo}/compare/${base}...${head}`]);
      const status = out?.status;
      if (!["identical", "ahead", "behind", "diverged"].includes(status)) {
        throw new Error(`compare returned unexpected status: ${status}`);
      }
      return status;
    },
    async getBranchHead(branch) {
      const out = apiJson([`repos/${repo}/branches/${encodeURIComponent(branch)}`]);
      return { sha: out?.commit?.sha ?? null };
    },
    async createIssue({ title, body, labels }) {
      const args = [`repos/${repo}/issues`, "-f", `title=${title}`, "-f", `body=${body}`];
      for (const l of labels) args.push("-f", `labels[]=${l}`);
      return apiJson(args);
    },
    async updateIssue(number, { title, body }) {
      return apiJson([
        `repos/${repo}/issues/${number}`,
        "-X",
        "PATCH",
        "-f",
        `title=${title}`,
        "-f",
        `body=${body}`,
      ]);
    },
    async addComment(number, body) {
      return apiJson([`repos/${repo}/issues/${number}/comments`, "-f", `body=${body}`]);
    },
    async closeIssue(number) {
      return apiJson([`repos/${repo}/issues/${number}`, "-X", "PATCH", "-f", "state=closed"]);
    },
  };
}

// ---------------------------------------------------------------- main ---

function summaryLines(result) {
  const c = result.classified;
  const e = result.effective;
  const lines = [
    `## ci-issue-tracker`,
    "",
    `- scope: \`${result.scope.repo}\` × \`${result.scope.workflow}\` × \`${result.scope.branch}\``,
    `- trigger run: ${c.run ? `id ${c.run.id}a${c.run.attempt} · sha \`${c.run.sha}\` · conclusion \`${c.run.conclusion}\` · ${c.run.url ?? "?"}` : "-"}`,
    `- classified: \`${c.kind}\`${c.reason ? ` (${c.reason})` : ""}`,
    `- newest evidence: ${e?.run ? `id ${e.run.id}a${e.run.attempt} · \`${e.kind}\`${e.ambiguous ? " (ambiguous ordering)" : ""}` : "-"}`,
    `- current main head: \`${result.mainHead ?? "?"}\``,
    `- canonical issue: ${result.canonical != null ? `#${result.canonical}` : "none"}`,
    "",
    "| action | detail |",
    "| --- | --- |",
    ...result.applied.map(
      (a) =>
        `| ${a.type} | ${a.issue != null ? `#${a.issue}` : ""} ${a.reason ?? ""}${a.recovery ? `recovered at ${a.recovery.sha}` : ""} |`.trimEnd(),
    ),
    "",
  ];
  return lines.join("\n");
}

export async function main(argv = process.argv, env = process.env) {
  const args = {
    event: env.TRACKER_EVENT_PATH ?? env.GITHUB_EVENT_PATH ?? null,
    repo: env.GITHUB_REPOSITORY ?? null,
    branch: "main",
    workflow: "ci",
    workflowFile: null,
    label: TRACKER_LABEL,
    report: null,
    summary: null,
  };
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    const next = () => argv[++i];
    if (a === "--event") args.event = next();
    else if (a === "--repo") args.repo = next();
    else if (a === "--branch") args.branch = next();
    else if (a === "--workflow") args.workflow = next();
    else if (a === "--workflow-file") args.workflowFile = next();
    else if (a === "--label") args.label = next();
    else if (a === "--report") args.report = next();
    else if (a === "--summary") args.summary = next();
    else if (a === "--help" || a === "-h") {
      console.log(
        "Usage: node ci/tracker/tracker.mjs [--event <file>] [--repo <owner/name>] [--branch <b>] [--workflow <name>] [--workflow-file <f.yml>] [--label <l>] [--report <file>] [--summary <file>]  (event path also read from $TRACKER_EVENT_PATH/$GITHUB_EVENT_PATH)",
      );
      process.exit(0);
    } else {
      console.error(`error: unknown argument ${a}`);
      process.exit(2);
    }
  }
  if (args.event == null) {
    console.error("error: no event payload (pass --event <file> or set TRACKER_EVENT_PATH)");
    process.exit(2);
  }
  if (args.repo == null) {
    console.error("error: no repo (pass --repo owner/name or set GITHUB_REPOSITORY)");
    process.exit(2);
  }
  const payload = JSON.parse(readFileSync(args.event, "utf8"));
  const scope = {
    ...defaultScope(args.repo),
    branch: args.branch,
    workflow: args.workflow,
    workflowFile: args.workflowFile ?? `${args.workflow}.yml`,
    label: args.label,
  };
  const client = makeGhClient({ repo: args.repo });
  const result = await run({ payload, client, scope });

  const report = {
    schemaVersion: 1,
    kind: "ci-issue-tracker",
    repo: args.repo,
    scope: { workflow: scope.workflow, branch: scope.branch, label: scope.label },
    classified: result.classified,
    effective: result.effective,
    canonical: result.canonical,
    mainHead: result.mainHead,
    actions: result.applied,
    generatedAt: new Date().toISOString(),
  };
  if (args.report) writeFileSync(args.report, JSON.stringify(report, null, 2) + "\n");

  const table = summaryLines(result);
  process.stdout.write(table + "\n");
  const summaryPath = args.summary ?? env.GITHUB_STEP_SUMMARY;
  if (summaryPath) appendFileSync(summaryPath, table + "\n");
}

const isMain =
  process.argv[1] != null &&
  import.meta.url === pathToFileURL(process.argv[1]).href;
if (isMain) {
  main().catch((e) => {
    console.error(`error: ${e?.message ?? e}`);
    process.exit(1);
  });
}
