---
id: "TSPEC-e0XfO8Zf3a26iGOk1UyquR"
type: "test-spec"
status: "approved"
title: "Issue #311 RF-GT-02 test specification — fixture conformance・record/replay determinism・fault matrix"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "docs/specs/issue-311-ipc-conformance-replay.md"
  changes:
    added:
      - type: "dir"
        path: "ui/guitest/test"
      - type: "file"
        path: "crates/ruviform-ui/tests/ipc_conformance.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-webview/src/scheme.rs"
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
    - "docs/designs/issue-311-ipc-conformance-replay.md"
    - "docs/plans/issue-311-ipc-conformance-replay.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-311-ipc-conformance-replay.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #311 RF-GT-02 test specification

検証対象: spec AC-1〜AC-5。責務は「契約上観測可能な振る舞いの一致」であり、実装内部の再検証ではない。

## Feature

- 単一 versioned fixture 集合が Rust serialize/deserialize と TS decode/validation の両側で同一 version/意味を持つこと。
- 入力列を実 Controller/Binding へ注入する deterministic replay が、文書化契約規則を符号化した独立 oracle と一致すること。
- Recorder が opt-in・allowlist・bounded で、値 payload・機微・巨大 binary・自由文を生で収集しないこと。
- fault matrix が truth/receipt を偽装しないこと（terminal event 欠落なし、成功捏造なし、stale/dead 世代の着弾なし）。

## Rules

- Rule: fixture は `{"v":1,"suite":..,"cases":[..]}` 形のみ。未知 `v`・未知 `suite`・未知 field 混入ケースは fail-closed で拒否。fixture を両側が読み、期待と照合する（`body` の byte 列、`wire` の構造体、`expect` の受理/拒否・安定コード・`class`）。
- Rule: `rvf.ipc.v1` は Intent の version/kind/op/size/`id` 型、Event envelope の `v`/`kind`/相関・optional 能力（`detail`/`details`）、ErrorBody code 集合を fixture で固定。Rust は `parse_intent`/`Event::to_json`/`UiRuntime::handle` で、TS は decoder で同一判定。
- Rule: `rvf://data` は URL パース・`offset`/`len` 境界（`offset==total` → 200 空、`offset>total` → 400、cap clamp）、`X-Rvf-Total`・status 写像・未知 query key 無視を fixture で固定。TS は URL 構築往復 + 応答契約モデルで照合。
- Rule: Binding 契約メッセージの受理/拒否は `bindingwire` validator の安定語彙で固定。CommandIntent は kind 別必須 field、CommandReceipt は status 語彙、StateProjection は revision、ViewportQuery は数値境界、QueryReply は generation/sourceRevision echo を検証。
- Rule: replay は `in` 語彙（gesture/action/advance/flush/receipt/projection/detach/attach/query/resolveQuery/rejectQuery/closeView と host 側 message/tick/close/destroy）を実インスタンスに注入し、産出を ContractModel の契約観測（dispatch 列・phase 列・receipt/projection/query 分類・診断カウント）と照合。同一 seed で 2 回実行して trace が一致する。
- Rule: fault matrix の各行で「終端観測が存在する」ことを assert する — gesture には commit/cancel dispatch、expectReceipt command には terminal または明示 timeout、query には outcome、job には finished/cancelled/failed、window には `closing`。
- Rule: 記録 JSONL は `v`/`tick`/`dir`/`type` + allowlist field のみ。denylist 語彙（`value`/`params`/`data`/`payload`/`message`/`detail`/`token`/`secret`/`grant`/`media`）を含む行は存在しない。超過時は `sys/dropped` 1 行で明示。
- Rule: `enabled:false` の recorder は wrap/observe しても挙動不変（dispatch 結果・phase・diagnostics が recorder なしと一致、sink 書き込みゼロ）。
- Rule: preview coalesce は同一 gesture の flush 未実行 preview のみ。`commit`/`cancel`/`action`/`begin` は coalesce/drop/reorder しない（`superseded` 計数は観測可）。
- Rule: `seed` 固定の生成シナリオは bit 同一に再生可能。seed を scenario meta に記録する。

## Examples

- Intent fixture `unsupported version`（`v:2`）を Rust `parse_intent` と TS decoder が双方 `malformed_message`/拒否で処理する。
- Event fixture `job progress with detail` の `wire` JSON を Rust `Event::to_json` が byte レベル一致（JSON 値同一）で生成し、TS decoder が `class:job`・`state:progress`・`detail` ありで受理する。
- `data` fixture `offset beyond end` は registry `OutOfRange` ↔ wire `400` ↔ TS `out_of_range` で一致。
- Scenario `late-applied-after-timeout`: commit → `advance(1100)` → receipt `applied`。oracle は phase `applied` + `timedOut` を期待し、実 Binding 出力と一致。
- Scenario `detach-with-pending`: preview 滞留中に detach → 旧世代 `cancel` が唯一の発行で、queue は purge、dead 世代 receipt は `staleReceipts`。
- Seeded `fault-ack-storm` は late/reordered/duplicate/unknown ack を seed から生成し、oracle の分類カウントと一致。
- Recorder `enabled:false` 下で scripted DAW backend への dispatch 結果と phase が recorder なし run と完全一致。
- JSONL round-trip: scripted session を recorder で `*.v1.jsonl` に書き、`jsonl` parser で構造検査 → replay で同 trace を再現。

## Guarantees

- 既存 `ui/binding`・`ui/core`・`crates/ruviform-ui` の product コードを変更しない（scheme.rs はテストモジュール内のみ）。
- 新規 npm/Rust dependencies を追加しない（typescript/@types/node の devDependency のみ）。
- `mise run test`（workspace）・`test-ui-guitest`・`test-ui-webview`・既存 UI レーンが green。
- fault matrix 全行が terminal event の存在を assert する（truth/receipt 偽装の検出経路を持つ）。

## Non-goals

- 実 DOM/WebView/OS window・描画・a11y・実 RT sample accuracy（RF-GT-01/03/04/05・RT lane の範囲）。
- Consumer 製品（Synvelis Project/Grant、AudioLens Dataset oracle）のシナリオ。mock は契約形状のみ。
- `rvf.ipc.v1` の新 kind・version 変更・第二正本。
