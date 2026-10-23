# ruviform-exec

Ruviform メディア基盤の非 RT 実行基盤: スレッドプール・ジョブ並列・
進捗/キャンセルトークン・バッチ駆動。RT プリミティブは持たない。

## 責務

- スレッド優先度機構（「OS スレッド詳細を隠す」責務の一部として所有）:
  `promote_current_thread_to_realtime`, `demote_current_thread_from_realtime`,
  `ThreadPriorityParams`, `ThreadPriorityGuard`, `PriorityError` — OS 提供の
  リアルタイムスケジューリング（Windows MMCSS / macOS time-constraint /
  Linux `SCHED_FIFO`）を `non-RT only` の自己昇格 API として公開する（Issue #21）。
- オフライン実行ドライバ（`offline` モジュール、Issue #24）: `OfflineDriver` が
  `Processor`/`RtProcessor` カーネルをチャンク駆動で逐次・並列実行し、
  `Progress`・`CancellationToken`・有界キューのバックプレッシャを提供する。
  ベースラインの発行規則が許す `RtContext` mint サイトでもある
  （決定性保証は `offline` モジュールのドキュメント参照）。

## 層・許可依存

- 層: Primitives（設計ベースライン クレート表 A.2）
- 許可依存: Primitives（`ruviform-math`, `ruviform-rt`）。現在使用中は `ruviform-rt`。
- `audio_thread_priority` は非公開の実装詳細 — その型・エラー・feature 名は
  公開 API に現れない。
- `ruviform-media-core` との相互依存は禁止（同階層依存禁止）。

## RT 契約

非 RT 専用。RT パスからの呼び出しは契約違反。特に `promote_*` / `demote_*` と
`ThreadPriorityGuard` の生成・破棄は OS syscall を伴う — 昇格は RT 契約区間の
開始前（`RtContext` の mint と最初の `process` 呼び出しの前）に行う。
全公開 API は `RT-suitable`（純粋値型）または `non-RT only`（昇格機構・
オフライン駆動）の `**RT contract:**` ラベルを持つ
（RT ADR の API ごと保証規則）。

## 関連

- 設計ベースライン: docs/designs/ruviform-architecture-design-baseline.md（クレート表 A.2、「RT／非 RT の分離方式」）
- RT 基盤実装 ADR: docs/adr/rt-foundation-primitives-implementation.md
- Issue #24 オフライン実行ドライバ: docs/designs/offline-execution-driver.md, docs/adr/offline-determinism-scope.md
