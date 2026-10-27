---
id: "TASK-034MLSzaZebh5KuCkE4guN"
type: "task"
status: "done"
title: "real-adapter final verification + closeout（Full Gate・opt-in lane 実測・AC 証跡・status 同期）"
created: "2026-10-26"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-114-real-adapter-final-verification-closeout-task-5-implementation.md"
    modified:
      - type: "file"
        path: "docs/plans/implement-issue-114-real-adapter-l1-e2e.md"
      - type: "file"
        path: "docs/specs/issue-114-real-adapter-l1-e2e.md"
      - type: "file"
        path: "docs/designs/issue-114-real-adapter-l1-e2e.md"
      - type: "file"
        path: "docs/test-specs/adapter-host-bridge-contract-tests.md"
      - type: "file"
        path: "docs/test-specs/real-adapter-l1-e2e-tests.md"
      - type: "file"
        path: "docs/test-specs/l1-real-plugin-lane-tests.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-fmt-check-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-check-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-lint-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-test-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-test-rt-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-bench-smoke-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-build-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-test-l1-real-plugin-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-lint-l1-real-plugin-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-prek-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-doc-audit-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-cargo-tree-adapters-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-cargo-tree-adapters-normal-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-cargo-tree-adapters-host-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-cargo-tree-reverse-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-cargo-doc-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-test-adapters-host-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-test-rt-host-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-runner-direct-launch-20261027.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-114-final-gate-SUMMARY-20261027.log"
  implements:
    - "docs/plans/implement-issue-114-real-adapter-l1-e2e.md"
  implemented-by:
    - "docs/impl/ir/issue-114-real-adapter-final-verification-closeout-task-5-implementation.md"
  depends-on:
    - "docs/plans/implement-issue-114-real-adapter-l1-e2e.md"
    - "docs/tasks/real-adapter-e2e-and-lane.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/engine-plugin-final-verification.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-114-real-adapter-l1-e2e.md"
    - "docs/designs/issue-114-real-adapter-l1-e2e.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/adapter-host-bridge-contract-tests.md"
    - "docs/test-specs/real-adapter-l1-e2e-tests.md"
    - "docs/test-specs/l1-real-plugin-lane-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# real-adapter final verification + closeout（Full Gate・opt-in lane 実測・AC 証跡・status 同期）

## Work

Plan actions 15–16。全実装 task 完了後に **Full Gate を 1 回のみ**実行し、opt-in lane の実測を含む AC 証跡を収集して canonical closeout（IR・status 同期・design/spec 実装相違の正本反映）を行う。繰返し実行はしない — 全 log を `docs/impl/exp/data/issue-114-final-gate-*-*.log` に保存する。

- **Full Gate（1 回のみ）**: `mise run fmt-check`・`check`・`lint`・`test`・`test-rt`・`bench-smoke`・`mise run test-l1-real-plugin`（opt-in lane・fixture+runner+`--ignored` E2E の実測）・`mise run lint-l1-real-plugin`（host feature clippy）・`mise exec -- prek run --all-files`・doc audit（`mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --json` — 0 findings）・`cargo tree -p ruviform-plugin-clap -p ruviform-plugin-vst3`（DAG 監査 — adapter→export/host(dev) 方向・逆向き依存ゼロ・`host` off で新規 dep が non-optional 木に出ない）・`mise exec -- cargo doc -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host --no-deps`（rustdoc warning 0）
- **`test-rt` は実一致件数で証明**: `cargo test --workspace rt_` の出力から `rt_` 命名テストの件数を確認し、本変更で件数が無退行（`rt_` 命名規約維持 — bridge 追加テストに `rt_` 名なし）であることを記録
- **opt-in lane 実測の証拠**: `mise run test-l1-real-plugin` の実行 log に fixture+runner の両生成と `--ignored` テストの全 pass（両規格主 E2E + kill + binary 不在系）が含まれることを確認 — 「task はあるが実行しない」偽装を排除
- **AC 証跡の収集**: AC1–AC8 ↔ actions/test-specs の写像を plan Verification 節と照合 — AC1（runner・`cargo build --features host` + runner 直接起動 exit 2/1）・AC2/AC3（CLAP/VST3 E2E・`--ignored` pass + handshake field-equality + ParamValue 係数）・AC4（`kill_child`→`Failed`+`InstanceFailed` once・binary 不在→`Err(Ipc)`）・AC5（OFX 境界明文化の crate README 記載）・AC6（`l1-real-verify` job の ci.yml 内存在・3-OS matrix）・AC7（`cargo tree` DAG 監査・wire/export/host/core 差分なし・default gate 不変グリーン）・AC8（Full Gate exit 0 + doc audit 0 findings）
- **canonical closeout**: 実績 IR（`docs/impl/ir/issue-114-*.md` — 実装記録）・coverage 記述（AC↔action↔test-spec 写像の実績版）・task/plan status 同期（全 task `done`・plan `completed` — closeout task 自体は作業完了後に `done`）・**3 test-spec を実結果で `approved` 化**（proposed → approved — 実測済みの証拠を持つ）・design/spec の実装相違は正本へ同期反映（実装中に確定した差分 — e.g. `factories()`/`descriptors()` の戻り値形・`Drop` 駆動 teardown・`uid()` platform-order id hex の実装決定）を design/spec に同期（`updated` を更新）・GitHub merge は lead の authoritative `gh` のみ（本 task では merge を実施しない — 実装完了の証跡収集と doc 同期に留まる）
- **IR（実装記録）の配置**: `docs/impl/ir/issue-114-clap-host-bridge-*.md`・`issue-114-vst3-host-bridge-*.md`・`issue-114-real-adapter-runner-e2e-*.md`（または統合 1 件 — task 単位か Issue 単位かは実装完了時の coverage 粒度で判断 — Issue #113 の `issue-113-plugin-core-endpoint-primitives-task-1-implementation.md` 先例は task 単位）

## Classification

- `normal-plan-task`（final verification — 全実装 task 完了後に 1 回のみ・closeout 含む）

## Upstream Documents

- `docs/plans/implement-issue-114-real-adapter-l1-e2e.md`（actions 15–16・Verification・Rollback）
- `docs/designs/issue-114-real-adapter-l1-e2e.md`（決定サマリ j・「検証戦略」「Full Gate」「canonical closeout」）
- `docs/specs/issue-114-real-adapter-l1-e2e.md`（AC8 — quality gate・status 同期）
- `docs/test-specs/adapter-host-bridge-contract-tests.md`・`real-adapter-l1-e2e-tests.md`・`l1-real-plugin-lane-tests.md`（実結果で `approved` 化の対象）
- `docs/tasks/engine-plugin-final-verification.md`（Issue #113 final verification 先例 — Full Gate・IR・status 同期・closeout）

## Dependencies

- `docs/plans/implement-issue-114-real-adapter-l1-e2e.md`
- `docs/tasks/real-adapter-e2e-and-lane.md`（lane + doc 配線 — Full Gate の対象となる全 lane/task が完了済みである前提）
- 全実装 task（`clap-host-bridge`・`vst3-host-bridge`・`real-adapter-runner-bins`・`real-adapter-e2e-and-lane`）が `done` であること — Full Gate は完了後の 1 回のみ

## Affected Files

- `docs/impl/ir/issue-114-*.md`（新規 — 実装記録）
- `docs/impl/exp/data/issue-114-final-gate-*.log`（新規 — Full Gate log 保存）
- `docs/plans/implement-issue-114-real-adapter-l1-e2e.md` — `status: completed`（closeout 時）
- `docs/tasks/clap-host-bridge.md`・`vst3-host-bridge.md`・`real-adapter-runner-bins.md`・`real-adapter-e2e-and-lane.md` — `status: done`（実装完了後・closeout 時に同期）
- `docs/tasks/real-adapter-final-verification.md` — `status: done`（本 task 自体）
- `docs/test-specs/adapter-host-bridge-contract-tests.md`・`real-adapter-l1-e2e-tests.md`・`l1-real-plugin-lane-tests.md` — `status: approved`（実測済み証拠を持つ）
- `docs/designs/issue-114-real-adapter-l1-e2e.md`・`docs/specs/issue-114-real-adapter-l1-e2e.md` — 実装相違の同期反映（`updated` 更新 — 実装中に確定した差分を正本に反映）
- `docs/plans/README.md`・`docs/tasks/README.md`・`docs/test-specs/README.md` — status 更新（plan completed・task done・test-spec approved）
- `docs/specs/issue-114-real-adapter-l1-e2e.md` — `status`（実装完了・全 AC 証跡ありなら `approved` 相当の更新 — 現在 `proposed`）

## Done When

- [x] Full Gate が 1 回のみ実行され exit 0 — `fmt-check`/`check`/`lint`/`test`/`test-rt`/`bench-smoke`/`test-l1-real-plugin`/`lint-l1-real-plugin`/`prek`/doc audit/`cargo tree`/`cargo doc` の全 log が `docs/impl/exp/data/issue-114-final-gate-*-*.log` に保存される（実績: **19 コマンド全 exit 0・20 log** — `issue-114-final-gate-*-20261027.log` + SUMMARY）
- [x] `mise run test-l1-real-plugin` の実測 log に fixture+runner 生成と `--ignored` テスト全 pass（両規格主 E2E + kill + binary 不在系）が含まれる（実績: 段 1 build で fixture cdylib + runner bin 生成・段 2 `--lib --bins` CLAP 63+2/VST3 47+2・段 3 `--ignored` CLAP 3 + VST3 3 全 pass）
- [x] `test-rt` の実一致件数が無退行（`rt_` 命名規約維持 — bridge 追加テストに `rt_` 名なし）で記録される（実績: **実一致 129・0 failed** — #113 final gate の 129 と同一。feature `host` 内の `rt_process_drives_fixture_block_allocation_free` は feature-gated で default lane に出ない — opt-in lane 段 2 と `test-rt-host` log で実 pass を別証跡化）
- [x] AC1–AC8 ↔ actions/test-specs の写像が実績として記録される（全 AC の証跡が揃う — IR `issue-114-real-adapter-final-verification-closeout-task-5-implementation.md` の coverage 表）
- [x] `cargo tree` の DAG 監査で adapter→export/host(dev) 方向・逆向き依存ゼロ・`host` off で新規 dep が non-optional 木に出ないことが記録される（実績: `--edges normal` で `clack-host`/`libloading`/`ruviform-plugin-export`/`ruviform-plugin-host` 非出現・export/host 逆方向にも adapter 非出現 — `cargo-tree-*` 4 log）
- [x] canonical closeout — IR・coverage 記述・全 task `done`・plan `completed`・3 test-spec `approved`（実測証拠）・design/spec 実装相違の正本同期・GitHub merge は lead の authoritative `gh` のみ（本 task では merge を実施しない）
- [x] doc audit（`mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --json`）が 0 findings（gate 時点 `blocking: 0`・`findings: []` + 本編集後の最終 audit も clean）

## Verification

- [x] `mise run fmt-check` — exit 0（`cargo fmt --all -- --check` 非破壊）
- [x] `mise run check` — exit 0（`cargo check --workspace --all-targets` dev プロファイル — default 構成）
- [x] `mise run lint` — exit 0（`cargo clippy --workspace --all-targets -- -D warnings` — default 構成）
- [x] `mise run test` — exit 0（`cargo test --workspace` — default 構成・`#[ignore]` E2E は実行されない — **58 binaries・1125 passed・0 failed**）
- [x] `mise run test-rt` — exit 0（`cargo test --workspace rt_` — **実一致 129・0 failed**・無退行）
- [x] `mise run bench-smoke` — exit 0（`cargo bench -p ruviform-math -p ruviform-rt -- --test` — 計測なし 1 回）
- [x] `mise run test-l1-real-plugin` — exit 0（**opt-in lane 実測** — `cargo build --features host` → `cargo test --features host --lib --bins` → `cargo test --test l1_real_adapter_e2e -- --ignored` — 両規格主 E2E + kill + binary 不在系全 pass・fixture+runner 生成の log 含む）
- [x] `mise run lint-l1-real-plugin` — exit 0（`cargo clippy -p ... --all-targets --features host -- -D warnings` — host feature 0 warning）
- [x] `mise exec -- prek run --all-files` — exit 0（設定済みフックを全ファイルに実行）
- [x] `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --json` — 0 findings（`blocking: 0`・`findings: []`）
- [x] `cargo tree -p ruviform-plugin-clap -p ruviform-plugin-vst3` — adapter→export/host(dev) 方向・逆向き依存ゼロ・`host` off で新規 dep が non-optional 木に出ない（`--edges normal` で判定 — dev-dep feature union は意図的構成）
- [x] `cargo tree -p ruviform-plugin-clap --features host` ・ `cargo tree -p ruviform-plugin-vst3 --features host` — `clack-host`/`libloading`/`ruviform-plugin-export`/`clack-extensions/clack-host`（clap）/`libloading`/`ruviform-plugin-export`（vst3）が `host` 指定時のみ出現
- [x] `mise exec -- cargo doc -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host --no-deps` — rustdoc warning 0（`RUSTDOCFLAGS="-D warnings"` 下で exit 0 = warning 0 の機械証明）
- [x] `cargo test -p ruviform-plugin-clap -p ruviform-plugin-vst3 --features host` — exit 0（bridge unit test 層 — `#[ignore]` E2E 除外・`--ignored` なし — CLAP lib 63 + bin 2・VST3 lib 47 + bin 2）

補足（task doc の Verification 範囲外・IR に記録）: `mise run build` exit 0・`cargo test -p ... --features host rt_`（CLAP lib 11 matched — `rt_process_drives_fixture_block_allocation_free` 実 pass 含む・VST3 lib 8 matched）・runner 直接起動 smoke（両 runner: 引数なし → exit 2 usage reject・`RUVIFORM_L1_REAL_PLUGIN_DIR=/nonexistent` → exit 1 defined rejection — AC1 証跡の再実測）・`actionlint` clean（`l1-real-verify` は workflow_dispatch 専用のため実行不可 — macOS leg 初実測は dispatch 時）。既知 flake（`l1_proxy_e2e` の `wedged_child_write_converges_and_teardown_is_bounded`/`rejected_block_replies_resolve_pass_through`）は未発生 — 1 発目から全 pass。design 正本へ実装確定差分を同期（CLAP output bounce・`descriptors() -> &[…]`・`Drop` teardown・VST3 `uid()` platform-order id hex・param 表 load-hoist・lane 3 段化・feature-gated bridge rt_ test）。spec は `proposed → implemented`（実装完了・全 AC 証跡あり — 「approved 相当の更新」条件充足）。詳細は `docs/impl/ir/issue-114-real-adapter-final-verification-closeout-task-5-implementation.md`。
