---
id: "PLAN-034JcCFGe6ILSTWLgUIjAn"
type: "plan"
status: "approved"
title: "Issue #26 動画基盤 API の実装（video-core 所有権契約・video-proc 新設・色変換カーネル）"
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
  implements:
    - "../specs/implement-video-foundation-api.md"
  implemented-by:
    - "../tasks/document-video-core-ownership-contract.md"
    - "../tasks/create-video-proc-crate-skeleton.md"
    - "../tasks/implement-yuv-to-rgb-kernel.md"
    - "../tasks/implement-rgb-to-yuv-and-f16-kernels.md"
    - "../tasks/sync-docs-issue-26.md"
  depends-on:
    - "implement-issue-18-data-model.md"
    - "issue-18-followup-enum-and-time-fix.md"
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
    - "../specs/implement-video-foundation-api.md"
    - "../adr/video-proc-kernel-api-and-color-hdr-scope.md"
    - "../discovery/explore-video-foundation-api-scope.md"
    - "../designs/ruviform-architecture-design-baseline.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/public-enum-non-exhaustive-policy.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/video-foundation-api.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/1"
    - "https://github.com/scarecrow173/ruviform/issues/41"
  defers: []
  deferred-by: []
---

# Issue #26 動画基盤 API の実装（video-core 所有権契約・video-proc 新設・色変換カーネル） Implementation Plan

## Goal

spec `SPEC-034JcicHlRidBgcK1wPtj2`（Issue #26）の受け入れ基準 AC1–AC11 を充足する: `ruviform-video-core` の `SharedPlane`/`VideoFrame` 所有権・drop・zero-copy 契約と `**RT contract:**` ラベルを rustdoc に確定し、新クレート `ruviform-video-proc` を新設してスカラー色変換カーネル（`Yuv420p`/`Yuv422p`/`Yuv444p ↔ Rgba8`、`Rgba8 ↔ RgbaF16`）を「非 RT の構築 → 準備済みオブジェクトの alloc-free `process`」の二相 API 形状で実装する。あわせて設計ベースライン採否表の「色変換・HDR の対応範囲 = 要PoC」行を本 Issue の決定（メタデータ搬送 + 非破壊）へ更新する。

## Scope

単一の実装ストリームとして扱う。video-core の契約文書化（T1）・video-proc 新設（T2）・色変換カーネル（T3a/T3b）・docs 同期（T4）は同一 Issue の受け入れ条件であり、video-proc が video-core の `VideoFrame`/`ColorInfo`/`PixelFormat` 表面に依存するため共通のクリティカルパスとリリース意図を共有する。T3 は YUV→RGB（upsample）と RGB→YUV（downsample）+ f16 でファイル・検証単位を分離し T3a/T3b に分割する。

範囲外（spec Out of Scope と一致）: `Processor` trait 定義・`RtContext` 引数・`ruviform-rt` 依存、スケーリング・合成カーネル、transfer 関数適用・トーンマッピング・広範色管理、GPU ポート trait 本体、`SharedPlane` offset・`PlaneBacking` enum 化、フレームプール実装、SIMD バックエンド・ランタイム dispatch、codec・demux/mux・I/O、`PixelFormat` 新 variant、外部クレート依存・`unsafe`、外部クレート型の公開 API 露出。spec の `status` 遷移（approved→implemented）は impl-record/最終同期ステップの責務であり T4 でも行わない。

## Upstream Documents

- [../specs/implement-video-foundation-api.md](../specs/implement-video-foundation-api.md) — `SPEC-034JcicHlRidBgcK1wPtj2`（approved）。本 plan が実装する spec。範囲・Requirements・AC1–AC11 の正本
- [GitHub Issue #26](https://github.com/scarecrow173/ruviform/issues/26) — 一次ソース（`relations.source`）。受け入れ条件は #26 コメント（#41 トリアージ item 6 由来: 所有権モデル・drop/プール契約・zero-copy 整合性・RT ラベル）

## Design Inputs

- [../adr/video-proc-kernel-api-and-color-hdr-scope.md](../adr/video-proc-kernel-api-and-color-hdr-scope.md) — `ADR-034JcidaiOOip5EhNPOnDI`（accepted）。D1–D7 確定の正本: 二相カーネル形状（D1-b）・実装範囲（D2-a 縮小版）・HDR=メタデータ搬送+非破壊（D3-a）・GPU ポート所有宣言（D4-b）・現行所有モデル維持+契約文書化（D5-i）・配線（D6-a）・media-core `Error` 再利用（D7-a）
- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — `DESIGN-034IPynWe2V30FLubzQenb`（approved）。クレート責務表（video-core L194 / video-proc L197）・RT/非 RT 分離（L282–287）・DSP/SIMD 決定（L370–376）・層内順序（L169）・採否表 L482 の要PoC 行
- [../adr/realtime-contract-and-verification-policy.md](../adr/realtime-contract-and-verification-policy.md) — RT ADR。「保証範囲は API 単位で定義・明示」→ 全対象 API の `**RT contract:**` ラベル規則
- [../adr/public-enum-non-exhaustive-policy.md](../adr/public-enum-non-exhaustive-policy.md) — `ADR-034J6OnZouPI8HQy6cIg9D`。公開 enum `#[non_exhaustive]`・未知 variant は `_` arm で graceful 処理（`unreachable!()` 禁止）
- [../discovery/explore-video-foundation-api-scope.md](../discovery/explore-video-foundation-api-scope.md) — `DISC-034JcicdqiAQzbUKFHiCbc`。現行 API インベントリ・D1–D7 選択肢分析・エッジケース一覧の正本

## File Map

| Path | Responsibility | Notes |
| --- | --- | --- |
| `crates/ruviform-video-core/src/lib.rs` | crate doc: 所有権モデル・drop/RT ライフサイクル規則の集約 | T1 |
| `crates/ruviform-video-core/src/frame.rs` | `VideoFrame` クラス doc（Clone=Arc 共有・drop=dealloc・RT での所有 drop 禁止・Drop 退避・プール返却方向）+ `new_owned`/`from_shared_planes`/`plane`/`plane_mut` doc 精緻化 | T1 |
| `crates/ruviform-video-core/src/plane.rs` | `SharedPlane` doc（offset-less 既知制約・`from_shared_planes` が唯一の外部供給経路）+ `Plane`/`PlaneMut` ラベル確認 | T1 |
| `crates/ruviform-video-core/src/color.rs` | モジュール doc の「対応範囲は要PoC」記述を決定済み範囲へ更新 | T1 |
| `Cargo.toml` | `members` に `crates/ruviform-video-proc` を video-core 直後に追加 + メンバーコメント同期 | T2 |
| `crates/ruviform-video-proc/Cargo.toml` | workspace 継承 + `ruviform-video-core` path 依存 | T2 新設（T3a で `ruviform-media-core` 追加） |
| `crates/ruviform-video-proc/src/lib.rs` | crate doc（責務・許可依存層・RT 制約・GPU ポート所有宣言・`Processor` 拡張余地）+ `version()` + 再エクスポート | T2 骨格。T3a/b で再エクスポート追加 |
| `crates/ruviform-video-proc/src/coef.rs`（仮） | matrix（Bt601/Bt709/Bt2020）× range（Limited/Full）係数導出の内部共有ロジック | T3a 新設。T3b が再利用 |
| `crates/ruviform-video-proc/src/yuv_to_rgb.rs` | `YuvToRgb` 準備済みカーネル + chroma upsample | T3a |
| `crates/ruviform-video-proc/src/rgb_to_yuv.rs` | `RgbToYuv` 準備済みカーネル + chroma ダウンサンプル | T3b |
| `crates/ruviform-video-proc/src/rgba_f16.rs` | `Rgba8ToRgbaF16` / `RgbaF16ToRgba8` カーネル | T3b |
| `Cargo.lock` | 新メンバー・依存の記録 | T2/T3a で `cargo check`/`build` により自動更新 |
| `docs/designs/ruviform-architecture-design-baseline.md` | 採否表「色変換・HDR の対応範囲」行の 要PoC→決定 更新 + 動画データモデル節の要PoC 箇条（L278 付近）の同期 | T4 |
| `AGENTS.md` | 「ワークスペース構成」節のメンバー一覧・Phase/確定状況文言の同期 | T4 |
| `README.md` | クレート一覧・構成記述の同期 | T4 |

## Implementation Sequence

### Phase A — T1: video-core 所有権・drop・zero-copy 契約の文書化（AC1–AC4）

- [ ] A-1. `lib.rs` crate doc に所有権モデルを集約する: `VideoFrame` の `Clone` は `Arc` 参照の共有（コピーではない）・`Arc::get_mut` 書込ゲート（共有中は読取専用・排他所有時のみ `plane_mut` が `Some`）・`VideoFrame` drop は最後の `Arc` 解放で backing を dealloc するため RT パスで所有を drop しない（退役フレームは非 RT 層へ送出 = Drop 退避）・フレームプールは非 RT 層が所有し返却・所有権移転は明示的 API として後続設計（現行契約は「共有 backing の refcount 管理 + 排他時のみ可変」まで）
- [ ] A-2. `frame.rs` `VideoFrame` クラス doc とメソッド doc を精緻化: `plane_mut()==None` を共有状態の観測点として明記（AC1）・`from_shared_planes` の検証規則（plane 数・`stride >= row_bytes`・`data.len() >= stride*(h-1)+row_bytes`）と唯一の外部供給経路であること（AC3）・確保系 API は non-RT only
- [ ] A-3. `plane.rs` `SharedPlane` doc に offset-less 制約（plane 先頭オフセット表現不可 → media-io Issue で再評価）と供給経路の唯一性を記録。借用・共有系 API（`plane`/`plane_mut`/`row`/`row_mut`/`data`/`data_mut`/`SharedPlane`/`VideoFrame::clone`/drop/確保系）の `**RT contract:**` ラベルが alloc/dealloc 挙動をカバーすることを監査・精緻化（AC4）
- [ ] A-4. `color.rs` モジュール doc の「対応範囲は要PoC」記述を決定済み範囲（HDR = メタデータ搬送 + 非破壊、transfer 適用・トーンマップなし、ADR 参照付き）へ更新
- [ ] A-5. introspection API は既定追加なし（`plane_mut()==None` が観測点）。契約文書化が必要とする場合のみ最小追加（例: `is_shared()`/`is_exclusively_owned()`）を許容し、追加時は根拠を記録する（ADR D5-i）

### Phase B — T2: ruviform-video-proc クレート骨格（AC5・AC10）

- [ ] B-1. `crates/ruviform-video-proc/Cargo.toml` を video-core 同形で作成（`version`/`edition`/`rust-version`/`license`/`publish` は workspace 継承、`[lints] workspace = true`、deps は `ruviform-video-core` path のみ）
- [ ] B-2. ルート `Cargo.toml` `members` に video-core 直後で追加しメンバーコメントを同期
- [ ] B-3. `src/lib.rs` に crate doc（責務=色変換・スケーリング・合成の基礎処理カーネル / codec・I/O 非保持、許可依存層=Primitives + Domain foundations、RT 制約=`process` 系は RT 適合設計だが保証は API 単位明示まで非保証・初期化は非 RT、**GPU 計算カーネル境界のポート所有は本クレート** — trait 本体は ruviform-gpu 実装 Issue、`Processor` trait 定義時に impl 追加で接続する後方互換の拡張余地）+ `version()` + doctest + smoke test + `#!` allow 群（video-core 同形）

### Phase C — T3a: カーネル API 形状 + YUV→RGB（AC6・AC7 前半・AC8・AC9）

- [ ] C-1. video-proc `Cargo.toml` に `ruviform-media-core` path 依存を追加（`Error`/`Result` 使用のため — ADR D7-a）
- [ ] C-2. 係数導出の内部共有ロジック（`src/coef.rs` 仮）: `MatrixCoefficients`（Bt601/Bt709/Bt2020）× `ColorRange`（Limited/Full）の係数導出。`Unspecified` 既定規則（matrix→Bt601・range→Limited）、`Identity`/未知 variant → `Err`（`_` arm・`unreachable!()` 禁止）
- [ ] C-3. `YuvToRgb` 準備済みカーネル: 非 RT コンストラクタ（係数・ワーク領域確定、non-RT only ラベル）+ `process(&mut self, src: &VideoFrame, dst: &mut VideoFrame) -> Result<()>`（`**RT contract:**` — alloc なし・ブロッキングなし・有界処理。`RtContext` 引数・`Processor` impl なし）。src `Yuv*p` → dst `Rgba8` の format・寸法・plane 検証と dst 共有中（`plane_mut()==None`）→ `Err`
- [ ] C-4. chroma upsample（420/422 → luma グリッド再構成。siting 前提: 水平 co-sited・垂直 center を doc 明記。奇数寸法の半端 chroma は存在する列・行のみ使用）・`row()`/`row_mut()` 単位処理（stride≠row_bytes で padding を跨がない）・出力 `ColorInfo` 更新（matrix→Identity・range 変換時は実値へ・primaries/transfer/hdr 伝播）
- [ ] C-5. 等価性契約 doc（スカラー=参照実装・整数演算のためビット完全一致）+ golden vector（3 matrix × 2 range）+ 単体テスト（エッジケース・Err 経路）+ doctest + `rt_` 契約テスト

### Phase D — T3b: RGB→YUV + Rgba8↔RgbaF16（AC7 後半・AC8・AC9）

- [ ] D-1. `RgbToYuv` カーネル: src `Rgba8` → dst `Yuv420p`/`Yuv422p`/`Yuv444p`（dst format で選択）。適用係数はカーネル構築引数で指定（既定規則・`Identity`/未知 variant→構築時 `Err` は C-2 の共有ロジックを再利用。plan Open Question Q2）。chroma downsample（420=2×2・422=水平 2 の平均、端数列・行は存在画素のみ）。出力 `ColorInfo`: matrix→適用係数・range→適用 range・残り伝播
- [ ] D-2. `Rgba8ToRgbaF16`/`RgbaF16ToRgba8` カーネル: チャンネル値の数値変換のみ（`ColorInfo` 不変伝播）。primitive `f16`（Rust 1.82+、MSRV 1.85 で可）で `to_le_bytes`/`from_le_bytes` によりバイト読み書き。正規化・丸め規則（Q3 既定）と等価性契約（ulp 上限・具体値）を doc 明記（**実績注記**: primitive `f16` は `x86_64-pc-windows-msvc` で pinned ツールチェーン 1.99.0 時点でも unstable — E0658・rust-lang/rust#116909 — のため前提不成立。出荷実装は `f16::from_f64` と全入力ビット一致のソフトウェア IEEE 754 binary16 コーデック。LE binary16 のワイヤ契約は不変。詳細は [impl record](../impl/ir/implement-rgb-to-yuv-and-f16-kernels.md)）
- [ ] D-3. golden vector + 単体テスト + doctest + `rt_` 契約テスト + `lib.rs` 再エクスポート

### Phase E — T4: ドキュメント同期・総合検証（AC11）

- [ ] E-1. baseline 採否表「色変換・HDR の対応範囲」行を 要PoC→決定（メタデータ搬送+非破壊・スカラーのみ、ADR 参照）+ 動画データモデル節の要PoC 箇条を同期
- [ ] E-2. `AGENTS.md` ワークスペース構成節（メンバー一覧・Phase/確定状況文言）・`README.md` クレート一覧を video-proc 追加に同期
- [ ] E-3. 総合検証スイート（`mise run fmt-check`/`check`/`lint`/`test`/`test-rt`・`cargo metadata`・`prek run --all-files`・docs audit）を実行

## Dependencies and Risks

### Dependency Graph

| Step | Depends on | Blocks |
| --- | --- | --- |
| T1 video-core 契約文書化 | plan・上流 spec/ADR | T3a・T4 |
| T2 video-proc 骨格 | plan | T3a・T4 |
| T3a カーネル API + YUV→RGB | T1・T2 | T3b・T4 |
| T3b RGB→YUV + f16 | T3a | T4 |
| T4 docs 同期・検証 | T1・T2・T3a・T3b | — |
| 本 plan | [implement-issue-18-data-model.md](implement-issue-18-data-model.md)・[issue-18-followup-enum-and-time-fix.md](issue-18-followup-enum-and-time-fix.md) | video-core の `VideoFrame`/`SharedPlane`/`ColorInfo`/`PixelFormat` API 表面と色 enum の `#[non_exhaustive]` 化を前提 |

- クリティカルパス: T2 → T3a → T3b → T4。T1 は T2 と並行可能（video-core doc と新規ディレクトリでファイル競合なし）だが、T3a が T1 の契約文言に整合するため T1 を T3a より先に置く
- video-proc は同順位クレート（将来の `dsp`/`media-io`）へ依存しない。`ruviform-rt` 依存なし（D6-a）

### Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | データ損失 | なし | — | 新規クレート + doc 変更のみ。既存 API の挙動変更・永続データ変換なし |
| 2 | 破壊的変更 | 低 | 低 | `publish = false` で利用者は workspace 内のみ。introspection API を追加する場合も additive。media-core `Error` variant 追加は `#[non_exhaustive]` のため non-breaking |
| 3 | 外部依存の利用可否 | なし | — | 外部クレート禁止・std のみ。f16 は primitive `f16`（追加依存なし） |
| 4 | RT ラベル漏れ・契約逸脱 | 中 | 中 | AC4 を完了条件化 + `**RT contract:**` grep 監査 + `rt_` 契約テスト + レビュー |
| 5 | `rt_` substring フィルタ誤検出 | 中 | 低 | 非 RT テスト名・公開 item 名に `rt_` 部分文字列を含めない（`assert_`/`convert_`/`support_`/`start_` 等を避ける命名規則を T3 タスクに明記） |
| 6 | RGB→YUV の係数決定源が spec 未規定 | 中 | 中 | Q2: 既定=カーネル構築引数で指定。実装時に spec 文言との整合を doc で明確化 |
| 7 | f16 正規化・丸め規則の曖昧さ | 中 | 低 | Q3: 既定=[0,1] 正規化 + 丸め規則・ulp 上限をカーネル doc に明記 |
| 8 | chroma siting 前提ずれ | 中 | 中 | 水平 co-sited・垂直 center の前提を doc 明記（spec 規定の既知リスク。siting 相違は半画素ずれ） |
| 9 | golden vector の誤り | 中 | 中 | BT 規格係数から導出、matrix×range 全組合せをカバー、往復・連鎖テストで交差検証 |

## Task Breakdown

| Task | 内容 | Phase |
| --- | --- | --- |
| [document-video-core-ownership-contract.md](../tasks/document-video-core-ownership-contract.md) | video-core の所有権・drop・zero-copy 契約と `**RT contract:**` ラベルの文書化（AC1–AC4） | A |
| [create-video-proc-crate-skeleton.md](../tasks/create-video-proc-crate-skeleton.md) | `ruviform-video-proc` 新設（members 配線・crate doc・GPU ポート所有宣言）（AC5・AC10） | B |
| [implement-yuv-to-rgb-kernel.md](../tasks/implement-yuv-to-rgb-kernel.md) | カーネル二相 API + `Yuv*p → Rgba8`（matrix/range・chroma upsample・エッジケース）（AC6・AC7 前半・AC8・AC9） | C |
| [implement-rgb-to-yuv-and-f16-kernels.md](../tasks/implement-rgb-to-yuv-and-f16-kernels.md) | `Rgba8 → Yuv*p` + `Rgba8 ↔ RgbaF16`（AC7 後半・AC8・AC9） | D |
| [sync-docs-issue-26.md](../tasks/sync-docs-issue-26.md) | baseline 採否表・AGENTS・README 同期 + 総合検証（AC11） | E |

## Verification

| ID | 対象 | 検証コマンド / 確認 | Pass condition |
| --- | --- | --- | --- |
| V-1 | T1（video-core doc） | `mise exec -- cargo test -p ruviform-video-core`・`mise exec -- cargo clippy -p ruviform-video-core --all-targets -- -D warnings`・`mise exec -- cargo doc -p ruviform-video-core --no-deps`・`grep -n "RT contract" crates/ruviform-video-core/src/*.rs` で借用/共有/確保系 API のラベル網羅 | exit 0・0 warnings・doc 生成成功・AC1–AC4 の記載確認 |
| V-2 | T2（骨格） | `mise run check`・`mise exec -- cargo metadata --format-version 1 --no-deps`（members に video-proc・deps=video-core のみ・全パッケージ `publish` が `[]`）・`mise exec -- cargo test -p ruviform-video-proc` | exit 0・配線が D6-a どおり・smoke test 合格 |
| V-3 | T3a（YUV→RGB） | `mise exec -- cargo test -p ruviform-video-proc`（golden vector・エッジケース含む）・`mise exec -- cargo test -p ruviform-video-proc rt_`（`rt_` テストのみ選択）・`mise exec -- cargo clippy -p ruviform-video-proc --all-targets -- -D warnings` | 全テスト合格・意図した `rt_` 集合のみ・0 warnings |
| V-4 | T3b（RGB→YUV + f16） | V-3 と同一コマンド群 | 同上 |
| V-5 | T4・全体 | `mise run fmt-check`・`mise run check`・`mise run lint`・`mise run test`・`mise run test-rt`・`mise exec -- cargo metadata --format-version 1 --no-deps`（依存辺: video-proc → video-core・media-core のみ）・`mise exec -- prek run --all-files`・`node .agents/skills/doc-status/scripts/audit_docs.js --type all` | 全 PASS・依存辺が D6-a どおり・0 findings |

振る舞いレベルの保証（共有 backing の書込ゲート・drop=dealloc・変換規則・HDR 非破壊・alloc-free 契約・`rt_` レーンの選択性）は [test spec](../test-specs/video-foundation-api.md) に委譲し、ここでは step ごとの検証入口のみを管理する。

## Rollback Strategy

| 対象 | Trigger | Procedure | Verification |
| --- | --- | --- | --- |
| 本変更全体 | 契約誤記・カーネル API 形状の根本誤りが実装後に判明 | feature PR の merge commit を `git revert -m 1`（または対象 commit を `git revert`） | revert 後に `mise run check`・`mise run test` がグリーン |
| video-proc 部分（T2–T3b） | カーネル API のやり直し | `crates/ruviform-video-proc/` を削除し `members`・`Cargo.lock`・AGENTS/README の当該行を戻す | `cargo metadata` でメンバー解消・残クレートの check/test がグリーン |
| video-core doc 部分（T1） | 契約文の誤り | 対象ファイルの doc 変更のみ revert（doc-only でありコード挙動不変。introspection API を追加した場合はその API ごと除去） | `cargo test -p ruviform-video-core`（doctest 含む）グリーン |
| baseline 採否表（T4） | 決定の再審議 | 該当行の編集を revert | `audit_docs.js --type all` が 0 findings |

- 不可逆性は低い: 公開 API 変更は additive のみ（introspection を追加した場合）。`publish = false` で外部利用者不在。マイグレーション・データ変換・破壊的なフォーマット変更はなし。

## Open Questions / Gaps

- **Q1 introspection API（T1）**: AC1 の「共有状態が観測できる」は `plane_mut()==None` を観測点として doc 明記する方針を既定とし、API 追加は行わない。契約文書化がそれを要求する場合のみ `is_shared()`/`is_exclusively_owned()` 相当の最小追加を許容し、追加時は根拠を記録する（spec In Scope・ADR D5-i の条件付き許容）。
- **Q2 RGB→YUV の係数決定源（T3b）**: spec は「入力フレームの `ColorInfo.matrix` を参照」とあるが、RGB 入力の `matrix` は `Identity`/`Unspecified` が自然であり、入力参照だと Identity 規則により変換不能になり得る。既定: `RgbToYuv` の適用係数はカーネル構築引数（`prepare`/コンストラクタの setup）で指定し、引数側にも既定規則（`Unspecified`→Bt601/Limited、`Identity`/未知 variant→構築時 `Err`）を適用する。実装時に spec 文言との整合を doc で明確化する。
- **Q3 `Rgba8 ↔ RgbaF16` の値規則（T3b）**: spec は「チャンネル値の数値変換のみ」。既定: u8→f16 は `v/255` による [0,1] 正規化、f16→u8 は逆写像 + 丸め規則（要 doc 明記）を採用する。等価性契約は浮動小数点を含むため ulp 上限（具体値をカーネル doc に明記）。（実績注記: 値規則は既定どおり採用。ただし primitive `f16` は windows-msvc で unstable のため、内部表現はソフトウェア binary16 コーデック — D-2 注記参照）
- **Q4 `process` の dst 時刻伝播（T3a/T3b）**: spec 未規定。既定: `process` は `dst` の `pts`/`duration` を `src` から伝播する（色変換は時刻を変えない）をカーネル doc に明記する。
- **Q5 Issue #26 コメント本文**: `gh` CLI 不在環境のため最終確認未実施（discovery Open Question どおり）。AC 認識は #41 トリアージ対応表由来の二次情報 — 実装着手前に `gh issue view 26 --comments` での確認を推奨。
- `Yuv444p.chroma_subsampling() == Some((1,1))` の統一解釈は #18 由来の open question — 変換実装で自然確定する見込み。

## Self-Review

- [x] spec の Requirements・AC1–AC11 が Phase A–E・V-1〜V-5・5 タスクに対応している
- [x] ADR の D1–D7・baseline の RT/非 RT 分離・DSP/SIMD 決定・enum ADR・RT ADR が実装・検証計画に反映されている
- [x] TBD / TODO /「後で実装」のプレースホルダは残っていない（据え置きは spec Deferred Design Concerns と Open Questions / Gaps に記録）
- [x] ファイルパス・relation・status・用語が spec/ADR/タスクと整合している

## Implementation Handoff

- Recommended mode: delegated or subagent-capable implementation
- Independent work streams: T1（video-core doc のみ）と T2（新規ディレクトリのみ）はファイル競合なしで並行可能。T3a→T3b は video-proc 内の共有ファイル（`coef.rs`・`lib.rs`）のため直列。T4 は全実装完了後の直列フェーズ
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 明示的な仮定: Q1–Q4 の既定（introspection 追加なし・RGB→YUV 係数は構築引数・f16 は [0,1] 正規化・dst 時刻伝播）は spec の明示文言のない判断点であり、実装時に doc で確定する。レビューでは spec/ADR との齟齬を重点確認されたい。
- 未解決の依存: Q5 の `gh` による Issue #26 コメント確認（実装着手前の推奨事項。AC 認識の一次確認）。
- 上流参照: spec `SPEC-034JcicHlRidBgcK1wPtj2`・ADR `ADR-034JcidaiOOip5EhNPOnDI`・baseline `DESIGN-034IPynWe2V30FLubzQenb`・RT ADR・enum ADR `ADR-034J6OnZouPI8HQy6cIg9D`・discovery `DISC-034JcicdqiAQzbUKFHiCbc`・Issue #26/#41/#1。
