---
id: "ADR-034INZsD434t5cQ7gBp7XN"
type: "adr"
status: "accepted"
title: "メディアI/Oバックエンドの抽象境界分離"
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
    - "realtime-contract-and-verification-policy.md"
    - "licensing-and-distribution-policy.md"
    - "platform-and-toolchain-targets.md"
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
    - "https://github.com/pdeljanov/Symphonia"
    - "https://ffmpeg.org/legal.html"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# メディアI/Oバックエンドの抽象境界分離

## Context and Problem Statement

GitHub Issue #1（全体アーキテクチャ詳細設計）の確定決定として、コンテナの demux/mux と codec の decode/encode を担うメディア I/O のバックエンド統合方式を定める。Ruviform は symphonia 等の純 Rust ライブラリや FFmpeg 等の外部エンジンの利用を有力候補とするが、外部クレートの型・エラー・機能フラグを公開 API に露出すると、バックエンドの交換・バージョンアップがそのまま公開 API の破壊的変更になる。また FFmpeg 系はライセンス（LGPL/GPL）・配布条件が純 Rust 依存と大きく異なり、コアクレートへの直接組み込みは配布方針を不当に拘束する。公開 API からの隔離、純 Rust 実装と外部バックエンドの扱い、外部バックエンドのクレート配置という相互に関係する決定を 1 つの ADR として記録する。

## Decision Drivers

* バックエンドの差替可能性（純 Rust 実装 ⇄ FFmpeg 等の外部エンジン、codec 単位の追加・除去）
* 公開 API の安定性（外部ライブラリの型・エラー・semver 変更をクレート API に波及させない）
* 外部依存のライセンス・配布条件の隔離（FFmpeg 等を必須依存にしない）
* コアクレートを純 Rust・依存最小に保つ（Issue #1 の基礎クレート方針と整合）
* コンテナ（demux/mux）と codec（decode/encode）の概念分離（Issue #1 の横断設計要求）

## Considered Options

**バックエンド統合方式:**

* クレート内の抽象境界の背後に全メディア I/O バックエンドを配置する
* 外部クレート（symphonia / ffmpeg-next 等）へ直接依存し、その型・エラーを公開 API に露出する
* GStreamer 等の外部メディアエンジンへメディア処理を全面委譲する

**純 Rust 実装の扱い:**

* 純 Rust 実装（symphonia 等）も同一の抽象境界経由とする
* 純 Rust 実装のみコアクレートへ直接統合し、抽象境界は外部バックエンド専用とする

**外部バックエンドの配置:**

* 外部バックエンドを分離クレートとし、opt-in feature で有効化する
* コアクレート内の feature flag のみで外部バックエンドを切り替える

## Decision Outcome

以下の組み合わせを採用する。

* **統合方式**: 全メディア I/O をクレート内の抽象境界の背後に配置する。外部ライブラリの型・エラー・機能フラグを公開 API に一切露出しない。直接依存の露出はバックエンド変更が API 破壊に直結するため不採用。GStreamer 等への全面委譲は、Ruviform 自身の RT 契約・データモデル（バッファ・時刻・所有権）を外部エンジンのモデルに従属させ、差替可能性の目的にも反するため不採用。
* **純 Rust 実装**: symphonia 等の純 Rust 実装も同一の抽象境界経由とする。純 Rust であることを理由に境界を bypass すると「外部依存のみ差替可能・純 Rust は固定」という二種類の統合規則が生じ、将来の実装置換（自前実装への切替等）で API を破壊する。外部依存は純 Rust か否かを問わず、全て差替可能な実装詳細とする。
* **外部バックエンドの配置**: コアは純 Rust に保ち、FFmpeg 等の外部バックエンドは分離クレート + opt-in feature とする。コアクレート内 feature での切替は、feature の付替で外部依存・ライセンス条件が変わり依存者が把握しにくいため不採用。クレート分離により「FFmpeg なしでコアがビルドできる」ことを構造的に保証する。
* **概念分離**: コンテナの demux/mux と codec の decode/encode は別の抽象として分離する（Issue #1 の横断設計要求）。音声・動画に共通の仕組みを設けるかは責務と依存関係の検証後に決める。

### Consequences

* 良い点: バックエンド交換が公開 API を破壊しない。FFmpeg 等のライセンス・配布条件が opt-in 依存として隔離され、コア配布を拘束しない。「FFmpeg なしでビルドできる」ことがクレート構造で強制される。純 Rust・外部を問わない単一の統合規則になり、実装者が境界の有無を場合分けする必要がない。
* 悪い点: 抽象境界（trait・型・エラーモデル）の設計コストが先行する。バックエンド固有の高度な機能（codec 個別パラメータ等）を抽象経由で表現する API 設計が難しく、全機能を汎用化すると抽象が肥大する。バックエンド非対応機能の扱い（feature ネゴシエーション・エラー化）を契約として決める必要がある。

### Confirmation

下記 Verification のチェックリストを、対象クレートの実装 Issue の受け入れ条件と併せて確認する。公開 API に外部クレート由来の型・エラー・feature 名が露出しないことは、生成ドキュメントと公開 API 差分のレビューで確認する。コアクレートが外部バックエンド依存なしでビルドできることは CI で検証する。

## Implementation Plan

* 影響するパス:
  * `crates/` 配下のメディア I/O 系クレート（仮称 `ruviform-audio-io` / `ruviform-video-io`。最終命名・分割は Issue #1 のクレート構成で確定）
  * 外部バックエンド用の分離クレート（FFmpeg 系等。未作成・後続 Issue）
  * 抽象境界の trait・型定義（demux/mux・decode/encode・フォーマット記述・エラーモデル）
* 従うパターン: 公開 API は Ruviform 固有の型（バッファ・時刻・フォーマット記述・エラー）のみで構成し、バックエンドは境界の実装として非公開に保持する。バックエンド選択はクレート/feature 単位の依存として表現し、実行時の文字列指定等にはしない
* 避けるパターン: 外部クレートの型・エラー・列挙値を `pub` で再エクスポートしない。symphonia 等の純 Rust 実装をコア API に直結しない。機能フラグ名・feature 名にバックエンド固有名詞を露出させない（feature 名はバックエンド有効化の意図を表す命名に限定する）
* 依存関係: 本 ADR は境界方針のみ確定し、対応 codec/container・バックエンド選定・個別ライセンス調査は後続 Issue とする（[licensing-and-distribution-policy.md](licensing-and-distribution-policy.md) の個別調査方針を参照）

## Pros and Cons

**統合方式 — 外部クレート直接依存の露出:**

* 良い点: 実装が直接で速い。外部ライブラリの全機能にそのまま到達できる。
* 悪い点: バックエンド交換・依存バージョンアップが公開 API の破壊的変更になる。バックエンド固有の概念が API に混入し、Ruviform のデータモデルと不整合が生じやすい。

**統合方式 — 外部メディアエンジンへの全面委譲（GStreamer 等）:**

* 良い点: 成熟したエンジンの広範なフォーマット対応をそのまま利用できる。
* 悪い点: RT 契約・バッファ所有権・時刻モデルが外部エンジンのモデルに従属し、Ruviform の実行モデルと競合する。外部エンジン自体が重量級の必須依存になり「差替可能な実装詳細」の目的に反する。

**統合方式 — 抽象境界の背後に配置（採用）:**

* 良い点: バックエンド交換が API 非破壊で行え、外部依存のライセンス・配布条件を opt-in に隔離できる。
* 悪い点: 抽象層の設計・維持コスト。バックエンド間の機能差を吸収する API 設計が必要。

**純 Rust 実装 — コア直接統合:**

* 良い点: デフォルト構成で追加境界なしにフォーマット対応できる。
* 悪い点: 「純 Rust は固定・外部のみ差替可能」という二種類の統合規則が生じ、実装置換時に API を破壊する。コアの依存木が外部クレートで肥大する。

**外部バックエンド — コア内 feature 切替:**

* 良い点: クレート数が少ない。
* 悪い点: feature 付替で外部依存・ライセンス条件が変わり、依存者から見えにくい。「外部バックエンドなしでコアをビルドできる」ことを構造的に検証しにくい。

## Verification

* [ ] メディア I/O 系クレートの公開 API（`cargo doc` 出力・公開 trait/型）に外部クレート由来の型・エラー・feature 名が存在しない
* [ ] FFmpeg 等の外部バックエンドを有効化しない構成でコアクレートが `cargo check`/`cargo build` に成功する（CI で検証）
* [ ] 純 Rust バックエンドと外部バックエンドが同一の抽象境界を実装し、利用側コードがバックエンド種別を意識せず切替できる
* [ ] コンテナ demux/mux と codec decode/encode が別の抽象として分離されている
* [ ] 外部バックエンドが分離クレート + opt-in feature として追加され、コアクレートの `[dependencies]` に外部バックエンド依存が直接記述されない

## More Information

* 一次ソース: GitHub Issue [#1 [design] Ruviformの全体アーキテクチャ・クレート構成・RT/オフライン実行モデルを詳細設計する](https://github.com/scarecrow173/ruviform/issues/1)
* 上流ドキュメント: [SPEC-034INYy2aDLxse9dy8MgGZ](../specs/architecture-design-baseline-deliverables.md)（`relations.derives-from`）
* 関連 ADR: [リアルタイム契約と検証方針](realtime-contract-and-verification-policy.md)（I/O・ブロッキングの RT 制約）、[ライセンス・公開方針](licensing-and-distribution-policy.md)（外部バックエンドのライセンス個別調査）、[対象プラットフォーム・ツールチェーン方針](platform-and-toolchain-targets.md)（基礎クレートの依存方針）
* 参考資料: [Symphonia](https://github.com/pdeljanov/Symphonia)（純 Rust バックエンド候補）、[FFmpeg Legal](https://ffmpeg.org/legal.html)（LGPL/GPL・特許条件の調査起点。対応範囲は後続 Issue で確定）
