---
id: "TASK-034V2hqLGYpcCEu3a3OqFm"
type: "task"
status: "in-progress"
title: "Issue #314 slice 6 — Consumer-shaped semantic oracle fixtures"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
  changes:
    added:
      - type: "file"
        path: "docs/tasks/issue-314-semantic-oracle.md"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/viz-semantic-audiolens.json"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/viz-semantic-synvelis.json"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_parity.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-314-semantic-oracle.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-314-semantic-oracle.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-314-golden-evidence.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/issue-314-semantic-oracle.md"
  verified-by:
    - "docs/test-specs/issue-314-semantic-oracle.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #314 slice 6 — Consumer-shaped semantic oracle fixtures

## Work

Issue #314 の semantic oracle 要求に対し、AudioLens/Synvelis 形状の
consumer-shaped semantic oracle fixture 2 件を driver-lab parity
fixture set に追加し、Fake payload 保持 lane と GPUI drawn-plan
lane の両方で同一 `expect` oracle を検証する。

- `crates/ruviform-ui-test/fixtures/driver-lab/parity/viz-semantic-audiolens.json`
  新設: Waveform（f64/長尺 `frames=5_000_002_000`/partial
  `count=173`/gap `count=0`/viewport 外 bin）+ TimeAxis
  （fractional tick・viewport 外 tick）+ Seek（domain `[0,200]`
  committed plain/normalized）。
- `crates/ruviform-ui-test/fixtures/driver-lab/parity/viz-semantic-synvelis.json`
  新設: Meter（partial `count=37`/gap/over-range `peak=1.7`/
  `rms=1.2`/viewport clamp・cull）+ Spectrogram（gap column/
  `magnitude_range` 外 magnitude/clamp・cull）+ TimeAxis。
- `crates/ruviform-ui-test/tests/driver_parity.rs`:
  `parity_viz_semantic_{audiolens,synvelis}_payload_retained_fake`
  追加 — mount 前 payload の verbatim 保持（f64 `to_bits`
  bit-exact・gap/partial 集計・over-range 値・viewport 外 tick
  温存）+ mount 受理 key + Seek committed 値。
- `crates/ruviform-ui-gpui/tests/consumer_observation.rs`:
  `observes_viz_semantic_{audiolens,synvelis}_oracle_through_seam`
  追加 — `GpuiObserver` seam 経由の mounted projection +
  `DrawnPlan`（`drawnChannels`/`drawnMarks`/`drawnBars`/
  `drawnCells`/Seek committed）が `expect.*.drawn*` と一致。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-314-semantic-oracle.md`（実装計画）
- `docs/designs/issue-314-semantic-oracle.md`（設計 — fixture
  schema・payload/plan 非対称・serde_json 制約・NaN 表現・lane
  分離）
- `docs/specs/issue-314-semantic-oracle.md`（spec — R1〜R6/AC）
- `docs/test-specs/issue-314-semantic-oracle.md`（test 仕様）

## Dependencies

- slice 1〜5（`docs/specs/issue-314-golden-*.md`、PR #481/#483/
  #487/#514 系）— `GpuiObserver` seam・`DrawnPlan`・parity
  fixture 構造・lane 語彙。
- #426（`docs/specs/issue-426-gpui-lab-binding.md`）— consumer
  story fixture registry。slice 4（pixel golden 接続）予約 scope
  であり本 slice とは別。
- #464（`docs/specs/issue-464-scenario-runner.md`）— Scenario
  Contract capability 分離。
- #313 — OS input lane owner。本 slice は同 lane に触れない。

## Affected Files

- `crates/ruviform-ui-test/fixtures/driver-lab/parity/viz-semantic-audiolens.json`（新規）
- `crates/ruviform-ui-test/fixtures/driver-lab/parity/viz-semantic-synvelis.json`（新規）
- `crates/ruviform-ui-test/tests/driver_parity.rs`（test 追加）
- `crates/ruviform-ui-gpui/tests/consumer_observation.rs`（test 追加）
- `docs/{specs,designs,plans,tasks,test-specs}/issue-314-semantic-oracle.md`（新規）
  + `docs/*/README.md`（index 自動生成）

## Done When

- [x] 両 fixture が `rvf.driver-lab.parity/v1` envelope
      （`schemaVersion`/`manifest`/`issue`/`description`/`view`/
      `expect`）を持ち、description が lane 境界・NaN 表現・
      serde_json 制約・非対象領域を記述する。
- [x] Fake lane が mount 前 payload の verbatim 保持を検査する
      （f64 `to_bits` bit-exact・gap `count==0`・partial
      `count<span`・over-range 値・viewport 外 tick 温存）。
- [x] GPUI lane が公開 `GpuiObserver` seam 経由で `observe`/
      `drawn` を検査し、`DrawnPlan` が `expect.*.drawn*` と一致
      する。
- [x] `DrawnPlan` pass が pixel golden pass に昇格しない lane
      分離が維持される（semantic/plan/pixel/OS-input を混同
      しない）。
- [x] serde_json/`str::parse` の丸め差が fixture 小数選定と
      文書で対処されている。
- [x] mise canonical 検証一式が pass する。

## Verification

- [x] `mise run fmt-check` / `check` / `lint` / `test`
- [x] `mise run test-ui-parity`（Fake 側新規 test 含む）
- [x] `mise run test-ui-gpui`（gpui-test、`consumer_observation`
      新規 test 含む）
- [x] `mise run test-ui-scenario-contract` / `test-ui-golden` /
      `test-ui-driver-contract` / `check-ui-driver-boundary`
      （非回帰）
- [x] `mise run test-rt`（非回帰）
- [x] `cargo test -p ruviform-ui-gpui --features gpui --test
      golden_capture -- --ignored --test-threads=1` — Windows 11
      x64 実 desktop session で実測（2026-11-10）: 3 passed
      （probe + `LAB-SV-FADER-DOCK`/`LAB-AL-WAVEFORM-SEEK` の
      story-keyed real-window capture。PrintWindow/BitBlt backend、
      blocked/baseline_missing → promote → pass/match、evidence
      JSONL 2 行を test 内で検証）。`--features gpui-test` では
      `unsupported`（window has no OS handle）を正直に返すことを
      確認済み — OS handle の無い lane が pixel pass に昇格しない
      capability 分離の実測証拠。
