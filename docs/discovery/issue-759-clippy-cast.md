---
id: "DISC-034ZhjBMAgFhy2Ry7D4Z5S"
type: "discovery"
status: "resolved"
title: "Issue #759 plugin-clap host.rs f64 as u32 cast — clippy lint lane 破損の診断と境界契約"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/759"
    - "crates/ruviform-plugin-clap/src/host.rs"
    - "crates/ruviform-plugin-core/src/boundary.rs"
    - "crates/ruviform-plugin-vst3/src/host.rs"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/tasks/issue-759-clippy-cast.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-389-clap-l0-editor-attach.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/tasks/issue-759-clippy-cast.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #759 plugin-clap host.rs f64 as u32 cast — clippy lint lane 破損の診断と境界契約

## Summary

`crates/ruviform-plugin-clap/src/host.rs` の `set_gui_size` が
plugin-core の `GuiSize { width: f64, height: f64 }`（論理単位）を
clack `GuiSize { width: u32, height: u32 }`（ピクセル）へ `as` cast
で写像しており、workspace clippy（pedantic・`-D warnings`）が
`cast_possible_truncation` + `cast_sign_loss` × 2 フィールド = 4
events を error として報告し、`mise run lint ruviform-plugin-test-clap`
等の host feature 含有 lane が main で常時 Fail していた
（`ruviform#754` merge `0450f92f` 由来）。診断の結果、修正は
「負値・NaN・非有限・`u32::MAX` 超過の境界契約を決め、境界チェック
付き変換で実装する」方向で確定した — 採用契約は **最近接ピクセルへの
round + `[0, u32::MAX]` への飽和**（saturation）であり、VST3 adapter
が同一 neutral verb に適用する `ViewRect` 写像と整合する。

## Exploration Goal

Issue `ruviform#759` の再現・根因確定と、`set_gui_size` の
f64→u32 変換が負値・NaN・非有限・`u32::MAX` 超過をどう扱うかの
境界契約の決定。lint を通すだけでなく、cast の設計意図を
コードまたは doc に明記することが受入条件。

## Key Issues and Assumptions

- `GuiSize`（plugin-core `boundary.rs`）は「scale-independent な
  論理単位」の `f64` で、定義域の明示的な境界を持たない。
  `clap.ext.gui` の `GuiSize` は `u32` ピクセル — 型の精度差は
  adapter の責務として写像する必要がある。
- `HostError::InvalidValue`（「caller-supplied value が定義域外」）
  は存在するが、`GuiSize` の neutral 契約に「非負・有限・
  `u32::MAX` 以下」という定義域が明文化されていないため、
  「定義域外」と断定する根拠が弱い。
- 兄弟 adapter の VST3 host は同じ verb で
  `size.width.round() as i32` + `#[expect(clippy::cast_possible_truncation)]`
  を採用済み — 同一 neutral verb の adapter 間で拒否/飽和が
  分岐すると consumer が一貫した挙動を期待できない。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| 飽和写像（round → `[0, u32::MAX]` clamp → `as u32`） | verb が全域で定義される。VST3 の `ViewRect` 写像と整合。plugin の `set_size` accept/decline が最終判定として残る。Rust `as` の飽和意味論と一致 | `as` 自体は lint 対象のため局所 `#[expect]` が必要（clamp 後も構文 lint は発火） | yes |
| 拒否（`HostError::InvalidValue`） | 「caller-supplied value が定義域外」の既存 variant に素直に乗る。異常値を plugin に渡さない | neutral `GuiSize` に定義域の明文化がなく「定義域外」の断定が弱い。VST3 adapter と verb 挙動が分岐する。`InvalidValue` は video surface 起源の variant で GUI への転用は意味の拡張 | no |
| 素の `#[expect]` のみ（契約未決定） | diff 最小 | 境界挙動の契約が暗黙のまま — Issue の「契約を決める」要求を満たさない | no |

## Tentative Conclusions and Hypotheses

- **confirmed**: lint error は `host.rs` の `set_gui_size` 内
  `width: size.width as u32` / `height: size.height as u32` の
  2 cast ×（`cast_possible_truncation` + `cast_sign_loss`）= 4
  events。workspace `clippy::pedantic = warn` + lint task の
  `-D warnings` で error 化する。
- **confirmed**: 採用契約は飽和写像 — 負値・NaN・非有限 → `0`、
  `u32::MAX` 超過 → `u32::MAX`、小数部は最近接ピクセルへ round
  （VST3 の `.round()` と同じ丸め）。`set_gui_size` は全域で
  `Ok`/`PluginFailed`/`Unsupported` を返し、異常サイズの最終判定は
  plugin の `set_size` に委ねる。
- **confirmed**: 実装は `gui_size_component(f64) -> u32` ヘルパーに
  集約し、境界契約を doc comment に明記。cast lint は clamp で
  実際に安全化した上で `#[expect]` + 理由を局所許可する
  （`#[allow]` は使用しない）。

## Open Questions

- なし — spec 相当の要件は Issue 本文の受入条件で確定済み。

## Promotion Candidates

- [ ] spec-doc needed: なし（Issue 本文が spec 相当 — 小規模 defect slice）
- [ ] adr-doc needed: なし（adapter 内の局所的な写像契約であり、
      アーキテクチャ決定ではない）

## Sources and Relations

- Issue: https://github.com/scarecrow173/ruviform/issues/759
- 由来: `ruviform#754`（merge `0450f92f`、editor verbs 実装）
- 設計参照: `docs/designs/issue-389-clap-l0-editor-attach.md`（CLAP
  L0 editor attach — `open_gui`/`set_gui_size` の verb 群）
- 兄弟実装: `crates/ruviform-plugin-vst3/src/host.rs` の
  `set_gui_size`（`ViewRect` 写像 + `#[expect]` 先例）
- neutral 契約: `crates/ruviform-plugin-core/src/boundary.rs`
  `GuiSize`、`src/plugin.rs` `Plugin::set_gui_size`

## Evidence

- 2026-11-18: `mise run lint ruviform-plugin-test-clap` が
  `host.rs` の `as u32` cast で clippy error を報告（Issue 記載の
  再現と一致）。修正後は同 lane が exit 0。
- 2026-11-18: `cargo clippy` 実測で `f64 as u32` は
  `cast_possible_truncation` + `cast_sign_loss` の 2 lint が
  発火することを確認（2 cast で計 4 events — Issue の「4 error」と一致）。
