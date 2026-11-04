---
id: "DISC-UsHnmyyijB53IvR586RquN"
type: "discovery"
status: "resolved"
title: "Issue #311 RF-GT-02 IPC schema conformance / deterministic record-replay / fault matrix — contract inventory"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/311"
    - "https://github.com/scarecrow173/ruviform/issues/257"
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
  related:
    - "docs/designs/issue-156-gui-host-foundation.md"
    - "docs/designs/issue-181-multi-window.md"
    - "docs/designs/issue-182-dock-tree.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "docs/specs/issue-311-ipc-conformance-replay.md"
    - "docs/designs/issue-311-ipc-conformance-replay.md"
  verifies: []
  verified-by: []
  references:
    - "crates/ruviform-ui/src/ipc.rs"
    - "crates/ruviform-ui/src/data.rs"
    - "crates/ruviform-ui/src/bridge.rs"
    - "crates/ruviform-ui/src/runtime.rs"
    - "crates/ruviform-ui-webview/src/scheme.rs"
    - "ui/core/src/interaction.ts"
    - "ui/binding/src/contract.ts"
    - "ui/binding/src/binding.ts"
    - "ui/binding/src/coalesce.ts"
    - "ui/binding/src/query.ts"
    - "ui/binding/src/link.ts"
  defers: []
  deferred-by: []
---

# Issue #311 RF-GT-02 — IPC 契約棚卸し

Issue #311 が検証対象として名指しする既存契約を、実コードと照合して列挙する。
本書は「発見」文書であり、新契約の提案は行わない（spec で扱う）。

## 1. `rvf.ipc.v1` — Rust↔フロントエンド wire（`crates/ruviform-ui/src/ipc.rs`）

**方向:** 双方向で単一の versioned envelope を共有。

- Frontend→Rust: `Intent` JSON over `window.ipc.postMessage`（文字列ペイロードのみ、wry `with_ipc_handler` 契約）。
- Rust→frontend: `Event` JSON を `evaluate_script` 経由で `window.__rvf.dispatch` に注入（`bridge.rs::dispatch_script`、JSON を二重エンコードして JS 文字列リテラル境界を越える）。

**型（正本）:**

| 要素 | 内容 |
| --- | --- |
| `SCHEMA_VERSION` | `1`（envelope の `"v"` フィールド） |
| `MAX_INTENT_BYTES` | 1 MiB。超過は `message_too_large`。bulk/binary は `rvf://data` pull 経路 |
| `MAX_OP_LEN` | 128。`ns.name` 形、`[a-z0-9_.-]`、各セグメント非空（`valid_op`） |
| `codes` | `malformed_message`/`message_too_large`/`unknown_op`/`invalid_params`/`unknown_job`/`service_error`/`worker_failed`/`cancelled`/`unavailable`（安定文字列） |
| `ErrorBody` | `{code, message, details?}`。`details` は additive optional |
| `Intent` | `{v, kind:"intent", id, op, params?}`。`params` default は `null` ではなく省略可（serde default） |
| `Event` | `kind` タグで `result`/`job`/`notify`/`lifecycle`/`error`。`Envelope{v, ...flatten}` |
| `JobState` | `progress`/`finished`/`cancelled`/`failed`（snake_case、non_exhaustive） |
| `LifecycleState` | `closing`（non_exhaustive、拡張余地） |

**相関・順序契約:**
- 1 Intent → ちょうど 1 つの `result`（`id` 相関）。長時間呼出しは別 job id で `job` イベントを追加発行。
- `notify` は latest-wins（`seq` 単調増加、受信側は最新のみ保持、eval push は coalesce され得る）。
- `job` progress は bounded mailbox 経由で latest-wins coalesce（`detail` は Issue #172 の additive 拡張で、未対応 consumer との互換を保つ）。terminal（finished/cancelled/failed）は drop しない。
- パース不能な body は相関 id を持たない uncorrelated `error` イベント。

**未認識要素の扱い:**
- `v != 1`、`kind != "intent"`、不正 `op` 形は `malformed_message` で拒否。
- `Intent` の未知フィールドは serde default で無視される（`deny_unknown_fields` なし）。
- 受信側（Rust）の `Event` デシリアライズ経路は存在しない（Serialize のみ）— frontend は `__rvf.dispatch` で素の JS オブジェクトとして受け取る。

**JS 側 bridge（`bridge.rs::BRIDGE_JS`）:** `call(op, params)→Promise`、`onEvent(cb)`、`dataUrl(name, offset?, len?)`、`dispatch(m)`。id は `i-<seq>` で採番、`result.ok` で resolve / `error` で reject（`e.code` を伝播）。`dataUrl` は `rvf://data/<name>?offset=N&len=M` を構築。

## 2. `rvf://data` — bounded binary pull（`data.rs` + `scheme.rs`）

- `DataRegistry`: named `DataSource`（`len`/`read`）を共有登録。`max_chunk` 既定 16 MiB/pull、`max_sources` 任意 LRU 上限。`unregister`/`clear`/LRU eviction で解放、in-flight read は `Arc` で生存。
- `DataRequest{name, offset, len?}` → `DataChunk{bytes, total}`。`offset > total` → `DataError::OutOfRange`、`offset == total` → 空本体。`len` は cap と残りで clamp。
- Wire（`scheme.rs::serve_data`）: `rvf://data/<name>?offset=N&len=M`。`offset` default 0、`len` 省略は「cap まで」。未知クエリキーは無視、`=` なし pair は 400。name はマルチセグメント可だが空セグメント・`..` は 400。Windows は `http://rvf.data/...` に書き換えられるため `rvf.` host prefix を剥がす。
- 200 応答: `Content-Type: application/octet-stream`、`Access-Control-Allow-Origin: *`、`Cache-Control: no-store`、`X-Rvf-Total: <total>`（paging 終端子）。client fault→4xx、unregistered→404、source failure→500。

## 3. Binding 契約 — TS 側（`ui/binding/src/*`、Issue #261）

`rvf.ipc.v1` の上に乗るのではなく、**frontend 内の headless 契約**（UI intent ↔ consumer `BindingPort`）。Rust 側に対応型は存在しない。

**Command（contract.ts）:**
- `CommandIntent` union: `begin`/`preview`/`commit`/`cancel`/`action`。共通基底 = `target`(LogicalTarget), `controlId`, `sequence`, optional `gestureId`/`originView`/`atomic`。
- 配送クラス: `preview` のみ droppable（`coalescible: true`）。`begin`/`commit`/`cancel`/`action` は non-droppable。
- `intentFromGesture`: `unavailable`/`rejected`/`noop` は local-only で port に届かない。target 解決不能でも `undefined`。
- `CommandReceipt{commandId, status, revision?, detail?}`、`ReceiptStatus = accepted|queued|applied|rejected|unsupported`。
- `StateProjection{revision, value, provenance?}` — `revision` が順序トークン（数値比較 or 異なる identity は supersede）。
- `CommandPhase`: `idle|preview|pending|accepted|applied|observed|rejected|unsupported|timeout|stale`（`BindingViewState` + `observedSeq`/`pendingCommandId`/`timedOut`/`generation`/`attached`）。
- `ViewportQuery{queryId, sourceId, sourceRevision, viewId, generation, start, end, buckets?, channels?, maxBytes?, encoding?}`、`QueryReply{generation, sourceRevision, payload, bytes?}`、`QueryErrorKind = cancelled|stale|over-budget|unsupported|error`、`DEFAULT_QUERY_MAX_BYTES = 16 MiB`。
- `BindingPort{dispatch, subscribe?, query?, cancelQuery?}`、`DispatchContext{commandId, viewId?, windowId?, generation, issuedAt, expectReceipt}`、`SubscriptionContext{viewId, windowId?, generation}`。

**CommandBinding（binding.ts）:**
- commandId は `cmd-N` 連番。`expectReceipt` は `commit`/`action` のみ。
- receipt 順序ランク: in-flight(0) < accepted/queued(1) < applied(2) < terminal(3: rejected/unsupported)。逆順・重複は吸収して計数、unknown/dead generation は stale。
- ack timeout: `ackTimeoutMs` 超過で `timeout` phase（明示失敗、捏造した成功でない）。timeout 後の遅延 applied は受理するが `timedOut` フラグが残る。
- projection guard: revision 前進のみ受理、stale/duplicate は計数。
- detach: in-flight gesture を cancel コマンドとして（旧 generation で）発行、キューを purge、generation++。dead epoch の receipt/projection は stale。attach で head/activeGesture をリセット。
- `TRACKED_LIMIT = 64` で tracked command を bound（receipt-expecting in-flight は退避しない）。
- `BindingDiagnostics`: intents/dispatched/coalesced/supersededPreviews/staleReceipts/outOfOrderReceipts/staleProjections/detachedIntents。

**IntentQueue（coalesce.ts）:** preview のみ同一 gesture 内で latest-wins。tombstone 方式で順序保持。`commit`/`cancel` で pending preview は superseded。統計は received/emitted/coalesced/superseded。

**QueryBinding（query.ts）:** `queryId = q-<viewId>-<seq>`。`maxInflight` 既定 4（超過で最古を cancel-evict）。reply は generation 二重チェック + `sourceRevision` echo + optional `revisionProbe` で stale 判定、`bytes > maxBytes` は over-budget。cancel は AbortSignal + `cancelQuery` 二経路。detach で generation++ かつ全 cancel。

**ViewLink / LinkRegistry（link.ts）:** `viewId` は window を跨ぐ論理実体。`attach(windowId)` で generation++ + 全 binding を再 subscribe（即時 projection push 契約）。`detach(reason)` は subscription 解放・gesture cancel・query cancel、ドメイン Job/Project/Engine には触れない。同一 window 再 attach・二重 detach は no-op。`closeView` は detach + 登録抹消。

**InteractionControl（ui/core interaction.ts、Issue #258）:**
- `ControlInput`: begin/preview/commit/cancel/set/adjust/reset/focus-lost/detach。
- `InteractionEvent`: begin/preview(commit/cancel 不可の `coalescible`)/unavailable/rejected/noop。meta は `controlId/target?/originView?/gestureId?/sequence/source?/atomic?`。`sequence` は control 単位で 1 始まり単調。
- `seq` 入力順序: 重複 `seq` → `noop(duplicate-input)`、逆行 → `rejected(out-of-order)`。lifecycle 入力（focus-lost/detach）は drop せず必ず評価。
- 原子的 `set`/`adjust`/`reset`（gestureId なし）は `atomic-1` 等の合成 id で begin+commit 対（`atomic: true`）。
- `detach` 既定 policy は cancel。terminated gesture id は `terminatedGestureHistory`（既定 32）で stale 判定。

## 4. Rust 側 Controller — `UiRuntime`（runtime.rs + service.rs + event.rs）

- `HostEvent`: `Message(String)`/`Tick`/`CloseRequested`/`Destroyed`/`Ready`/`Resized`/`Focused`/`ScaleFactorChanged`/`Moved`。
- `UiRuntime::handle`（single-window）と `handle_window(window, ev)`（multi-window routing、`RoutedEvent{window, event}`）。両者を混ぜない契約。
- 組込み op: `sys.ping`/`sys.capabilities`/`job.cancel`。`sys.*`・`job.cancel` は service 登録不可。
- `Service{ops(), call()}` → `CallOutcome::Result(Value)` or `Job(JobSpec)`。`JobSpec.execute` は worker thread で `catch_unwind`、panic は `worker_failed`。
- job mailbox: `JOB_MAILBOX = 256` bounded sync channel。progress は try_send（backpressure で drop = latest-wins）、terminal Finished は `send`（block）。
- `CloseRequested`: `handle` では全 job cancel + `lifecycle{closing}`、`handle_window` では `lifecycle{closing}` のみ（job cancel しない — `cancel_window_jobs` が opt-in の別語彙）。`Destroyed` は runtime ではイベントを生産しない（配信層が drop）。
- `cancel_job`: `job` param 数値必須 → `invalid_params`、未実行 job → `unknown_job`、受理 → `{job, cancelling:true}`。

## 5. 観測したギャップ（spec への入力）

1. **TS 側に `rvf.ipc.v1` の型付き decode/validation が存在しない。** BRIDGE_JS は素朴な分岐のみ（`m.kind === 'result'` 以外は listener へ素通し）。fixture による双方向 conformance を取るための検証器が必要。
2. **Binding 契約の wire 表現検証器も存在しない。** `CommandIntent`/`CommandReceipt`/`StateProjection`/`ViewportQuery`/`QueryReply` は TS 型としては定義済みだが、外部データ（fixture/記録）を検証するランタイム入口がない。
3. **Record/Replay 基盤が存在しない。** `mocks.ts` はスクリプト駆動のテスト用 consumer だが、記録・交換形式・注入・独立 oracle は未実装。
4. **fault matrix は部分的に散在する。** binding.test.ts が reordered/duplicate/stale/timeout/detach を、query.test.ts が stale/over-budget/cancel を、link.test.ts が attach/detach/close を個別カバー。`disconnect/crash`（Rust 側では worker panic → `worker_failed`）や `backpressure`（JOB_MAILBOX 溢れ・preview coalesce 境界・`TRACKED_LIMIT` 退避）、`overlapping gesture`、`window destroy`（`Destroyed`/dead-window 配信 drop）、`unsupported` の網羅表としては未整備。
5. **`rvf://data` の TS 側対称物は `dataUrl` 構築のみ。** 応答の `X-Rvf-Total`/bounded paging 契約を検証する TS 側器がない。

## 6. 境界確認（Issue 非対象の再確認）

- `rvf.ipc.v1` に新しい kind や第二正本を追加しない。製品独自 Gateway を Ruviform に新設しない。
- Synvelis の Project/Authorization/Grant、AudioLens の Dataset oracle は各 Consumer 所有。
- 常時稼働の Observability/Event Store は作らない（Recorder は opt-in テスト基盤）。
- 実 DOM/OS WebView の操作・描画検証は RF-GT-01(#310)/RF-GT-04(#313)。
