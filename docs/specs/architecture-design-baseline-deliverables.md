---
id: "SPEC-034INYy2aDLxse9dy8MgGZ"
type: "spec"
status: "proposed"
title: "Ruviform 設計ベースライン文書群の作成（アーキテクチャ・クレート構成・RT/オフライン実行モデル）"
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
    - "establish-rust-dev-foundation.md"
    - "replace-template-derived-docs-with-ruviform.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../../README.md"
    - "../designs/overview.md"
    - "../designs/dev-foundation.md"
    - "../discovery/explore-architecture-design-baseline.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/1"
  derived-by:
    - "../adr/licensing-and-distribution-policy.md"
    - "../adr/media-io-backend-abstraction.md"
    - "../adr/platform-and-toolchain-targets.md"
    - "../adr/plugin-architecture-clap-vst3.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/ui-architecture-webview-hybrid.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/2"
    - "https://github.com/scarecrow173/ruviform/issues/3"
  defers: []
  deferred-by: []
---

# Ruviform 設計ベースライン文書群の作成（アーキテクチャ・クレート構成・RT/オフライン実行モデル）

## What Should Be Built

GitHub Issue #1 の成果物として、Ruviform の現時点の構想を実装前にレビュー・分割・検証できる**設計ベースライン文書群**を作成する。本 spec が要求するのはアーキテクチャの実装やその具体的技術内容ではなく、「**Issue #1 が要求する設計ベースライン成果物**」——何を設計成果物として作り、どの基準を満たせば Issue #1 が完了とみなせるか——である。

作成する設計成果物（Issue #1「この Issue の成果物」に基づく）:

- `docs/architecture/` 等に配置する**実際の**アーキテクチャ文書と依存関係図（配置先は設計時に確定可）
- クレート単位の「責務／公開 API の境界／依存可能なレイヤー／RT 制約」の一覧
- リアルタイム処理とオフライン処理の具体的なデータフロー（それぞれ 1 例以上）
- CPU SIMD とスカラー実装の比較方法、音声ドロップアウト／レイテンシ／動画処理性能の検証計画
- 未決定事項の「決定／保留／要 PoC」分類と、必要な ADR・後続実装 Issue への分解
- Phase 1（math＋基礎データ型）→ Phase 2（音声／動画基礎）→ Phase 3（エンジン・描画・UI）の暫定実装ロードマップ
- `README.md`／`AGENTS.md` の再確認結果と、詳細設計に伴う差分の記録・更新タスク化

## Why Now

- 実施順序は **#2 → #3 → #1**。前提 Issue #2（mise/APM lockfile・Rust ツールチェーン・最小ワークスペース・手動 CI）と #3（README・AGENTS・APM の Ruviform 固有化）は完了・マージ済みであり、詳細設計の記述・検証基盤が揃っている。
- 現状、構想は Issue #1 本文・`README.md`・`AGENTS.md` の暫定原則に分散しており、最終的なクレート分割・公開 API 境界・RT 契約・対象プラットフォーム戦略等は未確定のままである。このまま実装に着手すると、層の責務・依存方向・RT 制約が暗黙の判断で固定され、後からレビュー・分割できない設計になるリスクがある。
- #2 で構築した基盤（初期クレート・ツールチェーン／MSRV・CI のプラットフォーム構成等）は**必要に応じて再検討できる暫定的な足場**であり、#1 の設計判断を制約しない。設計確定に伴い基盤へ反映すべき変更と移行コストを特定する必要がある。

## Users and Value

- **設計レビューア・第三者**: どの層が何を所有し、どの層に依存してよいかを説明できる文書を得る。
- **後続実装 Issue の実装者**: 参照すべき契約・境界・RT 制約と、分類済みの未決定事項を得て、実装前に前提を確認できる。
- **#2 開発基盤の保守者**: 設計決定に伴い基盤（`Cargo.toml`・`mise.toml`・CI 構成等）へ反映すべき変更と移行方針を特定できる。
- **設計者・AI エージェント**: 決定事項・保留・要 PoC の混在を防ぎ、設計議論の出発点と完了条件を共有できる。

## Scope

### In Scope

- `docs/` 配下へのアーキテクチャ文書群と依存関係図の作成（Issue #1 のレイヤー案を出発点とし、実際のクレート DAG を確定する）
- クレート単位の責務・非責務／公開 API の境界／依存可能なレイヤー／RT 制約の一覧化
- リアルタイム処理・オフライン処理それぞれのデータフロー例（スレッド境界・バッファ所有・初期化／再構成の所在を含む）を各 1 例以上
- CPU SIMD とスカラー実装の比較方法、音声ドロップアウト／レイテンシ／動画処理性能の検証計画
- 未決定事項の「決定／保留／要 PoC」分類と、必要な ADR・後続実装 Issue への分解
- Phase 1–3 の暫定実装ロードマップの精査・確定
- `README.md`／`AGENTS.md` の再確認、設計との差分記録、更新タスク化
- 設計判断に必要な小規模 PoC・検証コード、および設計に必要な範囲のリポジトリ設定・ドキュメント変更（#2 基盤への反映を含む。変更する場合は互換性と移行コストを明示する）

### Out of Scope

- 製品機能・クレートの本格実装（`ruviform-audio` 等のコード自体は作らない。設計の合意・文書化が Issue #1 の完了条件）
- ライセンスの最終決定（`LICENSE` 作成と `Cargo.toml` の `license = "MIT"` 仮置きの確定を含まない）
- UI 具体技術の選定確定（WebView 実装・描画バックエンド・フレームワーク等は PoC による検証に委ねる）
- mobile／組込み／wasm の初期対応（設計でこれらを排除はしないが、初期ターゲットには含めない）
- 個別 codec／container 形式・外部バックエンドのサポート確定（抽象境界の設計は対象内だが、個別形式の対応を約束しない）

## Requirements

### A. 設計成果物の要件

1. **アーキテクチャ文書と依存関係図**: Issue #1 のレイヤー案・クレート境界候補を出発点に、実際の依存 DAG（optional platform adapters の依存方向を含む）を確定して文書化する。基礎クレートから GUI・オーディオデバイス・GPU への強制依存を作らない。クレートを細かく分けること自体を目的化せず、公開 API・独立再利用・依存コスト・ライフサイクルを根拠に最終的な分割を決める。
2. **クレート一覧**: 設計対象の各クレートについて「責務・非責務／公開 API の境界／依存してよいレイヤー／RT 制約」を一覧化する。
3. **実行モデルのデータフロー**: RT 系統（コールバック駆動・事前確保・有界処理）とオフライン系統（バッチ・並列化・チャンク戦略）の具体的なデータフローをそれぞれ 1 例以上示し、スレッド境界・バッファ所有・初期化／処理グラフ再構成／codec 初期化が非 RT 区間に置かれる構造を明確にする。
4. **性能検証計画**: SIMD とスカラーリファレンス実装の比較方法（ベンチマーク・境界条件テスト・採否基準）と、音声ドロップアウト／レイテンシ／動画処理性能の検証計画を示す。
5. **未決定事項の分類と分解**: 未決定事項をすべて「決定／保留／要 PoC」に分類し、必要な ADR と後続実装 Issue へ分解する。「将来検討する」への先送りを行わない。
6. **暫定ロードマップ**: Phase 1（math＋基礎データ型）→ Phase 2（音声／動画基礎）→ Phase 3（エンジン・描画・UI）を精査した暫定実装ロードマップを示す。
7. **設計の具体化**: クレート DAG・責務・依存方向・公開 API 境界・データモデル・処理グラフ・DSP／SIMD・検証計画・ロードマップを設計案として具体化し、各項目を「確定／要 PoC／保留」に分類する。
8. **基盤との整合**: #2 の暫定的足場（初期クレート・MSRV・CI プラットフォーム構成等）を変更する場合は互換性と移行コストを明示し、開発基盤へ反映すべき変更を特定する。`README.md`／`AGENTS.md` を再確認し、設計との差分を記録して更新タスク化する。

### B. 設計が満たすべき確定済み制約（ユーザー承認済み決定）

以下は設計ベースラインが従うべき確定済みの制約である。逸脱が必要になった場合は ADR 等で差分と根拠を明示する。

#### B-1. ターゲットと提供物の優先順位

- 第 1 ターゲットは DAW／再生ホスト／プラグイン制作。基礎クレートはエンジン・ホストから独立して単独利用可能であること（**汎用ライブラリ性は絶対条件**）。
- 第 2 ターゲットは NLE／映像制作級（動画再生・同期 → 編集・コンポジット）。

（本項目は製品スコープの優先順位決定であり、選択肢分析を要するアーキテクチャ決定ではないため ADR 化しない。B-1 の内容自体は本節に正本として記録する）

#### B-2. メディア I/O の抽象境界

- すべてのメディア I/O はクレート内の抽象境界の背後にバックエンドを置く。
- 外部ライブラリの型・エラー・機能フラグを公開 API に露出しない。
- 純 Rust 実装（symphonia 等の利用を含む）も同じ境界を経由する差し替え可能な実装詳細とする。
- コアは純 Rust とし、FFmpeg 等の外部バックエンドは分離クレート＋opt-in feature とする。

#### B-3. プラットフォーム

- 対象 OS／CPU: Windows・macOS・Linux、x86_64＋aarch64。
- 基礎クレート（math・audio-core・video-core・DSP・IO）は GPU 非依存。`wgpu` はエンジン層の opt-in とする。
- stable Rust のみを使用する（nightly 前提の機能を必須としない）。
- モバイル・組込み・wasm は設計で排除しないが、初期ターゲット外とする。

#### B-4. リアルタイム（RT）契約

- RT 契約は Ruviform 内部だけでなく、アプリケーションや外部プラグインを含む RT クリティカルパス全体に適用する。ただしアプリケーション全体に制約は課さず、保証範囲は自身の RT 対応実装に限定する。
- RT クリティカルパスでは alloc／dealloc を禁止し、ブロッキング・I/O・無制限の待機／リトライ・非有界処理を制限する。
- RT／非 RT を API とライフサイクルで分離する。
- 検証可能な違反は CI で検出し、静的に判定不可能なものはドキュメント契約＋コードレビューで担保する。CI 成功は完全証明ではない旨を明記する。
- OS レベルのハードリアルタイム保証は提供しない。
- 詳細な API 設計・検証機構は design-doc で定義する。

#### B-5. プラグイン

- ホスト側とプラグイン制作側の双方が第 1 ターゲット（後段追加ではない）。
- CLAP／VST3 を同格とし、実装順序は技術的都合で決める（同時実装は必須としない）。
- 共通処理基盤と規格固有アダプターを分離する。
- RT 契約はホスト・プラグイン双方に適用する。
- Issue #1 でアーキテクチャ・契約・ABI 境界・ライフサイクル・分離方針を確定する。
- 映像プラグインは OFX が有力候補。対応範囲は別途設計する。
- プラグイン GUI 統合はホスト／制作双方で必須だが、全プラグインに独自 GUI を強制しない。
- VST3 SDK は 3.8 以降 MIT ライセンス（確認済み事実）である。

#### B-6. UI

- WebView ファースト・ハイブリッドを第 1 候補とする（HTML/CSS/JS・TS、フレームワーク不固定、PoC で検証）。
- 高頻度描画（波形・タイムライン・スペクトラム等）は DOM 更新に限定せず、WebView 内 Canvas2D／WebGL2／WebGPU・`wgpu` 等のネイティブ GPU・ネイティブサーフェス合成を検討する。ただしこれらを固定の実装順序や単一フォールバック階層にしない。
- 理想構造は、描画バックエンドに依存せず WebView 側から通常コンポーネントと同様に使えること。
- 描画方式とコンポーネントを分離する（公開 API／状態管理／描画データ生成 ↔ バックエンド）。波形ピーク・FFT 等の描画データは Rust 側で生成・キャッシュする。
- 全コンポーネントに共通抽象を強制しない。
- プラグイン GUI にも同一方針を適用する（外側は規格準拠のネイティブウィンドウ）。
- UI 基盤は DAW／ホスト／プラグイン制作で再利用できる構造とする。
- エンジンと UI を分離し、RT スレッドに UI 処理・ブロッキング同期を持ち込まない。
- Issue #1 では UI の責務・境界・スレッド契約・プラグイン GUI 統合方針を確定する。具体技術は PoC に委ねる。

#### B-7. ライセンス・公開方針

- ライセンスの正式決定は保留（MIT または MIT OR Apache-2.0 が有力）。`license = "MIT"` は仮置きであり、未決定であることを設計上も明確化する。
- Private 開発を継続し、公開可否はクレート単位で選択する。未決定クレートは原則 `publish = false` とする。
- 0.x を継続し、1.0 は Phase 番号ではなく成熟度で判断する。
- プラグイン ABI 適合性と内部 Rust API 安定性は分離して管理する。
- 依存のライセンス義務はクレート分離で解消しない。公開前にライセンスレビューを必須化する。

## Acceptance Criteria

Issue #1 の完了条件に基づく。

- [ ] どの層が何を所有し、どの層に依存してよいかを第三者が説明できる（アーキテクチャ文書・依存関係図・クレート一覧が存在し、責務と依存方向が一貫している）。
- [ ] RT／オフライン双方の利用例・制約とテスト方針が具体的に文書化されている（データフロー例が各 1 例以上、RT 制約一覧、性能検証計画を含む）。
- [ ] 決定事項と未決定事項が混在していない（すべての未決定事項が「決定／保留／要 PoC」に分類済み）。#2 で構築した開発基盤に反映すべき変更と、後続の実装 Issue が参照すべき事項が特定されている。
- [ ] 設計文書群が Requirements B の確定済み制約と整合している。逸脱する箇所がある場合は ADR 等で差分と根拠が明示されている。
- [ ] `README.md`／`AGENTS.md` を再確認し、詳細設計に伴い変更が必要な箇所を記録・更新タスク化している。
- [ ] 全クレートの実装を完了条件に含めない（設計の合意・文書化が完了条件である）。
- [ ] Phase 1→2→3 の暫定実装ロードマップが文書化され、Phase 間の依存関係と根拠が記述されている（Requirement A.6 に対応）

## Deferred Design Concerns

| 論点 | 本 spec での扱い | 判断・移管先 | 未対応のままのリスク |
| --- | --- | --- | --- |
| ライセンスの最終決定（MIT or MIT OR Apache-2.0、`LICENSE` 作成、`Cargo.toml` 仮置きの確定） | 未決定の明確化と公開前レビュー必須化まで | 配布方針の決定（公開前ライセンスレビュー） | 仮置き値が確定扱いのまま公開が進む |
| UI 具体技術（WebView 実装・描画バックエンド・フレームワーク） | 責務・境界・スレッド契約・プラグイン GUI 統合方針のみ確定 | 要 PoC 項目として設計で分類し、PoC 結果で決定 | 未検証の技術固定が後続実装を拘束する |
| mobile／組込み／wasm 対応 | 設計で排除しない方針を記録し、初期ターゲット外 | 初期ターゲット安定後の別途検討 | 排除前提の設計が将来の移植を阻害する |
| OFX 等の映像プラグイン規格の対応範囲 | 有力候補として記録し、範囲は確定しない | 第 2 ターゲット向けの別途設計 | 音声規格前提の抽象化が映像に不適合となる |
| 個別 codec／container・外部バックエンド（FFmpeg 等）の採否 | 抽象境界と分離クレート方針のみ設計し、個別対応を約束しない | 後続実装 Issue（メディア I/O 設計・採否判断） | 具体形式の未確定が I/O 境界設計を曖昧にする |
| RT 違反の CI 検出機構・検証ツールの実装 | 「検証可能な違反は CI 検出」の契約と設計方針のみ | design-doc で機構を定義し、後続実装 Issue で導入 | 契約が検証手段なしに形骸化する |
| RT 契約が適用される境界（アプリ・外部プラグインを含む範囲）の API ごとの保証定義 | 全体適用・保証範囲限定の原則のみ確定 | design-doc で API 単位の保証を定義 | 未確定 API を RT-safe と誤認して使用する |

## Sources and Relations

- 一次ソース: GitHub Issue [#1 [design] Ruviformの全体アーキテクチャ・クレート構成・RT/オフライン実行モデルを詳細設計する](https://github.com/scarecrow173/ruviform/issues/1)（`relations.source` / `relations.derives-from`）。目的・着手順序・レイヤー案・横断設計の契約・決定すべき事項・成果物・完了条件はすべてこの Issue を正本とする。
- 前提（`relations.depends-on`）: spec `SPEC-034IcwhwIFSNUNKtX7KK0Q`（[establish-rust-dev-foundation.md](./establish-rust-dev-foundation.md)、Issue #2、approved）と spec `SPEC-034Ivc5nj1AE6P2PcZU6Ki`（[replace-template-derived-docs-with-ruviform.md](./replace-template-derived-docs-with-ruviform.md)、Issue #3、approved）。Issue #1 は #2・#3 の完了を前提とし、それらの成果を詳細設計の記述・検証基盤として利用する。
- 関連 Issue（`relations.references`）: [#2](https://github.com/scarecrow173/ruviform/issues/2)・[#3](https://github.com/scarecrow173/ruviform/issues/3)。ともに完了済みで、#2 の暫定的足場（初期クレート・MSRV・CI 構成）は #1 で再検討可能、#3 の README/AGENTS は #1 完了後の差分記録・更新対象。
- 関連ドキュメント（`relations.related`）: リポジトリルートの [`AGENTS.md`](../../AGENTS.md)（暫定開発規約・RT 意図・#1 委譲事項の所在、再確認対象）、[`README.md`](../../README.md)（構想説明、再確認対象）、[`overview.md`](../designs/overview.md)（設計成果物の全体マップ）、[`dev-foundation.md`](../designs/dev-foundation.md)（#2 基盤の仮置き決定と根拠の記録）。
- 本 spec から派生した ADR（`relations.derived-by`）: Requirements B の確定済み制約を領域別に記録した 6 件の ADR——[メディア I/O バックエンド抽象](../adr/media-io-backend-abstraction.md)、[プラットフォーム・ツールチェーン対象](../adr/platform-and-toolchain-targets.md)、[RT 契約と検証方針](../adr/realtime-contract-and-verification-policy.md)、[プラグインアーキテクチャ（CLAP/VST3）](../adr/plugin-architecture-clap-vst3.md)、[UI アーキテクチャ（WebView ハイブリッド）](../adr/ui-architecture-webview-hybrid.md)、[ライセンス・配布方針](../adr/licensing-and-distribution-policy.md)。設計ベースライン文書群はこれらの決定と整合させる。
