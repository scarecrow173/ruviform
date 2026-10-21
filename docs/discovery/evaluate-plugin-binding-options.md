---
id: "DISC-034JccH31pBpKOSW5fst0g"
type: "discovery"
status: "resolved"
title: "プラグイン基盤 binding 方式の評価（Issue #29）"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../adr/plugin-architecture-clap-vst3.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../specs/implement-media-data-model-api.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/29"
    - "../adr/plugin-architecture-clap-vst3.md"
  derived-by:
    - "../specs/implement-plugin-foundation.md"
    - "../plans/implement-plugin-foundation.md"
    - "../adr/plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md"
  verifies: []
  verified-by: []
  references:
    - "https://crates.io/crates/clack-plugin"
    - "https://crates.io/crates/clack-host"
    - "https://crates.io/crates/clack-extensions"
    - "https://docs.rs/clack-plugin/latest/clack_plugin/"
    - "https://github.com/prokopyl/clack"
    - "https://crates.io/crates/vst3"
    - "https://coupler.rs/vst3-rs/vst3/"
    - "https://github.com/coupler-rs/vst3-rs"
    - "https://crates.io/crates/clap-sys"
    - "https://github.com/RustAudio/vst3-sys"
    - "https://crates.io/crates/nice-plug"
    - "https://github.com/robbert-vdh/nih-plug"
    - "https://crates.io/crates/vst3-host"
  defers: []
  deferred-by: []
---

# プラグイン基盤 binding 方式の評価（Issue #29）

## Exploration Goal

GitHub Issue [#29 [impl] プラグイン基盤（plugin-core + CLAP/VST3 adapter・binding 方式 PoC）](https://github.com/scarecrow173/ruviform/issues/29) の briefing 成果を記録する。本書が回答する問い:

- 規格固有アダプタ層の構築方式として、どの binding クレート（または自前実装）が有力候補か。各候補のバージョン・ライセンス・保守状況・API 形状・unsafe 面・RT 適合性の事実ベース比較は何か
- Issue #29 コメントの受け入れ条件（CLAP/VST3 の `process` が供給するチャンネル独立 planar バッファのゼロコピー借用）を各候補が満たせるか
- PoC で何を作り・何を確認すべきか（方式 ADR の入力となる検証計画）

> **位置づけ**: 構造（共通基盤＋規格アダプタ分離・ホスト/制作対称・ABI 境界・ライフサイクル状態機械・RT 契約双方適用）は [plugin ADR](../adr/plugin-architecture-clap-vst3.md) と [設計ベースライン](../designs/ruviform-architecture-design-baseline.md) で確定済み。本書が扱うのは確定範囲外として保留された「binding 方式」（自前実装 vs binding クレート）の評価のみであり、最終決定は PoC 結果に基づく方式 ADR で行う。Lean は仮説であり採否決定ではない。

## Key Issues and Assumptions

- **ゼロコピー借用の適合性**: 既存の `AudioBuffer` / `AudioBufferView` / `AudioBufferViewMut`（`crates/ruviform-audio-core/src/buffer.rs`）はチャンネルメジャー**連続**ストレージを前提としており、プラグイン API が渡す「チャンネルごとに独立したポインタ列」を直接ラップできない。`&mut [&mut [f32]]` 相当の per-channel スライス列を束ねる新しい借用 view/newtype が共通基盤（または audio-core）に必要になる — 前提として確定。
- **RT 契約の引き継ぎ**: `process` 系コールバックはホスト・プラグイン双方で RT パス。binding クレートの process 経路が暗黙に alloc/dealloc・ブロッキングを行わないこと（または回避可能なこと）を確認する必要がある。
- **MSRV 制約**: 本リポジトリは Rust `1.99.0` ピン・MSRV `1.85`（`rust-version`）。候補クレートの `rust_version` がこれを超えないこと。
- **unsafe 方針**: リポジトリ暫定規約は `unsafe` 原則禁止（SAFETY コメント付き局所許可のみ）。binding クレート内部の unsafe は依存として許容しうるが、アダプタ層が書く unsafe の量・性質は評価対象。
- **ライセンス前提**: VST3 SDK 3.8 以降は MIT（plugin ADR で確認済み事実）。生成バインディングの出自（どの SDK 版から生成されたか）と再配布適合性は要確認。
- **ホスト/制作対称性**: ADR は双方を第一ターゲットとする。plugin 側だけでなく host 側にも使える（または対称に構築できる）かは方式選定の因子。

## Alternatives and Comparison

### 主候補（規格別 binding クレート）

| Option | 事実（調査日時点） | Pros | Cons | Lean |
| --- | --- | --- | --- | --- |
| `clack`（`clack-plugin` / `clack-host` / `clack-extensions`） | 0.2.0（2026-09-12）。MIT OR Apache-2.0。`rust_version = 1.85.0`（本リポジトリ MSRV と一致）。repo [prokopyl/clack](https://github.com/prokopyl/clack)（241 stars・最終 push 2026-09-12・2021 年から継続）。内部で `clap-sys` 0.5.0 に依存 | plugin・host **双方**の safe ラッパーを提供（唯一の実用的選択肢）。`Plugin::process(Process, Audio, Events)` が `PortPair` → `channels()?.into_f32()` → `ChannelPair::{InputOnly(&[f32]), OutputOnly(&mut [f32]), InputOutput, InPlace}` で**チャンネル単位の `&[f32]`/`&mut [f32]` スライスを直接返す**（ゼロコピー・process スコープに束縛）。`clack-extensions` が params/gui/audio-ports/state/thread-check/thread-pool 等を feature 分割提供 — thread-check は RT スレッド契約の検査機構と整合。`clack_export_entry!` で cdylib エントリポイント生成 | 若いクレート系（0.2.0 は 2026-09 リリース）で API churn リスク。内部に FFI unsafe を含む（依存としての許容判断が必要）。拡張網羅度は要確認 | yes（CLAP 第一候補） |
| `vst3`（coupler-rs/vst3-rs） | 0.3.0（2025-12-07）。MIT OR Apache-2.0。`rust_version` 未宣言。repo [coupler-rs/vst3-rs](https://github.com/coupler-rs/vst3-rs)（79 stars・最終 push 2026-03-28）。生成バインディングはリポジトリにコミット済み（`generate/` + `vst3sdk` サブモジュールは再生成用） | VST3 API 全域の生成バインディング + COM 足場（`ComPtr`/`ComRef`/`ComWrapper`・`Class` trait・各インターフェースの `XxxTrait`）。プラグイン実装側とホスト側インターフェース双方をカバー。`ProcessData`（`#[repr(C)]`）は `inputs/outputs: *mut AudioBusBuffers`・`numSamples`・`symbolicSampleSize`（kSample32/kSample64）を持ち、`AudioBusBuffers.channelBuffers32: *mut *mut f32` の**チャンネル独立ポインタ列**をアダプタ側で `&mut [f32]` 化すればゼロコピー可能 | **safe ラッパーではない**（「these bindings are unsafe, and no attempt is made to abstract over the VST 3 API itself」— 全 trait メソッドが `unsafe fn`）。アダプタ側の SAFETY コメント付き unsafe 量は clack 側より多くなる。メンテナ規模が小さい | yes（VST3 第一候補） |

### 代替案（比較・文脈用）

| Option | 事実 | Pros | Cons | Lean |
| --- | --- | --- | --- | --- |
| `clap-sys` 0.5.0（直接利用） | 2025-01-03・MIT/Apache-2.0・`rust_version = 1.64`。clack の下位層 | 生 C ABI に最も近く制御最大 | 全関数 `unsafe`。安全ラッパー・エントリポイント・拡張機構を自前で再実装するコスト（clack が既に提供する層の再発明） | no（clack が不適と判明した場合の退路としてのみ） |
| `vst3-sys`（RustAudio） | crates.io 未公開（git リポジトリのみ）。ライセンス NOASSERTION（GPLv3 時代の SDK 由来汚染の疑い）。最終 push 2023-06-19 で事実上放置 | — | ライセンス不明確・未保守。VST3 SDK 3.8 MIT 化で存在意義が消滅 | no |
| `nih-plug` → 後継 `nice-plug` | `nih_plug` は crates.io 未公開（後継 `nice-plug` 0.4.2・2026-09-14・ISC・codeberg RustAudio/nice-plug・`rust_version = 1.88`）。repo [robbert-vdh/nih-plug](https://github.com/robbert-vdh/nih-plug)（3000 stars・活発） | 成熟したフルフレームワーク（params/state/GUI/バッファ抽象・CLAP+VST3 export）。process モデルの参照実装として価値大 | フレームワーク全面依存は ADR で「設計主導権の従属」として不採用方針（再裁定は方式 ADR の範囲）。MSRV 1.88 > リポジトリ MSRV 1.85 で現行不適合 | no（参照実装としてのみ） |
| 自前 FFI（`clap-sys` + 独自 VST3 COM 層） | — | 依存ゼロ・完全制御・外部型非露出が自明 | VST3 は COM + 多数インターフェースで自前コスト大。clack/vst3 が提供する層の再発明 | no（PoC で両候補が不適と判明した場合の退路） |
| `vst3-host` 0.10.0 | 2026-10-09・「Safe, simple VST3 plugin hosting」。repo HelgeSverre/rust-vst3-host | ホスト側の safe ラッパーとして新規選択肢 | 極新規・単一作者・実績薄。プラグイン制作側は対象外 | maybe（ホスト側実装時の参考候補。本 PoC の主対象外） |
| `truce` | 独自ライセンス（Issue #1 探索で確認済み） | 7 形式対応の上位フレームワーク | コード流用不可・設計参照のみ | no |

### ゼロコピー適合性（受け入れ条件との対応）

- **clack**: `ChannelPair` が per-channel の `&[f32]`/`&mut [f32]` を返すため、新 view/newtype へのラップはスライスの再束縛のみで可能（unsafe 不要な可能性が高い。要 PoC 確認）。借用ライフタイムは `process` 引数由来でスコープに束縛される。
- **vst3**: `channelBuffers32` の `*mut *mut f32` から `&mut [f32]` 列を構築する必要がある（`numSamples`・`numChannels`・`symbolicSampleSize` の検証を含む SAFETY コメント付き局所 unsafe）。同様に借用は process コールバックスコープに束縛可能。
- **変換回数**: 両規格とも canonical planar f32 と同一レイアウト（CLAP は 32-bit float 必須・64-bit 任意、VST3 は `symbolicSampleSize` で f32/f64 選択）のため、f32 経路では変換不要。f64 入力は本 Issue では非目標（境界 1 回変換は将来拡張）。
- **in-place 処理とスライス化スクラッチ（追記の findings）**: clack は入出力同一バッファを `ChannelPair::InPlace` として露出するため、アダプタは境界でポインタ一致を検出し、エイリアスした同一領域上に `&[f32]` と `&mut [f32]` を同時に構築しないよう分岐（専用 in-place view 分岐またはスクラッチ経路）が必要。`vst3` の `channelBuffers32` は `*mut *mut f32` であり、スライス view 化にはアダプタ側のチャンネルリスト用スクラッチが必要 — activate/prepare 時に最大バスチャンネル数で事前確保し、process 経路で alloc しない設計が spec 契約として要求される。
- **RT**: clack は「allocation を最小化」を設計目標に明記。process 経路で alloc が発生しないことは `assert_no_alloc` 系レーン（Issue #22 連携）で検証する。

## PoC 評価計画

方式 ADR の入力とするため、両規格で最小限のエンドツーエンドを構築し以下を確認する（詳細な受け入れ条件は spec [implement-plugin-foundation.md](../specs/implement-plugin-foundation.md) が保持）。

1. **CLAP PoC**（`clack` 想定）: `clack_export_entry!` による最小ゲインプラグイン（cdylib）。`process` 内で `ChannelPair` の per-channel スライスを共通基盤のゼロコピー view/newtype にラップして処理。確認: エントリポイント生成・ホストからのロード可否（最小ホストまたは `clack-host` 経由）・借用が process スコープに束縛されること・process パスで alloc なし・unsafe がアダプタ側に不要か。
2. **VST3 PoC**（`vst3` 想定）: `Class`/`ComWrapper` で `IPluginBase`/`IComponent`/`IAudioProcessor`/`IEditController` 最小実装 + `GetPluginFactory`/モジュールエントリ。`process(*mut ProcessData)` で `channelBuffers32` を SAFETY コメント付き unsafe で同じ view/newtype にラップ。確認: エクスポートシンボルとビルド要件（SDK ヘッダがビルド時に要るか）・ゼロコピー成立・alloc なし・unsafe の量と局所性。
3. **横断確認項目**（方式 ADR の判断材料）: バージョン・ライセンス・`rust_version` ≤ 1.85・内部 unsafe 面・アダプタが書く unsafe の量・ゼロコピー可否・process 経路の alloc 有無・ホスト側提供の有無（clack-host / vst3 の COM 全般）・拡張カバレッジ（params・events・thread-check 等）・保守頻度・ビルド複雑さ・CI 適合性。
4. **フォールバック条件**: clack が CLAP 側で不適なら `clap-sys` 直上の自前薄ラッパー、vst3 が不適なら自前 COM 層の範囲見積りを方式 ADR に併記する。

## Tentative Conclusions and Hypotheses

| 仮説 | 根拠 | 確度 |
| --- | --- | --- |
| CLAP 側は `clack` が最有力（safe・plugin/host 双方・per-channel スライス・thread-check・MSRV 一致） | 上記事実表 | 高（要 PoC: process での alloc 有無と API churn の実測） |
| VST3 側は `vst3`（coupler-rs）が最有力（唯一の保守された生成バインディング + COM 足場）だが safe 抽象は自前 | 上記事実表 | 中〜高（要 PoC: unsafe 量・ビルド要件・エントリポイント成立） |
| 規格ごとに異なる構築方式（CLAP=safe ラッパー / VST3=生成バインディング上の自前薄アダプタ）でも ADR の「薄いアダプタ」構造と矛盾しない | 分離構造は決定済み、構築方式は規格単位の実装詳細として許容される | 中 |
| ゼロコピー受け入れ条件 5 項目は両候補で達成可能 | 両者とも per-channel ポインタ/スライスを提供し変換不要 | 中（PoC で実測） |
| `nice-plug`（nih-plug 後継）は参照実装の価値は高いが、全面依存採用は MSRV 不整合（1.88 > 1.85）と ADR 方針の両面で現実的でない | crates.io `rust_version`、plugin ADR Pros/Cons | 高 |

## Open Questions

- `vst3` クレートのビルドが VST3 SDK ヘッダ/サブモジュールをビルド時に要求するか — crates.io のメタデータと docs.rs のビルド成功から、生成済みバインディングがクレート内同梱で SDK ビルド要件はない見込みが強い。残余確認として PoC 開始時に実測で確定する
- `vst3` の生成バインディングがどの SDK 版から生成されたか、MIT（3.8+）版由来か（licensing ADR の個別調査手順に従い確認）
- `clack` 0.2.x の API 安定度と拡張カバレッジが必要範囲（params・events・audio-ports・thread-check・gui）を満たすか
- `clack-host` 経由でも process 側のバッファが同様にゼロコピーで包めるか（ホスト側の `Audio`/`Buffer` API 形状）
- ゼロコピー view/newtype の正確な型形状（`&mut [&mut [f32]]` リスト型 vs 逐次アクセス型）— spec/実装で確定
- `assert_no_alloc` 系の alloc 検証レーン整備は Issue #22 のスコープ（本 PoC では簡易検証または手動計測で代替するか要調整）
- VST3 側のエントリポイント検証手段（最小ホストを自前で COM 呼び出しするか、`vst3-host` 等を検証専用 dev-dependency とするか）

## PoC Findings — CLAP `clack` 0.2.0（T3 実測・2026-10-21）

`crates/ruviform-plugin-clap`（cdylib+rlib、最小ゲインプラグイン）で実測した方式 ADR 入力:

| 評価項目 | 実測結果 |
| --- | --- |
| 採用バージョン | `clack-plugin`/`clack-common`/`clack-extensions` `=0.2.0`（`clap-sys 0.5.0` 同梱）・dev-dep `clack-host =0.2.0` |
| ゼロコピー可否 | **達成** — `ChannelPair` の per-channel `&[f32]`/`&mut [f32]` を `PlanarBufferView`/`PlanarBufferViewMut` で直接ラップ。`clack-host` ハーネス経由のテストでポインタ一致を実証（view がホストバッファを借用） |
| unsafe 量・局所性 | **アダプタ記述 unsafe = 0**（`clack_export_entry!` マクロ生成の static のみ内部 allow）— 候補評価で最良の結果 |
| process 経路 alloc | **なし** — 全スクラッチ（`BlockScratch`: チャンネルポインタ簿・イベント列）を `activate` で `with_capacity` 事前確保・`clear()` で容量保持。イベント未ソート時の整列は `sort_unstable_by_key` の in-place ソート（補助 alloc なし） |
| in-place 処理 | `ChannelPair::InPlace` を clack が分類 → アダプタはその分類を写像（独自ポインタ比較なし）。専用分岐: in-place ペアは読み側 view を空にし出力を入出力として処理（`&`+`&mut` 同時構築を構造上回避）。部分エイリアス・重複出力は境界検証で reject |
| ビルド要件 | **追加ツールチェーン不要**（clang/cmake/SDK なし・crates.io 取得のみ）。Windows x64 で `clap_entry` エクスポートを `.dll` に確認 |
| MSRV/ライセンス | `rust_version 1.85` の workspace 制約内でビルド成功。ライセンス MIT OR Apache-2.0（ポリシー適合） |
| 拡張カバレッジ | 使用: `audio-ports`・`params`（`PluginAudioPorts`/`PluginParams`+`in_place_pair` 宣言）。clack は params/events/thread-check/gui 等を feature 分割で提供 |
| 保守状況 | clack 0.2.0 — crates.io 直近系の活発なリリース（評価時点）。API churn は要監視（0.x のため exact pin で固定、ADR で緩和可否を記録） |
| 依存木サイズ | `cargo tree --prefix none | sort -u` = **15 パッケージ**（workspace クレート含む・host dev-dep 除く本経路） |
| ライフサイクル写像 | CLAP init/activate/start_processing/process/stop_processing/deactivate/destroy → `Lifecycle` 遷移表へ 1 対 1 写像。プロトコル違反は `PluginError::Message` → `CLAP_PROCESS_ERROR` で reject（`AtomicU8` の `AtomicLifecycle` で main/audio スレッド共有） |
| 境界検証 | `frames_count`>max_frames・チャンネル数>容量・f64-only/混在ペア・null ポインタ（frames>0）・非整列ポインタ・部分エイリアス・重複出力を定義済み reject/skip — spec 境界検証表と一致 |

- 採否推奨: **CLAP 側は `clack` 採用を推奨**（unsafe ゼロ・ゼロコピー達成・追加ビルド要件なし・spec 契約を全て満たす最小 PoC が成立）
- 留意点（ADR 記録対象）: 0.x API churn → exact pin で管理・緩和は方式 ADR で記録。`sort_unstable` の同オフセット順序差は単一パラメータ PoC では benign（複数パラメータでは host 順序差が可観測になりうる — T6 で安定化を検討）。`clack-host` は dev-dep としてテスト駆動に有効

## Promotion Candidates

- [x] spec-doc needed: Issue #29 の実装範囲・受け入れ条件を [implement-plugin-foundation.md](../specs/implement-plugin-foundation.md)（`relations.derived-by` / spec 側 `derives-from` で相互リンク）として作成済み
- [x] adr-doc needed: binding 方式の採否は [plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md](../adr/plugin-binding-method-clack-for-clap-coupler-rs-vst3-for-vst3.md)（`ADR-034JDzseXORnckswW1oYKh`・accepted）として作成・決定済み（CLAP=`clack`、VST3=`vst3`。`derived-by` で相互リンク済み）

## Sources and Relations

- 一次ソース（`relations.source`）: GitHub Issue [#29](https://github.com/scarecrow173/ruviform/issues/29)（本体 + #18 フォローアップ由来のゼロコピー受け入れ条件コメント）。
- 上流文書（`relations.derives-from`）: Issue #29 と [plugin ADR](../adr/plugin-architecture-clap-vst3.md)（binding 方式の採否を後続評価に保留した決定 — 本探索の直接の起点）。
- 関連ドキュメント（`relations.related`）: [plugin ADR](../adr/plugin-architecture-clap-vst3.md)（binding 方式を後続評価に保留した決定の正本）、[RT 契約 ADR](../adr/realtime-contract-and-verification-policy.md)（process パスの契約・検証方針）、[設計ベースライン](../designs/ruviform-architecture-design-baseline.md)（プラグインアーキテクチャ節・クレート表）、[implement-media-data-model-api.md](../specs/implement-media-data-model-api.md)（整合対象の `AudioBuffer` 借用モデルの実装 spec）、[AGENTS.md](../../AGENTS.md)（暫定開発規約）。
- 調査証拠（`relations.references`）: crates.io / docs.rs / GitHub の各候補クレートページ（バージョン・ライセンス・`rust_version`・リポジトリ活動・API 形状の一次情報）。

## PoC Findings — VST3 / T4 実測（2026-10-21 追記）

`crates/ruviform-plugin-vst3`（task `TASK-034JgsHeZt7Bm40uBMo9wg`・IR [../impl/ir/poc-vst3-binding-adapter.md](../impl/ir/poc-vst3-binding-adapter.md)）の実測結果。方式 ADR（T5）の入力として記録する。

### ビルド要件（Open Question の解消）

- **`vst3` 0.3.0 は VST3 SDK ヘッダ・`vst3sdk` サブモジュールをビルド時に要求しない** — 生成済みバインディングがクレート内同梱で、crates.io 依存のみで `cargo build` 完走を実測（Windows x64・pinned 1.99.0）。`generate/` 機構は上流の再生成入力でありビルド入力ではない
- `rust_version` は未宣言 — pinned 1.99.0 では素の `repr(C)`/extern 生成コードとしてビルド可能。MSRV 1.85 単独ビルドは未実測であり、T5/T6 での確認残件とする
- ライセンス: MIT OR Apache-2.0（crates.io メタデータどおり）。生成バインディングの SDK 出自は upstream の `generate/` 参照が要継続調査

### unsafe の量と局所性（実測）

- 生成 trait は **全メソッド `unsafe fn`・レシーバ `&self`**（「no attempt to abstract」の記述どおり）。アダプタ側が負う unsafe は「シグネチャ義務 54 件」＋「局所 `unsafe {}` ブロック 51 件（約半数はテスト COM ダブル）」で、raw メモリ操作を伴う実質ブロックは `block.rs` の 3 箇所（slice 再解釈 ×2・bounce 書き戻し）に集中した
- COM `&self` 契約のため `RefCell<ComponentState>`＋アトミックの内部可変性が必須 — ホストの同一スレッド再入違反は `try_borrow_mut` の定義済み拒否で処理する（レビュー後の確定挙動。panic→FFI 越え unwind はホスト abort のため採用しない）
- edition 2024 固有: `#[no_mangle]` は `#[unsafe(no_mangle)]`（unsafe 属性として `#[expect(unsafe_code)]` が必要）、`&mut x`→`*mut` 暗黙変換は `borrow_as_ptr` で `&raw mut` が必須、生成定数はプラットフォーム依存型で `as` キャスト＋局所許容が要る

### エントリポイント（実測）

- `GetPluginFactory`＋Windows `InitDll`/`ExitDll` を PE エクスポートディレクトリ解析で実在確認（`nm`/`dumpbin` 非搭載環境のため Python パーサで代替 — `vst3-host` 等の dev-dep は不採用）
- **命名不整合の発見**: upstream `vst3` example は macOS フックを `BundleEntry`/`BundleExit` と命名するが VST3 spec（・タスク文言）は小文字 `bundleEntry`/`bundleExit`。spec 名を採用 — upstream example に倣うと spec 不適合になる点は採用時の注意として記録
- macOS/Linux エントリは `cfg` ゲート済みコードとして存在するが、クロスビルド未実測（T6 の残件）

### ゼロコピー・イベント・ライフサイクル（実測）

- `channelBuffers32` の `*mut *mut f32` → `PlanarBufferView{,Mut}` のゼロコピー経路が成立 — 非エイリアス出力チャンネルはホストバッファ直書きをテストで実測。全エイリアスパターン（`out[i]==in[j]`・出力重複）は per-channel 対比較で検出し該当チャンネルのみ activate 時確保の bounce 領域へ逃がす
- `process` のチャンネルリストスクラッチ・イベントスクラッチは activate/`setupProcessing` 時の最大フレーム数で事前確保し process 経路で伸長しない構造（alloc 検証レーンは Issue #22 待ち）
- `IParameterChanges`/`IParamValueQueue` は `ComRef::from_raw` で借用（refcount 非操作・非保持）、ブロック範囲外オフセットはドロップ・stable sort・`MAX_BLOCK_EVENTS` 切り捨ての定義済み挙動（走査も `MAX_BLOCK_EVENTS` キュー/ポイントで有界）
- ライフサイクルは VST3 呼び出し列を中立 `Lifecycle` へ写像（`setActive`→Activate/Deactivate・`setupProcessing`→`prepare`・`setProcessing`→Start/StopProcessing・`terminate`→Destroy）、不正順序は `kResultFalse`
- `kSample64` は f32 再解釈せず定義済み拒否（`setupProcessing`/`process` 双方）。state ストリーム・`IPlugView` GUI は `kNotImplemented` スタブ（T6 へ）

### T5 方式 ADR への示唆

- `vst3` は「唯一の保守された生成バインディング＋COM 足場」として lean どおり実用的 — ただし safe 抽象がなく、`RefCell` 内部可変性・`unsafe fn` 義務・手書きのバリデーション層は規格の COM 本質由来であり、自前 COM 層でも同等に必要（クレート採否より「薄アダプタ上の unsafe 局所化方針」が実質の決定点）
- clack 側（T3）との対称比較点: アダプタが書く unsafe ブロック数・`&self` COM 可変性の有無・エントリ生成の自動度・ホスト側提供の有無・拡張カバレッジ — CLAP 側実測が揃い次第 T5 で判定
