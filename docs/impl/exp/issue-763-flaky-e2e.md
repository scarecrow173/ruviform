# Issue #763 l1_real_test_plugin_e2e instrument tests flaky — 実装実験記録（2026-11-18）

`docs/tasks/issue-763-flaky-e2e.md` の slice で試行・観測した事項の
記録。実装結果の正本は
[`docs/impl/ir/issue-763-flaky-e2e.md`](../ir/issue-763-flaky-e2e.md)、
イベントログは
[`issue-763-flaky-e2e-exp.jsonl`](issue-763-flaky-e2e-exp.jsonl)
を参照。

## 試行 1: flaky の再現と機構の二分

- **目的**: 未修正コードで flaky を再現し、失敗の機構を
  `passthrough_submits` / `dropped_submits` delta で分類する。
- **観測**: 60 iteration の dual-binary 負荷 loop（両 e2e binary を
  同時実行）で 12 run failure（約 10%）。`note_off_releases_to_
  silence` ×7・`transport_clicks` ×6 を中心に、clap/vst3 両 crate
  で観測。
- **結論**: 失敗は 2 機構の混合 — (1) substituted output
  （`pt_delta=1` の stale `last_output` replay が phase window を
  跨ぐ）、(2) dropped submit（`drop` delta により NoteOff 等の
  event が child に未到達、voice が sustain して matched block
  でさえ oracle 違反）。`pt` 分類のみでは機構 2 を検出できない
  ことが `pt_delta=0` での非零 tail で実証された。

## 試行 2: oracle 設計の選定（Coordinator 承認）

- **目的**: 2 機構を両方カバーする決定的 oracle を選ぶ。
- **観測**: 案 B（matched gate のみ）は機構 2 で依然 flaky、
  案 C（spec 文言どおり pt のみ）は機構 2 で不十分と実証済み。
  案 A（pt 分類 + NoteOff の bounded resend-until-landed）は
  両機構を構造的に排除し、assertion/gate を緩めない。
- **結論**: Coordinator が案 A を承認（条件: resend 上限付き・
  drop は evidence に分類記録・assertion と最低到達数 gate 不変）。

## 試行 3: seq 帰属の確認

- **目的**: matched block の source call 帰属が drop 下でも正しいか
  を production code で確認する。
- **観測**: `data.rs:683-711` で `next_block` が submit 試行前に
  increment され、drop 時も seq を消費 — seq 空間 = call 空間。
  `expected = seq − lookahead` の result が drain された call の
  output は source call `call − 2` の rendering に厳密帰属。
- **結論**: `source = call − L1_LOOKAHEAD` の exact attribution が
  drop 下でも成立。release 進行は arrived block 数
  （`arrived_between`）で測る必要があることも確定。

## 試行 4: 実装と canonical lane 初回

- **目的**: 両 crate に oracle を実装し、canonical lane で
  正しさを確認する。
- **観測**: `cargo check` で両 crate compile 通過、
  `mise run test-l1-real-plugin` 初回で全 10 test × 2 crate が
  green（instrument 4 + attach 3 + full_series + spawn/kill）。
- **結論**: oracle は少なくとも非負荷下で正しく動作。残りは
  looped + load lane の反復検証と lint/fmt/precheck。

## 試行 5: attach 経路の resend 設計

- **目的**: attach の NoteOff が L1 submit drop で失われる機構 2 を
  直接経路と同型に対処する。
- **観測**: attach の event は MIDI cue → `InputTx` → node →
  block event list の経路で、block の L1 submit が落ちるとその
  block の event は全て失われる。cue を block 毎に窓内 resend
  すれば、各 cue は別 block に乗り、最後に着地したものが最終
  release を開始する（fixture law の再武装と整合）。
- **結論**: `AttachRig` に `host`+`handle` を保持し `pump_block`
  で per-block `LegDeltas` を記録。cue を tick 1500+512k
  （block 2..=9）に 8 個 resend、`input.mapped/delivered` 期待値を
  9 に更新（node 側 delivery 証明としては強化）。

## 試行 6: post-fix loop の残存 flaky と exact attribution への pivot

- **目的**: window 維持版 oracle の looped 検証で残存 flaky が
  ないか確認する。
- **観測**: 40-iter dual-binary loop（iters 31-40 は concurrent
  `cargo build` 負荷併走）で 3 run failure — 全て content assert
  ではなく `verified >= 2` 系の最低到達 gate:
  `cc_and_offset_gating` ×2（"the full-volume phase never
  delivered"）、`transport_clicks` ×1（click phase verified=1）。
  matched-only 計上で実効 sample が狭い window（cc: 4 call、
  transport: phase 毎 ~5 call）に縮み、substitution burst 下で
  gate を割った。`evidence/rf763/logs3/` iter-8/19/21。
- **結論**: window は matched ⇒ source = call − 2 の厳密帰属を
  使い切れていなかった — spec が許容した「window 簡素化」案を
  採用し、cc/transport を **source call の phase そのものへの
  assert** に書き換え（cc は 16→22 call、transport は 8→12
  call/phase ×4 = 48 call に延長して matched sample を確保）。
  assert 内容・gate 値は不変で、assert 対象は全 matched block に
  拡大（強化方向）。gate 割れの構造的原因を除去した結果、
  同一 loop が 40/40 green（`evidence/rf763/logs3/`・`logs4/`
  再実行分）。

## 試行 7: review-protocol 2 rounds

- **目的**: 実装済み oracle の静的レビューで残存する oracle
  盲点がないか独立 2 視点で検査する。
- **観測**: Reviewer A が 4 件、Reviewer B2 が 3 件を報告。
  実在と判定し修正したもの:
  - attach note-on の sustained count が substituted replay を
    計上し得た → `deltas[i].passthrough == 0` のみ計上。
  - attach/direct の silence 系 assert が endpoint/child failure
    の defined-silence（`passthrough_submits` 不増のため
    「matched」と誤分類）を false-pass し得た → 両経路に
    `lifecycle_state == Processing` の liveness assert を追加
    （direct は driver exit-poll 収束をカバーする 5ms settle
    sleep 付き）。
  - direct note-off の `off_pending` が resend window 後も残り、
    post-window の non-dropped submit（`empty` events）が
    `off_landed` を誤記録し得た → landing 判定を
    `8..8 + NOTE_OFF_RESEND_CALLS` に gate。
  - `arrived_between` の `a > b` 未 guard → attach と同じ
    `source <= landed → continue` を direct にも追加
    （現状到達不能、parity のため）。
  - spec R2 の「単一 NoteOff」記述が resend 実装と矛盾 → R7
    参照へ更新。
  - 棄却: attach first-result stale-discard（Reviewer A 候補 4）
    は 25ms pacing で緩和済みの既存 low-severity 懸念として
    本 slice 範囲外。
- **結論**: 修正後 instrument suite 10×2 green・fmt-check・
  lint-l1-real-plugin（`-D warnings`）PASS・loop4.sh
  （post-review binary、40 iter、iters 31-40 load 併走）
  **40/40 green**（`evidence/rf763/logs5/`・`logs6/`）。
