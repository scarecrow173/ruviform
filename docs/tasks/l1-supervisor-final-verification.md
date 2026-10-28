---
id: "TASK-034NOXJpTlwWqDQf7WFQFQ"
type: "task"
status: "done"
title: "L1 supervisor final verification（Full Gate・AC 証拠・IR・status 同期）"
created: "2026-10-28"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/116"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-116-supervisor-implementation.md"
    modified:
      - type: "file"
        path: "docs/specs/issue-116-supervisor.md"
      - type: "file"
        path: "docs/plans/implement-issue-116-supervisor.md"
      - type: "file"
        path: "docs/test-specs/issue-116-supervisor-core-tests.md"
      - type: "file"
        path: "docs/test-specs/issue-116-supervisor-e2e-tests.md"
      - type: "file"
        path: "docs/tasks/l1-test-worker-fault-flags.md"
      - type: "file"
        path: "docs/tasks/l1-supervisor-core.md"
      - type: "file"
        path: "docs/tasks/l1-supervisor-e2e.md"
      - type: "file"
        path: "docs/designs/issue-116-supervisor.md"
      - type: "file"
        path: "docs/adr/issue-116-supervisor-placement.md"
      - type: "file"
        path: "docs/discovery/issue-116-supervisor.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-116-supervisor.md"
  implemented-by:
    - "docs/impl/ir/issue-116-supervisor-implementation.md"
  depends-on:
    - "docs/plans/implement-issue-116-supervisor.md"
    - "docs/tasks/l1-test-worker-fault-flags.md"
    - "docs/tasks/l1-supervisor-core.md"
    - "docs/tasks/l1-supervisor-e2e.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/l1-bundle-child-final-verification.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-116-supervisor.md"
    - "docs/designs/issue-116-supervisor.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# L1 supervisor final verification（Full Gate・AC 証拠・IR・status 同期）

## Work

T1-T3 で着地した supervisor 実装の Full Gate・AC/R-V 証拠収集・IR・status 同期を行う closeout task。spec AC1-AC10 + R-V1–R-V6 の全行を evidence で確認し、実装の不整合・欠落があれば同 task で完結する fix を追加（新機能は追加しない — 欠落が大きい場合は前 task へ差し戻す）。

1. **Full Gate** — `mise run fmt-check` → `check` → `lint` → `test` → `test-rt` → `bench-smoke` を全 pass。`mise exec -- cargo doc -p ruviform-plugin-host --no-deps` が warning 0（新規公開 API doc — sole-drain WARNING 含む）。`mise exec -- prek run --all-files` も確認。opt-in lane `mise run lint-l1-real-plugin`・`mise run test-l1-real-plugin` を実行（wire/runner/adapter 無 diff で default gate 外だが、`ruviform-plugin-host` 公開面の additive 追加に対する共有面 sanity — AC10 完全性のため）
2. **focused E2E** — `cargo test -p ruviform-plugin-host --test l1_supervisor_e2e`（supervisor 全列）・`cargo test -p ruviform-plugin-host --test l1_proxy_e2e`（無退行 — `l1_proxy_e2e.rs` 無 diff green・R-V5）・`cargo test -p ruviform-plugin-host --lib`（supervisor unit test）
3. **AC1-AC10 + R-V1–R-V6 証拠 matrix** — spec `docs/specs/issue-116-supervisor.md` の各 AC + design Verification Notes の R-V 各行に対し、対応する test 名・実行結果を収集（plan の Verification Matrix を正本として使用）
4. **literal sweep 最終確認** — `git diff` で `crates/ruviform-plugin-host/src/composite.rs`・`src/l1/`・`src/host.rs`・`crates/ruviform-plugin-core/src/wire.rs` が無 diff（AC1/AC4 の機械的証明）・`supervisor.rs` が `crate::l1`/`pub(crate)` 内部型を参照しない・`grep -rn "supervisor\|Supervisor" crates/ruviform-plugin-core/ crates/ruviform-plugin-host/src/l1/ crates/ruviform-plugin-host/src/composite.rs crates/ruviform-plugin-export/` で core/l1/export への語彙混入なし・`tests/l1_supervisor_e2e.rs` が `#[path]`/`mod l1` を含まない（`pub(crate)` 非到達の compile-level 証明が維持されている）
5. **doc audit** — `node .agents/skills/doc-status/scripts/doc_status.js audit --json` が `blocking:0`・`findings:[]`
6. **IR + status 同期** — `docs/impl/ir/issue-116-supervisor-implementation.md` を作成（AC/R-V 証拠・task 完了記録・design 差分・fixture flag 追加・spec OPEN 行と design 確定値の照合 — `SupervisedId` 採用・`Err(Unsupported)` skip 確定・`RestartPolicy` 既定値 等）し、`docs/impl/ir/README.md` の curated index に登録。spec `status: draft → implemented`・各 task `todo → done`・plan `approved → completed`・test-spec `proposed → approved`（実装完了時の convention に従う）・`docs/specs/README.md` の status 列同期 + `docs/plans/README.md`・`docs/tasks/README.md`・`docs/test-specs/README.md` の index 行確認（planning 時点で作成済み — status 列を実装状態へ同期する場合のみ更新）
7. **flake 切り分け** — 既知の負荷依存 flake（`l1_proxy_e2e`）が出た場合は孤立 rerun で切り分け、結果を IR へ記録

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-116-supervisor.md](../plans/implement-issue-116-supervisor.md)（T4 — closeout）
- [../specs/issue-116-supervisor.md](../specs/issue-116-supervisor.md) AC1-AC10・R-V1–R-V6（証拠対象 — status 同期で `draft → implemented`）
- [../designs/issue-116-supervisor.md](../designs/issue-116-supervisor.md) Verification Notes（AC/R-V の oracle — fixture gap 行・`stable_window` drain pre-check 回帰 pin・adopt）
- [../adr/issue-116-supervisor-placement.md](../adr/issue-116-supervisor-placement.md)（util 層・core 無変更の最終確認）
- 先行 task: [l1-test-worker-fault-flags.md](l1-test-worker-fault-flags.md)・[l1-supervisor-core.md](l1-supervisor-core.md)・[l1-supervisor-e2e.md](l1-supervisor-e2e.md)

## Dependencies

- 先行タスク: T1（fixture flag）・T2（supervisor core）・T3（E2E）の全て — 本 task が収束点

## Affected Files

- `docs/impl/ir/issue-116-supervisor-implementation.md` — 新規 IR
- `docs/impl/ir/README.md` — IR の index 行登録（curated index — 全 IR が登録される）
- `docs/specs/issue-116-supervisor.md`・`docs/plans/implement-issue-116-supervisor.md`・4 task 文書・2 test-spec 文書 — status 同期（実装完了時）
- `docs/designs/issue-116-supervisor.md`・`docs/adr/issue-116-supervisor-placement.md`・`docs/discovery/issue-116-supervisor.md` — `updated:`-tick 簿記（design Verification Notes の checkbox 証拠化を含む — plan `changes.modified` が既に列挙する範囲）
- `docs/specs/README.md`・`docs/plans/README.md`・`docs/tasks/README.md`・`docs/test-specs/README.md` — status 列・index 行確認（planning 時点で作成済み）

## Done When

- [x] Full Gate（`fmt-check`/`check`/`lint`/`test`/`test-rt`/`bench-smoke`/`cargo doc`/`prek`/opt-in lane/doc audit）が全 pass（実績: 全コマンド初回 exit 0 — `test` 60 binaries・1294 passed・0 failed・`test-rt` 137 一致 0 failed・`cargo doc` は `RUSTDOCFLAGS="-D warnings"` 下で初回から warning 0・補正不要）
- [x] spec AC1-AC10 + R-V1–R-V6 の全行が test 名 + 実行結果で証拠化されている（IR coverage 表 — `docs/impl/ir/issue-116-supervisor-implementation.md`・各 AC/R-V へ test 名・file:line・pin 内容を記録）
- [x] literal sweep が残存なし — `composite.rs`/`l1/`/`host.rs`/`wire.rs` 無 diff・`supervisor.rs` が `pub(crate)`/`crate::l1` 非参照・`l1_supervisor_e2e.rs` が `#[path]` 非使用・core/export への supervisor 語彙混入なし（実績: merge-base `fc02103` 対比 4 file byte-level 無 diff・`supervisor.rs` の `use` は `lib.rs` 公開面のみ・`#[path]`/`mod l1` 0 hit・語彙 grep 0 hit）
- [x] IR が AC/R-V 証拠・task 完了・design 差分（spec OPEN → design 確定値の照合 — `SupervisedId` 採用・`Err(Unsupported)` skip 確定・`RestartPolicy` 既定値・`Orphaned` 追加・`--hang-after-msgs` 未使用/`--fail-stop-processing` 到達不能 deviation）を収録し `docs/impl/ir/README.md` index に登録されている（行追加済み）
- [x] spec/task/plan/test-spec の status が実装状態へ同期されている（spec `draft → implemented`・plan `approved → completed`・2 test-spec `proposed → approved`・4 task `todo → done`・design/ADR/discovery `updated:`-tick + design Verification Notes checkbox 証拠化・5 index 行同期）
- [x] `doc_status.js audit --json` が `blocking:0`・`findings:[]` を返す（実績: baseline 及び最終編集後とも `blocking:0`・`findings:[]`・392 documents）

## Verification

- [x] `mise run fmt-check`・`check`・`lint`・`test`・`test-rt`・`bench-smoke` が全 exit 0（実績: 全初回 pass — `test` 60 binaries・1294 passed・0 failed・`test-rt` 137 一致 0 failed・既知負荷依存 flake 未発生）
- [x] `mise run lint-l1-real-plugin`・`mise run test-l1-real-plugin` が pass（opt-in lane — `required-features = "host"` で default gate 外。host 公開面 additive の共有面 sanity）（実績: lint exit 0・test 3 段全 green — build → `--lib --bins`（CLAP 63+2/VST3 47+2）→ `--ignored`（CLAP 3+VST3 3・実子プロセス））
- [x] `mise exec -- cargo doc -p ruviform-plugin-host --no-deps` が warning 0（実績: `RUSTDOCFLAGS="-D warnings"` 下で初回から exit 0・warning 0 — 新規公開 API doc・sole-drain WARNING 全解決済み）
- [x] `mise exec -- prek run --all-files` が pass（exit 0・全 hook pass — `check illegal windows names` は no files で Skipped）
- [x] focused E2E（`l1_supervisor_e2e`・`l1_proxy_e2e`・`--lib` supervisor unit）が全 pass（実績: `l1_supervisor_e2e` 22/22・`l1_proxy_e2e` 100/100 無 diff green・`--lib supervisor` 33/33）
- [x] `node .agents/skills/doc-status/scripts/doc_status.js audit --json` が `blocking:0`・`findings:[]` を返す（baseline 及び最終編集後に実施）
- [x] spec AC1-AC10 + R-V1–R-V6 の証拠 matrix が完備されている（IR coverage 表）
