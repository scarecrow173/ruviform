---
id: "IDEA-034JbTBzB9hkv9D4aVkU57"
type: "idea"
status: "promoted"
title: "オーディオデバイスバックエンド評価（cpal 等）と採否"
created: "2026-10-20"
updated: "2026-10-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/23"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../discovery/evaluate-audio-device-backends.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# オーディオデバイスバックエンド評価（cpal 等）と採否

## Summary

Issue #23。`ruviform-audio-core` が定義し `ruviform-audio-device` が実装するデバイス境界 trait 層の契約に従い、cpal 等のオーディオデバイスバックエンドを評価して採否を決める PoC。cpal のメンテナンスリスクを踏まえ backend 抽象を維持しつつ、代替候補を比較する。

## Problem and Motivation

- 設計ベースライン（Issue #1）の後続実装 Issue として起票。実デバイス入出力を担う層のバックエンド選定が未確定のままでは、デバイス境界 trait 契約を現実の制約（コールバック駆動・レイテンシ・サンプル形式）に照らして検証できない
- cpal は事実上の標準だがメンテナンス状況にリスクがあると指摘されており、採否には根拠が必要

## Expected Value

- バックエンド採否の判断材料と、デバイス境界 trait 契約が外部ライブラリに適合することの検証結果が得られる
- cpal 不採用の場合でも差し替え可能な backend 抽象の維持方針が確定する

## Open Questions

- 評価対象候補の確定（cpal 以外: oboe-rs / cubeb 系 / プラットフォーム専用 API 直結 等）
- 評価軸（メンテナンス状況・ライセンス・依存木サイズ・RT 適合性・対応プラットフォーム）の重み付け
- 採否決定の記録形式（ADR で採否を確定するか）
- PoC 実装の範囲（最小限の列挙+ストリーム開始レベルか、trait との適合検証まで行うか）

## Next Action

- [x] Promote to discovery-doc for deeper exploration
- [ ] Promote directly to spec-doc if requirements are clear
- [ ] Park for later reconsideration
- [ ] Discard — reason: <!-- why -->
