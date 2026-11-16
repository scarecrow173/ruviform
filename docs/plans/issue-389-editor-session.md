---
id: "PLAN-034YtAeTZbhyJdAJhfmsZp"
type: "plan"
status: "approved"
title: "Issue #389 editor-session real-session-terminate 実装計画"
created: "2026-11-16"
updated: "2026-11-16"
owners: []
relations:
  source:
    - "docs/specs/issue-389-editor-session.md"
    - "docs/designs/issue-389-editor-session.md"
  changes:
    added:
      - type: "doc"
        path: "docs/plans/issue-389-editor-session.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-389-editor-session.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-389-editor-session.md"
  blocks:
    - "docs/tasks/issue-389-editor-session.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-389-editor-session.md"
    - "docs/test-specs/issue-389-editor-session.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-389-editor-session.md"
    - "docs/designs/issue-389-editor-session.md"
  derived-by:
    - "docs/tasks/issue-389-editor-session.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-389-editor-session.md"
  references:
    - "evidence/rf389/"
  defers: []
  deferred-by: []
---

# Issue #389 editor-session real-session-terminate 実装計画 Implementation Plan

## Goal

`rf631.plugin-dock-real` catalog に `real-session-terminate` case を
1 行追加し、test_editor attach 中の `HostCommand::Destroy` →
`notify_terminated(HostLost)` → `Terminated` exactly-once → child
HWND teardown → host 存続を real OS window で判定する。

## Scope

単一実装ストリーム。`crates/ruviform-plugin-dock-test/` crate 内
（feature `real-dock` 配下の `real_dock.rs` + GPUI-free
`real_catalog.rs` + console test + doc 記述）に閉じる。
plugin-host/plugin-vst3/plugin-clap・`mise.toml`・
`Cargo.toml`/`Cargo.lock`・docs index は変更しない。

## Upstream Documents

- `docs/specs/issue-389-editor-session.md`
- `docs/designs/issue-389-editor-session.md`
- `docs/specs/issue-631-real-row.md`（catalog 構造の先例）

## Design Inputs

- `docs/designs/issue-389-editor-session.md`（approved）

## File Map

- `crates/ruviform-plugin-dock-test/src/real_catalog.rs` —
  `REAL_CATALOG` 末尾へ 1 行追加、module doc 追従。
- `crates/ruviform-plugin-dock-test/src/real_dock.rs` —
  `real_entries()` 13 entry 化、case 本体
  （`observe_session_terminate_lifecycle` + `session_terminate_body`
  + `real_session_terminate_case`）、`editor_attach_body` の
  verdict 写像を `editor_child_body` へ集約、blocked/test の
  行数追従、module doc・case 表追従。
- `crates/ruviform-plugin-dock-test/tests/console.rs` —
  `list_include_real_appends_real_lane_rows` の 21 行・期待 case
  一覧追従。
- `crates/ruviform-plugin-dock-test/src/lib.rs`、
  `src/bin/plugin-dock-console.rs`、
  `src/bin/plugin-dock-real-scenario.rs`、
  `src/bin/plugin-dock-lab.rs`、`README.md` — "twelve"/12 行
  記述と verdict 表の追従。
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp,
  discovery}/issue-389-editor-session.*` — doc chain。
- `evidence/rf389/` — lane 実走 JSONL・verification report。

## Implementation Sequence

- [ ] `real_catalog.rs`: `REAL_CATALOG` に `real-session-terminate`
  行を `real-editor-redock` の次へ追加し、module doc の
  "twelve" 記述を追従。
- [ ] `real_dock.rs`: `real_entries()` を `[...; 13]` 化し、同位置に
  新 entry を追加（spec extras は `editor_attach_extras` 同形）。
- [ ] `real_dock.rs`: `observe_session_terminate_lifecycle` /
  `session_terminate_body` / `real_session_terminate_case` を実装
  （observe_open seam で child HWND 記録 → Deactivate+Destroy →
  notify_terminated(HostLost) → Closed+concluded → events 列断言 →
  再 notify no-op → IsWindow(child)==0 → Scan 成功 + 破棄 handle
  InvalidHandle）。
- [ ] `real_dock.rs`: `editor_attach_body` を共有
  `editor_child_body`（observe + pass detail/evidence 引数）へ
  集約し、attach/terminate 両 case が同一 verdict 写像を使う。
- [ ] `real_dock.rs`: `blocked_reports`/`blocked_probe_violations`/
  `mod tests` の `[...; 12]`→`[...; 13]` と `reports.len()==12`→13
  を追従。
- [ ] `tests/console.rs`: 20→21 行・期待 case 一覧に
  `real-session-terminate` を追加。
- [ ] crate doc（`lib.rs`/bin 3 本/`README.md`）の行数記述・
  verdict 表を追従。
- [ ] `mise run check/lint/test ruviform-plugin-dock-test
  --features real-dock` で feature-gated test を green にする。
- [ ] `mise run test-ui-plugin-test-vst3` 相当の fixture build
  （`mise run build ruviform-plugin-test-vst3`）を実行し、
  headless blocked probe（`test-ui-plugin-dock-real-headless`）と
  console lane（`test-ui-plugin-dock-console`）を実走して証拠化。
- [ ] interactive 環境で `mise run test-ui-plugin-dock-real` を
  実走し、新 case を含む全 verdict を `evidence/rf389/` に記録
  （不可なら `NotRun`+`environment_blocked` を証拠化）。
- [ ] doc chain の impl/ir・impl/exp・exp.jsonl・discovery を
  実績で更新し、review-protocol を 1 cycle 実施。

## Dependencies and Risks

- `attempt_real_attach` teardown の 2 回目 `Destroy` は
  `InvalidHandle` で `let _ =` に吸収されることを実装で確認済み
  （`instance_mut` が stale handle を拒否）。`end_session` は
  `Closed` session に `Ok` を返す。
- instance は `Activated` のため `Destroy` 直打ちは lifecycle 上
  不正 — `Deactivate` を先行させる（headless `host-terminate`
  と同順）。Deactivate 失敗時も Destroy を独立に試行し、各々の
  結果を step row に記録する。
- host 存続 oracle は `HostCommand::Scan` 成功 + 破棄 handle への
  command が `HostError::InvalidHandle` で拒否されることで
  「command surface 生存」と「instance 死亡」を同時に観測する。
  第 2 instance activate は `plugin_id` が observe seam に渡らない
  ため採用しない。
- `real-dcomp-pixel` の既知 `Fail` は lane exit 1 の正直な
  regression 信号であり、本 slice で緩和しない。
- `docs/**/README.md` index は生成物 — Worker は編集・再生成・
  `index apply` を実行しない。Coordinator が統合時に再生成する。

## Task Breakdown

- T1: catalog/entry 追加と行数追従（real_catalog.rs +
  real_dock.rs entries/blocked/tests）。
- T2: case 本体実装（observe + body + verdict 写像集約）。
- T3: console test・crate doc・README 追従。
- T4: 検証 lane 実走と evidence/rf389/ 記録。
- T5: doc chain 完結（ir/exp/discovery）と review-protocol。

## Verification

- [ ] `mise run check ruviform-plugin-dock-test` /
  `lint` / `test`（default feature — headless 非回帰）。
- [ ] `mise run check/lint/test ruviform-plugin-dock-test
  --features real-dock`（本 slice の主変更面）。
- [ ] `mise run check/lint ruviform-plugin-dock-test
  --features lab-gui`（lab merged projection 非回帰）。
- [ ] `mise run build ruviform-plugin-test-vst3`（fixture cdylib）。
- [ ] `mise run test ruviform-plugin-host --test editor_session
  --test editor_session_driver`（消費 session 契約の非回帰）。
- [ ] `mise run test-ui-plugin-dock`（headless lane 8 case 非回帰）。
- [ ] `mise run test-ui-plugin-dock-real-headless`
  （`--blocked-probe` — 13 行 NotRun+environment_blocked・
  violation 0・exit 0）。
- [ ] `mise run test-ui-plugin-dock-real`（interactive 実走 —
  新 case verdict と environment_kind=real-window を記録、
  不可なら NotRun+environment_blocked を証拠化）。
- [ ] `mise run test-ui-plugin-dock-console`（`--list
  --include-real` 21 行・real-selector 拒否 exit 2）。
- [ ] `mise run check-ui-driver-boundary`（GPUI/Win32 型の
  漏洩なし）。
- [ ] `mise exec -- prek run --all-files`（prek 導入済みの場合）。
- [ ] `review-protocol` 1 cycle・blocker 0。

## Open Questions / Gaps

- なし（Coordinator 分析で設計確定済み。`Scan`+`InvalidHandle`
  oracle・Deactivate 先行・event 列
  `[Attached{Docked}, Terminated{HostLost}]` は実装で確認する）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are
  consistent.

## Implementation Handoff

- Recommended mode: inline execution（Worker worktree 内で
  単一ストリーム実行）。
- Independent work streams: none。
- User approval required before dispatch: no（review-protocol の
  blocker 0 を承認として扱う）。
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute
  task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same
  dependency graph and verification matrix.

## Review Handoff

- 前提: `attempt_real_attach` の teardown が observe 内 Destroy
  後も成立すること、`notify_terminated` の exactly-once が
  `by_instance` binding 除去で保証されること、event 列に
  `Closed` event が含まれないこと（graceful close 専用）。
- reviewer は catalog/entry の pairwise 一致・verdict 写像の
  共有・行数 assertion の追従漏れ・evidence の正直性を重点確認。
