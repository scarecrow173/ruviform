---
id: "PLAN-034LFFj86hwRvUljHN54Mc"
type: "plan"
status: "completed"
title: "Issue #99 実施計画 — video-proc 色変換カーネル（linear working space + ICC 入出力経路）"
created: "2026-10-25"
updated: "2026-10-25"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/99"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/color-conversion-kernels.md"
  implemented-by:
    - "docs/tasks/rgba-f32-format-and-transfer-internals.md"
    - "docs/tasks/working-space-kernels.md"
    - "docs/tasks/tone-map-sdr-kernel.md"
    - "docs/tasks/icc-transform-feature.md"
    - "docs/tasks/rgba-f32-composite-affine-arms.md"
    - "docs/tasks/color-conversion-final-verification.md"
    - "docs/tasks/color-conversion-closeout.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-issue-97-composite-kernels.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/color-conversion-kernels.md"
    - "docs/designs/color-conversion-kernels.md"
    - "docs/adr/broad-color-management.md"
    - "docs/adr/video-proc-kernel-api-and-color-hdr-scope.md"
  derived-by:
    - "docs/tasks/rgba-f32-format-and-transfer-internals.md"
    - "docs/tasks/working-space-kernels.md"
    - "docs/tasks/tone-map-sdr-kernel.md"
    - "docs/tasks/icc-transform-feature.md"
    - "docs/tasks/rgba-f32-composite-affine-arms.md"
    - "docs/tasks/color-conversion-final-verification.md"
    - "docs/tasks/color-conversion-closeout.md"
  verifies: []
  verified-by:
    - "docs/test-specs/color-working-space-kernel-tests.md"
    - "docs/test-specs/icc-transform-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #99 実施計画 — video-proc 色変換カーネル（linear working space + ICC 入出力経路）

> status: completed — 全 7 task 完了。Task 1 (rgba-f32-format-and-transfer-internals) 完了済み（commit `c88860e`・actions 1–3・narrow gate 3 コマンド green・lead review F1–F5 修正済み）。Task 2 (working-space-kernels) 完了（commit `6d27460`・actions 4–6・`mise run check`/`lint`/`test`/`test-rt` 全 exit 0・独立 review 2 件 P0/P1 なし・指摘全修正済み）。Task 3 (tone-map-sdr-kernel) 完了（commit `d73b8b9`・actions 7–8・`mise run check`/`lint`/`test`/`test-rt` 全 exit 0・独立 review 2 件 APPROVE・P0/P1/P2 なし）。Task 4 (icc-transform-feature) 完了（commit `c5aa5a6`・actions 9–11・feature `icc` + `IccTransform`/`IccIntent` + `test-video-proc-icc`/`lint-video-proc-icc` lane + CI `icc-verify`・`fmt-check`/`check`/`lint`/`test`/`test-video-proc-icc`/`lint-video-proc-icc`/`test-rt` 全 exit 0・独立 review 2 件 — reviewer A APPROVE・reviewer B の P1 ×2 修正済み・P2 ×2 は follow-up 記録）。Task 5 (rgba-f32-composite-affine-arms) 完了（commit `878d602`・actions 12–14・`composite.rs`/`affine.rs` の `RgbaF32` arm + crate README/`crates/README.md`/`lib.rs` doc 整合・`check`/`lint`/`test`/`test-rt`/`fmt-check` 全 exit 0・独立 review 2 件 APPROVE・P0/P1 なし — `oa < f32::MIN_POSITIVE` の domain-relative cut を両 reviewer が正当と裁定・divergence-pin test で固定）。Task 6 (color-conversion-final-verification) 完了（commit `6693801`・action 15・Full Gate 10 コマンド全 exit 0・全 log `docs/impl/exp/data/issue-99-final-gate-*-20261025.log` 保存・`test` 49 binaries 932 passed 0 failed・`test-rt` 実一致 107 件 >0・`test-video-proc-icc` feature-on 211 pass・`lint-video-proc-icc` exit 0・`bench-smoke` exit 0・prek 7 pass 1 skip・doc audit blocking 0 findings []）。Task 7 (closeout) 完了（実績 IR・task/plan status 同期済み・両 test-spec を実結果で confirmed＝`approved` へ更新）。**GitHub merge は lead の authoritative `gh` のみ — 本計画は PR merged / issue closed の主張を含まない。** task/test-spec は起票済み（2 TSpec approved・7 task）。

## Goal

spec `docs/specs/color-conversion-kernels.md`・approved design `docs/designs/color-conversion-kernels.md` の正本どおり、`ruviform-video-proc` に f32 linear working space の色管理カーネル群を実装する: `PixelFormat::RgbaF32`（video-core 拡張）・`ToWorking`/`FromWorking`（working space 入口/出口・decode+gamut 融合パス）・`ToneMapSdr`（BT.2390-8 §5.4 EETF の HDR→SDR 輝度写像）・`IccTransform`+`IccIntent`（feature `icc` 配下の moxcms 0.9 経路）・既存 `CompositeOver`/`Crossfade`/`AffineTransform` への `RgbaF32` arm（#97 の linear-light deferred を閉じる）。

## Non-Goals

- OCIO 設定消費・ACES IDT/ODT/RRT フル管（ADR D2-b 保留・D3-b 後続）
- GPU 計算カーネル・バックエンド実装（ruviform-gpu Issue）・planar YUV への直接 gamut/transfer 適用（`Yuv*p` は `YuvToRgb`/`RgbToYuv` 経由の二段処理）
- ICC profile の供給経路（media-io/アプリ層責務）・`ColorInfo` → CICP profile 便利コンストラクタ・intent 拡張集合（Saturation/AbsoluteColorimetric）・黒点補償・CMYK/多チャネル layout・`in_place`/`options`/`any_to_any`/SIMD 系 moxcms feature
- `FromWorking` の `RgbaF16` dst・`RgbaF32 ↔ RgbaF16` shuffle・10/12-bit integer パイプライン・tone-map ターゲットのパラメータ化（`L_DS` = 100 nits 固定）・HLG `L_W` 設定化・stage 分割カーネル
- 既存 code-value カーネル（`YuvToRgb`/`RgbToYuv`/`Rgba8ToRgbaF16`/`RgbaF16ToRgba8`）の挙動変更 — `RgbaF32` は `_` arm で graceful 拒否のまま
- `Processor` trait 実装・`RtContext` 引数・フレームプール・`ruviform-edit`/engine/OFX への配線・`assert_no_alloc` の video-proc 導入（#97 と同一レーンの後続 Issue 管轄）
- moxcms バージョン/feature 更新時の golden 再導出手続きの自動化（pin + `Cargo.lock` で固定）

## Scope and Split Boundary

単一クリティカルパスの 1 計画（同一クレート内の順次カーネル追加 — 独立並行は主張しない）。**7 buildable task**（task 文書は起票済み・各 task が ordered substeps A1…An を持つ）。commit は論理ビルド可能単位で lead が管理 — 1 substep/1 commit 強制・red 専用 commit はしない。

- `RgbaF32` → `transfer.rs` 内部基盤（Task 1）が全カーネルの共有前提 — Task 2/3/5 は Task 1 に、Task 4 は Task 1 + Cargo feature 追加に依存
- Task 3（tone map）は Task 2 と独立（`tone_map.rs` は `transfer.rs` のみに依存）— 直列実行で依存解決
- Task 6（final verification）は Task 2–5 の全完了後に 1 回のみ — full gate の繰返し実行はしない
- テストは test-spec 2 件の契約を各モジュール inline `#[cfg(test)] mod tests` で実装 — golden/ulp-bound oracle は design の定義式を f64 で評価した参照に対して検証（厳密一致 golden ではなく bound 比較 — プラットフォーム `powf` 差を吸収）。`IccTransform` のみ pin 版 moxcms 出力への bit-exact

## Files / Ownership

| Path | Responsibility | Notes |
| --- | --- | --- |
| `crates/ruviform-video-core/src/pixel.rs` | `PixelFormat::RgbaF32` variant（`plane_count()==1`・`is_planar()==false`・`bytes_per_sample(0)==16`・`chroma_subsampling()==None`）+ 画素 layout doc + テスト行 | `#[non_exhaustive]` で non-breaking |
| `crates/ruviform-video-proc/src/transfer.rs`（新規・`pub(crate)`） | transfer decode/encode 関数群（Linear/Bt709/Smpte170m/Pq/Hlg + clamp 規則）・primaries → BT.2020 結合 3×3 f64 定数テーブル・`primaries_slot`/`transfer_slot` 解決関数・working-space 定数（`L_DS`・HLG nominal 1000 nits・`SDR_SCALE`） | 全カーネル共有の内部基盤 |
| `crates/ruviform-video-proc/src/working_space.rs` | `ToWorking`・`FromWorking` + 規約 doc + inline tests | |
| `crates/ruviform-video-proc/src/tone_map.rs` | `ToneMapSdr` + BT.2390 EETF + `L_HS`/`L_min` 導出 + inline tests | |
| `crates/ruviform-video-proc/src/icc.rs` | `IccTransform`・`IccIntent`（`#[cfg(feature = "icc")]` 配下・moxcms 閉じ込め）+ inline tests | feature-gated |
| `crates/ruviform-video-proc/src/composite.rs`・`src/affine.rs` | `RgbaF32` arm（受理集合・f32 評価パス・doc 等価性契約更新）+ tests | 既存規約不変 |
| `crates/ruviform-video-proc/src/lib.rs` | `mod`/`pub use`・カーネル一覧・feature `icc` 記述・色管理予約記述の実装状況への整合 | |
| `crates/ruviform-video-proc/Cargo.toml` | `moxcms` optional 依存 + `[features]` | design の正当化記録どおり |
| `crates/ruviform-video-proc/README.md`・`crates/README.md` | カーネル一覧・feature・許可依存・索引行の更新 | |
| `mise.toml` | `test-video-proc-icc`・`lint-video-proc-icc` タスク | 下記「Required mise tasks」 |
| `.github/workflows/ci.yml` | `icc-verify` ジョブ（ubuntu-latest・rt-verify と同形） | feature `icc` 列の CI 担保 |
| `Cargo.lock` | moxcms + `num-traits` + `pxfm` の追加 | 追跡継続 |

## Implementation Sequence（16 action — 将来 task 名のみ確定）

| # | Action | Scope |
| --- | --- | --- |
| 1 | `PixelFormat::RgbaF32` 追加（variant + 4 メソッド arm + layout doc + テスト行・`bytes_per_sample` doc に `RgbaF32` = 16 bytes 追記） | video-core |
| 2 | `transfer.rs` 新設 — decode/encode 関数群 + primaries→BT.2020 行列定数 + slot 解決 + 定数 + inline unit tests（decode/encode 往復・boundary・既定規則） | video-proc internals |
| 3 | narrow gate: `mise run check`/`lint`/`test` exit 0（既存カーネル無退行確認） | foundations |
| 4 | `working_space.rs` — `ToWorking`（`new()`+`Default`・`process(src, dst)`・3 src format・`Unspecified` 既定・`ColorInfo` 書換え）+ test-spec ケース | working-space |
| 5 | `FromWorking`（`new(dst_primaries, dst_transfer) -> Result<Self>`・ctor 焼付け・`Rgba8`/`RgbaF32` dst・RHU 量子化）+ test-spec ケース + lib.rs `mod`/`pub use`/一覧 | working-space |
| 6 | gate: `mise run test`/`lint`/`test-rt`（`rt_process_*` 選択確認） | working-space |
| 7 | `tone_map.rs` — `ToneMapSdr`（BT.2390 EETF・`L_HS`/`L_min` 導出・`hdr` 消費）+ test-spec ケース + lib.rs | tone-map |
| 8 | gate: `mise run test`/`lint`/`test-rt` | tone-map |
| 9 | `Cargo.toml` — `moxcms = { version = "0.9", optional = true, default-features = false, features = ["lut", "extended_range"] }` + `[features] icc = ["dep:moxcms"]`（`default = []` 維持）+ `Cargo.lock` 更新 | icc |
| 10 | `icc.rs` — `IccIntent` + `IccTransform`（ctor parse/executor 焼付け・per-pixel transform・alpha パススルー・`CmsError` 非露出）+ test-spec ケース + lib.rs feature-gated re-export/一覧/feature 記述 | icc |
| 11 | `mise.toml` に `test-video-proc-icc`/`lint-video-proc-icc`・`.github/workflows/ci.yml` に `icc-verify` ジョブ + gate: `mise run test-video-proc-icc`/`lint-video-proc-icc`・`mise run test`/`lint`（ベース経路無退行） | icc gate |
| 12 | `composite.rs` — `CompositeOver`/`Crossfade` の `RgbaF32` arm（f32 評価・binary16 丸めなし・≤ 2 ulp 契約）+ tests | arms |
| 13 | `affine.rs` — `AffineTransform` の `RgbaF32` arm（写像/複写のみ・bit-exact 維持）+ tests | arms |
| 14 | gate + doc 整合: `mise run test`/`lint`/`test-rt`・crate README・`crates/README.md` 索引・lib.rs 一覧の実装状況一致確認 | arms |
| 15 | **Full Gate（1 回のみ・本計画最後段）**: `mise run fmt-check`・`check`・`lint`・`test`・`test-rt`・`test-video-proc-icc`・`lint-video-proc-icc`・`bench-smoke`・`mise exec -- prek run --all-files`・doc audit — 全 log 保存・`test-rt` は実一致件数で証明（0 件一致スキップを成功と偽らない） | gate |
| 16 | canonical closeout: 実績 IR・coverage・task/plan status・GitHub merge は authoritative `gh` のみ（文書チェックリストで代行しない） | closeout |

- test-spec（起票済み・proposed）: [color-working-space-kernel-tests](../test-specs/color-working-space-kernel-tests.md)・[icc-transform-contract-tests](../test-specs/icc-transform-contract-tests.md) — `verified-by` は実ファイルに更新済み
- **7 task 文書**（起票済み・todo・actions 1-3/4-6/7-8/9-11/12-14/15/16）: [rgba-f32-format-and-transfer-internals](../tasks/rgba-f32-format-and-transfer-internals.md)・[working-space-kernels](../tasks/working-space-kernels.md)・[tone-map-sdr-kernel](../tasks/tone-map-sdr-kernel.md)・[icc-transform-feature](../tasks/icc-transform-feature.md)・[rgba-f32-composite-affine-arms](../tasks/rgba-f32-composite-affine-arms.md)・[color-conversion-final-verification](../tasks/color-conversion-final-verification.md)・[color-conversion-closeout](../tasks/color-conversion-closeout.md)
- 各 task 間は狭い検証のみ（full suite 繰返しはしない — Full Gate は action 15 の一回）

## Test Oracles（design/test-spec の正本どおり・新規代替アルゴリズム禁止）

- working space = `RgbaF32` + `{primaries: Bt2020, transfer: Linear, matrix: Identity, range: Full}` — `1.0` = 100 cd/m² 絶対輝度・`[0,1]` 制限なし（負値・1 超を lossless 保持）・alpha straight パススルー
- transfer decode: `Bt709` `v = 255` → 1.0（= 100 nits）・`Pq`/`Hlg` `v = 1.0` → 100.0/10.0（10000/1000 nits）・`Hlg` `v = 0.5` で `S = 0.5²/3` 分岐・`Pq`/`Hlg` 信号域外は clamp・`Bt709` toe `v < 0.081` → `v/4.5`（負入力も定義済み）
- `Unspecified` primaries/transfer → `Bt709` 既定（既存 `matrix → Bt601`・`range → Limited` の上位拡張規則）・未知 variant → `InvalidValue("color primaries")`/`"color transfer"`
- `FromWorking`: `Rgba8` 出力は `round(clamp(v,0,1)·255)`（RHU・境界 ±1 code）・ctor `Bt2020`/`Linear` で `RgbaF32 → Rgba8` の純粋量子化・`Pq`/`Hlg` encode は信号域 clamp・`Bt709`/`Linear` encode は非 clamp
- `ToneMapSdr`: `L_HS` = `mastering_display.max_luminance`（`/10_000` → nits・>0）→ `content_light_level.max_cll`（>0）→ 既定 1000 nits。`L_min` = `mastering_display.min_luminance`（`max_luminance` 未満のみ・不正順序は `b = 0`）。`L_HS ≤ L_DS`（100 nits）→ `ks ≥ 1` で全域 passthrough。`Y_src ≤ 0` は画素 passthrough（厳密一致）。`hdr` は出力で `None`
- `CompositeOver`/`Crossfade`/`AffineTransform` `RgbaF32` arm: `RgbaF16` パス同式の f32 評価（binary16 丸めなし）· ≤ 2 ulp — straight alpha・Q8 量子化・transparent black・メタデータ規約は不変
- `IccTransform`: pin 版 moxcms 0.9（`default-features=false` + `lut` + `extended_range`・SIMD 無効）のスカラー executor 出力に bit-exact — golden vector は同条件で導出。バージョン/feature 変更時は golden 再導出
- 共通: 全検証書込み前（共有 `dst` → `InvalidValue("dst plane")`）・部分書込みなし・`stride > row_bytes` パディング不変・`src` 非破壊・`pts`/`duration` 伝播

## Required mise tasks（lead 確定 — task `icc-transform-feature` の action で追加）

```toml
[tasks.test-video-proc-icc]
description = "Run video-proc tests with the icc feature enabled (moxcms ICC path)"
run = "cargo test -p ruviform-video-proc --features icc"
[tasks.lint-video-proc-icc]
description = "Lint video-proc and its test targets with the icc feature enabled"
run = "cargo clippy -p ruviform-video-proc --all-targets --features icc -- -D warnings"
```

- `.github/workflows/ci.yml` に `icc-verify` ジョブ（`runs-on: ubuntu-latest`・rt-verify と同形の mise セットアップ）で上記 2 タスクを実行 — `cargo test --workspace` は default feature のみのため icc 経路は別レーンで担保する（design が義務として記録）。`CARGO_INCREMENTAL=0`・rt-verify/bench と同じ ubuntu 単独運用
- moxcms 依存追加の正当化は design「Non-Functional Constraints」節の記録どおり（保守状況・ライセンス BSD-3-Clause OR Apache-2.0・依存木 3 crate・MSRV 1.89・`default-features = false` で moxcms 側 `#![forbid(unsafe_code)]`）— PR 説明にも理由を記録する（暫定規約「依存追加」）

## Risks / Readiness

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | moxcms の実効 MSRV/API 形状が宣言と異なる（`create_transform_f32`/`Layout::Rgba`/feature 名） | med | med | task `icc-transform-feature` の最初に docs.rs/cargo doc で実 API を確認 — 相違なら blocker 記録（fake success なし） |
| 2 | `RgbaF32` の `bytes_per_sample` 誤りで frame stride/確保が破損 | low | high | `plane_layout` 表駆動テストに行追加（16 B/px）+ video-core の既存テスト無退行 |
| 3 | EETF/transfer 式の実装誤り（toe 分岐・定数・clamp 境界） | med | high | f64 参照式との ulp-bound golden・端点 oracle を test-spec で固定 |
| 4 | feature `icc` 経路が default 検証対象外のまま regress | med | med | mise タスク + CI `icc-verify` ジョブで別レーンを構造保証（Full Gate に含める） |
| 5 | `IccTransform` の alloc-free が moxcms executor 依存で破る | med | high | per-pixel 1 要素スライス呼出し + 実装レビュー確認（機械検出は後続 Issue — #97 同一レーン） |
| 6 | 既存カーネル（composite/affine）の `RgbaF32` arm 追加で `Rgba8`/`RgbaF16` 経路が退行 | low | med | 既存テスト全件 + 新規 `RgbaF32` ケース — 受理集合拡張のみで既存 arm 不変 |

## Rollback

- 全変更は additive（新規ファイル/variant/arm/feature）— 履歴保持の corrective/revert commit で task 単位に戻す（reset/force なし）。`Cargo.lock` は `git checkout -- Cargo.lock` で戻し再生成
- `PixelFormat::RgbaF32` は `#[non_exhaustive]` enum への variant 追加 — revert しても downstream match の破壊にならない（内部 enum 拡張）
- feature `icc` は `default = []` の opt-in — moxcms 経路の revert は feature 削除で完結（ベース経路は無関係）
- publish は false・DB/persistent data migration なし

## Verification

- per-task narrow checks のみ — 各 task の `## Verification` が run 可能コマンドを持つ（`mise run test`/`lint`/`check`/`test-rt`・feature 経路は `test-video-proc-icc`/`lint-video-proc-icc` — 追加後）
- Full Gate は action 15 の一回のみ — 全 log 保存・`test-rt` 実一致件数で証明
- `ruviform-video-proc` の依存が `ruviform-video-core`・`ruviform-media-core`（+ feature `icc` 時の `moxcms`）のみ — `Cargo.toml` 確認（spec AC4）
- doc 監査チェーン起因 findings 0（`mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --json`）
- test-spec link は `verified-by` に更新済み — **実検証は未実施**（PLANNED・未実行）

## Open Questions / Gaps

- なし — design が discovery の Open Questions 全項目を確定済み（working space primaries・トーンマップ方式・ICC エンジン・f32 保持形・`Unspecified` 既定・カーネル集合・ulp bound）。Deferred Design Concerns は本計画の範囲外として確定済み

## Self-Review

- [x] Every spec requirement maps to a task（7 AC → 7 task + 2 test-spec の写像を確認済み）
- [x] No placeholders remain（ファイル名・mise task 名・依存指定は確定値のみ）
- [x] File paths, relations, status values, and terminology are consistent（`ToWorking`/`FromWorking`/`ToneMapSdr`/`IccTransform`/`IccIntent`/`RgbaF32`/`rt_process_*`・`InvalidValue` 文字列は design の正本どおり）

## Implementation Handoff

- Recommended mode: delegated 実行 + lead の task 完了ごとの実レビュー（ユーザ承認済みを前提 — 環境依存の skill ID は本計画に埋め込まず、実環境で発見される capability に routing する）
- Independent work streams: none — Task 3/5 は Task 1 のみに依存するが単一クリティカルパスで直列実行
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- レビュー焦点: `transfer.rs` の定義式（decode/encode・clamp 境界・`Unspecified` 既定）が design と逐語一致しているか — 等価性契約の基盤
- `ToneMapSdr` の `L_HS`/`L_min` 導出優先順位と `b = 0` 規則・`Y_src ≤ 0` passthrough が doc/実装一致しているか（spec AC5）
- `IccTransform` — moxcms 型が公開 API に露出していないか・`TransformOptions` 値が design どおりか・per-pixel 呼出しで行バッファ確保をしていないか（alloc-free の構造保証）
- `CompositeOver`/`Crossfade` の `RgbaF32` arm が `RgbaF16` パスと同式の f32 評価で binary16 丸めを適用していないか
- feature `icc` 無効時のベース経路が外部依存ゼロのままか（`cargo tree -p ruviform-video-proc` / Cargo.toml 確認）
