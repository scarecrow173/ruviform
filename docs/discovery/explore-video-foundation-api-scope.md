---
id: "DISC-034JcicdqiAQzbUKFHiCbc"
type: "discovery"
status: "draft"
title: "Issue #26 に向けた動画基盤 API・色変換/HDR 対応範囲の調査"
created: "2026-10-20"
updated: "2026-10-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/26"
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
  derived-by:
    - "../specs/implement-video-foundation-api.md"
    - "../adr/video-proc-kernel-api-and-color-hdr-scope.md"
    - "../plans/implement-issue-26-video-foundation.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/1"
    - "https://github.com/scarecrow173/ruviform/issues/41"
  defers: []
  deferred-by: []
---

# Issue #26 に向けた動画基盤 API・色変換/HDR 対応範囲の調査

## Exploration Goal

本書は GitHub Issue [#26 [impl] 動画基盤（video-core / video-proc API・色変換・HDR 対応範囲）](https://github.com/scarecrow173/ruviform/issues/26) の API 確定に先立つ調査の中間成果を記録する canonical ドキュメントである。調査は設計ベースライン（[ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md)）の契約事実、現行 `ruviform-video-core` の実装済み API インベントリ、Issue #41 トリアージ由来の受け入れ条件を統合し、7 つの決定論点（D1–D7）それぞれの選択肢と傾向を整理した。

本書が回答する問い:

- `Processor` trait・`RtContext` mint API が未実装の現状で、video-proc のカーネル API をどの形状にするか（D1）
- 初期実装するカーネルの範囲をどこまでにするか（D2）
- ベースライン採否表で「要PoC」とされた「色変換・HDR の対応範囲」を何に確定するか（D3）
- GPU 計算カーネル境界（ポート）の所有と trait 定義時期（D4）
- `SharedPlane`/`VideoFrame` の所有・drop・zero-copy 契約をどう文書化し、型拡張が必要か（D5）
- 新クレート `ruviform-video-proc` の依存配線（D6）とエラーモデル（D7）

> **位置づけ**: 本書は調査の推論経路と一次情報を記録するものであり、最終決定を含めない。下記の傾向（Lean）はすべて「仮説」であり、確定は並行作成された [spec](../specs/implement-video-foundation-api.md) と [ADR](../adr/video-proc-kernel-api-and-color-hdr-scope.md) が担う（`relations.derived-by`）。一次調査ブリーフは `tmp/issue-26/research-brief.md`（リポジトリ管理外の作業ファイル）に残る。

## Key Issues and Assumptions

### 設計ベースラインの契約事実（前提として確定済み）

- クレート責務: `ruviform-video-core` は動画フレーム/plane/pixel 形式・stride・色空間・HDR メタデータ・フレーム時刻を持ち、**変換・合成・I/O は持たない**。フレーム参照は RT 適合、確保・pool は非 RT API（ベースライン クレート責務表 L194）。
- `ruviform-video-proc` の責務は**色変換・スケーリング・合成の基礎処理カーネルと GPU 計算カーネル境界（ポート）の定義**。`process` 系は RT 適合設計（alloc なし）だが、RT 保証は API 単位で個別明示されるまで非保証。初期化は非 RT（同 L197）。
- 処理契約の形状: `prepare(setup)`（非 RT・確保可）で初期化・構成を確定し、`process(&mut self, io, ctx)` を持つ別の処理オブジェクトを生成する。`Processor` trait と `RtContext` は `ruviform-rt` が所有（ベースライン「RT／非 RT の分離方式」L283）。
- `RtContext` は発行規則によりドライバ（RT 駆動者・オフライン駆動者）のみが mint 可能で、アプリコードは構築不可（L284）。
- DSP/SIMD 決定: **スカラー参照実装を必須**とし、カーネルごとに「ビット完全一致」または「ulp 上限内」の等価性契約を定義してスカラーを参照として検証する。SIMD 二層構造・バックエンド選定は要 PoC（L363–376）。
- 採否表: 「色変換・HDR の対応範囲」は**要 PoC** で「動画基盤 Issue」（＝ #26）に割当済み（L479）。「デコーダ zero-copy バッキングの具体方式」は media-io 実装 Issue の要 PoC（L478）。広範色管理（moxcms/OpenColorIO 級）は**保留**（第 2 ターゲット設計時、L275–276・L497）。
- 層内順序: `audio-core`・`video-core` → `dsp`・`video-proc`・`media-io`（**同順位同士の相互依存は禁止**）→ `audio-analysis`（L169）。GPU 境界 API は利用側ポートとして `video-proc`（計算）と `ui`（描画）が定義する（L179）。

### 現行実装インベントリ（調査で確認済み）

- `ruviform-video-core`（deps: `ruviform-media-core` のみ）:
  - `PixelFormat { Yuv420p, Yuv422p, Yuv444p, Rgba8, RgbaF16 }`（`#[non_exhaustive]`、`pixel.rs:34`）。`plane_count`/`is_planar`/`bytes_per_sample`/`chroma_subsampling` は全て `const fn`（Yuv\*=3 plane・8bit、Rgba8=4B/px、RgbaF16=8B/px、420=(2,2)・422=(2,1)・444=(1,1)）。
  - `Plane<'a>`（`Copy`・読取）/ `PlaneMut<'a>`（書込）: 借用ビュー。使用領域は `stride*(h-1)+row_bytes` に trim され、`row(y)`/`row_mut(y)` は `Option` 返却で padding を跨がない（`plane.rs`）。構築は `pub(crate)`。
  - `SharedPlane { pub data: Arc<[u8]>, pub stride: usize }`（`plane.rs:313`。**offset フィールドなし** — plane 先頭オフセットを表現できない既知の制約）。
  - `VideoFrame`（`frame.rs:129`、private fields・`Clone` = Arc 共有）: `new_owned`（ゼロ初期化・tightly packed・`isize::MAX` 上限）と `from_shared_planes`（**許容された zero-copy 供給経路**。plane 数・stride≥row_bytes・data 長を検証）。`plane_mut` は **`Arc::get_mut` ゲート** — clone 共有中は `None`。
  - 色メタデータ: `ColorPrimaries`/`TransferCharacteristics`/`MatrixCoefficients`/`ColorRange`（全 `#[non_exhaustive]`・`#[default] Unspecified`）+ `Chromaticity`/`MasteringDisplayInfo`/`ContentLightLevel`/`HdrMetadata`/`ColorInfo`（`color.rs`）。
- `ruviform-rt` は `RtContext { _private: () }` のみで **`Processor` trait・mint API は未実装**（`rt/src/lib.rs:44`）— `RtContext` を要求する API は現時点でテストからも呼べない。
- `ruviform-media-core::Error` は `#[non_exhaustive]`・`Copy`・RT-suitable（`InvalidValue(&'static str)` 等、`error.rs:19`）。
- `audio-core` の先例: 非 RT 確保系コンストラクタ（`new`/`try_new`/`from_interleaved`）+ RT 借用ビュー（`view`/`view_mut`→`AudioBufferView`/`ViewMut`）の分離（`audio-core/src/buffer.rs`）。

### Issue #26 コメントの受け入れ条件（#41 トリアージ item 6 由来）

- `SharedPlane`/`VideoFrame` の所有権モデル文書化: Arc 強共有 + `Arc::get_mut` 書き込みゲート（共有中は読取専用、排他所有時のみ書込可）。
- drop 挙動 + 将来のフレームプールへの返却・所有権移転契約の定義。
- 既存 `SharedPlane`/`VideoFrame`/`Plane`/`PlaneMut` API との zero-copy 経路整合性（外部バッファ取り込みを含む）。
- 借用・共有系 API への `**RT contract:**` ラベル（alloc/dealloc 挙動をカバー）。

## Alternatives and Comparison

### D1. video-proc カーネル API 形状（`Processor`/`RtContext` 不在）

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) 状態なし自由関数 `convert(src, dst)` | 最も単純 | `prepare`/`process` 分離の契約形状（ベースライン L283）に合わない。係数・ワーク領域の事前確保先がない | no |
| (b) 非 RT `prepare`/コンストラクタ → 準備済みオブジェクトの `process(&mut self, src, dst)`（ctx なし） | 契約形状と一致。`RtContext` 不要で今日実装・テスト可能。将来の `Processor` impl は後方互換で追加可能 | 将来 trait 導入時にシグネチャ整合の注意が要る | yes |
| (c) `ruviform-rt` に `Processor` trait を今定義 | 正規の契約型に早期到達 | RT 基盤 Issue（#22 系）の権限を侵す。I/O 型のジェネリック設計を本 Issue で巻き取るのは越権 | no |
| (d) `process` が `RtContext` 引数を要求 | 最終形に近い | mint API が存在せず誰も呼べない（`rt/src/lib.rs:44`）。実質テスト不能 | no |

### D2. 初期カーネル実装範囲

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) 色変換のみ実装（Yuv*p ↔ Rgba8 + Rgba8 ↔ RgbaF16） | 要 PoC 行「色変換・HDR 対応範囲」を最小の実装で確定できる。スカラー参照の役割を果たす | スケーリング・合成は未実装のまま | yes（縮小版） |
| (b) (a) + リサイズ/合成カーネル | video-proc 責務の全領域を早期カバー | スコープ過大。フィルタ設計・合成モデルの論点が増え本 Issue が収束しない | no |
| (c) API 定義のみで実装は次 Issue | 本 Issue が軽い | 「対応範囲」の確定が空文化し、等価性契約・エッジケース規定の検証対象がない | no |

### D3. HDR「対応範囲」

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) メタデータ搬送 + 非破壊のみ（code-value 演算、transfer 関数は適用しない） | 第 1 ターゲットに十分。実装・検証が小さい。`ColorInfo` 型の設計意図（記述のみ・変換しない）と整合 | PQ/HLG の正しい表示変換はできない（上位層の責務として残る） | yes |
| (b) (a) + transfer 考慮変換（PQ/HLG→リニア化・トーンマップ） | HDR 実用性が上がる | リニア化は浮動小数点パイプライン・出力 range 規定を要求しスコープが跳ねる。ベースラインは広範色管理を保留済み | no |
| (c) 広範色管理（moxcms/OCIO 級） | 最終的な正しさ | ベースラインで**保留**と明記（第 2 ターゲット設計時）。本 Issue の権限外 | no |

### D4. GPU 計算ポート trait

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) ポート trait を今定義 | 境界を早期固定できる | 実装者ゼロの trait は YAGNI。形状が wgpu 採否（要 PoC）・バックエンド選定に依存 | no |
| (b) 所有宣言のみ記録し trait 定義は ruviform-gpu Issue へ | ベースライン L179 の責務所在を明文化しつつ過剰設計を回避 | trait 不在の間は GPU 経路が未接続（許容済み） | yes |
| (c) 完全後送り（宣言もしない） | 追加作業ゼロ | ポート所有が暗黙化し、gpu Issue 側で責務争いが再燃する余地 | no |

### D5. `SharedPlane`/`VideoFrame` 所有・zero-copy・プール契約

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (i) 現行維持 + 契約文書化（`SharedPlane` は offset-less のまま。decoder が plane 単位の `Arc` を供給する前提） | 型を変えずに AC の文書化要求を満たす。FFmpeg の per-plane `AVBufferRef` モデルと概ね整合 | 外部バッファの plane 先頭オフセットを表現できない制約が残る（media-io で再評価） | yes |
| (ii) `SharedPlane` に `offset: usize` 追加 | 一つの `Arc` から複数 plane を切り出せる | 未検証の media-io 実需を先回りで固定化。検証規則が複雑化 | no |
| (iii) `PlaneBacking` enum（Owned/External{arc, offset, len}） | 最も表現力が高い | 公開 API 表面が膨らみ、zero-copy 具体方式（media-io 要 PoC）を先取りする | no |

付随論点 — `VideoFrame` の共有/排他状態 introspection（`is_exclusively_owned` 相当）を今追加するか: プール返却設計で必要になる可能性があるが、文書化目的だけなら YAGNI 寄り。**契約文書化が必要とする場合のみ最小追加**とするのが妥当（仮説）。

### D6. クレート配線

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `crates/ruviform-video-proc` を `members` に追加（video-core 直後）。deps = `ruviform-video-core`（+ `ruviform-media-core` は `Error` 使用時のみ）。`ruviform-rt`・同順位クレートへの依存なし | 層内順序（L169）と依存方向ルールに整合。YAGNI で rt 非依存 | 将来 `Processor` impl 時に rt 依存追加が必要（その時点で正当化される） | yes |
| `ruviform-rt` にも依存しておく | 将来の接続が楽 | `RtContext` を取らない設計では未使用依存。YAGNI 違反 | no |

### D7. エラーモデル

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) `ruviform_media_core::Error` 再利用 | video-core/audio-core と同一パターン。`Copy`・RT-suitable・`#[non_exhaustive]` で variant 追加も non-breaking | video 固有エラーの粒度は `InvalidValue("…")` に依存 | yes |
| (b) video-proc 独自 `#[non_exhaustive] enum Error` | ドメイン固有 variant を自由に設計可 | エラー型の分岐が増え、変換パイプラインで 2 系統を橋渡しする必要 | no |
| (c) thiserror 等の外部クレート | 記述が楽 | 外部依存追加の禁止方針（std のみ）に反する | no |

## Tentative Conclusions and Hypotheses

すべて仮説（確定は spec・ADR の役割）。裏付けは「Key Issues and Assumptions」の一次情報。

- **仮説 D1-b**: カーネルは「非 RT の `prepare`/コンストラクタ → 準備済み処理オブジェクト → alloc-free の `process(&mut self, src, dst)`」形状とし、`RtContext` 引数も `Processor` trait 実装も持たない。`ruviform-rt` の `Processor` 実装 Issue で trait impl を後方互換的に追加する余地を残す。`**RT contract:**` ラベルは既存クレート同様に API 単位で付す。
- **仮説 D2-a（縮小）**: 初期実装はスカラーのみの色変換 — `Yuv*p ↔ Rgba8`（`MatrixCoefficients` Bt601/Bt709/Bt2020・`ColorRange` Limited/Full を考慮、chroma resampling 含む）と `Rgba8 ↔ RgbaF16`。スケーリング・合成は将来カーネルとして予約し実装しない。カーネルごとに等価性契約（スカラー = 参照実装、ベースライン L370/L375）を doc 明記する。
- **仮説 D3-a**: HDR 対応範囲 = **メタデータ搬送 + 非破壊**。transfer 関数の適用・トーンマップは行わない。`Unspecified` 色フィールドの扱いと出力 `ColorInfo` 伝播規則を規定する。
- **仮説 D4-b**: video-proc の crate doc が GPU 計算カーネル境界のポート所有を宣言し、trait 本体は ruviform-gpu 実装 Issue に移管。
- **仮説 D5-i**: `SharedPlane`/`VideoFrame` の現行所有モデル（Arc 強共有 + `Arc::get_mut` 書込ゲート）を文書化し、drop 挙動（最後の `Arc` が backing を解放 → RT パスでの frame drop は dealloc 違反 → 退役は非 RT 送出 = Drop 退避 L287）と将来のフレームプール返却・所有権移転の契約方向を定義する。`SharedPlane` の offset-less は既知の制約として記録し media-io Issue で再評価。introspection API は契約文書化が必要とする場合のみ最小追加。
- **仮説 D6/D7**: `crates/ruviform-video-proc` を追加し deps は `ruviform-video-core`（+ 必要時 `ruviform-media-core`）。`ruviform-rt`・同順位依存はなし。エラーは `ruviform_media_core::Error` を再利用。
- **確認済み事実**: ベースライン節・現行コード引用はすべてファイル・行番号で検証済み（ブリーフ A/B 節）。`gh` CLI 非搭載環境のため Issue コメント本文の直接確認は未実施（下記 Open Questions）。

## Open Questions

- Issue #26 コメント本文の最終確認（本書の AC 認識は #41 トリアージ対応表由来の二次情報）— 実装着手前に `gh issue view 26 --comments` で親エージェント/メンテナが確認すべき。
- 4:2:0 chroma siting（水平 left/center 共置）を `PixelFormat`/`ColorInfo` で表現する手段がない — 変換カーネルの siting 前提をどこに固定するか（spec で規定予定。半画素ずれリスクの扱い）。
- `SharedPlane` offset-less 制約が media-io の zero-copy 実需（一つの `AVBufferRef` からの複数 plane 参照等）を満たすか — media-io 実装 Issue で再評価。
- video 固有エラー variant（`UnsupportedFormat` 相当）を media-core `Error` に追加する要否 — 実装時判断（`#[non_exhaustive]` のため non-breaking）。
- `test-rt` の substring フィルタ精度（非 RT テスト名に `rt_` を含めない運用注意）— Issue #22 の既知課題。
- `Yuv444p` の `chroma_subsampling() == Some((1,1))` 統一解釈（IR `implement-video-core-data-model.md` の open question）— 変換実装で自然確定する見込み。

## Promotion Candidates

- [x] spec-doc needed: Issue #26 の API 確定スコープ（video-core 所有モデル文書化 + video-proc クレート新設・色変換カーネル + HDR 対応範囲の確定）を [implement-video-foundation-api.md](../specs/implement-video-foundation-api.md)（SPEC-034JcicHlRidBgcK1wPtj2、approved）として並行作成済み
- [x] adr-doc needed: D1–D7 の設計判断（カーネル API 形状・実装範囲・HDR 範囲・GPU ポート・所有/zero-copy 契約・配線・エラーモデル）を [video-proc-kernel-api-and-color-hdr-scope.md](../adr/video-proc-kernel-api-and-color-hdr-scope.md)（ADR-034JcidaiOOip5EhNPOnDI、accepted）として並行作成済み。ベースライン採否表の「色変換・HDR の対応範囲」行（要PoC → 動画基盤 Issue）を本 ADR で決定に昇格する
- [ ] 今後の昇格候補: 本 spec からの実装 plan/task（`docs/plans/`・`docs/tasks/`）、GPU ポート trait 本体（ruviform-gpu Issue）、フレームプール返却の詳細設計（非 RT 層の後続設計）、`SharedPlane` offset 再評価（media-io Issue）

## Sources and Relations

- 一次ソース: GitHub Issue [#26 [impl] 動画基盤（video-core / video-proc API・色変換・HDR 対応範囲）](https://github.com/scarecrow173/ruviform/issues/26)（`relations.source`）。プレーン/stride/pixel format 表現・色変換・HDR メタデータの対応範囲を含む API 確定を要求。親 Issue は #1。
- 上流文書（`relations.derives-from`）: [設計ベースライン](../designs/ruviform-architecture-design-baseline.md) — クレート責務表（video-core L194 / video-proc L197）、動画データモデル節（L252–259）、RT／非 RT 分離（L280–288・L335–337）、DSP/SIMD 決定（L363–376）、採否分類表の「色変換・HDR の対応範囲 = 要PoC → 動画基盤 Issue」行（L479）。
- 調査ブリーフ: `tmp/issue-26/research-brief.md`（リポジトリ管理外の作業成果物）。本書の各判断材料（API インベントリ・決定論点・エッジケース）の一次集約。
- 派生（`relations.derived-by`）: [spec](../specs/implement-video-foundation-api.md)・[ADR](../adr/video-proc-kernel-api-and-color-hdr-scope.md) — 本調査の仮説を確定する下流文書。
- 関連（`relations.related`）: [`AGENTS.md`](../../AGENTS.md)（暫定開発規約 — unsafe 原則禁止・enum ポリシー・検証入口）。
- 参照（`relations.references`）: Issue [#1](https://github.com/scarecrow173/ruviform/issues/1)（設計ベースラインの一次ソース・#26 の親）、Issue [#41](https://github.com/scarecrow173/ruviform/issues/41)（#26 へ移管された受け入れ条件のトリアージ表の所在）。
