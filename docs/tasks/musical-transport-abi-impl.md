---
id: "TASK-034KpKrywhQhq1kR5WqrgS"
type: "task"
status: "done"
title: "TransportInfo.musical + アダプタ写像実装"
created: "2026-10-22"
updated: "2026-10-22"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-91-musical-transport-abi.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-91-musical-transport-abi.md"
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
---

# TransportInfo.musical + アダプタ写像実装

## Work

`TransportSnapshot`（media-core）と `TransportInfo`（plugin-core）に
`musical: Option<MusicalTimeInfo>` を追加し、CLAP/VST3 アダプタの inbound
トランスポート写像で音楽時間フィールドをフラグゲート付きで写像する。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-issue-91-musical-transport-abi.md
- docs/designs/music-time-model.md（写像表。beattime 係数を実 ABI 2³¹ に修正済み）
- docs/test-specs/musical-transport-abi-tests.md

## Dependencies

- Issue #90 / PR #93（`MusicalTimeInfo`・`PPQN` の提供元、merged）

## Affected Files

- crates/ruviform-media-core/src/transport.rs（`musical` フィールド・`with_musical`・`musical()`）
- crates/ruviform-plugin-core/src/transport.rs（同上 + doc 更新）
- crates/ruviform-plugin-clap/src/adapt.rs（`beats_to_ticks` + `adapt_transport` 拡張）
- crates/ruviform-plugin-vst3/src/component.rs（`ppq_to_ticks`・`musical_from_context`・`transport` 統合）
- docs/designs/music-time-model.md（beattime 係数 65536 → 2³¹ 修正）

## Done When

- [x] 両スナップショット型に `musical` が `Option` で乗り `reserved` 温存・`Copy` 維持
- [x] CLAP/VST3 双方がフラグゲート規則通り `Some`/`None` を返す
- [x] test-spec の全ケースが実装・パス

## Verification

- [x] `adapt_transport_maps_musical_fields` / `_gates_and_unset_forms`（CLAP）
- [x] `musical_from_context_maps_valid_fields` / `_gates_and_unset_forms`・`transport_combines_media_and_musical`（VST3）
- [x] `mise run check` / `lint` / 該当クレート `test` 全パス
