# Issue #744 bounded 非 RT JobExecutor — 実装実験記録（2026-11-17）

`docs/tasks/issue-744-job-executor.md` の slice（#727 approved
API 契約の `ruviform-exec` 実装）で試行・観測した事項の記録。
実装結果の正本は
[`docs/impl/ir/issue-744-job-executor.md`](../ir/issue-744-job-executor.md)、
イベントログは
[`issue-744-job-executor-exp.jsonl`](issue-744-job-executor-exp.jsonl)
を参照。

## 試行 1: queued 状態・結果表現の設計確定

- **目的**: #727 契約の「queued と実行中を区別可能」「queued
  cancel の結果」を既存型の additive 変更で満たせるか確認。
- **観測**: `JobState` は `#[non_exhaustive]` + `AtomicU8`
  encoding（`STATE_RUNNING=0/CANCELLING=1/FINISHED=2`）。
  `JobError<E>` も `#[non_exhaustive]` で
  `Operation(E)`/`Panicked` の 2 variant。body が走らない
  job の `wait()` は `Result<T, JobError<E>>` を返す必要が
  あり、`T`/`E` を捏造できない。
- **結論**: `JobState::Queued`（=3）と `JobError::Cancelled`
  を additive に追加。`state_from_u8`/`request_cancel` を
  `pub(crate)` 関数として `OfflineJob`/`QueuedJob` で共有し、
  state 語彙の一貫性を一箇所に集約した。

## 試行 2: bounded queue の同期機構選択

- **目的**: `submit`（blocking）と `try_submit`（fail-fast）を
  単一の bounded 構造で実現し、shutdown で parked submitter
  を起床させる方法を確定。
- **観測**: `std::sync::mpsc::sync_channel` は容量超過時に
  blocking send するが、受信側が shutdown しても送信側を
  起床させる経路がない。`Mutex<VecDeque> + Condvar` なら
  queue と shutdown flag を同一 mutex で直列化でき、
  `has_space.notify_all` で parked submitter を
  `SubmitError::Shutdown` で起床できる。
- **結論**: `Mutex<VecDeque<JobTask>> + Condvar(has_job)
  + Condvar(has_space)` を採用。worker は pop 時に
  `has_space.notify_one`、submit は push 時に
  `has_job.notify_one`。

## 試行 3: result publish と try_wait の順序契約

- **目的**: `try_wait` が `Finished` state を見て result を
  `try_recv` する設計で、result 未到着の race が起きないか
  確認。
- **観測**: `OfflineJob` は worker thread の join 後に result
  を取り出すため順序問題がない。pool では worker が state を
  store しても result は channel 経由なので、state が先に
  見えると `try_recv` が `Empty` を返し得る。
- **結論**: `run_job` で `result_tx.send(result)` →
  `state.store(FINISHED, Release)` の順序を固定。
  `try_wait` は `Finished` 観測後に `try_recv` するため
  result が必ず先に channel にある。`Empty` は
  unreachable として receiver を保持して retry、
  `Disconnected`（内部 worker fault）は `Panicked` に写像。

## 試行 4: shutdown の並行呼出と worker join

- **目的**: `JobExecutor` が `Arc` 経由で共有される場合に
  並行 `shutdown` が deadlock/早期 return しないか確認。
- **観測**: `Inner::shutdown` が workers vec を drain して
  join する実装では、2 つ目の呼出が空 vec を得て worker
  完了前に return し得る。
- **結論**: workers mutex の guard を join 中も保持し、
  2 つ目の呼出を mutex でブロック → drain 済みの空 vec を
  得て return。全呼出が worker 完了後に return する。

## 試行 5: 検証実行と lint 適合

- **目的**: canonical 検証と clippy pedantic 適合を確認。
- **観測**: `needless_pass_by_value` が `run_job`/`worker_loop`
  の by-value 引数を指摘 → 参照渡しに変更（所有は task
  closure が保持）。`similar_names` が `submitter`/`submitted`
  を指摘 → `submit_thread` に rename。`redundant_closure`
  が `|| task()` を指摘 → `AssertUnwindSafe(task)` に変更。
  `dropping_a_handle_requests_cancellation` テストは drop が
  queued 状態に着地すると body skip（正しい `Cancelled`
  semantics）で flake → body 開始を `poll_until` で待って
  から drop するよう修正。
- **結論**: `check`/`lint`/`test`/`fmt-check` 全て Pass。
  dependent 4 crate の `check` も Pass（additive 変更に
  互換）。
