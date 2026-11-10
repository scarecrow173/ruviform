---
id: "SPEC-034V2gKIAk9L49Yej6EXsG"
type: "spec"
status: "proposed"
title: "Issue #314 slice 6 — Consumer-shaped semantic oracle fixtures (AudioLens/Synvelis)"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
    - "https://github.com/scarecrow173/ruviform/issues/426"
    - "https://github.com/scarecrow173/ruviform/issues/464"
    - "https://github.com/scarecrow173/ruviform/issues/313"
    - "https://github.com/scarecrow173/ruviform/issues/262"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-314-semantic-oracle.md"
      - type: "file"
        path: "docs/designs/issue-314-semantic-oracle.md"
      - type: "file"
        path: "docs/plans/issue-314-semantic-oracle.md"
      - type: "file"
        path: "docs/tasks/issue-314-semantic-oracle.md"
      - type: "file"
        path: "docs/test-specs/issue-314-semantic-oracle.md"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/viz-semantic-audiolens.json"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/viz-semantic-synvelis.json"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_parity.rs"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/consumer_observation.rs"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-314-golden-core.md"
    - "docs/specs/issue-314-golden-scenario.md"
    - "docs/specs/issue-314-golden-capture.md"
    - "docs/specs/issue-314-golden-evidence.md"
    - "docs/specs/issue-426-gpui-lab-binding.md"
    - "docs/specs/issue-464-scenario-runner.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-314-golden-evidence.md"
    - "docs/specs/issue-426-gpui-lab-binding.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-314-golden-evidence.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-314-semantic-oracle.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #314 slice 6 — Consumer-shaped semantic oracle fixtures (AudioLens/Synvelis)

（slice 番号について: slice 4 は `issue-314-golden-capture` spec が
consumer story fixture 接続（#426 系・consumer 側）として予約済みで
あり、本 slice の対象はその pixel golden 接続ではなく semantic
oracle fixture の追加である。番号衝突を避けて slice 6 とする。）

## What Should Be Built

Issue #314 の「semantic Oracle を実装と独立した Fixture から検証」
要求に対し、AudioLens 形状・Synvelis 形状の 2 つの consumer-shaped
semantic oracle fixture を driver-lab parity fixture set
（manifest `rvf.driver-lab.parity/v1`）に追加し、Fake/GPUI の両 lane
で同一 `expect` oracle を検証する:

- `crates/ruviform-ui-test/fixtures/driver-lab/parity/viz-semantic-audiolens.json`
- `crates/ruviform-ui-test/fixtures/driver-lab/parity/viz-semantic-synvelis.json`

各 fixture は `view`（`ViewSpec` の JSON 表現）と `expect`
（semantic payload 保持値 + GPUI draw plan 期待値）を持つ。
`expect.*.drawn*` は draw plan lane の期待値であり pixel ではない。

- **Fake lane**（`crates/ruviform-ui-test/tests/driver_parity.rs`）:
  mount 前の payload が宣言値を verbatim に保持すること
  （f64 bit-exact・gap/partial bin・over-range 値・viewport 外
  tick の温存）と、mount が `expect.mountedKeys` の全 key を受理
  することを検査する。
- **GPUI lane**（`crates/ruviform-ui-gpui/tests/consumer_observation.rs`）:
  公開 `GpuiObserver` seam の `observe`/`drawn` 経由で、mounted
  projection と `DrawnPlan` が `expect.*.drawn*` と一致することを
  検査する。

## Why Now

slice 1〜5（`issue-314-golden-core`/`-scenario`/`-capture`/`-evidence`）
で Golden Utility・Scenario Contract 接続・実 Window capture・
環境正規化/証拠 lane は main 入りしたが、Issue #314 が要求する
「AudioLens の f64/長尺/precision と Synvelis の RT snapshot/frame
UI を損なわず、2 Consumer-shaped fixture で検証」に対応する
semantic oracle fixture は driver-lab parity set に存在しなかった。
既存の `new-kinds.json`/`selection-command.json` 等は component
種別の mount/intent 契約を担い、consumer 形状の source 値
（f64 precision・u32 超 frame 数・gap/partial・over-range・
viewport 端 clamp/cull）を独立 oracle として固定する fixture は
未整備である。

## Users and Value

- Ruviform の UI driver/test 保守者: consumer 形状の semantic
  oracle が Fake payload lane と GPUI draw-plan lane の両方で
  同一 fixture から検証され、描画意味の退行と視覚表現の退行が
  分離される。
- AudioLens/Synvelis consumer 側: 製品 fixture の複製なしに、
  自分たちのデータ形状（f64 精度・長尺 timeline・RT snapshot）
  に対応する oracle 契約を共有層で参照できる。
- #262 採用判定・#313 OS Gate: semantic/plan/pixel/OS-input の
  lane が混同されない証拠構造が維持される。

## Scope

### In Scope

- `viz-semantic-audiolens.json`: Waveform（2ch・`frames=5_000_002_000`
  の u32 超長尺・partial bin `count=173`・gap bin `count=0`・
  f64 precision 値・viewport 外 bin）+ TimeAxis（fractional tick・
  viewport 外 tick）+ Seek（domain `[0,200]` の committed
  plain/normalized 値）。
- `viz-semantic-synvelis.json`: Meter（2ch RT-like level bin・
  partial `count=37`・gap bin・over-range `peak=1.7`/`rms=1.2`・
  viewport clamp/cull）+ Spectrogram（7 column・gap column・
  `magnitude_range` 外 magnitude・viewport clamp/cull）+
  TimeAxis（fractional tick・viewport 外 tick）。
- Fake 側 payload 保持検査と GPUI 側 drawn-plan 一致検査の test
  追加（既存 test ファイルへの additive 変更）。
- 本 slice の文書チェーン。

### Out of Scope

- consumer 固有 Theme/製品 fixture の基盤側複製 — fixture は
  consumer-*shaped* な `ViewSpec` のみを持ち、theme/product
  infra を共有層に持ち込まない。
- pixel golden への接続（story fixture → golden verdict）—
  slice 4（#426 系として予約済み）の範囲。
- 実 Window capture・OS input — #313/`golden_capture.rs` の
  既存 lane の責務。本 slice の `DrawnPlan` は pixel ではない。
- stale/selection/LOD/Revision・NativeSurface の oracle —
  既存 lane（reprojection test・selection-command.json・
  AttachEpoch/scenario test・#388）が担う。
- JSON で表現できない NaN 個別値の描画検査 — `render.rs`/
  `view.rs` の unit test が担う。
- 新規依存・公開 API 変更。

## Requirements

- R1: fixture は実装と独立した JSON であり、manifest
  `rvf.driver-lab.parity/v1`・`schemaVersion: 1`・Issue #314
  source URL を持つ。`view` は mount 可能な `ViewSpec`、
  `expect` は検査期待値を持つ。
- R2: Fake lane は mount 前 payload が宣言値を verbatim に保持
  することを検査する — f64 は `to_bits` で bit-exact、gap bin
  （`count==0`）・partial bin（`count<span`）・over-range 値・
  viewport 外 tick は payload に残り、renderer の clamp/cull は
  payload を書き換えない。
- R3: GPUI lane は公開 `GpuiObserver` seam のみを使い、
  `observe` の mounted projection と `drawn` の `DrawnPlan`
  が `expect.*.drawn*`（`drawnChannels`/`drawnMarks`/
  `drawnBars`/`drawnCells`/`seek` committed 値）と一致することを
  検査する。`DrawnPlan` は semantic/plan lane の証拠であり
  pixel golden ではない。
- R4: fixture の `description` は semantic oracle・draw plan・
  pixel の境界、NaN の `count==0` 表現、serde_json の丸め制約、
  および本 fixture が担わない領域（stale/selection/LOD/
  Revision/NativeSurface）を明記する。
- R5: bit-exact oracle の小数は serde_json と `str::parse` が
  一致する値に限定する（serde_json 1.0.154 は一部の 17 桁小数を
  1 ulp 誤丸めすることを実測済み — fixture/test コメントに記録）。
- R6: lane 分離を維持する — semantic oracle pass・drawn-plan
  pass・pixel golden pass・OS input pass を相互に昇格させない。
  実 Offscreen/OS capture 未実測を Fake/旧 WebView の pass で
  代替しない（#262/#313 への証拠リンクは既存 lane が保持）。

## Acceptance Criteria

- [ ] `cargo test -p ruviform-ui-test --test driver_parity` が
  exit 0 で、新規 2 test（AudioLens/Synvelis payload 保持）を含む。
- [ ] `cargo test -p ruviform-ui-gpui --features gpui-test --test
  consumer_observation` が exit 0 で、新規 2 test（両 fixture の
  drawn-plan 一致）を含む。
- [ ] 両 fixture の `description` が semantic/plan/pixel 境界と
  coverage 限界を記述している。
- [ ] mise canonical 検証 lane（fmt-check/check/lint/test/
  関連 task）が exit 0。
- [ ] docs チェーン完備・index 再生成済み・#262/#313 への証拠
  リンクが記録されている。

## Deferred Design Concerns

- consumer story fixture（#426 `lab_story`）の pixel golden 接続
  | slice 4 として予約済みの別 scope | #426 story fixture の
  golden verdict 接続着手時 | semantic oracle と pixel golden の
  lane が混同される
- 本 fixture view の実 Window pixel capture | `golden_capture.rs`
  の story-keyed lane が lab_story fixture を対象とする現行構成
  | consumer-shaped view の pixel baseline 需要が生じた時 |
  semantic/plan 証拠のみで pixel 証拠が欠けたまま残る
- NaN 個別値・stale・LOD・Revision の oracle 深化 | 既存 unit
  test/lane が現行契約を担保 | contract 変更で既存担保が崩れた時 |
  JSON 表現不能領域の oracle が不足する

## Sources and Relations

- Issue #314（RF-GT-05）— semantic oracle を実装と独立した
  fixture から検証し、AudioLens/Synvelis の 2 consumer-shaped
  fixture で検証する要求の正本。
- Issue #426（GPUI lab binding / story fixture）— consumer
  story fixture の code registry。slice 4（pixel golden 接続）
  の対象であり本 slice とは別 scope。
- Issue #464（Scenario Contract / capability 分離）— semantic/
  visual capability の分離契約。
- Issue #313（Windows 実 OS driver / SendInput gate）— OS input
  lane の owner。本 slice は同 lane に触れない。
- Issue #262（native 採否 PoC）— semantic/plan/pixel/OS-input
  lane 分離の採用判定証拠の受け手。
- `docs/specs/issue-314-golden-evidence.md`（slice 5）— lane
  語彙（`pixel-golden`）と #262/#313 リンク構造の先行 slice。
