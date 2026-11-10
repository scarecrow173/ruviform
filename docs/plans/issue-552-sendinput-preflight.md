---
id: "PLAN-034VehuU5KuL3vurAXEXph"
type: "plan"
status: "approved"
title: "Issue #552 — SendInput preflight 実装計画"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/552"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-552-sendinput-preflight.md"
      - type: "file"
        path: "docs/impl/ir/issue-552-sendinput-preflight.md"
      - type: "file"
        path: "evidence/rf552-sendinput-preflight/report.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/gpui_driver/os_input.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/examples/real_os_smoke.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/examples/real_os_pointer_smoke.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/examples/real_os_multiwindow_smoke.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-552-sendinput-preflight.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-552-sendinput-preflight.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-313-windows-realos-smoke.md"
    - "docs/plans/issue-313-windows-pointer-smoke.md"
    - "docs/plans/issue-243-windows-multiwindow-smoke.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-552-sendinput-preflight.md"
    - "docs/designs/issue-552-sendinput-preflight.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-552-sendinput-preflight.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #552 — SendInput preflight 実装計画

## Goal

`os_input` に注入→実効 read-back の preflight を追加し、3 つの
real-OS input lane が不通時に `environment_blocked`（exit 3）で
終了するようにする。

## Scope

単一実装ストリーム。`os_input` への API 追加と 3 example の配線を
1 タスクで行う（example 間の差分は scope 定数と verdict 関数の
追加のみで、分割するほどの独立性はない）。

## Upstream Documents

- `docs/specs/issue-552-sendinput-preflight.md`

## Design Inputs

- `docs/designs/issue-552-sendinput-preflight.md`（approved）
- `docs/test-specs/issue-552-sendinput-preflight.md`（approved）

## File Map

- `crates/ruviform-ui-gpui/src/gpui_driver/os_input.rs` —
  `PreflightScope`/`PreflightReport`/`PreflightCheck`/`preflight`、
  内部 `Probe` trait、pointer/keyboard probe、`probe_target`、
  `virtual_screen` 共通化、Windows/non-Windows `imp` の `OsProbe`、
  mock probe ユニットテスト。
- `crates/ruviform-ui-gpui/examples/real_os_smoke.rs` —
  keyboard preflight 呼び出し、`environment_blocked` verdict +
  exit 3、schema v2 + `preflight` フィールド。
- `crates/ruviform-ui-gpui/examples/real_os_pointer_smoke.rs` —
  pointer preflight、同上。
- `crates/ruviform-ui-gpui/examples/real_os_multiwindow_smoke.rs` —
  pointer+keyboard preflight、同上。
- `evidence/rf552-sendinput-preflight/` — 実走 verdict JSON と
  report.md。
- `docs/impl/ir/issue-552-sendinput-preflight.md` — 実装記録。

## Implementation Sequence

- [ ] `os_input.rs`: `Probe` trait + `OsProbe`（Windows: 既存関数
  委譲 + `inject_key`/`key_down_state`/`virtual_screen` 追加、
  non-Windows: unsupported 応答）を実装。
- [ ] `os_input.rs`: `probe_target`・`probe_pointer`・
  `probe_keyboard`・`preflight`/`preflight_inner`・公開型を実装。
- [ ] `os_input.rs`: mock probe ユニットテストを追加し
  `cargo test -p ruviform-ui-gpui --features gpui-test` で通す。
- [ ] 3 example に preflight 呼び出し・`environment_blocked`
  verdict（schema v2・`preflight` フィールド・exit 3）を配線。
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-ui-gpui`/
  `check-ui-driver-boundary` を通す。
- [ ] 対話 session で 3 lane（最低 `example-ui-gpui-real-os-smoke`
  と `example-ui-gpui-real-os-multiwindow-smoke`）を実行し、
  verdict JSON を `evidence/rf552-sendinput-preflight/` に保存。
  実行不能なら NotRun を記録。
- [ ] `docs/impl/ir/` 記録と index 再生成、最終 review。

## Dependencies and Risks

- `windows-sys` の `Win32_UI_Input_KeyboardAndMouse` feature は
  既に有効 — `GetAsyncKeyState`/`VK_F24` は追加依存なしで使える。
- `os_input` は `gpui` feature 配下 — ユニットテストは
  `gpui-test` feature の `cargo test` でのみコンパイルされる
  （既存構成どおり）。
- 実機 lane は対話 desktop session が必要 — Orca worktree の
  実行環境が非対話なら evidence は NotRun とする。

## Task Breakdown

- T1: `os_input` preflight API + probe + ユニットテスト。
- T2: 3 example 配線（verdict/exit code/schema）。
- T3: 検証 suite + 実機 evidence + IR。

## Verification

- [ ] `cargo test -p ruviform-ui-gpui --features gpui-test`（mock
  probe テスト含む）
- [ ] `mise run fmt-check` / `check` / `lint` / `test` /
  `test-ui-gpui` / `check-ui-driver-boundary`
- [ ] 実機 lane の verdict JSON（`environment_blocked` 分岐は
  自然発生時のみ実測、無ければ mock テスト + NotRun 記録）

## Open Questions / Gaps

- exit code 3 の「blocked 区分」既存慣行は無く新設（design で決定）。
- `environment_blocked` verdict の consumer-verify 写像は deferred
  （design の Deferred Design Concerns）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（Orca worker dispatch のため
  subagent 不使用）。
- Independent work streams: none（T1→T2→T3 は逐次）。
- User approval required before dispatch: yes → review-protocol
  レビューで代替（blocker 無しを承認とみなす）。

## Review Handoff

- `GetAsyncKeyState` が SendInput 注入キーの down 状態を反映する
  という前提は Windows の既知挙動だが、実機 preflight 通過で
  裏付ける。万が一反映されない場合 keyboard probe が常に
  blocked になるため、実機実行時に要確認。
