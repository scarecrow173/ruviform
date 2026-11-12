---
id: "IR-Dq3ecXwX1PJIko0O9pISR5"
type: "impl"
status: "completed"
title: "Issue #238 実装記録 — l1_proxy_e2e 壁時計 sleep 同期を barrier/有界 poll へ置換"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/238"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-238-l1-proxy-e2e-flake.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-238-l1-proxy-e2e-flake.md"
    - "docs/plans/issue-238-l1-proxy-e2e-flake.md"
    - "docs/test-specs/issue-238-l1-proxy-e2e-sync.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-238-l1-proxy-e2e-flake.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-238-l1-proxy-e2e-flake.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-238-l1-proxy-e2e-sync.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #238 実装記録 — l1_proxy_e2e 壁時計 sleep 同期を barrier/有界 poll へ置換

## Summary

`l1_proxy_e2e.rs` の壁時計 `thread::sleep` 同期を全廃し、配信系は 2 round-trip `SaveState` pipeline barrier、カウンタ系は構造的上界、heir/drain 系は `freeze()` の `drain_cv` 待機へ置き換えた。assertion 式は 1 行も変更していない（`git diff` で全 `-` 行が sleep/コメントのみであることを確認）。変更ファイルは `l1_proxy_e2e.rs` のみ（+130/−35）。

## Diagnosis（実証済み）

- 根因: leg の submit gate `cleared[slot] >= last_pushed[slot]`（`data.rs`）は drain 済み seq でのみ slot を解放し、drain は `process` 呼び出し中にのみ走る。固定 sleep は「call N−1 の drain 終了までに seq N−D の completion が reader に push される」ことを仮定していた — Windows runner の scheduler stall がこれを破った（#236 `dropped_submits` 4 vs 0、#237 lookahead 未配送）。
- barrier の順序証明: `driver_main` は 1 iteration で `DriverCmd::Command` を stdin へ書いてから `drain_submits` で全 instance の submit channel を drain する（`mod.rs:1607+1665`）。stdin/stdout は直列、child（fixture worker と real runner の両方）は単一スレッド直列 loop、reader は 1 スレッドで順に demux するため、2 本目の `SaveState` の reply が返る時点で先行する全 `BlockDone`/flagged-`Reply` は result channel に push 済み。`command_timeout` で有界。
- hung child 系（`--hang-after-msgs=5`）は reply が一切来ないため barrier 不可 — 代わりに ring depth 3 / submit channel capacity = depth の構造的上界で `dropped_submits == CALLS−3` が timing 非依存であることを確認（子は hung 中も stdin を読み続けるため pipe も詰まらない）。
- heir 3 テストの 700 ms は `converge_child` → `mark_drain_bounded` → `freeze()` が `drain_cv` で `DrainState::Done`（drain 自身の ~500 ms deadline または EOF）を有界待機するため冗長だった。notification pop 自身が bounded wait になる。

## Changes（`crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs`）

- `pipeline_barrier(&mut L1Instance)` と `composite_e2e::pipeline_barrier(&mut PluginHost, PluginHandle)` を新設。`Err` は lifecycle mirror + counter dump 付き panic、message-counter fault flag との併用禁止を doc に明記。
- 9 配送系 call site（`process_roundtrip`/`process_underrun` refill/`bundled_child_hosts_independent_instances`/`rejected_block_replies`/`interleave_disjoint_blocks`/`mixed_routes_full_series`/`runner_bin_serves_real_gain`/`runner_bin_serves_bundled`×2 loop/`dedicated_route_isolates`）を call 毎 barrier 化。bundled は 1 barrier が共有 stdin 経由で全 instance queue を覆う。
- `audio_counters_track_rejected_blocks`（lookahead 8→depth 9）を call 毎 barrier 化 — `dropped_submits==0`・`results_failed_observed∈[10,18]`（実測は決定的に 17）が構造化。
- hung 2 テストの sleep 除去（構造的上界コメント付き）。heir 3 テストの 700 ms 除去。
- 残留 `thread::sleep` は bounded-poll wait helper ×5 のみ（93/101/2225/2250/2270）— 正しい形。

## Verification（実測、Windows 11 x64 worktree）

| 検証 | 結果 |
| --- | --- |
| `mise run fmt-check` | PASS |
| `mise run check`（`--workspace --all-targets`） | PASS |
| `mise run lint`（clippy `-D warnings`） | PASS |
| `mise run test`（workspace 全量、112 suite 全 `test result: ok`） | PASS — `l1_proxy_e2e` 172/172 含む |

### Repeated l1_proxy_e2e evidence

`mise exec -- cargo test -p ruviform-plugin-host --test l1_proxy_e2e`（runner bin は `cargo build -p ruviform-plugin-export --bin ruviform-plugin-l1-runner` で事前ビルド — `-p` 単体では bin が作られず既存の既知制約）:

| Run | threads | 結果 |
| --- | --- | --- |
| 1 | default | 172/172 PASS（10.7 s） |
| 2 | 16 | 172/172 PASS（10.7 s） |
| 3 | 16 | 172/172 PASS（10.7 s） |
| 4 | 1 | 172/172 PASS（21.3 s） |
| 5 | 1 | 172/172 PASS（20.5 s） |
| 6 | default | 172/172 PASS（10.7 s） |
| 7 | default | 172/172 PASS（10.7 s — review 後の doc 追加のみ変更で再確認） |

直列化・並列度調整は採用せず — barrier が wait 機構で flake を閉じ、各 test は固有 child/process を持つため資源競合なし（plan の判断通り、7 連続 green が裏付け）。

## Review（review-protocol）

- 客観チェック: fmt/check/lint/test + 反復実行（上表）— 全 PASS。
- Reviewer A（cooperative）/ B（adversarial）を並行 dispatch — 両者とも **Issue Candidate ゼロ**。barrier の wire-order 証明、 bundled 共有 stdin 網羅、hung 構造的上界、freeze/`drain_cv` 同期、assertion 非変更を双方が独立に検証。
- Judge: CONFIRMED P0/P1 なし → **ACCEPT WITH NON-BLOCKING NOTES**。非 blocking 指摘（「popped by next drain」の表現が over-promise、message-counter footgun の doc 注記）は両 helper の doc に反映済み。

## Followups

- `l1_supervisor_e2e.rs`・`l1_video_e2e.rs`・export 側 `l1_runner_e2e.rs` にも同種の sleep 同期が残る可能性 — Issue 範囲外、別途監査対象として報告。
- barrier helper の前提（drain-after-command per iteration）は `driver_main` の構造に依存 — 将来 driver loop の順序を変える変更時は本 doc/test-spec を再参照すること。
