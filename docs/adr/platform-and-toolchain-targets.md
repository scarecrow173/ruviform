---
id: "ADR-034IO01ZSFQcu2jEt2p0Tb"
type: "adr"
status: "accepted"
title: "対象プラットフォーム・CPUアーキテクチャ・ツールチェーン方針"
created: "2026-10-20"
updated: "2026-10-20"
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
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "rust-dev-foundation-toolchain-and-ci-profile.md"
    - "ui-architecture-webview-hybrid.md"
    - "media-io-backend-abstraction.md"
    - "licensing-and-distribution-policy.md"
    - "../discovery/explore-architecture-design-baseline.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/architecture-design-baseline-deliverables.md"
    - "https://github.com/scarecrow173/ruviform/issues/1"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://doc.rust-lang.org/nightly/rustc/platform-support.html"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# 対象プラットフォーム・CPUアーキテクチャ・ツールチェーン方針

## Context and Problem Statement

GitHub Issue #1（全体アーキテクチャ詳細設計）の確定決定として、対象 OS・CPU アーキテクチャ・ツールチェーンチャンネル・GPU 依存の配置を定める。Issue #2 の開発基盤（[ADR-034IcJdtqm6TP6MylyP5J7](rust-dev-foundation-toolchain-and-ci-profile.md)）では Rust `1.99.0`・Linux 中心 CI・手動ディスパッチという暫定的な足場を構築したが、対象プラットフォームの最終方針は Issue #1 で再検討することを前提としていた。対話の結果、デスクトップ 3 OS・x86_64+aarch64・stable-only・基礎クレートの GPU 非依存が確定した。対象 OS・CPU・ツールチェーン・GPU 依存・初期ターゲット外の扱いという相互に関係する決定を 1 つの ADR として記録する。

## Decision Drivers

* Ruviform は DAW/ホスト/プラグイン制作を含むデスクトップ用途を主軸とし、Windows・macOS・Linux のいずれも対象外にできない
* aarch64 は Apple Silicon のみならず ARM 版 Windows・Linux にも広がっており、x86_64 のみの対象では不足する
* stable-only は nightly 必須機能への依存を排除し、利用者・CI・リリースの再現性を安定させる
* 基礎クレートを GPU 非依存に保ち、wgpu 等の GPU 依存をエンジン層の opt-in に隔離する
* モバイル/組込/wasm を設計上排除しないが、初期検証・対応のコストを初期ターゲットに含めない

## Considered Options

**対象 OS:**

* Windows / macOS / Linux のデスクトップ 3 OS を対象とする
* Windows を第一対象とし、macOS/Linux は後追いとする
* モバイル OS を含む広範なターゲットを初期対象とする

**CPU アーキテクチャ:**

* x86_64 と aarch64 の双方を対象とする
* x86_64 のみを対象とする

**ツールチェーンチャンネル:**

* stable-only とし、nightly 必須機能を使用しない
* nightly 機能（portable SIMD 等）の使用を許容する

**GPU 依存の配置:**

* 基礎クレートを GPU 非依存とし、wgpu 等の GPU 依存はエンジン層の opt-in とする
* GPU 前提の設計として基礎クレートから GPU 型・依存を持つ

**モバイル/組込/wasm:**

* 設計上排除しないが初期ターゲット外とする
* 初期ターゲットに含める
* 設計上明示的に除外する

## Decision Outcome

以下の組み合わせを採用する。

* **対象 OS**: Windows / macOS / Linux のデスクトップ 3 OS を対象とする。Windows ファースト後追いは、プラグイン・DAW 用途で macOS が主要ターゲットである実態と、レイヤー構造を 1 OS 前提に固めてしまうリスクのため不採用。モバイル OS を含む初期対象は検証・維持コストに対し初期価値が薄いため不採用。
* **CPU**: x86_64 と aarch64 の双方を対象とする。x86_64 のみは Apple Silicon・ARM 系デスクトップを切り捨てるため不採用。SIMD 実装はスカラーリファレンスとランタイム分岐・フォールバックを**有力な前提仮説**とするが、採否・方式はベンチマーク基準の設計と比較評価（要 PoC）で確定する（Issue #1 の SIMD 採否方針と整合）。
* **ツールチェーン**: stable-only とし、nightly 必須機能（安定化前の portable SIMD 等）を使用しない。nightly 許容は、CI・利用者・リリースの再現性を nightly 変動に従属させ、Rust ツールチェーン pin 方針（[rust-dev-foundation-toolchain-and-ci-profile.md](rust-dev-foundation-toolchain-and-ci-profile.md)）の意義を損なうため不採用。MSRV・具体的 pin 版は [rust-dev-foundation-toolchain-and-ci-profile.md](rust-dev-foundation-toolchain-and-ci-profile.md) の仮置きを継続し、Issue #1 の検証で確定する。
* **GPU**: 基礎クレートを GPU 非依存とし、wgpu 等の GPU 依存はエンジン層の opt-in とする。GPU 前提設計は基礎クレート（math・audio-core・DSP 等）に GPU 型・依存を持ち込み、単独利用と移植性を損なうため不採用（[UIアーキテクチャ方針](ui-architecture-webview-hybrid.md)・[メディアI/Oバックエンドの抽象境界分離](media-io-backend-abstraction.md) の境界方針と整合）。
* **モバイル/組込/wasm**: 設計上排除しないが初期ターゲット外とする。明示的除外は将来の拡張を設計レベルで閉じるため不採用、初期ターゲット化は検証・対応コストを初期に割くため不採用。初期ターゲット外である（設計上は排除しない）ことを設計文書・README に明記する。

### Consequences

* 良い点: デスクトップ主用途の OS・CPU を網羅しつつ、stable-only で再現性を保てる。基礎クレートが GPU 非依存なため単独利用・移植・最小依存の構成が可能。モバイル/組込/wasm を排除しないため将来拡張の余地が残る。
* 悪い点: 3 OS・2 CPU の検証対象が広がり、CI・検証基盤の整備コストが上がる。aarch64 向け SIMD 実装・検証が必要。stable-only のため nightly 限定機能（portable SIMD 安定前の最適化等）は利用できない。

### Confirmation

下記 Verification のチェックリストを Issue #1 の成果物（アーキテクチャ文書・クレート一覧）と併せて確認する。対象 OS・CPU・stable-only・GPU 非依存の基礎クレート方針が設計文書に記述され、モバイル/組込/wasm が初期ターゲット外として明記されていることをレビューで確認する。CI のプラットフォーム構成・MSRV 確定は後続 Issue で検証する。

## Implementation Plan

* 影響するパス:
  * Issue #1 の成果物となるアーキテクチャ文書・クレート一覧（対象 OS・CPU・GPU 非依存・初期ターゲット外の記述先）
  * `Cargo.toml` / CI 設定（対象プラットフォーム・MSRV・stable-only 反映。具体設定は後続 Issue）
  * `docs/`（README・設計文書の対象プラットフォーム記述）
* 従うパターン: 基礎クレート（math・audio-core・video-core・DSP 等）を GPU・OS 固有 API に非依存の純 Rust として設計し、GPU・WebView・プラグイン SDK 等のプラットフォーム依存は上位アダプター/エンジン層の opt-in に隔離する。SIMD はスカラーリファレンス+ランタイム分岐+フォールバックを有力仮説として保持し、採否・方式のベンチマーク比較評価（要 PoC）が可能なようターゲット差をカプセル化する
* 避けるパターン: 基礎クレートに `wgpu`・WebView・OS 固有 API 依存を持ち込まない。nightly 必須機能（`#![feature(...)]`）を採用しない。モバイル/組込/wasm を「対応しない」と明示的に設計排除しない。1 OS 前提のパス・API 抽象化を基礎クレートに持ち込まない
* 依存関係: [rust-dev-foundation-toolchain-and-ci-profile.md](rust-dev-foundation-toolchain-and-ci-profile.md)（`related`、ツールチェーン pin・CI 基盤の暫定足場。MSRV・プラットフォーム構成は本 ADR の方針で再検討）、[UIアーキテクチャ方針](ui-architecture-webview-hybrid.md)・[メディアI/Oバックエンドの抽象境界分離](media-io-backend-abstraction.md)（境界 opt-in 方針と整合）

## Pros and Cons

**対象 OS — Windows ファースト後追い:**

* 良い点: 初期検証・実装が単純。
* 悪い点: macOS が主要ターゲットである実態に反し、1 OS 前提の抽象化が設計に残るリスク。

**対象 OS — モバイル含む広範ターゲット:**

* 良い点: 初期から広いカバレッジ。
* 悪い点: 検証・対応コストが初期価値に見合わない。DAW/プラグイン用途の主軸はデスクトップ。

**CPU — x86_64 のみ:**

* 良い点: SIMD 検証・CI 対象が半減。
* 悪い点: Apple Silicon・ARM 系デスクトップを切り捨て、後からの追加で ABI・最適化設計を見直す必要がある。

**ツールチェーン — nightly 許容:**

* 良い点: portable SIMD 等の最新最適化を早期利用できる。
* 悪い点: CI・利用者・リリースが nightly 変動に従属し、ツールチェーン pin の意義を損なう。安定版で表現できる最適化は stable で実装可能。

**GPU — 基礎クレート GPU 前提:**

* 良い点: GPU 型を基礎から使える設計が単純になる場合がある。
* 悪い点: 基礎クレートの単独利用・移植・最小依存を損ない、GPU 非対応環境で基礎機能すら使えなくなる。

**モバイル/組込/wasm — 明示的除外:**

* 良い点: 設計上の考慮が不要になり単純。
* 悪い点: 将来拡張を設計レベルで閉じ、後からの対応で API・ABI を見直すリスクが残る。

## Verification

* [ ] Issue #1 のアーキテクチャ文書に対象 OS（Windows/macOS/Linux）・CPU（x86_64+aarch64）が記載されている
* [ ] stable-only（nightly 必須機能不使用）が方針として明記され、基礎クレートに `#![feature(...)]` を採用しないことが確認できる
* [ ] 基礎クレートが GPU 非依存で、wgpu 等がエンジン層の opt-in として記述されている
* [ ] モバイル/組込/wasm が設計上排除しないが初期ターゲット外として記載されている
* [ ] SIMD のスカラーリファレンス+ランタイム分岐+フォールバックが有力仮説（採否・方式はベンチマーク比較評価で確定）として、ターゲット差カプセル化と併せて記述されている
* [ ] MSRV・pin 版が [rust-dev-foundation-toolchain-and-ci-profile.md](rust-dev-foundation-toolchain-and-ci-profile.md) の仮置きを継続し、Issue #1 での再検討対象として記録されている

## More Information

* 一次ソース: GitHub Issue [#1 [design] Ruviformの全体アーキテクチャ・クレート構成・RT/オフライン実行モデルを詳細設計する](https://github.com/scarecrow173/ruviform/issues/1)
* 上流ドキュメント: [SPEC-034INYy2aDLxse9dy8MgGZ](../specs/architecture-design-baseline-deliverables.md)（`relations.derives-from`）
* 関連 ADR: [開発基盤の toolchain・Cargoプロファイル・CI方針](rust-dev-foundation-toolchain-and-ci-profile.md)（`related`、暫定足場・pin 方針）、[UIアーキテクチャ方針](ui-architecture-webview-hybrid.md)（wgpu opt-in）、[メディアI/Oバックエンドの抽象境界分離](media-io-backend-abstraction.md)（基礎クレートの依存方針）、[ライセンス・公開方針](licensing-and-distribution-policy.md)（配布対象のクレート方針）
* 参考資料: [Rust Platform Support](https://doc.rust-lang.org/nightly/rustc/platform-support.html)（対象 OS・CPU の tier 確認起点）
