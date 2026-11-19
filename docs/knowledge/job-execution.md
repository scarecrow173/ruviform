---
id: "KNOW-vo0fHDvRkKX0obNGr5Yas6"
type: "knowledge"
status: "draft"
lifecycle: "draft"
knowledge_kind: "codebase"
title: "ジョブ実行・進捗・結果受領の基盤"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/24", "https://github.com/scarecrow173/ruviform/issues/172", "https://github.com/scarecrow173/ruviform/issues/377", "https://github.com/scarecrow173/ruviform/issues/456", "https://github.com/scarecrow173/ruviform/issues/727", "https://github.com/scarecrow173/ruviform/issues/744"]
  informs: []
  explains: []
  supersedes: []
---

# ジョブ実行・進捗・結果受領の基盤

本書は、ジョブの実行・進捗報告・結果受領に関する基盤について、過去の実装記録から抽出した再利用可能な設計知見をまとめたものである。対象となった実装記録は後に削除されたため、本書はそれらに依存せず単独で読めるように記述する。各知見の出典は Issue 番号で示す（例: Issue #727）。記録に残る設計判断とその理由は当時の解釈を含むため、観測された事実・契約として確認できる事項と、判断の根拠として記録された解釈を区別して読む必要がある。

## bounded executor の配置決定

汎用の bounded 非 RT job executor primitive は、`ruviform-exec` への module 追加として決裁された。判断は ADR（accepted）として記録されている（Issue #727）。

配置の根拠として記録されたのは三点である。汎用実行契約の所在は Ruviform と synvelis の実行境界 ADR の Decision 6 で確定済みであり、sv#518 が最初の実需として実在して shared-foundations の「汎用利用者の実在」トリガを満たしたこと、そして依存方向・RT contract・consumer 依存が既存 `ruviform-exec` と一致することである（Issue #727）。API 契約の詳細は design doc に分離し、ADR と併用する構成を採った（Issue #727）。

却下された案とその理由は次の通りである。新規 crate は独立した compile・OS・配布境界の必要がなく module-first 原則に反し、`ruviform-exec` への依存で分離利益も薄いため却下された。consumer（synvelis）側への実装は汎用実行契約の所在を誤り、将来の第2 consumer で重複実装と境界移行コストが発生するため却下された。`OfflineJob` の pool 化は「1 job = 1 専用 thread」契約を壊し、queue 満杯で `spawn` が block する可能性が生じるため却下された（Issue #727）。

## consumer 側の責務境界

synvelis の `orchestrator.rs` は `RenderExecutor` port（注入可能）・同期 `run()`/`drive()`・`ruviform_exec::CancellationToken` を使用し、Cargo.toml で既に `ruviform-exec` に依存している（Issue #727）。

consumer が必要とするのは bounded 実行機構のみであり、Job Registry・job ID・永続化・retry/resume は synvelis 側の責務とされた。これは Decision 6 と整合する境界である（Issue #727）。audio-lens の `al-job` は `ruviform-exec` の利用が smoke test のみであり、潜在 consumer として記録に留まる（Issue #727）。

## 既存 ruviform-exec の契約

`crates/ruviform-exec` は lib.rs 冒頭で「no standalone thread-pool or executor API is provided」と明記しており、提供するのは `OfflineJob`（1 job = 1 専用 thread）と `OfflineDriver::run_parallel`（呼び出しごとの scoped worker で永続 pool なし）のみである。`CancellationToken`/`Progress`/`JobScope`/`JobState`/`JobEvent`/`JobOutcome`/`JobError` は job/driver 共用契約として存在し、bounded executor は意図的に非提供だったことが確認されている（Issue #727）。

baseline 文書の「スレッドプール／ワーカープール」表記は Issue #24 実装（scoped worker、永続 pool なし）より前の記述であり、指摘自体は正確だったが引用行番号が現在とずれていた。行番号参照のみ最小修正して ADR 参照を追加し、baseline 本文の改訂は範囲外として別 Issue に委ねる結論が記録されている（Issue #727）。

## preemption の定義

sv#518 が要求する「preemption」は、「協調 cancel（`QueuedJob::cancel()` + body 側 checkpoint）+ queued job の実行 skip + worker-level scheduling（bounded pool の dequeue 順）」として定義された（Issue #727）。

任意の Rust コードの強制中断は Rust の実行モデル上安全に提供できず、unsafe による強制割り込みは unsafe ポリシーに反するため、強制 preemption は Deferred（別途 ADR が必要）とされた（Issue #727）。

## 進捗報告における detail の契約

`JobContext::progress_detail(done, total, detail)` が新設され、detail は report 単位の添付で後続 report へ持ち越されない契約（per-report、sticky 状態なし）とされた（Issue #172）。

sticky detail（producer 側の `set_detail` や runtime 側の `JobRecord` 保持）は却下された。却下理由として記録されたのは、「detail を持たない report が続く場合の意味」と消去 API・内部状態を新たに要求するためである。stage 名の永続表示は「最新の detail 付き event を保持する」consumer 側の責務として文書に明記された（Issue #172）。

wire 形式では、`Event::Job` に `detail: Option<String>` を `skip_serializing_if` 付きで additive に追加し、detail を持たない report ではキー自体を emit しない。`SCHEMA_VERSION` は v1 のまま据え置かれ、Intent 方向の parse 契約は不変である（Issue #172）。

`ruviform-exec` 側の `Progress`/`JobEvent` は `Copy` enum で共有 atomic を前提とするため触らず、detail は report を発行する job body が知る情報なので UI 層で添付するのが正しい境界と判断された（Issue #172）。

enum variant へのフィールド追加は additive だが、`#[non_exhaustive]` であっても struct literal で全 field を指定する構築コードはコンパイルエラーになる。本 workspace 内の構築は `Event::job_*` constructor 経由のみで影響がなく、consumer 側は deserialize/参照のみで無影響と確認された。合体 slot（latest map）に detail を同梱したことで、「最新 report の detail が残る」挙動が drain バッチを跨いで自然に成立する（Issue #172）。

## ジョブコマンド結果の二段通知

job command の結果通知は、「ack（従来どおり accepted を維持）」と「終端 receipt（job 終端後に onEvent で配送）」の二段に分ける設計が採用された（Issue #456）。

二回目の result を未知 id として扱う consumer が壊れないよう Spec §4 に明記され、wire 形式は既存 variant の再利用でデータ移行は不要とされた（Issue #456）。

ロールバックは runtime・decode 委譲・parity/fixture・文書の論理単位 commit に分割され、runtime commit の revert で ack のみの従来挙動へ戻せる構成とされた（Issue #456）。

decode は一元化され、`Event::Result` から `CommandReceipt` への decode を `CommandReceipt::from_result` に集約し、GPUI 側 `drain_inbox` の Result 分岐は委譲のみに限定された。decode 規則の重複実装を避けるためであり、境界は `result_events_decode_to_receipts` 単体テストで固定する方針である。render.rs の並行変更（RF#451）との競合を避けるため、変更範囲は `drain_inbox` の Result 分岐に限定された（Issue #456）。

## bounded queue の実装

bounded queue は `Mutex<VecDeque<JobTask>>` と二つの `Condvar`（`has_job`・`has_space`）で実装された。`std::sync::mpsc::sync_channel` は容量超過時の blocking send が shutdown で起床できないため却下された（Issue #744）。

単一 mutex で queue と shutdown flag を直列化し、「push された task は必ず `live` 登録済み」の不変条件を一箇所で保証する。worker は pop 時に `has_space.notify_one`、submit は push 時に `has_job.notify_one`、shutdown 時は `has_space.notify_all` で parked submitter を `SubmitError::Shutdown` で起床させる（Issue #744）。

## JobState・JobError の additive 拡張

`JobState::Queued`（=3）が追加され、#727 契約の「queued と実行中を区別可能」を直接満たした。`JobError::Cancelled` は、queued cancel で body が走らない job の `wait()` が `T`/`E` を捏造できないため、正直な結果 variant として追加された（Issue #744）。

`#[non_exhaustive]` のため additive に安全であり、`OfflineJob` は `Queued` を報告せず `Cancelled` に到達不能である。`state_from_u8`/`request_cancel` は `pub(crate)` 関数として `OfflineJob`/`QueuedJob` で共有され、state 語彙が一箇所に集約された（Issue #744）。

## 結果公開の順序と try_wait の race 回避

pool では worker が state を store しても result は channel 経由のため、state が先に見えると `try_wait` の `try_recv` が `Empty` を返し得る。この race を避けるため、`run_job` で `result_tx.send(result)` → `state.store(FINISHED, Release)` の順序が固定され、`try_wait` は Finished 観測後に `try_recv` する。これにより result が必ず先に channel にあることが保証される。`Empty` は unreachable として receiver 保持 retry とし、`Disconnected`（内部 worker fault）は `Panicked` に写像した（Issue #744）。

## panic 隔離と shutdown の直列化

body の panic は `catch_unwind` で `JobError::Panicked` に写像され、さらに task 全体を `catch_unwind` で包むことで、delivery path 自体の panic でも pool worker が死なないようにした（Issue #744）。

`Inner::shutdown` は workers mutex の guard を join 中も保持し、並行呼出の二つ目が worker 完了前に空 vec で return しないよう直列化した（Issue #744）。

live bookkeeping（`Mutex<HashMap<u64, CancellationToken>>`）は shutdown 時の一括 cancel 専用であり、job 完了時に除去される。product の Job Registry ではないと明記されている（Issue #744）。

## deadlock contract

blocking `submit` を pool worker 上の job body から呼ぶと deadlock し得ること、および `shutdown`/`Drop` の job body 内呼出も自己 join で deadlock することが、型と doc に明記された（Issue #744）。

#727 で contract 上禁止済みの worker-thread reentrancy は `try_submit` 併用で回避可能であり、実装側でも明示する運用知識とされた（Issue #744）。

## ViewLink の authoritative generation seam

`ViewLink.attach(windowId, generation?)` に authoritative generation seam が追加された。指定値は非負 safe integer のみ受理され（無効値は RangeError・状態不変）、省略時は既存の increment を維持する（Issue #377）。

同 window 同値は冪等 no-op、同 window 異 epoch は unmount detach→再 attach、異 window は dock-transfer detach→指定 epoch 採用となる。CommandBinding/QueryBinding の `attach(epoch?)` と subscription context は既存経路で伝搬し（#379）、Issue #377/#378 は混ぜない範囲限定とされた（Issue #377）。

## 文書運用上の知見

`docs/*/README.md` の index は Coordinator が統合時に再生成する運用のため、個別の作業 slice では一切触れない（Issue #727）。

route_graph は main 上に既存の `broken-relation:*` が多数ある baseline blocked 状態にある。新規文書が新たな finding を追加しないことを前後差分で確認する手法が採られ、baseline は `evidence/rf727/` に記録された（Issue #727）。

## 未確定・保留事項

記録には、次の未解決事項・未確認事項が残されている。

Issue #456 の spec/design/test-spec 文書（docs/specs・docs/designs 配下）は未読であり、二段 receipt の契約詳細は spec 側にある可能性がある。

Issue #744 の consumer 配線（synvelis#518 の RenderExecutor port への注入）は範囲外と明記されており、完了状況は不明である。

Issue #727 の「JobState queued 表現」は実装 Issue（#744）で `Queued=3` として確定済みだが、worker naming/priority・queue 具象型の最終確定は design doc 側の Deferred 記述に依存し、designs 文書自体は未読である。

ViewLink の対象文書は synvelis 側リポジトリの文書であり、対象コードは本 workspace 外の可能性がある。実装本体は未確認である。
