---
id: "DESIGN-034WZDvbXaGkQVl6nT6jRx"
type: "design"
status: "approved"
title: "Issue #532 DComp Native Dock 再開条件と方式 A/B/C 調査設計"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-532-dcomp-native-dock.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-532-dcomp-audit.md"
  implemented-by: []
  depends-on:
    - "docs/discovery/issue-532-dcomp-native-dock.md"
    - "docs/specs/issue-532-dcomp-audit.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-532-dcomp-native-dock.md"
    - "docs/designs/issue-388-native-dock-windowing-slice3.md"
    - "docs/test-specs/issue-388-389-native-dock-contract.md"
    - "evidence/rf388-native-dock-windowing/report.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-532-dcomp-native-dock.md"
    - "docs/specs/issue-532-dcomp-audit.md"
  derived-by:
    - "docs/plans/issue-532-dcomp-audit-plan.md"
    - "docs/test-specs/issue-532-dcomp-audit.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-532-dcomp-audit.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/synvelis/issues/176"
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 DComp Native Dock 再開条件と方式 A/B/C 調査設計

## Context

Issue #532 の DComp topmost 制約は実機で再現・記録済み（`evidence/rf388-native-dock-windowing/report.md`）だが、解消方式は未決定。本設計は L2/L3 利用要件発生時に実施する再開条件と方式 A/B/C の調査手順を、Issue 本文の「終了条件」「比較・選択」Step 1〜4 に基づいて固定する。**本設計は調査手順の記録であり、方式の採否・実装を決定しない**。

## Scope

### In Scope

- L2/L3 再開条件（対象 OS・GPU/driver・GPUI version・feature tuple・実 Plugin・Window/DPI・対応目的・測定環境・証拠・Owner）。
- Step 1（最小再現・root cause）→ Step 2（比較・選択）→ Step 3（契約維持）→ Step 4（判定）の手順。
- 方式 A/B/C の定義・比較軸・採用条件・却下条件。
- 契約維持条件（lease/epoch・attach→resize/clip/focus→detach・2-phase re-dock/rollback・private 境界）。

### Out of Scope

- 実機調査の実施・方式の採否判定・実装。
- macOS/Linux の合成修正（Issue 本文「対象外」）。
- Synvelis 製品 UI の変更・上流 GPUI へのパッチ提出運用。
- Floating/overlay/Generic Editor を Native Dock PASS として扱うこと。

## 再開条件（L2/L3 利用要件発生時）

| 条件 | 要件 |
| --- | --- |
| 対象 OS | Windows 11 x64 実機（実 display・実 GPU）。VM/Xvfb/リモート desktop は DComp 実経路の再現にならないため不可 |
| GPU/driver | 実 GPU + 実 driver。GPU/driver/DPI は証拠に記録 |
| Windows/GPUI version | Windows 11 + GPUI 0.2.2（crates.io pin）+ blade-graphics 0.7.1。GPUI 更新時は本監査を再実施 |
| Cargo feature tuple | `ruviform-poc-native-dock` の `gpui`（実 plugin 検証時 `vst3`）。headless feature は L2/L3 の代替にならない |
| 実 Plugin | 実 VST3/対象形式 binary（例: iZotope Vinyl 1.12.0）。in-process probe は機構確認用で実 plugin 画素の証拠ではない |
| Window/DPI | GPUI main window + child HWND の実配置。DPI 100%/150%/200%・マルチモニタ・2 Window を含む |
| 対応目的 | `NativeSurfaceSlot` 内への実 plugin editor 埋め込みで resize/clip/z-order/focus/input/re-dock が視覚的に成立 |
| 測定環境 | `RVF_POC_SCENARIO=dock-windowing`（`mise run poc-native-dock-windowing`）+ 識別色 child fixture + 実 plugin。画面キャプチャ・`GetWindowRgn`/`WindowFromPoint`/`GetFocus`/pixel readback を定量記録 |
| 期待/観測 | 画素表示・入力配送・focus 遷移・resize 追従・clip 適用・detach/再親化・re-dock rollback・crash/hang 復旧を Pass/Fail/NotRun で記録 |
| 証拠 | `evidence/rf532/` 配下に report + 生ログ + screenshot。SHA・環境 tuple・実行コマンドを provenance として記録 |
| Owner | #388（契約）・#389（Editor Session/L0-L1）・SV #176（製品 OS 受入）・#262（採用判定の入力） |

## 調査手順（Step 1〜4）

### Step 1: 最小再現と root cause

- Windows build・GPU/driver・GPUI revision・DPI・L0/L1・Plugin 形式/bitness・child HWND の parent/Z-order/clip・DComp target 設定・input/focus 状態を全て記録。
- 識別色の Win32 child HWND fixture と実 Plugin Editor の両方を `NativeSurfaceSlot` に埋め込む。
- Win32 単体親ウィンドウと GPUI Main Window の画素差を比較し、HWND 階層・attach/detach・画素が失われる最初の境界を特定する。
- `topmost=false`（方式 A の最小変更候補）が child HWND を DComp visual の背面に描画するか、あるいは child が引き続き覆われるかを fixture で確かめる（未検証の仮説）。

### Step 2: 比較・選択

- A/B/C を同一条件（同 OS/build/plugin/DPI/geometry/input sequence）で比較する。
- 比較軸: 上流修正/保守・Plugin 対応率・clip/resize/DPI・pointer/keyboard/focus・multi-window/re-dock・L0/L1 分離・hang/crash/rollback・CPU/GPU/frame-time。
- 実 Plugin が offscreen surface を提供する前提の方式は普遍性を実証するまで採用しない。
- Floating/overlay/Generic Editor を Native Dock 合格へ読み替えない。

### Step 3: 契約維持

- `NativeSurfaceSlot` の lease/epoch・attach→resize/clip/focus→detach・2-phase re-dock/rollback を変更しない。
- HWND/GPUI/DComp 型は `ruviform-ui-gpui` の private adapter 内に閉じ、RT callback・Consumer Domain に漏らさない。

### Step 4: 判定

- 採用方式で fixture と実 Plugin 双方を実窓検証し、能力ごとに Pass/Fail/Unsupported/NotRun を確定する。
- (i) 採用案の実画素 E2E 実証と #388 への handoff、または (ii) 能力非採用/Unsupported の正式決裁と代替 Owner/復帰条件を出す。
- 方式の選択だけでは画素 Full PASS としない。原因未切り分けを「解決」とは記録しない。

## 方式 A/B/C の定義と採否条件

| 方式 | 定義 | 採用条件 | 却下/保留条件 |
| --- | --- | --- | --- |
| A: GPUI 内最小修正 | GPUI 0.2.2 DComp target の設定・stacking 変更（例: `topmost=false`）。公開 API 範囲での実現可能性を最優先実証 | 識別色 child fixture と VST3/CLAP 実 GUI で画素表示・DComp present/clip/z-order・resize/tab-hide/re-dock・DPI/2Window/input/focus/popup/hang・復旧が成立。`topmost=false` 等の単一パラメータ変更は再現と全関連 Case を満たす場合のみ採用（断定禁止）。Perf は同 GPU/同 plugin/同 OS baseline で比較 | 公開 API で実現不能・z-order 逆転で plugin 画素が GPUI UI の下に隠れる・upstream 変更が大規模 |
| B: Windows private 合成 | `ruviform-ui-gpui` 内に閉じた Windows 限定の private compositor / native child attachment 経路。Driver 内部・upstream 改変最小 | A が不成立の場合の本命。契約を変更せず adapter 層で吸収し、HWND/GPUI/DComp 型を private に閉じられる。実 plugin 画素が視覚的に成立 | 実装・保守コストが過大・DComp 仕様の制約で実現不能・upstream private API への依存が避けられない |
| C: 代替 Native 経路 | 実 plugin editor 画素を保てる他の OS 公開 API/Host 連携（例: 別 top-level window・別合成経路） | A/B が不成立の場合の予備案。実 plugin 画素が視覚的に成立し、Dock としての操作性（resize/clip/focus/re-dock）が保たれる | Floating/overlay/スクリーンショット/Generic のみの場合は回避 UX であって Dock PASS ではない。実 plugin が offscreen surface を提供する前提の方式は普遍性を実証するまで採用しない |

## Components and Boundaries

- **調査実施者**: #388/#389 Owner（Ruviform 側の契約・adapter）+ SV #176 Owner（製品 OS 受入）。
- **判定受領者**: #262（GPUI 採用判定）・#419（後続 ADR/adoption）。
- **境界**: `NativeSurfaceSlot` 契約・`ruviform-plugin-host` の session 契約は変更しない。変更は `ruviform-ui-gpui` の private adapter 内に閉じる。

## Risks and Mitigations

- **方式 A の z-order 逆転**: `topmost=false` が child を背面に描く場合、plugin 画素が GPUI UI の下に隠れて実用上の Dock として成立しない可能性。Step 1 の fixture で早期に確認。
- **upstream 依存**: 方式 B が upstream gpui の private API/内部状態を必要とする場合、fork/patch が必要になり保守コストが増大。upstream の公開面を先に調査。
- **証拠の過大評価**: fake/headless/Float/Generic/overlay の成功を実 plugin 画素の証拠にしない。各 Case を Pass/Fail/NotRun で正直に記録。
