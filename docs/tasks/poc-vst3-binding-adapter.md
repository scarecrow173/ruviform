---
id: "TASK-034JgsHeZt7Bm40uBMo9wg"
type: "task"
status: "todo"
title: "coupler-rs vst3 による VST3 binding PoC（最小ゲインプラグイン）"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
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
  implements:
    - "../plans/implement-plugin-foundation.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-plugin-foundation.md"
    - "create-plugin-core-foundation.md"
    - "implement-borrowed-planar-view.md"
  blocks:
    - "decide-plugin-binding-method-adr.md"
    - "sync-docs-and-verify-plugin-foundation.md"
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../../Cargo.toml"
    - "../../Cargo.lock"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-plugin-foundation.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/plugin-foundation.md"
  references:
    - "https://github.com/coupler-rs/vst3-rs"
    - "https://github.com/scarecrow173/ruviform/issues/22"
  defers: []
  deferred-by: []
---

# coupler-rs vst3 による VST3 binding PoC（最小ゲインプラグイン）

## Work

plan `PLAN-034JcvpHIlt5JmqvgOquvI` の T4（Task Breakdown の `poc-vst3-binding` 仮称行に対応）。coupler-rs `vst3` クレートで VST3 の規格エントリポイント成立と process 経路のゼロコピー借用を実測し、方式 ADR（T5）の入力となる発見事項を記録する。クリティカルパス上の最長見込みタスク（SDK ビルド要件・COM 足場・unsafe 量の未確定要素が多い）。T3（CLAP PoC）とは相互独立・並行実行可能だが、ルート `Cargo.toml` の `members` 同時追加で競合し得るため統合時に調停する。

- [ ] `crates/ruviform-plugin-vst3/` を作成し `members` へ追加、`vst3` を依存に追加（`Cargo.toml` は **`crate-type = ["cdylib", "rlib"]`** — File Map 参照）
- [ ] ビルド要件を最初に確認する（discovery の既知未確定事項）: 生成済みバインディング同梱で SDK ヘッダ/サブモジュールがビルド時に不要か、生成元 SDK 版が MIT 前提の 3.8+ 由来か、`rust_version` と MSRV 1.85 の適合、クレートのライセンス
- [ ] `Class`/`ComWrapper` で `IPluginBase`/`IComponent`/`IAudioProcessor`（最小限 `IEditController` を含む）を実装し、`GetPluginFactory` モジュールエントリの成立を確認（検証手段: 自前最小 COM 呼び出しハーネス、または `vst3-host` 等の検証専用 dev-dependency — 実装時に選択）
- [ ] `process(*mut ProcessData)` で `inputs`/`outputs` の `channelBuffers32`（`*mut *mut f32`）から `numSamples`（0 以下・prepare 済み最大フレーム数超過）・`numChannels`（activate 時スクラッチ容量超過）・`symbolicSampleSize`/`kSample64`（f64 入力は f32 として再解釈せず定義済みの拒否/スキップ）・チャンネルポインタの 4 バイトアライメント・null を境界で検証した上で `&mut [f32]` 列を SAFETY コメント付き局所 `unsafe` で構築し、T2 の view にラップして読み書き。検証失敗は定義済みの拒否（process 呼び出し拒否・バッファスキップ・エラーステータス）とし、未定義動作・無言のチャンネルインデックスずらしを生じさせない
- [ ] **activate 時スクラッチ**: ポインタ列→スライス列化に必要なチャンネルリスト用スクラッチは activate/prepare 時に最大バスチャンネル数で事前確保し、process 経路では alloc しない。スクラッチはすべてプラグインインスタンスごとの状態とし（`static`/`thread_local` 共有は禁止）、in-place 用データバウンスバッファを採る場合も同様に activate/prepare 時に事前確保する
- [ ] 入力・出力が同一バッファを指す in-place 構成は per-channel ポインタ対比較で検出する（バッファ全体の等価判定のみに留めず、部分エイリアス `out[i]==in[j]`・出力チャンネルポインタ重複も対象）。T2 で決定した方式（専用 in-place view 分岐またはスクラッチ経路）を適用してエイリアス領域上の `&[f32]`+`&mut [f32]` 同時構築を避ける
- [ ] 発見事項を記録する（T3 と同じ観点 + アダプタ側 unsafe の量と局所性・エントリポイント検証手段・ビルド要件の実測）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-plugin-foundation.md](../plans/implement-plugin-foundation.md) — `PLAN-034JcvpHIlt5JmqvgOquvI`（approved）。T4 チェックリスト・V-4 の正本
- [../discovery/evaluate-plugin-binding-options.md](../discovery/evaluate-plugin-binding-options.md) — `DISC-034JccH31pBpKOSW5fst0g`。`vst3` 0.3.0 が VST3 第一候補・退路は自前 COM 層の根拠と既知未確定事項（SDK ヘッダ要否・SDK 版・エントリ検証手段）
- [../test-specs/plugin-foundation.md](../test-specs/plugin-foundation.md) — `TSPEC-034Jg6YSP6CLjJHnFGG5oL`。in-place 検出・activate 時スクラッチ・エントリポイント実証の不変条件

## Dependencies

- 前提タスク: [create-plugin-core-foundation.md](create-plugin-core-foundation.md)（`TASK-034JgsFV6CWAjSnPuekBgs`）— T1 の規格中立型・処理契約が前提（plan 依存グラフ T4←T1）。[implement-borrowed-planar-view.md](implement-borrowed-planar-view.md)（`TASK-034JgsGmcPdPRoeRTvicjW`）— 借用 view と in-place 分岐方針が前提
- 後続をブロック: [decide-plugin-binding-method-adr.md](decide-plugin-binding-method-adr.md) — 方式 ADR の入力は PoC 実測結果
- Issue #22 の alloc 検証基盤は未整備前提。process 経路 no-alloc は activate 時スクラッチ + `rt_` テスト + レビュー可能なドキュメント契約で担保する

## Affected Files

- `crates/ruviform-plugin-vst3/Cargo.toml` — 新規（`crate-type = ["cdylib", "rlib"]`）
- `crates/ruviform-plugin-vst3/src/lib.rs` — 新規（`GetPluginFactory` モジュールエントリ・COM クラス実装・`channelBuffers32`→view 変換・最小ゲインプラグイン PoC）
- `Cargo.toml` — `members` 追加・層内順序コメント
- `Cargo.lock` — `vst3` 依存の記録

## Done When

- [ ] `GetPluginFactory` モジュールエントリが成立し、最小ゲインプラグインの COM クラスがインスタンス化できる
- [ ] `channelBuffers32` から構築した `&mut [f32]` 列を T2 view にラップして読み書きでき、ゼロコピーが実測される
- [ ] チャンネルリスト用スクラッチが activate/prepare 時に事前確保され、process 経路で alloc しない
- [ ] in-place 構成で per-channel ポインタ対比較を境界検出し、エイリアス領域上の `&[f32]`+`&mut [f32]` 同時構築がない
- [ ] cdylib 成果物に `GetPluginFactory` とプラットフォーム条件付きモジュールエントリのエクスポートシンボルが存在する（macOS: `bundleEntry`/`bundleExit` 必須、Linux: `ModuleEntry`/`ModuleExit` 必須、Windows: `InitDll`/`ExitDll` 任意）
- [ ] ビルド要件の実測（SDK ヘッダ要否・SDK 版・MSRV・ライセンス）と発見記録が方式 ADR 入力として揃っている

## Verification

- [ ] `mise exec -- cargo test -p ruviform-plugin-vst3` exits 0
- [ ] `mise exec -- cargo clippy -p ruviform-plugin-vst3 --all-targets -- -D warnings` exits 0（SAFETY コメント付き局所 `#[expect(unsafe_code)]`（未対応なら `#[allow]`）のみで通過）
- [ ] `mise exec -- cargo build -p ruviform-plugin-vst3` で cdylib 成果物が生成され、エクスポートシンボルに `GetPluginFactory` とプラットフォーム条件付きモジュールエントリ（macOS: `bundleEntry`/`bundleExit` 必須、Linux: `ModuleEntry`/`ModuleExit` 必須、Windows: `InitDll`/`ExitDll` 任意）が存在する（`nm`/`dumpbin` 等で確認、または dev-dep のホストハーネスでロード実証。**`cargo test` 単独ではこの検証を満たさない**）
- [ ] process 経路に `rt_` プレフィックスの no-alloc 契約テスト + レビュー可能なドキュメント契約を置く（`rt_` 名フィルタは alloc を観測できないため、本確認はドキュメント契約 + レビュー対象として扱う Issue #22 基盤未整備の代替）
- [ ] 発見記録（T3 同観点 + unsafe の量と局所性・エントリ検証手段・ビルド要件実測）が揃っていることを確認
