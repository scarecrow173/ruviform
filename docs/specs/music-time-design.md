---
id: "SPEC-ey1gakUyUmWw9lBalMOE79"
type: "spec"
status: "approved"
title: "音楽時間設計（小節/拍/PPQN・テンポマップ）"
created: "2026-11-03"
updated: "2026-11-03"
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
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/time-model-extension.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/time-model-extension.md"
  derived-by:
    - "docs/adr/musical-time-model.md"
    - "docs/designs/music-time-model.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 音楽時間設計

## 背景

設計ベースラインの保留項「音楽時間（小節/拍/PPQN）設計 — 自前設計必要・第 1 期スコープ外。第 1 期のイベント/トランスポート型は拍位置を持たず拡張予約で吸収」の再着手 Issue（#33）。

第 1 期のトランスポート型（`ruviform-media-core::TransportSnapshot`・`ruviform-plugin-core::TransportInfo`）は `reserved` 拡張予約フィールドを持ち、媒体時刻（`MediaTime` = `TimeBase` + 整数カウント）のみを表現する。プラグイン規格側は音楽時間を要求する — CLAP `clap_event_transport`（`song_pos_beats`・tempo・time signature）・VST3 `ProcessContext`（musical position in PPQ・tempo・timeSig）— そのため中立 ABI との整合スコープを本設計で確定する。

## 要件

1. PPQN 分解能・tick 表現を確定する（規格間の変換損失を最小化する選択）
2. テンポマップ（テンポ・拍子変更の系列）のデータ構造と `MediaTime` ↔ tick 変換アルゴリズムを設計する
3. RT 契約との整合: 変換は allocation なし・有界コスト。テンポマップ本体の構築は非 RT、RT 参照は不変スナップショット経由
4. プラグイン transport ABI との整合スコープ（`TransportInfo` の拡張形・CLAP/VST3 への写像規則）を確定する
5. 所有クレートを確定する（media-core の時間モデルとしての位置付け）

## Acceptance Criteria

- [x] 時間表現（PPQN・tick 型）とテンポマップ方式の選択が ADR で記録（2+ 選択肢の比較付き）
- [x] design 文書に型設計・変換アルゴリズム・RT 制約・ABI 写像規則が記述
- [x] `TransportInfo`/`TransportSnapshot` の `reserved` 予約との整合が明示（拡張予約の吸収方針どおり ABI 形状を変えない設計）

## 関連文書

- [ADR: musical-time-model](../adr/musical-time-model.md)
- [Design: music-time-model](../designs/music-time-model.md)
- Issue: #33（親 Issue: #1）
