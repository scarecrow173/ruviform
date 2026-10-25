---
id: "IMPL-034LLGAQ0YeoT4FWlZ6YyD"
type: "impl"
status: "completed"
title: "Issue #99 IccTransform + IccIntent — Task 4 implementation"
created: "2026-10-25"
updated: "2026-10-25"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-video-proc/src/icc.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/tests/fixture/srgb.icc"
      - type: "file"
        path: "crates/ruviform-video-proc/tests/fixture/display_p3.icc"
      - type: "file"
        path: "crates/ruviform-video-proc/tests/fixture/gray_gamma22.icc"
      - type: "file"
        path: "docs/impl/ir/issue-99-icc-transform-task-4-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-video-proc/Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/ruviform-video-proc/src/lib.rs"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: ".gitattributes"
      - type: "file"
        path: "docs/test-specs/icc-transform-contract-tests.md"
      - type: "file"
        path: "docs/tasks/icc-transform-feature.md"
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
    - "docs/tasks/icc-transform-feature.md"
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

# Issue #99 IccTransform + IccIntent — Task 4 implementation

## Summary

Issue #99 Task 4 完了 — actions 9–11（feature `icc` 配線 +
`IccIntent`/`IccTransform` + feature 専用検証レーン）を全範囲実施。
`icc.rs`（新規・1057 行・`#[cfg(feature = "icc")]` 配下）に
`IccIntent`（`#[non_exhaustive]` 公開 enum —
`Perceptual`/`RelativeColorimetric`）と `IccTransform`（ctor で
profile parse → executor 焼付け・`Rgba8`/`RgbaF32` 任意端点
組合せの per-pixel 変換・alloc-free `process`）を実装 — moxcms
0.9.1 を optional dep として閉じ込め、公開 API にエンジン型・
エラー・feature 名を露出しない。独立レビュー 2 件: reviewer A
APPROVE（P2 ×2 を follow-up 記録）・reviewer B request-changes
→ 提示 P1 ×2 を修正済み（非 RGB color-space 受理の ctor ゲート・
host-independence 文言の訂正 + literal golden の `cfg` ゲート化）。
4 gate 全 green 実 log。

## Context

approved plan `docs/plans/implement-issue-99-color-conversion-kernels.md`
の Task 4（`docs/tasks/icc-transform-feature.md`・actions 9–11）。
upstream: Task 1 IR
（`docs/impl/ir/issue-99-rgba-f32-transfer-task-1-implementation.md` —
`PixelFormat::RgbaF32`）・Task 2 IR
（`docs/impl/ir/issue-99-working-space-kernels-task-2-implementation.md`）
・Task 3 IR
（`docs/impl/ir/issue-99-tone-map-sdr-task-3-implementation.md`）・
approved spec/design/ADR（color-conversion-kernels 系・
broad-color-management D2-a の ICC 経路確定）・TSpec
`icc-transform-contract-tests.md`（proposed — P1 修正で等価性契約
文言を同期）。`icc.rs` は `transfer.rs`/`working_space.rs`/
`tone_map.rs` と無関係の独立 mod（plan の依存グラフどおり Task 4
は Task 1 + Cargo feature 追加のみに依存）。後続 task
（rgba-f32-composite-affine-arms・final-verification・closeout）は
未着手。

## Implementation

### Changes

- `crates/ruviform-video-proc/src/icc.rs`（新規・1057 行・
  `lib.rs` で `#[cfg(feature = "icc")] mod icc` + feature-gated
  `pub use`）:
  - `IccIntent` — `#[non_exhaustive]` 公開 enum
    （`Perceptual`/`RelativeColorimetric`）— moxcms
    `RenderingIntent` への写像は crate-private の
    `rendering_intent()` に閉じ込め（match は網羅的 — 将来 variant
    追加時にコンパイルエラーで写像を強制決定）。
  - `IccTransform` — `executor: Arc<TransformF32Executor>` を保持。
    `new(src_profile, dst_profile, intent) -> Result<Self>`:
    `ColorProfile::new_from_slice` で両 profile を parse（失敗 →
    `InvalidValue("icc profile")`）→ **両端の
    `color_space == DataColorSpace::Rgb` を検証**（非 RGB は
    `InvalidValue("icc transform")` — reviewer B P1 修正。
    `Layout::Rgba` は RGB 空間 profile にのみ正直: moxcms は
    CMYK/Lab/多チャネルでも `Layout::Rgba` を受理するが channel 3
    が alpha passthrough でなくなり（CMYK では K インクとして
    消費・opaque alpha を出力）、executor 構築前に拒否）→
    `create_transform_f32(Layout::Rgba, …)` で executor を焼付け
    （失敗 → `"icc transform"`）。`TransformOptions` は契約固定:
    `rendering_intent` 写像・`allow_use_cicp_transfer: true`・
    `prefer_fixed_point: false`（f32 executor の参照精度を
    Q2.13 fixed-point に落とさない）・
    `allow_extended_range_rgb_xyz: true`（`extended_range` feature
    — matrix-shaper で [0,1] 外保持）・interpolation/barycentric は
    既定（`Linear`/`Low`）。`Debug` は `finish_non_exhaustive` —
    エンジン内部を漏らさない。
  - `process(src, dst)` — src/dst format ∈ {`Rgba8`, `RgbaF32`} 全
    端点組合せ（同一可）・同一寸法・共有 `dst` 拒否
    （`plane_mut(0)` → `None`）— 全検証を書込み前に完了
    （`"src format"`/`"dst format"`/`"dimensions"`/`"dst plane"`
    の `InvalidValue` 再利用）。per-pixel: 入力を `[f32; 4]`
    スタック quad に正規化（`Rgba8` は `v/255`・`RgbaF32` は raw
    little-endian `binary32`）→ 1 画素 1 呼出しで executor 実行
    （行バッファなし・alloc-free を構造保証）→ 出力エンコード
    （`RgbaF32` は raw `binary32` で extended range 保持・
    `Rgba8` は `round(clamp(v,0,1)·255)`）。alpha はエンジンの
    `Layout::Rgba` passthrough 経路で非変換。post-loop で
    `dst.set_color({primaries: Unspecified, transfer: Unspecified,
    matrix: Identity, range: Full, hdr: None})`（ICC profile の記述
    色空間は CICP enum に写像不可 — Unspecified が正直な記録）+
    `set_pts`/`set_duration` 伝播。`Plane::row`/`PlaneMut::row_mut`
    のみで padding を読み書きしない。
  - doc — module/type/method に変換規則・unspecified `ColorInfo`
    記録・等価性契約（pin 版 moxcms + feature 集合・同一 build
    構成の executor 出力に bit-exact — `mlaf` が aarch64 NEON /
    `target_feature = "fma"` x86/x86_64 で fused FMA を選択する
    ため target-feature/architecture 間の last-ulp 差を正直に明記
    — reviewer B P1 修正で "ホスト非依存" 文言を訂正）・alpha
    passthrough 規約・非原子性注記（executor の mid-frame 失敗は
    既書込み行を残す — 宣言済みだが baked RGBA layout では到達
    不能）・API 単位の `**RT contract:**` ラベル（`new` = 非 RT
    by convention・`process` = RT-suitable alloc-free）。公開 API
    例は doctest 3 件。
  - inline `#[cfg(test)] mod tests` — 16 tests（unit 14 +
    `rt_process_*` 2）+ `#[ignore]`d `generate_fixtures`（pin
    エンジンの built-in profile から `tests/fixture/` を再生成する
    明示実行 recipe）。literal assert を持つ 4 件
    （`srgb_identity_rgba8_bit_exact`・`rgba_f32_bit_exact_golden`
    の literal golden 2 件 + literal row assert を持つ padding
    系 2 件）は `#[cfg(all(target_arch = "x86_64",
    not(target_feature = "fma")))]` でゲートし、非ゲートの
    oracle/behavioral 経路（`rgba8_output_matches_engine` —
    同一 build 構成の raw executor 呼出しとの bit 比較・
    reviewer B P1 修正で追加した
    `icc_transform_rgba_f32_keeps_extended_range`）が全 target で
    検証を維持。他に ctor 拒否群（不正 profile・変換不能組合せ・
    非 RGB color-space — patched header で `DataColorSpace::Cmyk`
    に偽装した fixture を拒否）・端点組合せ・alpha passthrough・
    metadata/pts 伝播・intent 焼付け・拒否群（`dst` 不変断言
    付き）・padding 不変・RT 契約（repeatable/deterministic・
    src 非破壊）。
- `crates/ruviform-video-proc/tests/fixture/`（新規・3 ファイル）:
  `srgb.icc`（612 B）・`display_p3.icc`（598 B）・
  `gray_gamma22.icc`（312 B）— pin 版 moxcms の
  `ColorProfile::new_srgb`/`new_display_p3`/`new_gray_with_gamma(2.2)`
  を `encode()` した実 profile バイト列（`include_bytes!` で test
  が消費・`generate_fixtures` で再生成可能）。
- `crates/ruviform-video-proc/Cargo.toml`:
  `moxcms = { version = "0.9", optional = true,
  default-features = false, features = ["lut", "extended_range"] }`
  + `[features] icc = ["dep:moxcms"]`・`default = []` 維持。依存
  追加の正当化コメントを同箇所に記録（BSD-3-Clause OR
  Apache-2.0・pure-Rust 3-crate tree・MSRV 1.89・
  `default-features = false` で moxcms 側 `#![forbid(unsafe_code)]`
  が有効 — `in_place`/`options`/`any_to_any`/SIMD 系は有効化せず、
  特に `any_to_any` は選択される Katana executor が transform 毎に
  alloc するため alloc-free `process` 契約を破る — must-stay-off
  コメント明記）。
- `Cargo.lock`: `moxcms 0.9.1` + `pxfm 0.1.30` 追加（
  `ruviform-video-proc` deps に `moxcms` 反映・17 行）。`num-traits`
  は既存 transitive で新規追加なし。
- `crates/ruviform-video-proc/src/lib.rs`: `IccTransform` の kernel
  一覧 bullet（feature `icc` 限定の明記付き）・「Cargo features」
  doc 節（feature-off で外部依存ゼロ・feature lane の mise タスク
  案内）・`#[cfg(feature = "icc")] mod icc` + `pub use
  icc::{IccIntent, IccTransform}`。feature-off ビルドで doc link が
  切れないよう kernel 一覧は plain-text 名参照（intra-doc link 非
  使用）。
- `mise.toml`: `test-video-proc-icc`（`cargo test -p
  ruviform-video-proc --features icc`）・`lint-video-proc-icc`
  （`cargo clippy -p ruviform-video-proc --all-targets --features
  icc -- -D warnings`）— plan「Required mise tasks」節の定義どおり。
- `.github/workflows/ci.yml`: `icc-verify` ジョブ（
  `runs-on: ubuntu-latest`・rt-verify と同形の mise セットアップ +
  cargo cache・`CARGO_INCREMENTAL=0`・ALSA 依存なし）で上記 2
  タスクを実行 — `cargo test --workspace` が default feature のみ
  カバーするため feature 経路の regress 防止レーンを構造保証。
- `.gitattributes`: `*.icc binary`（fixture の line-ending 変換を
  抑止 — テキスト扱いすると golden が壊れる）。
- `docs/test-specs/icc-transform-contract-tests.md`: 等価性契約の
  「スカラー経路のみ・ホスト非依存」文言を実態に訂正（moxcms
  `mlaf` が aarch64 NEON / FMA x86 で fused FMA を選択 —
  target-feature 間で last-ulp 差が生じ得る・literal golden は
  x86_64 非 FMA baseline に pin + `cfg` ゲート）。reviewer B P1
  修正の文書側同期。

### Dependency justification（暫定規約「依存追加」記録）

- **moxcms 0.9.1**（BSD-3-Clause OR Apache-2.0）— design
  「Non-Functional Constraints」節の確定選択: pure-Rust 3-crate
  tree（moxcms + num-traits + pxfm — `num-traits` は既存
  transitive）・MSRV 1.89 適合・`default-features = false` で
  crate 側の `#![forbid(unsafe_code)]` が有効（SIMD feature 無効
  時）・保守状況は design で確認済み。optional + `default = []`
  で feature-off ベース経路は外部依存ゼロを維持（`cargo tree -p
  ruviform-video-proc` で実確認 — moxcms を含まない）。
- 却下した代替（design 記録の再確認・新規評価なし）: lcms2
  （C FFI・unsafe 境界・システム依存 — pure-Rust/unsafe 方針と
  非整合）・qcms（機能制約・保守状況）・自前 ICC 実装（mft/mAB
  LUT・A2B/B2A の再実装は scope 外）。

### Decisions During Implementation

- **flat `&[f32]` executor 呼出し**（deviation・deliberate）—
  design は `TransformExecutor<V>::transform(&[[V]], &mut [[V]])`
  の 1 要素 `&[[f32;4]]` スライス呼出しを記載していたが、実効
  API（moxcms 0.9.1 の `TransformF32Executor::transform`）は
  flat チャネルスライス `&[f32]`/`&mut [f32]` を取る。1 quad =
  4 channels として意味は同値 — per-pixel 1 呼出し・行バッファ
  なし・alloc-free の構造保証は design どおり維持。task の先行
  確認項目共通り実 API 形状を docs.rs/`cargo doc` で検証し、
  blocker 相当の相違（型・feature 名の不存在）ではなく呼出し
  形状の差と裁定。
- **fixture 3 件 + `*.icc binary`**（deviation・追加）— design
  の Files/Ownership 表に fixture はなかったが、決定的 golden の
  ために実 profile バイト列が必要（`include_bytes!` 消費）。
  pin エンジン built-in profile を `encode()` して同梱し、
  `#[ignore]`d `generate_fixtures` を再生成 recipe として残す。
  バイナリ fixture の line-ending 変換抑止に `.gitattributes`
  `*.icc binary` を追加。
- **`IccIntent` は design の 2 variant 集合どおり**（
  `Perceptual`/`RelativeColorimetric` — Saturation/
  AbsoluteColorimetric・黒点補償は plan non-goal）— moxcms
  `Intent` を公開せず crate-private 写像に閉じ込め。
- **CI `icc-verify` は ubuntu 単独・ALSA なし** — rt-verify と
  同形（mise-action + cargo cache + `CARGO_INCREMENTAL=0`）だが
  video-proc は audio-device と違い ALSA 不要 — ジョブ定義から
  省略。
- **literal golden の `cfg` ゲート**（reviewer B P1 修正）—
  moxcms `mlaf` が aarch64 NEON baseline・`target_feature = "fma"`
  x86/x86_64 で fused FMA を選ぶため bit 出力は build 構成間で
  異なり得る。golden は x86_64 非 FMA baseline（CI `icc-verify`
  lane）に pin し 4 件を `cfg` ゲート + 全 target で動く
  behavioral twin を追加。

### Independent reviews（APPROVE + request-changes→fixed）

実装後に独立レビュー 2 件を実施:

- **Reviewer A — APPROVE**。P0/P1 なし。P2 ×2 を記録:
  - `icc_transform_intents_build_and_run` が intent 非区別 —
    matrix-shaper sRGB fixture は intent-distinguishing LUT を持た
    ず両 intent が同一出力を返す（test コメントに正直に明記済み・
    intent-distinguishing LUT fixture pair は follow-up）。
  - `lut` feature 経路（mft1/mft2/mAB LUT profile）は fixture が
    matrix-shaper のみで未行使 — mft LUT fixture の coverage は
    follow-up。
- **Reviewer B — request-changes → P1 ×2 修正済み**:
  - **P1-1（fixed）**: 非 RGB color-space profile の受理 —
    `Layout::Rgba` + CMYK profile で channel 3 が K インクとして
    消費され alpha-as-K bug を生じ得た → ctor に
    `color_space == DataColorSpace::Rgb` 両端検証を追加（
    `"icc transform"` で拒否）+ patched-header（
    `DataColorSpace::Cmyk` 偽装 fixture）拒否テスト
    `icc_transform_ctor_rejects_non_rgb_color_space` を追加。
  - **P1-2（fixed）**: module doc/test-spec の「ホスト非依存・
    決定的」等価性契約が虚偽 — moxcms `mlaf` の FMA/NEON 選択で
    bit 出力は build 構成間で last-ulp 差を持ち得る → module
    doc・test-spec の文言を「同一 build 構成内で決定的」へ訂正・
    literal golden 4 件を `#[cfg(all(target_arch = "x86_64",
    not(target_feature = "fma")))]` でゲート・非ゲート behavioral
    twin `icc_transform_rgba_f32_keeps_extended_range` を追加。
    修正後の再レビューで承認。

## Related Experiments

### Adopted

なし — approved design/plan の確定実装のみ（実験ログなし）。

### Rejected

なし — 代替エンジン評価は design の確定事項を再確認したのみ
（新規 experiment なし）。

## Validation

- gate（実 log・全 exit 0）:
  - `mise run fmt-check` — exit 0
  - `mise run check` — exit 0（`cargo check --workspace
    --all-targets` — feature-off ベース）
  - `mise run lint` — exit 0（`cargo clippy --workspace
    --all-targets -- -D warnings` clean）
  - `mise run test` — exit 0（`cargo test --workspace` —
    feature-off ベース無退行）
  - `mise run test-video-proc-icc` — exit 0（feature-on —
    ruviform-video-proc lib tests **163/163 pass**（icc.rs 新規
    16 含む — `generate_fixtures` は `#[ignore]` で非選択）・
    doctests **24/24 pass** — icc.rs 由来 3 件を含む）
  - `mise run lint-video-proc-icc` — exit 0（feature-on clippy
    clean）
  - `mise run test-rt` — exit 0（`rt_` 名前フィルタで **101 件**
    実選択 — `icc` feature-on 時は icc.rs の `rt_process_icc_*`
    2 件が加わる形で `test-video-proc-icc` レーン内でも選択可能）
- 構造確認（実コマンド）:
  - `cargo tree -p ruviform-video-proc`（feature-off）— moxcms を
    含まない（media-core/video-core のみ）を実確認
  - `rg -n 'cfg.*feature = "icc"' crates/ruviform-video-proc/src/lib.rs`
    — `mod icc`/`pub use` 両方が feature-gated（L137/L148）
  - `rg -n "icc" crates/ruviform-video-proc/Cargo.toml` — optional
    dep + `icc = ["dep:moxcms"]` 定義あり
  - `rg -n "test-video-proc-icc|lint-video-proc-icc" mise.toml
    .github/workflows/ci.yml` — mise.toml L125/L128・ci.yml
    L147–151 に lane 定義が両方存在
- 注: `test-rt` は feature-off レーン（`cargo test --workspace`）
  — `rt_process_icc_*` は `test-video-proc-icc` レーンで選択・
  pass を確認（test-spec の「feature 有効時の test-rt 相当経路」
  規約どおり）。

## Non-Goals

- `CompositeOver`/`Crossfade`/`AffineTransform` の `RgbaF32` arm —
  Task 5 の scope。
- intent 拡張集合（Saturation/AbsoluteColorimetric）・黒点補償・
  CMYK/多チャネル layout・`in_place`/`options`/`any_to_any`/SIMD
  系 moxcms feature — plan non-goals どおり（`any_to_any` は
  alloc-free 契約を破るため明示的に off 維持 — Cargo.toml
  コメントに記録）。
- ICC profile の供給経路（media-io/アプリ層責務）・`ColorInfo` →
  CICP profile 便利コンストラクタ — plan non-goal。
- `assert_no_alloc` による alloc 機械検出 — design で後続 Issue
  へ defer 済み（`rt_process_*` は repeatable/deterministic・src
  非破壊の観測検証 — #97/#98 と同一レーン）。
- クロスエンジン等価性（lcms2/qcms との一致）— 契約外（pin 版
  moxcms + feature 集合が参照）。
- `Processor` trait 実装・`RtContext` 引数・engine/OFX 配線 —
  plan non-goals どおり。

## Risks

- golden vector は pin 版 moxcms 0.9.1 + 当 feature 集合・x86_64
  非 FMA baseline に束縛 — バージョン/feature 更新時は再導出が
  必要（`generate_fixtures` + golden 再導出手続きの自動化は plan
  non-goal・手順は test-spec と `#[ignore]`d recipe に記録済み）。
- literal golden が `cfg` ゲートされているため aarch64/FMA
  ホストでは bit-exact 検証がスキップされる — behavioral twin
  （executor 直接比較）で緩和済みだが、golden 相当の厳密検証は
  CI `icc-verify`（ubuntu x86_64）に依存。
- `IccTransform` は呼出し側が供給する profile バイト列を信用入力
  として parse する — 悪意ある/破損 profile は ctor で拒否するが
  profile 内容の正しさ（意図した色空間か）は consumer 責務。
- `mlaf` FMA 選択は feature 集合でなく target-feature 依存 —
  `icc-verify` CI lane（非 FMA x86_64）と異なる構成の本番利用で
  last-ulp 差は契約どおり許容。

## Follow-ups

- Task 5 `docs/tasks/rgba-f32-composite-affine-arms.md`（actions
  12–14）— 既存カーネルの `RgbaF32` arm。
- Task 6–7（final verification・closeout）— plan の単一クリティカ
  ルパス順序どおり（Full Gate は action 15 の一回 — `test-video
  -proc-icc`/`lint-video-proc-icc` を含む）。
- reviewer A P2: intent-distinguishing LUT fixture pair の追加 —
  `IccIntent` が mapping を変えることの実証 coverage（現状は
  焼付けの受理/決定性のみ検証）。
- reviewer A P2: `lut` feature 経路（mft1/mft2/mAB LUT profile）の
  fixture coverage — 現行 fixture は matrix-shaper profile のみで
  LUT 変換パスは未行使。
