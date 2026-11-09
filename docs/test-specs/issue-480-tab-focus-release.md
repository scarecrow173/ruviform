---
id: "TSPEC-034UiRtw5zDZciI3peq94F"
type: "test-spec"
status: "approved"
title: "Issue #480 tab_focus 解放テスト仕様"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/designs/issue-480-tab-focus-release.md"
    - "docs/plans/issue-480-tab-focus-release.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-ui-gpui/src/render.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-480-tab-focus-release.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-480-tab-focus-release.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-480-tab-focus-release.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-480-tab-focus-release.md"
    - "docs/designs/issue-480-tab-focus-release.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/480"
  defers: []
  deferred-by: []
---

# Issue #480 tab_focus 解放テスト仕様

## Purpose

`RvfView::tab_focus` の op 駆動解放契約を恒久テストで固定する。
「key が spec を去った strip の handle は解放され、残る strip の
handle・フォーカス・入力経路は一切変わらない」ことを、実フォーカスと
実 key dispatch を通す headless（gpui-test）で検証する。実装方法
（どの関数で remove するか）ではなく、外部から観測できる map の
存否とフォーカス保持を保証する。

## Feature

selectable Tabs strip の FocusHandle ライフサイクル —
`ProjectionOp::{Removed, Unmounted}` 駆動の解放と、
`Updated`/sibling に対する保持。

## Rules

- selectable Tabs strip（`selection: Some`）を描画すると、その key の
  `tab_focus` entry が存在する。
- `update_spec` で strip の key が spec から消える（`Removed`）と、
  その key の entry は解放される。sibling strip の entry・フォーカス・
  key 応答は保持される。
- 同一 key の kind-swap（`Removed`+`Mounted` の対）でも entry は
  解放される（新しい component identity は次回描画で新 handle を得る）。
- `host.unmount()`（全 key に `Unmounted`）で `tab_focus` は空になる。
- `Updated` op（同一 key の props 変更。`selection: Some → None` を
  含む）では entry を解放しない — key は mounted のままで、
  selection が戻ればフォーカスを維持できる（design の決定）。
- 非 selectable Tabs（`selection: None`）は entry を作らず、op の
  有無に関わらず影響を受けない。

## Examples

- Given: `strip.a`・`strip.b` の 2 つの selectable strip を持つ spec が
  mount・描画済み。When: `strip.b` の header を押してフォーカスし、
  `update_spec` で `strip.a` を除去する。Then: `tab_focus` から
  `strip.a` が消え、`strip.b` の entry とフォーカスが残り、
  `strip.b` は arrow key で selection を動かせる。
- Given: 同上。When: `strip.b` の `selection` を `None` にする
  `update_spec`（`Updated` op）。Then: `strip.b` の entry は残る。
- Given: 同上。When: `strip.a` を同一 key の Label に kind-swap する
  `update_spec`（`Removed`+`Mounted`）。Then: `strip.a` の entry は
  消え、`strip.b` は残る。
- Given: 同上。When: `host.unmount()` して `flush_events` する。
  Then: `tab_focus` は空。

## Guarantees

- `flush_events` が `Removed`/`Unmounted` op を処理する限り、
  `tab_focus` のサイズは「現在 spec に存在し selectable になりうる
  strip の履歴」に有界であり、key の消失と同期して縮む。
- 解放は `Intent` の egress・receipt・selection undo に影響しない
  （`flush_events` の Intent 経路は不変）。

## Non-goals

- `control_focus` の解放（#479 FU-c — 別 Issue）。
- detach 時の解放（tree は mounted のまま — 契約上保持が正しい）。
- 実 OS window でのフォーカス証拠（headless の `is_focused` で十分;
  実窓手動確認は #313 の責務）。
- gpui `FocusMap` 内部状態の直接検査（ref_count・slot 除去は gpui の
  `release_dropped_focus_handles` が担保する実装詳細）。

## Risk

- 解放を `Updated` にまで広げる誤実装: selection の一時的な喪失で
  フォーカスを失う退行。test-spec の「`Updated` で保持」ルールが
  これを固定する。
- 描画 sweep への置き換え: 未描画 strip の handle 喪失でタブ復帰時に
  フォーカスが切れる退行。「op 駆動のみ」のルールがこれを固定する。
