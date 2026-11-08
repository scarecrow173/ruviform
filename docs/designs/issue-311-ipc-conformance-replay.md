---
id: "DESIGN-wrF61tPcejIc31wMBYJHok"
type: "design"
status: "approved"
title: "Issue #311 RF-GT-02 ui/guitest — IPC conformance fixture・deterministic record/replay・fault matrix 設計"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/311"
    - "docs/specs/issue-311-ipc-conformance-replay.md"
  changes:
    added:
      - type: "dir"
        path: "ui/guitest"
      - type: "file"
        path: "crates/ruviform-ui/tests/ipc_conformance.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-webview/src/scheme.rs"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-311-ipc-conformance-replay.md"
  implemented-by:
    - "docs/plans/issue-311-ipc-conformance-replay.md"
  depends-on:
    - "docs/specs/issue-311-ipc-conformance-replay.md"
    - "docs/designs/issue-156-gui-host-foundation.md"
    - "docs/designs/issue-181-multi-window.md"
    - "docs/designs/issue-182-dock-tree.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-28-ui-poc.md"
    - "docs/adr/ui-architecture-webview-hybrid.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-311-ipc-conformance-replay.md"
    - "docs/discovery/issue-311-ipc-conformance-inventory.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "ui/binding/src/contract.ts"
    - "ui/binding/src/binding.ts"
    - "ui/binding/src/coalesce.ts"
    - "ui/binding/src/query.ts"
    - "ui/binding/src/link.ts"
    - "ui/core/src/interaction.ts"
    - "crates/ruviform-ui/src/ipc.rs"
    - "crates/ruviform-ui/src/runtime.rs"
    - "crates/ruviform-ui-webview/src/scheme.rs"
  defers: []
  deferred-by: []
---

# Issue #311 RF-GT-02 — ui/guitest 設計

> **歴史/旧方式（legacy）注記**: 本書の `rvf.ipc.v1` schema・versioned
> fixture corpus の契約部分は driver-neutral 経路（`WindowLifecycle::Action(Intent)`）
> でも共有・維持される。一方、`ui/guitest` の TS 実装・`window.ipc.postMessage` /
> `evaluate_script` 輸送・WebView DOM 前提の検証 lane は legacy/preview。
> 物理撤去は Issue #465 P2 の別判断。棚卸し正本:
> [issue-465-legacy-webview-inventory](../discovery/issue-465-legacy-webview-inventory.md)。

設計正本は Issue #257（GUI Testing v1.0 / RF-GT-02 行）および spec `docs/specs/issue-311-ipc-conformance-replay.md`。本書は実装選択と形式の詳細を固定する。

## 1. パッケージ配置と責務

新規 `ui/guitest` — GUI テスト共有基盤のうち RF-GT-02 分担面（IPC conformance + record/replay + fault matrix）。既存 `ui/*` と同じ規律: private package、`npm ci` lockfile 復元、pinned mise node、**dependencies ゼロ**（devDependency は typescript/@types/node のみ。happy-dom すら不要 — 本パッケージは DOM を触らない）。

```text
ui/guitest/
  package.json / tsconfig.json / README.md
  fixtures/                     # 単一 versioned fixture 集合（両側共有）
    README.md                   # 形式・バージョン・更新規則の正本
    ipc/intents.v1.json         # Intent 受理/拒否ケース
    ipc/events.v1.json          # Event envelope golden
    ipc/data.v1.json            # rvf://data 要求/応答契約ケース
    binding/messages.v1.json    # Binding 契約メッセージ検証ケース
    scenarios/binding/*.v1.json # replay シナリオ（入力列 + oracle 期待）
    scenarios/host/*.v1.json    # Rust UiRuntime シナリオ
    recordings/*.v1.jsonl       # recorder 往復用の参照記録
  src/
    ipcwire.ts     # rvf.ipc.v1 decode/validate + rvf://data URL/応答契約
    bindingwire.ts # Binding 契約メッセージ validator
    jsonl.ts       # record line 型 + parse/serialize（v1）
    recorder.ts    # opt-in/allowlist/bounded 記録器
    replay.ts      # 入力注入ドライバ + deterministic run
    oracle.ts      # 独立 ContractModel（文書化契約規則のみ符号化）
    consumers.ts   # ScriptedDawBackend / ScriptedAnalysisBackend
    seed.ts        # 決定的 PRNG（mulberry32）+ seeded シナリオ生成
    index.ts
  test/
    conformance.test.ts / record.test.ts / replay.test.ts /
    faultmatrix.test.ts / fixtures.test.ts / env.ts
```

- **既存コード再利用:** `CommandBinding`/`QueryBinding`/`ViewLink`/`IntentQueue`/`InteractionControl` と契約型は `ui/binding`・`ui/core` の src を相対 import（`ui/binding` → `../core/src` と同じ規則）。契約型の複製・再定義はしない。
- **mise レーン:** `check-ui-guitest`（npm ci + tsc --noEmit）、`test-ui-guitest`（npm ci + node --test）。`build-ui-guitest` は持たない — 本パッケージはテスト基盤であり dist 配布しない（消費者は src TS を直接 import する既存慣行に倣う）。CI `ui-verify` ジョブに check/test の 2 ステップを追加。
- **Rust 側:** `crates/ruviform-ui/tests/ipc_conformance.rs`（workspace `test` 既定 lane）と `crates/ruviform-ui-webview/src/scheme.rs` 内テストの fixture 駆動拡張（`test-ui-webview` lane）。fixture の配置は `ui/guitest/fixtures/` のみ — Rust テストは `CARGO_MANIFEST_DIR` からの相対パスで読む。fixture を crate 内に複製しない。

## 2. Fixture 形式（`"v": 1`）

全 fixture は `{"v": 1, "suite": "<name>", "cases": [...]}`。`v` は fixture 形式のバージョンで、`rvf.ipc.v1` のスキーマバージョンと独立。未知 `v` は fail-closed（新版形式を黙って読まない）。

### 2.1 `ipc/intents.v1.json`

```jsonc
{ "v": 1, "suite": "rvf.ipc.v1.intents",
  "cases": [
    { "name": "sys.ping minimal",
      "body": "{\"v\":1,\"kind\":\"intent\",\"id\":\"i-1\",\"op\":\"sys.ping\"}",
      "expect": { "ok": true, "intent": { "id": "i-1", "op": "sys.ping" } } },
    { "name": "unsupported version",
      "body": "{\"v\":2,\"kind\":\"intent\",\"id\":\"x\",\"op\":\"a.b\"}",
      "expect": { "ok": false, "code": "malformed_message" } },
    { "name": "oversized body", "bodyRepeat": ["x", 2097152], "bodyPrefix": "{...}",
      "expect": { "ok": false, "code": "message_too_large" } }
  ] }
```

- `body` は生文字列（生 JSON 断片・改行混入・非 UTF-8 相当も表現可）。`expect.ok:false` は安定 `code` のみ照合（`message` は人間可読で固定しない）。
- `expect.ok:true` は復元された `id`/`op`/`params` を照合。`params` 省略時の default 意味（Rust `serde default` ↔ TS `undefined`）を case で固定する。
- 収録カテゴリ: 正例（最小/params 付/未知 field 混在）、version 違反（0/2/非数/欠落）、kind 違反、op 形違反（大文字・先頭 digit・`.`なし・空セグメント・128 超・非 ASCII）、非 JSON、サイズ超過、構造型違反（`id` が非文字列等）。

### 2.2 `ipc/events.v1.json`

```jsonc
{ "v": 1, "suite": "rvf.ipc.v1.events",
  "cases": [
    { "name": "result ok",
      "wire": { "v": 1, "kind": "result", "id": "i-1", "ok": true, "data": {"pong": true} },
      "ts": { "accept": true, "class": "result", "id": "i-1" } } ] }
```

- `wire` は**正本 wire JSON**（serde の実際の出力形）。Rust 側はケース記述から `Event` を組み立て `serde_json::to_value(envelope)` が `wire` と完全一致することを検証（key 順は比較しない・`flatten` 展開済み形）。
- TS 側は `wire` を decoder に通し、`accept`/`class`（result/job/notify/lifecycle/error）/相関フィールド（id/job/state/seq）を照合。不正 wire（`v` 不一致・未知 kind・欠落必須 field・`result` の ok/data/error 不整合）は `accept:false` + 理由クラスを固定。
- optional 能力ケース: `job` progress の `detail` あり/なし両方、`ErrorBody.details` あり/なし — additive 拡張の双方向互換を固定。

### 2.3 `ipc/data.v1.json`

```jsonc
{ "v": 1, "suite": "rvf.data.v1",
  "registry": { "maxChunk": 8, "sources": [{ "name": "demo/peaks", "bytes": 64, "fill": 1 }] },
  "cases": [
    { "name": "bounded pull",
      "url": "rvf://data/demo/peaks?offset=10&len=4",
      "expect": { "status": 200, "bodyLen": 4, "total": 64 } },
    { "name": "offset beyond end",
      "url": "rvf://data/demo/peaks?offset=65",
      "expect": { "status": 400, "class": "out_of_range" } } ] }
```

- wire レベル（URL→status/ヘッダ/本体長）は `scheme.rs` の `serve` で照合（`X-Rvf-Total`、200 空本体境界、400/404/500 写像）。registry レベルは ruviform-ui 側で `DataRegistry::read` の `DataChunk`/`DataError` に写像（`class` が共通語彙）。TS 側は `dataUrl` 構築の往復一致 + 応答契約モデル（bodyLen ≤ min(len, total-offset, maxChunk)・`X-Rvf-Total` 終端子）を検証。

### 2.4 `binding/messages.v1.json`

`CommandIntent`/`CommandReceipt`/`StateProjection`/`ViewportQuery`/`QueryReply` 各型の受理/拒否ケース。TS validator（`bindingwire.ts`）専用 — Rust 側に対応型がないこと自体が契約（第二 IPC 正本を作らない）。`expect.error` クラスは validator 固有の安定語彙（`missing-field`/`bad-kind`/`bad-type`/`out-of-range`）で固定。

### 2.5 `scenarios/*.v1.json`（replay 入力列）

```jsonc
{ "v": 1, "suite": "binding.scenario",
  "meta": { "seed": "fault-ack-storm", "ackTimeoutMs": 1000, "targets": {"track.vol": 0.0} },
  "steps": [
    { "in": { "gesture": { "type": "begin", "gestureId": "g1", "controlId": "f1", "seq": 1, "value": 0.5 } } },
    { "in": { "advance": 600 } },
    { "in": { "receipt": { "commandId": "cmd-2", "status": "applied" } } },
    { "in": { "detach": "unmount" } } ] }
```

- `in` 語彙: `gesture`（InteractionEvent）/`action`/`advance`（clock ms）/`flush`/`receipt`/`projection`/`detach`/`attach`/`query`/`resolveQuery`/`rejectQuery`/`closeView`。host 側は `message`/`tick`/`close`/`destroy`。
- **oracle 期待は scenario に書かない** — `oracle.ts` の ContractModel が `in` 列から期待を導出する。scenario は入力のみを所有し、記録済み応答の replay ではない（Issue 要件）。

## 3. Recorder（`recorder.ts` + `jsonl.ts`）

### 3.1 記録語彙（`v:1` JSONL、1 行 1 record）

```jsonc
{"v":1,"tick":7,"dir":"in","type":"gesture","kind":"commit","target":"track.vol","controlId":"f1","gestureId":"g1","seq":3,"gen":0}
{"v":1,"tick":8,"dir":"out","type":"command","kind":"commit","commandId":"cmd-2","target":"track.vol","gestureId":"g1","expectReceipt":true,"gen":0}
{"v":1,"tick":9,"dir":"in","type":"receipt","commandId":"cmd-2","status":"applied","gen":0}
{"v":1,"tick":10,"dir":"out","type":"phase","phase":"applied","gen":0}
{"v":1,"tick":11,"dir":"sys","type":"dropped","count":12,"reason":"maxEntries"}
```

- `dir`: `in`（UUT への注入）/ `out`（UUT 産出）/ `sys`（recorder 自身の制御情報）。
- `type`（allowlist 語彙）: `gesture`/`action`/`receipt`/`projection`/`command`/`query`/`queryReply`/`phase`/`lifecycle`/`dropped`。
- 収録 field のみ allowlist（`v`/`tick`/`dir`/`type`/`kind`/`commandId`/`queryId`/`gestureId`/`target`/`controlId`/`sourceId`/`viewId`/`windowId`/`gen`/`seq`/`status`/`phase`/`reason`/`expectReceipt`/`bytes`/`count`/`schema`）。**値 payload（`value`/`params`/`data`/`detail`/`payload`/`message` 等）は語彙に存在しない — redaction は「そもそも書き出さない」で実現する。** 文字列は 128 文字で truncate（`valid_op` の `MAX_OP_LEN` に揃える）。
- `tick` は注入可能な logical clock（既定は単調カウンタ）。replay 時刻の再現に使い、wall clock は書かない。
- `schema` field は `rvf.ipc.v1` 系を記録する境界で `1` を保持（将来 `rvf.ipc.v2` が来ても行を見分けられる）。

### 3.2 API

```ts
new IpcRecorder({ enabled, allow?: RecordType[], maxEntries?, maxBytes?, tick?, sink })
recorder.wrapPort(port)        // dispatch/subscribe/query/cancelQuery を観測する BindingPort
recorder.observe(binding)      // binding 側の receipt/projection/phase 入口を包むフック
recorder.toJsonl(): string     // 収録分の JSONL 出力
```

- `enabled:false`（既定）は全 API が no-op で delegate を素通し — 収集 OFF の意味は「ラッパを付けても挙動が変わらない」ことをテストで固定。
- `maxEntries`（既定 4096）/`maxBytes`（既定 256 KiB）で bounded。超過は 1 度だけ `sys/dropped` を書き以後の行を捨てる（unbounded 成長しない）。
- `sink` は行ごとの追記先（メモリ収集のほかテストで fs に流せる）。CI artifact = scenario から生成した `*.v1.jsonl` を fixture として commit し差分レビュー対象にする。

## 4. Replay + 独立 Oracle

### 4.1 `replay.ts` — 注入ドライバ

- `runBindingScenario(scenario, {seed?})`: `InteractionControl`/`CommandBinding`/`QueryBinding`/`ViewLink` の**実装**を生成し（`ManualTimers` 等の決定的制御点を注入）、`in` 列を順に注入、産出 trace（dispatch された CommandIntent 列・各 step 後の `BindingViewState`/`QueryDiagnostics`・診断カウンタ）を収集して返す。
- `runHostScenario(scenario, service)`: 同様に `UiRuntime`（Rust 側・本設計では TS 側は対象外 — Rust テストが同型で駆動）。
- 出力 trace は `out` record 列と同型。記録（`recordings/*.jsonl`）を入力として読む経路と、scenario（`scenarios/*.json`）を読む経路の両方を持つ（記録の往復 = record→JSONL→replay の round-trip 検証）。

### 4.2 `oracle.ts` — ContractModel

実装内部（`binding.ts`/`query.ts`/`coalesce.ts`/`link.ts`）と**コードを共有しない**、文書化契約規則のみを符号化した観測モデル。`in` 列から以下を導出する:

- **dispatch 期待列:** gesture event → command kind 写像（local-only event は産出なし、target 未解決は産出なし、detach 中は産出なし、detach 時 cancel の旧世代発行）、preview coalescing（同一 gesture で flush 間に挟まれた preview は最後の 1 件のみ、gesture 終端で pending preview は supersede）、`commit`/`action` の `expectReceipt`。
- **phase 期待:** 契約の phase 表（active gesture→preview、head 状態→pending/accepted/applied/rejected/unsupported、timedOut→timeout、detached→stale、lastObserved あれば observed、なければ idle）を `in` 列だけから算出。
- **receipt 分類期待:** rank 表（in-flight<accepted/queued<applied<terminal）、generation 不一致/未知 id → stale、rank 非前進/重複 → out-of-order。
- **projection 判定期待:** revision 前進比較（数値・identity supersede）、detached→drop。
- **query 判定期待:** generation 二重チェック・sourceRevision echo・`bytes>maxBytes`→over-budget・`maxInflight` evict・cancel/Abort 双方の経路。
- **link 期待:** attach generation 採番・detach 時の contractual cancel・再 subscribe・closeView の detach+抹消。

照合は trace 同型比較ではなく**契約観測の同値性**: dispatch kind/target/gestureId/順序、phase 遷移列、診断分類カウント、receipt/projection/query の受理/拒否分類。`issuedAt` 等の clock 値は注入 clock の deterministic 値で一致確認。

### 4.3 決定性と seed

- 全シナリオを fresh インスタンスで 2 回実行し trace が完全一致することを assert。
- `seed.ts` は mulberry32 の決定的 PRNG + 生成器（fault 種別・間引き・順序を seed から導出）を提供し、`seed` 固定で同一シナリオが再生成されることを固定する。scenario `meta.seed` は証拠項目として記録・CI で再現される。

## 5. Fault matrix

| 故障種 | TS（Binding/Query/Link 層） | Rust（UiRuntime/scheme 層） |
| --- | --- | --- |
| late ack | timeout 超過後の applied → phase `applied` + `timedOut` 維持 | 遅い Finished signal の drain（mailbox に残存していれば次 Tick で terminal が出る） |
| reordered ack | applied→accepted は rank 逆行で吸収（`outOfOrderReceipts`） | n/a（単一 result 相関） |
| duplicate ack | 重複 status は吸収・二重適用なし | duplicate `job.cancel` は `unknown_job`/`cancelling` の決定的応答 |
| view detach/reattach | detach→cancel 発行・dead gen receipt/projection は stale・attach で再 subscribe | `handle_window` の CloseRequested → `lifecycle{closing}` を当該 window のみ |
| window destroy | `closeView` で detach+登録抹消・以後の projection は drop | `Destroyed`（terminal event、dead window への配信は配信層 drop）の語彙固定 |
| stale source/revision | stale projection・stale query reply を破棄 | n/a（host は source revision を持たない） |
| overlapping gesture | 別 controlId/gestureId の交錯は相互に影響しない・detach は全 cancel | n/a |
| backpressure | preview burst coalesce（superseded 計数）・`TRACKED_LIMIT` 退避・`maxInflight` evict | JOB_MAILBOX 溢れ下の progress drop + terminal 非 drop（bounded mailbox 契約の fixture 化） |
| timeout | ack bound 超過 → `timeout` phase（明示失敗、成功捏造なし） | n/a（host に ack timeout 語彙なし — `job.cancel` 要求後の terminal は worker 責務） |
| cancel | cancel コマンド・query AbortSignal+cancelQuery 二経路・detach cancel | `job.cancel` → CancellationToken → `cancelled` terminal event |
| unsupported | `unsupported` receipt → terminal phase・unsupported query port → `unsupported` outcome | `unavailable`/`unknown_op` 安定コード |
| disconnect/crash | port.dispatch が throw しない契約・query port reject → `error` outcome（接続断は crash でなく error として表面化） | job worker panic → `worker_failed` terminal（disconnect 相当）、mailbox disconnect → drain 正常終了 |

「terminal event 欠落なし」の機械検証: 各 fault シナリオで、gesture/query/job を開始した入力に対し**終端に相当する観測**（cancel/commit dispatch、terminal receipt、terminal job event、cancelled/stale outcome、`closing` lifecycle）が trace 内に存在することを assert する。

## 6. セキュリティ設計（AC-4）

- **opt-in:** `enabled:false` 既定。no-op パススルーをテストで固定。
- **allowlist:** record `type` と field 名は固定語彙。未知 type/field の注入は拒否または `sys/dropped` に落とす。
- **redaction:** 語彙に値系 field を持たない設計（payload 本体・`message`/`detail` 自由文・URL クエリ値・token 類を書かない）。文字列は 128 文字 truncate。
- **容量上限:** `maxEntries`/`maxBytes`。超過は `sys/dropped` 1 行のみ追記し停止 — 無制限成長しない。
- **収集 ON/OFF:** OFF では sink への書込みゼロ・delegate 呼出しの挙動不変・追加大域状態なし。
- **CI artifact:** commit される `recordings/*.jsonl` はすべて seeded 生成物で、値/個人情報/認証情報を含まないことを fixture lint（`fixtures.test.ts` の語彙検査 + field denylist スキャン）で機械検証する。
- **依存:** 新規 dependencies ゼロ（typescript/@types/node は既存 devDependency 慣行に合わせる）。Rust 側の新規依存もゼロ（serde_json 既存）。

## 7. 検証入口・証拠（AC-5 / #294 接続）

- mise: `check-ui-guitest`・`test-ui-guitest` を新設し CI `ui-verify` に追加。Rust 側は既存 `test`/`test-ci`（workspace）と `test-ui-webview`（feature lane）が拾う。`verify-ci` 列は変更しない（workspace `test` で ruviform-ui の conformance が走る）。
- 証拠項目（#294 の Pass/Fail/NotRun/Unsupported 記録に接続可能な形）: lane 名（`test-ui-guitest`/`test`/`test-ui-webview`）、fixture `v`、scenario `meta.seed`、対象契約（`rvf.ipc.v1`/`rvf.data.v1`/`binding.v1`）、判定結果。IR 文書に記録する。
- 失敗時は NotRun/Unsupported と Pass を混同しない — webview lane は feature 要求のため `test-ui-webview` 実行環境が整う OS でのみ評価（Linux では WebKitGTK 依存、CI では 3 OS 実行済み）。

## 8. 却下した代替案

- **zod/JSON Schema の導入:** Issue が必須採用を先決しない + 本パッケージの「依存ゼロ」規律と衝突。手続き型 validator で足りる。
- **ui/binding への同居:** recorder/replay はテスト基盤であり binding の product src に混ぜると dist 配布面が汚れる。別パッケージで isolation。
- **記録済み応答を expectation に使う方式:** Issue が明示禁止（「記録済み応答を再生して PASS としない」）。独立 ContractModel を採用。
- **fixture の Rust crate 内コピー:** 単一正本原則に反するため相対パス参照を採用（`include_str!` で埋め込まず実行時読み — fixture 差分がそのままテストに効く）。
- **常時稼働 Event Store / Observability 基盤:** Issue 非対象。recorder は pull 型（明示 `toJsonl`）で常駐しない。
