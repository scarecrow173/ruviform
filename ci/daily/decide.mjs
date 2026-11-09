#!/usr/bin/env node
// decide.mjs — Issue #367 common daily CI scheduler decision engine.
//
// Decides, once per day (UTC schedule) or on manual re-check, whether the
// current main HEAD already has a valid Full CI result in the GitHub
// Actions run history of the target workflow (default `ci.yml`). If not,
// it dispatches that workflow on `ref: main` with `expected_sha` set to
// the observed SHA — the Issue #366/#367 dispatch contract enforced by
// `ci-summary` against the measured common checkout SHA.
//
// The ONLY state authority is the GitHub Actions run history itself: no
// dedicated DB, no repository variables. Every difficulty (eventual
// consistency, retention expiry, reruns, dispatch/landing races) is
// absorbed by strict matching plus fail-closed classification — a result
// may only be recorded as verified when the API reports a completed
// `success` run whose head_sha/head_branch/event all match.
//
// Strict matching (spec R2): an attempt "on" SHA S is a `ci.yml` run with
// head_sha === S AND head_branch === mainRef AND event ∈ FULL_CI_EVENTS
// ({push, workflow_dispatch, schedule} — pull_request runs point at the
// PR head/branch and never qualify). Scheduler runs, other workflows,
// other branches, other SHAs, and PR runs contribute nothing.
//
// Retry budget (spec R4): auto dispatches for S are counted as runs with
// event === "workflow_dispatch" whose ORIGIN is automation. Per the API
// contract, `actor` is the run's original initiator and `triggering_actor`
// is the actor of the LATEST (re)run attempt — so a bot-dispatched run
// rerun by a human keeps actor=github-actions[bot] while
// triggering_actor=<human>. Counting triggering_actor alone would refund
// the budget on every human rerun, so a run counts as auto when EITHER
// provenance field names an auto actor, and a run with no identifiable
// actor at all counts too (an undercounted budget retries forever —
// overcounting merely dispatches less, the safe side). Human
// `workflow_dispatch` runs do not consume the budget.
// run status/conclusion/run_attempt always describe the run's LATEST
// attempt, matching the "latest attempt on the SHA" decision rule.
//
// Tuple mode (Issue #294, `--mode tuple` / `DAILY_MODE=tuple`): the unit of
// verification is the candidate tuple
//   (ruviform main SHA, consumer main SHAs..., "consumer-verify-tuple/v1")
// evaluated against `consumer-verify.yml` run history. Workflow run records
// do not expose dispatch inputs, so the tuple each run verified is recovered
// from its `consumer-evidence-<consumer>-attempt-<n>` artifacts. A run only
// counts as verified for the candidate tuple when it completed `success`
// AND every consumer's evidence reports `consumer.status === "pass"` AND
// the recovered tuple matches all candidate components exactly. Missing,
// expired, malformed, or inconsistent evidence is "unverifiable" — never
// verified, never silently ignored for retry accounting.
//
// Usage:
//   GITHUB_TOKEN=... GITHUB_REPOSITORY=owner/repo node ci/daily/decide.mjs
//       [--workflow ci.yml] [--main-ref main] [--api-base URL]
//       [--max-auto-dispatches 2] [--max-pages 10]
//       [--confirm-timeout-ms 120000] [--confirm-interval-ms 15000]
//       [--report FILE] [--summary FILE] [--fixture FILE] [--dry-run]
//   # tuple mode:
//   ... node ci/daily/decide.mjs --mode tuple --workflow consumer-verify.yml
//       [--consumer-repo audiolens=owner/repo[,synvelis=owner/repo]]
//       [--tuple-contract consumer-verify-tuple/v1] [--max-evidence-runs 60]
//
// Exit codes: 0 every completed outcome (including budget-exhausted /
// hold-unknown / history-truncated / main-moved / dispatched-unconfirmed
// — the decision ran to completion and the unverified state is reported
// via warnings), 1 on `error` (the scheduler could not complete its
// decision — API failure, malformed response, unreadable fixture), 2 usage.

import { execFile } from "node:child_process";
import { appendFileSync, readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import { promisify } from "node:util";
import { inflateRawSync } from "node:zlib";

const execFileAsync = promisify(execFile);

// Events that represent a Full CI execution of the target workflow.
// pull_request runs are excluded structurally (head_branch is the PR
// source branch, head_sha is the PR head). Unknown future events are
// allowlist-external and can never contribute a false verification.
export const FULL_CI_EVENTS = ["push", "workflow_dispatch", "schedule"];
// GitHub-documented non-terminal statuses; anything else is treated as
// unknown and holds the decision rather than guessing.
export const IN_FLIGHT_STATUSES = ["queued", "waiting", "in_progress", "pending", "requested"];
export const TERMINAL_STATUS = "completed";
// GitHub-documented terminal conclusions. A completed run whose
// conclusion is null or outside this set is classified "unknown" —
// holding the decision rather than dispatching on an unrecognised state.
export const KNOWN_CONCLUSIONS = ["success", "failure", "neutral", "cancelled", "skipped", "timed_out", "action_required", "startup_failure", "stale"];

const SHA_RE = /^[0-9a-f]{40}$/;
const MAX_ATTEMPTS_IN_REPORT = 20;

class ApiError extends Error {
  constructor(message, status = null, body = null) {
    super(message);
    this.status = status;
    this.body = body;
  }
}

// ---------------------------------------------------------------------------
// Decision core (pure)
// ---------------------------------------------------------------------------

function isFullCiRun(run, mainRef) {
  return (
    run != null &&
    typeof run === "object" &&
    run.head_branch === mainRef &&
    FULL_CI_EVENTS.includes(run.event) &&
    typeof run.head_sha === "string" &&
    SHA_RE.test(run.head_sha)
  );
}

// A record may only be EXCLUDED from the SHA's attempt set when some
// field positively proves it cannot be a Full CI run on that SHA:
//   valid head_sha !== sha | non-empty head_branch !== mainRef
//   | non-empty event outside FULL_CI_EVENTS.
// Anything else — {}, missing/shapeless identification fields — is
// UNIDENTIFIABLE: it cannot be proven on-target, but cannot be proven
// off-target either, so it may hide an attempt in any state.
function provenNonTarget(run, mainSha, mainRef) {
  if (run == null || typeof run !== "object") return false;
  if (typeof run.head_sha === "string" && SHA_RE.test(run.head_sha) && run.head_sha !== mainSha) return true;
  if (typeof run.head_branch === "string" && run.head_branch !== "" && run.head_branch !== mainRef) return true;
  if (typeof run.event === "string" && run.event !== "" && !FULL_CI_EVENTS.includes(run.event)) return true;
  return false;
}

// Latest-attempt recency. `created_at` never moves (original creation);
// `run_started_at` is the start of the run's LATEST attempt, so a rerun
// moves it forward even though created_at stays old (verified on the
// real API: run 41958976200 attempt 2, created 03:01 / started 03:07).
// `updated_at` is excluded from the ordering key — it also moves on
// non-attempt bookkeeping — but is used as an ambiguity probe below:
// a run with recent activity that produced no newer attempt is
// unexplained evidence and must block a success claim, not be ignored.
function attemptTime(r) {
  let best = null;
  for (const key of ["created_at", "run_started_at"]) {
    const t = Date.parse(r?.[key] ?? "");
    if (!Number.isNaN(t) && (best == null || t > best)) best = t;
  }
  return best; // null = unorderable
}
function activityTime(r) {
  let best = attemptTime(r);
  const t = Date.parse(r?.updated_at ?? "");
  if (!Number.isNaN(t) && (best == null || t > best)) best = t;
  return best;
}

// Newest-first ordering: latest-attempt recency desc, then run id desc
// (higher id was created later when timestamps tie). Runs with no
// parseable timestamps sort last — they are unorderable, not newest.
function newestFirst(a, b) {
  const ta = attemptTime(a);
  const tb = attemptTime(b);
  if (ta !== tb) {
    if (ta == null) return 1;
    if (tb == null) return -1;
    return tb - ta;
  }
  return (b.id ?? 0) - (a.id ?? 0);
}

function actorOf(run) {
  return run?.triggering_actor?.login ?? run?.actor?.login ?? null;
}

function isSuccess(r) {
  return r != null && r.status === TERMINAL_STATUS && r.conclusion === "success";
}

// Coarse fail-closed classification of one run's latest-attempt state.
function classifyRun(r) {
  if (r.status !== TERMINAL_STATUS) {
    return IN_FLIGHT_STATUSES.includes(r.status) ? "in-flight" : "unknown";
  }
  if (!KNOWN_CONCLUSIONS.includes(r.conclusion)) return "unknown";
  return r.conclusion === "success" ? "success" : "terminal-non-success";
}

// Retry-budget accounting (see header): automation-origin dispatches.
// actor = original initiator, triggering_actor = latest attempt's
// triggerer — a human rerun of a bot dispatch does not refund budget.
// A dispatch counts as auto unless BOTH provenance fields identify as
// non-auto actors: one side that is null/empty/malformed cannot disprove
// an automated origin (undercounting retries forever — the unsafe side).
function isAutoDispatch(run, autoActors) {
  if (run?.event !== "workflow_dispatch") return false;
  const ident = (u) => (typeof u?.login === "string" && u.login !== "" ? u.login : null);
  const actor = ident(run.actor);
  const triggerer = ident(run.triggering_actor);
  if (actor != null && triggerer != null) {
    return autoActors.includes(actor) || autoActors.includes(triggerer);
  }
  return true;
}

// Evaluates the current main SHA against the run history.
//   decide({ mainSha, runs, mainRef, autoActors, maxAutoDispatches })
//   -> { decision, reason, latest, inFlight, autoDispatches, latestVerified }
// decision ∈ skip-verified | suppress-in-flight | hold-unknown |
//            budget-exhausted | dispatch
export function decide({ mainSha, runs, mainRef = "main", autoActors = ["github-actions[bot]"], maxAutoDispatches = 2 }) {
  const all = runs ?? [];
  const attempts = all
    .filter((r) => isFullCiRun(r, mainRef) && r.head_sha === mainSha)
    .sort(newestFirst);
  // Records that can neither be proven on-target nor proven off-target.
  // They may hide an attempt on this SHA in ANY state — in-flight,
  // success, or a consumed auto dispatch — so no verdict is safe while
  // they are present: not dispatch (could double-fire), not skip-verified
  // (an unverified attempt could hide behind a visible success).
  const unidentifiable = all.filter(
    (r) => !(isFullCiRun(r, mainRef) && r.head_sha === mainSha) && !provenNonTarget(r, mainSha, mainRef),
  );
  const latest = attempts[0] ?? null;
  const inFlight = attempts.filter((r) => classifyRun(r) === "in-flight");
  const autoDispatches = attempts.filter((r) => isAutoDispatch(r, autoActors));

  const verifiedPool = all
    .filter((r) => isFullCiRun(r, mainRef) && isSuccess(r))
    .sort(newestFirst);
  const latestVerified = verifiedPool[0] ?? null;

  let decision;
  let reason;
  if (inFlight.length > 0) {
    // Something is still running for this SHA — never double-dispatch.
    decision = "suppress-in-flight";
    reason = `${inFlight.length} in-flight run(s) on ${mainSha.slice(0, 12)}`;
  } else if (unidentifiable.length > 0) {
    decision = "hold-unknown";
    reason = `${unidentifiable.length} history record(s) cannot be proven target or non-target — cannot certify the verification state`;
  } else if (latest != null && classifyRun(latest) === "unknown") {
    // The newest attempt reports a status or conclusion we cannot
    // classify — hold rather than guess success or dispatch a duplicate.
    decision = "hold-unknown";
    reason = `unrecognised state "${latest.status}"/"${latest.conclusion ?? "null"}" on latest run ${latest.id}`;
  } else if (latest != null && isSuccess(latest)) {
    // A success claim is only sound if `latest` is provably the newest
    // effective attempt on the SHA. A tied or unorderable non-success
    // run, or one with newer unexplained activity (updated_at past the
    // latest attempt start — e.g. a rerun not yet reflected in
    // run_started_at), makes the "latest" ambiguous: hold, never
    // record a false verified.
    const top = attemptTime(latest);
    const contender = attempts.find((r) => {
      if (r === latest || isSuccess(r)) return false;
      const at = attemptTime(r);
      if (top == null || at == null || at === top) return true;
      const act = activityTime(r);
      return act != null && act > top;
    });
    if (contender != null) {
      decision = "hold-unknown";
      reason = `latest-attempt ordering ambiguous: run ${contender.id} contests success run ${latest.id}`;
    } else {
      decision = "skip-verified";
      reason = `run ${latest.id} completed success on ${mainSha.slice(0, 12)}`;
    }
  } else {
    const base = latest == null ? "no-history" : `latest:${latest.status}/${latest.conclusion ?? "null"}`;
    if (autoDispatches.length >= maxAutoDispatches) {
      decision = "budget-exhausted";
      reason = `${base}; ${autoDispatches.length} auto dispatches >= limit ${maxAutoDispatches}`;
    } else {
      decision = "dispatch";
      reason = `${base}; auto dispatches ${autoDispatches.length}/${maxAutoDispatches}`;
    }
  }

  return { decision, reason, attempts, latest, inFlight, autoDispatches, latestVerified, unidentifiable };
}

// ---------------------------------------------------------------------------
// Tuple-mode decision core (Issue #294)
// ---------------------------------------------------------------------------
//
// The tuple-mode history scan classifies every in-scope terminal run into
// exactly one of three buckets before decideTuple sees it:
//   covering     — evidence recovered AND the run's tuple equals the
//                  candidate tuple on every component (ruviformRev,
//                  every consumerResolvedSha, contract). consumerStatus
//                  carries each consumer's evidence status.
//   non-target   — evidence recovered AND provably a DIFFERENT tuple
//                  (any definite component mismatch, including a
//                  different contract version). Contributes nothing.
//   unverifiable — evidence could not be recovered or fails validation
//                  (artifact missing/expired/corrupt, inconsistent tuple
//                  fields across consumers, runId mismatch). Never counts
//                  toward verified; auto-origin ones count toward the
//                  retry budget conservatively.
//
// In-flight runs are NOT evidence-scanned: an unfinished run cannot prove
// which tuple it belongs to, so every in-scope in-flight run suppresses
// dispatch (a foreign-tuple in-flight delays us one cycle — never a
// duplicate dispatch).

// A covering entry counts as verified for the candidate tuple only when
// the run completed `success` AND every configured consumer's recovered
// evidence reports status "pass". not_run / unsupported / fail consumers
// make the run a partial verification — the tuple stays unverified.
function isVerifiedCovering(entry, consumers) {
  if (entry == null || !isSuccess(entry.run)) return false;
  const status = entry.consumerStatus;
  if (status == null || typeof status !== "object") return false;
  return consumers.every((c) => status[c.name] === "pass");
}

// Evaluates the candidate tuple against covering/in-flight/unverifiable
// run classifications. Decision vocabulary matches decide():
//   skip-verified | suppress-in-flight | hold-unknown |
//   budget-exhausted | dispatch
export function decideTuple({
  candidate,
  covering = [],
  inFlight = [],
  unverifiable = [],
  autoActors = ["github-actions[bot]"],
  maxAutoDispatches = 2,
}) {
  const consumers = candidate?.consumers ?? [];
  const sortedCovering = [...covering].sort((a, b) => newestFirst(a.run, b.run));
  const latest = sortedCovering[0] ?? null;
  const verified = (e) => isVerifiedCovering(e, consumers);
  const latestVerified = sortedCovering.find(verified) ?? null;

  let decision;
  let reason;
  if (inFlight.length > 0) {
    // An in-flight run cannot prove its tuple — it may be verifying ours.
    // Suppress rather than risk a duplicate dispatch.
    decision = "suppress-in-flight";
    reason = `${inFlight.length} in-flight run(s) (tuple unverifiable until completion)`;
  } else if (latest != null && classifyRun(latest.run) === "unknown") {
    decision = "hold-unknown";
    reason = `unrecognised state "${latest.run.status}"/"${latest.run.conclusion ?? "null"}" on latest covering run ${latest.run.id}`;
  } else if (latest != null && verified(latest)) {
    // A verified claim is sound only if `latest` is provably the newest
    // effective attempt on this tuple. A non-verified covering run with
    // tied/unorderable/newer-unexplained activity, or an unverifiable run
    // that is newer-or-unorderable relative to it, could hide a newer
    // attempt — hold, never record a false verified.
    const top = attemptTime(latest.run);
    const contender = sortedCovering.find((e) => {
      if (e === latest || verified(e)) return false;
      const at = attemptTime(e.run);
      if (top == null || at == null || at === top) return true;
      const act = activityTime(e.run);
      return act != null && act > top;
    });
    const uContender = unverifiable.find((u) => {
      const at = attemptTime(u.run);
      return at == null || top == null || at >= top;
    });
    if (contender != null || uContender != null) {
      const hid = contender?.run ?? uContender?.run;
      decision = "hold-unknown";
      reason = `latest-attempt ordering ambiguous on tuple: run ${hid.id} contests verified run ${latest.run.id}`;
    } else {
      decision = "skip-verified";
      reason = `run ${latest.run.id} completed success with all consumers pass on the candidate tuple`;
    }
  } else {
    // No verified covering run on the candidate tuple. Retry budget counts
    // auto dispatches attributable to this tuple (covering) PLUS
    // unverifiable auto dispatches — an unverifiable run cannot disprove
    // an automated origin on this tuple, so it counts on the safe side
    // (overcounting dispatches less; undercounting retries forever).
    const autoOnTuple =
      sortedCovering.filter((e) => isAutoDispatch(e.run, autoActors)).length +
      unverifiable.filter((u) => isAutoDispatch(u.run, autoActors)).length;
    const base = latest == null ? "no-covering-history" : `latest-covering:${latest.run.status}/${latest.run.conclusion ?? "null"}`;
    if (autoOnTuple >= maxAutoDispatches) {
      decision = "budget-exhausted";
      reason = `${base}; ${autoOnTuple} auto dispatches (covering+unverifiable) >= limit ${maxAutoDispatches}`;
    } else {
      decision = "dispatch";
      reason = `${base}; auto dispatches ${autoOnTuple}/${maxAutoDispatches}`;
    }
  }

  return {
    decision,
    reason,
    covering: sortedCovering,
    latest,
    latestVerified,
    inFlight,
    unverifiable,
    autoOnTuple:
      sortedCovering.filter((e) => isAutoDispatch(e.run, autoActors)).length +
      unverifiable.filter((u) => isAutoDispatch(u.run, autoActors)).length,
  };
}

// Classifies one terminal run's recovered evidence against the candidate
// tuple. `evidences` maps consumer name -> parsed evidence JSON (null when
// unrecoverable). The candidate's consumer list (name/repo/resolved sha)
// is authoritative — config consumers carry no sha. Returns
// { kind: "covering", consumerStatus } | { kind: "non-target" } |
// { kind: "unverifiable" }.
export function classifyRunEvidence({ run, evidences, candidate, contract }) {
  const consumers = candidate?.consumers ?? [];
  const evs = consumers.map((c) => evidences?.get(c.name) ?? null);
  if (evs.some((e) => e == null)) return { kind: "unverifiable" };
  for (let i = 0; i < consumers.length; i++) {
    const ev = evs[i];
    const c = consumers[i];
    const t = ev?.tuple;
    if (
      ev.kind !== "consumer-verify-evidence" ||
      typeof ev.contract !== "string" ||
      ev.contract === "" ||
      ev.run?.runId !== String(run.id) ||
      ev.consumer?.name !== c.name ||
      ev.consumer?.repo !== c.repo ||
      typeof ev.status !== "string" ||
      ev.status === "" ||
      t == null ||
      typeof t.ruviformRev !== "string" ||
      !SHA_RE.test(t.ruviformRev) ||
      typeof t.consumerResolvedSha !== "string" ||
      !SHA_RE.test(t.consumerResolvedSha)
    ) {
      return { kind: "unverifiable" };
    }
  }
  // Cross-evidence consistency: both consumers must attest the SAME
  // ruviform rev and contract — disagreement means the evidence set is
  // internally inconsistent and proves nothing.
  const rfRevs = new Set(evs.map((e) => e.tuple.ruviformRev));
  const contracts = new Set(evs.map((e) => e.contract));
  if (rfRevs.size !== 1 || contracts.size !== 1) return { kind: "unverifiable" };
  const match =
    evs[0].tuple.ruviformRev === candidate.ruviformSha &&
    evs[0].contract === contract &&
    consumers.every((c, i) => evs[i].tuple.consumerResolvedSha === c.sha);
  if (!match) return { kind: "non-target" };
  return {
    kind: "covering",
    consumerStatus: Object.fromEntries(consumers.map((c, i) => [c.name, evs[i].status])),
  };
}

// ---------------------------------------------------------------------------
// Artifact evidence recovery helpers (stdlib only)
// ---------------------------------------------------------------------------

// Minimal ZIP reader for `actions/upload-artifact` archives: EOCD scan ->
// central directory -> first `.json` entry (fallback: first entry) ->
// local header -> stored (method 0) or deflate (method 8, raw) payload ->
// UTF-8 JSON.parse. Sizes always come from the central directory, so a
// data-descriptor local header cannot corrupt extraction. Anything that
// is not a well-formed zip with a parseable JSON entry is an ApiError —
// callers treat that as "unverifiable", never as verified.
export function jsonFromArtifactZip(buf) {
  const b = Buffer.isBuffer(buf) ? buf : Buffer.from(buf);
  if (b.length < 22) throw new ApiError("artifact zip: too small");
  let eocd = -1;
  const floor = Math.max(0, b.length - (22 + 0xffff));
  for (let i = b.length - 22; i >= floor; i--) {
    if (b.readUInt32LE(i) === 0x06054b50) {
      eocd = i;
      break;
    }
  }
  if (eocd < 0) throw new ApiError("artifact zip: no end-of-central-directory record");
  const count = b.readUInt16LE(eocd + 10);
  const cdOffset = b.readUInt32LE(eocd + 16);
  const entries = [];
  let off = cdOffset;
  for (let i = 0; i < count; i++) {
    if (off + 46 > b.length || b.readUInt32LE(off) !== 0x02014b50) {
      throw new ApiError("artifact zip: malformed central directory entry");
    }
    const method = b.readUInt16LE(off + 10);
    const compSize = b.readUInt32LE(off + 20);
    const nameLen = b.readUInt16LE(off + 28);
    const extraLen = b.readUInt16LE(off + 30);
    const commentLen = b.readUInt16LE(off + 32);
    const localOff = b.readUInt32LE(off + 42);
    const name = b.toString("utf8", off + 46, off + 46 + nameLen);
    entries.push({ name, method, compSize, localOff });
    off += 46 + nameLen + extraLen + commentLen;
  }
  if (entries.length === 0) throw new ApiError("artifact zip: empty archive");
  const pick = entries.find((e) => /\.json$/i.test(e.name)) ?? entries[0];
  const lo = pick.localOff;
  if (lo + 30 > b.length || b.readUInt32LE(lo) !== 0x04034b50) {
    throw new ApiError("artifact zip: malformed local file header");
  }
  const dataStart = lo + 30 + b.readUInt16LE(lo + 26) + b.readUInt16LE(lo + 28);
  if (dataStart + pick.compSize > b.length) {
    throw new ApiError("artifact zip: entry data out of bounds");
  }
  const data = b.subarray(dataStart, dataStart + pick.compSize);
  let raw;
  if (pick.method === 0) {
    raw = data;
  } else if (pick.method === 8) {
    try {
      raw = inflateRawSync(data);
    } catch (e) {
      throw new ApiError(`artifact zip: inflate failed: ${e.message}`);
    }
  } else {
    throw new ApiError(`artifact zip: unsupported compression method ${pick.method}`);
  }
  let parsed;
  try {
    parsed = JSON.parse(raw.toString("utf8"));
  } catch (e) {
    throw new ApiError(`artifact zip: entry "${pick.name}" is not valid JSON: ${e.message}`);
  }
  return parsed;
}

// Strict `git ls-remote <url> refs/heads/<ref>` output parser: exactly one
// line of `<40hex>\trefs/heads/<ref>`. Anything else — empty output,
// multiple matches, non-SHA first field, wrong ref tail — is an ApiError
// so a consumer SHA is never guessed.
export function parseLsRemote(stdout, ref) {
  const lines = String(stdout ?? "")
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l.length > 0);
  const valid = lines.filter((l) => {
    const parts = l.split(/\s+/);
    return parts.length === 2 && SHA_RE.test(parts[0]) && parts[1] === `refs/heads/${ref}`;
  });
  if (lines.length !== 1 || valid.length !== 1) {
    throw new ApiError(`git ls-remote refs/heads/${ref} returned ${lines.length} line(s), expected exactly 1 matching ref`);
  }
  return valid[0].split(/\s+/)[0];
}

// Recover one run's per-consumer evidence JSON. For each configured
// consumer, select the `consumer-evidence-<name>-attempt-<attempt>`
// artifact (skipping expired entries), download its zip, and parse the
// JSON. Any per-artifact failure yields null for that consumer — the run
// becomes "unverifiable" downstream. Only the artifact LIST call may
// throw (an API failure is an `error` outcome, not unverifiable).
async function recoverRunEvidence({ listRunArtifacts, downloadArtifactZip, runId, attempt, consumers }) {
  const artifacts = await listRunArtifacts(runId);
  const result = new Map();
  for (const c of consumers) {
    const wanted = `consumer-evidence-${c.name}-attempt-${attempt}`;
    const art = (artifacts ?? []).find((a) => a != null && typeof a === "object" && a.name === wanted);
    if (art == null || art.expired === true || typeof art.id !== "number") {
      result.set(c.name, null);
      continue;
    }
    try {
      const zip = await downloadArtifactZip(art.id);
      result.set(c.name, jsonFromArtifactZip(zip));
    } catch {
      result.set(c.name, null);
    }
  }
  return result;
}

// Fixture-only ZIP writer (stored method, no compression): produces real
// zip bytes so tests exercise the same jsonFromArtifactZip path as
// production. CRC32 table implementation — no dependencies.
const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();

function crc32(buf) {
  let c = 0xffffffff;
  for (const byte of buf) c = CRC_TABLE[(c ^ byte) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

export function buildStoredZip(entries) {
  const locals = [];
  const centrals = [];
  let offset = 0;
  for (const e of entries) {
    const name = Buffer.from(e.name, "utf8");
    const data = Buffer.isBuffer(e.data) ? e.data : Buffer.from(e.data);
    const crc = crc32(data);
    const local = Buffer.alloc(30);
    local.writeUInt32LE(0x04034b50, 0);
    local.writeUInt16LE(20, 4);
    local.writeUInt16LE(0, 6);
    local.writeUInt16LE(0, 8);
    local.writeUInt32LE(crc, 14);
    local.writeUInt32LE(data.length, 18);
    local.writeUInt32LE(data.length, 22);
    local.writeUInt16LE(name.length, 26);
    local.writeUInt16LE(0, 28);
    locals.push(local, name, data);
    const central = Buffer.alloc(46);
    central.writeUInt32LE(0x02014b50, 0);
    central.writeUInt16LE(20, 4);
    central.writeUInt16LE(20, 6);
    central.writeUInt16LE(0, 8);
    central.writeUInt16LE(0, 10);
    central.writeUInt32LE(crc, 16);
    central.writeUInt32LE(data.length, 20);
    central.writeUInt32LE(data.length, 24);
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

// ---------------------------------------------------------------------------
// GitHub API client + fixture client
// ---------------------------------------------------------------------------

export function createGitHubClient({ token, repo, apiBase = "https://api.github.com", fetchImpl = fetch }) {
  const base = apiBase.replace(/\/+$/, "");
  async function request(method, path, body) {
    let res;
    try {
      res = await fetchImpl(`${base}${path}`, {
        method,
        headers: {
          Authorization: `Bearer ${token}`,
          Accept: "application/vnd.github+json",
          "X-GitHub-Api-Version": "2022-11-28",
          "User-Agent": "ruviform-daily-ci",
        },
        body: body == null ? undefined : JSON.stringify(body),
      });
    } catch (e) {
      throw new ApiError(`github api ${method} ${path} failed: ${e.message}`);
    }
    const text = await res.text();
    if (!res.ok) {
      throw new ApiError(`github api ${method} ${path} -> ${res.status}`, res.status, text.slice(0, 500));
    }
    if (!text) return null;
    try {
      return JSON.parse(text);
    } catch (e) {
      throw new ApiError(`github api ${method} ${path} invalid JSON: ${e.message}`);
    }
  }

  const client = {
    // heads/<mainRef> must resolve to a commit sha — validate before use.
    async getRefSha(ref) {
      const data = await request("GET", `/repos/${repo}/git/ref/heads/${ref}`);
      const sha = data?.object?.sha;
      if (typeof sha !== "string" || !SHA_RE.test(sha)) {
        throw new ApiError(`ref heads/${ref} did not resolve to a commit sha`);
      }
      return sha;
    },
    // Paginate the workflow run history. The server-side `branch` filter is
    // an optimisation only — strict matching lives in decide(). A 200
    // body without the workflow_runs array is a malformed response —
    // an error, never "empty history" (empty history dispatches).
    async listRuns({ workflow, branch, perPage = 100, maxPages = 10 }) {
      const all = [];
      for (let page = 1; page <= maxPages; page++) {
        const data = await request(
          "GET",
          `/repos/${repo}/actions/workflows/${workflow}/runs?branch=${encodeURIComponent(branch)}&per_page=${perPage}&page=${page}`,
        );
        if (data == null || !Array.isArray(data.workflow_runs)) {
          throw new ApiError(`runs list page ${page} malformed: missing workflow_runs array`);
        }
        // Minimum schema: the identification + state fields every real
        // run record carries. A record missing them cannot even be
        // proven target or non-target — malformed response, not
        // "a run we can ignore".
        const badEntry = data.workflow_runs.some(
          (r) =>
            r == null ||
            typeof r !== "object" ||
            typeof r.id !== "number" ||
            typeof r.head_sha !== "string" ||
            typeof r.head_branch !== "string" ||
            typeof r.event !== "string" ||
            typeof r.status !== "string",
        );
        if (badEntry) {
          throw new ApiError(`runs list page ${page} malformed: run entry missing required identification/state fields`);
        }
        const pageRuns = data.workflow_runs;
        all.push(...pageRuns);
        if (pageRuns.length < perPage) return { runs: all, truncated: false };
      }
      return { runs: all, truncated: true };
    },
    // POST dispatches returns 204 with no body; a non-2xx throws.
    async dispatch({ workflow, ref, inputs }) {
      await request("POST", `/repos/${repo}/actions/workflows/${workflow}/dispatches`, { ref, inputs });
    },
    // Resolve a consumer repo's branch head via anonymous `git ls-remote`
    // (Issue #294 tuple mode). Same access boundary as consumer-verify's
    // anonymous probe: public repos resolve without credentials; a private
    // or unreachable repo fails -> ApiError -> `error` outcome, never a
    // guessed SHA.
    async getRepoRefSha(targetRepo, ref) {
      const url = `https://github.com/${targetRepo}.git`;
      let stdout;
      try {
        ({ stdout } = await execFileAsync("git", ["ls-remote", url, `refs/heads/${ref}`], { timeout: 30_000 }));
      } catch (e) {
        throw new ApiError(`git ls-remote ${targetRepo} refs/heads/${ref} failed: ${e.message}`);
      }
      return parseLsRemote(stdout, ref);
    },
    // List a run's artifacts (paginated). The `artifacts` array is
    // required — a 200 body without it is malformed, not "no artifacts".
    async listRunArtifacts(runId) {
      const all = [];
      for (let page = 1; page <= 10; page++) {
        const data = await request(
          "GET",
          `/repos/${repo}/actions/runs/${runId}/artifacts?per_page=100&page=${page}`,
        );
        if (data == null || !Array.isArray(data.artifacts)) {
          throw new ApiError(`run ${runId} artifacts page ${page} malformed: missing artifacts array`);
        }
        const badEntry = data.artifacts.some(
          (a) => a == null || typeof a !== "object" || typeof a.id !== "number" || typeof a.name !== "string",
        );
        if (badEntry) {
          throw new ApiError(`run ${runId} artifacts page ${page} malformed: artifact entry missing id/name`);
        }
        all.push(...data.artifacts);
        if (data.artifacts.length < 100) return all;
      }
      return all;
    },
    // Download an artifact zip. The endpoint answers 302 to a short-lived
    // signed URL; the redirect is followed MANUALLY so the GitHub token is
    // never forwarded to the storage host.
    async downloadArtifactZip(artifactId) {
      const headers = {
        Authorization: `Bearer ${token}`,
        Accept: "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
        "User-Agent": "ruviform-daily-ci",
      };
      let res;
      try {
        res = await fetchImpl(`${base}/repos/${repo}/actions/artifacts/${artifactId}/zip`, {
          method: "GET",
          redirect: "manual",
          headers,
        });
      } catch (e) {
        throw new ApiError(`github api GET artifact ${artifactId} zip failed: ${e.message}`);
      }
      if (res.status >= 300 && res.status < 400) {
        const location = res.headers?.get?.("location");
        if (typeof location !== "string" || location === "") {
          throw new ApiError(`artifact ${artifactId} zip redirect without location`);
        }
        let zip;
        try {
          zip = await fetchImpl(location, { method: "GET" });
        } catch (e) {
          throw new ApiError(`artifact ${artifactId} zip download failed: ${e.message}`);
        }
        if (!zip.ok) throw new ApiError(`artifact ${artifactId} zip download -> ${zip.status}`, zip.status);
        return Buffer.from(await zip.arrayBuffer());
      }
      if (!res.ok) {
        const text = await res.text().catch(() => "");
        throw new ApiError(`github api GET artifact ${artifactId} zip -> ${res.status}`, res.status, text.slice(0, 500));
      }
      return Buffer.from(await res.arrayBuffer());
    },
    // Compose artifact listing + zip download + JSON extraction into the
    // per-consumer evidence map consumed by classifyRunEvidence.
    async getRunEvidence(runId, { attempt, consumers }) {
      return recoverRunEvidence({
        listRunArtifacts: (id) => client.listRunArtifacts(id),
        downloadArtifactZip: (id) => client.downloadArtifactZip(id),
        runId,
        attempt,
        consumers,
      });
    },
  };
  return client;
}

// Deterministic offline client driven by a fixture JSON:
//   { "refReads":  [sha, ...],           // consumed per getRefSha; repeats last
//     "listCalls": [[[run...]], ...],    // one entry per listRuns call; each is
//                                        // an array of pages; repeats last
//     "dispatchCalls": [{"error": {...}} | {}],
//     "dispatchError": {...},            // shorthand for a single failing call
//     "refError": {...}, "listError": {...},
//     "truncated": false,
//     // tuple mode:
//     "repoRefReads": {"owner/repo": [sha, ...]},  // per getRepoRefSha; repeats last
//     "repoRefErrors": {"owner/repo": {"message": "..."}},
//     "runArtifacts": {"<runId>": {"<attempt>": {"<consumer>":
//        <evidence JSON object> | "__missing__" | "__expired__" |
//        "__malformed__" | "__badzip__"}}}} }
export function createFixtureClient(fixture) {
  const calls = { refReads: 0, listCalls: 0, dispatches: [], repoRefReads: {}, artifactDownloads: [] };
  const refReads = fixture.refReads?.length ? fixture.refReads : [fixture.mainSha ?? null];
  const listCalls = fixture.listCalls?.length ? fixture.listCalls : [fixture.runs ? [fixture.runs] : [[]]];
  const dispatchCalls = fixture.dispatchCalls ?? (fixture.dispatchError ? [{ error: fixture.dispatchError }] : []);
  const repoRefReads = fixture.repoRefReads ?? {};
  const repoRefErrors = fixture.repoRefErrors ?? {};
  const repoRefSeqs = new Map(
    Object.entries(repoRefReads).map(([repo, seq]) => [repo, Array.isArray(seq) ? seq : [seq]]),
  );
  const repoRefCounts = new Map();
  // artifactId -> { consumer, attempt, runId, value } — populated lazily by
  // listRunArtifacts so downloadArtifactZip can serve real zip bytes built
  // by buildStoredZip (the production unzip path stays under test).
  const artifactStore = new Map();
  let nextArtifactId = 900_000;

  const client = {
    calls,
    async getRefSha() {
      if (fixture.refError) throw new ApiError(`fixture refError: ${fixture.refError.message ?? "injected"}`);
      const sha = refReads[Math.min(calls.refReads++, refReads.length - 1)];
      if (typeof sha !== "string" || !SHA_RE.test(sha)) {
        throw new ApiError(`fixture refRead ${calls.refReads - 1} is not a 40-hex sha`);
      }
      return sha;
    },
    async listRuns() {
      if (fixture.listError) throw new ApiError(`fixture listError: ${fixture.listError.message ?? "injected"}`);
      const group = listCalls[Math.min(calls.listCalls++, listCalls.length - 1)];
      return { runs: group.flat(), truncated: fixture.truncated === true };
    },
    async dispatch({ ref, inputs }) {
      const entry = dispatchCalls.length
        ? dispatchCalls[Math.min(calls.dispatches.length, dispatchCalls.length - 1)]
        : null;
      calls.dispatches.push({ ref, inputs });
      if (entry?.error) throw new ApiError(`fixture dispatchError: ${entry.error.message ?? "injected"}`, entry.error.status ?? null);
    },
    async getRepoRefSha(repo, ref) {
      if (repoRefErrors[repo]) {
        throw new ApiError(`fixture repoRefError ${repo}: ${repoRefErrors[repo].message ?? "injected"}`);
      }
      const seq = repoRefSeqs.get(repo);
      if (seq == null) throw new ApiError(`fixture has no repoRefReads for ${repo}`);
      const n = repoRefCounts.get(repo) ?? 0;
      repoRefCounts.set(repo, n + 1);
      calls.repoRefReads[repo] = (calls.repoRefReads[repo] ?? 0) + 1;
      const sha = seq[Math.min(n, seq.length - 1)];
      if (typeof sha !== "string" || !SHA_RE.test(sha)) {
        throw new ApiError(`fixture repoRefRead ${repo}[${n}] is not a 40-hex sha`);
      }
      if (ref !== "main") throw new ApiError(`fixture getRepoRefSha only models refs/heads/main (got ${ref})`);
      return sha;
    },
    async listRunArtifacts(runId) {
      const perAttempt = fixture.runArtifacts?.[String(runId)] ?? {};
      const out = [];
      for (const [attempt, byConsumer] of Object.entries(perAttempt)) {
        for (const [consumer, value] of Object.entries(byConsumer ?? {})) {
          if (value === "__missing__") continue;
          const id = nextArtifactId++;
          artifactStore.set(id, { runId, attempt, consumer, value });
          out.push({
            id,
            name: `consumer-evidence-${consumer}-attempt-${attempt}`,
            expired: value === "__expired__",
            size_in_bytes: 128,
          });
        }
      }
      return out;
    },
    async downloadArtifactZip(artifactId) {
      calls.artifactDownloads.push(artifactId);
      const entry = artifactStore.get(artifactId);
      if (entry == null) throw new ApiError(`fixture artifact ${artifactId} unknown`);
      if (entry.value === "__badzip__") return Buffer.from("this is not a zip archive");
      const text = entry.value === "__malformed__" ? "{not valid json" : JSON.stringify(entry.value);
      return buildStoredZip([{ name: `consumer-evidence-${entry.consumer}.json`, data: Buffer.from(text, "utf8") }]);
    },
    async getRunEvidence(runId, { attempt, consumers }) {
      return recoverRunEvidence({
        listRunArtifacts: (id) => client.listRunArtifacts(id),
        downloadArtifactZip: (id) => client.downloadArtifactZip(id),
        runId,
        attempt,
        consumers,
      });
    },
  };
  return client;
}

// ---------------------------------------------------------------------------
// Orchestration
// ---------------------------------------------------------------------------

function attemptRow(r) {
  return {
    id: r.id ?? null,
    event: r.event ?? null,
    status: r.status ?? null,
    conclusion: r.conclusion ?? null,
    runAttempt: r.run_attempt ?? null,
    headSha: r.head_sha ?? null,
    createdAt: r.created_at ?? null,
    actor: actorOf(r),
    url: r.html_url ?? null,
  };
}

// Executes the full scheduler decision flow and returns the outcome plus
// the report object. Never throws for expected conditions — API/fixture
// failures become the `error` outcome.
export async function execute(cfg) {
  const warnings = [];
  const report = {
    schemaVersion: 1,
    kind: "daily-ci-decision",
    repository: cfg.repo ?? null,
    workflow: cfg.workflow,
    mainRef: cfg.mainRef,
    observedShas: [],
    decisionSha: null,
    outcome: null,
    reason: null,
    latestVerified: null,
    attemptsOnDecisionSha: [],
    autoDispatchCount: null,
    maxAutoDispatches: cfg.maxAutoDispatches,
    historyTruncated: false,
    dispatch: null,
    warnings,
    generatedAt: cfg.now(),
  };
  const done = (outcome, reason, extra = {}) => {
    report.outcome = outcome;
    report.reason = reason;
    return { outcome, exitCode: 0, report, warnings, ...extra };
  };

  try {
    let sha = await cfg.client.getRefSha(cfg.mainRef);
    report.observedShas.push(sha);
    let listed = await cfg.client.listRuns({ workflow: cfg.workflow, branch: cfg.mainRef, maxPages: cfg.maxPages });
    let d = decide({
      mainSha: sha,
      runs: listed.runs,
      mainRef: cfg.mainRef,
      autoActors: cfg.autoActors,
      maxAutoDispatches: cfg.maxAutoDispatches,
    });

    const recordDecision = () => {
      report.decisionSha = sha;
      if (d.latestVerified != null) {
        report.latestVerified = {
          headSha: d.latestVerified.head_sha,
          updatedAt: d.latestVerified.updated_at ?? null,
          url: d.latestVerified.html_url ?? null,
        };
      }
      report.attemptsOnDecisionSha = (d.attempts ?? []).slice(0, MAX_ATTEMPTS_IN_REPORT).map(attemptRow);
      report.autoDispatchCount = d.autoDispatches.length;
      report.unidentifiableCount = (d.unidentifiable ?? []).length;
    };
    // Truncated history is fail-closed on BOTH sides: unseen older pages
    // could hide auto dispatches (retry-budget undercount) or a rerun on
    // this SHA whose recent activity would contest a success claim. No
    // outcome that asserts "verified" or fires a dispatch is safe on
    // incomplete data — hold, warn, and let ops widen DAILY_MAX_PAGES.
    const truncatedHold = () => {
      report.historyTruncated = true;
      recordDecision();
      warnings.push(`run history truncated at ${cfg.maxPages} page(s); cannot certify the verification state — widen DAILY_MAX_PAGES or re-run manually`);
      return done("history-truncated", `run history truncated at ${cfg.maxPages} page(s); held underlying decision "${d.decision}"`);
    };
    report.historyTruncated = listed.truncated;
    if (listed.truncated) return truncatedHold();

    // Re-check the ref right before dispatching so a moving main never
    // gets a run stamped with a stale expected_sha. Bounded: if main keeps
    // moving we defer to the next run instead of chasing it.
    let rechecks = 0;
    while (d.decision === "dispatch") {
      const current = await cfg.client.getRefSha(cfg.mainRef);
      if (current === sha) break;
      if (report.observedShas[report.observedShas.length - 1] !== current) report.observedShas.push(current);
      if (++rechecks > cfg.maxRechecks) {
        warnings.push(`main moved ${rechecks} time(s) during decision; deferring to next run`);
        return done("main-moved", `main moved past ${sha.slice(0, 12)} during decision`);
      }
      sha = current;
      listed = await cfg.client.listRuns({ workflow: cfg.workflow, branch: cfg.mainRef, maxPages: cfg.maxPages });
      d = decide({
        mainSha: sha,
        runs: listed.runs,
        mainRef: cfg.mainRef,
        autoActors: cfg.autoActors,
        maxAutoDispatches: cfg.maxAutoDispatches,
      });
      if (listed.truncated) return truncatedHold();
    }

    recordDecision();
    if (d.decision !== "dispatch") {
      if (d.decision === "budget-exhausted" || d.decision === "hold-unknown") warnings.push(d.reason);
      return done(d.decision, d.reason);
    }
    if (cfg.dryRun) {
      return done("dry-run", `would dispatch ${cfg.workflow} on ${cfg.mainRef} with expected_sha=${sha} (${d.reason})`);
    }

    // Dispatch, then confirm a new workflow_dispatch run actually appeared.
    // The dispatch POST returns 204 with no run id, so the new run is
    // identified as a workflow_dispatch run on mainRef whose id was not in
    // the pre-dispatch set. A simultaneous manual dispatch on the same SHA
    // is indistinguishable — harmless: it is still "a dispatch for this
    // SHA" and its landed head_sha is reported.
    const preDispatchIds = new Set(
      (listed.runs ?? [])
        .filter((r) => r.event === "workflow_dispatch" && r.head_branch === cfg.mainRef)
        .map((r) => r.id),
    );
    const requestedAt = cfg.now();
    await cfg.client.dispatch({ workflow: cfg.workflow, ref: cfg.mainRef, inputs: { expected_sha: sha } });
    report.dispatch = { requestedAt, expectedSha: sha, ref: cfg.mainRef, confirmedRun: null };

    const maxPolls = Math.max(1, Math.ceil(cfg.confirmTimeoutMs / cfg.confirmIntervalMs));
    let confirmed = null;
    for (let poll = 0; poll < maxPolls && confirmed == null; poll++) {
      await cfg.sleep(cfg.confirmIntervalMs);
      const relisted = await cfg.client.listRuns({ workflow: cfg.workflow, branch: cfg.mainRef, maxPages: cfg.maxPages });
      confirmed =
        relisted.runs.find(
          (r) => r.event === "workflow_dispatch" && r.head_branch === cfg.mainRef && !preDispatchIds.has(r.id),
        ) ?? null;
    }

    if (confirmed == null) {
      warnings.push(`dispatch accepted but no new run observed within ${cfg.confirmTimeoutMs}ms (eventual consistency); next scheduled run re-evaluates`);
      return done("dispatched-unconfirmed", `dispatched expected_sha=${sha.slice(0, 12)} but run not observed yet`);
    }
    report.dispatch.confirmedRun = attemptRow(confirmed);
    if (confirmed.head_sha !== sha) {
      warnings.push(
        `dispatched run ${confirmed.id} landed on ${confirmed.head_sha} not ${sha}; ci-summary's expected_sha check fails that run — the expected SHA is NOT verified`,
      );
      return done(
        "dispatch-landed-mismatch",
        `dispatched run landed on ${confirmed.head_sha.slice(0, 12)} != expected ${sha.slice(0, 12)}`,
      );
    }
    return done("dispatch-confirmed", `dispatched run ${confirmed.id} on ${sha.slice(0, 12)}`);
  } catch (e) {
    report.outcome = "error";
    report.reason = e.message;
    report.error = { message: e.message, status: e.status ?? null, body: e.body ?? null };
    return { outcome: "error", exitCode: 1, report, warnings };
  }
}

// ---------------------------------------------------------------------------
// Tuple-mode orchestration (Issue #294)
// ---------------------------------------------------------------------------

// Resolve the full candidate tuple: ruviform main via the repo ref API,
// each consumer's main via anonymous `git ls-remote`. Any resolution
// failure throws -> `error` outcome; a tuple is never guessed.
async function resolveTupleCandidate(client, cfg, observedShas) {
  const ruviformSha = await client.getRefSha(cfg.mainRef);
  observedShas.push({ repo: cfg.repo ?? null, ref: cfg.mainRef, sha: ruviformSha, role: "ruviform" });
  const consumers = [];
  for (const c of cfg.consumers) {
    const sha = await client.getRepoRefSha(c.repo, "main");
    observedShas.push({ repo: c.repo, ref: "main", sha, role: c.name });
    consumers.push({ name: c.name, repo: c.repo, sha });
  }
  return { ruviformSha, consumers, contract: cfg.tupleContract };
}

function sameTuple(a, b) {
  return (
    a.ruviformSha === b.ruviformSha &&
    a.contract === b.contract &&
    a.consumers.length === b.consumers.length &&
    a.consumers.every((c, i) => c.name === b.consumers[i].name && c.repo === b.consumers[i].repo && c.sha === b.consumers[i].sha)
  );
}

// List + scope + evidence-scan the workflow run history for `candidate`.
// Returns { listed, inFlight, covering, unverifiable } or a hold marker
// ({ truncated } / { evidenceTruncated, terminalCount }).
async function scanTupleHistory(client, cfg, candidate) {
  const listed = await client.listRuns({ workflow: cfg.workflow, branch: cfg.mainRef, maxPages: cfg.maxPages });
  if (listed.truncated) return { truncated: true, listed };
  const scoped = (listed.runs ?? []).filter((r) => isFullCiRun(r, cfg.mainRef));
  const inFlight = scoped.filter((r) => classifyRun(r) === "in-flight");
  const terminal = scoped.filter((r) => classifyRun(r) !== "in-flight").sort(newestFirst);
  // Evidence recovery costs ~2 API calls per consumer per run — bound the
  // scan so a pathological history cannot turn the daily job into an
  // unbounded crawl. Exceeding the bound holds rather than guesses.
  if (terminal.length > cfg.maxEvidenceRuns) {
    return { evidenceTruncated: true, terminalCount: terminal.length, listed, inFlight, covering: [], unverifiable: [] };
  }
  const covering = [];
  const unverifiable = [];
  for (const run of terminal) {
    const evidences = await client.getRunEvidence(run.id, {
      attempt: run.run_attempt,
      consumers: cfg.consumers,
    });
    const cls = classifyRunEvidence({
      run,
      evidences,
      candidate,
      contract: cfg.tupleContract,
    });
    if (cls.kind === "covering") covering.push({ run, consumerStatus: cls.consumerStatus });
    else if (cls.kind === "unverifiable") unverifiable.push({ run });
    // non-target: proven different tuple — contributes nothing.
  }
  return { listed, inFlight, covering, unverifiable };
}

// Executes the tuple-mode scheduler flow. Same outcome/exit-code contract
// as execute(): 0 for every completed decision, 1 on `error`.
export async function executeTuple(cfg) {
  const warnings = [];
  const report = {
    schemaVersion: 1,
    kind: "daily-ci-tuple-decision",
    mode: "tuple",
    repository: cfg.repo ?? null,
    workflow: cfg.workflow,
    mainRef: cfg.mainRef,
    tuple: null,
    observedShas: [],
    decisionTuple: null,
    outcome: null,
    reason: null,
    latestVerified: null,
    coveringAttemptsOnDecisionTuple: [],
    unverifiableCount: null,
    autoOnTuple: null,
    maxAutoDispatches: cfg.maxAutoDispatches,
    historyTruncated: false,
    evidenceTruncated: false,
    dispatch: null,
    warnings,
    generatedAt: cfg.now(),
  };
  const done = (outcome, reason, extra = {}) => {
    report.outcome = outcome;
    report.reason = reason;
    return { outcome, exitCode: 0, report, warnings, ...extra };
  };

  try {
    let candidate = await resolveTupleCandidate(cfg.client, cfg, report.observedShas);
    report.tuple = candidate;
    let scan = await scanTupleHistory(cfg.client, cfg, candidate);
    let d = null;

    const recordDecision = () => {
      report.decisionTuple = candidate;
      report.coveringAttemptsOnDecisionTuple = (d?.covering ?? scan.covering ?? [])
        .slice(0, MAX_ATTEMPTS_IN_REPORT)
        .map((e) => ({ ...attemptRow(e.run), consumerStatus: e.consumerStatus ?? null }));
      report.unverifiableCount = (scan.unverifiable ?? []).length;
      report.autoOnTuple = d?.autoOnTuple ?? null;
      if (d?.latestVerified != null) {
        report.latestVerified = {
          runId: d.latestVerified.run.id ?? null,
          headSha: d.latestVerified.run.head_sha ?? null,
          updatedAt: d.latestVerified.run.updated_at ?? null,
          url: d.latestVerified.run.html_url ?? null,
          consumerStatus: d.latestVerified.consumerStatus ?? null,
        };
      }
    };
    const truncatedHold = () => {
      report.historyTruncated = true;
      recordDecision();
      warnings.push(`run history truncated at ${cfg.maxPages} page(s); cannot certify the tuple verification state — widen DAILY_MAX_PAGES or re-run manually`);
      return done("history-truncated", `run history truncated at ${cfg.maxPages} page(s)`);
    };
    const evidenceHold = () => {
      report.evidenceTruncated = true;
      recordDecision();
      warnings.push(`terminal run count ${scan.terminalCount} exceeds DAILY_MAX_EVIDENCE_RUNS=${cfg.maxEvidenceRuns}; evidence scan bound hit — cannot certify the tuple verification state`);
      return done("evidence-truncated", `terminal runs ${scan.terminalCount} > evidence scan bound ${cfg.maxEvidenceRuns}`);
    };

    if (scan.truncated) return truncatedHold();
    if (scan.evidenceTruncated) return evidenceHold();
    d = decideTuple({
      candidate,
      covering: scan.covering,
      inFlight: scan.inFlight,
      unverifiable: scan.unverifiable,
      autoActors: cfg.autoActors,
      maxAutoDispatches: cfg.maxAutoDispatches,
    });

    // Re-resolve the whole tuple right before dispatching so a moving
    // consumer main never gets a run stamped with a stale expected_sha.
    // Bounded like the single-SHA path: a tuple that keeps moving defers
    // to the next run instead of chasing it.
    let rechecks = 0;
    while (d.decision === "dispatch") {
      const next = await resolveTupleCandidate(cfg.client, cfg, report.observedShas);
      if (sameTuple(next, candidate)) break;
      if (++rechecks > cfg.maxRechecks) {
        warnings.push(`candidate tuple moved ${rechecks} time(s) during decision; deferring to next run`);
        return done("candidate-moved", `candidate tuple moved during decision (recheck bound ${cfg.maxRechecks} exceeded)`);
      }
      candidate = next;
      report.tuple = candidate;
      scan = await scanTupleHistory(cfg.client, cfg, candidate);
      if (scan.truncated) return truncatedHold();
      if (scan.evidenceTruncated) return evidenceHold();
      d = decideTuple({
        candidate,
        covering: scan.covering,
        inFlight: scan.inFlight,
        unverifiable: scan.unverifiable,
        autoActors: cfg.autoActors,
        maxAutoDispatches: cfg.maxAutoDispatches,
      });
    }

    recordDecision();
    if (d.decision !== "dispatch") {
      if (d.decision === "budget-exhausted" || d.decision === "hold-unknown" || d.decision === "evidence-truncated") {
        warnings.push(d.reason);
      }
      return done(d.decision, d.reason);
    }
    if (cfg.dryRun) {
      return done("dry-run", `would dispatch ${cfg.workflow} on ${cfg.mainRef} for tuple ruviform=${candidate.ruviformSha.slice(0, 12)} (${d.reason})`);
    }

    // Dispatch with the full tuple inputs consumer-verify.yml expects:
    // ruviform_rev pins the candidate; *_ref selects the consumer branch;
    // *_expected_sha makes the lane fail-closed on tuple_mismatch.
    const preDispatchIds = new Set(
      (scan.listed?.runs ?? [])
        .filter((r) => r.event === "workflow_dispatch" && r.head_branch === cfg.mainRef)
        .map((r) => r.id),
    );
    const inputs = { ruviform_rev: candidate.ruviformSha };
    for (const c of candidate.consumers) {
      inputs[`${c.name}_ref`] = "main";
      inputs[`${c.name}_expected_sha`] = c.sha;
    }
    const requestedAt = cfg.now();
    await cfg.client.dispatch({ workflow: cfg.workflow, ref: cfg.mainRef, inputs });
    report.dispatch = { requestedAt, tuple: candidate, ref: cfg.mainRef, inputs, confirmedRun: null };

    const maxPolls = Math.max(1, Math.ceil(cfg.confirmTimeoutMs / cfg.confirmIntervalMs));
    let confirmed = null;
    for (let poll = 0; poll < maxPolls && confirmed == null; poll++) {
      await cfg.sleep(cfg.confirmIntervalMs);
      const relisted = await cfg.client.listRuns({ workflow: cfg.workflow, branch: cfg.mainRef, maxPages: cfg.maxPages });
      confirmed =
        relisted.runs.find(
          (r) => r.event === "workflow_dispatch" && r.head_branch === cfg.mainRef && !preDispatchIds.has(r.id),
        ) ?? null;
    }

    if (confirmed == null) {
      warnings.push(`dispatch accepted but no new run observed within ${cfg.confirmTimeoutMs}ms (eventual consistency); next scheduled run re-evaluates`);
      return done("dispatched-unconfirmed", `dispatched tuple ruviform=${candidate.ruviformSha.slice(0, 12)} but run not observed yet`);
    }
    report.dispatch.confirmedRun = attemptRow(confirmed);
    // head_sha is the dispatch-time main HEAD; the tuple under test is
    // pinned by the ruviform_rev INPUT, so a mismatch means main moved
    // between resolution and landing — the evidence tuple stays valid,
    // but the race is reported honestly.
    if (confirmed.head_sha !== candidate.ruviformSha) {
      warnings.push(
        `dispatched run ${confirmed.id} landed on ${confirmed.head_sha} not ${candidate.ruviformSha}; the run's ruviform_rev input still pins the decided tuple — reported as landed-mismatch`,
      );
      return done(
        "dispatch-landed-mismatch",
        `dispatched run landed on ${confirmed.head_sha.slice(0, 12)} != decided ruviform ${candidate.ruviformSha.slice(0, 12)}`,
      );
    }
    return done("dispatch-confirmed", `dispatched run ${confirmed.id} for tuple ruviform=${candidate.ruviformSha.slice(0, 12)}`);
  } catch (e) {
    report.outcome = "error";
    report.reason = e.message;
    report.error = { message: e.message, status: e.status ?? null, body: e.body ?? null };
    return { outcome: "error", exitCode: 1, report, warnings };
  }
}

// ---------------------------------------------------------------------------
// Reporting
// ---------------------------------------------------------------------------

function fmtSha(sha) {
  return sha == null ? "-" : `\`${sha.slice(0, 12)}\``;
}

export function markdownSummary(report) {
  const lines = [
    `## daily-ci${report.mode === "tuple" ? "-tuple" : ""} outcome: ${report.outcome}`,
    "",
    `- reason: \`${report.reason ?? "-"}\``,
  ];
  if (report.mode === "tuple" && report.tuple != null) {
    const t = report.tuple;
    lines.push(
      `- tuple: ruviform ${fmtSha(t.ruviformSha)} | ${t.consumers.map((c) => `${c.name} ${fmtSha(c.sha)}`).join(" | ")} | contract \`${t.contract}\``,
      `- covering attempts on decision tuple: ${(report.coveringAttemptsOnDecisionTuple ?? []).length}  unverifiable runs: ${report.unverifiableCount ?? "-"}  auto on tuple: ${report.autoOnTuple ?? "-"} / ${report.maxAutoDispatches}`,
      `- latest verified: ${report.latestVerified == null ? "-" : `run ${report.latestVerified.runId} @ ${report.latestVerified.updatedAt ?? "-"}`}${report.latestVerified?.url ? `  (${report.latestVerified.url})` : ""}`,
    );
  } else {
    lines.push(
      `- decision sha: ${fmtSha(report.decisionSha)}  observed: ${report.observedShas.map((s) => `\`${s.slice(0, 12)}\``).join(", ") || "-"}`,
      `- latest verified: ${fmtSha(report.latestVerified?.headSha)} @ ${report.latestVerified?.updatedAt ?? "-"}${report.latestVerified?.url ? `  (${report.latestVerified.url})` : ""}`,
      `- auto dispatches on decision sha: ${report.autoDispatchCount ?? "-"} / ${report.maxAutoDispatches}`,
    );
  }
  lines.push(`- history truncated: ${report.historyTruncated}`);
  if (report.mode === "tuple") lines.push(`- evidence scan truncated: ${report.evidenceTruncated}`);
  if (report.dispatch != null) {
    const c = report.dispatch.confirmedRun;
    const expected = report.dispatch.expectedSha ?? report.dispatch.tuple?.ruviformSha;
    lines.push(
      `- dispatch: expected ${fmtSha(expected)} on \`${report.dispatch.ref}\` @ ${report.dispatch.requestedAt}`,
      c == null
        ? `- confirmed run: not observed within poll window`
        : `- confirmed run: [${c.id}](${c.url})  landed ${fmtSha(c.headSha)}  status \`${c.status}\`/\`${c.conclusion}\`  attempt ${c.runAttempt ?? "-"}`,
    );
    if (report.dispatch.inputs != null) {
      lines.push(`- dispatch inputs: \`${JSON.stringify(report.dispatch.inputs)}\``);
    }
  }
  const attempts = report.attemptsOnDecisionSha ?? report.coveringAttemptsOnDecisionTuple ?? [];
  if (attempts.length > 0) {
    lines.push(
      "",
      "| run | event | status | conclusion | attempt | created_at | actor | consumer status | url |",
      "| --- | --- | --- | --- | --- | --- | --- | --- | --- |",
      ...attempts.map((r) => {
        const cs = r.consumerStatus == null ? "-" : Object.entries(r.consumerStatus).map(([k, v]) => `${k}=${v}`).join(",");
        return `| ${r.id ?? "-"} | ${r.event ?? "-"} | ${r.status ?? "-"} | ${r.conclusion ?? "-"} | ${r.runAttempt ?? "-"} | ${r.createdAt ?? "-"} | ${r.actor ?? "-"} | ${cs} | ${r.url ?? "-"} |`;
      }),
    );
  }
  if (report.warnings.length > 0) {
    lines.push("", "### warnings", "", ...report.warnings.map((w) => `- ${w}`));
  }
  lines.push("");
  return lines.join("\n");
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

function parsePositiveInt(value, flag) {
  const n = Number.parseInt(value, 10);
  if (!Number.isFinite(n) || n < 0) throw new Error(`invalid value for ${flag}: ${value}`);
  return n;
}

const REPO_SPEC_RE = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/;
const CONSUMER_NAME_RE = /^[a-z][a-z0-9-]*$/;

// Parse `name=owner/repo[,name=owner/repo]` consumer specs. Order is
// preserved — it defines evidence/artifact and dispatch-input ordering.
export function parseConsumerRepos(spec) {
  const out = [];
  for (const part of String(spec ?? "").split(",")) {
    const trimmed = part.trim();
    if (trimmed === "") continue;
    const eq = trimmed.indexOf("=");
    if (eq <= 0) throw new Error(`invalid --consumer-repo entry "${trimmed}" (expected name=owner/repo)`);
    const name = trimmed.slice(0, eq);
    const repo = trimmed.slice(eq + 1);
    if (!CONSUMER_NAME_RE.test(name)) throw new Error(`invalid consumer name "${name}"`);
    if (!REPO_SPEC_RE.test(repo)) throw new Error(`invalid consumer repo "${repo}" (expected owner/repo)`);
    out.push({ name, repo });
  }
  return out;
}

export function resolveConfig(argv = process.argv, env = process.env) {
  const cfg = {
    repo: env.GITHUB_REPOSITORY ?? null,
    token: env.GITHUB_TOKEN ?? null,
    apiBase: env.DAILY_API_BASE ?? "https://api.github.com",
    workflow: env.DAILY_TARGET_WORKFLOW ?? "ci.yml",
    mainRef: env.DAILY_MAIN_REF ?? "main",
    mode: env.DAILY_MODE ?? "single",
    consumers: parseConsumerRepos(
      env.DAILY_CONSUMER_REPOS ?? "audiolens=scarecrow173/audio-lens,synvelis=scarecrow173/synvelis",
    ),
    tupleContract: env.DAILY_TUPLE_CONTRACT ?? "consumer-verify-tuple/v1",
    maxAutoDispatches: 2,
    maxPages: 10,
    maxRechecks: 2,
    maxEvidenceRuns: 60,
    autoActors: (env.DAILY_AUTO_ACTORS ?? "github-actions[bot]").split(",").map((s) => s.trim()).filter(Boolean),
    confirmTimeoutMs: 120_000,
    confirmIntervalMs: 15_000,
    reportPath: env.DAILY_REPORT ?? "daily-ci-report.json",
    summaryPath: env.GITHUB_STEP_SUMMARY ?? null,
    fixture: null,
    dryRun: env.DRY_RUN === "true" || env.DRY_RUN === "1",
  };
  if (env.DAILY_MAX_AUTO_DISPATCHES != null) cfg.maxAutoDispatches = parsePositiveInt(env.DAILY_MAX_AUTO_DISPATCHES, "DAILY_MAX_AUTO_DISPATCHES");
  if (env.DAILY_MAX_PAGES != null) cfg.maxPages = parsePositiveInt(env.DAILY_MAX_PAGES, "DAILY_MAX_PAGES");
  if (env.DAILY_MAX_RECHECKS != null) cfg.maxRechecks = parsePositiveInt(env.DAILY_MAX_RECHECKS, "DAILY_MAX_RECHECKS");
  if (env.DAILY_MAX_EVIDENCE_RUNS != null) cfg.maxEvidenceRuns = parsePositiveInt(env.DAILY_MAX_EVIDENCE_RUNS, "DAILY_MAX_EVIDENCE_RUNS");
  if (env.DAILY_CONFIRM_TIMEOUT_MS != null) cfg.confirmTimeoutMs = parsePositiveInt(env.DAILY_CONFIRM_TIMEOUT_MS, "DAILY_CONFIRM_TIMEOUT_MS");
  if (env.DAILY_CONFIRM_INTERVAL_MS != null) cfg.confirmIntervalMs = parsePositiveInt(env.DAILY_CONFIRM_INTERVAL_MS, "DAILY_CONFIRM_INTERVAL_MS");

  let consumerFlagSeen = false;
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    const next = () => argv[++i];
    if (a === "--workflow") cfg.workflow = next();
    else if (a === "--main-ref") cfg.mainRef = next();
    else if (a === "--api-base") cfg.apiBase = next();
    else if (a === "--repo") cfg.repo = next();
    else if (a === "--mode") cfg.mode = next();
    else if (a === "--consumer-repo") {
      // The flag replaces the env-derived list on first use — appending
      // would silently union env defaults with explicit flags.
      if (!consumerFlagSeen) {
        cfg.consumers = [];
        consumerFlagSeen = true;
      }
      cfg.consumers.push(...parseConsumerRepos(next()));
    }
    else if (a === "--tuple-contract") cfg.tupleContract = next();
    else if (a === "--max-auto-dispatches") cfg.maxAutoDispatches = parsePositiveInt(next(), a);
    else if (a === "--max-pages") cfg.maxPages = parsePositiveInt(next(), a);
    else if (a === "--max-rechecks") cfg.maxRechecks = parsePositiveInt(next(), a);
    else if (a === "--max-evidence-runs") cfg.maxEvidenceRuns = parsePositiveInt(next(), a);
    else if (a === "--auto-actors") cfg.autoActors = next().split(",").map((s) => s.trim()).filter(Boolean);
    else if (a === "--confirm-timeout-ms") cfg.confirmTimeoutMs = parsePositiveInt(next(), a);
    else if (a === "--confirm-interval-ms") cfg.confirmIntervalMs = parsePositiveInt(next(), a);
    else if (a === "--report") cfg.reportPath = next();
    else if (a === "--summary") cfg.summaryPath = next();
    else if (a === "--fixture") cfg.fixture = next();
    else if (a === "--dry-run") cfg.dryRun = true;
    else if (a === "--help" || a === "-h") {
      return { help: true };
    } else {
      throw new Error(`unknown argument ${a}`);
    }
  }
  if (cfg.mode !== "single" && cfg.mode !== "tuple") {
    throw new Error(`invalid --mode "${cfg.mode}" (expected single|tuple)`);
  }
  if (cfg.mode === "tuple" && cfg.consumers.length === 0) {
    throw new Error("tuple mode requires at least one --consumer-repo/DAILY_CONSUMER_REPOS entry");
  }
  return cfg;
}

export async function main(argv = process.argv, env = process.env) {
  let cfg;
  try {
    cfg = resolveConfig(argv, env);
  } catch (e) {
    console.error(`error: ${e.message}`);
    process.exit(2);
    return;
  }
  if (cfg.help) {
    console.log(
      "Usage: node ci/daily/decide.mjs [--mode single|tuple] [--workflow ci.yml] [--main-ref main] [--api-base URL] [--repo owner/repo] [--consumer-repo name=owner/repo] [--tuple-contract V] [--max-auto-dispatches N] [--max-pages N] [--max-rechecks N] [--max-evidence-runs N] [--auto-actors a,b] [--confirm-timeout-ms N] [--confirm-interval-ms N] [--report FILE] [--summary FILE] [--fixture FILE] [--dry-run]",
    );
    process.exit(0);
    return;
  }

  let client;
  let sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  if (cfg.fixture != null) {
    let fixture;
    try {
      fixture = JSON.parse(readFileSync(cfg.fixture, "utf8"));
    } catch (e) {
      console.error(`error: cannot read fixture ${cfg.fixture}: ${e.message}`);
      process.exit(2);
      return;
    }
    client = createFixtureClient(fixture);
    sleep = async () => {}; // fixture polls are instant
  } else {
    if (!cfg.token) {
      console.error("error: GITHUB_TOKEN is required (or use --fixture/--dry-run for offline evaluation)");
      process.exit(2);
      return;
    }
    if (!cfg.repo) {
      console.error("error: GITHUB_REPOSITORY is required (or pass --repo owner/repo)");
      process.exit(2);
      return;
    }
    client = createGitHubClient({ token: cfg.token, repo: cfg.repo, apiBase: cfg.apiBase });
  }

  const exec = cfg.mode === "tuple" ? executeTuple : execute;
  const { exitCode, report, warnings } = await exec({
    client,
    repo: cfg.repo ?? "fixture/fixture",
    workflow: cfg.workflow,
    mainRef: cfg.mainRef,
    mode: cfg.mode,
    consumers: cfg.consumers,
    tupleContract: cfg.tupleContract,
    autoActors: cfg.autoActors,
    maxAutoDispatches: cfg.maxAutoDispatches,
    maxPages: cfg.maxPages,
    maxRechecks: cfg.maxRechecks,
    maxEvidenceRuns: cfg.maxEvidenceRuns,
    confirmTimeoutMs: cfg.confirmTimeoutMs,
    confirmIntervalMs: cfg.confirmIntervalMs,
    dryRun: cfg.dryRun,
    sleep,
    now: () => new Date().toISOString(),
  });

  for (const w of warnings) console.log(`::warning::${w}`);
  const table = markdownSummary(report);
  process.stdout.write(table + "\n");
  if (cfg.reportPath) writeFileSync(cfg.reportPath, JSON.stringify(report, null, 2) + "\n");
  const summaryPath = cfg.summaryPath;
  if (summaryPath) appendFileSync(summaryPath, table + "\n");
  process.exit(exitCode);
}

const isMain = process.argv[1] != null && import.meta.url === pathToFileURL(process.argv[1]).href;
if (isMain) {
  await main();
}
