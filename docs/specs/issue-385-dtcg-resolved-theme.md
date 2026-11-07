---
id: "SPEC-034T6SaLC8Kz85jG9rbdsJ"
type: "spec"
status: "approved"
title: "DTCG Design Token Interpreter — backend-neutral ResolvedTheme/Theme Registry（Issue #385）"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/385"
    - "docs/specs/issue-384-driver-neutral-gui-architecture.md"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-theme"
      - type: "file"
        path: "crates/ruviform-ui-theme/tests/fixtures"
      - type: "file"
        path: "crates/ruviform-ui-theme/tests/pipeline.rs"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "crates/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-385-dtcg-resolved-theme.md"
  depends-on:
    - "docs/specs/issue-384-driver-neutral-gui-architecture.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-384-driver-neutral-gui-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/385"
    - "https://github.com/scarecrow173/ruviform/issues/384"
  derived-by:
    - "docs/designs/issue-385-dtcg-resolved-theme.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/386"
    - "https://github.com/scarecrow173/ruviform/issues/387"
    - "https://github.com/scarecrow173/ruviform/issues/390"
    - "https://www.designtokens.org/"
  defers: []
  deferred-by: []
---

# DTCG Design Token Interpreter — backend-neutral ResolvedTheme/Theme Registry（Issue #385）

## What Should Be Built

Git 管理の DTCG JSON を入力とし、`parse → schema/type validation → alias 解決（循環・存在検証） → theme/mode/consumer override merge → normalized typed ResolvedTheme` の**決定的パイプライン**を、Driver 非依存の Rust クレート `ruviform-ui-theme` として提供する。DTCG 解釈・Theme/asset/mode・reference/alias 解決の唯一の正本は本実装であり、GPUI・gpui-base/gpui-component/gpui-kit・Figma API への依存を持たない。

## Why Now

- Issue #385（2026-11-07 確定仕様 v2）で「DTCG → typed backend-neutral ResolvedTheme → Driver 専用 mapper」の二段階が必須化され、#386/#387 の Driver/Renderer 実装と Synvelis #177 が共通入力契約を待っている。
- Driver（GPUI mapper）や Toolkit の Theme/Palette 解釈を共通契約の第二正本にしない境界を、コードとして先に固定する必要がある。

## Users and Value

- #386 `ruviform-ui-gpui` の Theme mapper: `ResolvedTheme` を消費して GPUI 固有の Style へ一方向変換する。
- #387 Component/View spec: variant/state ごとの ResolvedTheme 値参照。
- Consumer（Synvelis/AudioLens）: Product/Theme override document のみで自社 Theme を構成でき、Driver 差替え時に Theme 定義を変更しない。
- Fake Driver/契約 fixture（#390）: 同一 ResolvedTheme fixture を Fake/GPUI 双方へ渡して semantic parity を検証する。

## Scope

### In Scope

- `ruviform-ui-theme` 新クレート: DTCG profile の採用範囲（型・参照・単位・theme/mode・alias・循環検出）の実装。
- 4 層 precedence（Primitive 基底 < 共通 Semantic < 共通 Component < Consumer Product/Theme 明示 override）の merge と、同一 token key・同一 mode scope に限定された override 規則。
- typed `ResolvedTheme` 不変 snapshot: version/mode/scale context/token source identity を保持。色は色空間/alpha、長さは単位/rounding、Font は family/weight/size/line-height/fallback を型で表す。
- エラー分類: type mismatch・alias cycle・未知参照・missing mode・未対応 token（schema version で扱いを明示）。
- Fixtures: valid/invalid/ref-cycle/type-mismatch/missing-mode/consumer-override の normalized Golden 出力と、テストでの deterministic 検証。
- 依存境界: Parser/Resolver/Theme Registry が Driver/GPUI/gpui-component なしで build・test できること。

### Out of Scope

- GPUI 専用 Theme mapper・`gpui::Hsla/Pixels/Style` への変換（#386 の Driver 内部成果物。本 crate は解決済み値を提供するだけ）。
- Component/View spec・Widget 実装（#387）。
- Figma API・REST・リアルタイム同期・Code Connect の実連携（明示的一方向 Import＋PR レビュー方針の記述のみ。Figma を実行時・ビルド時の必須依存にしない）。
- Consumer 固有 Theme の中身（各 Consumer Issue が所有）。
- WASM・ブラウザ経路。
- 旧 WebView `ui/components` の tokens.ts の即時撤去（縮退順序に従う。意味の移植のみ本範囲）。

## Requirements

- パイプラインは deterministic: 同一入力に対しバイト同一の normalized 出力。エラー報告は fail-fast（最初の 1 件）だが、走査順序は辞書順で固定のため同一入力には常に同一エラーが返る。
- 優先順位（初期規約）: Primitive 基底 < 共通 Semantic < 共通 Component < Consumer Product/Theme 明示 override。override は同一 token key と mode scope にのみ有効。型不一致・循環・未知参照はエラー。未定義の任意 token の扱いは schema version で明示（v1: 厳格 = 未知 `$type` はエラー、未知 `$extensions` は ignore）。
- `ResolvedTheme` は解決後の不変 snapshot として schema version・mode・scale context・token source identity を持つ。勝手な色置換・単位変換をしない。
- 色は色空間 + alpha、長さは単位 + rounding 規約、Font は family/weight/fallback を型で表す（size・line-height は `dimension`/`number` token の typed 値として保持し、複合 `typography` 型は v1 対象外）。`gpui::Hsla/Pixels/Style` 等を公開 API に露出しない。
- mode 欠落はエラー（missing mode）。Driver 側が alias や override 優先順位を独自判断しない。
- 公開 enum は `#[non_exhaustive]`（workspace 規約）。依存は serde + serde_json に限定（通常の依存審査: JSON 解析に必要・メンテナンス・license・依存木を確認済み）。
- `unsafe` なし・doc test は公開 API の例に使用。

## Acceptance Criteria

- [ ] `cargo build -p ruviform-ui-theme`（経由 `mise run` タスク）が GPUI/Driver 依存なしで成功し、crate の Cargo 依存が serde/serde_json に限定される。
- [ ] valid/invalid/ref-cycle/type-mismatch/missing-mode/consumer-override 各 fixture で deterministic な normalized 出力または分類済みエラーが得られる（`mise run test` green）。
- [ ] 同一 ResolvedTheme fixture が Fake Driver 向けと同一 semantic で参照でき、Consumer override が同一 key/mode のみに作用することをテストで検証。
- [ ] Theme 差分が業務数値・操作状態・Timing を変えない設計上の分離が design doc に記述される。
- [ ] `ResolvedTheme` の公開 API に GPUI/Toolkit/DOM/wry 型が存在しない（docs/dep 検査で確認）。
- [ ] README に DTCG profile の採用/対象外・schema migration・色空間/単位の変換規則・実行手順を記載。

## Deferred Design Concerns

- OS font missing・DPI scale・text metrics の mapper 側差分 → Driver capability/実機検証（#386/#390）。crate は未対応特性を Capability/Unsupported として表現できる設計に留める。
- 大規模 token 集合のビルド時 codegen（Rust 定数生成）→ 必要時の別 Issue。初期は実行時 resolve で十分。
- Figma Variables import ツール → 別 Issue（Import→validate→diff→review の手順のみ README に記述）。

## Sources and Relations

- Issue #385（DTCG 解釈・ResolvedTheme・fixtures 契約）、#384（Driver-neutral 構造・最小依存）、#257（共通 UI 正本）、#390（fixture/parity owner）。
- ADR: [driver-neutral-gui-gpui-driver-webview-first-issue-384.md](../adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md)。
- 契約詳細: [issue-384-driver-neutral-gui-contract.md](../designs/issue-384-driver-neutral-gui-contract.md)（crate 配置）。
- 実装設計: [issue-385-dtcg-resolved-theme.md](../designs/issue-385-dtcg-resolved-theme.md)。
- DTCG 形式: https://www.designtokens.org/ （Design Tokens Community Group format）。
