---
id: "PLAN-034JcvpHIlt5JmqvgOquvI"
type: "plan"
status: "completed"
title: "プラグイン基盤の実装（ruviform-plugin-core + CLAP/VST3 アダプタ・binding PoC）"
created: "2026-10-20"
updated: "2026-10-20"
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
  implements:
    - "../specs/implement-plugin-foundation.md"
  implemented-by:
    - "../tasks/create-plugin-core-foundation.md"
    - "../tasks/implement-borrowed-planar-view.md"
    - "../tasks/poc-clap-binding-adapter.md"
    - "../tasks/poc-vst3-binding-adapter.md"
    - "../tasks/decide-plugin-binding-method-adr.md"
    - "../tasks/implement-plugin-adapters.md"
    - "../tasks/sync-docs-and-verify-plugin-foundation.md"
  depends-on:
    - "implement-issue-18-data-model.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../../Cargo.toml"
    - "../../mise.toml"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/implement-plugin-foundation.md"
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../adr/plugin-architecture-clap-vst3.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../discovery/evaluate-plugin-binding-options.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/plugin-foundation.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/41"
    - "https://github.com/scarecrow173/ruviform/issues/22"
    - "https://github.com/prokopyl/clack"
    - "https://github.com/coupler-rs/vst3-rs"
  defers: []
  deferred-by: []
---

# プラグイン基盤の実装（ruviform-plugin-core + CLAP/VST3 アダプタ・binding PoC） Implementation Plan

## Goal

spec `SPEC-034JcgdAKPNeh1YCFyprzh`（Issue #29）の受け入れ基準を充足する: 規格中立のプラグイン基盤クレート `ruviform-plugin-core`（プラグイン/パラメータ記述・sample-accurate イベント・オーディオバス/ポート記述・規格中立トランスポート/イベント型・ライフサイクル状態機械・`**RT contract:**` ラベル）と、独立 per-channel planar バッファのゼロコピー借用 view/newtype、`ruviform-plugin-clap` / `ruviform-plugin-vst3` の薄い規格アダプタ、両規格の binding 方式 PoC、および PoC 結果に基づく binding 方式 ADR を実装・検証する。

## Scope

単一の実装ストリームとして扱う。共通基盤の型・ライフサイクル・借用契約が借用 view・PoC・アダプタ・方式 ADR のすべての前提となる同一クリティカルパス上にあり、Issue #29 という単一リリース意図を共有する。分割しない。

範囲外（spec Out of Scope と一致）: プラグイン GUI（ui ADR の内側実装・`baseview`/`wxp` 等の GUI 基盤選定）、OFX・映像プラグイン、サンドボックス/クラッシュ分離の具体方式（インプロセス既定と「ハンドル + 直列化可能コマンド境界」の構造制約のみ遵守）、音楽時間（小節/拍/PPQN・テンポ — opaque 予約フィールドのみ）、`ruviform-plugin-host` / `ruviform-plugin-export` の本格実装（共通契約の確立のみ）、f64 サンプルの本格対応（型形状上の予約のみ・RT 正準は f32）、AU / AAX / LV2 等の追加規格、パラメータ自動化・状態保存・拡張機能の網羅的実装。

## Upstream Documents

- [../specs/implement-plugin-foundation.md](../specs/implement-plugin-foundation.md) — `SPEC-034JcgdAKPNeh1YCFyprzh`（**proposed**。approved ではない点は Risk Register に記録）。本 plan が実装する spec。Issue #29 範囲・受け入れ基準・Requirements の正本
- [GitHub Issue #29](https://github.com/scarecrow173/ruviform/issues/29) — 一次ソース（`relations.source`）。本体 + #18 フォローアップ由来のゼロコピー受け入れ条件コメント

## Design Inputs

- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — `DESIGN-034IPynWe2V30FLubzQenb`（approved）。「プラグインアーキテクチャ」節（共通基盤+規格アダプタ分離・ABI 境界・ライフサイクル状態機械・規格中立トランスポート型の opaque 拡張予約・インプロセス既定・RT 契約双方適用・binding クレート利用を妨げない）とクレート表 A.2（`ruviform-plugin-*` の責務・許可依存層・RT 位置づけ）の根拠
- [../adr/plugin-architecture-clap-vst3.md](../adr/plugin-architecture-clap-vst3.md) — plugin ADR `ADR-034IO01bG8AyjIJWxHeL77`（accepted）。提供範囲（ホスト/制作対称）・CLAP/VST3 同格・分離構造・実装順序は技術的依存・効率で決定（同時実装を要求しない）・binding 方式は後続評価に保留、の正本
- [../adr/realtime-contract-and-verification-policy.md](../adr/realtime-contract-and-verification-policy.md) — RT ADR `ADR-034IO00JjhxeTXUbCZtcDX`（accepted）。process 系コールバックの RT パス適用・禁止事項（alloc/dealloc・ブロッキング・I/O・無制限待機・非有界処理）・API 単位 `**RT contract:**` ラベル規則・「CI 成功 ≠ 完全証明」の根拠
- [../discovery/evaluate-plugin-binding-options.md](../discovery/evaluate-plugin-binding-options.md) — `DISC-034JccH31pBpKOSW5fst0g`（draft）。binding 候補の事実ベース比較（`clack` 0.2.0 / `vst3` 0.3.0 が規格別第一候補、`clap-sys`・自前 COM 層が退路、`nice-plug` は MSRV 1.88 不整合で不適）と PoC 評価計画・既知の未確定事項（VST3 SDK ヘッダ要否・binding SDK 版・エントリポイント検証手段）の正本
- [../adr/public-enum-non-exhaustive-policy.md](../adr/public-enum-non-exhaustive-policy.md) — 公開 enum `#[non_exhaustive]` 統一方針 ADR（accepted）。新規公開 enum 全件の適用規則（enum レベル・兄弟クレート=外部・内部 enum 免除）の正本
- [../adr/media-io-backend-abstraction.md](../adr/media-io-backend-abstraction.md) — 外部型非露出規則の原則（binding クレートの型・エラー・feature 名を公開 API に露出しない — spec Requirements が同一方針を引用）
- [../specs/implement-media-data-model-api.md](../specs/implement-media-data-model-api.md) — planar f32 canonical・`AudioBuffer` 借用モデル・`TransportSnapshot` の opaque `reserved` パターン・`**RT contract:**` ラベル記法の実装先例

## File Map

新規クレート（拡張層。依存は Domain foundations + Primitives のみ）:

- `crates/ruviform-plugin-core/Cargo.toml` — workspace 継承（`version`/`edition`/`rust-version`/`license`/`publish`）+ `[lints] workspace = true` + path 依存（`ruviform-media-core`・`ruviform-audio-core`。`ruviform-rt` の `RtContext` capability 受け入れは RT 契約実装 Issue に委譲する確定決定のため、現段階では実利用がない限り `ruviform-rt` 依存は付けない — Open Questions / Gaps の決定記録参照）
- `crates/ruviform-plugin-core/src/lib.rs` — crate doc（責務・許可依存層・RT 制約の転記）+ フラット再エクスポート + `rt_` テスト
- `crates/ruviform-plugin-core/src/descriptor.rs` — 規格中立のプラグイン記述（ID・name・vendor・version・カテゴリ等の最小集合）
- `crates/ruviform-plugin-core/src/param.rs` — パラメータ記述（id・範囲・既定値・フラグ）とパラメータ値型
- `crates/ruviform-plugin-core/src/event.rs` — sample-accurate イベント（sample offset 付き。opaque 拡張予約フィールドを持つ規格中立型）
- `crates/ruviform-plugin-core/src/bus.rs` — オーディオバス/ポート記述（チャンネル数・`ChannelLayout`・入出力・有効フラグ）
- `crates/ruviform-plugin-core/src/transport.rs` — 規格中立トランスポート/イベント状態型（`TimeBase` + 整数カウントのみ・音楽時間なし・opaque `reserved` フィールド — `TransportSnapshot` と同一方針）
- `crates/ruviform-plugin-core/src/lifecycle.rs` — ライフサイクル状態機械（`create → init → activate → processing → deactivate → destroy` の状態列挙と遷移規則。不正遷移を型・結果で表出）
- `crates/ruviform-plugin-core/src/processor.rs` — 規格中立のプラグイン処理契約（prepare/process 形状の trait。仮称 `PluginProcessor` 相当 — プラグイン作者が実装しアダプタが駆動する契約で、所有は plugin-core。`**RT contract:**` ラベル付き。`RtContext` 等の将来の capability 注入を破壊的変更なく吸収できるよう context 引数は opaque/予約型とする）
- `crates/ruviform-plugin-clap/` — `Cargo.toml`（`crate-type = ["cdylib", "rlib"]`: cdylib は規格エントリポイントのエクスポート用・rlib は doctest/`cargo test` 用）+ `src/lib.rs` + 規格エントリポイント + ABI→中立型変換。PoC 資産（最小プラグイン example/test）は本クレート内に置く（決定根拠は Task Breakdown T3 参照）
- `crates/ruviform-plugin-vst3/` — 同上（VST3 側。`crate-type = ["cdylib", "rlib"]` も同様）

ゼロコピー借用 view の配置先（T2 で確定。詳細は Task Breakdown T2）:

- lean: `crates/ruviform-audio-core/src/planar.rs` — 独立 per-channel バッファ列の借用 view/newtype（音声データモデルの借用形状として一般化）
- fallback: `crates/ruviform-plugin-core/src/buffer.rs` — 同上（プラグイン ABI 境界固有と判断された場合）
- **T2 確定結果（2026-10-21）: lean 採用 — `crates/ruviform-audio-core/src/planar.rs` に `PlanarBufferView<'a, S: Sample = f32>` / `PlanarBufferViewMut<'a, S: Sample = f32>` として実装**。根拠: 独立 per-channel 借用形状はプラグイン ABI 境界固有ではなく device/host 境界にも現れる汎用音声形状であり、`AudioBufferView`/`AudioBufferViewMut`・video-core `Plane`/`PlaneMut` と同じ「domain クレートに借用 view を置く」先例と整合。`S: Sample` ジェネリック化により f64 経路予約を閉塞しない。`BufferView`/`BufferViewMut` trait の impl は plugin-core 側に配置（trait ローカルのため orphan rule 適合・audio-core → plugin-core の逆依存を生じない）。in-place 方針の評価結果: bounce-scratch をあらゆるエイリアスパターンに効く uniform 安全デフォルトとし、binding が in-place ペアを分類する場合（`clack` の `ChannelPair::InPlace`）のみ専用分岐が有効 — 詳細は T5 の方式 ADR で決定記録

変更対象（既存）:

- `Cargo.toml` — `members` に 3 クレート追加 + 層内順序コメントの更新（拡張層として既存 6 クレートの後段）
- `Cargo.lock` — 新クレートと binding 依存（`clack-*`・`vst3` 系）の記録更新
- `README.md` — `crates/` 節のクレート一覧と実装状態記述の同期
- `AGENTS.md` — 「ワークスペース構成」節のメンバー記述同期
- `docs/adr/<binding-method-adr>.md` — 方式 ADR（T5 で新規作成。ファイル名は adr-doc skill の命名規約に従う）
- `docs/adr/README.md` — 方式 ADR の索引登録（T5。doc-maintenance の生成索引）
- `docs/specs/implement-plugin-foundation.md` — 方式 ADR への relation リンク追加 + status 同期（T5。spec 受け入れ基準が要求）
- `docs/discovery/evaluate-plugin-binding-options.md` — PoC 結果と方式 ADR への `derived-by` リンク・status 同期（T5。任意の追記）

## Implementation Sequence

### T1: `ruviform-plugin-core` クレート骨格 + 規格中立型・ライフサイクル

- [ ] `crates/ruviform-plugin-core/` を作成（`Cargo.toml`: workspace 継承・`[lints] workspace = true`・path 依存 `ruviform-media-core` / `ruviform-audio-core`。`ruviform-rt` は `RtContext` 受け入れを RT 契約実装 Issue に委譲する決定により現段階では付けない — File Map・Open Questions / Gaps 参照）
- [ ] ルート `Cargo.toml` の `members` に `crates/ruviform-plugin-core` を追加し、層内順序コメントに拡張層を追記
- [ ] `descriptor` / `param` / `event` / `bus` / `transport` / `lifecycle` / `processor` モジュールの公開型を実装。transport/event は音楽時間を持たず `TimeBase` + 整数カウントのみ、opaque `reserved` 拡張フィールドを持つ。lifecycle は `create → init → activate → processing → deactivate → destroy` の遷移規則を型と `Result` で固定。`processor` は規格中立のプラグイン処理契約（prepare/process 形状・仮称 `PluginProcessor` 相当）を定義する — **契約の所有は plugin-core であり、アダプタ（`ruviform-plugin-clap` / `-vst3`）はこの契約を実装・駆動する側で、契約自体は決して定義しない**。将来の `RtContext` 等 capability 注入を破壊的変更なく吸収できるよう、context 引数は opaque/予約型とする（Open Questions / Gaps の決定記録参照）
- [ ] `lib.rs` に crate doc（責務・許可依存層 Domain foundations + Primitives・RT 制約・規格中立性の非目標宣言）・再エクスポートを実装。全公開 API に `**RT contract:**` ラベル、全公開 enum に enum レベル `#[non_exhaustive]`、公開 API 例を doctest として整備
- [ ] `mise exec -- cargo test -p ruviform-plugin-core` と `mise exec -- cargo clippy -p ruviform-plugin-core --all-targets -- -D warnings` を実行し全合格を確認

### T2: 独立 per-channel planar バッファのゼロコピー借用 view/newtype

- [ ] 配置先を確定する。**lean: `ruviform-audio-core`**（`AudioBufferView`/`AudioBufferViewMut` と同じ「音声バッファ借用形状」の責務であり、video-core の `Plane`/`PlaneMut` に相当する借用 view が domain クレートに置かれる先例と整合。ホスト側・デバイス境界等 plugin 以外の per-channel 非連続バッファ供給にも再利用可能）。fallback: `ruviform-plugin-core`（借用形状がプラグイン ABI 境界固有と判明した場合）。確定結果と根拠を crate doc と本 plan 追記に反映する
- [ ] 独立したチャンネルごとのポインタ/スライス列を所有権移転なしでラップする view/newtype を実装（読み取り用 `&[f32]` 列・書き込み用 `&mut [f32]` 列の双方。既存 `AudioBuffer` 系はチャンネルメジャー連続ストレージ前提のため拡張ではなく新設。`&'a mut [&'a mut [f32]]` 相当の型形状で `'a` を process コールバックスコープに束縛）。canonical planar f32 と同一レイアウトで変換回数ゼロであることを API 設計で担保。コンストラクタはアダプタ側が事前確保するチャンネルリスト用スクラッチを受け取れる形状とし、view 構築自体が alloc しないこと（T3/T4 のスクラッチ事前確保と対応）
- [ ] in-place 処理への対応方針を本タスクの評価事項として決定する: ホストが入出力に同一バッファを渡す場合（CLAP `in_place_pair` / clack `ChannelPair::InPlace` 相当）、アダプタはエイリアスした同一領域上に `&[f32]` と `&mut [f32]` を同時に構築してはならない。境界でポインタ一致を検出し「専用の in-place view 分岐」または「スクラッチ経路」のどちらを採用するかを評価し、選択と根拠を方式 ADR（T5）の判断材料として記録する
- [ ] エッジ契約を API ドキュメントに明記する: ゼロチャンネル・ゼロフレーム・非アクティブバスの view は空のチャンネルリストとして構築可能だがサンプルを公開しない。null ポインタ上に view を構築しない
- [ ] 「借用中にホスト側が解放・再配置しない」前提契約と `**RT contract:**` ラベルを API ドキュメントに明記。借用がコールバックを脱退できないことを `compile_fail` doctest で固定（型シグネチャのライフタイム束縛によるコンパイル時担保。期待するエラー種別は borrow-escapes-scope 系であることを明記し、正常系 doctest・単体テストと対で整備する）
- [ ] 独立チャンネルバッファから構築した view が各チャンネルの読み書きを提供する単体テスト、`rt_` プレフィックスの no-alloc 契約テストを追加。Issue #22 の alloc 検証基盤が未整備の場合は `rt_` テスト + レビュー可能なドキュメント契約で代替し、その旨を crate doc と本 plan に明記する（spec 受け入れ基準準拠）
- [ ] `mise exec -- cargo test -p <配置クレート>` と `mise exec -- cargo clippy -p <配置クレート> --all-targets -- -D warnings` を実行し全合格を確認

### T3: CLAP binding PoC（`clack` 系）

- [ ] `crates/ruviform-plugin-clap/` を作成し `members` へ追加、`clack-plugin`（必要に応じ `clack-extensions`）を依存に追加（`Cargo.toml` は `crate-type = ["cdylib", "rlib"]` — File Map 参照）。検証ハーネスとして dev-dependency に `clack-host` を使うか、エントリ構造体を直接駆動するかを実装時に選択（lean: 規格エントリポイント経由の構築を実証できる `clack-host` 経路を優先）
- [ ] `clack_export_entry!` による最小ゲインプラグインで規格エントリポイントの成立を確認（cdylib エクスポートまたはホストハーネス経由のインスタンス化）
- [ ] `process(Process, Audio, Events)` の `PortPair` → `channels()` → `ChannelPair` が返す per-channel `&[f32]`/`&mut [f32]` を T2 の view にラップして読み書きし、ゼロコピー借用を実測。`ChannelPair::InPlace`（入出力同一バッファ）はポインタ一致を境界で検出し、T2 で決定した方式（専用 in-place view 分岐またはスクラッチ経路）を適用してエイリアス領域上の `&[f32]`+`&mut [f32]` 同時構築を避ける
- [ ] 発見事項を記録する（方式 ADR の入力）: ゼロコピー可否・アダプタ側が書く unsafe の量・process 経路の alloc 有無・ビルド要件・`rust_version` ≤ 1.85・ライセンス適合・拡張カバレッジ（params・events・audio-ports・thread-check 等）・保守状況・依存木サイズ
- [ ] `mise exec -- cargo test -p ruviform-plugin-clap` と `mise exec -- cargo clippy -p ruviform-plugin-clap --all-targets -- -D warnings` を実行し全合格を確認

### T4: VST3 binding PoC（coupler-rs `vst3`）

- [ ] `crates/ruviform-plugin-vst3/` を作成し `members` へ追加、`vst3` を依存に追加（`Cargo.toml` は `crate-type = ["cdylib", "rlib"]` — File Map 参照）
- [ ] ビルド要件を最初に確認する（discovery の既知未確定事項）: 生成済みバインディング同梱で SDK ヘッダ/サブモジュールがビルド時に不要か、生成元 SDK 版が MIT 前提の 3.8+ 由来か、`rust_version` と MSRV 1.85 の適合、クレートのライセンス
- [ ] `Class`/`ComWrapper` で `IPluginBase`/`IComponent`/`IAudioProcessor`（最小限 `IEditController` を含む）を実装し、`GetPluginFactory` モジュールエントリの成立を確認（検証手段: 自前最小 COM 呼び出しハーネス、または `vst3-host` 等の検証専用 dev-dependency — 実装時に選択）
- [ ] `process(*mut ProcessData)` で `inputs`/`outputs` の `channelBuffers32`（`*mut *mut f32`）から `numSamples`・`numChannels`・`symbolicSampleSize` を検証した上で `&mut [f32]` 列を SAFETY コメント付き局所 `unsafe` で構築し、T2 の view にラップして読み書き。ポインタ列→スライス列化に必要なチャンネルリスト用スクラッチは activate/prepare 時に最大バスチャンネル数で事前確保し、process 経路では alloc しない。入力・出力が同一バッファを指す in-place 構成はポインタ一致を境界で検出し、T2 で決定した方式（専用 in-place view 分岐またはスクラッチ経路）を適用する
- [ ] 発見事項を記録する（T3 と同じ観点 + アダプタ側 unsafe の量と局所性・エントリポイント検証手段・ビルド要件の実測）
- [ ] `mise exec -- cargo test -p ruviform-plugin-vst3` と `mise exec -- cargo clippy -p ruviform-plugin-vst3 --all-targets -- -D warnings` を実行し全合格を確認

### T5: binding 方式 ADR

- [ ] T3/T4 の PoC 実測に基づき、`docs/adr/` に binding 方式 ADR を作成する（adr-doc skill の規約に従う）。記録内容: 規格ごとの構築方式決定（CLAP=`clack` 採否、VST3=`vst3` 採否 — 規格単位の方式差異を許容）、比較根拠（ゼロコピー可否・unsafe 量・alloc 有無・ビルド要件・MSRV/ライセンス適合・保守状況・依存木サイズ）、in-place 処理の対応方式選択（専用 in-place view 分岐 vs スクラッチ経路 — T2 で評価し本 ADR で決定を記録）とスライス化スクラッチの activate/prepare 時事前確保方針、不採用側の退路（`clap-sys` 直上の自前薄ラッパー / 自前 COM 層の範囲見積り）
- [ ] 本 spec の front matter relations に方式 ADR へのリンクを追加し、`docs/adr/README.md` の索引を更新する
- [ ] `node .agents/skills/doc-status/scripts/doc_status.js audit --type adr --json` と `lint --rule broken-link --json` で 0 findings を確認

### T6: 薄い規格アダプタの完成（方式 ADR の決定に従う）

- [ ] `ruviform-plugin-clap` / `ruviform-plugin-vst3` を完成させる: 規格エントリポイント、ABI ↔ 共通基盤型（バッファ・時刻・パラメータ・イベント）の境界限定一方向翻訳、規格の状態遷移を共通ライフサイクル状態機械へ写像（規格ごとの遷移規則対応を単体テストで検証）。**アダプタは plugin-core が所有する処理契約を実装・駆動する側であり、契約自体は決して定義しない**（T1・spec In Scope 参照）
- [ ] 外部型非露出を監査する（公開 API シグネチャに `clack` / `vst3` / `clap-sys` 等の binding クレート型・エラー・feature 名が現れないこと。`use` 行・re-export による短縮名での露出を含め、grep と `cargo doc` 公開面で確認 — V-6 参照）
- [ ] 両クレートの `mise exec -- cargo test -p ruviform-plugin-clap -p ruviform-plugin-vst3` と `mise exec -- cargo clippy -p ruviform-plugin-clap -p ruviform-plugin-vst3 --all-targets -- -D warnings` を実行し全合格を確認

### T7: ドキュメント同期 + 総合検証

- [ ] ルート `Cargo.toml` の層内順序コメント、`README.md`（`crates/` 節・実装状態）、`AGENTS.md`（ワークスペース構成節）を実装状態に同期する
- [ ] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` / `bench`、`mise exec -- cargo metadata --format-version 1 --no-deps`（全クレート `publish` が `[]`・依存辺が plugin 系→Domain foundations+Primitives のみ）、`mise exec -- prek run --all-files` を実行し全合格を確認
- [ ] spec / plan / task の status・`changes` 記録を実施結果に同期する

## Dependencies and Risks

### Dependency Graph

| 依存元 | 依存先 | 内容 |
| --- | --- | --- |
| T2 | T1 | 借用 view の API 形状は plugin-core の process モデル・規格中立型と整合させて設計するため core 先行。audio-core 配置時も T1 完了後に確定する順序とする |
| T3 / T4 | T1・T2 | PoC は規格中立型と借用 view の上に構築する。T3 と T4 は相互独立・並行実行可能 |
| T5 | T3・T4 | 方式 ADR の入力は両規格の PoC 実測結果のみ（アダプタ実装結果は不要 — spec 受け入れ基準） |
| T6 | T5 | アダプタの構築方式は方式 ADR が決定ゲート。binding 不適判明時のアダプタ手戻りを防ぐため決定記録を先行させる |
| T7 | T1〜T6 | ドキュメント同期と総合検証は全実装完了後に実施 |
| 本 plan | `implement-issue-18-data-model.md` | `TimeBase`/`MediaTime`・`AudioBuffer` 借用モデル・`TransportSnapshot` の reserved パターン・`**RT contract:**` 規則を提供する実装済みデータモデルを前提 |

- 実装順序の根拠: 技術的依存は core → 借用 view → PoC（T3‖T4 並行可）→ 方式 ADR（決定ゲート）→ アダプタ → docs 同期。spec の「実装順序は技術的依存・効率で決定する（同時実装を要求しない）」に一致
- クリティカルパスは T1 → T2 → T4（VST3 PoC — SDK ビルド要件・COM 足場・unsafe 量の未確定要素が多く最長見込み）→ T5 → T6 → T7
- 新規 plugin 系クレート間の相互依存は `ruviform-plugin-clap` / `ruviform-plugin-vst3` → `ruviform-plugin-core` の一方向のみ。plugin 系 ↔ ui 系の依存は作らない（ベースライン規則）

### Risk Register

| リスク | 影響 | 確率 | 対応 |
| --- | --- | --- | --- |
| データ損失 | なし（新規クレート・新規文書の追加のみ。既存 API 変更は T2 が audio-core 配置の場合の追加 API のみ） | なし | — |
| 破壊的変更 | `publish = false`・利用は workspace 内のみ。audio-core への view 追加は additive で既存 API 非破壊 | 低 | 追加 API のみとし、既存 `AudioBuffer` 系の契約は変更しない |
| 外部依存の可用性（`clack`） | 0.2.0 の若いクレート系で API churn の可能性。`rust_version = 1.85` は MSRV 境界ちょうど | 中 | T3 で実測し方式 ADR に記録。不適なら `clap-sys` 直上の自前薄ラッパー（discovery の退路）を方式 ADR で選択 |
| 外部依存の可用性（`vst3`） | `rust_version` 未宣言で MSRV 不整合の可能性・生成バインディングの SDK 出自（MIT 3.8+ 前提）・保守規模小 | 中 | T4 の最初にビルド要件・SDK 出自・MSRV を確認。不適なら自前 COM 層の範囲見積りを方式 ADR に併記 |
| VST3 のビルド要件 | SDK ヘッダ/サブモジュールがビルド時必須なら CI・再現ビルドが壊れる | 中 | T4 の最初の確認項目。生成済みバインディング同梱なら不要の見込み。必須なら方式 ADR で自前/代替を評価 |
| アダプタ側 `unsafe` の量 | VST3 側は全 trait メソッドが `unsafe fn` で量が増える見込み | 中 | SAFETY コメント付き局所許可に限定し量と局所性を PoC で実測。許容超過なら方式 ADR で別方式を選択 |
| binding クレート内部の unsafe・ライセンス | 依存内部 unsafe は許容しうるが、ライセンス再配布適合は要確認 | 低 | PoC で `cargo tree` + ライセンス確認を実施し、licensing ADR の個別調査手順に従って方式 ADR に記録 |
| spec が `proposed`（未承認） | spec 改訂でスコープ・受け入れ条件が変わる可能性 | 低 | plan-doc 規約どおり draft/proposed 由来のリスクとして明示。spec 改訂時は本 plan を改訂する |
| Issue #22 の alloc 検証基盤が未整備 | process パスの alloc 自動検証ができない | 高（既に未整備） | spec 受け入れ基準の代替規定どおり `rt_` テスト + レビュー可能なドキュメント契約で代替し明記。基盤整備は Issue #22 に委譲 |
| cdylib エントリポイントの OS 差 | CLAP/VST3 エクスポートが 3 OS matrix で差異を持つ可能性 | 中 | 手動検証ワークフロー（`.github/workflows/ci.yml`）で 3 OS の build/test を確認。ローカルは Windows 主検証 |

## Task Breakdown

| Task | 内容 | 対応 Phase |
| --- | --- | --- |
| `implement-plugin-core-foundation`（仮称。task-doc 作成時に確定） | plugin-core クレート作成・規格中立型・ライフサイクル状態機械・RT ラベル・doctest | T1 |
| `implement-planar-borrow-view`（仮称） | ゼロコピー per-channel 借用 view/newtype（配置先確定・契約 doc・`compile_fail` doctest・`rt_` テスト） | T2 |
| `poc-clap-binding-clack`（仮称） | CLAP 最小プラグインのエントリポイント成立 + process 経路ゼロコピー借用 + 発見記録 | T3 |
| `poc-vst3-binding`（仮称） | VST3 最小プラグインのエントリポイント成立 + `channelBuffers32` の unsafe 局所ラップ + 発見記録 | T4 |
| `record-plugin-binding-method-adr`（仮称） | PoC 結果に基づく方式 ADR 作成・spec リンク・ADR 索引更新 | T5 |
| `implement-plugin-standard-adapters`（仮称） | CLAP/VST3 薄いアダプタ完成（状態写像・ABI→中立翻訳・非露出監査） | T6 |
| `sync-docs-and-verify-plugin-foundation`（仮称） | ドキュメント同期・総合検証スイート・status/changes 同期 | T7 |

分割ルール: task-doc は次ステップで本表の 1 行ずつに対応して作成し、plan の `implemented-by` から相互リンクする。各 task は依存グラフの順序制約（T2←T1、T3/T4←T1+T2、T5←T3+T4、T6←T5、T7←全）を `depends-on`/`blocks` で表現する。

## Verification

| ID | 対象 | 検証コマンド / 確認 |
| --- | --- | --- |
| V-1 | T1（plugin-core） | `mise exec -- cargo test -p ruviform-plugin-core`（ライフサイクル遷移規則・規格中立型の不変条件を含む）・`mise exec -- cargo clippy -p ruviform-plugin-core --all-targets -- -D warnings`・規格中立性は語彙 grep ではなくコード要素監査で確認: `use` 行に `clack`/`vst3`/`clap_sys` 等の binding クレートへのパスがないこと・`mise exec -- cargo metadata --format-version 1` で `ruviform-plugin-core` から binding クレートへの依存辺がないこと・`cargo doc` の公開面シグネチャに規格固有型が現れないこと（非目標宣言として doc コメント内に "CLAP"/"VST3" の語を含むことは明示的に許容）・`grep -c "#\[non_exhaustive\]" crates/ruviform-plugin-core/src/*.rs` と公開 enum 数の照合・`**RT contract:**` ラベル存在確認 |
| V-2 | T2（借用 view） | `mise exec -- cargo test -p <配置クレート>`（独立チャンネル構築・読み書き・`rt_` テストを含む）・`compile_fail` doctest で借用のスコープ脱退がコンパイル阻止されること・clippy `-D warnings` で局所許可のない `unsafe` が混入しないこと・canonical planar f32 との変換が f32 経路でゼロ回であることのドキュメント確認 |
| V-3 | T3（CLAP PoC） | `mise exec -- cargo test -p ruviform-plugin-clap`・エントリポイント経由の構築実証・`ChannelPair` per-channel スライス → view ラップの読み書き確認・`mise exec -- cargo build -p ruviform-plugin-clap` で cdylib 成果物（`*.dll`/`*.so`/`*.dylib`）が生成され、エクスポートシンボルに `clap_entry` が存在すること（`nm`/`dumpbin` 等で確認、または dev-dep のホストハーネスでロード実証。`cargo test` 単独ではこの検証を満たさない）・発見記録が方式 ADR 入力として揃っていること |
| V-4 | T4（VST3 PoC） | `mise exec -- cargo test -p ruviform-plugin-vst3`・`GetPluginFactory` エントリの成立確認・`channelBuffers32` → view ラップの読み書き確認・`mise exec -- cargo build -p ruviform-plugin-vst3` で cdylib 成果物が生成され、エクスポートシンボルに `GetPluginFactory`/`ModuleEntry` 系が存在すること（`nm`/`dumpbin` 等で確認、または dev-dep のホストハーネスでロード実証。`cargo test` 単独ではこの検証を満たさない）・SAFETY コメント付き局所 `#[expect(unsafe_code)]`（未対応なら `#[allow]`）のみで `-D warnings` 通過・発見記録が揃っていること |
| V-5 | T5（方式 ADR） | `node .agents/skills/doc-status/scripts/doc_status.js audit --type adr --json` で 0 findings・`lint --rule broken-link --json` で 0 findings・`docs/adr/README.md` に新規行・spec front matter に方式 ADR リンク |
| V-6 | T6（アダプタ） | `mise exec -- cargo test -p ruviform-plugin-clap` / `-p ruviform-plugin-vst3`（規格遷移→状態機械写像の単体テストを含む）・外部型非露出を監査: `pub` シグネチャの `clack`/`vst3`/`clap_sys` パターンに加え、`use vst3::…` 等の `use` 行・re-export による短縮名での露出も対象とする（`grep -rn "use \|pub use " crates/ruviform-plugin-clap/src crates/ruviform-plugin-vst3/src` と `cargo doc` の公開面シグネチャを監査し、`pub.*vst3` パターンのみの grep に限定しない）・clippy `-D warnings` |
| V-7 | T7（ワークスペース全体） | `mise run fmt-check`・`mise run check`・`mise run lint`・`mise run test`・`mise run test-rt`・`mise run bench`・`mise exec -- cargo metadata --format-version 1 --no-deps`（9 パッケージ `publish` が `[]`・依存辺が plugin 系→Domain foundations+Primitives のみ）・`mise exec -- prek run --all-files` |

振る舞いレベルの保証（ライフサイクル遷移規則・ゼロコピー借用契約・RT/no-alloc 契約・規格中立性の不変条件・ライフタイム束縛）は test-spec [plugin-foundation.md](../test-specs/plugin-foundation.md)（`TSPEC-034Jg6YSP6CLjJHnFGG5oL`・`relations.verified-by` で相互リンク済み）が保持し、ここでは step ごとの検証入口のみを管理する。

## Rollback Strategy

| 対象 | 戻し方 | 影響 | 備考 |
| --- | --- | --- | --- |
| 本変更全体 | `git revert` で対象 commit を revert（squash マージなら merge commit `-m 1`） | 3 新規クレート・方式 ADR・members/README/AGENTS 記述が一括で巻き戻る | 新規追加中心で既存 API を壊さないため revert はクリーン |
| 部分的戻し | クレート単位 revert（`crates/ruviform-plugin-*` の削除 + `members` 行の除去） | T1–T2 完了品は独立に残せる。アダプタ削除時は binding 依存も `Cargo.toml`/`Cargo.lock` から除去 | T2 が audio-core 配置の場合、view 削除は additive API の除去で既存コードに影響しない |
| 方式 ADR の撤回 | ADR の status を `rejected`/`superseded` に更新（文書は削除しない — doc 規約の可変性ルールに従う） | 採用した binding 方式の記録のみ変更。コードは別途 revert | T6 完了後の撤回はアダプタ差替えを伴うため別 plan で扱う |

## Open Questions / Gaps

- 借用 view の配置先は lean（`ruviform-audio-core`）を記録したが、最終確定は T2 の実装時判断（spec が「実装で確定」と明示）。確定結果と根拠は crate doc と本 plan に追記する
- 借用 view の正確な型形状（`&'a mut [&'a mut [f32]]` リスト型 vs 逐次アクセス型）と命名（仮称 `PlanarChannels`/`PlanarChannelsMut` 系）は T2 で確定する（discovery Open Questions と同じ未確定事項）
- CLAP PoC の検証ハーネス方針（`clack-host` dev-dependency によるエントリ経由インスタンス化 vs エントリ構造体の直接駆動）と VST3 PoC のエントリ検証手段（自前最小 COM ハーネス vs `vst3-host` 等の検証専用 dev-dep）は T3/T4 で実測して選択する
- `vst3` クレートのビルド時 SDK ヘッダ要否・生成バインディングの SDK 版（MIT 3.8+ 前提）は T4 の最初の確認項目として未確定のまま残す
- ~~`ruviform-plugin-core` の `ruviform-rt` 依存は `RtContext` 連携の要否で変わり得る~~ → **決定（closed）**: 現段階では plugin の process 系 API は doc 契約 `**RT contract:**` ラベルのみを負い、`RtContext` capability の受け入れは RT 契約実装 Issue に委譲する。将来の capability 注入を破壊的変更なく吸収できるよう、process API の context 引数は opaque/予約型として確保する（File Map・T1 参照）。この決定により `ruviform-rt` 依存は現時点では付けず、T3/T4 は本件の未確定でブロックされない
- Issue #22 の RT alloc 検証基盤は現時点で未整備の見込み。`rt_` テスト + ドキュメント契約の代替で進め、基盤整備後に自動検証へ移行する（spec Deferred Design Concerns と同一扱い）

## Self-Review

- [x] spec の Requirements・受け入れ基準（ゼロコピー借用 5 件・基盤/統合 9 件・PoC/方式 ADR 2 件）が T1–T7 と V-1〜V-7、7 タスクに対応している
- [x] plugin ADR（分離構造・対称性・binding 保留）・RT ADR（process の RT パス・API 単位ラベル）・設計ベースライン（ライフサイクル・opaque 予約・crate 表）・enum ADR（`#[non_exhaustive]`）の規則が実装・検証に反映されている
- [x] TBD / TODO / 「後で実装」のプレースホルダは残っていない（据え置き項目は spec Deferred Design Concerns と Open Questions / Gaps に記録）
- [x] ファイルパス・relation・status・用語が spec/ADR/タスク規約と整合している（plan status は `proposed` が型として無効のため `draft` — Review Handoff 参照）

## Implementation Handoff

- Recommended mode: delegated or subagent-capable implementation（T3 と T4 は相互独立の PoC で並行余地あり。それ以外は依存グラフ上の直列順序を維持するのが安全）
- Independent work streams: T3 ‖ T4（両方 T1・T2 完了後に並行実行可能。ただし `members` への同時追加で `Cargo.toml` の競合に注意 — 並行実行時は統合時に調停する）
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 前提（`relations.depends-on`）: [implement-issue-18-data-model.md](implement-issue-18-data-model.md)（`PLAN-034J5K603qXDNmT82wI9v7`・completed）で確定した `TimeBase`/`MediaTime`・`AudioBuffer` 借用モデル・`TransportSnapshot` の opaque `reserved` パターン・`**RT contract:**` 規則を本実装の前提表面とする。`dev-foundation-bootstrap.md` 系の前提は issue-18 plan の depends-on 連鎖で既に担保されるため直接列挙しない
- 明示的な仮定: 上流 spec は `proposed`（未承認）で、内容は現行版を正本とする。借用 view の配置 lean は `ruviform-audio-core`。アダプタ公開面は最小（ベースライン crate 表「規格固有部の公開面は最小」）。PoC コードは実クレート内（scratch ではない）に置く
- status の注記: 依頼では `proposed` を求められたが、リポジトリの doc スキーマで plan の有効 status は `draft`/`approved`/`in-progress`/`blocked`/`completed`/`superseded` のみ（`proposed` を設定すると `invalid-status` lint 検出になる）ため `draft` とした。未承認・レビュー待ちの意味合いは `draft` が担う
- 未解決の依存: Issue #22 の RT alloc 検証基盤は本 plan の範囲外（代替手段を T2/T3/T4 に明記済み）。方式 ADR で binding 不適が判明した場合の退路（`clap-sys` 自前ラッパー/自前 COM 層）はスコープ見積りを ADR に併記し、実装は別 plan で扱う
- 上流参照: spec `SPEC-034JcgdAKPNeh1YCFyprzh`、design `DESIGN-034IPynWe2V30FLubzQenb`、plugin ADR `ADR-034IO01bG8AyjIJWxHeL77`、RT ADR `ADR-034IO00JjhxeTXUbCZtcDX`、enum ADR `ADR-034J6OnZouPI8HQy6cIg9D`、discovery `DISC-034JccH31pBpKOSW5fst0g`、Issue #29・#41・#22
