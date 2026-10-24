---
id: "IMPL-034LJ7kQmXwNpVtRhYsZdF"
type: "impl"
status: "completed"
title: "Issue #99 ToneMapSdr kernel — Task 3 implementation"
created: "2026-10-25"
updated: "2026-10-25"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-video-proc/src/tone_map.rs"
      - type: "file"
        path: "docs/impl/ir/issue-99-tone-map-sdr-task-3-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-video-proc/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/transfer.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/working_space.rs"
      - type: "file"
        path: "docs/tasks/tone-map-sdr-kernel.md"
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
    - "docs/tasks/tone-map-sdr-kernel.md"
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

# Issue #99 ToneMapSdr kernel — Task 3 implementation

## Summary

Issue #99 Task 3 完了 — actions 7–8（`ToneMapSdr` カーネル + gate）を
全範囲実施。`tone_map.rs`（新規・1016 行）に HDR→SDR 輝度写像
カーネル `ToneMapSdr` を実装 — BT.2390-8 §5.4 EETF（PQ 信号域の
Hermite spline を BT.2020 luma に適用し RGB を luma 比でスケール）を
design の式どおり逐語実装し、source peak `L_HS` / black `L_min` を
`src.color().hdr` から call 毎に導出する。`transfer.rs` の
`#[expect(dead_code)]` は Task 2 の removal note どおり `L_DS` 実
消費に伴い除去。4 gate 全 green 実 log・独立レビュー 2 件共に
APPROVE（P0/P1/P2 finding なし）。

## Context

approved plan `docs/plans/implement-issue-99-color-conversion-kernels.md`
の Task 3（`docs/tasks/tone-map-sdr-kernel.md`・actions 7–8）。
upstream: Task 1 IR
（`docs/impl/ir/issue-99-rgba-f32-transfer-task-1-implementation.md` —
`transfer.rs` 内部基盤と `PixelFormat::RgbaF32`）・Task 2 IR
（`docs/impl/ir/issue-99-working-space-kernels-task-2-implementation.md`
— 残置した `L_DS` の `#[expect(dead_code)]` は本 task が消費）・
approved spec/design/ADR（color-conversion-kernels 系・
broad-color-management）・TSpec
`color-working-space-kernel-tests.md`（proposed）。`tone_map.rs` は
`transfer.rs` のみに依存し Task 2 のカーネルとは独立（plan の依存
グラフどおり）。後続 task（icc-transform-feature・
rgba-f32-composite-affine-arms・final-verification・closeout）は未
着手。

## Implementation

### Changes

- `crates/ruviform-video-proc/src/tone_map.rs`（新規・1016 行・
  公開 mod — `lib.rs` で `pub use`）:
  - `Eetf` — per-call EETF パラメータ（`e_src = pq_eotf_inv(L_HS)`・
    `max_lum = pq_eotf_inv(L_DS)/e_src`・`ks = 1.5·max_lum − 0.5`・
    black-lift 係数 `b`）を `Eetf::from_hdr` が `src.color().hdr`
    から導出。`L_HS` 優先順位は
    `mastering_display.max_luminance`（ST 2086 の 1/10 000 cd/m²
    単位 `/10_000` → nits・`> 0` のみ）→
    `content_light_level.max_cll`（cd/m² 直接・`> 0` のみ）→ 既定
    1000 nits（`DEFAULT_L_HS`・BT.2390 参照 mastering peak）。
    `b` は `mastering_display.min_luminance`（同スケール）を
    `min < max` の整合値のみ採用し `pq_eotf_inv(L_min)/e_src` —
    absent・`min ≥ max` 逆転は `b = 0`（no black lift）
    フォールバック。
  - pixel codec — `rgba_f32_signal`（raw little-endian `binary32`
    quad → `f64` 拡大・`NaN`/`±∞` 伝播）・`f32_px_bytes`（`f64` →
    `binary32` LE serialize — 参照定義の丸めそのもの。
    `working_space.rs` の同名 helper と意図的に重複 —
    `transfer.rs` が両 mod の唯一の共有内部境界であり、pixel I/O
    helper は各 mod ローカルに置く依存境界規約）。
  - `tone_map_px` — 1 pixel の EETF 適用: BT.2020 luma
    `Y = 0.2627·R + 0.6780·G + 0.0593·B` → `e = pq_eotf_inv(Y·100)`
    を `e_src` で正規化 → `E ≤ ks` は identity・`E > ks` は Hermite
    補間 `t = (E−ks)/(1−ks)`、
    `E' = (2t³−3t²+1)·ks + (t³−2t²+t)·(1−ks) + (−2t³+3t²)·maxLum`
    → black lift `E'' = b·(1−E')⁴ + E'` →
    `Y_out = pq_eotf(E''·e_src)/100` で RGB を `Y_out/Y_src` 比率
    スケール・alpha パススルー。`ks ≥ 1`（`L_HS ≤ L_DS` の
    SDR 等価 source）と `Y_src ≤ 0`（比率スケール不能）は画素を
    verbatim コピー — `NaN` luma は `<= 0.0` を fail して式経路
    へ進み伝播。全演算は pedantic `suboptimal_flops` 対応の
    mul_add 形。
  - `ToneMapSdr` — unit ctor `new()` + `Default` impl（設定面なし —
    frame の `hdr` が call を駆動・構築は失敗しない）・
    `process(src, dst)`。src/dst 共に `RgbaF32`・同一寸法・共有
    `dst` 拒否（`plane_mut(0)` → `None`）— 全検証を書込み前に完了
    （`"src format"`/`"dst format"`/`"dimensions"`/`"dst plane"`
    の `InvalidValue`・拒否呼出しは `dst` 不変・部分書込みなし）。
    src の working-space 記述は非検証（composite カーネルと同規約
    — tag は `dst` へ verbatim 伝播）。post-loop で
    `dst.set_color({hdr: None, ..src_color})`（`hdr` 消費 —
    下流 stage の二重 tone-map を防止）+ `set_pts`/`set_duration`
    伝播。`process` は alloc-free — 行アクセスは `Plane::row`/
    `PlaneMut::row_mut` のみで padding を読み書きしない。
  - doc — module/type/method に入力規約（`RgbaF32` linear
    absolute-luminance working space・1.0 = 100 cd/m²）・`hdr`
    導出優先順位・`hdr` 消費規則・EETF 式・passthrough 規則・
    HDR 不在時の既定 1000 nits 挙動・≤ 4 ulp（binary32）等価性
    契約（passthrough 経路は bit-exact）・非有限伝播（宣言 `L_HS`
    を大きく超える finite 入力も Hermite spline を非 clamp 外挿
    し非有限を生じ得る — 契約が bound するのは finite 出力のみ、
    reviewer A 指摘の honesty wording を module doc に明記）・
    API 単位の `**RT contract:**` ラベル（`new`/`Default` = 非 RT
    by convention・`process` = RT-suitable alloc-free・非ブロッ
    キング・有界）。公開 API 例は doctest 2 件（型レベル・
    `ToneMapSdr::new`）。
  - inline `#[cfg(test)] mod tests` — 17 tests（unit 15 +
    `rt_process_*` 2）: 既定 1000-nit golden（source peak → SDR
    peak 写像）・EETF 形状（knee 下 identity・knee 上圧縮）・
    `L_HS` 優先順位（mastering 優先・`MaxCLL` フォールバック・
    既定 1000 nits）・`L_min` black lift・逆転 `min ≥ max` 無視・
    `ks ≥ 1` 全域 passthrough（bit-exact）・`Y_src ≤ 0`
    passthrough（bit-exact・mixed-channel 負 luma 含む）・RGB 比率
    スケール（負 channel 非 clamp 持越し）・working-space tag 非
    検証（mistagged src が pixel-identical に写像され tag 伝播）・
    `hdr` 消費 + メタデータ伝播（`dst` pts 事前 scrub で伝播を観測
    可能化）・拒否群（`dst` 不変断言付き）・padding 不変（dst 非
    書込み + src padding 非読取り）・RT 契約（repeatable/
    deterministic・src 非破壊）。
- `crates/ruviform-video-proc/src/lib.rs`: `ToneMapSdr` の kernel
  一覧 bullet・`mod tone_map;`・`pub use tone_map::ToneMapSdr;`
  を追加。
- `crates/ruviform-video-proc/src/transfer.rs`: `L_DS` が実消費者を
  得たため Task 2 の `#[expect(dead_code)]` + 理由コメントを除去
  （`expect` の意図どおりの自然な着火）。
- `crates/ruviform-video-proc/src/working_space.rs`: `FromWorking`
  doc の `ToneMapSdr` 言及を intra-doc link
  （`[`ToneMapSdr`](crate::ToneMapSdr)`）に更新 — コード変更なし。

### Decisions During Implementation

- `ks ≥ 1` 全域 passthrough（deviation・deliberate）— design
  L261-262 の記述どおり `ks ≥ 1` を全画素 passthrough の defined
  non-compression として実装（no-op early return ではなく画素
  パスを通し `dst` の `ColorInfo` 記録も実行 — declared peak を
  大きく超える画素も verbatim コピーで bit-exact）。reviewer 提示
  の代替解釈（peak 内のみ passthrough）は design 文面と非整合と
  裁定。
- `b = 0` フォールバック境界（deviation）— `min_luminance` の採用
  は `min < max` ordering rule の逐語適用。`min = 0` は整合値の
  ため採用され `b = pq_eotf_inv(0)/e_src` ≈ 9.7e-7（literal `b = 0`
  ではない）— golden で固定。
- `Default` impl は clippy `new_without_default` 対応で明示実装 —
  task の `new() + Default impl` 規約どおり（`ToWorking` と同型）。
- pixel codec helper（`rgba_f32_signal`・`f32_px_bytes`）は
  `working_space.rs` と同形を mod ローカルに重複 — `tone_map.rs`
  の許可依存は `transfer.rs` のみ（plan の依存境界）で、mod 間
  `pub(crate)` helper 共有を作らない KISS 選択。
- `pq_eotf_inv`/`pq_eotf` は nits 域で適用（`Y·100` ↔ `/100`）—
  design の正規化表記と同値（`pq_eotf_inv` の係数が nits 入力を
  仮定するため working-scale での直接呼出しは不可）。

### Independent reviews（APPROVE ×2 — P0/P1/P2 なし）

実装後に独立レビュー 2 件を実施 — 両 reviewer 共 APPROVE、
P0/P1/P2 finding はゼロ。提示された解釈候補は全て design 整合と
裁定:

- `ks ≥ 1` whole-domain passthrough — design L261-262 の文面どおり
  の実装と確認（代替解釈は却下）。
- `min = 0` 採用で `b ≈ 9.7e-7` — ordering-rule 逐語適用の帰結と
  確認（design の `b = 0` 記述は absent/逆転のフォールバックを指
  す）。
- `pq_eotf_inv` nits 域呼出し — design の normalized 表記と同値と
  確認。

記録済み nit（非 blocking・修正は別途判断）:

- `assert_ulp` は sign-magnitude bit 距離で ulp を測る — 同一符号
  比較のみに有効（現テストは全て同一符号 — 将来の cross-zero
  比較では不適）。
- alpha passthrough は `assert_ulp` 比較 — passthrough 契約上は
  `to_bits` 厳密 assert がより忠実。
- mistagged-src + `hdr` 同時存在の組合せは未テスト（tag 非検証と
  `hdr` 消費は別テストで各々検証済み）。
- crate README の kernel 一覧が stale（`ToneMapSdr`/`ToWorking`/
  `FromWorking` 未記載）— closeout task 7 の action 14 doc 整合
  scope へ defer。

## Related Experiments

### Adopted

なし — approved design/plan の確定実装のみ。

### Rejected

なし。

## Validation

- A8 gate（実 log・全 exit 0）:
  - `mise run check` — exit 0（`cargo check --workspace
    --all-targets`）
  - `mise run lint` — exit 0（`cargo clippy --workspace
    --all-targets -- -D warnings` clean — `L_DS` の
    `#[expect(dead_code)]` 除去後も warning なし）
  - `mise run test` — exit 0（`cargo test --workspace`・
    ruviform-video-proc lib tests **147/147 pass** — うち 17 が
    `tone_map.rs` 新規（unit 15 + `rt_process_*` 2）・doctests
    **21/21 pass** — tone_map 由来 2 件を含む）
  - `mise run test-rt` — exit 0（`rt_` 名前フィルタで
    `tone_map::tests::rt_process_tone_map_sdr_*` 2 件が実選択・
    pass）
  - `mise exec -- cargo fmt -p ruviform-video-proc -- --check` —
    exit 0（doc fix 後も clean）
- 構造確認（実コマンド）:
  - `rg -n "pub use tone_map" crates/ruviform-video-proc/src/lib.rs`
    — `pub use tone_map::ToneMapSdr;`（L132）
  - `rg -n "RT contract" crates/ruviform-video-proc/src/tone_map.rs`
    — API 単位の `**RT contract:**` ラベル 2 箇所（`new`・
    `process` — `Default` は `new` と等価の doc）
  - `rg -n "dead_code" crates/ruviform-video-proc/src/transfer.rs`
    — `#[expect(dead_code)]` 除去済み・`dead_code` 0 件
- reviewer A の honesty wording 指摘を module doc に反映後、
  `cargo test -p ruviform-video-proc` 再実行 — lib 147/147・
  doctests 21/21 pass を再確認済み。

## Non-Goals

- `IccTransform`/`IccIntent`・composite/affine の `RgbaF32` arm・
  feature `icc` — Task 4–5 の scope。
- tone-map ターゲットのパラメータ化（`L_DS` = 100 nits 固定）・
  stage 分割カーネル・HLG `L_W` 設定化 — plan non-goals どおり。
- `assert_no_alloc` による alloc 機械検出 — design で後続 Issue へ
  defer 済み（`rt_process_*` は repeatable/deterministic・src 非
  破壊の観測検証 — #97/#98 と同一レーン）。
- `Processor` trait 実装・`RtContext` 引数・フレームプール・
  engine/OFX への配線 — plan non-goals どおり。
- src の working-space 記述検証 — composite 規約どおり metadata
  解釈は consumer 責務（mistagged 入力も pixel-identical に写像）。

## Risks

- 宣言 `L_HS` を大きく超える finite 入力は Hermite spline を非
  clamp 外挿し非有限出力を生じ得る — 契約が bound するのは
  finite 出力のみ（module doc に明記済み・consumer が正しい
  `hdr` を供給する前提）。
- `ToneMapSdr` は `src.color().hdr` を信用入力として読む — 誤タグ
  の HDR metadata は誤った `L_HS`/`L_min` 導出に直結する（既定
  規則自体は spec/design の確定事項）。
- `assert_ulp` の sign-magnitude ulp 測定は同一符号比較限定 —
  将来 cross-zero golden を追加する場合は ulp 計測法の見直しが
  必要（nit として記録済み）。

## Follow-ups

- Task 4 `docs/tasks/icc-transform-feature.md`（actions 9–11）—
  feature `icc` + `IccTransform`/`IccIntent`。
- Task 5 `docs/tasks/rgba-f32-composite-affine-arms.md`（actions
  12–14）— `RgbaF32` arm と crate README kernel 一覧の整合（本
  task の stale README nit を回収）。
- Task 6–7（final verification・closeout）— plan の単一クリティカ
  ルパス順序どおり。
