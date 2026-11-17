---
id: "TASK-034Zf8CVOiy6ouwq8fioMs"
type: "task"
status: "in-progress"
title: "Issue #629 os endpoint wiring lane registration tasks"
created: "2026-11-18"
updated: "2026-11-18"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/plans/issue-629-os-endpoint-lane.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-629-os-endpoint-lane.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-629-os-endpoint-lane.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-629-os-endpoint-lane.md"
    - "docs/specs/issue-629-os-endpoint-lane.md"
    - "docs/designs/issue-629-os-endpoint-lane.md"
    - "docs/designs/issue-629-test-runtime-wiring.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-629-os-endpoint-lane.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-629-os-endpoint-lane.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 os endpoint wiring lane registration tasks

- [x] T1: `mise.toml` に `[tasks.test-os-endpoint-wiring]`
  追加 — `cargo test -p ruviform-audio-test --test
  os_endpoint -- --ignored --test-threads=1`、description に
  provisioning 必須・manual・never CI・fail-closed
  `verdict: NotRun`・evidence 格納先 `evidence/rf629/` を
  明記。`test-midi-virtual-endpoint` 直後に配置、既存 task
  行は無変更。
- [x] T2: `docs/development-environment.md` の lane 一覧表
  に `test-os-endpoint-wiring` 行追加 + opt-in lane 注記
  追記（provisioned pair 要件・fail-closed NotRun・CI 対象
  外・専用 lint lane なし・evidence 格納先を明記）。
- [x] T3: stale 記述の同期 — `os_endpoint.rs` module doc
  を `mise run test-os-endpoint-wiring` 参照に更新（doc の
  み・挙動不変）、`docs/designs/issue-629-test-runtime-
  wiring.md` の canonical lane 未登録記述を reconcile、
  `docs/designs/issue-629-virtual-endpoint-lane.md`・
  `docs/designs/issue-629-provisioning-runbook.md`・
  `docs/discovery/issue-629-virtual-endpoint-survey.md`
  に dated 追記（append-only — 既存 entry 無編集）。
  merged IR `docs/impl/ir/issue-629-test-runtime-wiring.md`
  はリース外のため無変更。
- [x] T4: lane self-run + 補助 `--nocapture` 実行の
  `evidence/rf629/os_endpoint_wiring_lane_selfrun-
  windows-local.txt` 記録 — canonical = mise lane、直接
  cargo は補助証拠と明記。未 provisioning のため
  `verdict: NotRun`（`NOT_PROVISIONED` 系）を正直に記録、
  Pass を主張しない。
- [x] T5: docs chain 作成 — spec → design → plan →
  task → test-spec → impl/ir → impl/exp(+exp.jsonl)。
- [x] T6: 検証 matrix 実行 — `mise run fmt-check` /
  `mise run check ruviform-audio-test` / `mise run lint
  ruviform-audio-test` / `mise run test
  ruviform-audio-test` / `mise run test-os-endpoint-
  wiring` / 既存 lane 退行確認（`test-audio-virtual-
  endpoint`・`test-midi-virtual-endpoint`）/ `mise exec --
  prek run --all-files` — 全て Pass（self-run は
  `verdict: NotRun` の fail-closed を確認）。MSRV lane は
  NotRun（daily CI 管轄）。
- [x] T7: impl/ir・impl/exp(+jsonl) の最終記録 +
  review-protocol レビュー（blocker 0）。
- [ ] T8: commit・push・PR 作成・worker_done。
