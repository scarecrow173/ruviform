---
id: "TASK-034NlPHWwEG0sAKxw5XB88"
type: "task"
status: "todo"
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
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-117-bundle-child-process.md"
  implemented-by: []
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

1. Full Gate — `mise run fmt-check` → `check` → `lint` → `test` → `test-rt` → `bench-smoke` を全 pass。`cargo doc -p ruviform-plugin-core -p ruviform-plugin-host -p ruviform-plugin-export --no-deps` が warning 0。`prek run --all-files`（pre-commit hook）も確認
2. focused E2E — `cargo test -p ruviform-plugin-host --test l1_proxy_e2e`（bundle + dedicated）・`cargo test -p ruviform-plugin-export --test l1_runner_e2e`（registry bundle）・`cargo test -p ruviform-plugin-core`（codec）・`cargo test -p ruviform-plugin-host rt_`（bundle RT）
3. AC1-AC12 証拠 matrix — spec `docs/specs/issue-117-bundle-child-process.md` の各 AC に対し、対応する test 名・実行結果を収集（plan の Verification Matrix を正本として使用）
4. literal sweep 最終確認 — `grep -rn "WIRE_VERSION" crates/` で `1` 残存なし・`grep -rn '\-\-ruviform-\|one child\|per-instance spawn\|1 子\|singleton' crates/ docs/` で historical record 以外に残存なし
5. doc audit — `node .agents/skills/doc-status/scripts/doc_status.js audit --json` が `blocking:0`・`findings:[]`
6. IR + status 同期 — `docs/impl/ir/issue-117-bundle-child-implementation.md` を作成（AC 証拠・task 完了記録・design 差分を収録）。spec `status: draft → implemented`・各 task `status: todo → done`・plan `status: approved → completed`（実装完了時の convention に従う）。`docs/plans/README.md`・`docs/tasks/README.md`・`docs/test-specs/README.md` の index 行を確認（planning 時点で作成済み）
7. flake 切り分け — 既知の負荷依存 flake（`l1_proxy_e2e`）が出た場合は孤立 rerun で切り分け、結果を IR へ記録

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-117-bundle-child-process.md](../plans/implement-issue-117-bundle-child-process.md)（T6 — closeout）
- [../specs/issue-117-bundle-child-process.md](../specs/issue-117-bundle-child-process.md)（AC1-AC12 の証拠対象）
- [../designs/issue-117-bundle-child-process.md](../designs/issue-117-bundle-child-process.md) Verification Notes（L381-396 — AC 証拠の oracle）
- [../adr/issue-117-bundle-child-failed-granularity.md](../adr/issue-117-bundle-child-failed-granularity.md)（Failed 粒度の最終確認）
- 先行 task: [l1-wire-v2-codec-cutover.md](l1-wire-v2-codec-cutover.md)・[l1-child-instance-split.md](l1-child-instance-split.md)・[l1-route-pool-and-isolation-surface.md](l1-route-pool-and-isolation-surface.md)・[l1-bundle-e2e-and-granularity-verification.md](l1-bundle-e2e-and-granularity-verification.md)・[l1-wire-v2-docs-sweep.md](l1-wire-v2-docs-sweep.md)

## Dependencies

- 先行タスク: T4（bundle 検証）・T5（docs sweep）の両方 — 本 task が収束点

## Affected Files

- `docs/impl/ir/issue-117-bundle-child-implementation.md` — 新規 IR
- `docs/specs/issue-117-bundle-child-process.md`・`docs/plans/implement-issue-117-bundle-child-process.md`・6 task 文書 — status 同期（実装完了時）
- `docs/plans/README.md`・`docs/tasks/README.md`・`docs/test-specs/README.md` — index 行確認（planning 時点で作成済み・status 列を実装状態へ同期する場合のみ更新）

## Done When

- [ ] Full Gate（`fmt-check`/`check`/`lint`/`test`/`test-rt`/`bench-smoke`/`cargo doc`/doc audit/pre-commit）が全 pass
- [ ] spec AC1-AC12 の全行が test 名 + 実行結果で証拠化されている
- [ ] literal sweep（`WIRE_VERSION` `1`・`--ruviform-` 等）が historical record 以外に残存なし
- [ ] IR が AC 証拠・task 完了・design 差分を収録する
- [ ] spec/task/plan の status が実装状態へ同期されている
- [ ] `doc_status.js audit --json` が `blocking:0`・`findings:[]` を返す

## Verification

- [ ] `mise run fmt-check`・`check`・`lint`・`test`・`test-rt`・`bench-smoke` が全 exit 0
- [ ] `mise exec -- cargo doc -p ruviform-plugin-core -p ruviform-plugin-host -p ruviform-plugin-export --no-deps` が warning 0
- [ ] `mise exec -- prek run --all-files` が pass
- [ ] `node .agents/skills/doc-status/scripts/doc_status.js audit --json` が `blocking:0`・`findings:[]` を返す
- [ ] focused E2E（`l1_proxy_e2e`・`l1_runner_e2e`・`wire` codec・`rt_` bundle）が全 pass
- [ ] spec AC1-AC12 の証拠 matrix が完備されている
