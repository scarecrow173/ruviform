---
id: "ADR-034IO01BpGWCl1DcEbcTQI"
type: "adr"
status: "superseded"
title: "UIアーキテクチャ方針（WebViewファースト・ハイブリッド）"
created: "2026-10-20"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/1"
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
    - "realtime-contract-and-verification-policy.md"
  blocks: []
  supersedes: []
  superseded-by:
    - "driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
  related:
    - "plugin-architecture-clap-vst3.md"
    - "platform-and-toolchain-targets.md"
    - "../discovery/explore-architecture-design-baseline.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/architecture-design-baseline-deliverables.md"
    - "https://github.com/scarecrow173/ruviform/issues/1"
  derived-by:
    - "../designs/ruviform-architecture-design-baseline.md"
    - "docs/specs/plugin-gui-poc.md"
    - "docs/adr/plugin-gui-embedding.md"
  verifies: []
  verified-by: []
  references:
    - "https://wgpu.rs/"
    - "https://developer.mozilla.org/en-US/docs/Web/API/Canvas_API"
    - "https://developer.mozilla.org/en-US/docs/Web/API/WebGL_API"
    - "https://developer.mozilla.org/en-US/docs/Web/API/WebGPU_API"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# UIアーキテクチャ方針（WebViewファースト・ハイブリッド）

> **Status: superseded（2026-11-07）**。本 ADR の「WebView ファースト・ハイブリッド第一候補」は Issue #384 の 2026-11-07 横断レビュー確定仕様 v2 により **Driver-neutral GUI Architecture + GPUI 標準 Driver** 方針へ置き換わった。後継: [driver-neutral-gui-gpui-driver-webview-first-issue-384.md](driver-neutral-gui-gpui-driver-webview-first-issue-384.md)。本文書の PoC 記録・設計検討は履歴証拠として保持し、本文自体は改訂しない。

## Context and Problem Statement

GitHub Issue #1（全体アーキテクチャ詳細設計）の確定決定として、UI アーキテクチャの方向を定める。Ruviform は DAW/ホスト/プラグイン制作で再利用できる UI 基盤を構想し、GUI に WebView ベースの構成を有力候補としていたが、採用範囲・高頻度描画（波形・タイムライン・スペクトラム・スペクトログラム等）の方式・ネイティブ UI 部品との関係が未確定だった。対話の結果、WebView ファースト・ハイブリッドを第一候補とし、高頻度描画は DOM 更新に限定せず複数の高速描画方式を比較検討する方針が確定した。Issue #1 では UI の責務・境界・スレッド契約・プラグイン GUI 統合方針を確定し、wgpu 等の具体的な採用範囲は PoC 検証に委ねる。UI 基盤の方向・高頻度描画方式の扱い・描画とコンポーネントの分離・決定時期という相互に関係する決定を 1 つの ADR として記録する。

## Decision Drivers

* HTML/CSS/JS・TS による UI 記述力と開発効率（フロントエンドエコシステムの活用）
* 高頻度・低遅延描画（波形・タイムライン・スペクトラム・スペクトログラム等）には DOM 更新が性能上不十分になり得る
* 描画バックエンド非依存で WebView 側から通常コンポーネント同様に使える構造が理想（ネイティブサーフェスは DOM 要素でないため実現性は PoC 検証事項）
* エンジンと UI の分離、および RT スレッドへの UI 処理・ブロッキング同期の持ち込み禁止（RT 契約と整合）
* プラグイン GUI への同一方針適用（外側は規格準拠ネイティブウィンドウ/ビュー、内側は WebView 可）
* 技術確定を急がない（具体採用範囲は PoC で検証）

## Considered Options

**UI 基盤の方向:**

* WebView ファースト・ハイブリッド構成を第一候補とする（HTML/CSS/JS・TS、フレームワーク不固定、全面採用は PoC 検証）
* WebView を使わない純ネイティブ UI とする
* wgpu 等のネイティブ GPU 描画のみで UI を構築する

**高頻度描画の方式:**

* DOM 更新に限定せず、WebView 内 Canvas2D/WebGL2/WebGPU・wgpu 等ネイティブ GPU 描画・ネイティブサーフェス+WebView ハイブリッドを並行検討し、固定順序・単一フォールバック階層として扱わない
* 高頻度描画を含めて DOM/Canvas による Web 実装に一本化する
* wgpu 等のネイティブ GPU 描画に一本化する

**描画方式とコンポーネントの分離:**

* 描画方式をコンポーネントの公開 API・状態管理・描画データ生成から分離し、全コンポーネントに共通抽象を強制しない
* 全コンポーネントに共通の描画抽象を強制する

**決定時期:**

* Issue #1 で UI 責務・境界・スレッド契約・プラグイン GUI 統合方針を確定し、wgpu 等の具体採用範囲は PoC に委ねる
* UI 技術を即時確定する

## Decision Outcome

以下の組み合わせを採用する。

* **UI 基盤**: WebView ファースト・ハイブリッド構成を第一候補とする。記述技術は HTML/CSS/JS・TS でフロントエンドフレームワークは不固定とし、全面採用は PoC 検証を経て判断する。純ネイティブ UI は Web 技術の記述力・エコシステムを捨てるため不採用、ネイティブ GPU 描画のみは汎用 UI 構築の生産性とエコシステムの面で不採用。
* **高頻度描画**: DOM 更新に限定せず、高速描画方式を検討する。選択肢は WebView 内 Canvas2D/WebGL2/WebGPU、wgpu 等のネイティブ GPU 描画、ネイティブサーフェス+WebView ハイブリッドであり、固定順序・単一フォールバック階層として扱わない。DOM/Canvas 一本化は高頻度描画の性能要件を満たさない可能性が高く、ネイティブ GPU 一本化は WebView 内統合の柔軟性を失うため不採用。理想は描画バックエンド非依存で WebView 側から通常コンポーネント同様に使える構造とするが、ネイティブサーフェスは DOM 要素でないため実現性は PoC で検証する。
* **分離方針**: 描画方式とコンポーネントを分離する（公開 API/状態管理/描画データ生成 ↔ バックエンド）。Rust 側で波形ピーク・FFT 等の描画データを生成・キャッシュする。全コンポーネントに共通抽象を強制せず、コンポーネントごとに適した描画方式を選べる構造とする。共通抽象の強制は、コンポーネント間で要件の異なる描画方式を一律の抽象に押し込み、設計を過剰に複雑化するため不採用。
* **プラグイン GUI**: 同一方針を適用する。外側は規格準拠のネイティブウィンドウ/ビュー、内側は WebView 可とする（[プラグインアーキテクチャ方針](plugin-architecture-clap-vst3.md) と整合）。
* **再利用構造**: UI 基盤は DAW/ホスト/プラグイン制作で再利用できる構造とし、エンジンと UI を分離する。RT スレッドに UI 処理・ブロッキング同期を持ち込まない（[リアルタイム契約と検証方針](realtime-contract-and-verification-policy.md) に従う）。
* **Issue #1 での確定範囲**: UI の責務・境界・スレッド契約・プラグイン GUI 統合方針を Issue #1 で確定する。wgpu 等の具体採用範囲・フレームワーク・描画バックエンドの最終選択は PoC に委ねる。UI 技術の即時確定は検証なしに採用範囲を固定し、後からの PoC 結果を無効にするため不採用。

### Consequences

* 良い点: Web 技術の UI 記述力を得つつ、高頻度描画の性能要件を複数方式の比較で検証できる。描画バックエンドとコンポーネントの分離で、PoC 結果に応じて実装を差替えられる。プラグイン GUI・DAW/ホスト UI で方針を再利用できる。RT 契約との整合がスレッド契約として明確になる。
* 悪い点: 第一候補であって確定ではなく、wgpu 等の採用範囲・フレームワーク・配布形態は PoC 次第で未確定が残る。ネイティブサーフェス+WebView ハイブリッドは実現性が未検証で、理想構造が成立しない場合に設計の見直しが要る。複数描画方式の並行検討は検証工数を増やす。

### Confirmation

下記 Verification のチェックリストを Issue #1 の成果物（アーキテクチャ文書）と併せて確認する。UI 責務・境界・スレッド契約・プラグイン GUI 統合方針が設計文書に記述され、具体採用範囲が「要 PoC」として未確定管理されていることをレビューで確認する。実装方式の最終選択は本 ADR の対象外であり、PoC 後続 Issue で検証する。

## Implementation Plan

* 影響するパス:
  * UI 系クレート（仮称 `ruviform-ui`・`ruviform-gpu` 等。未作成・後続 Issue）
  * Issue #1 の成果物となるアーキテクチャ文書（UI 責務・境界・スレッド契約・プラグイン GUI 統合方針の記述先）
  * 高頻度描画方式の PoC 計画（後続 Issue）
* 従うパターン: UI コンポーネントの公開 API・状態管理・描画データ生成を描画バックエンドから分離する。波形ピーク・FFT 等の描画データは Rust 側で生成・キャッシュする。プラグイン GUI は外側を規格準拠ネイティブウィンドウ/ビュー、内側を WebView 可とする。RT スレッドと UI の間は非ブロッキングの受け渡しに留める
* 避けるパターン: 高頻度描画を DOM 更新だけで実装しない。描画方式をコンポーネントの公開 API に混入させない。全コンポーネントに共通描画抽象を強制しない。RT スレッドから UI 更新を同期的に呼ばない。wgpu・WebView 実装・フレームワークを本 ADR 段階で確定しない
* 依存関係: [リアルタイム契約と検証方針](realtime-contract-and-verification-policy.md)（`depends-on`、スレッド契約の前提）、[プラグインアーキテクチャ方針](plugin-architecture-clap-vst3.md)（プラグイン GUI 統合）、[対象プラットフォーム・ツールチェーン方針](platform-and-toolchain-targets.md)（wgpu のエンジン層 opt-in 方針）

## Pros and Cons

**UI 基盤 — 純ネイティブ UI:**

* 良い点: 各 OS のネイティブ描画に直結し、WebView ランタイム不要。
* 悪い点: HTML/CSS/JS エコシステムの記述力・資産を捨て、OS ごとの UI 実装が必要になる。DAW/プラグイン制作での再利用構造が難しい。

**UI 基盤 — ネイティブ GPU 描画のみ:**

* 良い点: 描画性能の上限が高く、WebView 依存を持たない。
* 悪い点: 汎用 UI（フォーム・メニュー・設定画面）を全て GPU で自前構築する必要があり、生産性・アクセシビリティ・エコシステムの面で不利。

**高頻度描画 — DOM/Canvas 一本化:**

* 良い点: Web 標準のみで完結し、ネイティブ描画との統合が不要。
* 悪い点: 波形・スペクトログラム等の高頻度更新で DOM/Canvas がボトルネックになり得る。性能要件を満たせない場合に退路がない。

**高頻度描画 — ネイティブ GPU 一本化:**

* 良い点: 描画性能の上限が高い。
* 悪い点: WebView 内コンポーネントとの混在・統合が難しく、UI 全体が GPU 描画に引きずられる。

**分離方針 — 全コンポーネント共通抽象の強制:**

* 良い点: 抽象が一本化され、学習コストが下がる可能性。
* 悪い点: 要件の異なる描画方式（DOM・WebGL・ネイティブサーフェス）を一律の抽象に押し込み、抽象が肥大して実装を歪める。

**決定時期 — UI 技術の即時確定:**

* 良い点: 設計が単純になり、PoC が不要。
* 悪い点: 未検証のまま採用範囲を固定し、性能・実現性のリスクを先送りする。理想構造（バックエンド非依存のコンポーネント利用）の実現性を確認しないまま決め打ちになる。

## Verification

* [ ] Issue #1 のアーキテクチャ文書に UI の責務・境界・スレッド契約・プラグイン GUI 統合方針が記述されている
* [ ] WebView ファースト・ハイブリッドが第一候補として記載され、全面採用が PoC 検証次第と明示されている
* [ ] 高頻度描画の選択肢（WebView 内 Canvas2D/WebGL2/WebGPU・wgpu 等ネイティブ GPU 描画・ネイティブサーフェス+WebView ハイブリッド）が列挙され、固定順序・単一フォールバック階層でないことが明記されている
* [ ] 描画方式とコンポーネント（公開 API/状態管理/描画データ生成）の分離、および Rust 側での描画データ生成・キャッシュ方針が記述されている
* [ ] 全コンポーネントへの共通描画抽象強制をしない旨、およびプラグイン GUI への同一方針適用が記載されている
* [ ] RT スレッドに UI 処理・ブロッキング同期を持ち込まない旨がスレッド契約として明記されている
* [ ] wgpu 等の具体採用範囲・フレームワークが「要 PoC」として未確定管理されている

## More Information

* 一次ソース: GitHub Issue [#1 [design] Ruviformの全体アーキテクチャ・クレート構成・RT/オフライン実行モデルを詳細設計する](https://github.com/scarecrow173/ruviform/issues/1)
* 上流ドキュメント: [SPEC-034INYy2aDLxse9dy8MgGZ](../specs/architecture-design-baseline-deliverables.md)（`relations.derives-from`）
* 関連 ADR: [リアルタイム契約と検証方針](realtime-contract-and-verification-policy.md)（`depends-on`）、[プラグインアーキテクチャ方針](plugin-architecture-clap-vst3.md)（プラグイン GUI）、[対象プラットフォーム・ツールチェーン方針](platform-and-toolchain-targets.md)（GPU 非依存の基礎クレート方針）
* 参考資料: [wgpu](https://wgpu.rs/)、[Canvas API](https://developer.mozilla.org/en-US/docs/Web/API/Canvas_API)、[WebGL](https://developer.mozilla.org/en-US/docs/Web/API/WebGL_API)、[WebGPU](https://developer.mozilla.org/en-US/docs/Web/API/WebGPU_API)（高頻度描画の検討対象）
