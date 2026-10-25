---
id: "IMPL-034LZxQm9NvTyRHwKdPsGjF"
type: "impl"
status: "completed"
title: "Issue #99 色変換カーネル closeout — Task 7 implementation"
created: "2026-10-25"
updated: "2026-10-25"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-99-color-conversion-closeout-task-7-implementation.md"
    modified:
      - type: "file"
        path: "docs/tasks/color-conversion-closeout.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/plans/implement-issue-99-color-conversion-kernels.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/test-specs/color-working-space-kernel-tests.md"
      - type: "file"
        path: "docs/test-specs/icc-transform-contract-tests.md"
      - type: "file"
        path: "docs/test-specs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/color-conversion-closeout.md"
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

# Issue #99 色変換カーネル closeout — Task 7 implementation

## Summary

Issue #99 Task 7 完了 — canonical closeout（実績 IR・coverage・
task/plan status 更新）を immutable な実 log のレビューのみで実施
（新規メトリクス再計算なし）。全 7 task の実装・gate・独立レビュー
修正が commit 済み証跡で確認できたため、task 文書を `done`・plan
`status` を `"completed"` へ遷移させ、`verified-by` の両
test-spec を実結果で確認済み（schema 上の対応 status =
`approved`）へ更新し、索引 3 件（tasks/IR・plans・test-specs
README）を同期した。本 task は doc-only — コード変更なし。
**GitHub PR merge は lead の authoritative `gh` で別途実施 — 本 IR
および本 commit のいずれの文書も PR merged / issue closed の主張を
含まない。**

## Context

approved plan `docs/plans/implement-issue-99-color-conversion-kernels.md`
の Task 7（`docs/tasks/color-conversion-closeout.md`・action 16）。
upstream: Task 6 IR
`issue-99-final-verification-task-6-implementation.md`（commit
`6693801`）および Task 1–5 の全 IR・gate log。本 task は実装作業を
持たず、branch `feature/issue-99-color-conversion` 上の全完了証跡を
帳簿へ反映する最終段。

## Implementation

### Changes（doc-only — コード変更なし）

- `docs/impl/ir/issue-99-color-conversion-closeout-task-7-implementation.md`
  （新規）: 本 IR — 全 issue スコープ・per-task commit chain・gate
  証跡・review 集計・non-goal/deferred・spec/design relation 状態を
  集約記録。
- `docs/tasks/color-conversion-closeout.md`: `status` → `done`・
  `owners` → `scarecrow173`・`implemented-by` → 本 IR・`changes` に
  本 task で触れたファイル列を記録・verification checkbox 2 件を
  実実行後に [x] 化。
- `docs/tasks/README.md`: `TASK-034LFHv6XBnCj3XsdaEGVG` 行の
  status を `done` へ同期。
- `docs/impl/ir/README.md`: 本 IR の行を追記。
- `docs/plans/implement-issue-99-color-conversion-kernels.md`:
  frontmatter `status` → `"completed"`・冒頭 progress note を全
  7 task 完了（Task 6 commit `6693801` 含む）へ更新 —
  merged 主張なし。
- `docs/plans/README.md`: `PLAN-034LFFj86hwRvUljHN54Mc` 行の
  status を `completed` へ同期（index-metadata consistency）。
- `docs/test-specs/color-working-space-kernel-tests.md`・
  `docs/test-specs/icc-transform-contract-tests.md`: `status`
  `proposed` → `approved`・body `> status:` 注記を実結果確認済み
  へ更新 — task 文書の「`verified-by` の両 test-spec を実結果で
  confirmed に更新」の実施（test-spec の valid status 集合は
  `draft`/`proposed`/`approved`/`deprecated`/`superseded` で
  `confirmed` 値は存在しない — ratified-by-actual-results に
  対応する schema 値 `approved` へ写像）。
- `docs/test-specs/README.md`: 両 test-spec 行の status を
  `approved` へ同期。

### Whole-issue scope（7 task 集約）

| Task | Task 文書 | Actions | Commit | IR |
| --- | --- | --- | --- | --- |
| 前提 | —（discovery/spec/design/plan/tasks 文書パッケージ） | — | `fed943b` | — |
| 1 | `rgba-f32-format-and-transfer-internals.md` | A1–A3 | `c88860e` | `issue-99-rgba-f32-transfer-task-1-implementation.md` |
| 2 | `working-space-kernels.md` | A4–A6 | `6d27460` | `issue-99-working-space-kernels-task-2-implementation.md` |
| 3 | `tone-map-sdr-kernel.md` | A7–A8 | `d73b8b9` | `issue-99-tone-map-sdr-task-3-implementation.md` |
| 4 | `icc-transform-feature.md` | A9–A11 | `c5aa5a6` | `issue-99-icc-transform-task-4-implementation.md` |
| 5 | `rgba-f32-composite-affine-arms.md` | A12–A14 | `878d602` | `issue-99-rgba-f32-arms-task-5-implementation.md` |
| 6 | `color-conversion-final-verification.md` | A15 | `6693801` | `issue-99-final-verification-task-6-implementation.md` |
| 7 | `color-conversion-closeout.md` | A16 | 本 docs commit | 本 IR |

scope 全体: video-core `PixelFormat::RgbaF32`（packed RGBA・
4× binary32 LE・1 plane・16 B/px・`#[non_exhaustive]` で
non-breaking）+ `transfer.rs` 共有内部基盤（transfer
decode/encode・primaries→BT.2020 f64 結合行列・slot 解決・
working-space 定数）→ working-space 入口/出口カーネル
`ToWorking`/`FromWorking`（`{Bt2020, Linear, Identity, Full}`・
1.0 = 100 cd/m² 絶対輝度・decode+gamut 融合）→ HDR→SDR
`ToneMapSdr`（BT.2390-8 §5.4 EETF・`L_HS`/`L_min` を `hdr` から
call 毎導出）→ feature `icc` 配下の `IccTransform`/`IccIntent`
（moxcms 0.9.1 optional dep 閉じ込め・`test-video-proc-icc`/
`lint-video-proc-icc` mise lane + CI `icc-verify`）→ 既存
`CompositeOver`/`Crossfade`/`AffineTransform` の `RgbaF32` arm
（#97 の linear-light 合成 deferred を閉じる）→ full workspace
gate → 本 closeout。

### Per-task gate evidence（immutable log・再計算なし）

- Task 1: narrow gate 3 コマンド green — `mise run check`・
  `lint`・`test`（video-proc lib **100/100** — transfer.rs 新規
  12 + pixel.rs `RgbaF32` 行 3 件）・`cargo fmt --check` clean。
- Task 2: `check`・`lint`・`test`（lib **130/130** — working_space
  新規 30・unit 26 + `rt_process_*` 4・doctests **19/19**）・
  `test-rt`（`rt_process_*` 4 件実選択）・`fmt-check` 全 exit 0。
- Task 3: `check`・`lint`・`test`（lib **147/147** — tone_map 新規
  17・unit 15 + `rt_process_*` 2・doctests **21/21**）・`test-rt`
  （2 件実選択）・fmt clean 全 exit 0。
- Task 4: 7 gate green — `fmt-check`・`check`・`lint`・`test`
  （feature-off 無退行）・`test-video-proc-icc`（feature-on lib
  **163/163** — icc.rs 新規 16 含む・doctests **24/24**）・
  `lint-video-proc-icc`・`test-rt`（`rt_` 一致 **101 件**）。
- Task 5: `check`・`lint`・`test`（lib **171/171** — composite
  新規 18 + affine 新規 6・doctests **21/21**）・`test-rt`
  （workspace `rt_` 一致 **107 件**・video-proc 27・新規
  `rt_process_*_f32_*` 6 件含む）・`fmt-check` 全 exit 0。
- Task 6: full gate **10 コマンド全 exit 0** — `fmt-check`・
  `check`・`lint`・`test`（**49 test binaries・932 passed・0
  failed**）・`test-rt`（**実一致 107 tests・0 failed** — 0 件
  スキップでないことを件数で証明）・`test-video-proc-icc`（lib
  187 + doctest 24 = **211 passed**・1 ignored・0 failed）・
  `lint-video-proc-icc`・`bench-smoke`・`prek --all-files`（7
  hooks passed・1 skipped=Windows 非対象）・doc audit（303
  documents・`blocking: 0`・`findings: []`）— log は
  `docs/impl/exp/data/issue-99-final-gate-*-20261025.log`。

### Independent review findings totals

各 task ともレビューを受け、全 CONFIRMED finding は修正済み・
修正後に該当 gate 再実行で再 green を確認。

- Task 1: lead review F1–F5 — `encode_pq(+∞)` saturate guard・
  `FROM_BT2020` doc の結合方向誤記訂正・`pq_eotf` NaN 伝播の
  明文化・`encode_hlg` exp 分岐 golden 追加・往復 bound 根拠
  コメント — 全修正。
- Task 2: 独立レビュー 2 件（P0/P1 なし）— pts 伝播 tautology 化の
  修正・防御的 `_` arm 強化・doc wording・doctest bindings・
  テスト 3 件追加・拒否テストの `dst` 不変断言 — 全修正。
- Task 3: 独立レビュー 2 件共に APPROVE（P0/P1/P2 なし）— 解釈
  候補 3 件（`ks ≥ 1` passthrough・`min = 0` → `b ≈ 9.7e-7`・
  `pq_eotf_inv` nits 域）は全て design 整合と裁定。記録済み nit:
  `assert_ulp` sign-magnitude 限定・alpha passthrough の
  `assert_ulp` 比較・mistagged+`hdr` 組合せ未テスト・crate
  README stale（→ Task 5 の A14 doc 整合で回収済み）。
- Task 4: reviewer A APPROVE（P2 ×2 を follow-up 記録 —
  intent-distinguishing LUT fixture pair・mft LUT 経路
  coverage）・reviewer B request-changes → **P1 ×2 修正済み**
  （非 RGB color-space profile 受理の ctor ゲート追加 + patched
  header 拒否テスト・「ホスト非依存」等価性契約文言の訂正 +
  literal golden 4 件の `cfg` ゲート化 + 非ゲート behavioral
  twin 追加）— 修正後の再レビューで承認。
- Task 5: 独立レビュー 2 件共に APPROVE（P0/P1 なし）—
  `oa < f32::MIN_POSITIVE`（`2⁻¹²⁶`）の domain-relative cut
  解釈を両 reviewer が正当と裁定（divergence-pin test で固定）。
  適用済み修正: stale "both" → "every" 掃討・divergence-pin
  テスト追加・task 文書 typo「≤ 1 ulp」→「≤ 2 ulp」訂正・
  `q8_weight` doc 整合。
- Task 6/7: コードレビュー対象の実装変更なし（gate 記録・帳簿のみ）。

### Spec/design relation state（記録のみ・変更なし）

- `docs/specs/color-conversion-kernels.md` — `status: "proposed"`・
  plan の `implements` 先。task 文書の closeout 列挙 scope（実績
  IR・coverage・task/plan status・両 test-spec confirmed）に spec
  status 遷移は含まれないため本 task では変更しない（#98 では spec
  `approved` のまま closeout・#97 も同様 — status 遷移は一貫した
  慣行がなく、本 task の権限外として現状態を記録）。
- `docs/designs/color-conversion-kernels.md` — `status: "approved"`・
  plan の `derives-from` 先（+ `broad-color-management`・
  `video-proc-kernel-api-and-color-hdr-scope` 両 ADR）。
- `docs/test-specs/color-working-space-kernel-tests.md`・
  `docs/test-specs/icc-transform-contract-tests.md` — plan/task の
  `verified-by` 先・本 task で `approved` へ遷移（上述）。
- crate README: `crates/ruviform-video-proc/README.md` は Task 5 の
  A14 doc 整合で Issue #99 kernel section が新設済み（実確認 —
  `ToWorking`/`FromWorking`/`ToneMapSdr`/`IccTransform`+`IccIntent`・
  composite/affine 受理 format `{Rgba8, RgbaF16, RgbaF32}`・
  references に Issue #99 行）— 本 task での追加変更なし。

### Decisions During Implementation

- 実績記述は immutable な実 log（commit history・各 task IR・gate
  log）のレビューのみで構成 — 新規メトリクス再計算・再検証は行わ
  ない（task 文書の expected 契約どおり）。
- plan `status` の `"completed"` 遷移は全 7 task の done が commit
  済み証跡で確認できた時点でのみ実施 — 未来の完了を先取りしない。
- test-spec の "confirmed" は schema 値 `approved` へ写像 —
  test-spec の valid status に `confirmed` は存在せず、実結果で
  ratify された契約文書に対応するのは `approved`。
- **GitHub merge は lead の authoritative `gh` のみ** — 文書
  チェックリストで代行せず、本 IR・task・plan のいずれにも PR
  merged / issue closed の主張を書かない。
- `docs/plans/README.md` の status 列も同期 — doc audit の
  index-metadata consistency が plan status を index と照合する
  ため。

## Related Experiments

### Adopted

なし — approved plan の確定 closeout 手順のみ。

### Rejected

なし。

## Validation

- `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js
  audit --json` — exit 0・`blocking: 0`・`findings: []`（本 commit
  全編集後に実実行）。
- `rg -q '^status: "completed"$'
  docs/plans/implement-issue-99-color-conversion-kernels.md` — exit
  0（plan status 遷移後に実実行）。

## Non-Goals / Deferred

- reviewer A P2（Task 4）: intent-distinguishing LUT fixture pair —
  `IccIntent` が mapping を変えることの実証 coverage（現状は焼付け
  の受理/決定性のみ検証）。後続 Issue 管轄。
- reviewer A P2（Task 4）: `lut` feature 経路（mft1/mft2/mAB LUT
  profile）の fixture coverage — 現行 fixture は matrix-shaper
  profile のみで LUT 変換パスは未行使。後続 Issue 管轄。
- `assert_no_alloc` による alloc 機械検出の video-proc 適用 —
  design で後続 Issue へ defer 済み（`rt_process_*` は
  repeatable/deterministic・src 非破壊の観測検証 — #97/#98 と
  同一レーン）。
- OCIO 設定消費・ACES IDT/ODT/RRT フル管 — ADR D2-b 保留・D3-b
  後続 Issue（plan Non-Goals どおり）。
- `FromWorking` の `RgbaF16` dst・`RgbaF32 ↔ RgbaF16` shuffle・
  10/12-bit integer パイプライン・tone-map ターゲットのパラメータ
  化（`L_DS` = 100 nits 固定）・HLG `L_W` 設定化・stage 分割
  カーネル — plan Non-Goals どおり。
- intent 拡張集合（Saturation/AbsoluteColorimetric）・黒点補償・
  CMYK/多チャネル layout・`in_place`/`options`/`any_to_any`/SIMD
  系 moxcms feature — plan Non-Goals どおり（`any_to_any` は
  alloc-free 契約を破るため明示的に off 維持）。
- `Processor` trait 実装・`RtContext` 引数・engine/OFX 配線・
  ICC profile 供給経路（media-io/アプリ層責務）・`ColorInfo` →
  CICP profile 便利コンストラクタ・moxcms golden 再導出手続きの
  自動化 — plan Non-Goals どおり。
- GitHub PR merge・issue close — 本 bookkeeping の範囲外。lead が
  `gh` で別途実施する。

## Risks

- 残存リスクなし — doc-only 変更・全 gate green の証跡は branch 上に
  commit 済み。
- merged 主張がないため、docs と実 GitHub 状態の乖離リスクはない
  （merge 実施は lead の責務として明確に分離）。
- spec が `proposed` のまま残るのは記録済みの状態 — 実装完了との
  status 不整合は本 task の権限外として follow-up 記録。

## Follow-ups

- lead による authoritative `gh` での #99 PR 実 merge — 本 IR は
  merged 主張を含まず、merge 完了の記録も本 bookkeeping の範囲外。
- intent-distinguishing LUT fixture pair の追加（Task 4 reviewer A
  P2）。
- `lut` feature 経路（mft1/mft2/mAB LUT profile）の fixture
  coverage（Task 4 reviewer A P2）。
- video-proc への `assert_no_alloc` レーン適用（design defer —
  後続 Issue・#97/#98 同一レーン）。
- OCIO/ACES 経路（ADR D2-b/D3-b — 後続 Issue）。
- spec `docs/specs/color-conversion-kernels.md` の status 遷移判断
  （`proposed` → `implemented` 等）は closeout scope 外 — lead が
  別途裁定。
