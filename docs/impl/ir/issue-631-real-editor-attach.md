---
id: "IMPL-034Y3oHi457Q1b3AfFnQ6T"
type: "impl"
status: "in-progress"
title: "Issue #631 real-editor-attach 実装記録"
created: "2026-11-15"
updated: "2026-11-16"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-631-real-editor-attach.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #631 real-editor-attach 実装記録

## Summary

`real-editor-attach` case を `test_editor` fixture の実 `IPlugView`
で `Unsupported` → `Pass` 化した。descriptor 選択を
`d.name == "test_editor"` に変更し silent fallback を撤去。
open 成功時に query-only Win32 観測（`EnumChildWindows` /
`GetParent` / `IsWindowVisible` / `IsWindow` /
`GetWindowThreadProcessId` / `GetCurrentThreadId`）で実 child
HWND の存在・parent・visibility・detach 後 `IsWindow==0`・
session 収束・events を `observed.steps` に記録し、全 check 合意
のみ `Pass` とする fail-closed verdict にした。

## Context

`docs/tasks/issue-631-real-editor-attach.md`（task chain:
spec `SPEC-034Y3nW74RKYWUkLfv81MG` → design `DESIGN-034Y3o3I0foDX68pAnpDEx`
→ plan `PLAN-034Y3odXzCYwJRpyjtWZgK` → test-spec
`TSPEC-034Y3opmTfAWO3534oKOoH`）に従う。前提 fixture は
`a6d57355`（PR #668、Issue #630 `test_editor` slice）で merge
済みの `ruviform-plugin-test-vst3` — `createView("editor")` が
実 `IPlugView` を返し、Windows `attached` が container HWND 配下に
`WS_CHILD | WS_VISIBLE` の実 child HWND を生成、`removed` で破棄
する。production host path（`Vst3Plugin::open_gui` /
`close_gui` → `PluginEditorSession`）は変更せず消費する。

## Implementation

### Changes

- `crates/ruviform-plugin-dock-test/Cargo.toml`
  - `[target.'cfg(windows)'.dependencies]` に optional
    `windows-sys = "0.61"`（`Win32_Foundation` /
    `Win32_System_Threading` /
    `Win32_UI_WindowsAndMessaging`）を追加し、`real-dock` feature
    に `dep:windows-sys` を接続。default feature・headless 依存木は
    不変。`ruviform-poc-native-dock` の先例に倣う。
  - `Cargo.lock` は optional dep 追加に伴う再生成のみ。
- `crates/ruviform-plugin-dock-test/src/real_dock.rs`
  - `load_plugin`: descriptor 選択を `d.name == "test_editor"` に
    変更。`.or_else(first())` fallback を撤去 — `test_editor`
    不在は `PluginUnavailable{prerequisite:false}` → `Fail`。
  - `observe_container_token` / `open_session_docked` helper を
    `attempt_real_attach` から抽出（clippy `too_many_lines` 対応）。
    `container_token` step を lease live 中に記録。
  - `oschild` module（`#[cfg(windows)]` / `#[cfg(not(windows))]`
    stub）: query-only Win32 観測。`#[expect(unsafe_code)]` +
    SAFETY コメント局所許可。window 生成・破棄・移動・message
    は一切行わない。観測値は `u64`/`u32`/`bool`/`Vec` に限定し
    Win32 型を公開面に出さない。
  - `observe_editor_child_lifecycle` + phase helper
    （`check_attached_children` / `close_editor_session` /
    `check_detached_children` / `check_session_converged` /
    `child_extras` / `record_obs_fail`）: open `Ok` 時のみ実行。
    `child_enumerated`・`child_contract`・`session.close`・
    `child_hwnd_detached`・`session_concluded`・`session_events`
    の各 check step を pass/fail/unsupported で記録。
    `thread_affinity` は verdict `note` の観測記録（check では
    ない — review-protocol 指摘を受け `pass` から修正）。
  - `ChildVerification`（`supported`/`verified`/`failure`/`extras`）
    が verdict を gate。`editor_attach_body` の verdict 分岐を更新:
    open `Ok` + 観測合意 → `Pass`、観測不一致 → `Fail`、
    `EditorOpError::Unsupported` → `Unsupported`（維持）、
    open `Ok` だが観測非対応 → `Unsupported`（非 Windows の
    正直な出口、実質未到達）。
- `crates/ruviform-plugin-dock-test/src/lib.rs` / `README.md`:
  real lane の `real-editor-attach` 行を実態に更新。headless lane
  表は不変。
- `evidence/rf631/plugin-dock-real-scenario.jsonl`: interactive
  Windows session での実走記録に更新（`real-editor-attach` =
  pass、全 OS 観測値入り）。
- `evidence/rf631/plugin-dock-scenario.jsonl`: headless lane 再実行
  — verdict 不変（差分は repo_sha/path/DLL digest の provenance のみ）。
- `evidence/rf631/verification.md`: 本 slice の検証節を追記。

### Decisions During Implementation

- **`test_editor` 不在は silent fallback せず `Fail`**: 旧来の
  `.or_else(first())` は editor 非搭載 fixture（`test_gain`）へ
  黙って後退し `Unsupported` に退化させるため撤去。artifact が
  存在するのに `test_editor` descriptor が無いのは
  present-but-broken として `prerequisite:false` → `Fail`。
- **観測は open `Ok` の後のみ**: `createView`/`isPlatformTypeSupported`/
  `attached` のいずれかが defined refusal した場合は観測対象の
  child が存在しないため、`Unsupported` 分岐を維持。
- **close は観測関数内で 1 回だけ**: accepted open は
  `observe_editor_child_lifecycle` 内で `session.close` 済み。
  その後の teardown は `close` を skip（2 回目は defined
  `InvalidState` no-op だが冗長）。refused/failed open は
  `editor_live=false` で `close` が no-op adapter call — 既存の
  rejecting redeemer 経路を維持。
- **thread-affinity は観測のみ**: `GetWindowThreadProcessId` で
  caller/container/child の owning thread id を記録するが
  verdict 入力にしない。scenario caller thread（17592）≠
  container pump thread（49084 = GPUI app thread）の逸脱は
  Issue #389 D1 からの既知の差異として `thread_affinity` step
  と detail に明記。production host では UI thread attach が前提。
- **events は `[Attached{Docked}, Closed]` 完全一致**: drain した
  event 列が想定と違う場合（degrade/failure/terminate 混入）は
  contract surprise として `Fail`。
- **`oschild` は query-only**: `EnumChildWindows`/`GetParent`/
  `IsWindow`/`IsWindowVisible`/`GetWindowThreadProcessId`/
  `GetCurrentThreadId` のみ。window lifecycle は production
  host/plugin 側が所有し、observer は一切変更しない。
- **clippy 対応の構造**: `too_many_lines`（`observe_editor_child_
  lifecycle` 237→分割、`attempt_real_attach` 129→helper 抽出）、
  `struct_excessive_bools`（`ChildWindowFacts` の bool 4 個を
  `ChildAttachFacts` に集約）、`trivially_copy_pass_by_ref`
  （`ParentSlotLease` は値渡し）、`if_not_else`（supported 分岐を
  positive first に）を解消。`#[allow]` 不使用。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-631-real-editor-attach.jsonl` — 実装中の
  判断記録（採用: descriptor 明示選択 + OS 観測分離、
  `windows-sys` optional dep の feature 閉じ込め）。

### Rejected

- なし（本 slice では代替実装の比較実験は未実施 — 設計で
  query-only observer + phase 分割を確定済み）。

## Validation

canonical = `mise` lane（全て exit 0）:

- `mise run fmt-check`
- `mise run check ruviform-plugin-dock-test`（default）
- `mise run lint ruviform-plugin-dock-test`（default, `-D warnings`）
- `mise run test ruviform-plugin-dock-test`（default — console
  14 tests + integration 4 tests 含む）
- `mise run check/lint/test ruviform-plugin-dock-test
  --features real-dock`（`--locked` 整合確認済み）
- `mise run test-ui-plugin-dock`（headless lane 非回帰 —
  `editor-attach` は `test_gain` のまま `unsupported`、verdict
  不変）
- `mise run check-ui-driver-boundary`（dep-graph/signature/
  allowlist pass — `windows-sys` 追加後も境界漏洩なし）
- `mise run test-ui-plugin-dock-real`（interactive Windows
  session 実走 — `real-editor-attach` = **pass**、実測値は
  `evidence/rf631/verification.md` 本 slice 節に記録）
- `mise exec -- prek run --all-files`（結果は PR 本文に記録）

## Risks

- **thread-affinity 逸脱は未解決の設計課題**: scenario は caller
  thread で `open`/`attached` を駆動するが、container HWND の
  pump は GPUI app thread にある。現状は観測記録のみ —
  production host での UI thread attach 保証は別 slice/設計課題。
- **child HWND input/resize/focus/re-dock は未検証**: 本 slice の
  観測は存在・parent・visibility・detach のみ。input 系は
  `IsWindow` gate を通る実 HWND に対する OS input 駆動が必要で
  後続 slice。
- **dock-pixels / DComp #532**: pixel・合成判定は未解決のまま —
  本 slice で Pass を主張しない。
- **非 Windows の観測 stub**: `oschild` non-Windows stub は
  `supported:false` を返すのみで到達不能（`NativePluginDock`
  capability が Windows-only）。万が一 accepted open が非
  Windows で起きた場合 `Unsupported` を記録する設計だが、
  実質デッドパス。

## Follow-ups

- `mise.toml` `test-ui-plugin-dock-real` description の
  `test_gain` 前提文言更新（本 Worker の lease 外 — 後続
  hygiene slice）。
- child HWND input/focus・resize negotiation・re-dock の Pass
  判定（Issue #631 残存 scope）。
- dock-pixels / DComp `scarecrow173/ruviform#532` 判定
  （未解決 — 本 slice では claim しない）。
- thread-affinity 契約（Issue #389 D1）への適合 — production
  host 側で UI thread attach を保証する設計が必要。
- `docs/*/README.md` index の再生成（doc-maintenance 手順で
  実施 — 本 record 確定後に同期）。
