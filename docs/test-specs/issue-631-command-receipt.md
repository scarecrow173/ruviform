---
id: "TSPEC-Ji15qVcbOD8vLh0tYx6aPg"
type: "test-spec"
status: "approved"
title: "Issue #631 real-command-receipt case のテスト仕様"
created: "2026-11-17"
updated: "2026-11-17"
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
    - "docs/designs/issue-631-command-receipt.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-631-command-receipt.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 real-command-receipt case のテスト仕様

## Purpose

`real-command-receipt` が「実 Window の gesture が mint した
command の `command_id` に対し、driver-neutral `post_event` で
投入した実 `Event::Result`/`Event::Notify` が binding を正しく
settle する」ことを保証する test spec。receipt を合成しない・
`command_id` を表示文字列や位置で推測しない・`Applied` と
`Observed` を混同しない、という honesty 契約が将来の変更で
崩れないようにする。

## Feature

`rf631.plugin-dock-real` catalog の `real-command-receipt` case
（`crates/ruviform-plugin-dock-test`、`real-dock` feature、
interactive Windows desktop 前提）+ `real-window-close` の終端
assert 一般化 + AC4 `receipts` evidence（`jsonl.rs` 既存配線）。

## Rules

- Rule: gesture 駆動は `dispatch_input`（semantic input）を使い、
  intent 採取は `drain_intents` — OS-level の pointer/key 注入
  ではなく driver-neutral 契約経由。
- Rule: receipt-expecting intent は `expect_receipt` と
  decode 済み `CommandIntent` を持つ `MintedIntent` で識別し、
  `command_id` は minted 値をそのまま `post_event` の
  `Event::Result.id` 相関に使う — 表示文字列・配列位置・推測で
  command を特定しない。
- Rule: `post_event` する `Event::Result`/`Event::Notify` は
  view へ実配送される event であり、receipt は同じ event を
  `CommandReceipt::from_result` で decode したもの — receipt
  を別経路で合成・捏造しない。
- Rule: `Applied`（receipt 適用）と `Observed`（authoritative
  projection 適用）は別 evidence level — `Event::Notify` の
  `control_id`/`revision`/`value` でのみ `Observed` へ settle
  し、revision の stale guard を迂回しない。
- Rule: rejected receipt は `CommandPhase::Rejected` へ settle
  し、直前の authoritative projection（`observed` の
  revision/value）が binding に保持されることを観測する —
  reject が観測済み state を黙って巻き戻す/進めることを
  許さない。`display`/`committed` は component snapshot の
  実値をそのまま記録し、観測値を合成・補完しない。
- Rule: receipt-expecting intent が mint されない・phase が
  期待と異なる・observe が失敗する場合は honest `Fail` —
  `NotRun`/`Pass` に偽装しない。収集済み receipt は失敗時も
  `ScenarioOutcome.receipts` に保持する。
- Rule: `REAL_CATALOG` と `real_entries` の `case`/`scenario`/
  `required` は協調して更新する — 片方だけの変更は
  `real_catalog_matches_entries` で検出される。
- Rule: `real-window-close` の終端 assert は「mount 時 window
  集合から close target を除く全 window が生存」— sentinel A
  だけでなく fixture window D も非対象 survivor として
  close lifecycle 非観測を検証する。assert を弱めない。
- Rule: headless/blocked では全行 `NotRun`+
  `observed.environment_blocked`・`environment_kind` が
  `real-window` を名乗らない fail-closed 形状を維持する。
- Rule: GPUI/Win32 型を公開 API・JSONL に出さない —
  driver boundary（`check-ui-driver-boundary`）を維持する。

## Examples

- Example: applied→observed — Given window D の fader で
  Begin→Preview→Commit が receipt-expecting intent
  （`command_id` 記録）を mint、When `post_event` で
  `{"status":"applied","revision":1}` の result を投入し settle
  する、Then `CommandPhase::Applied` が観測され committed/
  display 値が gesture 値になる。続く `Event::Notify`
  （`revision:2`・`value.normalized`）で `Observed`・
  `observed.revision=2` が観測される。
- Example: rejected — Given 第 2 gesture の commit intent、
  When `{"status":"rejected","detail":...}` の result を投入
  する、Then `CommandPhase::Rejected` が観測され、直前の
  authoritative projection（`observed.revision=2`）が binding
  に保持される。
- Example: receipts populate — Given 上記 2 経路、When report
  が `render_jsonl` される、Then `receipts` に decode 済み
  `CommandReceipt`（`command_id`/`status`/`revision`/`detail`）
  が 2 件出る。
- Example: close survivor — Given window A/B/C/D mount、
  When close case が B/C を破壊する、Then `windows()` が
  `[A, D]`（非対象全員）と一致し、A/D の event log に
  `CloseRequested`/`Destroyed` が無い。
- Example: blocked — Given 実 desktop 不在、When real lane が
  session を開始できない、Then 全 22 行が `NotRun`+
  `environment_blocked`・`deterministic-backend` で exit 0。

## Guarantees

- `real-command-receipt` が実 Window で gesture→intent→
  receipt/projection→`Applied`→`Observed`/`Rejected` の phase
  遷移を実観測し、`command_id` 相関の receipt を JSONL に
  非空で記録する。
- `Applied` と `Observed` が別 level の evidence として区別
  され、projection の revision guard が守られる。
- rejected 経路で `CommandPhase::Rejected` と直前 projection
  の保持が観測される。
- `real-window-close` が window D 追加後も「非対象 window 全
  生存」で正しく判定する（終端 assert は survivor 集合への
  一般化であり、検証強度は維持・強化）。
- `REAL_CATALOG`/`real_entries` の required capabilities が
  一致し、catalog 整合 test が green。
- `plugin-dock-real-headless.jsonl` が全行 `NotRun`+
  `environment_blocked` を維持し、22 case 行を出力する。
- 実 desktop 不在時は `environment_blocked`/`NotRun` が記録
  され、偽 Pass にならない。

## Non-goals

- HEADLESS_MATRIX 側 command-receipt 行（ruviform#725 lease）。
- `real-dcomp-pixel` の Fail 修正（#532 gate）。
- editor verbs・IME・DPI 変更・crash/hang（#389/#630 領域）。
- consumer 投影（synvelis#114/#176・audio-lens#127）。
- AC4 の残 field（GPU 情報・semantic tree 全量・HWND/plugin
  hash）。
- `Unsupported`/`Queued`/`Accepted` receipt status の実経路
  駆動（本 slice は `Applied`/`Rejected` の 2 経路 — status
  decode の網羅は `CommandReceipt::from_result` の unit test
  が担保）。

## Risk

- gesture の実 OS timing で intent mint が遅延/失敗 — bounded
  `settle_until` で待機し、mint 不在は honest `Fail`。
  実走で安定を確認済みだが、環境差で flake し得る —
  `settle` の timeout は scenario の既存 timeout に従う。
- `command_id` の採取を誤る（begin/preview intent を commit
  と混同）— `expect_receipt` flag と `CommandIntent` variant
  で識別し、gesture ごとに新規 mint を待つ。
- window D が他 case へ干渉 — case 順で command-receipt は
  close より先、D は以降も生存するが参照するのは
  command-receipt のみ。close の survivor assert は一般化で
  対応済み。
- projection revision の stale guard で `Observed` に届か
  ない — revision を実 commit 値+1 で送り、stale 判定を
  迂回しない。
- evidence の `receipts`/`semantic_tree` が大きくなりすぎる —
  fader 1 control の snapshot のみで bounded。
