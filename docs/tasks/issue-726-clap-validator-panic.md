---
id: "TASK-726clapValidatorPanic05"
type: "task"
status: "done"
title: "Issue #726 clap-validator param-fuzz panic — task"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/plans/issue-726-clap-validator-panic.md"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-726-clap-validator-panic.md"
  implements:
    - "docs/plans/issue-726-clap-validator-panic.md"
  implemented-by:
    - "docs/impl/ir/issue-726-clap-validator-panic.md"
  depends-on:
    - "docs/plans/issue-726-clap-validator-panic.md"
    - "docs/discovery/issue-726-clap-validator-panic.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #726 task

- [x] T1: baseline 確認 — `node --test
      ci/plugin-abi-validator/tests/run.test.mjs`（21 tests pass）と
      fixture DLL build、upstream checkout（master `5bc2ef3`）の source 照合
- [x] T2: panic 最小再現の絞り込み — `--include`/`--exclude` filter で
      `param-fuzz-basic`/`param-fuzz-modulation` が単独 panic、
      `param-fuzz-sample-accurate` は clean `crashed` 記録と確定。
      raw 出力を `evidence/rf726/filter-runs/` に保存
- [x] T3: 根因連鎖の確定 — fixture `exit(0)` mode（stepped `2.0`）→
      sandboxed child success+無出力 → `sandbox.rs`:98 `.expect()`
      panic → `cli.rs`:55 scoped thread panic → exit 101・JSON 未出力。
      upstream source で未修正を確認
- [x] T4: upstream 照合 — issue list に同件なし、#41/#42 は master 修正済み
      だが 0.4.1 未収録と整合。報告要否 = 報告する（ドラフトを
      `evidence/rf726/upstream-report-draft.md` に記録、実起票は
      Coordinator hand-off）
- [x] T5: runner 契約照合 — verdict ロジック（`normalizeClapLeg`/
      `evaluate`）は契約適合を確認したが、lane 実走で pipe stdio の
      ETIMEDOUT 誤記録（orphan grandchild が pipe 保持 → 完了済み
      JSON を捨てて Failed）を実測 → `spawnCapture`（file-backed
      stdio）を追加し `runClapLeg` に適用 + contract test 2 件
- [x] T6: leg 扱い決定 — canonical exclude + informational Unsupported
      継続、pin は upstream fix 入り release 待ち、無フィルタ run 再開条件を
      design doc に明文化
- [x] T7: opt-in lane 実走 — 修正前 runner で `abi-validator/`（全 leg
      ETIMEDOUT 誤記録を証拠保存）、修正後 runner で
      `abi-validator-fixed/` に run.json 記録
- [x] T8: `evidence/rf726/` に investigation.md・upstream-report-draft.md・
      filter-runs/・abi-validator/ を整理
- [x] T9: docs chain 整備 + `node --test` + `prek run --all-files` 検証

## Verification

- [x] `node --test ci/plugin-abi-validator/tests/` exits 0（23 tests）
- [x] `node ci/plugin-abi-validator/run.mjs --clap-validator
      "$LOCALAPPDATA/ruviform/plugin-abi-validator/clap-validator.exe"
      --clap-production --evidence-dir evidence/rf726/abi-validator-fixed`
      — run.json が出力され verdict が記録される（結果は impl 記録参照）
- [x] `clap-validator validate --json --plugin-id
      org.ruviform-test.clap-test-fault --include param-fuzz-basic
      target/debug/ruviform_plugin_test_clap.dll` が exit 101・
      JSON 未出力で panic を再現（`filter-runs/param-fuzz-basic.*`）
- [x] `mise exec -- prek run --all-files` exits 0
