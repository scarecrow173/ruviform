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
const MISE_TOML = fileURLToPath(new URL("../../../mise.toml", import.meta.url));
const miseSrc = readFileSync(MISE_TOML, "utf8");

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
    // daily tuple recovery (decide.mjs --mode tuple) trusts tuple.ruviformRev
    // and tuple.consumerResolvedSha — both must come from pinned inputs /
    // step outputs, never from a consumer task's $GITHUB_ENV write.
    for (const key of ["RUVIFORM_REV", "RESOLVED_SHA", "EXPECTED_SHA", "REQUESTED_REF", "CONSUMER_NAME", "CONSUMER_REPO"]) {
      assert.ok(new RegExp(`${key}: `).test(record), `result env must pin ${key}`);
    }
  });

  test(`contract: ${name} artifact name is attempt-unique and consumer-exact`, () => {
    // decide.mjs tuple recovery selects artifacts by the literal name
    // consumer-evidence-<consumer>-attempt-<run_attempt> — a renamed or
    // re-patterned artifact makes every historical run unverifiable.
    const consumer = name.replace(/^verify-/, "");
    assert.ok(
      job.includes(`consumer-evidence-${consumer}-attempt-\${{ github.run_attempt }}`),
      `artifact name must be exactly consumer-evidence-${consumer}-attempt-\${{ github.run_attempt }}`,
    );
  });

  test(`contract: ${name} declares >=2 static unsupported rows and a job timeout`, () => {
    const count = (job.match(/"unsupported" /g) ?? []).length;
    assert.ok(count >= 2, `expected >=2 static unsupported rows, got ${count}`);
    assert.ok(/timeout-minutes:/.test(job), "extra tasks are unbounded without a job timeout");
  });

  test(`contract: ${name} GUI contract lanes evaluate the candidate tree (guard + fake semantic)`, () => {
    // A-1 更新: GUI-CONTRACT boundary guard と GUI-SEMANTIC fake driver
    // contract は ruviform 候補 tree（RUVIFORM_REV）上で実行され、別 step・
    // 別 lane 行として outcome が記録される。候補に task がない場合は owner
    // 付き not_run（dispatch ref checkout の task では代用しない — 偽 pass
    // 禁止）。実 window/plugin 必須 lane は unsupported 固定行のまま。
    const record = job.slice(job.indexOf("Record evidence"));
    for (const id of ["gui_candidate", "gui_boundary", "gui_semantic"]) {
      assert.ok(new RegExp(`\\n\\s+id: ${id}\\n`).test(job), `step id ${id} missing`);
      assert.ok(record.includes(`steps.${id}.outcome`), `${id} outcome must map to a lane row`);
    }
    // 候補 fetch は dep step の url.insteadOf rewrite（deploy_key mode）に
    // 依存するため、gui_candidate は dep より後に置く。
    assert.ok(job.indexOf("id: dep") < job.indexOf("id: gui_candidate"),
      "gui_candidate must run after the dep access step");
    // 候補 checkout は Consumer clone と同一の統一 fetch 経路で候補 SHA を
    // 取得し、rev-parse HEAD を候補 SHA と照合してから評価に使う。
    assert.ok(/git -C "\$cand" fetch -q --depth 1 origin "\$RUVIFORM_REV"/.test(job),
      "candidate checkout must fetch the candidate SHA");
    assert.ok(/git -C "\$cand" rev-parse HEAD/.test(job) && /"\$head" != "\$RUVIFORM_REV"/.test(job),
      "candidate checkout must verify rev-parse HEAD == RUVIFORM_REV");
    // probe outputs は mise install 成功後にのみ出る — 先出しに戻すと install
    // 失敗時に lane skipped + pass-with-coverage 化する不誠実経路に戻る
    // （else 分岐の gui_candidate.outcome=fail 行へ落とす整合の pin）。
    const guiCand = job.slice(job.indexOf("id: gui_candidate"), job.indexOf("id: gui_boundary"));
    const installPos = guiCand.indexOf("mise install --locked rust");
    assert.ok(installPos >= 0, "gui_candidate must install the rust toolchain when tasks exist");
    assert.ok(guiCand.indexOf('echo "boundary=') > installPos,
      "probe outputs must be published only after mise install succeeds");
    assert.ok(!/GITHUB_OUTPUT/.test(guiCand.slice(0, installPos)),
      "no output may be published before the install block");
    // task probe — 候補 mise.toml の task 存在で実行可否を判定
    for (const task of ["check-ui-driver-boundary", "test-ui-driver-contract"]) {
      assert.ok(new RegExp(`\\[tasks\\\\\\.${task.replace(/-/g, "\\-")}\\\\\\]`).test(job),
        `candidate mise.toml probe for ${task} missing`);
      assert.ok(new RegExp(`mise run ${task.replace(/-/g, "\\-")}`).test(job),
        `lane step must run mise run ${task}`);
    }
    // lane step は候補 checkout dir を working-directory に持つ
    assert.equal(
      (job.match(/working-directory: \$\{\{ runner\.temp \}\}\/ruviform-candidate/g) ?? []).length >= 2,
      true, "GUI lane steps must run inside the candidate checkout");
    // task 不在時の not_run 行は owner（RF #390）+ 再実行条件 + 代用禁止を保持
    assert.ok(/GUI-CONTRACT boundary guard[^\n]*"not_run" "[^"]*RF #390/.test(job),
      "task-absent not_run row missing for boundary guard");
    assert.ok(/GUI-SEMANTIC fake driver contract[^\n]*"not_run" "[^"]*RF #390/.test(job),
      "task-absent not_run row missing for fake driver contract");
    // Consumer 側通常 View/Domain headless は未接続のまま owner 付き not_run
    assert.ok(/GUI-CONTRACT consumer-side[^\n]*"not_run" "[^"]*#1(66|75)/.test(job),
      "consumer-side GUI contract must stay a not_run row with owners");
    // capability-absent 固定行は維持
    const visual = job.match(/rec "GUI visual\/OS \+ 実 window lanes" "[^"]*" "[^"]*" "(\w+)"/);
    assert.ok(visual && visual[1] === "unsupported", "visual/OS real-window must stay unsupported");
    // 混同していた旧行名が残っていないこと
    assert.ok(!job.includes("contract/semantic/visual-OS/dock"), "conflated capability row must be removed");
    // 未接続時代の固定 not_run 行名は置換済みであること（同じ行名の再利用は不可）
    assert.ok(!job.includes('rec "GUI-CONTRACT checks'), "fixed not_run GUI-CONTRACT row must be replaced");
    assert.ok(!job.includes('rec "GUI semantic/fake-oracle checks'), "fixed not_run semantic row must be replaced");
  });

  test(`contract: ${name} candidate dep fetch fills the whole-workspace cache before probe outputs`, () => {
    // run42348131128 の実障害: 候補の GUI task は内部で `cargo metadata
    // --offline` を全 workspace（dev-dep 含む、例 criterion→alloca）に掛けるが、
    // Consumer clone の cargo fetch cache は ruviform 全依存を覆わない。候補
    // checkout で候補自身の lock（--locked）による `mise exec -- cargo fetch` を
    // toolchain 解決後・probe outputs 公開前に置く。fetch 失敗は step fail →
    // outputs 空 → record else 分岐の fail 行（not_run/pass 化しない）。
    const guiCand = job.slice(job.indexOf("id: gui_candidate"), job.indexOf("id: gui_boundary"));
    const installPos = guiCand.indexOf("mise install --locked rust");
    const fetchPos = guiCand.indexOf("mise exec -- cargo fetch");
    // install/fetch は task 存在分岐の中でのみ行う — task 不在候補で実行して
    // その失敗を fail 化しない（owner 付き not_run のまま）
    assert.ok(/if \[ "\$boundary" = "1" \] \|\| \[ "\$contract" = "1" \]/.test(guiCand),
      "install+fetch must stay inside the task-present branch");
    assert.ok(/set -euo pipefail/.test(guiCand), "step must keep fail-fast shell flags");
    assert.ok(installPos >= 0, "gui_candidate must install the rust toolchain when tasks exist");
    assert.ok(fetchPos > installPos,
      "candidate cargo fetch must run after the pinned toolchain install");
    // fetch 失敗が step fail になるよう、fetch は subshell の `&&` 末端である
    // こと（`|| true`・`;` 継続・コメント化は失敗を隠蔽して outputs 公開へ進む）
    assert.ok(/&& mise exec -- cargo fetch --locked\s*\)/.test(guiCand),
      "fetch must be the &&-chained tail so its failure fails the step");
    // 両 output とも fetch 成功後にのみ公開（partial early-publish は else
    // fail 行を迂回して lane 実行へ進ませる）
    for (const out of ["boundary", "contract"]) {
      assert.ok(fetchPos < guiCand.indexOf(`echo "${out}=`),
        `probe output '${out}' must be published only after the candidate fetch succeeds`);
    }
    assert.ok(
      /cd "\$cand"[^)]*mise exec -- cargo fetch --locked/.test(guiCand),
      "fetch must run inside the candidate checkout via mise exec on the candidate's own lockfile",
    );
    // 依存取得失敗が record 側の fail 行へ落ちる経路（boundary/semantic 両
    // else 分岐の理由に依存取得を含む — 片側だけ残す修正は行名偽装）
    const record = job.slice(job.indexOf("Record evidence"));
    assert.ok((record.match(/steps\.gui_candidate\.outcome[^\n]*依存取得/g) ?? []).length >= 2,
      "both gui_candidate failure rows must name dependency fetch in their reason");
  });
}

test("contract: boundary guard task stays --offline (fetch, not online relaxation)", () => {
  // run42348131128 の修復は guard の offline 契約を緩めない — 欠落依存の取得は
  // 候補 fetch 側で行う。`--offline` を落とすと fresh-cache 条件の退行が
  // online 化で隠れ、guard の別契約（online 許容）へすり替わる。
  const begin = miseSrc.indexOf("[tasks.check-ui-driver-boundary]");
  const end = miseSrc.indexOf("[tasks.test-ui-driver-contract]");
  assert.notEqual(begin, -1, "check-ui-driver-boundary task missing from mise.toml");
  assert.notEqual(end, -1, "test-ui-driver-contract task missing from mise.toml");
  const task = miseSrc.slice(begin, end);
  assert.ok(/gui-boundary-guard -- --offline/.test(task),
    "check-ui-driver-boundary must keep the --offline guard contract");
});

test("contract: concurrency group keys on the full tuple, not just refs", () => {
  // B-1: group が RF SHA + Consumer ref のみだと、同 ref・別 expected SHA /
  // extra tasks の run が pending 置換で証拠なく cancel される。tuple の全
  // 要素を key に含める。
  const group = src.match(/group: (consumer-verify-.+)\n/);
  assert.ok(group, "concurrency group missing");
  for (const part of [
    "inputs.ruviform_rev || github.sha",
    "inputs.audiolens_ref",
    "inputs.synvelis_ref",
    "inputs.audiolens_expected_sha",
    "inputs.synvelis_expected_sha",
    "inputs.audiolens_extra_tasks",
    "inputs.synvelis_extra_tasks",
  ]) {
    assert.ok(group[1].includes(`\${{ ${part} }}`), `concurrency group must include ${part}`);
  }
  assert.ok(/cancel-in-progress: false/.test(src));
});

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
