---
id: "TASK-034NlPHWwEG0sAKxw5XB88"
type: "task"
status: "done"
title: "L1 bundle child final verification（Full Gate・AC 証拠・IR・closeout）"
created: "2026-10-28"
updated: "2026-10-28"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/117"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-117-bundle-child-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/wire.rs"
      - type: "file"
        path: "docs/specs/issue-117-bundle-child-process.md"
      - type: "file"
        path: "docs/plans/implement-issue-117-bundle-child-process.md"
      - type: "file"
        path: "docs/test-specs/issue-117-wire-v2-codec-tests.md"
      - type: "file"
        path: "docs/test-specs/issue-117-bundle-failed-granularity-tests.md"
      - type: "file"
        path: "docs/test-specs/issue-117-bundle-e2e-tests.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/exp/data/issue-117-final-gate-fmt-check-20261028.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-117-final-gate-check-20261028.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-117-final-gate-lint-20261028.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-117-final-gate-test-20261028.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-117-final-gate-test-rt-20261028.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-117-final-gate-bench-smoke-20261028.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-117-final-gate-lint-l1-real-plugin-20261028.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-117-final-gate-test-l1-real-plugin-20261028.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-117-final-gate-cargo-doc-before-fix-20261028.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-117-final-gate-cargo-doc-20261028.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-117-final-gate-prek-20261028.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-117-final-gate-doc-audit-20261028.log"
  implements:
    - "docs/plans/implement-issue-117-bundle-child-process.md"
  implemented-by:
    - "docs/impl/ir/issue-117-bundle-child-implementation.md"
  depends-on:
    - "docs/plans/implement-issue-117-bundle-child-process.md"
    - "docs/tasks/l1-bundle-e2e-and-granularity-verification.md"
    - "docs/tasks/l1-wire-v2-docs-sweep.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/real-adapter-final-verification.md"
    - "docs/tasks/l1-internal-cleanup.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-117-bundle-child-process.md"
    - "docs/designs/issue-117-bundle-child-process.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/76"
  defers: []
  deferred-by: []
---

# L1 bundle child final verification（Full Gate・AC 証拠・IR・closeout）

## Work

T1-T5 で着地した bundle 実装の Full Gate・AC 証拠収集・IR・status 同期を行う closeout task。spec AC1-AC12 の全行を evidence で確認し、実装の不整合・欠落があれば同 task で完結する fix を追加（新機能は追加しない — 欠落が大きい場合は前 task へ差し戻す）。

1. Full Gate — `mise run fmt-check` → `check` → `lint` → `test` → `test-rt` → `bench-smoke` を全 pass。`mise run test-l1-real-plugin`（opt-in lane — real clap/vst3 runner bin が v2 handshake（spawn→`Hello`→`Scan`→`CreateInstance`）を実子プロセスで exercise。runner bin は `required-features = "host"` で default gate では compile されない）・`mise run lint-l1-real-plugin` も実行。`cargo doc -p ruviform-plugin-core -p ruviform-plugin-host -p ruviform-plugin-export --no-deps` が warning 0。`prek run --all-files`（pre-commit hook）も確認
2. focused E2E — `cargo test -p ruviform-plugin-host --test l1_proxy_e2e`（bundle + dedicated）・`cargo test -p ruviform-plugin-export --test l1_runner_e2e`（registry bundle）・`cargo test -p ruviform-plugin-core`（codec）・`cargo test -p ruviform-plugin-host rt_`（bundle RT）
3. AC1-AC12 証拠 matrix — spec `docs/specs/issue-117-bundle-child-process.md` の各 AC に対し、対応する test 名・実行結果を収集（plan の Verification Matrix を正本として使用）
4. literal sweep 最終確認 — `grep -rn "WIRE_VERSION" crates/` で `1` 残存なし・`grep -rn '\-\-ruviform-\|one child\|per-instance spawn\|1 子\|singleton' crates/ docs/` で historical record 以外に残存なし
5. doc audit — `node .agents/skills/doc-status/scripts/doc_status.js audit --json` が `blocking:0`・`findings:[]`
6. IR + status 同期 — `docs/impl/ir/issue-117-bundle-child-implementation.md` を作成（AC 証拠・task 完了記録・design 差分を収録）し、`docs/impl/ir/README.md` の curated index に index 行を登録する（全 IR が登録される運用のため欠落させない）。spec `status: draft → implemented`・各 task `status: todo → done`・plan `status: approved → completed`（実装完了時の convention に従う）。`docs/plans/README.md`・`docs/tasks/README.md`・`docs/test-specs/README.md` の index 行を確認（planning 時点で作成済み）
7. flake 切り分け — 既知の負荷依存 flake（`l1_proxy_e2e`）が出た場合は孤立 rerun で切り分け、結果を IR へ記録

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-117-bundle-child-process.md](../plans/implement-issue-117-bundle-child-process.md)（T6 — closeout）
- [../specs/issue-117-bundle-child-process.md](../specs/issue-117-bundle-child-process.md)（AC1-AC12 の証拠対象）
- [../designs/issue-117-bundle-child-process.md](../designs/issue-117-bundle-child-process.md) Verification Notes（L394-408 — AC 証拠の oracle）
- [../adr/issue-117-bundle-child-failed-granularity.md](../adr/issue-117-bundle-child-failed-granularity.md)（Failed 粒度の最終確認）
- 先行 task: [l1-wire-v2-codec-cutover.md](l1-wire-v2-codec-cutover.md)・[l1-child-instance-split.md](l1-child-instance-split.md)・[l1-route-pool-and-isolation-surface.md](l1-route-pool-and-isolation-surface.md)・[l1-bundle-e2e-and-granularity-verification.md](l1-bundle-e2e-and-granularity-verification.md)・[l1-wire-v2-docs-sweep.md](l1-wire-v2-docs-sweep.md)

## Dependencies

- 先行タスク: T4（bundle 検証）・T5（docs sweep）の両方 — 本 task が収束点

## Affected Files

- `docs/impl/ir/issue-117-bundle-child-implementation.md` — 新規 IR
- `docs/impl/ir/README.md` — IR の index 行登録（curated index — 全 IR が登録される）
- `docs/specs/issue-117-bundle-child-process.md`・`docs/plans/implement-issue-117-bundle-child-process.md`・6 task 文書 — status 同期（実装完了時）
- `docs/plans/README.md`・`docs/tasks/README.md`・`docs/test-specs/README.md` — index 行確認（planning 時点で作成済み・status 列を実装状態へ同期する場合のみ更新）

## Done When

- [x] Full Gate（`fmt-check`/`check`/`lint`/`test`/`test-rt`/`bench-smoke`/`test-l1-real-plugin`/`lint-l1-real-plugin`/`cargo doc`/doc audit/pre-commit）が全 pass（実績: 12 コマンド — `cargo doc` は `wire.rs` `pub mod shm` doc の pre-existing intra-doc link 欠陥で初回 exit 101 → doc-only 補正（`[`X`]`→`[`shm::X`]`）後 exit 0・warning 0。その他 11 は初回から全 exit 0。全 log `docs/impl/exp/data/issue-117-final-gate-*-20261028.log`）
- [x] spec AC1-AC12 の全行が test 名 + 実行結果で証拠化されている（IR `docs/impl/ir/issue-117-bundle-child-implementation.md` の coverage 表 — 各 AC へ test 名・file:line・pin 内容を記録）
- [x] literal sweep（`WIRE_VERSION` `1`・`--ruviform-` 等）が historical record 以外に残存なし（実績: `WIRE_VERSION = 2` 単一・`--ruviform-` は退役拒否契約/歴史記述のみ・spawn 側は `spec.args` のみ — 生きた v1 argv 残存ゼロ）
- [x] IR が AC 証拠・task 完了・design 差分を収録し、`docs/impl/ir/README.md` index に登録されている（行追加済み）
- [x] spec/task/plan の status が実装状態へ同期されている（spec `draft → implemented`・plan `approved → completed`・3 test-spec `proposed → approved`・T1-T6 task `done`・5 index 行同期）
- [x] `doc_status.js audit --json` が `blocking:0`・`findings:[]` を返す（実績: 最終編集後に `blocking:0`・`findings:[]` — 初回の `docs/tasks/README.md` T4 index 未同期 warning 1 件は本 task の index 同期で解消）

## Verification

- [x] `mise run fmt-check`・`check`・`lint`・`test`・`test-rt`・`bench-smoke` が全 exit 0（実績: `test` 59 binaries・1239 passed・0 failed / `test-rt` 実一致 132・0 failed — 既知負荷依存 flake 未発生）
- [x] `mise run test-l1-real-plugin`・`mise run lint-l1-real-plugin` が pass（opt-in lane — real runner bin が v2 handshake を exercise。実績: lint exit 0・test 3 段全 green — build（fixture+runner 生成）→ `--lib --bins`（CLAP 63+2/VST3 47+2）→ `--ignored`（CLAP 3+VST3 3・実子プロセス））
- [x] `mise exec -- cargo doc -p ruviform-plugin-core -p ruviform-plugin-host -p ruviform-plugin-export --no-deps` が warning 0（`RUSTDOCFLAGS="-D warnings"` 下で exit 0 — 機械証明。初回 exit 101 の pre-existing 欠陥は doc-only 補正で解消・before/after 両 log 保存）
- [x] `mise exec -- prek run --all-files` が pass（exit 0・全 hook pass）
- [x] `node .agents/skills/doc-status/scripts/doc_status.js audit --json` が `blocking:0`・`findings:[]` を返す（最終編集後に実施）
- [x] focused E2E（`l1_proxy_e2e`・`l1_runner_e2e`・`wire` codec・`rt_` bundle）が全 pass（実績: `l1_proxy_e2e` 100 tests・`l1_runner_e2e` 21 tests・wire codec mod tests・`rt_l1_bundled_process_no_alloc` 実 pass — `test`/`test-rt` log）
- [x] spec AC1-AC12 の証拠 matrix が完備されている（IR coverage 表）
