---
id: "ADR-034IO00JjhxeTXUbCZtcDX"
type: "adr"
status: "accepted"
title: "リアルタイム契約と検証方針"
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
    - "plugin-architecture-clap-vst3.md"
    - "ui-architecture-webview-hybrid.md"
    - "media-io-backend-abstraction.md"
    - "../../AGENTS.md"
    - "../discovery/explore-architecture-design-baseline.md"
    - "../discovery/evaluate-plugin-binding-options.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/architecture-design-baseline-deliverables.md"
    - "https://github.com/scarecrow173/ruviform/issues/1"
  derived-by:
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../specs/implement-media-data-model-api.md"
    - "../plans/implement-issue-18-data-model.md"
    - "../specs/issue-18-followup-enum-and-time-fix.md"
    - "../plans/issue-18-followup-enum-and-time-fix.md"
    - "../plans/implement-rt-foundation.md"
    - "../specs/implement-video-foundation-api.md"
    - "video-proc-kernel-api-and-color-hdr-scope.md"
    - "../plans/implement-issue-26-video-foundation.md"
    - "../specs/implement-media-io-abstraction-and-first-backend-poc.md"
    - "../plans/issue-20-i-o-poc.md"
    - "../specs/implement-plugin-foundation.md"
    - "../plans/implement-plugin-foundation.md"
    - "../plans/implement-issue-58-time-type-extension.md"
    - "../test-specs/plugin-foundation.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# リアルタイム契約と検証方針

## Context and Problem Statement

GitHub Issue #1（全体アーキテクチャ詳細設計）の確定決定として、リアルタイム（RT）処理の契約適用範囲と検証方針を定める。Ruviform はオーディオコールバック駆動のリアルタイムパスとオフラインのバッチ処理を分離する設計であり、「RT クリティカルパスでは何を禁止し、その制約を誰に・どこまで適用し、どうやって遵守を確認するか」を決める必要がある。契約をコード上のどの境界に結び付け、自動検証と人的管理をどう分担し、保証範囲をどこまで主張するかは、API 設計・CI・ドキュメント・コードレビュー運用のすべてに波及する横断的決定である。契約の適用範囲、禁止事項の内容、RT/非 RT の分離方式、検証・強制のレベルという相互に関係する決定を 1 つの ADR として記録する。

## Decision Drivers

* RT クリティカルパスでのオーディオドロップアウト・グリッチの防止（allocation・ブロッキング・無制限待機が主因）
* 契約が Ruviform 自身の実装だけでなく、Ruviform を使うアプリケーションや外部プラグインの RT パスでも意味を持つこと（プラグイン API 境界を越えて RT パスが連続するため）
* 自動検証の可能性と限界（allocation 検出のような CI で検査できる違反と、ロック保持時間のような静的に検証しにくい制約の区別）
* 過剰な強制機構の回避（全 API にランタイム検査を入れるコストと複雑さ）
* 保証範囲の正直さ（CI 緑を「RT 安全の証明」と誤認させない。OS 全体のハードリアルタイム保証とは混同しない）

## Considered Options

**契約の適用範囲:**

* Ruviform 内部だけでなく、Ruviform を利用するアプリケーションや外部プラグインを含む RT クリティカルパス全体に適用する
* Ruviform 内部の RT API のみに適用する
* Ruviform を利用するアプリケーション全体に契約を課す

**禁止・制限事項の範囲:**

* alloc/dealloc を禁止し、ブロッキング操作・ファイル/ネットワーク I/O・無制限の待機/リトライ・実行時間が有界でない処理を制限する
* allocation のみを禁止し、他の操作はガイドライン程度に留める

**RT/非 RT の分離方式:**

* RT 処理と非 RT 処理を API およびライフサイクル上で分離する（別 API・別オブジェクト・別フェーズ）
* 同一 API 内で実行コンテキストに応じて内部で振る舞いを分岐させる

**検証・強制のレベル:**

* 検証可能な違反は CI で自動検出し、静的に検証できない制約はドキュメント契約 + コードレビューで管理する
* ドキュメント契約のみ・機械的検証なし
* ランタイム強制機構（RT コンテキストオブジェクト・全 RT API での実行時検査）を全面導入する

## Decision Outcome

以下の組み合わせを採用する。

* **適用範囲**: RT 契約は Ruviform 内部に限定せず、Ruviform を利用するアプリケーションや外部プラグインを含む RT クリティカルパス全体に適用する。ただしアプリケーション全体に制約を課すわけではなく、契約は「RT パスを構成する処理」にのみ効力を持つ。Ruviform の保証範囲は自身の RT 対応実装に限定し、利用側コードの RT 安全性を Ruviform が保証するわけではない。
* **禁止・制限事項**: RT クリティカルパスでは動的メモリ確保・解放（alloc/dealloc）を禁止し、ブロッキング操作・ファイル/ネットワーク I/O・無制限の待機/リトライ・実行時間が有界でない処理を制限する。初期化・グラフ再構成・メモリ再確保・codec 初期化等は非 RT 区間（制御スレッド）で実施する設計とする。
* **分離方式**: RT 処理と非 RT 処理を API およびライフサイクル上で分離する。「RT-safe」等の保証範囲は対象 API 単位で定義・明示し、同一 API の内部分岐にしない。
* **検証・強制**: 検証可能な違反（allocation 検出等）は CI で自動検出し、静的に検証できない制約はドキュメント契約 + コードレビューで管理する。CI のテスト成功を完全な RT 安全性の証明とみなさず、OS 全体のハードリアルタイム保証は行わない。ドキュメント契約のみの案は違反がレビュー漏れで混入しやすく不採用、ランタイム強制機構の全面導入は全 RT API に検査コストと複雑さを持ち込み見返りが限定的なため不採用。

### Consequences

* 良い点: 契約の効力範囲（RT パス全体）と保証主体（Ruviform は自身の実装のみ）が明確になり、利用側が誤解しない。自動検証できる部分と人的管理が必要な部分の責任分界が明文化される。過剰なランタイム検査を避けられる。
* 悪い点: 「検証可能な違反」と「ドキュメント契約に委ねる違反」の線引きを継続的に維持する必要があり、検証可能範囲の拡大には CI・テスト基盤の投資が要る。利用側への契約伝達はドキュメント品質に依存し、強制力は限定的。

### Confirmation

下記 Verification のチェックリストを、RT 対象 API の実装 Issue の受け入れ条件と併せて確認する。契約文言は API ドキュメント・設計文書で確認し、allocation 等の自動検出は CI ジョブの実在と実行結果で確認する。「CI 成功 ≠ RT 安全性の完全証明」であることを、設計文書・README の保証記述で確認する。

## Implementation Plan

* 影響するパス:
  * RT 対象 API を持つクレート（DSP・audio-core・engine 系、未作成・後続 Issue）
  * RT 契約のドキュメント（設計文書・API ドキュメント・AGENTS.md の暫定原則の確定版）
  * CI 検証ジョブ（allocation 検出等の RT 違反検査。具体的手法は後続 Issue で選定）
* 従うパターン: RT 対象と非 RT 対象を別 API・別オブジェクト・別ライフサイクルフェーズとして設計する。RT API の契約は API 単位で明示する。初期化・再構成・メモリ確保は非 RT 区間に配置する。ロックや I/O が必要な処理は RT パスの外に追い出す
* 避けるパターン: RT パス内で `Vec::push` の伸長・`Box`/`String` 確保・`Mutex::lock` のブロッキング取得・`std::fs`/`std::net`・無制限 `loop` リトライを行わない。同一関数内で `if rt { ... } else { ... }` の内部分岐にしない。「RT-safe」と断言できる保証がない API を RT 対応として宣伝しない
* 依存関係: 本 ADR は契約と検証方針を確定する。allocation 検出の具体手法（allocator 差替・検査用ビルド等）、RT スレッド構成、性能検証計画は後続 Issue で設計する

## Pros and Cons

**適用範囲 — Ruviform 内部のみ:**

* 良い点: 契約の責任範囲が単純。
* 悪い点: プラグイン API 境界を越える RT パスの契約が定義されず、Ruviform を通る外部コードの RT 違反を扱えない。

**適用範囲 — アプリケーション全体:**

* 良い点: 契約が全域で一貫する。
* 悪い点: Ruviform が利用側のアーキテクチャを不当に拘束し、非 RT 部分の自由を奪う。採用障壁が上がる。

**禁止範囲 — allocation のみ:**

* 良い点: 検証が単純で自動化しやすい。
* 悪い点: ブロッキング・I/O・無制限待機という現実の dropout 主因をカバーしない。

**検証 — ドキュメント契約のみ:**

* 良い点: 検証基盤の投資が不要。
* 悪い点: 違反がレビュー漏れで混入し、検出が実行時の症状（dropout）まで遅れる。

**検証 — ランタイム強制機構の全面導入:**

* 良い点: 違反を実行時に即検出できる可能性がある。
* 悪い点: 全 RT API に検査・コンテキスト伝達のコストと複雑さを持ち込み、検査機構自体の正当性も新たな問題になる。静的に検証できない制約は依然残る。

## Verification

* [ ] RT 契約（適用範囲・禁止事項・RT/非 RT 分離・保証範囲の限定）が設計文書に記述され、AGENTS.md の暫定原則と整合する
* [ ] RT 対象 API が API/ライフサイクル上で非 RT 処理と分離され、契約が API 単位で明示されている
* [ ] allocation 検出等、自動検証可能な違反を検査する CI ジョブが存在し、違反コードを含む変更で失敗することを確認できる
* [ ] 静的に検証できない制約がドキュメント契約として列挙され、コードレビュー観点に組み込まれている
* [ ] CI 成功を完全な RT 安全性証明とみなさない旨、および OS 全体のハード RT 保証を行わない旨が設計文書・README に明記されている

## More Information

* 一次ソース: GitHub Issue [#1 [design] Ruviformの全体アーキテクチャ・クレート構成・RT/オフライン実行モデルを詳細設計する](https://github.com/scarecrow173/ruviform/issues/1)
* 上流ドキュメント: [SPEC-034INYy2aDLxse9dy8MgGZ](../specs/architecture-design-baseline-deliverables.md)（`relations.derives-from`）
* 関連 ADR: [プラグインアーキテクチャ方針](plugin-architecture-clap-vst3.md)（ホスト・プラグイン双方の RT パスに適用）、[UIアーキテクチャ方針](ui-architecture-webview-hybrid.md)（RT スレッドに UI 処理・ブロッキング同期を持ち込まない）、[メディアI/Oバックエンドの抽象境界分離](media-io-backend-abstraction.md)（I/O 境界の RT 制約）
* 関連ドキュメント: [AGENTS.md](../../AGENTS.md)（RT/オフラインの暫定原則。本 ADR で確定版を定義）
