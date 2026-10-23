# ruviform-video

Ruviform メディア基盤の動画エンジン（Issue #27）。
codec 実装・UI は持たず、エンジンは `ruviform-media-io` の境界 trait を駆動する。

## 責務

- `FramePipeline` / `VideoStage` — フレームパイプライン
- `FrameScheduler` / `SyncDecision` / `SyncThresholds` — A/V 同期判定
  （audio master + PTS 追従）
- `VideoSession` — デコードループ駆動とシーク調停（demux seek → decoder flush →
  再同期）

## 層・許可依存

- 層: Engines（設計ベースライン）
- 許可依存: Domain foundations + Primitives + `ruviform-media-io`
  （現行 Cargo.toml: `ruviform-rt`, `ruviform-media-core`, `ruviform-video-core`,
  `ruviform-media-io`; テスト用に `ruviform-audio-core`）。
  `ruviform-audio`（エンジン）との相互依存は禁止 — A/V 同期は `ruviform-rt`・
  `ruviform-media-core` の共有時刻型経由。

## RT 契約

全公開 API は非 RT。A/V 同期面は `ruviform_rt::MasterClock` を非ブロッキングの
値ロードとして読むのみ — ベースラインのエンジン契約上、唯一の RT 隣接面。

## 関連

- 設計ベースライン: docs/designs/ruviform-architecture-design-baseline.md（クレート表 `ruviform-video` 行）
- エンジン設計（Issue #27）: docs/designs/video-engine.md
- A/V 同期判定ポリシー ADR: docs/adr/video-av-sync-policy.md
