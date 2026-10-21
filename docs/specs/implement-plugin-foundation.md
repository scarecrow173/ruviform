---
id: "SPEC-034JcgdAKPNeh1YCFyprzh"
type: "spec"
status: "implemented"
title: "プラグイン基盤の実装（ruviform-plugin-core + CLAP/VST3 アダプタ・binding 方式 PoC）"
created: "2026-10-20"
updated: "2026-10-22"
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
  depends-on:
    - "implement-media-data-model-api.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "../adr/plugin-architecture-clap-vst3.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md"
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../discovery/evaluate-plugin-binding-options.md"
  derived-by:
    - "../plans/implement-plugin-foundation.md"
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

# プラグイン基盤の実装（ruviform-plugin-core + CLAP/VST3 アダプタ・binding 方式 PoC）

## What Should Be Built

GitHub Issue #29 の成果物として、[plugin ADR](../adr/plugin-architecture-clap-vst3.md) と[設計ベースライン](../designs/ruviform-architecture-design-baseline.md)「プラグインアーキテクチャ」節の確定範囲を実装する。構成は「規格中立の共通処理基盤」+「規格ごとの薄いアダプタ」+「binding 方式の PoC 評価と方式 ADR」の 3 要素。

- **`ruviform-plugin-core`（拡張層。依存は Domain foundations + Primitives のみ・`ruviform-audio-core` / `ruviform-media-core` を想定）**:
  - 規格中立のプラグイン基盤 API: プラグイン記述・パラメータ記述/値・sample-accurate イベント・オーディオバス/ポート記述。規格固有の型・概念を含まない
  - ライフサイクル状態機械: `create → init → activate（確保・prepare）→ processing（RT）→ deactivate → destroy` を共通基盤で定義する（ベースライン確定範囲）
  - 規格中立トランスポート/イベント型: 音楽時間（小節/拍/PPQN・テンポ）を持たず、サンプル・媒体時刻（`TimeBase` + 整数カウント）のみを扱う。将来の音楽時間マッピングを ABI 変更なく吸収する opaque 拡張予約フィールドを設ける（`TransportSnapshot` の `reserved` と同一方針）
  - **独立 per-channel planar バッファのゼロコピー借用 view/newtype**: CLAP/VST3 の `process` が供給する「チャンネルごとに独立したポインタ/スライス列」を所有権移転なしでラップする型。既存 `AudioBuffer` / `AudioBufferView` / `AudioBufferViewMut`（`crates/ruviform-audio-core/src/buffer.rs`）はチャンネルメジャー連続ストレージ前提で非適合のため新設する（配置先は plugin-core または audio-core。実装で確定）
  - 公開 API ごとに `**RT contract:**` ラベル（`RT-suitable` / `non-RT only`）を付す
- **`ruviform-plugin-clap` / `ruviform-plugin-vst3`（薄い規格アダプタ）**: 規格 ABI ↔ 共通基盤型（バッファ・時刻・パラメータ・イベント）の一方向翻訳、規格エントリポイント、共通ライフサイクル状態機械への状態写像。binding クレート（`clack`・`vst3` 等）の利用はアダプタの実装詳細とし、外部型を公開 API に露出しない
- **binding 方式 PoC**: 両規格で最小プラグインのエントリポイント成立と `process` 経路のゼロコピーバッファ借用を実測する（評価計画は [discovery](../discovery/evaluate-plugin-binding-options.md) 参照）
- **binding 方式 ADR**: 両規格の PoC 結果に基づき構築方式（binding クレート採用 vs 自前実装、規格ごとの選択）を決定記録する。`docs/adr/` に作成し本 spec からリンクする

## Why Now

- Issue #1 設計ベースライン「後続実装 Issue の提案一覧」から起票された Phase 後半の拡張層実装。plugin ADR で構造・ABI 境界・ライフサイクル・RT 適用が確定済みで、残る未確定構造要素は binding 方式のみ（ADR が後続評価に保留）
- Issue #18 フォローアップ（#41 トリアージ課題 1）で「CLAP/VST3 `process` 由来の独立 planar バッファのゼロコピー借用」が本 Issue の受け入れ条件として明示的に委譲された。既存 `AudioBuffer` 系では表現不可能な借用形状であり、plugin 基盤の API 設計と同時に固定しないと後で破壊的変更になる
- `ruviform-plugin-host` / `-export` の本格実装や GUI 統合などの後続 Issue が、共通基盤の型・ライフサイクル契約・RT 契約ラベルを前提にする。先に基盤を確定させる

## Users and Value

- **Ruviform 上でプラグインを制作する開発者**: 規格中立 API に対して実装するだけで CLAP/VST3 の両規格にエクスポートできる基盤が得られる
- **ホスト機能の実装者（後続 Issue）**: 外部プラグインのロード・実行に使う共通契約（バッファ借用・イベント・ライフサイクル）が確定する
- **DSP・基盤の利用者**: `process` 経路で alloc なし・借用のみと明示された RT 適合 API を利用できる
- **レビュアー・後続作業者**: 方式 ADR とゼロコピー受け入れ条件により、binding 選定と RT 契約遵守を文書・テストの両面で検証できる

## Scope

### In Scope

- `ruviform-plugin-core` クレートの作成とワークスペース統合（規格中立 API・パラメータ/イベント・ライフサイクル状態機械・RT 契約ラベル・規格中立トランスポート/イベント型）
- 規格中立の**プラグイン処理契約**（prepare/process 形状を持つ、プラグイン作者が実装する trait。仮称 plugin-core 処理契約 = `PluginProcessor` 相当）— 所有は `ruviform-plugin-core`。アダプタはこの契約を実装・駆動する側であり、契約自体を定義しない。`ruviform-rt::Processor` / `RtContext` は上流の未実装プレースホルダのため本 Issue では plugin-core が独自契約を定義し、将来の統一は RT 契約実装 Issue に委譲する（Deferred Design Concerns 参照）
- 独立 per-channel planar バッファのゼロコピー借用 view/newtype（配置先は実装で確定 — plugin-core または audio-core）
- `ruviform-plugin-clap` / `ruviform-plugin-vst3` アダプタの作成（薄い ABI 変換・エントリポイント・ライフサイクル写像。ホスト/制作の対称性を阻害しない境界）
- binding 方式 PoC（両規格で最小プラグインエントリポイント + `process` 経路のゼロコピーバッファ借用を実装・検証）
- 方式 ADR の作成（`docs/adr/`。両規格の PoC 結果に基づく）
- 実装順序は技術的依存・効率で決定する（同時実装を要求しない — Issue #29 本体準拠）

### Out of Scope

- プラグイン GUI（ui ADR の内側実装・`baseview`/`wxp` 等の GUI 基盤選定を含む — ベースライン「要 PoC」/後続 Issue）
- OFX・映像プラグイン（範囲・採否は別途設計 — ADR で未確定のまま管理）
- サンドボックス/クラッシュ分離の具体方式（ADR が後続に保留。インプロセス既定と「分離を阻害しないハンドル+直列化境界」の構造制約のみ遵守）
- 音楽時間（小節/拍/PPQN・テンポマップ）— 型は持たず opaque 予約フィールドのみ（時間モデル節と同一方針）
- ホスト機能・制作機能の完全実装（本 Issue は共通基盤 + アダプタ PoC。`ruviform-plugin-host` / `ruviform-plugin-export` の本格実装は後続 Issue）
- f64 サンプルの本格対応（VST3 `symbolicSampleSize`/CLAP 64-bit 経路は型形状上の予約のみ。RT 正準は f32 — データモデル spec と同一方針）
- AU / AAX / LV2 等の追加規格（アダプタ構造上の追加経路は残すが実装対象外）
- パラメータ自動化・状態保存・拡張機能の網羅的実装（PoC と受け入れ条件に必要な最小限に留める）

## Requirements

- Rust edition 2024・MSRV 1.85・stable のみ。`publish = false`（workspace 継承）、`[lints] workspace = true`
- `unsafe` は原則禁止。必要箇所（VST3 生ポインタ列のスライス化等）は SAFETY コメント付き局所許可（`#[expect(unsafe_code)]`、未対応なら `#[allow(unsafe_code)]`）に限定する（暫定規約）
- `ruviform-plugin-core` は規格固有の型・概念・binding クレート型を持たない。外部クレート（`clack`・`vst3` 等）の型・エラー・feature 名を公開 API に露出しない（media-io ADR の非露出規則と同一方針）
- 依存方向は上位→下位のみ・循環禁止。plugin 系は拡張層として Domain foundations + Primitives にのみ依存し、plugin 系と ui 系の相互依存を作らない
- クレート外から到達可能な全公開 enum に enum レベルの `#[non_exhaustive]` を付与する（ADR 確定済み・再裁定不要）
- `process` 系コールバックはホスト・プラグイン双方の RT パスとして RT 契約を適用する。RT パスで alloc/dealloc・ブロッキング操作・I/O・無制限待機/リトライ・非有界処理を禁止し、保証範囲は API 単位で `**RT contract:**` ラベルを付す
- 音声 canonical は planar f32（interleaved は境界のみ）。時間モデルは `TimeBase` + 整数カウントを使い、f64 秒をコア表現にしない
- ゼロコピー借用 view/newtype は、借用のライフタイムが `process` コールバックスコープに束縛される型設計とし、「借用中にホスト側が解放・再配置しない」前提契約をドキュメント化する
- in-place 処理（ホストが入出力に同一バッファを渡す場合 — CLAP `in_place_pair` 相当）では、アダプタはエイリアスした同一領域上に `&[f32]` と `&mut [f32]` を同時に構築してはならない。アダプタは境界でポインタ一致を検出し、専用の in-place view 分岐またはスクラッチ経路を選択する（採用方式は plan T2・方式 ADR の評価事項として記録する）。スクラッチ経路を採用する場合、データバウンス用バッファもチャンネルリスト用スクラッチと同様に activate/prepare 時に事前確保する per-instance 状態とする
- `*mut *mut f32` 等のポインタ列をスライス view 化するために必要なチャンネルリスト用スクラッチは、activate/prepare 時に最大バスチャンネル数で事前確保し、`process` 経路では alloc を行わない
- 借用 view のエッジ契約を定義する: ゼロチャンネル・ゼロフレーム・非アクティブバスの view は空のチャンネルリストとして構築可能とし、サンプルを公開しない。アダプタは null ポインタ上に view を構築してはならない
- `pub unsafe fn` は plugin-core / アダプタの公開 API に置かない（外部型非露出と同階層の規則）。unsafe が必要な処理はクレート内部の局所許可に封じ込める
- 公開 API の例は doctest に書き `cargo test` で検証する。RT-suitable な操作を検証するテストは `rt_` プレフィックスで命名し `mise run test-rt` で選択可能にする
- binding クレート依存の追加は `Cargo.lock` 追跡のまま行い、メンテナンス状況・ライセンス・依存木サイズの確認結果を方式 ADR（採用時）または PoC 記録に残す
- 共通ライフサイクル状態機械は遷移表として定義し、規格の状態遷移との写像規則を含む。遷移表は順方向チェーン `create → init → activate → processing → deactivate → destroy` に加えて次を規定する: (a) `processing → activate` の処理停止遷移（CLAP `stop_processing` / VST3 `setProcessing(false)` は processing 停止後の activate 済み非処理状態へ写像し、`deactivate` は processing 停止を前提とする）、(b) activate ↔ processing の再アクティベート往復を合法とする、(c) init/activate 失敗時の遷移先状態、(d) `destroy` は先行する deactivate を要求する、(e) ホストのプロトコル違反（activate 前の process 呼び出し・processing 中の deactivate 等）に対するアダプタの定義済み拒否
- アダプタ境界はホストが供給する process データを検証する: `numSamples`（負の値、または prepare 済み最大フレーム数超過は拒否。0 フレームはイベントフラッシュ経路として受理する）、`numChannels`（activate 時に確保したスクラッチ容量超過）、`symbolicSampleSize`/`kSample64`（f64 入力は定義済みの拒否/スキップとし、f32 として再解釈しない）、チャンネルポインタのアライメント（f32 は 4 バイト）、null チャンネルポインタ。検証失敗は定義済みの結果（process 呼び出しの拒否・バッファスキップ・エラーステータス）を返し、未定義動作や無言のチャンネルインデックスずらしを生じさせない
- in-place 検出の粒度は per-channel ポインタ対比較とする（バッファ全体/`in_place_pair` の等価判定のみに留めない）。部分エイリアス（`out[i] == in[j]`）や出力チャンネルポインタの重複に加え、開始ポインタが一致しないサンプル範囲（`[ptr, ptr + frames×4)`）の部分的重複も検出し、決定済みの in-place 方式で処理するか定義済みの拒否とする。同一領域上に `&[f32]` と `&mut [f32]`、または 2 つの `&mut [f32]` を構築しない
- `PluginProcessor::process`（規格中立処理契約）は借用した入出力 view を受け取り、そのライフタイムはコールバックスコープに束縛される。ゼロコピー契約は慣例ではなくシグネチャ自体で強制する
- sample-accurate イベントリストは次の意味論を定義する: 順序（サンプルオフセットでソート済み。アダプタは未ソートのホスト入力をソートするか、拒否することを明記する）、オフセット表現（ブロック内フレームオフセットでブロックフレーム数未満に有界 — 絶対 `MediaTime` は wire 表現ではない。ブロック範囲外オフセットのイベントは個別にドロップし、誤ったフレームへのクランプやブロック全体の拒否は行わない）、パラメータ値域（正規化 f64 ↔ 内部表現の写像と NaN ポリシー）、ゼロフレーム時のイベントフラッシュ動作、process ステータス/テイルの中立ステータス型への写像
- 借用 view のチャンネル契約: 1 つの view 内の全チャンネルスライスは均一なフレーム長を共有し（異種長は構築時に拒否）、チャンネル順はバスのチャンネルインデックス順に一致し、view のチャンネル数は activate 時にネゴシエートされたバス構成と整合する
- 事前確保スクラッチ（チャンネルリスト・in-place バウンスバッファを含む）はすべてプラグインインスタンスごとの状態とし、`static`/`thread_local` で共有しない

## Acceptance Criteria

Issue #29 本体・コメント（#18 フォローアップ由来・#41 トリアージ課題 1）および plugin ADR・設計ベースラインの確定範囲に基づく。

### ゼロコピー借用（Issue #29 コメントの受け入れ条件）

- [ ] アダプタ層が、プラグイン API（CLAP/VST3 `process`）由来の per-channel ポインタ/スライス列を `AudioBuffer` 系（view/newtype）としてラップできる（所有権移転なしの借用）。単体テストまたは PoC コードで、独立したチャンネルバッファから構築した view が各チャンネルの読み書きを提供することを検証する
- [ ] 借用のライフタイムが `process` コールバックの範囲に束縛される設計が API ドキュメントに明記されている（「借用中にホスト側が解放しない」前提契約を含む）。ライフタイム束縛は型シグネチャで表現され、借用がコールバックを脱退できないことをコンパイルで担保する
- [ ] canonical な planar f32 表現との変換が不要、または境界で 1 回のみであることが確認される（f32 経路では変換ゼロであること。f64 入力への対応は非目標）
- [ ] `unsafe` が必要な箇所は SAFETY コメント付きの局所許可に限定される（暫定規約準拠。`mise run lint` の `-D warnings` ゲートで局所許可のない `unsafe` が混入しないことを確認）
- [ ] `process` パスで alloc/dealloc が発生しないことが RT 契約として文書化・検証される（Issue #22 の検証基盤と連携。基盤未整備の場合は `rt_` テスト + レビュー可能なドキュメント契約で代替し、その旨を明記する）

### 基盤・統合

- [ ] `ruviform-plugin-core` / `ruviform-plugin-clap` / `ruviform-plugin-vst3` の各クレートが作成され、ルート `Cargo.toml` の `members`・依存 DAG・README/AGENTS の記述が実装状態と一致する
- [ ] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` / `bench` と `mise exec -- prek run --all-files` がすべてグリーン
- [ ] クレート外から到達可能な全公開 enum に enum レベルの `#[non_exhaustive]` が付与されている
- [ ] 公開 API に外部型（`clack`・`vst3`・`clap-sys` 等の binding クレート型）が露出していない
- [ ] `ruviform-plugin-core` の公開 API・内部型に CLAP/VST3 固有の型・概念が含まれない（規格中立性）
- [ ] `process` 系 API を含む公開 API に `**RT contract:**` ラベルが付与されている
- [ ] 規格中立トランスポート/イベント型が音楽時間を持たず、opaque 拡張予約フィールドを持つ
- [ ] `cargo test` で doctest + 単体テストが実行・成功する（公開 API の doctest カバレッジを含む）
- [ ] ライフサイクル状態機械（`create → init → activate → processing → deactivate → destroy`）が共通基盤で定義され、両アダプタの状態写像が規格の遷移規則に対応することを単体テストで検証する

### PoC・方式 ADR

- [ ] 両規格で最小プラグインが規格エントリポイント経由で構築され、`process` 経路でゼロコピーバッファ借用が動作することを PoC で実証する
- [ ] 方式 ADR が `docs/adr/` に作成され、両規格の PoC 結果（ゼロコピー可否・unsafe 量・alloc 有無・ビルド要件・MSRV/ライセンス適合）に基づく構築方式決定を記録する。本 spec の `relations` からリンクする

## Deferred Design Concerns

| 論点 | 本 spec での扱い | 判断・移管先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| プラグイン GUI | 非目標。アダプタ境界が GUI 統合を阻害しない構造のみ遵守 | ui ADR 準拠の後続 Issue（`baseview`/`wxp` 等は別 PoC） | 後付けで ABI・ライフサイクル契約が非対称拡張される |
| サンドボックス/クラッシュ分離 | インプロセス既定。インスタンスを「ハンドル + 直列化可能コマンド境界」で設計し分離追加を阻害しない | 後続 Issue（plugin ADR が保留） | インプロセス前提を内側に漏らすと分離実装時に破壊的変更 |
| `ruviform-plugin-host` / `-export` 本格実装 | 両方向で利用可能な共通契約（バッファ借用・ライフサイクル・イベント）のみ確立 | 後続実装 Issue | 制作側のみ前提にするとホスト側追加時に契約が非対称化 |
| 音楽時間（小節/拍/PPQN・テンポ） | opaque 拡張予約フィールドのみ残し型は持たない | 第 2 期・テンポマップ設計 Issue | 後付けで公開イベント/トランスポート型を破壊 |
| f64 サンプル経路 | `symbolicSampleSize`/CLAP 64-bit を型形状上の予約に留める | 需要ベースの後続評価 | f32 前提を暗黙に埋め込むと 64-bit 対応時に API 破壊 |
| `assert_no_alloc` 系 RT 検証レーン | `rt_` テスト + ドキュメント契約で代替可能とする | Issue #22（RT 検証基盤） | 自動検証なしのままでは alloc 違反がレビュー漏れで混入 |
| `ruviform-rt::Processor` / `RtContext` との処理契約統一 | plugin-core が独自の処理契約を定義する（`Processor`/`RtContext` は上流の未実装プレースホルダ。現段階では doc 契約 `**RT contract:**` ラベルのみ。将来の capability 注入を破壊的変更なく吸収できる API 形状にする） | RT 契約実装 Issue（`ruviform-rt` 実装時に統一を再評価） | `Processor`/`RtContext` 前提で設計すると未実装契約に依存し、後付け統一でアダプタ・プラグイン実装の両面が破壊的変更になる |
| パラメータ/拡張の網羅的対応 | 受け入れ条件に必要な最小限 | 方式 ADR 確定後の後続 Issue | 過剰実装が binding 選定前の設計を固定する |

## Sources and Relations

- 一次ソース（`relations.source`）: GitHub Issue [#29 [impl] プラグイン基盤（plugin-core + CLAP/VST3 adapter・binding 方式 PoC）](https://github.com/scarecrow173/ruviform/issues/29)（本体 + ゼロコピー受け入れ条件コメント）。Issue #1 設計ベースライン「後続実装 Issue の提案一覧」から起票された。
- 上流文書（`relations.derives-from`）: [plugin ADR](../adr/plugin-architecture-clap-vst3.md)（提供範囲・同格・分離構造・RT 適用・binding 方式の保留）、[RT 契約 ADR](../adr/realtime-contract-and-verification-policy.md)（RT パス契約・検証方針・API 単位ラベル付け規則）、[設計ベースライン](../designs/ruviform-architecture-design-baseline.md)（プラグインアーキテクチャ節・クレート表・時間/バッファモデル）、[binding 方式評価 discovery](../discovery/evaluate-plugin-binding-options.md)（候補比較と PoC 評価計画）。
- 前提（`relations.depends-on`）: [implement-media-data-model-api.md](implement-media-data-model-api.md)（planar f32 canonical・`AudioBuffer` 系・`TimeBase`/`MediaTime`・`**RT contract:**` ラベル規則を提供する実装済み spec）。
- 関連（`relations.related`）: リポジトリルートの [`AGENTS.md`](../../AGENTS.md)（暫定開発規約・unsafe 原則禁止・lint ゲート・検証入口）。
- 参考（`relations.references`）: Issue [#41](https://github.com/scarecrow173/ruviform/issues/41)（ゼロコピー受け入れ条件のトリアージ元）、Issue [#22](https://github.com/scarecrow173/ruviform/issues/22)（RT 検証基盤・alloc 検証の連携先）、[prokopyl/clack](https://github.com/prokopyl/clack)・[coupler-rs/vst3-rs](https://github.com/coupler-rs/vst3-rs)（binding 候補。事実ベース比較は discovery が正本）。
