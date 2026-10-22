---
id: "SPEC-5eEaW8lLmoyVPgocJnooYD"
type: "spec"
status: "approved"
title: "第 2 ターゲット設計（NLE 範囲・OFX 等映像プラグイン規格・広範色管理）"
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
    - "docs/designs/plugin-isolation.md"
    - "docs/adr/video-proc-kernel-api-and-color-hdr-scope.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/adr/second-target-editing-model.md"
    - "docs/adr/ofx-video-plugin-scope.md"
    - "docs/adr/broad-color-management.md"
    - "docs/designs/second-target-design.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 第 2 ターゲット設計

## 背景

Issue #32。設計ベースラインで「第 2 ターゲット: NLE／映像制作級」と分類済みだが、NLE 編集範囲・OFX 等映像プラグイン規格の対応範囲・広範色管理（ACES/OCIO 級）の3論点は「保留」として本 Issue に委任されていた（baseline Deferred Design Concerns 表・採否表 L534/L539/L649/L653）。本 spec は3論点の設計確定を要求する。

## Scope

### In Scope

- NLE 範囲の確定: 非破壊編集データモデル（タイムライン/クリップ/トラック）と既存エンジン（`ruviform-audio`/`ruviform-video`）への評価経路、コンポジット/遷移の責務所在
- OFX 等映像プラグイン規格の採否と対応範囲（ADR）
- 広範色管理の方針確定（ICC/ACES/OCIO の段階範囲 — ADR）

### Out of Scope

- 上記の実装コード（design-only Issue — 実装は後続 Issue）
- アプリ層の編集操作 UX・undo/redo・素材管理/プロキシ運用（利用側責務）
- 音声プラグイン規格（CLAP/VST3 確定済み）の再検討、AU/AAX/LV2 追加規格
- mobile/組込/wasm 対応

## Acceptance Criteria

- [x] NLE 編集範囲が ADR で確定（非破壊編集モデル採用・責務分割・実施順序つき）
- [x] 映像プラグイン規格の採否が ADR で確定（採用規格・範囲・除外範囲）
- [x] 色管理方針が ADR で確定（段階範囲・既存 `ColorInfo` メタデータ規約との整合）
- [x] design が3 ADR を具体化し、後続実装 Issue 分解を定義
- [x] 既存 plugin 分離設計（`docs/designs/plugin-isolation.md`）・`ColorInfo` 規約（video-proc ADR D3）と整合
