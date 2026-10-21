---
id: "ADR-034JZVID00000000000002"
type: "adr"
status: "accepted"
title: "A/V 同期判定ポリシー（audio master + PTS 追従）"
created: "2026-10-21"
updated: "2026-10-21"
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
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/video-engine.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/designs/video-engine.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# A/V 同期判定ポリシー

## Context

ベースライン確定: 共通時間軸は `ruviform-media-core` 有理数時刻、`MasterClock`（オーディオデバイス時刻）を A/V 同期の基準時計とし、動画は PTS 追従（ffplay 型）。本 ADR は追従時の遅延・早期フレームへの判定ルールを確定する。

## Decision

**閾値ベースの 3 判定**: `lateness = pts - master_now` を計算し、

- `lateness > early_threshold` → **Wait**（まだ表示時刻でない）
- `|lateness| <= threshold` → **Present**
- `lateness < -drop_threshold` → **Drop**（表示期限を大きく過ぎたフレームは破棄して追従）

閾値は `SyncThresholds` で設定可能（既定: early/drop = ±40ms 程度、呼び出し側がフレームレートから決定）。

## Considered Options

- **Option A — 閾値 3 判定（採用）**: ffplay 型。単純・決定論的・テスト容易。遅延フレーム破棄で追従回復する
- **Option B — 再生速度変調（動画を早送り/遅延で同期）**: フレーム単位の補間が必要で複雑。ベースライン「音声サンプル時刻優先」の原則からも音声マスター固定が筋
- **Option C — 全フレーム Present（遅延無視）**: 長時間遅延で映像が音声から乖離し続ける。不採用

## Consequences

- エンジンは `SyncDecision` に従い frame を表示/スキップ。`Wait` の待機実行は呼び出し側（表示スケジューラ）責務 — エンジンは判定のみ
- シーク後は最初の判定前にスケジューラをリセット（基準 PTS を再同期）

## Verification

- [x] ベースラインの audio master + PTS 追従方針と整合
- [x] 判定が純粋関数（時刻入力のみ・副作用なし）でテスト可能
