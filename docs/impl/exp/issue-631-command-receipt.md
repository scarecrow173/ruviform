# Issue #631 real-command-receipt case — 実装実験記録（2026-11-17）

`docs/tasks/issue-631-command-receipt.md` の実装で試行・観測した
事項の記録。実装結果の正本は
[`docs/impl/ir/issue-631-command-receipt.md`](../ir/issue-631-command-receipt.md)、
イベントログは
[`issue-631-command-receipt-exp.jsonl`](issue-631-command-receipt-exp.jsonl)
を参照。

## 試行 1: command-minting fixture の置き場所

- **目的**: receipt-expecting intent を mint する control を real
  Window session で駆動する。
- **観測**: 既存 `plugin_dock_view_spec`（NativeSlot のみ）は
  command binding を持たず gesture が intent を mint しない。
  `ruviform-ui-test` の `sv_fader_dock_fixture` は fader
  `fader-ch1` を持つ command-bound spec で、`scenario_contract.rs`
  の receipt/projection 先例と同じ flow を real driver で再現
  できる。
- **結論**: 既存 window の spec 差し替え・B/C 流用はいずれも
  sentinel/close/focus の前提を壊すため、第 4 window D
  （`ViewInstanceId` 634・position (4350,80)）として新規 mount
  する形態を採用（ADR 記録）。

## 試行 2: `command_id` 相関の取り方

- **目的**: receipt を minted command に正しく結び付ける。
- **観測**: gesture が mint する `MintedIntent` は
  `command_id`・`expect_receipt`・decode 済み `CommandIntent`
  を持つ。begin/preview/commit で複数 intent が mint される
  ため、`expect_receipt` + `CommandIntent::Commit` で
  receipt-expecting commit を識別し、その `command_id` を
  `Event::Result.id` にそのまま使う。
- **結論**: display text・配列位置での推測はせず、minted 値を
  verbatim で相関。decode した `CommandReceipt.command_id` が
  minted id と一致することも assert した。

## 試行 3: applied→observed の 2 level 分離

- **目的**: AC1 の Applied/Rejected と AC4 の独立 state 照合を
  混同しない。
- **観測**: `Event::Result`（`status:"applied"`+revision）だけ
  では binding は `Applied` に留まり、`Event::Notify`
  （`control_id`/`revision`/`value`）の authoritative projection
  で初めて `Observed` へ settle する。revision の stale guard が
  あるため projection は実 commit revision+1 で送る必要がある。
- **結論**: applied 経路は receipt→`Applied`→projection→
  `Observed` の 2 段で観測し、`observed.revision=2` を evidence
  に記録。rejected 経路は `Event::result_err` で
  `status:"rejected"`+detail を decode させ `Rejected` を観測。

## 試行 4: window D 追加が close case を壊した件

- **観測**: 初回 `test-ui-plugin-dock-real` 実走で
  `real-command-receipt` は pass（receipts 2 件）したが、
  `real-window-close` が fail — 終端 assert が
  `windows() == [window_a]`（sentinel のみ残存）を要求しており、
  生存した window D（w4）を想定していなかった
  （`expected only sentinel window A to remain, got [w1, w4]`）。
- **結論**: assert を「mount 時 `windows()` から close target
  B/C を除く全 window が生存」へ一般化。survivor 全員の
  `window_events` に `CloseRequested`/`Destroyed` が無いことを
  検証 — 従来 A のみだった検査を D にも拡張し、強度は維持・
  強化。再実走で close pass を確認。

## 試行 5: interactive lab lane の扱い

- **観測**: `test-ui-plugin-dock-lab` は manual/opt-in の
  interactive GPUI session で、Run all→Quit の operator 操作が
  必要。本 host では `orca computer`（accessibility provider）が
  Defender に block され、GPUI window は UIA descendant を
  露出しない（`Zed::Window`・descendants 0）ため programmatic
  な click 経路が無い。
- **結論**: window 起動→WM_CLOSE で clean exit(0) までを確認し、
  Run-all→Quit の dogfooding は operator 実行の残件として
  正直に記録。catalog 投影（30 行 = headless 8 + real 22）は
  `test-ui-plugin-dock-console --list --include-real` と
  `catalog_rows_cover_headless_and_real` unit test で検証済み。
  偽の lab evidence は生成しない。
