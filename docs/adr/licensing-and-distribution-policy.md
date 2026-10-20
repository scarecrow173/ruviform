---
id: "ADR-034IO02s3n3KHsxnHckpRn"
type: "adr"
status: "accepted"
title: "ライセンス・公開方針"
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
    - "replace-template-derived-docs-metadata-with-ruviform.md"
    - "media-io-backend-abstraction.md"
    - "plugin-architecture-clap-vst3.md"
    - "platform-and-toolchain-targets.md"
    - "../../AGENTS.md"
    - "../../Cargo.toml"
    - "../discovery/explore-architecture-design-baseline.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/architecture-design-baseline-deliverables.md"
    - "https://github.com/scarecrow173/ruviform/issues/1"
  derived-by:
    - "../designs/ruviform-architecture-design-baseline.md"
  verifies: []
  verified-by: []
  references:
    - "https://spdx.org/licenses/MIT.html"
    - "https://spdx.org/licenses/Apache-2.0.html"
    - "https://doc.rust-lang.org/cargo/reference/manifest.html#the-publish-field"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# ライセンス・公開方針

## Context and Problem Statement

GitHub Issue #1（全体アーキテクチャ詳細設計）の確定決定として、ライセンス・公開方針・バージョニングの扱いを定める。[ADR-034Ivm746Q38Wa21HeqUiI](replace-template-derived-docs-metadata-with-ruviform.md)（Issue #3）では `LICENSE` 不在・ライセンス未決定であることを明示し、`Cargo.toml` の `license = "MIT"` は仮置きとして扱う方針を決めたが、配布・公開の方針自体は未決定のままだった。対話の結果、ライセンスの正式決定は保留とし、Private 開発継続・クレート単位の公開可否選択・`publish = false` 原則・0.x 継続・外部依存のライセンス個別調査が確定した。ライセンス決定の時期・公開方針・バージョニング・外部依存のライセンス管理という相互に関係する決定を 1 つの ADR として記録する。

## Decision Drivers

* `LICENSE` ファイルが存在せず、ライセンス自体が未決定である（仮置きの `license = "MIT"` は配布方針の確定ではない）
* MIT 単独か MIT OR Apache-2.0（Rust エコシステム慣行）かは有力候補だが、配布条件・依存構成を検証せずに即時確定はできない
* クレート単位で公開可否を選べる構造にしないと、外部バックエンド（FFmpeg・プラグイン SDK 等）を含むクレートが公開方針を不当に拘束する
* プラグイン ABI 適合性と内部 Rust API 安定性は別物であり、一括りにすると互換維持の基準が曖昧になる
* 外部依存のライセンス・配布条件はクレート分離だけでは解消せず、個別調査と公開前レビューが必要
* Private 開発継続のまま、将来の GitHub/crates.io 公開を選択肢として残す

## Considered Options

**ライセンス決定の時期:**

* MIT 即時確定
* MIT OR Apache-2.0 即時確定
* 正式決定を保留し、`license = "MIT"` を仮置きとして未決定を明確化する

**公開方針:**

* Private 開発を継続し、将来の GitHub/crates.io 公開を選択肢として残し、クレート単位で公開可否を選択する
* 即時公開前提で構成を決める
* 公開方針を先送りし、構造を考慮しない

**未決定クレートの publish 方針:**

* 未決定クレートは `publish = false` を原則とする
* 全クレートを公開可能状態で維持する

**バージョニング:**

* 0.x 継続とし、1.0 は Phase 番号ではなく API 安定性・実装成熟度・互換維持準備で判断する
* Phase 番号連動で 1.0 を決める
* 初期から 1.0 を付ける

**外部依存のライセンス管理:**

* codec/FFmpeg/GUI バックエンド/プラグイン SDK 等の外部依存はライセンス・配布条件を個別調査し、クレート分離だけで法的義務が解消しないことを前提に公開前レビューを必須化する
* クレート分離でライセンス問題を解決済みとみなす

## Decision Outcome

以下の組み合わせを採用する。

* **ライセンス**: 正式決定を保留とし、MIT または MIT OR Apache-2.0 を有力候補とする。`Cargo.toml` の `license = "MIT"` は仮置きであり、ライセンス未決定を明確化する（[ADR-034Ivm746Q38Wa21HeqUiI](replace-template-derived-docs-metadata-with-ruviform.md) の方針を本 ADR で確定的な公開方針として精緻化する）。MIT 即時確定・デュアル即時確定は、依存構成・配布条件の検証なしに決定を既成事実化するため不採用。
* **公開方針**: Private 開発を継続し、将来の GitHub/crates.io 公開を選択肢として残す。クレート単位で公開可否を選択する。即時公開前提は現在の成熟度・未決定事項と不整合、公開方針の先送り（構造を考慮しない）は外部バックエンド・`publish` 制御を含むクレート構造を方針なしに設計することになり、後からの修正を困難にするため不採用。
* **publish 制御**: 未決定クレートは `publish = false` を原則とし、公開可否をクレート単位で選択する。0.x 継続とし、1.0 は Phase 番号ではなく API 安定性・実装成熟度・互換維持準備で判断する。Phase 番号連動・初期 1.0 は、実装成熟度と無関係に安定性を宣言するため不採用。
* **ABI と API の分離**: プラグイン ABI 適合性と内部 Rust API 安定性は分離して管理する。プラグイン ABI は規格互換・バイナリ互換の観点で管理し、内部 Rust API は semver で管理する。一括りにすると「API を変えたら ABI を破壊した」等の誤った対応付けを生むため分離する。
* **外部依存ライセンス**: codec/FFmpeg/GUI バックエンド/プラグイン SDK 等の外部依存はライセンス・配布条件を個別調査する。クレート分離だけで法的義務は解消しない（opt-in 化・分離はコード構造上の隔離であり、利用・配布・特許条件の法的義務を消すものではない）。公開前にライセンス・配布条件レビューを必須化する。

### Consequences

* 良い点: 未決定を未決定のまま明示しつつ、クレート構造・`publish` 制御・バージョニング基準を設計に組み込める。外部バックエンドのライセンス問題を構造上の隔離と個別調査で扱える。公開を将来の選択肢として残しつつ、後からの対応コストを抑えられる。
* 悪い点: ライセンス未決定の状態が継続し、`Cargo.toml` 仮置き値とドキュメントの不整合を維持管理する必要がある。公開前レビュー・個別調査の運用工数が残る。`publish = false` 原則は、公開を始める際に各クレートで解除判断が必要。

### Confirmation

下記 Verification のチェックリストを Issue #1 の成果物（アーキテクチャ文書・クレート一覧）と併せて確認する。`publish = false` 原則・バージョニング基準・外部依存の個別調査方針が設計文書に記述され、`LICENSE` 不在・`license = "MIT"` 仮置きが明記されていることをレビューで確認する。実際のライセンス決定・公開実行は本 ADR の対象外であり、公開前レビューを行う後続 Issue で検証する。

## Implementation Plan

* 影響するパス:
  * Issue #1 の成果物となるアーキテクチャ文書・クレート一覧（ライセンス未決定・公開可否・publish 方針・バージョニング基準の記述先）
  * `Cargo.toml` / 各クレートマニフェスト（`license` 仮置き注記・`publish` 設定。具体設定は後続 Issue）
  * `docs/`（README・設計文書のライセンス・公開方針記述）
* 従うパターン: 新規クレート作成時は未決定状態を前提に `publish = false` を検討し、公開を決める際はクレート単位で外部依存のライセンス・配布条件レビューを行う。プラグイン ABI の互換性は内部 API の semver とは別の記録・管理にする。外部依存を追加する際はライセンス・配布条件の調査記録を残す
* 避けるパターン: `LICENSE` 不在のまま `license = "MIT"` を決定済みとして扱わない。全クレートを一律公開可能としない。Phase 番号・実装進捗と無関係に 1.0 を宣言しない。プラグイン ABI 適合性を内部 Rust API の semver で代替しない。クレート分離で外部依存のライセンス・配布条件を「解決済み」とみなさない
* 依存関係: [replace-template-derived-docs-metadata-with-ruviform.md](replace-template-derived-docs-metadata-with-ruviform.md)（`related`、ライセンス未決定・仮置き注記の方針を前提とする）、[メディアI/Oバックエンドの抽象境界分離](media-io-backend-abstraction.md)・[プラグインアーキテクチャ方針](plugin-architecture-clap-vst3.md)・[対象プラットフォーム・ツールチェーン方針](platform-and-toolchain-targets.md)（外部依存・クレート構造の前提）

## Pros and Cons

**ライセンス — MIT 即時確定:**

* 良い点: 単純で `Cargo.toml` 仮置き値と一致する。
* 悪い点: 依存構成・配布条件・特許条件を未検証のまま決定を既成事実化する。

**ライセンス — MIT OR Apache-2.0 即時確定:**

* 良い点: Rust エコシステムの慣行に合う。
* 悪い点: 同上。デュアルの条件・依存整合を未検証のまま決め打ちになる。

**公開方針 — 即時公開前提:**

* 良い点: 公開向けの構造・命名・ドキュメントを最初から整備できる。
* 悪い点: 現在の成熟度・未決定事項と不整合で、公開品質を偽るリスク。

**公開方針 — 構造を考慮しない先送り:**

* 良い点: 決定自体を後回しにできる。
* 悪い点: 外部バックエンド・`publish`・命名の構造を方針なしに設計し、公開時の修正が困難になる。

**バージョニング — Phase 番号連動・初期 1.0:**

* 良い点: 進捗が版番号に見えやすい。
* 悪い点: 実装成熟度・互換維持準備と無関係に安定性を宣言し、API 破壊の基準を曖昧にする。

**外部依存ライセンス — クレート分離で解決済みとみなす:**

* 良い点: 管理が単純。
* 悪い点: opt-in 化・分離はコード構造上の隔離であり、利用・配布・特許条件の法的義務を消さない。公開時に未調査の義務が残る。

## Verification

* [ ] ライセンス正式決定が保留として記載され、MIT または MIT OR Apache-2.0 が有力候補・`license = "MIT"` が仮置きであることが設計文書・README に明記されている
* [ ] Private 開発継続・将来の公開を選択肢として残すこと・クレート単位の公開可否選択が記述されている
* [ ] 未決定クレートの `publish = false` 原則が記述され、公開時のレビュー手順が示されている
* [ ] 0.x 継続・1.0 は API 安定性・実装成熟度・互換維持準備で判断する基準が記載されている
* [ ] プラグイン ABI 適合性と内部 Rust API 安定性の分離管理が記述されている
* [ ] 外部依存のライセンス・配布条件を個別調査し、公開前レビューを必須化する方針が記載されている

## More Information

* 一次ソース: GitHub Issue [#1 [design] Ruviformの全体アーキテクチャ・クレート構成・RT/オフライン実行モデルを詳細設計する](https://github.com/scarecrow173/ruviform/issues/1)
* 上流ドキュメント: [SPEC-034INYy2aDLxse9dy8MgGZ](../specs/architecture-design-baseline-deliverables.md)（`relations.derives-from`）
* 関連 ADR: [テンプレート由来ドキュメント・識別子のRuviform固有化方針](replace-template-derived-docs-metadata-with-ruviform.md)（`related`、仮置き方針の前提）、[メディアI/Oバックエンドの抽象境界分離](media-io-backend-abstraction.md)、[プラグインアーキテクチャ方針](plugin-architecture-clap-vst3.md)、[対象プラットフォーム・ツールチェーン方針](platform-and-toolchain-targets.md)
* 関連ドキュメント: [AGENTS.md](../../AGENTS.md)（ライセンス注記の暫定原則）、[Cargo.toml](../../Cargo.toml)（`license = "MIT"` 仮置き）
* 参考資料: [SPDX MIT](https://spdx.org/licenses/MIT.html)、[SPDX Apache-2.0](https://spdx.org/licenses/Apache-2.0.html)、[Cargo publish field](https://doc.rust-lang.org/cargo/reference/manifest.html#the-publish-field)
