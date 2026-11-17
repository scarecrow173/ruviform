# Issue #727 bounded non-RT job executor — 実装実験記録（2026-11-17）

`docs/tasks/issue-727-job-executor.md` の slice（bounded 非 RT
job executor primitive の責務判断決裁 + API 設計 + stale 表記
の最小修正）で試行・観測した事項の記録。実装結果の正本は
[`docs/impl/ir/issue-727-job-executor.md`](../ir/issue-727-job-executor.md)、
イベントログは
[`issue-727-job-executor-exp.jsonl`](issue-727-job-executor-exp.jsonl)
を参照。

## 試行 1: 既存 `ruviform-exec` primitive の契約確認

- **目的**: bounded executor が既存 primitive で代替可能か、
  または意図的に非提供かを確認する。
- **観測**: `crates/ruviform-exec/src/lib.rs` 冒頭は
  「no standalone thread-pool or executor API is provided」
  と明記。`src/job.rs` は `OfflineJob`（1 job = 1 専用
  thread）を提供し、registry/executor/永続化はアプリ責務と
  記述。`src/offline.rs` の `OfflineDriver::run_parallel` は
  呼び出しごとに `std::thread::scope` の scoped worker を
  生成し永続 pool を持たない。`CancellationToken`・
  `Progress`・`JobScope`・`JobState`/`JobEvent`/`JobOutcome`/
  `JobError` は job/driver 共用の契約として存在する。
- **結論**: 既存 primitive は「1 job の実行面」と「chunk
  driver」のみで、複数 job を bounded に受け付ける executor
  は意図的に非提供。sv#518 の要求（async scheduler・
  preemption・結果 join）を満たすには新規 primitive が必要。

## 試行 2: consumer 要件の read-only 照合

- **目的**: sv#518 が要求する最小 API と、Ruviform 側に
  置くべき契約の範囲を確定する。
- **観測**: `synvelis/crates/synvelis-render/src/render_job/
  orchestrator.rs` は `RenderExecutor` port（注入可能）・
  同期 `run()`/`drive()`・`ruviform_exec::CancellationToken`
  を使用し、`synvelis-render/Cargo.toml` は既に
  `ruviform-exec` に依存。`audio-lens/crates/al-job/` は
  JobDefinition/registry/runner を持つが `ruviform-exec`
  利用は smoke test のみで、潜在 consumer として記録に留める。
- **結論**: consumer が必要とするのは bounded 実行機構
  （worker pool・queue・cancel・join）であり、Job Registry・
  job ID・永続化・retry/resume は synvelis 側責務
  （synvelis-driven 実行境界 ADR Decision 6 と整合）。

## 試行 3: 配置判断の評価（4 選択肢）

- **目的**: Ruviform 側採用 vs consumer 側採用、および
  Ruviform 側の場合の配置先を決める。
- **観測**:
  - **Option 1（ruviform-exec module 追加）**: 汎用実行契約
    = Ruviform（Decision 6）に整合。依存方向・RT contract・
    consumer 依存が既存 `ruviform-exec` と完全一致。
    module-first 原則に適合。
  - **Option 2（新規 crate）**: 独立 compile/OS/配布境界の
    必要がなく、shared-foundations の分割判断基準に非該当。
    `ruviform-exec` の既存 primitive への依存が発生し分離の
    利益が薄い。
  - **Option 3（consumer 側実装）**: bounded executor の
    実行機構自体は consumer 非依存の汎用契約。Synvelis が
    自前で持つと将来の第 2 consumer で重複実装と境界移行
    コストが発生。既存 `CancellationToken`/`Progress`/
    `JobScope` との整合も consumer 側で別途維持が必要。
  - **Option 4（OfflineJob pool 化）**: 「1 job = 1 専用
    thread」契約を変更し、既存 consumer（headless CLI 等）の
    前提を崩す。queue 満杯で `spawn` が block する可能性が
    生じ、単純な利用経路に executor lifecycle 管理を強いる。
- **結論**: Option 1 を採用。新規 crate・consumer 側実装・
  `OfflineJob` pool 化は却下（ADR に pros/cons 記録）。

## 試行 4: preemption の定義

- **目的**: sv#518 が要求する「preemption」を正直に定義する。
- **観測**: 任意 Rust コードの強制中断は Rust の実行モデル
  上安全に提供不能（unsafe 強制割り込みは `AGENTS.md` の
  unsafe ポリシーに反する）。既存 `OfflineJob`/`OfflineDriver`
  の cancel は協調（`CancellationToken` を body が観測）。
- **結論**: 「preemption」= 協調 cancel（`QueuedJob::cancel()`
  + body 側 checkpoint）+ queued job の実行 skip + worker-
  level scheduling（bounded pool の dequeue 順）として定義。
  強制中断は提供しないことを contract で明示し、将来必要に
  なれば別途 ADR で検討する。

## 試行 5: baseline stale 表記の扱い

- **目的**: shared-foundations L282 が指摘する baseline の
  「スレッドプール／ワーカープール」表記との整合を記録する。
- **観測**: baseline `ruviform-architecture-design-baseline.md`
  の L262（クレート表 `ruviform-exec` 行）・L375
  （ワーカースレッドプール）・L415（スレッド構成図）が
  Issue #24 実装（`run_parallel` ごとの scoped worker、
  永続 pool なし）より前の記述。shared-foundations L282 の
  指摘自体は正確だが、引用行番号（L240/L352/L392）が
  baseline の現在の行番号（L262/L375/L415）とずれていた。
- **結論**: shared-foundations L282 の stale 行番号参照を
  最小修正（L262・L375・L415 に更新 + ADR 参照追加）。
  baseline 本文の「ワーカースレッドプール」表記の改訂は
  本 slice の範囲外 — ADR に「改訂必要・baseline 改訂
  Issue に委ねる」と結論を記録した。

## 試行 6: doc_status lint 前後差分

- **目的**: 新規文書が新たな finding を追加しないことを確認。
- **観測**: baseline（main）で adr/designs/specs/plans/tasks/
  discovery/test-specs 各 dir に既存の error/warning が多数
  存在（invalid-id-format・broken-relation・invalid-status
  等）。新規文書追加後の lint で、新規文書由来の finding は
  index 未同期由来の `index-missing-entry`/`orphan-index`/
  `orphan-navigation` warning のみ（Coordinator が統合時に
  index 再生成で解消）。`traceability-missing-upstream`
  （spec）と `inconsistent-reciprocal-relation` は issue-721/
  issue-716 の先例と同型の許容 warning。
- **結論**: 新規文書が新たな blocking finding を追加していない
  ことを確認。`evidence/rf727/` に baseline/post の lint 出力を
  記録。

## 残リスク

- API 契約の実装時確定事項（queue 具象型・result 配送・
  `JobState` queued 表現・worker naming/priority）は実装
  Issue で確定 — 設計 doc に Deferred として記録済み。
- blocking `submit` の worker-thread reentrancy — contract で
  禁止を明記済み。`try_submit` 併用で回避可能。
- route_graph baseline blocked（既存 broken-relation・
  focus-required）は本 slice の問題ではない — baseline を
  証拠化して区別。
- docs index 未同期 — Coordinator が統合時に再生成。

## 検証証拠

- `evidence/rf727/doc-status-lint-*-baseline.txt` — 変更前の
  doc_status lint 出力（main baseline）。
- `evidence/rf727/doc-status-lint-*-post.txt` — 変更後の
  doc_status lint 出力。
- `evidence/rf727/route-graph-baseline.json` — route_graph
  baseline 出力。
- `evidence/rf727/verification.md` — 検証 lane の実測記録
  （fmt-check・prek・adr scripts・route_graph post）。
- canonical 検証: `mise run`/`node` 各コマンドの exit code は
  `docs/impl/ir/issue-727-job-executor.md` Validation と
  `evidence/rf727/verification.md` に記録。
- 補助証拠（canonical = mise）: cargo 直接実行は未使用
  （docs-only slice）。
- NotRun: `*-msrv`・`*-full`・3 OS 横断・cargo 系 lane
  （docs-only・POLICIES.md §5）。Unsupported: なし。
