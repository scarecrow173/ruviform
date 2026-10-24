---
id: "TASK-034LFHdEfDbppnNeuh0u48"
type: "task"
status: "done"
title: "ToneMapSdr カーネル実装（BT.2390 EETF）"
created: "2026-10-25"
updated: "2026-10-25"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-video-proc/src/tone_map.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-video-proc/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/transfer.rs"
      - type: "file"
        path: "crates/ruviform-video-proc/src/working_space.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-99-color-conversion-kernels.md"
  implemented-by:
    - "docs/impl/ir/issue-99-tone-map-sdr-task-3-implementation.md"
  depends-on:
    - "docs/plans/implement-issue-99-color-conversion-kernels.md"
    - "docs/tasks/rgba-f32-format-and-transfer-internals.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/color-working-space-kernel-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# ToneMapSdr カーネル実装（BT.2390 EETF）

## Work

approved plan `implement-issue-99-color-conversion-kernels.md` の **actions 7–8 を 1 task の ordered substeps として実行**。`tone_map.rs` 新設 — `ToneMapSdr`（BT.2390-8 §5.4 EETF の HDR→SDR 輝度写像カーネル）を `transfer.rs` 内部基盤の上に実装する。Task `working-space-kernels` とは独立（`tone_map.rs` は `transfer.rs` のみに依存）。

### A7 — `ToneMapSdr`

- `crates/ruviform-video-proc/src/tone_map.rs` 新設 — `ToneMapSdr`（`new()` 無引数 + `Default` impl・`process(src, dst)`）— 輝度写像に設定面を持たない既定動作（BT.2390-8 §5.4 EETF 正本・固定 `L_DS = 100` nits）
- src `RgbaF32` → dst `RgbaF32`・同一寸法・排他所有（共有 `dst` → `InvalidValue("dst plane")`・寸法不一致 `"dimensions"`・`src.format != RgbaF32` → `"src format"` — 全検証書込み前・部分書込みなし）
- `src.hdr` からパラメータ導出（call-site 契約 — `src.color().hdr` を呼出し毎に読む）:
  - `L_HS` 導出優先順位: `mastering_display.max_luminance`（`/10_000` → nits・`> 0` のみ）→ `content_light_level.max_cll`（`> 0` のみ）→ 既定 1000 nits。不正/欠損は既定 1000 nits
  - `L_min` 導出: `mastering_display.min_luminance`（nits・`max_luminance` 未満の整合値のみ）— 不正順序は `b = 0` にフォールバック
  - EETF: `Y_src`（BT.2020 luma）に `b = min(1, (L_min/L_HS)^…)` と design 記載の `f(E₁)`/`ks = 1.5·L_DS/L_HS − 0.5`・3 次補間式を適用（式・定数は design「ToneMapSdr」節の正本どおり逐語実装 — 独自改変なし）
  - `L_HS ≤ L_DS`（100 nits）→ `ks ≥ 1` → 全域 passthrough 規則
  - `Y_src ≤ 0`（負値・ゼロ luma）は画素を passthrough（luma が非正の画素は比率スケール不能 — 厳密一致）
  - 比率保持 RGB スケール — `dst.rgb = src.rgb × (Y_dst/Y_src)`・alpha パススルー
- `dst.color` = src の color を保持して `hdr: None` のみクリア（primaries/transfer/matrix/range は伝播 — BT.2020/Linear の working-space 表現のまま）・`pts`/`duration` 伝播
- doc: `**RT contract:**` RT-suitable（alloc-free・非ブロッキング・有界）・参照実装 + ≤ 4 ulp（binary32）等価性契約・`hdr` 消費規則（出力で `hdr: None` — 配下 stage への再適用二重化を防ぐ）・passthrough 規則・HDR 不在時の既定 1000 nits 挙動を明記
- `lib.rs`: `mod tone_map` + `pub use tone_map::ToneMapSdr` + カーネル一覧へ追加

### A8 — gate

- test-spec `color-working-space-kernel-tests.md` の `ToneMapSdr` ケース（EETF 形状・`L_HS`/`L_min` 導出優先順位・`ks ≥ 1` passthrough・`Y_src ≤ 0` passthrough・`hdr` 消費・メタデータ伝播・拒否群・`rt_process_*`）を inline `#[cfg(test)] mod tests` に実装・全 pass
- `mise run test`・`mise run lint`・`mise run test-rt` exit 0

## Completion Gate

3 entry 全 green 実 log + lead review の後にのみ task 完了・buildable scope commit。

## Verification

- [x] `mise run test` — exit 0（tone_map.rs の unit tests 全 pass — video-proc lib 147/147・doctests 21/21・実 log）
- [x] `mise run lint` — exit 0（clippy `--all-targets -D warnings` clean）
- [x] `mise run test-rt` — exit 0 かつ本 task の `rt_process_*` が一致・pass（`tone_map::tests::rt_process_tone_map_sdr_*` 2 件を実選択・実 log）
- [x] `rg -n "pub use tone_map" crates/ruviform-video-proc/src/lib.rs` — `pub use tone_map::ToneMapSdr;` 存在（L132）
- [x] `rg -n "RT contract" crates/ruviform-video-proc/src/tone_map.rs` — API 単位の RT ラベル存在（`ToneMapSdr::new`/`process` の 2 箇所）
