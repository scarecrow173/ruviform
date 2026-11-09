---
id: "SPEC-034UnwHc80Nd7iT5pKN2Eg"
type: "spec"
status: "approved"
title: "Issue #389 Plugin Editor Session hardening — bounded timeout・明示 degrade・GPU cause・DComp 正直記録"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/389"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-389-plugin-editor-session.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-388-native-surface-slot.md"
    - "docs/designs/issue-388-native-dock-windowing-slice3.md"
  refines:
    - "docs/specs/issue-389-plugin-editor-session.md"
  refined-by: []
  derives-from:
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "evidence/rf388-native-dock-windowing/report.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/262"
  defers: []
  deferred-by: []
metadata:
  issue: 389
---

# Issue #389 Plugin Editor Session hardening — bounded timeout・明示 degrade・GPU cause・DComp 正直記録

## What Should Be Built

`ruviform-plugin-host::editor_session`（PR #427 で導入された契約層）の
hardening slice。Issue #389 epic の残 DoD のうち**契約層で完結する 4 項目**
を実装・記録する:

1. **Wall-clock timeout 監視**: request ごとの bounded timeout を、
   RT 非依存の注入可能な clock port で実現する。deadline 超過は
   `Timeout` の失敗終端（`Failed{Timeout}` event）として観測可能にする。
2. **Floating の明示的 degrade 経路**: Dock 要求が `Unsupported` で
   拒否されたとき、session が fallback mode（`AttachMode::Floating`）へ
   **明示的に** degrade する経路を、通常 attach / Native Dock PASS と
   区別できる Result と終端 event で表現する。
3. **GPU host failure cause**: `SessionFailCause` に graphics /
   driver-lost 系の cause を追加し、GPU/driver 喪失を `HostLost` や
   `Internal` に潰さず区別できるようにする。
4. **DComp 制約の正直な記録**: PR #501（`evidence/rf388-native-dock-windowing/`）
   の実測発見 — GPUI 0.2.2 `CreateTargetForHwnd(topmost=true)` が child
   HWND を swapchain で覆い plugin pixel が合成されない — を、本 Issue の
   capability matrix / doc チェーンに `Unsupported` 行として明記し、
   pixel compositing の実測は `NotRun` と記録する。成功を偽装しない。

## Why Now

PR #427 の契約層は状態機械・request 世代・terminal-once・L0/L1 写像を
確立したが、`docs/designs/issue-389-plugin-editor-session.md` 改訂 1 の
R6 DoD 表が残課題を明示している: timeout の wall-clock 監視基盤は
「Deferred Design Concerns」に留まり、Floating degrade は「未達」、
GPU host 固有 cause は「未定義・未達」、DComp による pixel 非合成は
#388 側で実測済みだが #389 の capability 記述へ未反映。本 slice は
これらを契約語彙として確定させ、後続の実 adapter / 実機 slice が
「何ができて何ができないか」を偽らずに積み上げられる土台を作る。

## Users and Value

- GUI driver / composition root の実装者: deadline 監視・degrade・
  failure cause を契約として利用でき、独自の timeout/degrade 規約を
  発明しなくて済む。
- レビュア・受入担当: capability matrix が実測（DComp Unsupported）と
  未実施（NotRun）を正直に区別し、Native Dock PASS の水増しを防げる。

## Scope

### In Scope

- `editor_session` への deadline/clock 導入: 注入可能な
  `SessionClock` port、request deadline の arm/evaluate、期限切れ
  request の `Timeout` 終端化。fake clock による決定的テスト。
- session レベルの明示 degrade: preferred mode の `Unsupported` 拒否
  に対し fallback mode へ 1 回だけ retry し、`Degraded{mode}` event と
  別個の Result（`OpenOutcome`）で終端を表現。async 経路用の
  degrade 開始 primitive を含む。
- `SessionFailCause` / `EditorOpError` への graphics/driver-lost 系
  variant 追加（`#[non_exhaustive]` による additive 変更）と写像。
- capability matrix（本 spec の derived design）への DComp
  `Unsupported` 行・pixel compositing `NotRun` の明記、
  `evidence/rf389-session-hardening/` への検証記録。
- contract test の追加: fake-clock timeout（attach/close/recover/
  detach）、degrade の成功・非対象 error・fallback 失敗・async 経路、
  graphics-lost 写像。

### Out of Scope

- raw `GuiParentToken` の public API からの除去（`Plugin` trait /
  command 面の契約変更を伴う — 別 slice）。
- 実 Plugin/Format の L0/L1 実機受入・GUI hang 非波及の実機検証
  （`NotRun`/`Unsupported` として記録のみ）。
- macOS/Linux/CLAP/AU・複数 window・multi-plugin sweep（`NotRun`）。
- pixel compositing の修復自体（上流 GPUI/DComp 制約。
  `ruviform-ui-gpui` は本 slice の編集範囲外）。
- wire 動詞・`WIRE_VERSION`・L1 GUI の実装。
- `ruviform-poc-native-dock` 他 crate の変更（既存 API は additive
  のみで非回帰を維持）。

## Requirements

- deadline/timeout は全て非 RT（UI thread）専用であり、clock port は
  RT thread から呼ばれない。fake clock で時刻を制御した決定的テストが
  可能であること。
- timeout 超過は request を `Failed{Timeout}` へ終端化し、terminal
  event の重複・欠落を起こさない。期限前の完了・stale request は
  従来どおり `StaleRequest` で拒否される。
- degrade は「要求 mode が `Unsupported` で拒否され、fallback が
  要求と異なる」場合に限り 1 回だけ試行され、成功時は `Degraded`
  終端で `Attached` event を発火しない（PASS 偽装の禁止）。
- degrade は Native Dock の受入 PASS を代替しない — capability
  matrix と event 語彙の双方で区別可能であること。
- GPU/driver 喪失は専用 cause で表現され、`HostLost`（process/instance
  消失）や `Internal` と区別される。
- DComp 制約は「実測で `Unsupported`、pixel compositing は `NotRun`」
  として記録され、成功・部分成功のように見せない。

## Acceptance Criteria

- [ ] `SessionClock` port と deadline policy が `editor_session` に
  存在し、注入した fake clock で attach/close/recover/detach 各
  request の期限切れが `Failed{Timeout}` + `Failed` event に終端する
  contract test が green。
- [ ] `open_or_degrade`（仮称・design で確定）が preferred
  `Unsupported` → fallback 成功で `Degraded{mode}` 終端を返し、
  `Attached` event を発火しない。非 `Unsupported` error では retry
  せず、fallback 失敗時は `Failed` で終端する test が green。
- [ ] async 経路（`begin_*`/`complete_*`）でも degrade が
  `Degraded` 終端として表現できる。
- [ ] `SessionFailCause::GraphicsLost`（仮称）が追加され、
  `EditorOpError` 経由の写像と `notify_terminated` 経由の両方で
  到達可能である test が green。
- [ ] capability matrix に「GPUI/DComp native surface pixel
  compositing = Unsupported（実測）」「pixel compositing 実測 =
  NotRun」の行があり、`evidence/rf389-session-hardening/report.md`
  が検証結果と未実施項目を記録している。
- [ ] `mise run fmt-check`/`check`/`lint`/`test` が green。
- [ ] 既存 28 contract test・`ruviform-poc-native-dock` が非回帰
  （API は additive のみ）。

## Deferred Design Concerns

- 実 Native Dock / Floating window の実 adapter（GPUI floating
  surface、VST3 `IPlugView` の非 docked attach 等）— 別 Issue。
  degrade の fallback が実際に floating editor を出すのは adapter
  実装待ち。
- GUI runtime 側の timer/cancel 基盤との接続（driver が `tick` を
  呼ぶ周期の所有）— composition root 側の責務。
- L1 wire での degrade/timeout 通知 — issue-115 解除条件充足後。

## Sources and Relations

- 親 spec: `docs/specs/issue-389-plugin-editor-session.md`（approved）
- 親 design: `docs/designs/issue-389-plugin-editor-session.md`
  （改訂 1 R6 の未達項目が本 slice の対象）
- 実測根拠: `evidence/rf388-native-dock-windowing/report.md`
  （PR #501 — GPUI DComp `topmost=true` が child HWND を覆う実測）
- 共同 gate: Issue #388（NativeSurfaceSlot）、Issue #262（GPUI
  採用 gate）、Synvelis #176（製品側実機受入）
