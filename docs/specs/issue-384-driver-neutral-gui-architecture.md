---
id: "SPEC-034T6RWP3Vz7KWTjv6TILE"
type: "spec"
status: "approved"
title: "Driver-neutral GUI Architecture — GPUI標準Driver方針と3repo移行正本（Issue #384）"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/384"
    - "https://github.com/scarecrow173/ruviform/issues/257"
  changes:
    added:
      - type: "file"
        path: "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
      - type: "file"
        path: "docs/designs/issue-384-driver-neutral-gui-contract.md"
    modified:
      - type: "file"
        path: "docs/adr/ui-architecture-webview-hybrid.md"
      - type: "file"
        path: "docs/adr/ui-tech-selection-poc.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
    - "docs/designs/issue-384-driver-neutral-gui-contract.md"
    - "docs/plans/rf384-rf385-driver-neutral-gui-contract-ruviform-ui-theme-dtcg-interpreter-issue.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-385-dtcg-resolved-theme.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/384"
    - "https://github.com/scarecrow173/ruviform/issues/257"
  derived-by:
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
    - "docs/designs/issue-384-driver-neutral-gui-contract.md"
    - "docs/plans/rf384-rf385-driver-neutral-gui-contract-ruviform-ui-theme-dtcg-interpreter-issue.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/ruviform/issues/385"
    - "https://github.com/scarecrow173/ruviform/issues/386"
    - "https://github.com/scarecrow173/ruviform/issues/387"
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/390"
    - "https://github.com/scarecrow173/ruviform/issues/294"
  defers: []
  deferred-by: []
---

# Driver-neutral GUI Architecture — GPUI標準Driver方針と3repo移行正本（Issue #384）

## What Should Be Built

Issue #384（2026-11-07 横断レビュー確定仕様 v2）の要求どおり、GUI の構造正本を **Driver-neutral 契約層**へ移すための後継 ADR と契約詳細設計を作成する。GPUI は標準 Driver 実装の仮決定であり、共通契約（Design Contract/ResolvedTheme・Component & View Specs・Headless Behavior・Window/Dock/NativeSurfaceSlot/PluginEditorSession）は GPUI・DOM・wry・OS handle・第三者 Toolkit に依存しない。旧 2 ADR（`ui-architecture-webview-hybrid.md`・`ui-tech-selection-poc.md`）に明示的 supersession を付し、PoC 証拠は削除しない。

## Why Now

- 旧 2 ADR が `accepted` のまま残り、現行コードの採用方針として新方針と矛盾する。Issue 追記だけを ADR 切替完了としないため、リポジトリ上の正本を更新する必要がある。
- 後続実装 Issue（#385〜#390・Synvelis #175〜#177・AudioLens #166）が共通契約の正本を必要としており、初期契約を早期に共有して下流を解放する。
- GPUI 本体の version/MSRV/OS/license/transitive 採否・Native Dock 実現性は実機 PoC（#262/#386/#388/#389）の別 phase であり、「契約設計の開始」と「技術採用の最終承認」を分離して記録する必要がある。

## Users and Value

- #385〜#390 の実装 Owner と両 Consumer の GUI 実装者: 共通契約の正本・依存方向・移行表・Gate を一次ソースとして参照できる。
- Coordinator / reviewer: GPUI 採否が無条件の成功と宣言されていないことを、ADR の Confirmation と Gate 節で確認できる。

## Scope

### In Scope

- 後継 ADR（Driver-neutral GUI Architecture）の新設と、旧 2 ADR への `superseded` / `superseded-by` リンク。
- 依存方向図・旧 WebView API→新共通契約の移行表・DriverCapability Matrix（宣言テンプレート）・Native Dock 採用 Gate・旧 Frontend 縮退順序・#257 責務マップ・3repo 非 GPUI 境界の明文化。
- 契約詳細設計（`docs/designs/issue-384-driver-neutral-gui-contract.md`）で ID 体系・ライフサイクル・fixture 対応を固定。
- 旧 `ruviform-ui` の WebView 前提 API の棚卸し（移行表として。実コードの移動・削除は #386 以降の実装 Issue）。

### Out of Scope

- GPUI 技術採用の最終承認（#262 verdict・#388/#389 実機証拠後の別 phase）。
- GPUI Driver・Window/Host の実コード実装（#386 の所有範囲）。
- Component Renderer（#387）、Native Dock PoC（#388/#389）、Fake Driver/Lab（#390）、workflow（#294）の実装。
- 旧 WebView API・`ui/*` TS 資産の削除/移動（縮退順序 4 の別明示判断）。
- Synvelis 旧 SolidJS ADR の後継化（Synvelis #175、別 repo/別 Owner）。
- WASM 関連の一切。

## Requirements

- 後継 ADR は MADR 4.0.0 準拠の `docs/adr/` 正本とし、採用スキーマの `supersedes`/`superseded-by` relation を双方に付ける。
- 「契約設計開始」と「GPUI 技術採用最終承認」を ADR 内で分離し、未検証の能力を Pass と記さない。
- Native Dock PASS は GPUI main dock の実 layout/clip/tab/resize/input/focus 管理下で Plugin 自身の nativeEditor が動くことのみとし、Floating/Generic/fake/static・remote screenshot を代替にしない。L1 は現行 `HostError::Unsupported` の実測から診断し、黙って L0 へ退化させない。
- 移行表は現行 `ruviform-ui` の WebView 前提 API（`HostHandle::eval`・`WindowHandle::eval`・`HostEvent::Ready { url }`・`WindowConfig::url/devtools`・`HostEvent::Message(String)`・`RenderTier::*`・`NativeSurfaceHost`・`bridge::dispatch_script`）を網羅し、共通契約維持型（`WindowId`/`ViewInstanceId`/`DockWorkspace`/`DataRegistry`）と区別する。
- 基本 GUI 依存は GPUI 本体のみ。Toolkit 追加は Driver 内部・記録付きの選択導入。DTCG/画像/フォント/テストの汎用依存はこの制限の対象外。

## Acceptance Criteria

- [ ] 新 ADR が存在し、旧 2 ADR の status が `superseded`、双方に superseded-by/supersedes の明示リンクがある。旧 PoC 証拠・履歴が削除されていない。
- [ ] ADR に依存方向図・移行表・DriverCapability Matrix・Native Dock Gate・縮退順序・責務マップ（#257 整理）・3repo 境界が含まれ、#257/#385〜#390・Consumer（Synvelis #175〜177/AudioLens #166）へのリンクがある。
- [ ] GPUI 採否が #262 の OS/format/host 方式別 Pass/Fail/Unsupported/NotRun を根拠とし、無条件の成功と宣言されていない（ADR の最終採用は PoC 結果確認後の逆反映）。
- [ ] Native Dock の必須受入を Floating/Generic Editor で代替していない。
- [ ] 独立レビュー（review-protocol）で CONFIRMED P0/P1 blocker が残っていない。

## Deferred Design Concerns

- GPUI の具体 version・MSRV・OS・license・transitive 依存の最終採否 → #262 verdict 後に ADR へ逆反映。
- `ruviform-ui-gpui` クレート名・Driver 選択機構（build 時/起動時）の最終形 → #386 実装 Issue で確定。
- 共通契約の `ViewSpec`/Component 型の最終シグネチャ → #387。Plugin Session/Parent lease → #389。

## Sources and Relations

- Issue #384・#257・#385（2026-11-07 確定仕様 v2 が旧記述に優先）、#262・#386〜#390・#294・Synvelis #175〜#177・AudioLens #166。
- Supersede 対象: `docs/adr/ui-architecture-webview-hybrid.md`・`docs/adr/ui-tech-selection-poc.md`。
- 維持する論理契約: `docs/adr/issue-181-*.md`（multi-window）・`docs/adr/issue-182-*.md`（dock-tree）。
