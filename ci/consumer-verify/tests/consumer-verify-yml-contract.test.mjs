// Issue #294 contract test: .github/workflows/consumer-verify.yml must keep
// the candidate-tuple evidence contract structurally intact. Textual checks
// (stdlib only, no yaml dep) — actionlint covers YAML validity; this file
// pins the semantics that must not silently regress:
//
//   - dispatch-only trigger (scheduler cadence lives in #367, not here)
//   - tuple inputs: refs accept branch/tag/SHA; optional expected SHAs and
//     per-consumer extra-task inputs exist
//   - both verify jobs capture `git rev-parse HEAD` (resolved consumer SHA),
//     record lanes via evidence.mjs, emit the JSON artifact on EVERY
//     terminal path (`if: always()` on record + upload), and declare the
//     capability-absent `unsupported` fixed rows
//   - unified fetch+FETCH_HEAD checkout (branch/tag/SHA in one path)
//   - preflight self-tests the evaluator before any lane uses it
//   - artifact names are attempt-unique (`-attempt-${{ github.run_attempt }}`)
//   - concurrency serializes without cancelling in-flight evidence runs
//   - summary keeps the not-run-is-not-pass enforcement

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const YML = fileURLToPath(
  new URL("../../../.github/workflows/consumer-verify.yml", import.meta.url),
);
const src = readFileSync(YML, "utf8");

// Slice the text of one job block (from `  <name>:` at jobs-level indent to
// the next same-indent key or EOF) so per-job assertions can't drift.
function jobBlock(name) {
  const start = src.indexOf(`\n  ${name}:`);
  assert.notEqual(start, -1, `job ${name} missing`);
  const rest = src.slice(start + 1);
  const next = rest.slice(1).search(/\n  [a-zA-Z][\w-]*:\n/);
  return next < 0 ? rest : rest.slice(0, next + 1);
}

const audiolens = jobBlock("verify-audiolens");
const synvelis = jobBlock("verify-synvelis");
const preflight = jobBlock("preflight");
const summary = jobBlock("summary");

test("contract: trigger is workflow_dispatch only", () => {
  const on = src.slice(src.indexOf("\non:"), src.indexOf("\npermissions:"));
  assert.ok(/workflow_dispatch:/.test(on));
  for (const banned of ["pull_request", "push:", "schedule:", "workflow_run:"]) {
    assert.ok(!new RegExp(`\\n\\s{2}${banned}`).test(on), `banned trigger ${banned}`);
  }
});

test("contract: tuple + extra-task dispatch inputs exist", () => {
  for (const input of [
    "ruviform_rev",
    "audiolens_ref",
    "synvelis_ref",
    "audiolens_expected_sha",
    "synvelis_expected_sha",
    "audiolens_extra_tasks",
    "synvelis_extra_tasks",
  ]) {
    assert.ok(new RegExp(`\\n\\s{6}${input}:`).test(src), `missing input ${input}`);
  }
});

test("contract: concurrency serializes without cancelling in-flight runs", () => {
  assert.ok(/concurrency:/.test(src));
  assert.ok(/cancel-in-progress:\s*false/.test(src));
});

test("contract: preflight validates inputs and self-tests the evaluator", () => {
  assert.ok(/classify-ref/.test(preflight), "preflight lost classify-ref validation");
  assert.ok(/test-ci-consumer-verify/.test(preflight), "preflight lost evaluator self-test");
  // self-test must come before the credential probe (evaluator checked before use)
  const selfTest = preflight.indexOf("test-ci-consumer-verify");
  const probe = preflight.indexOf("id: probe");
  assert.ok(selfTest >= 0 && probe >= 0 && selfTest < probe);
});

// Step ids that intentionally never appear in a `rec` line: `gate` (its
// outcome IS the gate), `presence` (covered via the absent branch and via
// resolution's rev-parse fail row when the embedded clone fails),
// `resolution` (self-records its own fail/tuple_mismatch rows), `result`
// (is the recorder itself), `probe` (preflight-side). Everything else with
// an id MUST surface its outcome in the record step — an unrecorded failing
// step yields only `skipped` rows and launders `fail` into
// `unsupported`/`not_run` (the exact lie the contract exists to prevent).
const SELF_HANDLING = new Set(["gate", "probe", "presence", "resolution", "result"]);

for (const [name, job] of [
  ["verify-audiolens", audiolens],
  ["verify-synvelis", synvelis],
]) {
  test(`contract: ${name} captures resolved consumer SHA via rev-parse`, () => {
    assert.ok(/git -C "\$CONSUMER_DIR" rev-parse HEAD/.test(job), "rev-parse capture missing");
    assert.ok(/resolved_sha=/.test(job), "resolved_sha output missing");
    assert.ok(/resolved_sha: \$\{\{ steps\.resolution\.outputs\.resolved_sha \}\}/.test(job),
      "job outputs must expose resolved_sha from the resolution step");
  });

  test(`contract: ${name} uses unified fetch + detached FETCH_HEAD checkout`, () => {
    assert.ok(/git -C "\$CONSUMER_DIR" fetch -q --depth 1 origin "\$REQUESTED_REF"/.test(job));
    assert.ok(/git -C "\$CONSUMER_DIR" checkout -q --detach FETCH_HEAD/.test(job));
    assert.ok(!/--branch/.test(job), "branch-only clone would reject SHA refs");
  });

  test(`contract: ${name} emits evidence + artifact on every terminal path`, () => {
    assert.ok(/"\$EV" finalize/.test(job), "finalize call missing");
    // record step and upload step must both be if: always()
    const record = job.slice(job.indexOf("Record evidence"));
    assert.ok(/id: result\n\s+if: always\(\)/.test(record), "record step must run always()");
    const upload = job.slice(job.indexOf("Upload evidence"));
    assert.ok(/if: always\(\)/.test(upload.slice(0, 300)), "upload step must run always()");
    assert.ok(/if-no-files-found: error/.test(upload), "missing evidence file must fail the upload");
    // finalize failure must still leave a minimal evidence artifact
    assert.ok(/evidence_finalize_error/.test(record), "degraded-path minimal evidence missing");
  });

  test(`contract: ${name} resolution runs always() to capture clone failures`, () => {
    const res = job.slice(job.indexOf("id: resolution"));
    assert.ok(/if: always\(\) && steps\.gate\.outputs\.gate == 'run'/.test(res.slice(0, 300)),
      "resolution must run even when the clone step itself failed");
  });

  test(`contract: ${name} every recorded-capable step id maps to a lane row`, () => {
    const ids = [...job.matchAll(/^\s+id: (\w+)$/gm)].map((m) => m[1]);
    const record = job.slice(job.indexOf("Record evidence"));
    for (const id of ids) {
      if (SELF_HANDLING.has(id)) continue;
      assert.ok(
        record.includes(`steps.${id}.outcome`),
        `step id '${id}' has no rec row — its failure would be recorded as 'skipped' rows only`,
      );
    }
  });

  test(`contract: ${name} lane steps gate on tuple_ok`, () => {
    // every lane step after resolution must refuse to run on tuple mismatch
    const laneSteps = [...job.matchAll(/if: steps\.gate\.outputs\.gate == 'run' && steps\.resolution\.outputs\.tuple_ok == 'true'/g)];
    assert.ok(laneSteps.length >= 8, `expected >=8 tuple_ok-gated steps, got ${laneSteps.length}`);
  });

  test(`contract: ${name} lanes file is truncated before recording`, () => {
    assert.ok(/: > "\$RUNNER_TEMP\/\w+-lanes\.jsonl"/.test(job),
      "stale JSONL rows on persistent runners could fabricate pass");
  });

  test(`contract: ${name} extra-task step outcome is backstop-recorded`, () => {
    const record = job.slice(job.indexOf("Record evidence"));
    assert.ok(/extra tasks \(aggregate\).*"extra dispatch input" "\$\(o '\$\{\{ steps\.extra\.outcome \}\}'\)"/.test(record),
      "a mid-loop kill of the extra step must still produce a fail row");
  });

  test(`contract: ${name} result step pins tuple env against GITHUB_ENV forgery`, () => {
    const record = job.slice(job.indexOf("Record evidence"), job.indexOf("Upload evidence"));
    for (const key of ["EXPECTED_SHA", "REQUESTED_REF", "CONSUMER_NAME", "CONSUMER_REPO"]) {
      assert.ok(new RegExp(`${key}: `).test(record), `result env must pin ${key}`);
    }
  });

  test(`contract: ${name} artifact name is attempt-unique`, () => {
    assert.ok(/consumer-evidence-\w+-attempt-\$\{\{ github\.run_attempt \}\}/.test(job));
  });

  test(`contract: ${name} declares >=2 static unsupported rows and a job timeout`, () => {
    const count = (job.match(/"unsupported" /g) ?? []).length;
    assert.ok(count >= 2, `expected >=2 static unsupported rows, got ${count}`);
    assert.ok(/timeout-minutes:/.test(job), "extra tasks are unbounded without a job timeout");
  });
}

test("contract: expected-SHA guard is fail-closed in both lanes", () => {
  assert.equal((src.match(/tuple_mismatch/g) ?? []).length >= 2, true);
  assert.equal((src.match(/EXPECTED_SHA/g) ?? []).length >= 4, true);
});

test("contract: dispatch inputs never interpolate into run: scripts", () => {
  // ${{ inputs.* }} inside a run: block executes before validation can reject
  // it — all inputs must reach bash via env instead.
  const runs = [...src.matchAll(/run: \|([\s\S]*?)(?=\n\s{6}- name:|\n\s{2}\w|\n\w|$)/g)].map((m) => m[1]);
  for (const [i, body] of runs.entries()) {
    assert.ok(!/\$\{\{ inputs\./.test(body), `inputs.* interpolated into run: script #${i}`);
  }
});

test("contract: UI headless lanes wired on both consumers", () => {
  for (const lane of ["sync-ui-deps", "setup-ui", "check-ui", "test-ui"]) {
    assert.ok(audiolens.includes(`mise run ${lane}`), `audiolens lost ${lane}`);
    assert.ok(synvelis.includes(`mise run ${lane}`), `synvelis lost ${lane}`);
  }
  assert.ok(audiolens.includes("check-ruviform-rev.mjs --require-vendor"));
  assert.ok(synvelis.includes("check-ruviform-rev.mjs --require-vendor"));
  // synvelis post-build manifest leg must run after build-ui (vacuous before)
  const build = synvelis.indexOf("mise run build-ui");
  const post = synvelis.lastIndexOf("check-ruviform-rev.mjs --require-vendor");
  assert.ok(build >= 0 && post > build, "synvelis require-vendor must follow build-ui");
});

test("contract: extra tasks are validated and per-task recorded", () => {
  for (const job of [audiolens, synvelis]) {
    assert.ok(/IFS=',' read -ra tasks/.test(job), "extra task loop missing");
    assert.ok(/extra: mise run \$t/.test(job), "extra task lane record missing");
    assert.ok(/env\.EXTRA_TASKS != ''/.test(job), "extra step must gate on input presence");
  }
  assert.ok(/\^A-Za-z0-9\]\[A-Za-z0-9_-\]\*\$/.test(preflight) || /A-Za-z0-9_-\]\*\$/.test(preflight));
  assert.ok(/max 8|too many extra tasks/.test(preflight));
});

test("contract: summary keeps not-run-is-not-pass enforcement + tuple table", () => {
  assert.ok(/all not_run\/unsupported|no consumer lane produced evidence/.test(summary));
  assert.ok(/unsupported/.test(summary), "unknown/unsupported statuses must not pass");
  assert.ok(/resolved_sha/.test(summary), "tuple table lost resolved sha");
  assert.ok(/consumer-verify-tuple\/v1/.test(summary), "contract id missing from summary");
});

test("contract: evidence.mjs checkout is the dispatch ref (no candidate ref)", () => {
  const checkouts = [...src.matchAll(/uses: actions\/checkout@[\w.]+/g)];
  assert.ok(checkouts.length >= 3, "preflight + both verify jobs need ruviform checkout");
  assert.ok(!/with:\s*\n\s+ref: \$RUVIFORM_REV/.test(src) && !/ref: \$\{\{ env\.RUVIFORM_REV \}\}/.test(src),
    "script source must be the dispatch ref, not the candidate SHA");
});
