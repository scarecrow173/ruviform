---
id: "SPEC-Rf767SbmtDrpKm3hXwNq7Yt4"
type: "spec"
status: "proposed"
title: "Issue #767 — L1 submit 経路の drop(queue shedding)契約"
created: "2026-11-22"
updated: "2026-11-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/767"
    - "crates/ruviform-plugin-host/src/l1/data.rs"
    - "crates/ruviform-plugin-host/src/l1/mod.rs"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-767-submit-drop.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/discovery/issue-767-submit-drop.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-763-flaky-e2e.md"
    - "docs/specs/issue-895-dropped-submits.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
---

# Issue #767 — L1 submit 経路の drop(queue shedding)契約

`ruviform#767` の調査(`docs/discovery/issue-767-submit-drop.md`)で、L1 submit 経路の `dropped_submits` は欠陥ではなく bounded pipeline の設計意図と判定された。本書はその drop を契約として明記する。**現行実装の挙動を記述するものであり、新しい挙動を追加する spec ではない。**

## 適用範囲

`crates/ruviform-plugin-host/src/l1/` の host 側 data plane(`RtLeg` の submit 経路)。child plugin process・driver・reader 経路は completion 契約に限って言及する。

## 契約

### C-1: RT caller は submit のために block しない

`RtLeg::process` は RT-safe caller。block submit は必ず `try_push`(非 blocking)で行われ、失敗しても待機・再試行・ロック取得をしない。

### C-2: drop 条件はちょうど 2 つ

`dropped_submits` を increment するのは次の 2 分岐のみ(いずれも seq は消費される)。`ruviform#895` 以降、各分岐は集約 `dropped_submits` と自身の原因 counter(`dropped_submits_slot_busy` / `dropped_submits_chan_full`)の両方を increment する:

1. **slot still busy** — 対象 shm slot(`seq % depth`)の前 occupant がまだ complete していない(`cleared[slot] < last_pushed[slot]`)。この分岐は `try_push` に到達しない。`dropped_submits` + `dropped_submits_slot_busy` を increment。
2. **submit channel full** — slot gate は成立したが `submit.try_push` が bounded channel 容量(`depth`)で拒否した。`dropped_submits` + `dropped_submits_chan_full` を increment。

したがって `dropped_submits` と各原因 counter はいずれも 1 block あたり高々 +1(busy slot は `try_push` に到達しない)であり、`dropped_submits == dropped_submits_slot_busy + dropped_submits_chan_full` が構築により常に成立する。counter doc(`RtLegCounters::dropped_submits`)がこれを保証する。

補足:channel capacity = `depth` の下では chan-full 分岐は in-protocol 到達不能(channel が満杯なら全 slot が busy で gate が先に落とす)であり、`dropped_submits_chan_full` は `results_dropped` と同種の defensive counter として維持される。非ゼロが観測された場合は capacity/slot 不変条件の破損を意味する。

### C-3: drop 時の出力は defined pass-through

drop が起きた block は `passthrough_submits` も increment し、出力として `last_output`(まだ出力を受け取ったことがなければ silence)を返して `Ok(ProcessResult::CONTINUE)` で継続する。module doc の decision table の定義行:「slot still busy / submit channel full → `last_output`(silence if never received)」。

`last_output` は stale replay(前回の非ゼロ出力の繰り返し)であり silence とは限らない点に注意 — E2E oracle 設計で既知(`ruviform#763` mechanism-1)。

### C-4: drop された seq に completion は発生しない(発生する必要もない)

slot-busy drop は submit 自体を行わないため、対応する completion は存在しない。slot の occupant は在飛行中の `last_pushed` のままで、その completion が `cleared[slot]` を進める。completion 会計契約(全 submitted seq がちょうど 1 個の completion を yield)は submit された seq にのみ適用されるため、drop で会計が破綻・wedge することはない。

### C-5: completion 会計不変条件

submit された seq はちょうど 1 個の completion element を順序どおり yield する:

- 実 `BlockDone`/failed reply(child 応答)、または
- driver 側 rejection(stale generation・unarmed・destroying・failed・creating・dead child・shutdown)に対する `mark_failed` 合成。

この不変条件が `cleared` の進行と strict alternation を保つ。reader-side の result channel `try_push` 失敗は `results_dropped` で defensive に計数されるが、在飛行 completion は高々 `depth` 個のため in-protocol では発生しない。

### C-6: drop の発生頻度は環境依存で、存在自体は異常ではない

`dropped_submits > 0` は child process の round-trip が caller の block cadence × `depth` ブロック窓を超えたことを意味する queue shedding であり、それ自体は正常動作。頻度は child scheduling 粒度・spawn transient・負荷に依存する。E2E oracle 観点では、drop は「出力が substitute される defined 経路」として扱い、frame-exact PCM 一致は drop block には期待できない(`ruviform#763` の oracle 分類の前提)。

## 設計関係(判定根拠の構造的事実)

| 要素 | 値 / 場所 |
| --- | --- |
| slot 数 `depth` | `lookahead_blocks + 1`(test では `L1_LOOKAHEAD=2` → 3) |
| slot 選択 | `seq % depth` |
| submit channel capacity | `depth` |
| result channel capacity | `depth` |
| driver submit drain 周期 | `DRIVER_POLL_INTERVAL` ≈ 200µs(`driver_main` の recv timeout) |
| `cleared[slot]` 進行 | result channel 経由の completion のみ(`drain_to`) |

## 検証根拠

`evidence/rf767/` — 60 run(両 ABI 並行 × 30 iter)で drop 1,885 件、全件 slot-busy。chan-full・`results_dropped`・`results_failed_observed` は 0 件。判定の再現手順と数値は `evidence/rf767/reproduction.md`・`verification.md` を参照。

## 対象外

- `dropped_submits` の条件別分離は `ruviform#895` で実施済み — 上記 C-2 の原因 counter(`dropped_submits_slot_busy` / `dropped_submits_chan_full`)と `docs/specs/issue-895-dropped-submits.md` を参照。本 spec は集約 counter 側の契約(2 分岐・高々 +1/block・defined pass-through)の正本であり続ける。
- fault/stateful 系 staleness oracle(`ruviform#768` 領域)への影響評価は同 Issue 側の scope。
