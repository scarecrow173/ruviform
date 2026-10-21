---
id: "DESIGN-034IPynWe2V30FLubzQenb"
type: "design"
status: "approved"
title: "Ruviform architecture design baseline"
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
  implements:
    - "../specs/architecture-design-baseline-deliverables.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../../README.md"
    - "overview.md"
    - "dev-foundation.md"
    - "replace-template-derived-docs-with-ruviform.md"
    - "../discovery/evaluate-plugin-binding-options.md"
  refines: []
  refined-by:
    - "../adr/dsp-simd-backend-adoption.md"
  derives-from:
    - "../specs/architecture-design-baseline-deliverables.md"
    - "../adr/media-io-backend-abstraction.md"
    - "../adr/platform-and-toolchain-targets.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/plugin-architecture-clap-vst3.md"
    - "../adr/ui-architecture-webview-hybrid.md"
    - "../adr/licensing-and-distribution-policy.md"
    - "../discovery/explore-architecture-design-baseline.md"
    - "https://github.com/scarecrow173/ruviform/issues/1"
  derived-by:
    - "../plans/apply-issue-1-architecture-baseline-to-repo.md"
    - "../specs/implement-media-data-model-api.md"
    - "../plans/implement-issue-18-data-model.md"
    - "../test-specs/media-data-model-api.md"
    - "../adr/public-enum-non-exhaustive-policy.md"
    - "../specs/audio-engine.md"
    - "../specs/issue-18-followup-enum-and-time-fix.md"
    - "../plans/issue-18-followup-enum-and-time-fix.md"
    - "../test-specs/followup-enum-compat-and-time-arithmetic.md"
    - "../specs/time-type-extension-framerate-timecode-rounding.md"
    - "../adr/framerate-timebase-separation-and-timecode-extension.md"
    - "time-model-extension.md"
    - "../plans/implement-rt-foundation.md"
    - "../discovery/explore-video-foundation-api-scope.md"
    - "../specs/implement-video-foundation-api.md"
    - "../adr/video-proc-kernel-api-and-color-hdr-scope.md"
    - "../plans/implement-issue-26-video-foundation.md"
    - "../test-specs/video-foundation-api.md"
    - "../specs/implement-media-io-abstraction-and-first-backend-poc.md"
    - "../plans/issue-20-i-o-poc.md"
    - "../specs/implement-plugin-foundation.md"
    - "../plans/implement-plugin-foundation.md"
    - "../test-specs/plugin-foundation.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/2"
    - "https://github.com/scarecrow173/ruviform/issues/3"
  defers: []
  deferred-by: []
---

# Ruviform architecture design baseline

Ruviform の全体アーキテクチャ・クレート構成・RT/オフライン実行モデルの設計ベースライン。GitHub Issue #1 の成果物であり、実装に先立ってレビュー・分割・検証できるよう、レイヤー境界・データモデル・依存方向・実行モデル・性能/安全性契約・未決定事項を明確化する。

## Context

### 位置づけ

- 一次ソースは GitHub Issue [#1](https://github.com/scarecrow173/ruviform/issues/1)。本書は spec `SPEC-034INYy2aDLxse9dy8MgGZ`（[architecture-design-baseline-deliverables.md](../specs/architecture-design-baseline-deliverables.md)）の Requirements A.1–A.8 を満たす主設計文書である。
- 確定済み制約は spec の Requirements B（B-1〜B-7）と、そこから派生した 6 件の accepted ADR（`relations.derives-from` 参照）である。本書の設計案はこれらに反しない。逸脱が必要になった場合は ADR で差分と根拠を明示する。
- エコシステム調査の正本は [explore-architecture-design-baseline.md](../discovery/explore-architecture-design-baseline.md)。調査由来の候補は本書では「仮説」として扱い、採否は分類表で管理する。
- 配置先は `docs/designs/`（design-doc スキルのディレクトリ契約。Issue #1 の「`docs/architecture/` 等、配置先は後決め可」に従う）。本書が Issue #1 の設計成果物の正本であり、[overview.md](overview.md) が全体マップとして本書を参照する。

### 対象と優先順位（spec B-1）

- 第 1 ターゲット: DAW／再生ホスト／プラグイン制作。基礎クレートはエンジン・ホストから独立して単独利用可能であること（**汎用ライブラリ性は絶対条件**）。
- 第 2 ターゲット: NLE／映像制作級（動画再生・同期 → 編集・コンポジット）。

### 判断ステータスの凡例

本書では各設計論点を以下の 3 区分で管理する。**「将来検討」への無分類の先送りはしない**（spec Requirement A.5/A.7）。

- **決定**: 本書の設計案として確定した事項（ユーザー承認済みの ADR 由来の確定を含む）。後続実装 Issue はこれを前提に進める。
- **要PoC**: 設計案は提示するが、採否は PoC・ベンチマーク等の検証結果で確定する事項。判断材料と採否基準を併記する。
- **保留**: 判断材料が不足、または別フォーラム（後続 Issue・配布方針決定等）に権限が属する事項。再着手トリガを明記する。

### 前提（#2・#3 の成果）

- #2（[dev-foundation.md](dev-foundation.md)、approved）の最小ワークスペース・ツールチェーン pin・手動 CI を記述・検証基盤として利用する。これらは再検討可能な暫定足場であり、本書の設計判断を制約しない（差分は「#2 開発基盤への反映」節でタスク化）。
- #3 で Ruviform 固有化された `README.md` / `AGENTS.md` は設計の出発点であり、本書の決定との差分を記録・タスク化する。

## Scope

### In Scope

- 最終的なクレート DAG・責務・公開 API 境界・依存方向（循環依存防止）の設計案の確定（A.1, A.2, A.7）
- 音声・動画・時間のデータモデル統一定義案、メディア I/O 抽象（demux/mux と decode/encode の概念分離）
- RT／オフラインの実行モデル、処理グラフ、スケジューリング採否、各 1 例以上のデータフロー（A.3）
- DSP／SIMD の初期範囲・構造・比較方法、性能検証計画（A.4）
- プラグインの ABI 境界・ライフサイクル・分離方針・GUI 統合方針（plugin ADR の確定範囲）
- UI の責務・境界・スレッド契約・プラグイン GUI 統合方針（ui ADR の確定範囲）
- 未決定事項の 3 分類と後続 ADR・実装 Issue への分解（A.5）
- Phase 1–3 の暫定実装ロードマップ（A.6）
- #2 基盤への反映変更の特定、README/AGENTS 差分の記録とタスク化（A.8）

### Out of Scope

- クレート・API の実装コード（設計の合意・文書化が Issue #1 の完了条件）
- ライセンスの最終決定（`LICENSE` 作成・`license = "MIT"` 仮置きの確定を含まない）
- 個別 codec／container 形式・外部バックエンドのサポート確定（抽象境界の設計のみ）
- UI 具体技術（WebView 実装・描画バックエンド・フロントエンドフレームワーク）の選定確定
- プラグイン基盤の構築方式（自前実装 vs 既存フレームワーク・binding クレート上への構築）の採否
- mobile／組込み／wasm の初期対応（設計で排除もしない）
- OFX 等の映像プラグイン規格の対応範囲確定

## Components and Boundaries

### レイヤー構造と依存方向ルール

Issue #1 のレイヤー案を具体化し、依存は**上位 → 下位のみ**とする。循環依存は禁止。同レイヤー内の依存は「レイヤー内順序が定義できる場合のみ」許容し、原則として下位層への集約で解決する。

```text
Applications / Hosts / Integrations        （利用側。本設計の境界外）
        │
        ▼
┌─ Engines ──────────────────────────────────────────────┐
│   ruviform-audio            ruviform-video             │
└──┬───────────────────────────────────────┬────────────┘
   ▼ Domain foundations                    │（opt-in 依存）
┌──────────────────────────┐               │
│ ruviform-dsp             │               │
│ ruviform-audio-analysis  │               │
│ ruviform-video-proc      │               │
│ ruviform-media-io        │◄── 抽象境界（demux/mux・decode/encode）
│ ruviform-audio-core      │               │
│ ruviform-video-core      │               │
└──┬───────────────────────┘               │
   ▼ Primitives                            │
┌──────────────────────────┐               │
│ ruviform-media-core      │               │
│ ruviform-rt              │               │
│ ruviform-exec            │               │
│ ruviform-math            │               │
└──────────────────────────┘               ▼
        ▲                    ┌─ Optional platform adapters（opt-in）─┐
        │ 依存               │ ruviform-audio-device（デバイス I/O） │
        └────────────────────│ ruviform-gpu（wgpu アダプタ）         │
                             │ ruviform-io-*（メディア I/O バックエンド群）│
   Plugin / UI（拡張層）      │ ruviform-ui-webview（WebView 実装）  │
   ruviform-plugin-core ──┐  │                                     │
   ruviform-plugin-host   │  └──────────────────────────────────────┘
   ruviform-plugin-export │
   ruviform-plugin-clap / -vst3（規格アダプタ）
   ruviform-ui（UI 基盤・WebView 非依存部分）
```

依存方向ルール:

- Primitives はいかなる上位層・プラットフォーム・外部バックエンドにも依存しない。GPU・OS 固有 API・デバイス依存を持たない（platform ADR 確定）。
- Domain foundations は Primitives と、後述の層内順序で下位に位置する同層クレートにのみ依存する。GUI・オーディオデバイス・GPU への強制依存を作らない（Issue #1 確定前提）。
- Engines は Domain foundations と Primitives に依存し、Optional platform adapters には optional（feature／分離クレート）経由でのみ依存する。
- Optional platform adapters は Domain foundations／Primitives、および「自身が実装するポート境界を定義するクレート」にのみ依存できる（adapters → foundations/Primitives + ポート定義元）。ポート定義元が上位層にある場合もその依存は合法とする（例: `ruviform-ui-webview` → `ruviform-ui` は WebView ホスト境界の定義元への依存として許容）。foundations → adapters の逆方向依存は禁止。adapter → adapter の相互依存は禁止とし、複数 adapter の組み合わせは利用側（engines／app）の依存解決で行う。利用側はアダプターを**依存解決の選択肢として**取り込み、抽象境界を通じて使用する。
- Plugin／UI 拡張層は Domain foundations・Primitives に依存し、engines と同じかそれより上位に位置する。engines → 拡張層の直接依存は禁止（エンジンは `Processor` 等の下位ポート経由でプラグインを駆動する）。

層内順序（同層依存はこの順序で下位 → 上位方向のみ許容）:

- Primitives: `ruviform-math`・`ruviform-rt`（最下位・相互依存なし）→ `ruviform-media-core`・`ruviform-exec`（`ruviform-exec` ↔ `ruviform-media-core` は相互依存禁止）。
- Domain foundations: `ruviform-audio-core`・`ruviform-video-core`（データ型）→ `ruviform-dsp`・`ruviform-video-proc`・`ruviform-media-io`（処理・境界。同順位同士の相互依存は禁止）→ `ruviform-audio-analysis`（`ruviform-audio-core`・`ruviform-dsp` に依存可）。
- Engines: `ruviform-audio` ↔ `ruviform-video` の相互依存は禁止。A/V 同期は `ruviform-rt`・`ruviform-media-core` の共有時刻型経由で行う。
- 拡張層: `ruviform-plugin-core` → `ruviform-plugin-host`・`ruviform-plugin-export` → `ruviform-plugin-clap`・`ruviform-plugin-vst3`。`ruviform-ui` → `ruviform-ui-webview`（ポート定義元への adapter 依存）。plugin 系と ui 系の相互依存は禁止（プラグイン GUI 統合は利用側で配線する）。

契約型・境界ポートの所有（一元定義。実装は adapter／利用側の責務であり、media-io パターンに統一する）:

- `Processor` trait（`prepare`/`process` 分離の処理契約）、`RtContext` capability、`MasterClock` 時刻共有プリミティブ → `ruviform-rt` が定義する（IO 型・共有値型はジェネリックであり、`ruviform-rt` 自身は他クレートに依存しない）。
- トランスポート状態の値型（再生状態・`MediaTime` 位置等のスナップショット）→ `ruviform-media-core` が定義する。
- オーディオデバイス境界 trait → `ruviform-audio-core` が定義する。`ruviform-audio`（engine）と `ruviform-audio-device`（adapter）はこの境界を介して接続され、engine ↔ adapter の直接依存・循環を構造的に排除する。
- メディア I/O 境界（demux/mux・decode/encode）→ `ruviform-media-io` が定義する（`ruviform-io-*` が実装）。
- GPU 境界 API → 利用側ポートとして `ruviform-video-proc`（GPU 計算カーネル境界）と `ruviform-ui`（描画境界）が定義する。`ruviform-gpu` は `wgpu` 実装を非公開で抱える adapter。
- WebView ホスト境界 → `ruviform-ui` が定義する（`ruviform-ui-webview` が実装）。
- 規格中立プラグイン API・ライフサイクル状態機械 → `ruviform-plugin-core` が定義する（規格 ABI への変換は `ruviform-plugin-clap`・`ruviform-plugin-vst3`）。

### クレート一覧（A.2）

設計対象クレートの責務・公開 API 境界・依存可能レイヤー・RT 制約を一覧化する。命名は本書時点の設計名であり、実装 Issue での最終確定を妨げない。「依存可能なレイヤー」列は依存先レイヤー名で記し、同レイヤー内の依存は `レイヤー名（クレート名）` の括弧書きで示す。

| クレート | 責務／非責務 | 公開 API の境界 | 依存可能なレイヤー | RT 制約 |
| --- | --- | --- | --- | --- |
| `ruviform-math` | 数値演算・ベクトル・SIMD カーネル、CPU feature detection、数値ユーティリティ。メディア固有概念・I/O・スレッドを持たない | プリミティブな数値型・カーネル関数のみ。SIMD バックエンドクレートの型は公開 API に出さない | なし（std のみ。安全 SIMD クレート等の外部依存は実装詳細） | 全公開関数を alloc なし・有界の RT 適合設計とする（RT 契約の適用対象） |
| `ruviform-rt` | RT 契約の定義（`Processor` trait・`RtContext` capability・契約文書化の型）、RT-safe プリミティブ（SPSC リング、ドロップ退避、不変スナップショット交換、`MasterClock` 時刻共有、事前確保型の受け渡し構造）。スケジューリング・I/O は持たない | RT 契約型・lock-free/待機なしプリミティブのみ。外部同期クレート（rtrb 等）の型は露出しない | なし | 契約定義元。公開 API はすべて RT 適合（no-alloc・ノンブロッキング）または「非 RT 専用」の明示ラベル |
| `ruviform-exec` | 非 RT 実行基盤：スレッドプール、ジョブ並列化、進捗・キャンセルトークン、バッチ駆動。RT プリミティブは持たない | スレッドプール・ジョブ・キャンセル API。OS スレッドの詳細は隠蔽 | Primitives（ruviform-math・ruviform-rt） | 非 RT 専用。RT パスからの呼び出しは契約違反 |
| `ruviform-media-core` | 有理数時刻・タイムベース・PTS、ストリーム/フォーマット記述、メディア共通エラー型、タイムコード等の表示用変換、トランスポート状態の値型（再生状態・位置のスナップショット）。処理・I/O は持たない | 値型中心（コピー可能・alloc なし）の時刻・記述・エラー型 | Primitives（ruviform-math・ruviform-rt） | 値型のみ・RT 適合（変換は純粋関数） |
| `ruviform-audio-core` | 音声バッファ/フレーム/サンプル形式、interleaved/planar、サンプルレート、チャンネルレイアウト、音声時間の基本契約、オーディオデバイス境界 trait（ポート）の定義。処理・I/O・デバイス実装は持たない | バッファ・レイアウト・型のみ。確保（確保用ビルダー）は非 RT 用途として明示 | Primitives | バッファ型・参照は RT 適合。確保・伸長は非 RT API |
| `ruviform-video-core` | 動画フレーム/plane/pixel 形式、stride、色空間・HDR メタデータ、フレーム時刻。変換・合成処理・I/O は持たない | フレーム・plane・ピクセル形式・色メタデータ型 | Primitives | フレーム参照は RT 適合。確保・pool は非 RT API |
| `ruviform-dsp` | フィルタ、リサンプリング、FFT・窓関数、畳込み等の計算カーネル。I/O・デバイス・グラフ実行は持たない | 初期化（確保あり）と処理（確保なし）を分離したカーネル API | Primitives + Domain foundations（ruviform-audio-core） | `process` 系は RT 適合（事前確保済み状態でのみ動作）。初期化・再構成は非 RT |
| `ruviform-audio-analysis` | レベル・ピーク・スペクトル解析、UI 描画データ（波形ピーク・FFT データ）の生成・キャッシュ。変換 DSP の重複実装は持たない | 解析器・描画データ生成 API | Primitives + Domain foundations（ruviform-audio-core・ruviform-dsp） | `process` 系は RT 適合。描画データは非 RT 側で消費 |
| `ruviform-video-proc` | 色変換・スケーリング・合成の基礎処理カーネル、GPU 計算カーネル境界（ポート）の定義。codec・I/O は持たない | 初期化/処理分離のカーネル API | Primitives + Domain foundations（ruviform-video-core） | `process` 系は RT 適合設計（alloc なし）。ただし RT 保証は API 単位で個別明示されるまで非保証。初期化は非 RT |
| `ruviform-media-io` | コンテナ demux/mux と codec decode/encode の抽象境界、パケット・ストリーム記述・能力交渉・エラー。バックエンド実装は持たない | Ruviform 固有型のみ。外部クレートの型・エラー・feature 名を公開 API に露出しない（media-io ADR） | Primitives + Domain foundations（ruviform-audio-core・ruviform-video-core） | 非 RT 専用（ファイル・ネットワーク I/O・codec 初期化）。RT パスからの呼び出しは契約違反 |
| `ruviform-audio` | 音声エンジン：処理グラフ実行、ミックス、デバイスセッション調停、プラグインチェイン接続、RT コールバック配線。codec 実装・UI は持たない | エンジン・グラフ・セッション API。RT パス用 API と制御 API を分離 | Domain foundations + Primitives + Optional platform adapters（opt-in） | RT パスを所有。`process` 系は RT 適合、制御・構成 API は非 RT |
| `ruviform-video` | 動画エンジン：フレームパイプライン、A/V 同期（`MasterClock` 追従）、シーク、オフラインレンダ調停。codec 実装・UI は持たない | エンジン・パイプライン API | Domain foundations + Primitives + Optional platform adapters（opt-in） | 内部処理は原則非 RT。A/V 同期面で RT パスと接続（`MasterClock` 参照は読み取り専用・ノンブロッキング） |
| `ruviform-audio-device` | オーディオデバイス I/O のアダプタ（cpal 有力）。抽象境界経由の差替可能実装詳細。codec は持たない | `ruviform-audio-core` が定義するデバイス境界 trait の実装 | Domain foundations + Primitives | デバイスコールバックが RT パスの起点。コールバック内は RT 契約適用 |
| `ruviform-gpu` | `wgpu` 等の GPU 描画・計算のアダプタ（バージョン差・フォールバック吸収）。エンジン層の opt-in | `ruviform-video-proc` が定義する GPU 計算境界・`ruviform-ui` が定義する描画境界の実装。`wgpu` 等のバックエンド型は非公開 | Domain foundations + Primitives + 拡張層（ruviform-ui） | 非 RT |
| `ruviform-io-symphonia` / `ruviform-io-ffmpeg` 等 | メディア I/O バックエンド（候補の例示であり採否決定ではない）。`ruviform-media-io` の抽象境界の実装詳細 | 個別バックエンドクレートの公開 API（登録・バックエンド固有 feature は当該クレート内に限定） | Domain foundations + Primitives | 非 RT |
| `ruviform-plugin-core` | プラグイン共通処理基盤：パラメータ・イベント（sample-accurate）・ライフサイクル・RT 契約・規格中立型。規格 ABI は持たない | 規格中立のプラグイン基盤 API | Domain foundations + Primitives | `process` 系は RT 適合。初期化・状態保存等は非 RT |
| `ruviform-plugin-host` | ホスト側：プラグインスキャン・ロード・インスタンス管理・分離（プロセス内デフォルト、分離は境界設計で阻害しない）。規格 ABI 詳細は adapter に委譲 | ホスト API・インスタンスハンドル・分離境界 | Domain foundations + Primitives + 拡張層（ruviform-plugin-core） | プラグイン `process` 呼び出しは RT パス。ロード・スキャン・再構成は非 RT |
| `ruviform-plugin-export` | 制作側：Ruviform 製プラグインのビルド・エクスポート基盤（ライフサイクル実装・規格準拠エントリ生成）。規格 ABI 詳細は adapter に委譲 | 制作基盤 API・プラグイン定義 | Domain foundations + Primitives + 拡張層（ruviform-plugin-core） | プラグイン `process` 実装は RT 契約の適用対象 |
| `ruviform-plugin-clap` / `ruviform-plugin-vst3` | CLAP／VST3 の規格固有アダプタ（エントリポイント・ABI 変換・ライフサイクルマッピング）。薄いアダプタとして分離。binding クレート利用は実装詳細 | 規格固有部の公開面は最小。共通基盤に規格固有型を持ち込まない | Domain foundations + Primitives + 拡張層（ruviform-plugin-core・ruviform-plugin-host・ruviform-plugin-export） | 規格コールバック（process 等）が RT 境界 |
| `ruviform-ui` | UI 基盤：コンポーネントモデル、Rust⇄フロントエンド メッセージ境界、状態同期、描画データ（ピーク・FFT キャッシュ）の生成 I/F、WebView ホスト境界・GPU 描画境界（ポート）の定義。WebView 実装詳細は持たない | UI コンポーネント・メッセージ・スレッド契約 API・ポート境界。描画バックエンド型は露出しない | Domain foundations + Primitives | 非 RT 専用（UI スレッド）。RT スレッドに UI 処理・ブロッキング同期を持ち込まない |
| `ruviform-ui-webview` | WebView 実装アダプタ（`wry` 有力、プラグイン用途は `wxp` 等のライフサイクル対策候補）。opt-in | `ruviform-ui` が定義するホスト境界の実装 | Domain foundations + Primitives + 拡張層（ruviform-ui） | 非 RT |

### feature flag・公開方針

- **デフォルト最小**: 各クレートの default feature は純 Rust コア機能のみとし、プラットフォーム・外部バックエンド依存を含めない。
- **外部バックエンドは feature でなく分離クレート**: メディア I/O・codec・GPU・WebView 等の外部依存は分離クレート + opt-in 依存として表現する（media-io ADR 確定）。コアクレート内でバックエンドを feature 切替しない。バックエンドクレート自身が持つ内部 feature（例: `ruviform-io-ffmpeg` の対応 codec サブセット）は、当該バックエンドクレートの API 面として許容する。この外部型・feature 名の非露出規則が適用される公開契約は、コア抽象境界クレート（`ruviform-media-io`・`ruviform-audio-core`・`ruviform-ui` 等のポート定義元）の API 面である。opt-in でのみ取り込まれるバックエンドクレートの内部 API は安定性契約の対象外とし、公開 API としての互換維持を要求しない。
- **feature 命名**: feature 名は「バックエンド有効化の意図」を表す命名に限定し、外部ライブラリ固有名詞をコア API の feature 名に露出させない（media-io ADR）。エンジン層でアダプターを束ねる便宜 feature（例: `ruviform-audio` の `device` → `ruviform-audio-device` optional 依存）は意図命名として許容する。
- **publish 制御**: 未決定クレートは `publish = false` を原則とする（licensing ADR）。公開可否はクレート単位で選択し、外部依存を含むクレートは公開前ライセンスレビュー必須。
- **0.x 継続**: 1.0 は Phase 番号ではなく API 安定性・実装成熟度・互換維持準備で判断する。プラグイン ABI 適合性と内部 Rust API 安定性は分離管理。

### 採否

- **決定**: レイヤー構造・依存方向ルール・上記クレート分割案・feature/公開方針。
- **要PoC**: 個別アダプターのバックエンド選定（`cpal` / `wry` / `wgpu` / FFmpeg 等の採否・バージョン）、メディア I/O 個別 codec 対応範囲。
- **保留**: モバイル/組込/wasm（初期ターゲット外・排除しない）、OFX 等の映像プラグイン範囲、個別機能ライブラリ（`rubato`・`vtc`・`ringbuf` 等）の採否 — 包括ルールとして、個別ライブラリの採否は実装 Issue の依存追加規約（AGENTS.md）に委ねる。

## データモデル

### 時間モデル

- **中核型は有理数／整数カウント**とする（discovery: `vtc`・FilmCraft・Concat・kael_media_engines の 4 系統が同結論）。f64 秒を中核型にしない（長時間ドリフトで編集・同期用途に不適）。f64 は UI 表示等の限定的用途に留める。
- `ruviform-media-core` が定義する概念（擬似構造）:
  - `TimeBase` = 1 秒の有理数分割（例: `1/48000`, `1/90000`, `1001/30000`）。
  - `MediaTime` = `TimeBase` に対する整数カウント（符号あり。負値は pre-roll/オフセット表現用）。
  - `Duration` = 同様の有理数長さ。
  - `MediaRange` = `MediaTime` + `Duration`。
- 変換規則: 異なる timebase 間の変換は正確な有理数演算で行い、表現可能な場合は厳密値を返す。表現不可能な場合の丸めは呼び出し側が指定（既定: round-half-to-even）。音声 ↔ 動画の変換は**音声サンプル時刻を優先**（A/V 同期の master が音声のため）。
- 音声時間: サンプルインデックス（整数）+ サンプルレート（= timebase `1/rate`）。sample-accurate イベントはサンプルオフセットを保持する。
- 動画時間: コンテナ timebase の PTS + フレームインデックス。VFR は per-frame PTS を正とし、CFR を仮定した処理を設計しない。
- ドロップフレーム: タイムコードの**表示・ラベリング規約**であり、単調な媒体時間とは分離する。同期・編集の基本通貨は有理数 `MediaTime` であり、ドロップフレームはタイムコード変換層（media-core の表示用変換）の責務。
- `FrameRate`/`TimeBase` の意味分離と `Timecode` API 拡張・丸め誤差累積ポリシーは Issue #42 文書群（[time-model-extension.md](time-model-extension.md)・[対応 ADR](../adr/framerate-timebase-separation-and-timecode-extension.md)）で確定。タイムコード変換は fps 向き有理数を表す `FrameRate` 型を受け取る。
- シーク: エンジンの責務。demuxer に time 指定シークを要求し、decoder フラッシュと A/V 再同期を行う。
- 責務所在のまとめ: 有理数時刻・変換・タイムコード表示・トランスポート状態の値型 = `ruviform-media-core`。時刻共有プリミティブ（`MasterClock`）= `ruviform-rt`。A/V 同期方針（`MasterClock` + PTS 追従）・シーク・VFR の実行時振る舞い = engines。codec 側の PTS/DTS 取出し = `ruviform-media-io` 抽象境界経由。
- 第 1 期のトランスポート/イベント型の拡張余地: トランスポート状態型（再生状態・位置 `MediaTime` 等）とイベント型は拍位置（小節/拍/PPQN・テンポ）を持たず、サンプル・媒体時刻のみを表現する。将来の音楽時間導入に備え、opaque な拡張予約フィールドを残す。音楽時間の設計自体は保留（再着手トリガ: 第 2 期/テンポマップ設計 — 音楽時間設計 Issue）。

### 音声データモデル

- 単位: `Sample`（1 標本）→ `Frame`（全チャンネルの同一時刻標本）→ `Buffer`（frames × channels）。
- **処理用バッファは planar f32 を正準**とする。interleaved はデバイス・ファイル境界の入出力形式として扱い、境界で `planar ↔ interleaved` を変換する。内部処理を interleaved 前提にしない。
- `AudioBuffer` = planar チャンネル列 + フレーム数 + サンプルレート + `ChannelLayout`（mono/stereo/5.1 等の既知レイアウト + 明示スピーカーマスク）。
- サンプル型は `f32` を first-class とする。オフライン精度用途の `f64` は型パラメータで許容するが、RT 経路の正準は `f32`（`f64` の本格対応は保留 — 後続評価）。
- バッファ所有: RT パスでは事前確保済みバッファへの**借用・書き込み**のみ行い、RT パス内で確保・伸長しない。確保用のビルダー/プールは非 RT API として提供する。
- zero-copy: デコーダやデバイス境界では参照カウント済みの共有バッキングを許容する設計とし、RT パス内でコピーを強制しない。共有バッキングのライフタイム管理は非 RT 側の責務。

### 動画データモデル

- 単位: `Plane`（データ + stride + 幅高）→ `VideoFrame`（planes + `PixelFormat` + 解像度 + 色メタデータ + PTS）。
- `stride` は `width * bytes_per_pixel` と独立（パディングを明示）。plane 単位の stride を保持する。
- `PixelFormat` は yuv420p/yuv422p/yuv444p、rgba8/rgbaf16 等の列挙（初期対応範囲は後続 Issue で精査。拡張可能な設計）。
- 色空間・HDR メタデータ: primaries / transfer / matrix / 範囲（full/limited）+ HDR メタデータ（例: mastering display、content light level）をフレームに付随。変換処理自体は `ruviform-video-proc` の責務。
- フレーム時刻: `MediaTime`（timebase + PTS）を保持。duration は明示または次フレーム PTS から導出。
- 所有: デコーダからの zero-copy 供給を許容するため、参照カウント型のフレームバッキングを想定。フレームプールの所有は非 RT 層。

### メディア I/O 抽象（demux/mux と decode/encode の分離）

media-io ADR の確定どおり、コンテナの demux/mux と codec の decode/encode は**別の抽象**として分離する。概念を単一 crate `ruviform-media-io` に集約する案を採用する。

- 抽象的な流れ: `Demuxer` → `Packet`（ストリーム参照 + PTS/DTS + フラグを持つ符号化単位）→ `Decoder` → `AudioFrame`（`AudioBuffer` + PTS/duration を束ねる media-io 側の出力型）/ `VideoFrame`。mux/encode は逆方向。
- 分離する 4 系統: `Demuxer`（コンテナ→packet）、`Muxer`（packet→コンテナ）、`Decoder`（packet→frame）、`Encoder`（frame→packet）。デマルチプレックスとデコードを一つの「読み出し」抽象に併合しない。
- **音声/動画共通仕組みの採否**: 採用（決定）。`Packet`・`StreamDescriptor`・`StreamKind::{Audio,Video,Data}`・能力交渉・エラーモデルはメディア領域で共通であり、単一コンテナが音声と動画を混在させる現実にも整合する。代替案（`ruviform-audio-io` / `ruviform-video-io` の分離）は、同一ドメインルール（ストリーム記述・時刻・能力交渉）を 2 クレートへ複製し、抽象の二重管理を生むため不採用。`ruviform-media-io` が audio-core/video-core の両方に依存する点は、両 core が純粋データ型のみで依存コストが小さいため許容する。
- バックエンド: `ruviform-io-symphonia`・`ruviform-io-ffmpeg` 等の分離クレート（採否された場合の候補例）が trait を実装する差替可能な実装詳細。バックエンド選択はクレート/feature 単位の依存として表現し、実行時文字列指定等にはしない（media-io ADR）。**個別 codec/container の採否は後続 Issue**（spec Out of Scope と整合）。
- 能力交渉: バックエンドが非対応の機能（codec 個別パラメータ等）は、抽象 API で「非対応」を型/エラーとして表現し、全機能を一律汎用化しない（抽象肥大化の回避）。
- エラー: バックエンド由来の詳細は抽象エラー型に正規化し、外部エラー型を公開 API に露出しない。

### 採否

- **決定**: 有理数時刻を中核。音声正準 = planar f32（境界で interleaved 変換）。動画フレーム + 色/HDR メタデータ。demux/mux・decode/encode の 4 系統分離 + 単一 `ruviform-media-io` への集約。色変換・HDR の対応範囲（Issue #26・video-proc ADR で確定: `Yuv*p ↔ Rgba8`・`Rgba8 ↔ RgbaF16` のスカラーカーネル — matrix Bt601/Bt709/Bt2020 × range Limited/Full・chroma resampling 込み、HDR = メタデータ搬送 + 非破壊で transfer 適用・トーンマップなし）。`FrameRate`/`TimeBase` 意味分離（fps 向き newtype・`frame_duration()` のみ `TimeBase` 露出）、`Timecode` 引数の `FrameRate` 置換（破壊的変更）、ドロップフレーム対象レート集合の現行維持（`30000/1001`・`60000/1001` のみ）、丸め誤差累積ポリシーの文書契約化（Issue #42 文書群）。
- **要PoC**: 個別 codec/container 採否、デコーダ zero-copy バッキングの具体方式。連鎖変換プロパティテストの harness 選定（proptest dev-dep vs deterministic 列挙、Issue #42）。
- **保留**: 音楽時間（小節/拍/PPQN）の自前設計、`f64` サンプル対応の深さ（f32 正準の決定に伴い需要ベースの後続評価）、色管理の広範統合（`moxcms` / OpenColorIO 級 — 第 2 ターゲット設計時）。ドロップフレーム非 NTSC nominal 一般化・フィールドレート対応、`Timecode` → `MediaTime` 逆変換 API、frame index 型の所有（media-core vs video-core）（Issue #42）。

## Data and Control Flow

### RT／非 RT の分離方式（realtime ADR の確定に従う）

- **API・ライフサイクルで分離**する。同一 API の内部で `if rt { ... } else { ... }` 分岐しない。
- 型レベル分離: 処理対象は `prepare(setup)`（非 RT・確保可）で初期化・構成を確定し、RT 用の処理 API（`process(&mut self, io, ctx)`）を持つ**別の処理オブジェクト/状態**を生成する。`Processor` trait と `RtContext` は `ruviform-rt` が所有する（「契約型・境界ポートの所有」参照）。
- `RtContext` の発行規則: `RtContext` は **RT 契約区間を開始した駆動者のみが発行できる capability** とする。発行主体は RT 駆動者（`ruviform-audio` が `ruviform-audio-device` のデバイスコールバック内で発行）と、オフライン駆動者（`ruviform-exec` の非 RT ドライバが「有界実行を保証するコンテキスト」として発行）に限定し、mint API はこれら駆動者の入口点にのみ公開する。アプリケーション等の任意コードは `RtContext` を構築できないため、`prepare` を経ない `process` 呼び出しや契約区間外の実行を型で抑止できる。オフライン発行の `RtContext` 上では no-alloc 等の RT 制約は適用されないが、同一 `Processor` 契約で駆動される。
- ライフサイクル: 初期化・グラフ再構成・メモリ再確保・codec 初期化は**非 RT 区間（制御スレッド）**で実施。構成差し替えは、非 RT 側で新構成を構築 → atomic なスナップショット交換（RCU 相当）またはコマンド経由で RT 側がブロック境界で切替。
- RT ↔ 非 RT 通信: SPSC リング（`rtrb` 型）によるノンブロッキング通知。RT → 非 RT はメータ・イベント・退避オブジェクトの送出。非 RT → RT はパラメータ・制御コマンド。いずれも有界・ノンブロッキング（満杯時はドロップ/統合方針を API で定義）。
- Drop 退避: RT パスで退役した重いオブジェクトは非 RT 側へ送出して drop する（`basedrop` 型パターン）。
- 「RT-safe」等の保証範囲は **API 単位で定義・明示**する（realtime ADR）。未確定の API を RT-safe と断定しない。

### 処理グラフモデルとスケジューリング採否

- **グラフ構造**: ノード = 処理器（`Processor`: `prepare`/`process` 分離）。ポート = 音声バッファ・動画フレーム・イベント/パラメータ。トポロジは「コンパイル済みプラン」として確定し、変更は非 RT スレッドでの再コンパイル + スナップショット交換。
- **スケジューリング採否**: 処理グラフ内部は **pull ベース**を採用する（destination が必要分を上流へ要求）。理由: オーディオコールバック駆動の RT パスは必要ブロック数が出力側で確定しており、pull が遅延・キュー肥大の制御に適合する。push はシステム端（デバイス入力、decode→process の跨スレッド受け渡し）で**有界キュー**としてのみ用い、処理グラフ内部の push を基本にしない。
- 共通時間軸: `ruviform-media-core` の有理数時刻。オーディオデバイス時刻を保持する **`MasterClock`** を A/V 同期の基準時計（audio master）とし、動画は PTS 追従（ffplay 型・discovery の定石）。
- オフライン実行: 同一計算カーネル（`Processor`）を `ruviform-exec` の非 RT ドライバが `RtContext` を発行してバッチ・並列駆動する。チャンク長・並列度・精度/再現性（決定論モード）と低遅延のトレードオフを設定として明示する。

### スレッド構成（設計案）

- **RT スレッド**: デバイスコールバック（`ruviform-audio-device` 経由）が起点。alloc/dealloc 禁止、ブロッキング・I/O・無制限待機・非有界処理を制限。
- **非 RT 制御スレッド**: 初期化・グラフ再構成・確保・codec 初期化・状態保存。
- **I/O/デコードスレッド**: `ruviform-media-io` 経由の demux/decode。ブロッキング可・有界キューで処理系へ供給。
- **ワーカースレッドプール**（`ruviform-exec`）: オフライン処理の並列化・プラグインの補助スレッド。
- **UI スレッド**: `ruviform-ui`／WebView。RT スレッドへブロッキング同期を持ち込まない。
- スレッド優先度: RT スレッドは OS 提供の優先度付与（MMCSS/time-constraint/SCHED_FIFO 系）を活用する設計とし、抽象は `audio_thread_priority` 型を参考にする（要 PoC）。

### RT データフロー例：音声出力パス（A.3）

例: 音声出力コールバック → プラグインチェイン → ミックス → デバイス。バッファ 480 frames @48 kHz（10 ms 周期）。

```text
[非RT: 制御スレッド]                             [RT スレッド: デバイスコールバック]
  初期化・グラフ構築・確保 ──swap──► 実行プラン(不変スナップショット)
                                          │
                              デバイス出力コールバック(out: &mut AudioBuffer, ctx: &RtContext)
                                          │
                          ┌───────────────┴───────────────┐
                          ▼                               ▼
                   トランスポート時刻取得            イベント/パラメータ列(事前確保)
                          │                               │
                          └───────────────┬───────────────┘
                                          ▼
                          for node in graph.nodes (pull):
                              node.process(&mut node_buf, ctx)   // RT 契約: no alloc
                                          │
                                          ▼
                              ミキサー: 全入力を out に和算 (planar f32)
                                          │
                                          ▼
                      メータ/ピークを SPSC ring へ try_push（満杯時はドロップ）
                                          │
                                          ▼
                                   コールバック復帰
```

- **alloc/dealloc 禁止**: バッファは `prepare` で最大ブロック長に確保済み。`process` 内で `Vec` 伸長・`Box`/`String` 確保・`Mutex::lock` のブロッキング取得・`std::fs`/`std::net` を行わない。
- **バッファ受け渡し**: ノード間は事前確保済みバッファへの `&mut` 借用を引き回す。コピーが必要な境界（デバイス out）は事前確保領域への和算/書き込みで処理。
- **RT/非 RT 境界**: グラフ差替は非 RT 側で新プラン構築 → 不変スナップショットを atomic 交換（RT 側は次ブロックから新プランを参照）。パラメータ変更は非 RT → RT の SPSC コマンドでブロック境界に適用。退役リソースは RT → 非 RT へ送出して drop。

### オフラインデータフロー例：ファイル decode → 処理グラフ → encode（A.3）

```text
[制御スレッド]                    [I/O スレッド]          [ワーカープール]        [I/O スレッド]
  計画立案・確保 ──► demux 開始    demux → Packet ──►  decode ──► Frame ──┐
                                                                          │ 有界キュー(backpressure)
                              ┌─────────────────────────────────────────┘
                              ▼
                    ワーカー N 本: chunk を pull → 処理グラフ(同一カーネル) → 出力 frame
                              │
                              ▼
                    encode 側キュー ──► Encoder → Muxer → ファイル
  進捗: 原子カウンタ / キャンセル: CancellationToken をチャンク単位で確認 / エラー集約
```

- 初期化（demux/decoder/encoder オープン・確保）は制御スレッド（非 RT）。処理カーネルは RT 経路と同一の `Processor` を `prepare` → `process` で駆動。
- チャンク単位で進捗・キャンセル・エラー集約。ブロッキングは許容（非 RT）だが、各段は有界キューで接続しメモリを抑制。

### 採否

- **決定**: API・ライフサイクルによる RT/非 RT 分離（`RtContext` capability — 発行は RT 契約区間の駆動者に限定）、pull ベースのグラフ + エッジの有界キュー、`MasterClock` + PTS 追従、スナップショット交換による再構成、SPSC 通知・Drop 退避。
- **要PoC**: `rtrb` / `basedrop` / `audio_thread_priority` / `clack-extensions` thread-pool の採否、スナップショット交換の具体実装（arc-swap 型/RCU 型）。
- **保留**: オフラインの決定論的実行（seed/固定順序）の厳密保証範囲。

## DSP／SIMD 設計（A.4）

- **二層構造**: スカラー参照実装を正とし、対応ターゲット向けの SIMD 実装を**ランタイム分岐 + フォールバック**で提供する有力仮説とする（platform ADR と整合。採否・方式はベンチマーク比較評価で確定＝要 PoC）。
- **ターゲット差のカプセル化**: aarch64 NEON は baseline のためランタイム検出不要。x86_64 は SSE2 baseline → AVX2/FMA（将来 AVX-512）を起動時検出・ディスパッチトークンにキャッシュする設計。検出・分岐・フォールバックの機構をカーネルごとに重複実装せず、`ruviform-math` の dispatch 層に集約する。
- **候補（stable-only 前提、`std::simd` 不可）**: `fearless_simd`（第一候補）、`pulp`（対抗馬）、`multiversion` + `wide`/`std::arch`（定石、`std::arch` は unsafe カプセル化必要）。SIMD バックエンドクレートの型・API を Ruviform 公開 API に露出しない。
- **unsafe**: 原則禁止。`std::arch` 直接利用等で必要な場合のみ、SAFETY コメント付きの局所許可（`#[expect(unsafe_code)]`）で例外的に認める（AGENTS 暫定規約と整合）。
- **境界条件**: アラインメント・端数フレーム・NaN/Inf・denormal・オーバーフロー・丸め差異をテスト対象に明記。denormal はエンジン側で FTZ/DAZ を設定する方針を基本案とし、カーネル内での flush は各 API の契約で定義する。
- **等価性基準**: カーネルごとに「ビット完全一致」または「ulp 上限内」の契約を定義し、スカラーを参照として検証する。
- **採否基準（要 PoC の評価条件）**: 対象カーネルでのスカラー比の性能向上（目安を後続 PoC 計画で数値化）、コンパイルコスト、unsafe カプセル化の範囲、MSRV 適合、保守状況。

### 採否

- **決定**: スカラー参照実装を必須とする（正・等価性検証の参照）、stable-only・unsafe 局所許可、等価性契約の原則。
- **要PoC**: SIMD 二層構造の採否（ランタイム分岐 + フォールバック・dispatch 層の `ruviform-math` 集約を含む構造仮説）、`fearless_simd` / `pulp` / `multiversion` 系の採否・AVX-512 扱い・denormal 具体方針・数値等価基準の具体値。
- **保留**: `std::simd` 安定化後の移行判断。

## プラグインアーキテクチャ（plugin ADR の確定範囲）

- **構造**: 共通処理基盤（`ruviform-plugin-core`）と規格固有アダプタ（`ruviform-plugin-clap` / `ruviform-plugin-vst3`）を分離。ホスト側（`-host`）と制作側（`-export`）は共通基盤の上に対称に構築する。
- **ABI 境界**: 規格端は純 C ABI。アダプタが ABI ↔ 共通基盤型（バッファ・時刻・パラメータ・イベント）を一方向へ翻訳し、共通基盤は規格固有型・概念を持たない。Ruviform 製プラグインも同一アダプタ経由で規格 ABI に変換する（ホスト/制作の対称性）。
- **ライフサイクル**: `create → init → activate（確保・prepare）→ processing（RT）→ deactivate → destroy` の状態機械を共通基盤で定義し、CLAP/VST3 各規格の状態遷移へアダプタが写像する。パラメータ変更・状態保存は非 RT 区間。
- **規格中立トランスポート/イベント型の拡張余地**: 規格中立のトランスポート状態型・イベント型は拍位置（小節/拍/PPQN・テンポ）を持たず、サンプル・媒体時刻のみを扱う（時間モデル節と同一方針）。CLAP/VST3 が持つ音楽時間情報の将来マッピングを ABI 変更なく吸収できるよう、opaque な拡張予約フィールドを設ける。音楽時間の設計自体は保留（再着手: 第 2 期/テンポマップ設計 — 音楽時間設計 Issue）。
- **分離方針（サンドボックス/クラッシュ分離）**: デフォルトは**インプロセス**（性能）。アウトプロセス proxy 分離は、インスタンスを「ハンドル + 直列化可能なコマンド境界」として設計することで将来追加を阻害しない構造とする。分離方式の実装は保留（後続 Issue）。
- **RT 契約の適用**: `process` 系コールバックは RT パスの一部としてホスト・プラグイン双方に RT 契約を適用（plugin ADR + realtime ADR）。CLAP は `clack-extensions` の thread-check 型機構を活用する設計候補。VST3 も同様のスレッド契約をドキュメント + 検査で管理。
- **プラグイン GUI**: 外側は規格準拠のネイティブウィンドウ/ビュー（CLAP gui / VST3 IPlugView、`baseview` 有力候補）。内側は WebView/GPU/ネイティブ可（ui ADR と同一方針）。**独自 GUI は全プラグイン必須でない**（パラメータ記述子からの汎用 UI 生成で代替）。GUI 統合はホスト・制作の双方向で必須。
- **binding クレート**: `clack`（CLAP 有力）・`vst3`（coupler-rs、VST3 有力）の利用を妨げない。構築方式（自前実装 vs binding クレート上への構築）は**要 PoC/後続**（plugin ADR の決定範囲外）。
- **VST3 SDK**: 3.8 以降 MIT（確認済み事実）。利用版条件は licensing ADR の個別調査手順に従う。
- **映像プラグイン**: OFX が有力候補。範囲・採否は別途設計（保留）。

### 採否

- **決定**: 共通基盤 + 規格アダプタ分離、ホスト/制作対称、ABI 境界・ライフサイクル状態機械、インプロセス既定 + proxy 阻害しない境界、RT 契約の双方適用、GUI 双方向必須・独自 GUI 非強制。
- **要PoC**: `clack`/`vst3` binding 採否、`baseview`/`wxp`/`nih-plug-webview` 等の GUI 基盤、thread-check 機構の具体方式。
- **保留**: サンドボックス/クラッシュ分離の実装方式、OFX 範囲、AU/AAX/LV2 等の追加規格。

## UI アーキテクチャ（ui ADR の確定範囲）

- **構造**: WebView ファースト・ハイブリッドを第一候補（HTML/CSS/JS・TS、フロントエンドフレームワーク不固定）。`ruviform-ui` は描画バックエンド非依存の基盤を定義し、`ruviform-ui-webview` が実装アダプタ（`wry` 有力）。
- **エンジン↔UI のメッセージ境界**: Rust 側を状態の正本とし、フロントエンドは intent を送信・状態差分と高頻度データを受信する。メッセージは型付きで直列化（初期は JSON 系、高頻度データは必要に応じてバイナリ検討 — 要 PoC）。
- **スレッド契約**: UI スレッドが WebView/DOM を所有。エンジン状態は UI スレッドで統合・消費。RT スレッドから UI への直接・同期呼び出しは禁止（RT → UI は SPSC 等のノンブロッキング経由で中継）。
- **状態同期**: パラメータ・設定は Rust 側で一元管理し差分を UI へ push。メータ等の高レートデータは UI スレッドで coalesce して描画に供給。
- **高頻度描画（特性別評価・固定順序/単一フォールバック階層にしない）**: 評価軸 = 更新頻度・データ量・遅延・DOM 合成可否・配布コスト・OS 対応。候補は特性別に評価する（順位付けではない）:
  - WebView 内 Canvas2D — 互換性下限・単純だが高頻度は限界あり。
  - WebGL2 — 3 OS 共通の実用下限（discovery）。
  - WebGPU — 現状 Windows（WebView2）のみ信頼可、WebKitGTK は未整備。
  - `wgpu` 等ネイティブ GPU 描画 — 上限は高いが WebView 内統合と分離が課題。
  - ネイティブサーフェス + WebView ハイブリッド — macOS は layer-backed WKWebView で合成しやすい、Windows は CompositionController、Linux（WebKitGTK）は配布リスクが最大。
- **描画データと描画方式の分離**: コンポーネントの公開 API・状態管理・描画データ生成（波形ピーク・FFT 等は Rust 側で生成・キャッシュ）を描画バックエンドから分離する。全コンポーネントに共通抽象を強制しない（コンポーネントごとに適した方式を選択）。
- **プラグイン GUI**: 同一方針（外側 = 規格準拠ネイティブウィンドウ、内側 = WebView 可）。
- **デスクトップ配布の課題**: Linux の WebKitGTK 実行時依存が最大リスク、Windows の WebView2（エバーグリーン）、macOS の WKWebView（システム同梱）。配布形態は PoC で検証。

### 採否

- **決定**: WebView ファースト方針の構造・責務・境界・スレッド契約・描画/コンポーネント分離・Rust 側描画データ生成・プラグイン GUI 方針・共通抽象非強制。
- **要PoC**: WebView 実装（`wry`/`wxp`/`nih-plug-webview`）、描画バックエンド採否（Canvas2D/WebGL2/WebGPU/wgpu/ネイティブ合成）、フロントエンドフレームワーク、配布形態。
- **保留**: 全コンポーネント共通の描画抽象の導入（現状は不採用方針、再検討は要件変更時）。

## Non-Functional Constraints

### 対象プラットフォーム・ツールチェーン（platform ADR）

- 対象 OS／CPU: Windows・macOS・Linux、x86_64＋aarch64。
- stable Rust のみ（nightly 必須機能・`#![feature(...)]` を必須としない）。MSRV・pin 版は [dev-foundation.md](dev-foundation.md) / `mise.toml` の仮置きを継続し、再検討は後続 Issue。
- 基礎クレートは GPU 非依存。`wgpu` 等はエンジン層の opt-in。モバイル／組込／wasm は設計で排除しないが初期ターゲット外。

### RT 契約（API 単位の保証定義）

- RT クリティカルパス: alloc/dealloc 禁止。ブロッキング操作・ファイル/ネットワーク I/O・無制限待機/リトライ・非有界処理を制限。
- 保証範囲は対象 API 単位で定義・明示。Ruviform の保証は自身の RT 対応実装に限定し、利用側コードの RT 安全性を保証しない。OS レベルのハードリアルタイム保証は提供しない。
- 検証可能な違反は CI で検出し、静的判定不可能なものはドキュメント契約 + コードレビューで管理。**CI 成功は完全証明ではない**旨を設計文書・README に明記する。

### 性能検証計画（A.4）

- **音声ドロップアウト/レイテンシ**: ループバック計測（実機は手動検証として計画。CI はデバイス非依存で、オフラインモードでのアンダーランシミュレーション + alloc フック検査で代替）、RTT レイテンシ計測、小バッファでの負荷試験、連続動作（soak）試験。
- **動画スループット/性能**: オフラインレンダの real-time factor、decode スループット、フレーム正確性のゴールデン比較（決定論検証）。
- **SIMD/スカラー比較**: criterion 等でカーネル別ベンチマーク（複数サイズ・端数長を含む）、等価性テスト（ビット一致 or ulp 上限）、採否基準との対照。

### RT 違反検出の分担

- **機械検証（CI）**: allocation フック検査（`assert_no_alloc` 型）を RT パスのテストに適用。プラグイン境界は thread-check 型機構でスレッド契約違反を検出。これらを `mise.toml` の検証タスクとして追加する後続 Issue を設ける。
- **文書+レビュー管理**: ロック保持時間・優先度逆転・syscall レイテンシ・メモリ順序・OS スケジューリング依存の挙動は静的に判定困難であり、ドキュメント契約として列挙しコードレビュー観点へ組み込む。
- **免責の明記**: CI 緑を「RT 安全の証明」と誤認させない。OS 全体のハード RT 保証とは混同しない。

## Risks and Trade-offs

- **抽象境界の肥大化**: メディア I/O・プラグインの抽象がバックエンド固有機能を一律汎用化して肥大するリスク → 非対応機能を型/エラーで表現し、全機能汎用化を避ける（media-io ADR 準拠）。
- **外部依存の保守リスク**: `cpal` メンテ少数・`wgpu` semver churn・FFmpeg ライセンス → すべて抽象境界の背後＋分離クレート + opt-in で隔離（design の核心構造）。
- **検証コスト**: 3 OS × 2 CPU の検証対象が広い → CI matrix は後続 Issue で段階拡大、初期は Linux 必須検証を維持。
- **pull グラフの限界**: 複雑な分岐/フィードバックや非同期ソース混在で pull のみが不自然になる場合 → エッジの有界キュー・イベント機構で補完。本格対応は実装経験で再評価。
- **WebView 統合の実現性**: 理想構造（バックエンド非依存のコンポーネント利用）が Linux 配布・プラグイン統合で未検証 → 要 PoC として明示管理。
- **RT 契約の形骸化**: 検証手段なしの契約は形骸化 → `assert_no_alloc` 型 CI 検査の導入 Issue を分解・必須化。

## 未決定事項の分類（決定／保留／要PoC）

各項目に根拠と次のフォーラムを付記する。「決定」とは本設計で確定した内容、または上流 spec/ADR でユーザー承認済みの内容を指す。

| 項目 | 分類 | 根拠 | 次のフォーラム |
| --- | --- | --- | --- |
| レイヤー構造・依存方向・循環依存禁止 | 決定 | Issue #1 レイヤー案・ADR 群の整合から具体化 | 実装 Issue（各クレート） |
| クレート分割・命名（上記一覧） | 決定 | 公開 API・独立再利用・依存コスト・ライフサイクルで評価 | 実装 Issue（各クレート）。実装経験で微調整可 |
| feature flag 方針（最小 default・opt-in バックエンド分離） | 決定 | media-io ADR・licensing ADR | 実装 Issue |
| 時間モデル（有理数中核・f64 非中核） | 決定 | 4 系統の先行事例で同結論（discovery） | 実装 Issue（media-core） |
| 音声データモデル（planar f32 正準・境界 interleaved） | 決定 | DSP 標準・RT 受け渡し適合 | 実装 Issue（audio-core） |
| 動画データモデル（plane/stride/色メタ/PTS） | 決定 | 業界標準フレームモデル | 実装 Issue（video-core） |
| demux/mux・decode/encode 分離 + 単一 media-io | 決定 | media-io ADR・共通ドメインルールの DRY | 実装 Issue（media-io） |
| RT/非 RT の API・ライフサイクル分離（`RtContext` capability） | 決定 | realtime ADR。発行主体を駆動者に限定する規則を本文で定義 | 実装 Issue（rt・各 processor） |
| 処理グラフ pull ベース + エッジ有界キュー | 決定 | コールバック駆動の遅延・キュー制御適合 | 実装 Issue（engines） |
| A/V 同期 = `MasterClock` + PTS 追従 | 決定 | discovery の定石（ffplay 型） | 実装 Issue（engines） |
| RT→非RT = SPSC・Drop 退避・スナップショット交換 | 決定 | RT 実践パターンの標準（discovery） | 実装 Issue（rt） |
| スカラー参照実装必須・stable-only・等価性契約原則 | 決定 | 検証参照として必須（platform ADR 前提） | 実装 Issue（math） |
| `FrameRate`/`TimeBase` 意味分離（`FrameRate` = `TimeBase` newtype） | 決定 | Issue #42 ADR（`SampleRate` 先例整合・意味論的分離・依存ゼロ維持） | 実装 Issue（media-core） |
| `Timecode` の `frame_rate` 引数を `FrameRate` へ破壊的置換 | 決定 | Issue #42 ADR（`publish = false`・外部利用者不在のうちの移行） | 実装 Issue（media-core） |
| ドロップフレーム対象レート集合（`30000/1001`・`60000/1001` のみ） | 決定（現行維持） | SMPTE 規格の定義域に一致（Issue #42 ADR） | 変更が必要なら新規 ADR/実装 Issue |
| 丸め誤差累積ポリシー（呼び出しごと最大 1 回・安定アンカー・base-hopping 禁止の文書契約） | 決定 | Issue #42 設計文書（[time-model-extension.md](time-model-extension.md)）のポリシー節 | 実装 Issue（media-core・doc/プロパティテスト） |
| `FrameRate` API 形状（fps 向き newtype・`frame_duration()` のみ TimeBase 露出） | 決定 | Issue #42 ADR/設計文書 | 実装 Issue（media-core） |
| 色変換・HDR の対応範囲 | 決定 | Issue #26・video-proc ADR（D2-a 縮小版・D3-a）で確定: `Yuv*p ↔ Rgba8`・`Rgba8 ↔ RgbaF16` のスカラーカーネル（matrix Bt601/Bt709/Bt2020 × range Limited/Full・chroma resampling 込み）、HDR = メタデータ搬送 + 非破壊（transfer 適用・トーンマップなし） | 動画基盤 Issue（実装済み） |
| SIMD 二層構造（分岐/フォールバック・dispatch 層集約の構造仮説） | 要PoC | platform ADR の有力仮説。採否・方式はベンチマーク比較評価で確定 | DSP/SIMD PoC Issue → 採否 ADR |
| SIMD バックエンド選定（fearless_simd/pulp/multiversion）・採否基準 | 要PoC | ベンチマーク比較評価で確定（platform ADR） | DSP/SIMD PoC Issue → 採否 ADR |
| 音声デバイス I/O バックエンド（cpal 等） | 要PoC | 抽象境界必須・メンテリスク | デバイス PoC Issue |
| メディア I/O バックエンド（symphonia/ffmpeg 等・個別 codec） | 要PoC | 抽象境界は決定、個別対応は Out of Scope | メディア I/O 実装 Issue |
| オーディオ codec 純 Rust 第一候補（symphonia） | 要PoC | 有力候補だが採否は後続 | メディア I/O 実装 Issue |
| デコーダ zero-copy バッキングの具体方式 | 要PoC | 所有・境界設計は決定、方式は実装評価 | メディア I/O 実装 Issue |
| プラグイン binding クレート採否（clack/vst3）・基盤構築方式 | 要PoC | plugin ADR の決定範囲外 | プラグイン基盤 Issue → 方式 ADR |
| `clack-extensions` thread-pool 等のプラグイン補助スレッド基盤 | 要PoC | binding 採否と連動 | プラグイン基盤 Issue |
| プラグイン GUI ウィンドウ基盤（baseview/wxp 等） | 要PoC | 有力候補だが未検証 | プラグイン GUI Issue |
| WebView 実装（wry/wxp 等）・描画バックエンド・配布 | 要PoC | ui ADR が PoC に委任 | UI PoC Issue → 採否 ADR |
| 高頻度描画バックエンド（Canvas2D/WebGL2/WebGPU/wgpu/ネイティブ合成） | 要PoC | 特性別評価・非固定順序（ui ADR） | UI PoC Issue |
| フロントエンドフレームワーク選定 | 要PoC | ui ADR が不固定を確定 | UI PoC Issue |
| スレッド優先度付与の具体機構（audio_thread_priority 型） | 要PoC | OS 差あり | RT 基盤 Issue |
| `assert_no_alloc` 型 CI 検査の具体実装 | 要PoC | 契約は決定・機構は後続 | RT 検証 Issue |
| thread-check 型のスレッド契約検査機構の具体方式 | 要PoC | 契約は決定・機構は未選定 | RT 検証 Issue |
| RT 通信・退避・スナップショット交換の実装方式・依存クレート（rtrb/basedrop/arc-swap 型等） | 要PoC | 構造は決定、実装依存は未採否 | RT 基盤 Issue |
| 連鎖変換プロパティテストの harness 選定（proptest dev-dep vs deterministic 列挙） | 要PoC | media-core 依存ゼロ方針との兼ね合い（Issue #42 設計文書） | 実装 Issue（media-core） |
| ライセンス正式決定（MIT or MIT OR Apache-2.0） | 保留 | licensing ADR が保留を確定 | 配布方針決定（公開前レビュー） |
| サンドボックス/クラッシュ分離の実装方式 | 保留 | 境界設計は決定・実装は後続 | プラグイン分離 Issue |
| OFX 等映像プラグイン規格の対応範囲 | 保留 | plugin ADR が別途設計に委任 | 第 2 ターゲット設計 Issue |
| mobile／組込／wasm 対応 | 保留 | platform ADR が初期ターゲット外を確定 | 初期ターゲット安定後の検討 |
| AU/AAX/LV2 等の追加規格 | 保留 | 初期対象外（plugin ADR） | 規格追加の別途検討 |
| 音楽時間（小節/拍/PPQN）設計 | 保留 | 自前設計必要・第 1 期スコープ外。第 1 期のイベント/トランスポート型は拍位置を持たず拡張予約で吸収 | 音楽時間設計 Issue（第 2 期・テンポマップ設計） |
| `f64` サンプル本格対応 | 保留 | f32 正準を決定・f64 は型パラメータ範囲 | 需要に応じた後続評価 |
| 色管理の広範統合（ACES/OCIO 級） | 保留 | 第 2 ターゲット課題 | 第 2 ターゲット設計 Issue |
| `std::simd` 安定化後の移行判断 | 保留 | stable-only が前提・安定化時期未定 | stable Rust での安定化後に再評価 |
| オフライン決定論的実行（seed/固定順序）の厳密保証範囲 | 保留 | 需要と検証設計が後続 | オフライン実行 Issue |
| 全コンポーネント共通の描画抽象 | 保留（不採用方針） | 描画/コンポーネント分離は決定・共通抽象は強制しない（ui ADR） | 要件変更時の再検討 |
| 個別機能ライブラリ採否（rubato・vtc・ringbuf 等） | 保留 | 包括ルール: 実装 Issue の依存追加規約（AGENTS.md）に委任 | 各実装 Issue |
| ドロップフレーム非 NTSC nominal 一般化・フィールドレート対応 | 保留 | SMPTE 上定義外・現需要なし（Issue #42 ADR） | インターレース素材・非 NTSC drop 需要発生時の再検討 |
| `Timecode` → `MediaTime` 逆変換 API | 保留 | Issue #42 受入条件外・需要未確定（Issue #42 設計文書の Deferred） | 編集/NLE 系需要発生時の後続 Issue |
| frame index 型の所有（media-core vs video-core） | 保留 | Issue #26/#27 境界の設計問題（Issue #42 設計文書の Deferred） | 動画基盤/動画エンジン Issue |

## 暫定実装ロードマップ（A.6）

Phase 間依存と根拠を記述する。**依存は原則として前フェーズ完了 → 次フェーズ**だが、同一 Phase 内の並行・順序は技術的効率で調整する。

### Phase 1: math＋基礎データ型

- 対象: `ruviform-math`、`ruviform-rt`、`ruviform-exec`、`ruviform-media-core`、`ruviform-audio-core`、`ruviform-video-core`。
- 根拠: 全上位層がこれらの型（時刻・バッファ・契約）に依存するため最下位から確定させる。SIMD は scalar 参照実装を先に確立し、SIMD 経路は PoC 結果で追加する。
- 完了目安: データモデル（時刻・バッファ・フレーム・レイアウト）と RT 契約型・プリミティブが API として成立し、単体テストが通る。

### Phase 2: 音声／動画基礎

- 対象: `ruviform-dsp`、`ruviform-audio-analysis`、`ruviform-video-proc`、`ruviform-media-io` + 第一バックエンド（純 Rust 系有力）、`ruviform-audio-device`。
- 依存: Phase 1 の型・契約の上に構築。処理カーネルの `prepare`/`process` 分離を具体化し、RT 契約をコードに反映する。
- 根拠: 計算カーネル・I/O 境界がエンジンの前提。動画は純 Rust 実用域が AV1/H.264 に限られる実態（discovery）から、純 Rust 中心 + 広範形式は FFmpeg 系等の分離クレートという段階構成が現実的（FFmpeg の採否自体は要 PoC。採否された場合の構成例）。
- 完了目安: demux/decode→処理→mux/encode のオフライン経路が抽象境界経由で動作し、デバイス経由の最小 RT パスが成立。

### Phase 3: エンジン・描画・UI・統合

- 対象: `ruviform-audio`、`ruviform-video`、`ruviform-gpu`、`ruviform-ui`/`ruviform-ui-webview`、`ruviform-plugin-*`。
- 依存: Phase 2 の処理モデル・契約の上に、エンジンのグラフ実行・同期・UI・プラグイン統合を構築。
- 根拠: エンジンは基礎カーネルと I/O が前提。UI・プラグインは最も外部依存が大きく PoC 結果を待つ。
- 完了目安: RT パス経由のプラグインチェイン駆動、A/V 同期再生、UI の概念実証。

### 補足

- **CLAP/VST3 の実装順序**は技術的依存・効率で決定する（同時必須でない。plugin ADR）。有力仮説は CLAP 先行（binding 候補 `clack` が純 Rust で SDK 外部依存が薄いため着手コストが低い）だが、順序の確定はプラグイン基盤 Issue の技術評価に委ねる。binding クレートの採否自体は要 PoC。**プラグイン GUI** は UI 基盤（PoC 先行）の後に組み込む順序とする。
- 各 Phase の具体的 Issue 分解は「後続実装 Issue の提案一覧」節に列挙する。

## #2 開発基盤への反映と README／AGENTS 差分タスク（A.8）

現行設定を読み、設計決定に伴う変更が必要な箇所を特定・タスク化する。

### `Cargo.toml`（workspace）

- `publish = false` 方針の反映: `[workspace.package]` に `publish = false` を追加し、各メンバーで `publish.workspace = true` を継承させるタスク（licensing ADR の未決定クレート原則）。現行 `crates/ruviform-core/Cargo.toml` には未設定。
- `license = "MIT"` は仮置きのまま維持（未決定の明記を確認）。ライセンス注記は README/AGENTS と整合。
- `edition`/`rust-version`/`lints`/`profiles` の再評価: edition 2021 → 2024 の是非、`rust-version = "1.85"` の MSRV 見直し（将来 `wgpu` MSRV 1.87・`clack` 1.85 等との整合）は専用の確認タスクへ。現段階で変更しない。
- クレート追加計画: `members` を実装 Phase に応じて段階追加する方針を記録（本書の DAG 名と対応）。
- `crates/ruviform-core` の処遇: Phase 1 で `ruviform-*` クレート群への分割置換として削除する（Phase 1 基盤 Issue のタスク）。Phase 1 着手までは smoke test 用の仮置きクレートとして残置し、`publish = false` を継承させる。

### `README.md`

- 「未確定（#1 で決定）」節の更新: 対象プラットフォーム（Windows/macOS/Linux・x86_64+aarch64）は決定済みとして記載に反映。公開 API・サポート形式・ライセンス・最終クレート分割は引き続き未確定として区別して記述。
- `docs/designs/` へのエントリポイントとして本書を追記。
- RT 契約・保証範囲（CI 成功≠完全証明）の注意書きを追加。

### `AGENTS.md`

- 「暫定原則（詳細契約は #1 で確定）」項の更新: RT/オフラインの設計意図を本書の確定契約（API・ライフサイクル分離・API 単位保証）へ差し替え。SIMD の暫定原則を本書の構造仮説（スカラー参照実装必須・SIMD 経路は要 PoC）へ更新。クレート依存方向を本書の DAG ルールへ引用。
- ライセンス注記は維持（仮置き・未決定）。
- feature/unsafe 規約は現行維持で整合を確認。

### `.github/workflows/ci.yml`

- 現行の `workflow_dispatch` 専用・Linux 単一 job は維持。
- 後続: RT 検証ジョブ（`assert_no_alloc` 型テスト lane）、OS matrix 拡張、ベンチマーク workflow を実装 Issue として追加する提案（本設計の diff には含めない）。

### `mise.toml`

- 後続: criterion ベンチ用タスク、RT 検査タスク（例: `test-rt`）を実装 Issue で追加する提案。現行タスク表は正本として維持。

### 後続実装 Issue の提案一覧

- Phase 1 基盤 Issue（math/rt/exec/media-core/audio-core/video-core のスケルトン + smoke、`crates/ruviform-core` の削除/置換）
- データモデル実装 Issue（media-core/audio-core/video-core の API 確定）
- DSP/SIMD PoC Issue（fearless_simd/pulp/multiversion 比較ベンチマーク → 採否 ADR）
- メディア I/O 実装 Issue（抽象境界 + 第一バックエンド PoC）
- RT 基盤 Issue（rt プリミティブの実装方式・スレッド優先度機構・スナップショット交換方式の評価）
- RT 検証 Issue（`assert_no_alloc` 型 CI レーン・thread-check 統合）
- デバイス PoC Issue（cpal 等オーディオデバイスバックエンドの評価・採否）
- オフライン実行 Issue（`ruviform-exec` ドライバ・チャンク駆動・決定論モードの保証範囲）
- 音声エンジン Issue（グラフ・デバイスアダプタ・RT パス）
- 動画基盤 Issue（video-core・video-proc の API・色変換/HDR 対応範囲）
- 動画エンジン Issue（フレームパイプライン・A/V 同期・シーク）
- UI PoC Issue（wry/描画バックエンド/フロントエンドフレームワーク/配布検証 → 採否 ADR）
- プラグイン基盤 Issue（plugin-core + CLAP/VST3 adapter、binding 方式 PoC → 方式 ADR）
- プラグイン GUI Issue（baseview/WebView-in-plugin 検証）
- プラグイン分離 Issue（サンドボックス/プロセス分離の実装方式 — 保留の再着手）
- 第 2 ターゲット設計 Issue（NLE 範囲・OFX 等映像プラグイン規格・広範色管理）
- 音楽時間設計 Issue（小節/拍/PPQN・テンポマップ — 保留の再着手）
- edition/MSRV/publish 反映 Issue（workspace.package の `publish = false`・edition 評価）
- CI 拡張 Issue（OS matrix・RT 検証・ベンチ workflow）

後続の決定フォーラム（実装 Issue 化対象外。分類表の次フォーラムと対応）:

- 配布方針決定・公開前ライセンスレビュー（ライセンス正式決定）
- 規格追加の別途検討（AU/AAX/LV2）
- 初期ターゲット安定後の検討（mobile／組込／wasm）
- 需要に応じた後続評価（`f64` サンプル本格対応）
- stable Rust での安定化後に再評価（`std::simd` 移行）
- 要件変更時の再検討（全コンポーネント共通描画抽象）

## Deferred Design Concerns

本書で決定しないが将来扱う設計項目（「保留」の再着手条件を固定するための記録。分類表の「保留」項目と対応）。

| 据え置く項目 | 理由 | 再着手トリガ | 無視した場合のリスク |
| --- | --- | --- | --- |
| ライセンス正式決定（MIT / MIT OR Apache-2.0、`LICENSE` 作成、`license` 仮置き確定） | 依存構成・配布条件の未検証で即時確定不可 | 公開前ライセンスレビュー | 仮置き値が確定扱いのまま公開が進む |
| サンドボックス/クラッシュ分離の実装方式 | 境界設計は阻害しない構造と決定済み | 分離要件の発生（プラグイン分離 Issue） | 後付けで ABI/境界の破壊的変更 |
| OFX 等映像プラグイン規格の範囲 | 第 2 ターゲット課題 | 第 2 ターゲット設計着手 | 音声規格前提の抽象が映像に不適合 |
| mobile／組込／wasm | 初期ターゲット外（platform ADR） | 初期ターゲット安定 | 排除前提設計が将来移植を阻害 |
| 音楽時間（小節/拍/PPQN） | 第 1 期スコープ外の自前設計。第 1 期のイベント/トランスポート型は拡張予約で吸収 | 第 2 期・テンポマップ設計（音楽時間設計 Issue） | メディア時間と混在した不整合な時間モデル |
| `f64` サンプル本格対応 | f32 正準で十分（現判断） | オフライン高精度需要 | 不必要な API 一般化コスト |
| 広範色管理（ACES/OCIO 級） | 第 2 ターゲット課題 | NLE 級色管理要求 | ICC 範囲のみの暫定実装を固定化 |
| AU/AAX/LV2 等の追加規格 | 初期対象外（plugin ADR） | 規格追加要求 | 不要な抽象の先行設計 |
| `std::simd` 安定化後の移行判断 | stable-only が前提・安定化時期未定 | stable Rust での安定化 | nightly 依存の設計への誤誘導 |
| オフライン決定論的実行の厳密保証範囲 | 需要と検証設計が後続（オフライン実行 Issue） | 決定論モード要求の発生 | 再現性保証を暗黙に期待した実装 |
| 全コンポーネント共通の描画抽象 | 現状は不採用方針（描画/コンポーネント分離と整合） | 要件変更 | 不要な抽象の先行固定化 |
| 個別機能ライブラリ採否（rubato・vtc・ringbuf 等） | 依存追加規約への委任ルール適用 | 各実装 Issue での需要発生 | 未管理の外部依存の混入 |

## Verification Notes

- [ ] 本書が spec `SPEC-034INYy2aDLxse9dy8MgGZ` の Requirements A.1–A.8 をすべてカバーしている
- [ ] クレート一覧が「責務・非責務／公開 API 境界／依存可能レイヤー／RT 制約」の 4 列で記述され、依存方向が一貫している
- [ ] RT／オフライン双方のデータフロー例が各 1 例以上あり、スレッド境界・バッファ所有・初期化/再構成の非 RT 配置が示されている
- [ ] すべての未決定事項が「決定／保留／要PoC」に分類され、根拠と次フォーラムが付記されている
- [ ] 設計が spec Requirements B・6 ADR の Decision Outcome と矛盾しない（特に: 外部型非露出・opt-in 分離、stable-only、RT 契約、CLAP/VST3 同格・ホスト/制作双方、WebView ファースト・高頻度描画の非固定順序、publish=false・ライセンス未決定）
- [ ] CI 成功は完全証明でない旨・OS ハード RT 保証なしが明記されている
- [ ] Phase 1→2→3 の暫定ロードマップと Phase 間依存・根拠が記述されている
- [ ] `README.md`／`AGENTS.md` の再確認結果と設計差分の更新タスクが特定されている
- [ ] `publish = false` 等の #2 基盤への反映変更がタスク化されている
- [ ] 未承認の決定（codec 採否・UI フレームワーク確定・ライセンス・基盤構築方式）を「確定」として記述していない
- [ ] 機械依存パス・秘密情報を含まない
