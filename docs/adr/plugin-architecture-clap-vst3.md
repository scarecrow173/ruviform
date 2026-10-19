---
id: "ADR-034IO01bG8AyjIJWxHeL77"
type: "adr"
status: "accepted"
title: "プラグインアーキテクチャ方針（ホスト・制作双方とCLAP/VST3同格）"
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
  depends-on:
    - "realtime-contract-and-verification-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
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
    - "https://github.com/free-audio/clap"
    - "https://steinbergmedia.github.io/vst3_dev_portal/"
    - "https://github.com/robbert-vdh/nih-plug"
    - "https://www.thefoundry.co.uk/products/ofx/"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# プラグインアーキテクチャ方針（ホスト・制作双方とCLAP/VST3同格）

## Context and Problem Statement

GitHub Issue #1（全体アーキテクチャ詳細設計）の確定決定として、外部プラグイン規格との連携アーキテクチャを定める。Ruviform は VST 等のプラグイン連携を検討対象としていたが、対応仕様・優先度・ホスト/プラグインのどちらを提供するかが未決定だった。対話の結果、提供範囲（ホスト機能とプラグイン制作機能の双方）、対応規格（CLAP と VST3 の同格）、実装構造（共通処理基盤と規格固有アダプターの分離）が確定した。Issue #1 では ABI 境界・ライフサイクル・分離方針の設計方針までを確定し、実装は後続 Issue とする。提供範囲・対応規格・実装構造・GUI 統合・映像プラグインという相互に関係する決定を 1 つの ADR として記録する。

## Decision Drivers

* プラグイン API 境界を越えて RT クリティカルパスが連続するため、RT 契約をホスト・プラグイン双方に適用する必要がある
* CLAP と VST3 は主要なデスクトップ音声プラグイン規格であり、どちらか一方への一本化は利用者層を狭める
* 規格固有の ABI・ライフサイクルと Ruviform 共通の処理基盤（バッファ・時刻・パラメータ・RT 契約）の分離
* nih-plug 等の既存フレームワークへの全面依存は、Ruviform 自身のアーキテクチャ決定（RT 契約・データモデル・バックエンド境界）を外部フレームワークの設計に従属させる
* VST3 SDK は 3.8 以降 MIT ライセンス（確認済み）で、ライセンス上の障壁は低い
* プラグイン GUI 統合はホスト側・制作側の双方向で必要だが、全プラグインが独自 GUI を持つわけではない

## Considered Options

**提供範囲:**

* ホスト機能（外部プラグインの読み込み・実行）とプラグイン制作機能（Ruviform 製プラグインの公開）の双方を第一ターゲットとする
* ホスト機能のみを対象とし、プラグイン制作は対象外とする
* プラグイン制作機能のみを対象とする

**対応規格:**

* CLAP と VST3 を同格サポートとし、実装順序は技術的依存・効率で決定する（同時実装は必須としない）
* 規格を一本化する（CLAP のみ、または VST3 のみ）
* AU / AAX / LV2 等を含む全主要規格を初期対象とする

**実装構造:**

* 共通処理基盤と規格固有アダプターを分離する（分離構造そのものが本 ADR の決定。「自前実装 vs 既存フレームワーク・binding クレート上への構築」という構築方式の採否は本 ADR の決定範囲外とし、後続評価に保留する）
* nih-plug 等の既存プラグインフレームワークに全面依存する
* 規格ごとに独立した実装を持つ

**プラグイン GUI 統合:**

* 双方向の GUI 統合（ホスト内へのプラグイン GUI 埋め込み・Ruviform 製プラグインの GUI 提供）を必須とし、独自 GUI の有無はプラグイン任意とする
* GUI 統合を対象外とする
* 全プラグインに独自 GUI を必須とする

## Decision Outcome

以下の組み合わせを採用する。

* **提供範囲**: ホスト機能とプラグイン制作機能の双方を第一ターゲットとする（後付け追加ではない）。ホストのみ・制作のみの案は、片側を前提に設計すると ABI・ライフサイクル契約が非対称になり、後からの追加で破壊的変更を招くため不採用。
* **対応規格**: CLAP と VST3 を同格サポートとする。実装順序は技術的依存・効率で決定し、同時実装を必須としない。規格一本化は対象ユーザーを狭めるため不採用、AU/AAX/LV2 等を含む全規格の初期対象は検証・維持コストに対し初期価値が薄いため不採用（規格固有アダプターの構造上、後から追加は妨げない）。
* **実装構造**: 共通処理基盤（バッファ・時刻・パラメータ・RT 契約・ライフサイクル調停）と規格固有アダプター（CLAP adapter / VST3 adapter）を分離する。共通基盤とアダプターの構築方式（自前実装 vs 既存フレームワーク・binding クレート上への構築）は本 ADR の決定範囲外とし、規格固有アダプター層での `clack` / `vst3` 等の binding クレート利用を妨げない。基盤構築方式の採否は design-doc / 後続 Issue の評価に委ねる。規格ごとの独立実装は共通基盤の重複を生むため不採用。
* **RT 契約**: ホスト・プラグイン双方の RT パスに [リアルタイム契約と検証方針](realtime-contract-and-verification-policy.md) を適用する。プラグイン API 境界を越える呼び出しも RT クリティカルパスの一部として扱う。
* **Issue #1 での確定範囲**: ABI 境界・ライフサイクル・分離方針の設計方針を Issue #1 で確定する。規格固有の実装・対応規格の実装順序・サンドボックス/クラッシュ分離の具体方式は後続 Issue とする。
* **GUI 統合**: 双方向の GUI 統合を必須とするが、全プラグインに独自 GUI を強制しない（規格が提供する汎用パラメータ表示で代替可能）。内側の実装は [UIアーキテクチャ方針](ui-architecture-webview-hybrid.md) に従う。
* **映像プラグイン**: OFX を有力候補とし、範囲・採否は別途設計する（本 ADR では確定しない）。
* **ライセンス前提**: VST3 SDK 3.8 以降は MIT ライセンス（確認済み）。各規格 SDK の配布条件の個別調査は [ライセンス・公開方針](licensing-and-distribution-policy.md) の手順に従う。

### Consequences

* 良い点: ホスト・制作の双方を前提に ABI・ライフサイクル契約を対称に設計でき、後付けの破壊的変更を避けられる。CLAP/VST3 同格で主要エコシステムをカバーしつつ、規格固有アダプター構造により他規格の追加経路を残せる。共通基盤と規格固有部の分離で、RT 契約・データモデルが Ruviform 主導のまま保たれる。
* 悪い点: 双方を第一ターゲットにするため初期設計範囲が広く、ABI・ライフサイクル・分離方針の確定まで要する。2 規格の同格サポートは検証・維持対象を増やす。基盤構築方式（自前実装 vs 既存フレームワーク・binding クレート上への構築）は未決定であり、分離構造の実装コスト評価は後続の方式選定に依存する。

### Confirmation

下記 Verification のチェックリストを Issue #1 の成果物（アーキテクチャ文書）と併せて確認する。ABI 境界・ライフサイクル・分離方針が設計文書として存在し、ホスト/制作・CLAP/VST3 の双方に対称に適用されることをレビューで確認する。実装は本 ADR の対象外であり、後続 Issue で検証する。

## Implementation Plan

* 影響するパス:
  * プラグイン系クレート（仮称 `ruviform-plugin-*`。未作成・後続 Issue）
  * Issue #1 の成果物となるアーキテクチャ文書（ABI 境界・ライフサイクル・分離方針の記述先）
  * RT 契約文書（プラグイン API 境界を越える RT パスの契約記述）
* 従うパターン: 規格固有部（CLAP/VST3 のエントリポイント・ABI 変換）を薄いアダプターとして分離し、共通処理基盤は規格非依存に保つ。プラグイン GUI は外側を規格準拠のネイティブウィンドウ/ビューとし、内側は UI アーキテクチャ方針に従う
* 避けるパターン: 共通基盤に規格固有の型・概念を持ち込まない。ホスト・制作の片側を前提にした非対称 ABI を設計しない。VST3 SDK のライセンス条件を未確認のまま依存に組み込まない（3.8 以降 MIT を前提に、利用版の条件を個別確認する）。なお基盤構築方式（自前実装 vs 既存フレームワーク・binding クレート上への構築）は未決定であり、本 ADR は全面依存を確定禁止しない（後続評価に委ねる）
* 依存関係: [リアルタイム契約と検証方針](realtime-contract-and-verification-policy.md)（`depends-on`、RT パス契約の前提）、[UIアーキテクチャ方針](ui-architecture-webview-hybrid.md)（プラグイン GUI の内側）、[ライセンス・公開方針](licensing-and-distribution-policy.md)（SDK ライセンス調査手順）、[メディアI/Oバックエンドの抽象境界分離](media-io-backend-abstraction.md)（外部依存を境界の背後に置く方針の適用）

## Pros and Cons

**提供範囲 — ホストのみ / 制作のみ:**

* 良い点: 初期設計範囲が狭く、片側に特化した単純な契約になる。
* 悪い点: 後からもう片側を追加すると ABI・ライフサイクル契約が非対称に拡張され、破壊的変更を招く。

**対応規格 — 規格一本化:**

* 良い点: 検証・維持対象が半減し、初期工数を抑えられる。
* 悪い点: 対象外規格のユーザー層を切り捨てる。後からの規格追加で共通基盤が規格前提を含んでいた場合に設計修正が要る。

**対応規格 — 全主要規格の初期対象:**

* 良い点: 初期から広いカバレッジ。
* 悪い点: AU/AAX/LV2 等の検証・維持コストが初期価値に見合わない。規格ごとの要件差が初期設計を過剰に複雑化する。

**実装構造 — nih-plug 等への全面依存:**

* 良い点: 成熟した実装・規格対応を即座に再利用できる。
* 悪い点: Ruviform の RT 契約・データモデル・設計主導権が外部フレームワークの設計に従属する。フレームワークの仕様変更が Ruviform の公開 API に波及する。

**実装構造 — 規格ごとの独立実装:**

* 良い点: 各規格に最適化した実装が可能。
* 悪い点: バッファ・時刻・パラメータ・RT 契約・ライフサイクル調停が規格ごとに重複し、同じドメインルールが複数箇所に分散する。

## Verification

* [ ] Issue #1 のアーキテクチャ文書に、プラグインの ABI 境界・ライフサイクル・分離方針が記述され、ホスト/制作・CLAP/VST3 の双方に対称に適用される
* [ ] 共通処理基盤と規格固有アダプターの分離構造が文書化され、共通基盤に規格固有の型・概念が含まれないことが確認できる
* [ ] プラグイン API 境界を越える RT パスに RT 契約が適用されることが契約文書に明記されている
* [ ] CLAP と VST3 が同格サポートとして記載され、実装順序が技術的依存・効率で決定される方針である
* [ ] プラグイン GUI 統合が双方向必須・独自 GUI 非強制として記載されている
* [ ] VST3 SDK のライセンス前提（3.8 以降 MIT）が記録され、利用版の条件を個別確認する手順が [ライセンス・公開方針](licensing-and-distribution-policy.md) と整合している
* [ ] 映像プラグインが「OFX 有力候補・範囲は別途設計」として未確定のまま管理されている

## More Information

* 一次ソース: GitHub Issue [#1 [design] Ruviformの全体アーキテクチャ・クレート構成・RT/オフライン実行モデルを詳細設計する](https://github.com/scarecrow173/ruviform/issues/1)
* 上流ドキュメント: [SPEC-034INYy2aDLxse9dy8MgGZ](../specs/architecture-design-baseline-deliverables.md)（`relations.derives-from`）
* 関連 ADR: [リアルタイム契約と検証方針](realtime-contract-and-verification-policy.md)（`depends-on`）、[UIアーキテクチャ方針](ui-architecture-webview-hybrid.md)（プラグイン GUI）、[メディアI/Oバックエンドの抽象境界分離](media-io-backend-abstraction.md)、[ライセンス・公開方針](licensing-and-distribution-policy.md)
* 参考資料: [CLAP](https://github.com/free-audio/clap)、[VST3 Developer Portal](https://steinbergmedia.github.io/vst3_dev_portal/)、[nih-plug](https://github.com/robbert-vdh/nih-plug)（参考実装）、[OFX](https://www.thefoundry.co.uk/products/ofx/)（映像プラグイン候補）
