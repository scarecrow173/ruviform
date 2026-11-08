---
id: "TSPEC-034UewQGei6BGVolWZlKjI"
type: "test-spec"
status: "approved"
title: "Issue #478 Fader/Knob 縦寸法契約の検証"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/478"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-478-fader-knob-height-contract.md"
      - type: "doc"
        path: "docs/designs/issue-478-fader-knob-height-contract.md"
      - type: "doc"
        path: "docs/adr/issue-478-fader-knob-vertical-size-contract.md"
      - type: "doc"
        path: "docs/plans/implement-issue-478-fader-knob-height-contract.md"
      - type: "doc"
        path: "docs/tasks/implement-issue-478-fader-knob-height-contract.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../designs/issue-478-fader-knob-height-contract.md"
    - "../adr/issue-478-fader-knob-vertical-size-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../test-specs/issue-453-canvas-size-audit.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/issue-478-fader-knob-height-contract.md"
    - "../designs/issue-478-fader-knob-height-contract.md"
    - "../adr/issue-478-fader-knob-vertical-size-contract.md"
  derived-by: []
  verifies:
    - "../designs/issue-478-fader-knob-height-contract.md"
    - "../adr/issue-478-fader-knob-vertical-size-contract.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #478 Fader/Knob 縦寸法契約の検証

## Purpose

Fader / Knob の Panel 子 slot が「`flex_grow` で残り高さを分け合い、
`min_h` で下限を保つ」契約（ADR Option C）を満たすことを、gpui-test
の bounds 計測で検証可能にする。本 spec は後続の実装 dispatch が
書く回帰テストの契約であり、コード変更を伴わない本 dispatch では
テスト自体は追加しない。

## Feature

Panel 内の Fader / Knob は、兄弟要素と共存しても押し出されず、
単独配置では残り高さを使い、下限を下回らない。

## Rules

- Rule: Fader / Knob の canvas bounds は幅・高さともに非 0 であり、
  slot の `min_h` を下回らない。
- Rule: Fader / Knob と固定/最小高さを持つ兄弟（Seek・Waveform 等）
  が同居する Panel で、兄弟の宣言寸法（Seek 24 px・viz slot
  `min_h` 120 px・TimeAxis 24 px）が維持され、Fader/Knob が全高を
  占有して兄弟を押し出さない。
- Rule: Fader / Knob が実質的に単独の伸縮可能子である配置
  （例: Tabs body 内の唯一の子）では、header 等の固定要素を差し引いた
  残り高さまで伸びる。
- Rule: 既存の canvas bounds 規則（#453）と gesture / parity /
  consumer-observation の検証を弱めない。

## Examples

- Example: Panel `[Fader, Seek, Waveform]` を既定テストウィンドウで
  マウントすると、seek が 24 px・wave が `min_h` 120 px 以上を保ち、
  かつ fader + seek + wave のレイアウト高さ合計が Panel 高を超えない
  （兄弟の押し出しがない）。
  - 検証手段（実装 dispatch で追加）: gpui-test で
    `RvfView.bounds` の `fader`/`seek`/`wave` を計測し、上記の
    不変条件を assert する。#453 実測の fader 936 px（960 px 窓）に
    seek 24 px・wave 120 px が同居すると合計 1080 px で Panel 高を
    超えるため、現行 `h_full` では失敗し（赤）、契約適用後に
    成功する（緑）。
- Example: Panel `[Knob, Meter, TimeAxis, Spectrogram]`（shared
  parity fixture 相当）で、knob canvas が非 0 かつ `min_h` 以上の
  高さを持ち、ruler が 24 px を保つ。
  - 検証手段（実装 dispatch で追加）: 既存
    `rvf_view_canvas_bounds_nonzero_for_parity_kinds` を拡張するか
    同型テストを追加し、knob の高さ下限と兄弟寸法を assert する。
- Example: Tabs > Fader の単独配置で、fader canvas の高さが
  tab body の header を差し引いた残り高さに一致する
  （0 や `min_h` への張り付きではない）。
  - 検証手段（実装 dispatch で追加）: `component_showcase` 相当の
    spec を gpui-test でマウントし bounds を計測する。

## Guarantees

- Fader / Knob の実高さは sibling 構成に依らず「残り高さの
  grow 分配結果」であり、`min_h` を下回らない。
- canvas bounds は slot と一致する（#453 の規則を維持）。
- `ruviform-ui` 契約層・Fake driver・consumer の `ViewSpec` 記述は
  不変。

## Non-goals

- Panel の overflow / scroll 方針（min 合計超過時の振る舞い）。
- Tabs body 自体の `size_full`、orientation に基づく sizing、
  grow 兄弟間の重み付け。
- 実窓スクリーンショットによる目視検証（実装 dispatch の任意
  補助証拠。契約の判定は bounds 計測が担う）。

## Risk

この契約が落ちると、`h_full` の全高占有に戻り、低い窓で兄弟要素が
Panel の表示領域から押し出される #453 残課題が再発する。また
`min_h` を下回る縮退により、fader が pointer 操作不能な細片になる
可能性がある。
