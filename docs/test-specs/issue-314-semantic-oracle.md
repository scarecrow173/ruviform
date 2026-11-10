---
id: "TSPEC-034V2hpT0kkEMk2hJtieRh"
type: "test-spec"
status: "approved"
title: "Issue #314 slice 6 — Consumer-shaped semantic oracle fixtures のテスト仕様"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-314-semantic-oracle.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-314-semantic-oracle.md"
    - "docs/designs/issue-314-semantic-oracle.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-314-semantic-oracle.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-314-semantic-oracle.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-314-semantic-oracle.md"
    - "docs/designs/issue-314-semantic-oracle.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #314 slice 6 — Consumer-shaped semantic oracle fixtures のテスト仕様

全て headless。`ruviform-ui-test`（Fake lane）と
`ruviform-ui-gpui --features gpui-test`（GPUI simulated lane）の
2 系統で、実装と独立した fixture JSON の `expect` oracle を検証
する。実 OS window・実 pixel・GPU には依存しない。

## Feature: semantic oracle fixture の payload 保持（Fake lane）

### Rule: mount 前 payload は宣言値を verbatim に保持する

- Example: `viz-semantic-audiolens.json` の `view` を
  `view_spec()` で parse した `ViewSpec` について、Waveform の
  `channel_bins` 総数が `expect.waveform.declaredBins` と一致し、
  `count==0` の bin 数が `expect.waveform.gapBins` と一致し、
  partial bin（`count=173 < span`）が宣言 count を保持する。
- Example: f64 source 値は `to_bits()` で bit-exact に保持される
  — `channel_bins[0][3].min` が `-0.1234567890123456`、
  `.max` が `0.9876543210987654` の bit pattern と一致し、
  f32 への narrowing・再量子化・補間が起きない。
- Example: `frames=5_000_002_000`（u32 超）の長尺 frame 数と、
  viewport `[4999999000, 5000000000]` 外の bin（`start<viewport.start`
  /`end>viewport.end`）が payload に残る。
- Example: TimeAxis の `ticks` 総数が `expect.timeAxis.
  declaredTicks` と一致し、`at < viewport.start` と
  `at > viewport.end` の tick が payload に残る。
- 検証: `parity_viz_semantic_audiolens_payload_retained_fake`。

### Rule: over-range/partial/NaN-gap の source truth は payload に残る

- Example: `viz-semantic-synvelis.json` の Meter について、
  `channel_levels` 総数が `declaredBins`、`count==0` 数が
  `gapBins` と一致し、`peak=1.7`/`rms=1.2`（`>1.0` の over-range）
  と sparse partial bin（`count=37 < span`）が verbatim に保持
  される — clamp は renderer の責務であり payload を書き換えない。
- Example: Spectrogram の `columns` 数が `declaredColumns`、
  全 cell 数が `declaredCells`、`count==0` column 数が
  `gapColumns` と一致し、`magnitude_range` 外の magnitude
  （`-70.0`/`10.0`）が verbatim に保持される。
- Example: `count==0` は NaN-gap の oracle 表現であり、JSON で
  表現不能な個別 non-finite 値は `render.rs`/`view.rs` の unit
  test が担う（本 fixture の非対象）。
- 検証: `parity_viz_semantic_synvelis_payload_retained_fake`。

### Rule: mount は expect.mountedKeys の全 key を受理する

- Example: 両 fixture で `ViewHost::mount` の
  `ProjectionOp::Mounted` に `expect.mountedKeys` の全 key が
  含まれ、Seek control の committed `plain=100.0`/
  `normalized=0.5`（domain `[0,200]`）が observe snapshot で
  一致する。
- 検証: 両 parity test の mount/observe 検査。

## Feature: semantic oracle fixture の drawn-plan 一致（GPUI lane）

### Rule: DrawnPlan は expect.*.drawn* と一致し pixel ではない

- Example: `viz-semantic-audiolens.json` を `GpuiObserver` に
  mount すると、`observe` の `components` に `mountedKeys` 全
  key が `mounted` で存在し、`drawn` の `DrawnPlan::Waveform
  .channels[].bins` が `expect.waveform.drawnChannels` の
  `(start,end)` 列と一致する（gap bin は plan に現れない）。
- Example: `DrawnPlan::TimeAxis.marks` が `expect.timeAxis.
  drawnMarks`（viewport 内 tick の x/major/label）と一致し、
  viewport 外 tick は mark に現れない。
- Example: Seek control の committed `plain`/`normalized` が
  `expect.seek` と一致する（f64-exact）。
- Example: `viz-semantic-synvelis.json` では `DrawnPlan::Meter
  .bars` が `drawnBars`（channel/x0/x1/peak/rms）と一致し、
  over-range amplitude は `[0,1]` に clamp、部分重なりは
  viewport 端に clamp、非重なりは cull される。
- Example: `DrawnPlan::Spectrogram.cells` が `drawnCells`
  （x0/x1/y0/y1/intensity）と一致し、magnitude は
  `magnitude_range` に clamp、gap column・viewport 外は
  現れない。
- 検証: `observes_viz_semantic_{audiolens,synvelis}_oracle_
  through_seam`（`consumer_observation.rs`、gpui-test）。

### Rule: lane 分離 — plan pass は pixel/OS-input pass に昇格しない

- Example: 両 GPUI test は `GpuiObserver::drawn`（plan lane）
  のみを検査し、`capture_window`・`BaselineStore`・OS input
  API を呼ばない — semantic oracle・draw plan・pixel golden・
  OS input の証拠が混同されない。
- Example: fixture の `description` が `expect.*.drawn*` を
  「GPUI draw plan（semantic/plan lane）の期待値であり pixel
  ではない」と明記する。
- 検証: test 名・fixture description・本 test-spec の Non-goals。

## Guarantees

- `expect` oracle は fixture JSON に実装と独立して宣言され、
  test はその宣言値との一致のみを検査する（実装の出力を
  oracle にしない）。
- f64 bit-exact は `to_bits()` 比較で固定され、f32 narrowing・
  再量子化・補間を検出できる。
- `count==0` gap・`count<span` partial・over-range・viewport
  外値の source truth が payload に残り、clamp/cull は plan
  生成時の責務として分離される。
- `unsupported`/`not_run`/`blocked` が `pass` に昇格する経路は
  存在しない（既存契約の非回帰）。

## Non-goals

- pixel golden（baseline compare/promote）・実 Window capture・
  OS input — 別 lane/別 Issue の責務。
- stale（spec 更新後の再投影）・selection・LOD・Revision・
  NativeSurface の oracle — 既存 lane が担う。
- NaN/Infinity 個別値の描画 — JSON 表現不能、unit test が担う。
- consumer 固有 theme/product fixture の複製 — fixture は
  consumer-*shaped* `ViewSpec` のみ。
- 実 OS・GPU・フォント環境への依存 — headless/gpui-test のみ。

## Risk

- `expect.*.drawn*` が `DrawnPlan` contract 面を超えて plan
  内部実装に密結合すると、正当な plan 変更で test が壊れる —
  fixture は公開 `DrawnPlan` variant の field のみを固定する。
- serde_json の丸め差（1.0.154 が一部 17 桁小数を 1 ulp 誤丸め）
  で bit-exact 検査が環境依存になる — fixture 小数は両 parser
  一致の値に限定し、制約を description/test コメントに記録。
- Waveform の viewport 非 cull を「正しい挙動」として固定する
  リスク — description に「現行 contract の正直な記述」と明記
  し、contract 変更時は fixture 更新が review 対象になる。
- payload 保持検査を plan 比較と混同すると「描画されるはず」
  の誤 oracle になる — Fake lane は mount 前 payload のみを
  検査し、plan 一致は GPUI lane に限定する構造で防ぐ。
