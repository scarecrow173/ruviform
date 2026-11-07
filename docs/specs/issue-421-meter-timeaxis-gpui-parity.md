---
id: "SPEC-Yq7KpWx3RtNm8LbVd2HaJe"
type: "spec"
status: "proposed"
title: "Issue #421 — Meter/TimeAxis GPUI 描画と Knob/Toggle/Meter/TimeAxis の Fake/GPUI parity"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/421"
    - "https://github.com/scarecrow173/ruviform/issues/387"
    - "https://github.com/scarecrow173/ruviform/issues/390"
    - "https://github.com/scarecrow173/audio-lens/pull/169"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/parity/new-kinds.json"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/driver_parity.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/cases.json"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-387-component-view-specs.md"
    - "docs/specs/issue-390-driver-contract-lab.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-414-projection-controlled-display.md"
---

# Issue #421 — Meter/TimeAxis GPUI 描画と Fake/GPUI parity

## 背景

Issue #387 の Component/View Specs は、Knob/Toggle/Meter/TimeAxis の中立 schema（`ruviform-ui` の `ComponentSpec`）と、Knob/Toggle の GPUI 描画を PR #420 で main へ統合した。残りの DoD は次のとおり未達だった。

- `Meter` / `TimeAxis` は GPUI 側で `unsupported component: <kind>` placeholder のまま。
- Knob/Toggle/Meter/TimeAxis を Fake Driver と GPUI Driver で同一 fixture により semantic parity 検証していない。
- driver-lab の `cases.json` の GPUI lane（UI-DVR-02 など）が実装実態と合っていない。
- AudioLens PR #169 が、GPUI 側の projection/intent を consumer test から観測できないことを #421 に依存として記録した（`settle_for_test` / `inbox_len_for_test` が `pub(crate)`）。

## 目的

- `Meter` / `TimeAxis` を GPUI Driver で実描画し、headless 側の payload 意味論（gap・viewport・tick）と一致させる。
- 新規 kind を含む Fake/GPUI parity fixture を driver-lab に置き、両 driver が同一 fixture で検査されること。
- driver-lab manifest の status を、実際に走らせたテストの結果と一致させる。
- Consumer 観測 seam の公開範囲を設計判断として記録する（本 PR での公開可否を含む）。

## 受入条件（Issue #421 本文の受入に対応）

| # | Issue #421 受入 | 本 PR での扱い |
| --- | --- | --- |
| A1 | Meter/TimeAxis が GPUI Driver で描画され、headless fixture の意味論と一致する | 達成。描画 plan（gap skip・viewport 外 cull・amplitude clamp）を純関数化し、fixture の `expect.*.drawn*` と GPUI テストで一致検査 |
| A2 | 新規 kind を含む Fake/GPUI parity fixture が driver-lab に追加されている | 達成。`driver-lab/parity/new-kinds.json`（Knob/Toggle/Meter/TimeAxis）を Fake（`driver_parity`）と GPUI（`test-ui-gpui`）の両方が読む |
| A3 | driver-lab manifest の status が実装実態と一致している | 達成（範囲内）。UI-DVR-02 GPUI を pass、UI-DVR-02/03 の Fake parity binding を追加、UI-DVR-03 GPUI は Cancel 等が未接続のため not_run のまま note を実態に合わせた |
| A4 | 各項目の検証が mise タスクとして記録されている | 達成。`test-ui-parity`（新設）と既存 `test-ui-gpui` / `test-ui-driver-contract` / `check-ui-driver-boundary` |

## スコープ（本 PR）

- `ruviform-ui-gpui`: Meter/TimeAxis の element 描画、plan 純関数、gpui-test（parity・reconcile・gesture）。
- `ruviform-ui-test`: parity fixture と fake 側テスト（`tests/driver_parity.rs`）。
- `cases.json` の status 更新、`mise.toml` の `test-ui-parity`、`docs/development-environment.md` の表。
- 文書 chain（spec → design → plan → task → test-spec）。

## 非スコープ（未達として明示し follow-up とする）

- `NumericInput` / `Transport` 相当 Control: #387 の Non-goals であり、Consumer 必要性の再評価を要する。本 PR では実装しない。
- 長尺・RT snapshot・gap/NaN precision・bounded memory・cancel/stale・GPU release の GPUI 側検証: 範囲が大きく本 PR の DoD から外す。UI-DVR-03（Cancel・遅延/順序逆転）・UI-DVR-07・UI-DVR-08 の GPUI lane は not_run のまま。
- Consumer 観測 seam の公開: 本 PR では公開しない（設計判断は design doc の D5）。AudioLens 側は GPUI parity を NotRun のまま。
- `LAB-SV-FADER-DOCK` / `LAB-AL-WAVEFORM-SEEK` story の 2 scale factor × 2 theme × 状態遷移の GPUI 検証: 未実施のため stories.json の status は変更しない。

## 不変条件

- 共通層（`ruviform-ui` / `ruviform-ui-theme`）に GPUI 型を出さない。`check-ui-driver-boundary` が pass すること。
- 公開 enum は `#[non_exhaustive]`、MSRV 1.89、workspace lint（`-D warnings`、pedantic）準拠。
- 共通 schema（`ComponentSpec` の Meter/TimeAxis）は変更しない。描画 policy は renderer 側に置く。
