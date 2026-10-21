---
id: "TSPEC-034Jg6YSP6CLjJHnFGG5oL"
type: "test-spec"
status: "approved"
title: "プラグイン基盤 API の契約保証"
created: "2026-10-20"
updated: "2026-10-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/29"
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
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../adr/plugin-architecture-clap-vst3.md"
    - "../adr/realtime-contract-and-verification-policy.md"
  derived-by: []
  verifies:
    - "../specs/implement-plugin-foundation.md"
    - "../adr/plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md"
  verified-by: []
  references:
    - "https://github.com/prokopyl/clack"
    - "https://github.com/coupler-rs/vst3-rs"
  defers: []
  deferred-by: []
---

# プラグイン基盤 API の契約保証

## Purpose

Issue #29 のプラグイン基盤（`ruviform-plugin-core` + CLAP/VST3 アダプタ）が確定させる契約——規格中立のプラグイン処理契約、ライフサイクル状態機械、独立 per-channel planar バッファのゼロコピー借用、RT/no-alloc 契約、規格中立性——の「なぜテストが存在するか」を保持する。[spec](../specs/implement-plugin-foundation.md) の受け入れ基準と [plugin ADR](../adr/plugin-architecture-clap-vst3.md)・[RT ADR](../adr/realtime-contract-and-verification-policy.md)・[設計ベースライン](../designs/ruviform-architecture-design-baseline.md) の確定範囲を、task 分割や方式 ADR の決定が変わっても維持すべき保証として固定する。廃止してよいのは、対象契約自体を変更する ADR / spec 改訂（例: 処理契約の `ruviform-rt::Processor`/`RtContext` への統一、ライフサイクル状態列の変更）が行われたとき。

## Feature

規格中立プラグイン基盤の契約: `ruviform-plugin-core` が所有するプラグイン処理契約（prepare/process 形状）とライフサイクル状態機械、およびプラグイン API が供給する独立 per-channel planar バッファのゼロコピー借用 view が、規格アダプタ（CLAP/VST3）の境界を通じて spec の不変条件を満たすこと。

## Rules

- **プラグイン処理契約は `ruviform-plugin-core` が所有する**。prepare/process 形状の規格中立 trait（仮称 `PluginProcessor` 相当）はプラグイン作者が実装し、アダプタはこれを実装・駆動する側であって契約自体を定義しない。将来の `RtContext` 等の capability 注入を破壊的変更なく吸収できるよう、context 引数は opaque/予約型とする。
- **ライフサイクル遷移は `create → init → activate → processing → deactivate → destroy` の規則に従う**。不正遷移は型と `Result` で表出され、両アダプタの規格状態遷移はこの共通状態機械へ写像される。
- **借用 view は `process` コールバックスコープにライフタイムを束縛する**。所有権移転なしで per-channel ポインタ/スライス列をラップし、借用がコールバックを脱退するコードはコンパイルを通らない（borrow-escapes-scope 系のエラー）。canonical planar f32 と同一レイアウトで、f32 経路の変換回数はゼロ。
- **in-place 処理でエイリアスした `&[f32]` + `&mut [f32]` を同時に構築しない**。入出力同一バッファ（CLAP `in_place_pair` / `ChannelPair::InPlace`、VST3 の同一 `channelBuffers32` ポインタ）はアダプタが境界でポインタ一致または範囲重複（`[ptr, ptr + frames×4)` の交差）を検出し、専用 in-place view 分岐・スクラッチ経路または定義済み拒否を適用する。
- **`process` 経路は alloc/dealloc しない**。ポインタ列をスライス view 化するチャンネルリスト用スクラッチは activate/prepare 時に最大バスチャンネル数で事前確保される。RT 適合性は API 単位の `**RT contract:**` ラベルと `rt_` プレフィックスのテスト・レビュー可能なドキュメント契約で担保する。
- **規格中立性と外部型非露出を守る**。`ruviform-plugin-core` に binding クレート（`clack`/`vst3`/`clap-sys` 等）への `use`・依存辺・公開シグネチャ上の規格固有型を持たない。クレート外から到達可能な全公開 enum に enum レベル `#[non_exhaustive]` を付与する。
- **`unsafe` は SAFETY コメント付き局所許可に限定される**。必要箇所（VST3 生ポインタ列のスライス化等）は `#[expect(unsafe_code)]`（未対応なら `#[allow(unsafe_code)]`）の局所許可に封じ込め、plugin-core / アダプタの公開 API に `pub unsafe fn` を置かない。
- **借用 view のエッジ契約を守る**。ゼロチャンネル・ゼロフレーム・非アクティブバスの view は空のチャンネルリストとして構築可能だがサンプルを公開せず、null ポインタ上に view を構築しない。
- **規格中立トランスポート/イベント型は音楽時間を持たない**。`TimeBase` + 整数カウントのみを扱い、将来の音楽時間マッピングを ABI 変更なく吸収する opaque 拡張予約フィールドを持つ。

## Examples

- `crates/ruviform-plugin-core` 内で `use clack_plugin` または `use vst3` を書くコードは規格中立性監査で検出される。非目標宣言として doc コメントに "CLAP"/"VST3" の語を含むことは許容される。
- 独立した 3 チャンネルの `&mut [f32]` バッファ列から構築した mutable view は各チャンネルの読み書きを提供し、ホスト側バッファと同一メモリを介して反映される（コピーなし）。
- view を `process` コールバック外の `'static` 変数に逃がすコードはコンパイルエラーとなる（`compile_fail` doctest で固定。期待エラー種別は borrow-escapes-scope 系）。
- 入出力に同一バッファが渡された場合、アダプタはポインタ一致を検出して決定済みの in-place 方式へ分岐し、同一領域を `&[f32]` と `&mut [f32]` で同時借用しない。
- ゼロチャンネル・ゼロフレーム・非アクティブバスの view は構築可能だが、公開するサンプル数・チャンネル数は 0 であり null ポインタ由来の view は作られない。
- processing 中の `deactivate` 要求は定義済みエラーとして拒否される（activate 前の `process` 呼び出しも同様）。
- `mise exec -- cargo build -p ruviform-plugin-clap` / `-p ruviform-plugin-vst3` で cdylib 成果物が生成され、`clap_entry` / `GetPluginFactory`・`ModuleEntry` 系のエクスポートシンボルが存在する（`cargo test` 単独では検証しない）。

## Guarantees

- 正しい実装は、規格固有コードが plugin-core の公開面に漏出しないことをコード要素（`use`・依存辺・公開シグネチャ）で示せる。
- 正しい実装は、借用 view のライフタイム束縛・in-place 分岐・null/エッジ契約を型シグネチャとテストで機械的に担保する。
- 正しい実装は、process 経路で alloc/dealloc・ブロッキング・I/O・無制限待機・非有界処理を行わないことを `rt_` テストとドキュメント契約で示せる。
- 正しい実装は、両規格アダプタが共通処理契約を定義せず実装・駆動のみを行う構造（契約所有の一方向性）を維持する。

## Non-goals

- binding 方式の最終採否（`clack`/`vst3` の採用決定自体）は方式 ADR が保持し、本 spec は採否を検証しない。
- `assert_no_alloc` 系の自動 alloc 検証レーンの整備（Issue #22 のスコープ。基盤未整備の間は `rt_` テスト + ドキュメント契約で代替）。
- `ruviform-rt::Processor`/`RtContext` との契約統一（RT 契約実装 Issue に委譲。現段階は `**RT contract:**` doc ラベルのみ）。
- プラグイン GUI・サンドボックス分離・音楽時間・f64 経路・`ruviform-plugin-host`/`-export` 本格実装（spec Out of Scope に一致）。

## Risk

- 契約所有境界が崩れると、アダプタごとの独自契約が散在しホスト/制作対称性と将来の capability 統一が破壊される。
- 借用ライフタイム束縛や in-place エイリアス検出を落とすと、未定義動作・データ競合を含む修正困難な RT パス不具合が混入する。
- エントリポイントのエクスポート検証を欠くと、`cargo test` が緑でも規格準拠の cdylib が生成されていない欠陥を見逃す。
