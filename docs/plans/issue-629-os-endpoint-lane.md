---
id: "PLAN-034Zf5tX0ChfrY1d88CGQw"
type: "plan"
status: "approved"
title: "Issue #629 os endpoint wiring lane registration plan"
created: "2026-11-18"
updated: "2026-11-18"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-629-os-endpoint-lane.md"
    - "docs/designs/issue-629-os-endpoint-lane.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-629-os-endpoint-lane.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-629-os-endpoint-lane.md"
  implemented-by:
    - "docs/tasks/issue-629-os-endpoint-lane.md"
  depends-on:
    - "docs/specs/issue-629-os-endpoint-lane.md"
    - "docs/designs/issue-629-os-endpoint-lane.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-629-os-endpoint-lane.md"
    - "docs/test-specs/issue-629-os-endpoint-lane.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-629-os-endpoint-lane.md"
    - "docs/designs/issue-629-os-endpoint-lane.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-629-os-endpoint-lane.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 os endpoint wiring lane registration plan

## Goal

`os_endpoint.rs` の `#[ignore]` wiring leg 2 件に canonical
opt-in manual lane `test-os-endpoint-wiring` を登録し、stale
な「canonical lane 未登録」記述を同期して、未 provisioning
環境での fail-closed `NotRun` evidence を記録する。

## スコープ分割

単一実装ストリーム。lane 登録 + docs 同期 + self-run
evidence の一体 slice であり、`mise.toml` への task 追加・
docs 更新・evidence 記録・docs chain 作成をこの plan で
完結させる。

## Upstream Documents

- `docs/specs/issue-629-os-endpoint-lane.md`
- `docs/designs/issue-629-os-endpoint-lane.md`
- `docs/designs/issue-629-test-runtime-wiring.md`（reconcile
  対象 — merged IR の Follow-up を本 slice で消化）
- `docs/designs/issue-629-virtual-endpoint-lane.md` /
  `docs/designs/issue-629-provisioning-runbook.md` /
  `docs/discovery/issue-629-virtual-endpoint-survey.md`
  （append-only 追記対象）

## Design Inputs

- `docs/designs/issue-629-os-endpoint-lane.md`（approved）

## File Map

### 変更

- `mise.toml` — `[tasks.test-os-endpoint-wiring]` ブロック
  追加のみ（`test-midi-virtual-endpoint` 直後）。既存 task
  行は一切変更しない。
- `docs/development-environment.md` — lane 一覧表に
  `test-os-endpoint-wiring` 行追加 + opt-in lane 注記
  追記。
- `crates/ruviform-audio-test/tests/os_endpoint.rs` —
  module doc の stale 記述を `mise run
  test-os-endpoint-wiring` 参照に更新（doc のみ、挙動不
  変）。
- `docs/designs/issue-629-test-runtime-wiring.md` —
  canonical lane 未登録記述を reconcile。
- `docs/designs/issue-629-virtual-endpoint-lane.md` —
  dated 追記（append-only）。
- `docs/designs/issue-629-provisioning-runbook.md` —
  dated 追記（append-only）。
- `docs/discovery/issue-629-virtual-endpoint-survey.md` —
  Evidence 節に新規 dated entry（append-only、既存
  entry は編集しない）。

### 新規

- `docs/specs/issue-629-os-endpoint-lane.md`
- `docs/designs/issue-629-os-endpoint-lane.md`
- `docs/plans/issue-629-os-endpoint-lane.md`
- `docs/tasks/issue-629-os-endpoint-lane.md`
- `docs/test-specs/issue-629-os-endpoint-lane.md`
- `docs/impl/ir/issue-629-os-endpoint-lane.md`
- `docs/impl/exp/issue-629-os-endpoint-lane.md`
- `docs/impl/exp/issue-629-os-endpoint-lane-exp.jsonl`
- `evidence/rf629/os_endpoint_wiring_lane_selfrun-
  windows-local.txt`

## タスク分解

- [x] T1 `mise.toml` に `test-os-endpoint-wiring` task 追加
  — run は `cargo test -p ruviform-audio-test --test
  os_endpoint -- --ignored --test-threads=1`、description
  に provisioning 必須・manual・never CI・fail-closed
  NotRun・evidence 格納先を明記。
- [x] T2 `docs/development-environment.md` の lane 一覧表
  ・opt-in 注記を追記。
- [x] T3 `os_endpoint.rs` module doc の stale 記述更新 +
  `docs/designs/issue-629-test-runtime-wiring.md` の
  reconcile + 3 文書への dated 追記。
- [x] T4 lane self-run + 補助 `--nocapture` 実行の
  `evidence/rf629/` 記録。
- [x] T5 docs chain（spec→design→plan→task→test-spec→
  impl/ir→impl/exp+jsonl）作成。
- [x] T6 検証 matrix 実行（fmt-check/check/lint/test・
  lane self-run・既存 lane 退行確認・prek）。

## 依存関係

- T1〜T3 は並列可能だが同一 worktree のため逐次実施。
- T4 は T1 完了後（lane 登録が前提）。
- T5 は T1〜T4 の結果を impl/ir・exp に反映するため最後。
- T6 は全変更後。

## リスクと rollback

- `mise.toml` 誤記 → `prek run --all-files` の
  `check-toml` で TOML 検査 + lane self-run で検出。
  rollback は task ブロック削除のみ（既存 task 無変更の
  ため安全）。
- stale 記述の残存 → grep で「canonical lane がない」「後
  続 slice」等の残存を確認。
- Pass 誤記 → evidence・docs・PR で `verdict: NotRun`・
  `result:"not-run"` をそのまま記録し Pass 主張を明示的
  に否定。

## 検証 matrix

- [ ] `mise run fmt-check` — Pass。
- [ ] `mise run check ruviform-audio-test` — Pass。
- [ ] `mise run lint ruviform-audio-test` — Pass。
- [ ] `mise run test ruviform-audio-test` — Pass。
- [ ] `mise run test-os-endpoint-wiring` — exit 0 +
  `verdict: NotRun`（未 provisioning のため NotRun を
  記録、Pass 主張なし）。
- [ ] 補助証拠 `mise exec -- cargo test -p
  ruviform-audio-test --test os_endpoint -- --ignored
  --nocapture` — `run_metadata` JSON の語彙確認（補助証
  拠と明記）。
- [ ] `mise run test-audio-virtual-endpoint` /
  `mise run test-midi-virtual-endpoint` — exit 0（既存
  lane 非回帰）。
- [ ] `mise exec -- prek run --all-files` — Pass。
- [ ] MSRV lane — NotRun（daily CI 管轄）。

## Open Questions / Gaps

- なし。lane 名・run 文字列・配置・description 記法は
  Coordinator 決定 + 既存 opt-in lane 先例で確定済み。

## Self-Review

- [x] spec の全要件がタスクに対応する。
- [x] placeholder なし。
- [x] ファイルパス・relations・status 値・用語が一貫。

## Implementation Handoff

- Recommended mode: inline execution（lane 登録 + docs
  同期の一体 slice のため分割不要）。
- Independent work streams: なし。
- User approval required before dispatch: review-protocol
  レビューで blocker 0 を承認として扱う。

## Review Handoff

- レビュー観点: `mise.toml` が task 追加のみで既存行無変
  更であること、`os_endpoint.rs` が doc のみ変更である
  こと、append-only 文書の既存 entry が無編集であること、
  evidence が Pass を偽装していないこと。
