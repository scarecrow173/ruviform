---
id: "IMPL-034LMyg7k2P0ar6eR0gedo"
type: "impl"
status: "completed"
title: "Issue #99 RgbaF32 composite/affine arms — Task 5 implementation"
created: "2026-10-25"
updated: "2026-10-25"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-99-rgba-f32-arms-task-5-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-video-proc/src/composite.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/affine.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/README.md"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "docs/tasks/rgba-f32-composite-affine-arms.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/plans/implement-issue-99-color-conversion-kernels.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/rgba-f32-composite-affine-arms.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #99 RgbaF32 composite/affine arms — Task 5 implementation

## Summary

Issue #99 Task 5 完了 — actions 12–14（既存カーネルの `RgbaF32`
arm + gate + doc 整合）を全範囲実施。`composite.rs`（+約 620 行）
に `CompositeOver`/`Crossfade` の `RgbaF32` arm — `RgbaF16` パス
と同式を無 clamp の unclamped f32 ドメインで評価し binary16 丸め
なしで `binary32` へ書出し（`oa < 2⁻¹²⁶` の ill-conditioned 除算
領域は f16 `2⁻¹⁴` cut の binary32 対応物として定義済み
transparent black — straight alpha・NaN/±∞ 伝播・≤ 2 ulp
`binary32` 等価性契約）。`affine.rs`（+約 220 行）は既存の
`sample_bytes()` 非依存複写経路で `RgbaF32` を受理 — 写像/複写
のみのため NaN payload・`−0.0`・subnormal を含め bit-exact。
独立レビュー 2 件: 両者 APPROVE・P0/P1 なし — `oa <
MIN_POSITIVE` の domain-relative 解釈を両 reviewer が正当と
裁定。指摘修正: stale "both" 文言 → "every"・divergence-pin
テスト追加・task 文書 typo "≤ 1 ulp" → "≤ 2 ulp" 訂正・
`q8_weight` doc 文言。5 gate 全 green 実 log。#97 の linear-light
合成 deferred を閉じる。

## Context

approved plan `docs/plans/implement-issue-99-color-conversion-kernels.md`
の Task 5（`docs/tasks/rgba-f32-composite-affine-arms.md`・actions
12–14）。upstream: Task 1 IR
（`docs/impl/ir/issue-99-rgba-f32-transfer-task-1-implementation.md`
— `PixelFormat::RgbaF32`）・Task 2 IR
（`docs/impl/ir/issue-99-working-space-kernels-task-2-implementation.md`
— working space `RgbaF32` linear `{Bt2020, Linear, Identity, Full}`
フレームの供給経路）・Task 3 IR
（`docs/impl/ir/issue-99-tone-map-sdr-task-3-implementation.md` —
crate README kernel 一覧 stale の nit を defer 記録・本 task の
A14 が回収）・Task 4 IR
（`docs/impl/ir/issue-99-icc-transform-task-4-implementation.md`）。
approved spec/design/ADR（color-conversion-kernels 系・
composite-transition-kernels・broad-color-management）・TSpec
`color-working-space-kernel-tests.md`（proposed —
`RgbaF32` arm の受理・等価 bound・拒否群・metadata・
`rt_process_*` 契約）。design（broad-color-management D2-b 系の
確定事項）が `#97` で deferred した linear-light 合成を本 task が
閉じる — `RgbaF32` working-space フレーム上の composite は
符号空間演算のまま linear-light 合成となる。後続 task（
color-conversion-final-verification・color-conversion-closeout）は
未着手。

## Implementation

### Changes

- `crates/ruviform-video-proc/src/composite.rs`（+約 620 行）:
  - `CompositeOver::process` — `PixelFormat::RgbaF32` を受理集合に
    追加（`src`/`dst` 同一 format 規則・Q8 重み量子化 `w =
    round(opacity·255)` ・`w == 0` early return・共有 `dst` 拒否・
    全検証書込み前は既存規約どおり不変）。画素経路は
    `composite_over_f32` — `RgbaF16` と同式（`sa = a_s·(w/255)`・
    `b = a_b·(1−sa)`・`oa = sa + b`・`oc = (c_s·sa + c_b·b)/oa`）
    を `f32` で評価し `binary32` へ書出し — binary16 丸めも
    [0,1] clamp もない（HDR 1.0 超・負の広色域値を保持 —
    linear working-space arm の目的）。`oa < F32_MIN_NORMAL`
    （`f32::MIN_POSITIVE` = `2⁻¹²⁶` — binary32 の最小 normal）
    は zero/subnormal/負の `oa` を覆う defined transparent black
    — f16 の `oa < 2⁻¹⁴` cut の binary32 対応物（除算が
    ill-conditioned となる storage domain の subnormal 領域を
    契約から切離す規則の domain-relative 適用）。NaN は比較を
    fail して式経路へ進み `f32` セマンティクスどおり伝播。
  - `Crossfade::process` — 同じく `RgbaF32` を受理集合に追加し
    `crossfade_f32`（`oc = a_c·(1−t) + b_c·t`・全 4 チャネル・
    binary16 丸めなし）へ dispatch — 量子化されるのは呼出し毎
    `tq` のみ。`tq = 0`/`tq = 255` の端点再現は `RgbaF16` と同
    規則で数値的（`f32` add が `−0.0` を `+0.0` へ canonicalize
    し得る・NaN チャネルは payload 正規化され得る）— module
    doc に明記。
  - helper — `F32_MIN_NORMAL` const・`f32_channel`（LE `binary32`
    quad → `f32` の bit-exact read）・`f32_bytes`（`f32` → LE
    `binary32` quad の bit-exact write — `RgbaF32` 経路は
    binary16 丸めを経由しない）・`composite_over_f32`・
    `crossfade_f32`（両者とも normative 形を逐語維持するため
    `#[allow(clippy::suboptimal_flops)]` — fused `mul_add` は
    規範パイプラインの丸めを変える）。
  - doc — Formats（受理 format 3 種へ更新・`RgbaF32` 画素は LE
    `binary32` ×4 同順序）・Value rules（`RgbaF32`
    working-space フレームの合成が #97 deferred の
    linear-light 合成そのものである旨を明記 — kernel 自体は
    符号空間演算のまま）・`CompositeOver`/`Crossfade` 各規則節
    の `RgbaF32` 項・等価性契約節（`RgbaF32` 経路は f64 参照式
    の `binary32` 丸めに対しチャネル毎 ≤ 2 ulp（`binary32`
    ドメイン）・`oa < 2⁻¹²⁶` 領域と非有限値は契約外 —
    `RgbaF16` の ≤ 2 ulp `binary16` と並記）・`# Errors` の受理
    format 記述。
  - inline tests — 新規 18 tests（unit 14 + `rt_process_*` 4）:
    `CompositeOver` は opacity = 1 golden・mid-opacity ≤ 2 ulp
    bound（f64 参照式）・`oa` 範囲外（zero/subnormal/負）の
    transparent black・f16 cut との divergence pin（下記
    review 項）・非有限伝播・非 clamp 保持・`opacity = 0`
    byte-identical + 共有 `dst` 先行拒否・format/dimension
    不一致拒否（`dst` 不変断言付き）・metadata 伝播。
    `Crossfade` は `tq = 128` ≤ 2 ulp golden・端点再現
    （`−0.0` canonicalize・NaN payload）・非有限伝播・format
    不一致拒否・`from` metadata 伝播。`rt_process_*` 4 件は
    repeatable/deterministic・src 非破壊（`CompositeOver` は
    in-place のため反復毎に `dst` を復元）。
- `crates/ruviform-video-proc/src/affine.rs`（+約 220 行）:
  - `AffineTransform::process` — `PixelFormat::RgbaF32` を受理集合
    に追加。画素演算を持たない写像/複写カーネルのため新規
    pixel path は不要 — 既存の `sample_bytes()` サイズ駆動
    複写（16 B/px）が `RgbaF32` をそのまま扱う — opaque byte
    複写のため NaN payload・`−0.0`・subnormal を含め
    bit-exact。`map.dst` の canvas 外・未カバー画素は transparent
    black（zero byte = `+0.0` `binary32` チャネル）。
  - doc — Formats（受理 format 3 種・全 format が単一
    `sample_bytes` 経路を共有）・等価性契約（「両 format」→
    「全受理 format bit-exact」）・`# Errors` の受理 format 記述・
    bytes-per-pixel コメント（4/8/16）。
  - inline tests — 新規 6 tests（unit 4 + `rt_process_*` 2）:
    2→4 upscale の bit-exact 複写（NaN payload・`−0.0`・
    subnormal 含む任意 `binary32` pattern）・off-canvas /
    全画素 off-canvas の transparent black 充填・format
    不一致 + 共有 `dst` 拒否（`dst` 不変断言付き）・metadata
    伝播・RT 契約 2 件（repeatable/deterministic・src 非破壊）。
- `crates/ruviform-video-proc/src/lib.rs`: kernel 一覧 bullet を
  実装状況へ整合 — `CompositeOver` に `RgbaF32`（binary16 丸め
  なしの unrounded `f32` パイプライン — linear working-space
  arm）を追記・`Crossfade` の量子化/等価規則が `CompositeOver`
  と同規則である旨・`AffineTransform` の「両 format」表現を
  「every accepted format」へ。
- `crates/ruviform-video-proc/README.md`: Issue #99 の kernel
  section を新設 — `ToWorking`/`FromWorking`/`ToneMapSdr`/
  `IccTransform`+`IccIntent`（feature `icc`・検証 lane 記載）の
  受理 format・等価契約を一覧化し、composite/遷移 kernel 節の
  受理 format を `{Rgba8, RgbaF16, RgbaF32}`（#99 で追加 —
  linear-light 合成 deferred を閉じる arm）へ更新・`RgbaF32`
  の量子化/等価規則を追記・references に
  color-conversion-kernels design と Issue #99 行を追加（Task
  2–4 で stale だった kernel 一覧を A14 の doc 整合で回収）。
- `crates/README.md`: `ruviform-video-proc` 索引行を実装状況へ
  更新（色変換・`RgbaF32` linear working space 色管理・
  コンポジット/遷移・feature `icc` で ICC 変換）。
- `docs/tasks/rgba-f32-composite-affine-arms.md`: A12 節の等価性
  契約記述に typo を発見し訂正 — `RgbaF16` の等価性契約は
  design・module doc 共に ≤ 2 ulp `binary16` だが task 文書には
  「≤ 1 ulp binary16」と誤記されていた（実装・検証の正本は
  design/契約側で不変 — 文書側の修正のみ）。

### Decisions During Implementation

- **`oa < F32_MIN_NORMAL`（`2⁻¹²⁶`）を cut に採用**（deviation・
  deliberate — 両 reviewer が正当と裁定）— composite design
  の f16 規約は `oa < 2⁻¹⁴`（binary16 最小 normal）を
  ill-conditioned 除算領域として契約から切離し定義済み
  `(0,0,0,0)` とする。`RgbaF32` arm が同じ cut を literal `2⁻¹⁴`
  で引くか、b32 の最小 normal `2⁻¹²⁶` で引くかは design/task の
  未規定点 — cut の理由が「storage domain の subnormal 領域で
  除算が ill-conditioned」という domain-relative な根拠である
  ため、`binary32` では `f32::MIN_POSITIVE` が対応する閾値と
  判定（literal `2⁻¹⁴` なら normal `binary32` の `a_s = 2⁻²⁰`
  等まで f16 と同様に blank してしまい、f32 の表現域を縮める
  不正な読みとなる）。divergence window `[2⁻¹²⁶, 2⁻¹⁴)` — f16
  では blank・f32 では normal 合成 — を pin する
  `composite_over_f32_faint_alpha_diverges_from_f16_cut` を追加
  して解釈を固定（review 修正）。
- **crate README の更新範囲を Issue #99 kernel 全体に拡大**
  （deviation・追加）— A14 は「カーネル一覧・受理 format 表・
  feature `icc` 記述の実装状況一致確認」。確認の結果、Task 2–4
  の kernel（`ToWorking`/`FromWorking`/`ToneMapSdr`/
  `IccTransform`）が README 未記載のままだった（Task 3 IR で
  defer 記録済みの stale nit）ため、arm 文言だけの最小修正では
  doc 整合が成立しない — Issue #99 kernel section を新設して
  一覧・format 表・feature 記述を一括で実装状況に一致させた
  （coherent change — 分離すると本 task の完了条件を満たさない）。
- **task 文書の ≤ 1 ulp typo を訂正**（deviation — 文書側修正）—
  A12 の記述が `RgbaF16` 契約を「≤ 1 ulp binary16」と誤記（正
  本は composite design・module doc 共に ≤ 2 ulp）。実装は正本
  側に忠実なためコード変更は不要で、文書 typo のみ訂正した。
- **affine `RgbaF32` arm は format gate 追加のみ** — 画素演算を
  持たない写像/複写カーネルで `sample_bytes()` が 16 B/px を
  返すため、新規 helper・pixel path は設けず受理集合と doc のみ
  拡張（YAGNI — bit-exact 契約は複写経路の構造そのもの）。
- **composite f32 helper は f16 helper と並置** — `q8_weight`/
  `q8_scale` は既存のまま共有し、channel codec は `f32_channel`/
  `f32_bytes` を mod ローカルに新設（`binary16` codec は
  `rgba_f16.rs` 経由だが `binary32` は codec クレート不要の
  bit-exact LE read/write — mod 間 `pub(crate)` helper を増やさ
  ない依存境界規約どおり）。

### Independent reviews（APPROVE ×2 — P0/P1 なし）

実装後に独立レビュー 2 件を実施 — 両 reviewer 共 APPROVE、
P0/P1 finding はゼロ。解釈争点の裁定と適用済み修正:

- **threshold 解釈（adjudicated 正当）** — `oa <
  f32::MIN_POSITIVE` が f16 `2⁻¹⁴` cut の正しい binary32 対応物
  であることを両 reviewer が確認。cut は storage-domain-relative
  （出力 alpha が storage domain で normal として表せず除算が
  ill-conditioned になる領域を契約外にする）— design の
  domain-relative 根拠どおりの読みと裁定され、literal `2⁻¹⁴`
  踏襲の代替解釈は却下。
- **適用済み修正（review 指摘）**:
  - stale "both" 文言の掃討 — 受理 format が 3 種になったため
    "both pixel paths" → "every pixel path"（composite module
    doc・`q8_weight` doc・`CompositeOver`/`Crossfade` の per-call
    量子化コメント）、"both formats" → "every/all three"（affine
    module doc・equivalence 契約・off-span コメント・`lib.rs`
    bullet・README）へ整合。
  - divergence-pin テスト
    `composite_over_f32_faint_alpha_diverges_from_f16_cut` を新規
    追加 — `a_s = 2⁻²⁰`（binary16 subnormal = `0x0010` だが
    normal `binary32` = `0x3580_0000`）で f16 経路は定義済み
    transparent black・f32 経路は通常合成（`oc = c_s`・
    `oa = 2⁻²⁰`）となる分岐を固定し、domain-relative cut の
    解釈を regress から保護。
  - task 文書 typo「≤ 1 ulp」→「≤ 2 ulp」訂正（上述）。
  - `q8_weight` doc の文言整合（"both pixel paths" → "every
    pixel path" — 上記掃討の一環として記録）。

## Related Experiments

### Adopted

なし — approved design/plan の確定実装のみ（実験ログなし）。

### Rejected

なし — threshold 解釈の代替（literal `2⁻¹⁴`）は review 裁定で
却下（experiment ではなく adjudication 記録）。

## Validation

- gate（実 log・全 exit 0）:
  - `mise run check` — exit 0（`cargo check --workspace
    --all-targets` — feature-off ベース）
  - `mise run lint` — exit 0（`cargo clippy --workspace
    --all-targets -- -D warnings` clean）
  - `mise run test` — exit 0（`cargo test --workspace` —
    ruviform-video-proc lib tests **171/171 pass**（composite.rs
    新規 18 + affine.rs 新規 6 を含む）・doctests **21/21
    pass**）
  - `mise run test-rt` — exit 0（`rt_` 名前フィルタで workspace
    **107 件** 実選択 — ruviform-video-proc では **27 件** が
    一致・新規 `rt_process_*_f32_*` 6 件を含む）
  - `mise run fmt-check` — exit 0
- 構造確認（実コマンド）:
  - `rg -n "RgbaF32" crates/ruviform-video-proc/src/composite.rs
    crates/ruviform-video-proc/src/affine.rs` — 両モジュールの
    受理 match arm・画素経路 dispatch・doc・tests に `RgbaF32`
    arm が存在
  - `rg -n "RgbaF32" crates/ruviform-video-proc/README.md
    crates/README.md` — kernel 一覧・索引行が実装状況と一致
  - `rg -n "F32_MIN_NORMAL" crates/ruviform-video-proc/src/composite.rs`
    — `f32::MIN_POSITIVE` 定義 + `oa < F32_MIN_NORMAL` cut が
    存在
- 注: `assert_ulp`（composite tests の `binary32` ulp 計測）は
  Task 3 と同じ sign-magnitude bit 距離 — 同一符号比較のみに
  有効（現テストは全て同一符号 — 既知の nit として継続記録）。

## Non-Goals

- `Rgba8`/`RgbaF16` 既存 arm の挙動変更 — 受理集合拡張のみ
  （Q8 量子化・`w == 0`/`tq` 規則・メタデータ規約・`_` arm の
  graceful 拒否は不変 — 既存テスト全件が無退行証拠）。
- premultiplied alpha・planar YUV 上の composite・`RgbaF16` dst
  その他の format 拡張 — plan non-goals どおり。
- `assert_no_alloc` による alloc 機械検出 — design で後続 Issue
  へ defer 済み（`rt_process_*` は repeatable/deterministic・src
  非破壊の観測検証 — #97/#98 と同一レーン）。
- `Processor` trait 実装・`RtContext` 引数・engine/OFX 配線 —
  plan non-goals どおり。
- feature `icc` 関連の追加作業 — Task 4 で完了済み（本 task の
  gate は feature-off ベース — feature lane は Task 6 Full Gate
  で再検証）。
- test-spec `color-working-space-kernel-tests.md` の status 更新
  （proposed 維持 — Task 7 closeout の責務）。

## Risks

- `oa ∈ [2⁻¹²⁶, 2⁻¹⁴)` の faint alpha 領域は f16/f32 で定義済み
  出力が分岐する（f16 blank・f32 通常合成）— domain-relative
  cut の意図的な帰結として divergence-pin test で固定済みだが、
  将来 f16↔f32 を cross-validate する比較器はこの window を
  契約外として扱う必要がある。
- `assert_ulp` の sign-magnitude ulp 計測は同一符号比較限定 —
  cross-zero の bound 比較を将来追加する場合は計測法の見直しが
  必要（Task 3 IR と同じ nit・記録継続）。
- `Crossfade` f32 端点再現は数値的（`−0.0` → `+0.0`
  canonicalize・NaN payload 正規化の余地）— 端点 bit-exact を
  期待する consumer は doc 記述を参照（`Rgba8` のみ bit-exact
  再現）。
- composite/affine の `RgbaF32` 経路は channel 値を解釈しない
  符号空間演算 — working-space tag の正当性（`{Bt2020, Linear,
  Identity, Full}`）の保証は consumer 責務（mistagged 入力も
  pixel-identical に処理 — composite 規約どおり doc 明記済み）。

## Follow-ups

- Task 6 `docs/tasks/color-conversion-final-verification.md`
  （action 15）— Full Gate 一回（`fmt-check`・`check`・`lint`・
  `test`・`test-rt`・`test-video-proc-icc`・`lint-video-proc-icc`・
  `bench-smoke`・`prek run --all-files`・doc audit — `test-rt`
  実一致件数で証明）。
- Task 7 `docs/tasks/color-conversion-closeout.md`（action 16）—
  実績 IR・coverage・task/plan status・authoritative `gh` merge。
- test-spec 2 件の proposed → verified 系 status 遷移 — Task 7
  closeout の責務（本 task は contract 実装を pass するのみで
  TSpec 文書自体は unchanged）。
