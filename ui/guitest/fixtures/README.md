# guitest fixtures — format v1

Issue #311 の単一 versioned fixture 集合。Rust(`crates/ruviform-ui` テスト)と
TypeScript(`ui/guitest`)の**両側が同じファイルを読み**、同じ version・同じ意味で処理する。
ここが fixture 形式の正本。新形式は `v` を上げて追加し、両側の読み手を同時に更新する。

## 共通構造

```json
{ "v": 1, "suite": "<suite>", "id": "<file id>", "cases": [ ... ] }
```

- `v` は必須。`1` 以外は fail-closed で拒否(unknown version 自体をテストする case は
  case 内の body/wire に閉じ込める)。
- `suite` は固定語彙: `ipc-intents` / `ipc-events` / `ipc-data` / `binding-messages` /
  `scenario-binding` / `scenario-host`。
- 未知 `suite`・未知 top-level field・case 語彙外の field は拒否(構造検査は
  `src/fixtureload.ts` / Rust `ipc_conformance.rs` の両側で実施)。
- fixture / recording に `value`・`params`・`payload`・`token` 等の実データを抱える
  field を test input 以外で書かない。scenario の intent value は `{"plain":n,"normalized":n}`
  固定形のみ。

## suite: ipc-intents

`rvf.ipc.v1` frontend→Rust intent の受理/拒否を固定する。

```json
{ "id": "valid-sys-ping",
  "body": "{\"v\":1,\"kind\":\"intent\",\"id\":\"i-1\",\"op\":\"sys.ping\",\"params\":{}}",
  "expect": { "accept": true, "op": "sys.ping", "id": "i-1", "params": {} } }
```

- `body`: 生 JSON 文字列。`bodyGen: {"kind":"pad-intent","padTo":<bytes>}` の場合は
  両側が同じ規則で生成する: `{"v":1,"kind":"intent","id":"i-pad","op":"sys.ping","params":{"pad":"<padTo調整のx列>"}}`
  を byte 長 padTo にする(文字列 pad は ASCII `x` のみ。body の UTF-8 byte 長で判定)。
- `expect.accept: true` → `op`/`id`/`params`(欠落時 `null`)を照合。`false` →
  `code`(`malformed_message`/`message_too_large`)を照合。
- 未知 top-level field を含む body は serde 規約どおり受理される case として固定する。

## suite: ipc-events

Rust→frontend `Event` envelope の wire 形を固定する。

```json
{ "id": "job-progress-detail",
  "wire": {"v":1,"kind":"job","job":1,"state":"progress","done":3,"total":10,"detail":"peaks"},
  "expect": {"accept": true, "class":"job", "state":"progress"} }
```

- `wire` は envelope の JSON 値。Rust は対応する `Event` を `to_json()` して `wire` と
  JSON 値一致を照合し、TS は decoder で `expect` と照合する。
- `expect.accept:false` は `code`(`malformed_message`)を照合 — version 不一致・未知
  kind・必須 field 欠落。未知 field は additive extension 規約どおり受理される。
- `expect` の補助語彙: `class`(`result|job|notify|lifecycle|error`)、`correlation`
  (`id`/`job`/`seq`/`state` の echo)、`ok`、`state`、`code`。

## suite: ipc-data

`rvf://data` の paging 契約を固定する。registry は case 内に宣言する。

```json
{ "id": "range-mid",
  "registry": {"maxChunk":8,"sources":[{"name":"demo/peaks","size":64,"byte":1}]},
  "request": {"name":"demo/peaks","offset":10,"len":4},
  "expect": {"status":200,"bodyLen":4,"total":64} }
```

- `registry`: `maxChunk`(既定 16MiB)と `sources`(name + 内容 byte 列生成規則
  `byte`(fill)または `bytesHex`)。
- `request.name` は raw path(multi-segment 可)。`offset`/`len` は省略可
  (`offset` 既定 0、`len` 省略 = cap まで)。
- `expect.status`: `200`/`400`/`404`/`500`。`200` は `bodyLen`・`total`・header
  契約(`X-Rvf-Total`/`Content-Type: application/octet-stream`/`Cache-Control:
  no-store`/`Access-Control-Allow-Origin: *`)を照合。`400` は `class`
  (`bad_name|bad_params|out_of_range`)、`404` は `not_found` を照合。
- `url` case: `"call":{"name":"demo/peaks","offset":10,"len":4}` +
  `"expect":{"url":"rvf://data/demo/peaks?offset=10&len=4"}` — bridge `dataUrl`
  構築規則(segment ごと `encodeURIComponent`、`offset` 既定 0、`len` は指定時のみ)
  の往復を固定する。

## suite: binding-messages

`ui/binding` 契約メッセージの構造受理/拒否を固定する。

```json
{ "id":"intent-commit",
  "message": {"kind":"commit","target":"mixer.ch1.fader","controlId":"fader-1",
              "sequence":7,"gestureId":"g-1","value":{"plain":0.5,"normalized":0.5}},
  "expect": {"accept":true,"class":"intent","commandKind":"commit"} }
```

- `class`: `intent`(`CommandIntent`)/`receipt`/`projection`/`viewportQuery`/`queryReply`。
- `expect.accept:false` は安定語彙 `missing-field`/`bad-kind`/`bad-type`/`out-of-range`
  を `reason` で照合。

## suite: scenario-binding

deterministic replay 用の入力列。実 `InteractionControl`/`CommandBinding`/
`QueryBinding`/`ViewLink` に注入し、contract oracle と照合する。

```json
{ "id":"commit-applied-observed", "seed": null,
  "setup": {"viewId":"view-1","windowId":"win-1","ackTimeoutMs":1000,
            "targets":[{"control":"fader-1","target":"mixer.ch1.fader","initial":0.0}],
            "consumer":"daw"},
  "in":[
    {"op":"gesture","control":"fader-1","inputs":[{"type":"begin","gestureId":"g-1","source":"pointer"}]},
    {"op":"flush"},
    {"op":"receipt","status":"applied"},
    {"op":"projection","target":"mixer.ch1.fader","revision":1,"value":{"plain":0.5,"normalized":0.5}}
  ],
  "expect": {"phases":["preview","pending","applied","observed"],
             "diagnostics":{"staleReceipts":0}} }
```

- `in` の `op` 語彙: `gesture`/`action`/`flush`/`advance`/`receipt`/`projection`/
  `detach`/`attach`/`closeView`/`query`/`resolveQuery`/`rejectQuery`/`setSourceRevision`。
- `expect` は省略可能 — 記述の有無にかかわらず replay は oracle trace と照合する
  (記録済み応答の再生は PASS 条件ではない)。`phases`/`diagnostics`/`dispatched`
  は pin する追加 golden。

## suite: scenario-host

Rust runtime 側の入力列(`HostEvent` 語彙)。`expect.events` が routed event の
golden(種別・窓・相関・code)。

```json
{ "id":"close-then-message",
  "windows":["w1","w2"],
  "in":[{"op":"message","window":"w1","body":"{\"v\":1,...}"},
        {"op":"close","window":"w1"},
        {"op":"destroy","window":"w1"},
        {"op":"message","window":"w1","body":"...ignored..."},
        {"op":"tick"}],
  "expect":{"events":[{"window":"w1","kind":"lifecycle","state":"closing"}]} }
```

- `destroy` した window 以後の `message` は配信層が drop する(runtime は産出しても
  届かない) — 「dead window への配信なし」を fixture で固定する。

## recordings(*.v1.jsonl)

1 行 1 record の JSONL。`{"v":1,"tick":N,"dir":"in|out|sys","type":"...","...":...}`。
allowlist field のみ。replay は `dir:"in"` のみを入力として注入し、`dir:"out"` は
期待として使わない(oracle との照合が判定根拠)。
