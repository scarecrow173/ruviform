---
id: "DISC-Rf767SbmtDrpXv4nQyZk8Hw2"
type: "discovery"
status: "resolved"
title: "Issue #767 dropped_submits の周期的発生 — L1 submit 経路の drop 原因調査"
created: "2026-11-22"
updated: "2026-11-22"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/767"
    - "crates/ruviform-plugin-host/src/l1/data.rs"
    - "crates/ruviform-plugin-host/src/l1/mod.rs"
    - "docs/discovery/issue-763-flaky-e2e.md"
    - "evidence/rf767/"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-767-submit-drop.md"
      - type: "file"
        path: "docs/specs/issue-767-submit-drop.md"
      - type: "file"
        path: "evidence/rf767/"
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
    - "docs/discovery/issue-763-flaky-e2e.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
---

# Issue #767 — dropped_submits の周期的発生: L1 submit 経路の drop 原因調査

## 結論

**設計意図(bounded queue shedding)と判定。production code の変更は行わない。**
drop 契約は `docs/specs/issue-767-submit-drop.md` に明記した。

- 観測された drop は **100% slot-busy 分岐**(`submit channel full` 分岐は 60 run で 0 件)。
- drop 頻度は「各 test run の先頭ブロック列(spawn 直後の transient)+ OS scheduling jitter」で説明でき、周期的タイマや固定 call index の不具合ではない。
- 全 drop が decision table(module doc `data.rs:65` 付近)の defined pass-through 経路に乗り、`cleared`/completion 会計の不整合(`results_dropped`・`results_failed_observed` の増加、wedge)は 60 run 全件で観測されなかった。

## 調査方法

`ruviform#763` と同じアプローチ:counter は条件を区別しないため、一時的な観測 probe(環境変数 `RUVIFORM_L1_PROBE` gate、commit 前に除去)で帰属を取った。

1. `data.rs` の 2 drop 分岐に `eprintln` probe を挿入し、`kind`(slot-busy / chan-full)・`seq`・`slot`・`last_pushed[slot]`・`cleared[slot]` を記録。
2. 両 ABI の `l1_real_test_plugin_e2e.rs` に per-call probe を挿入し、`passthrough_submits`/`dropped_submits`/`results_failed_observed`/`results_dropped` の delta を記録。
3. instrumented binary を `evidence/rf767/bin/` に stage し、`loop-observe.sh`(両 binary 並行 × 30 iter、`--nocapture` 必須 — harness は pass した test の stderr を捨てる)で観測。
   - 補助証拠扱い:canonical lane は `mise run test-l1-real-plugin`。

## 発生条件の構造的確認

`RtLeg::process`(`data.rs` submit 経路)の drop site は 2 条件のみ。

- **slot still busy**(`data.rs:692` 付近):gate `last_pushed[slot].is_none_or(|p| cleared[slot] >= p)` が false。seq は消費されるが submit せず pass-through。
- **submit channel full**(gate 成立後の `submit.try_push` 失敗):staging 済みでも doorbell できず pass-through。

周辺の設計関係:

- `depth = lookahead_blocks + 1`(test 側 `L1_LOOKAHEAD = 2` → `depth = 3`)。slot は `seq % depth` で選ばれ、前 occupants(seq-3)の completion が `cleared[slot]` に反映されるまで再利用不可。
- submit/result channel の capacity はともに `depth`。driver は `DRIVER_POLL_INTERVAL`(約 200µs)周期で `drain_submits` を回す。
- submit された全 seq はちょうど 1 個の completion(実 `BlockDone`/failed reply、または driver 側 rejection の `mark_failed` 合成)を順序どおり yield する(module doc の completion 契約)。slot-busy drop の seq は submit 自体が起きないため completion を持たず、slot の occupant は在飛行中の `last_pushed` のまま — wedge しない。

## 観測結果(`evidence/rf767/` — 補助証拠)

環境:Windows(OS-L1)、instrumented E2E binary 両 ABI 並行 × 30 iter = 60 run、全件 pass。

| 指標 | 値 |
| --- | --- |
| drop 総数 | 1,885(60 run、1 run あたり平均 ~31) |
| kind 内訳 | `slot-busy` 1,885 / `chan-full` 0 |
| `results_dropped` > 0 の call | 0 / 12,297 call |
| `results_failed_observed` > 0 の call | 0 / 12,299 call |
| pass-through(drop なし、underrun 等) | 4,514 call |
| drop seq 分布 | `seq=3..9`(各 test の先頭 D..2D ブロック)が最多、末尾 ~`0x2a` まで裾野 |

代表性サンプル(probe 出力):

```text
[rf767-probe] slot-busy seq=0x8000000000000003 slot=2 last_pushed=Some(0x8000000000000000) cleared=0x0
[rf767] block pt=0 drop=1 failed_obs=0 results_dropped=0
```

## site ごとの分類と判定根拠

### (i) slot-busy 分岐 — 設計意図(観測された全 drop がここ)

- 「slot 再利用には前 occupant の completion が必要」は strict alternation の不変条件そのもの。子(child process)の block 処理 + `BlockIn` → result channel → `drain_to` の往復が、caller の block cadence × D ブロック分の窓を超えた場合に発火する。
- 観測では各 test run 先頭(seq 3〜9、child spawn + shm handshake 直後の初回往復遅延)に集中し、以降は ~10-20 並行 child process 環境下の scheduling jitter に追随して散発。固定 call index・周期性タイマに依存しない。
- `cleared` の追跡・`mark_failed` 合成・result channel 会計に不整合は見つからなかった(`results_dropped`=0・`results_failed_observed`=0・全 run 完走)。

### (ii) chan-full 分岐 — 設計意図(観測 0 件、到達条件は正当)

- submit channel capacity = `depth` = 3、driver drain 周期 ~200µs に対し test 側 block 生成 cadence は ms オーダ。drain が 1 周期遅れても capacity 3 で吸収可能であり、60 run で 0 件は設計と整合。
- caller が driver の drain 速度を恒常的に上回る場合(RT callback が極端に短い block period で回る等)にのみ発火し得る defined 経路であり、RT 側が block しないための shedding として正当。

### (iii) driver 側 rejection(stale generation・unarmed・destroying 等)— 対象外

- `dropped_submits` を increment しない経路。該当時は `mark_failed` が合成され `results_failed_observed` に現れるが、観測では全 call で 0。本 Issue の drop 原因ではない。

### (iv) `results_dropped`(reader-side `push_result` overflow)— 対象外

- 在飛行 completion は高々 `depth` 個のため in-protocol では不可能との設計注記どおり、観測 0 件。defensive counter として機能。

## 「周期的発生」の説明

Issue の「周期」はタイマ的な周期ではなく、**各 E2E test が新しい `PluginHost`+child を spawn するたびに先頭ブロック列で再現する startup transient** が run ごとに規則的に現れるもの。`ruviform#763` が観測した「call-8 drop=1」(lookahead 関係上 seq 7-9 相当)もこの先頭 transient の一部であり、同一 mechanism で説明できる。

## 残件(対象外)

- `dropped_submits` counter は slot-busy / chan-full を区別しない。恒久的な帰属性(診断面での分離)が欲しければ別 Issue(counter 分割か `diag.rs` 面の拡張)で検討する余地あり。本調査では一時 probe で十分だったため scope 外。
- 実 audio callback(非 test pump)cadence 下での頻度は未観測。経路と契約は同一だが、頻度特性は環境依存。
- `ruviform#768`(fault/stateful E2E の staleness oracle)とは、本判定が「drop = defined 挙動」を確認した点で前提を共有する。相互リンクのみ。

## 参考

- drop 契約:`docs/specs/issue-767-submit-drop.md`
- 観測手順・集計:`evidence/rf767/reproduction.md`・`evidence/rf767/verification.md`・`evidence/rf767/loop-observe.sh`
- 先行分類(oracle 側 mechanism-1/2):`docs/discovery/issue-763-flaky-e2e.md`
