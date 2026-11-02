---
id: "SPEC-Kn4byTEKolEb9bRq7eawMU"
type: "spec"
status: "approved"
title: "Issue #238 l1_proxy_e2e Windows flake — 壁時計 sleep 同期を有界 readiness/barrier 待機へ置換する"
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
  implements: []
  implemented-by:
    - "docs/plans/issue-238-l1-proxy-e2e-flake.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
    - "docs/designs/plugin-l1-out-of-process.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #238 l1_proxy_e2e Windows flake — 壁時計 sleep 同期を有界 readiness/barrier 待機へ置換する

## Problem

`crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs` の `verify (windows-latest)` が 2 連続で flake した（無関係 PR #236: `audio_counters_track_rejected_blocks` `dropped_submits` 4 vs 0、#237: `rejected_block_replies_resolve_pass_through` lookahead 未配送）。ubuntu/macos は非発火、rerun で green。事前に #190 作業中にも同種 flake が記録されている。

## Diagnosis（変更前に診断する）

テストは実 child process + shm を、`thread::sleep` による壁時計仮定で同期させている。root cause は **submit/drain のスロット解放と結果チャネル到達を「決まった ms で間に合う」と仮定する構造**であり、CI runner の scheduler 遅延がこの仮定を破る:

- `RtLeg::process` の submit gate は `cleared[slot] >= last_pushed[slot]`（`src/l1/data.rs`）。slot を再利用するには、先行 seq の completion element が **leg の drain で pop されている**ことが必要。drain は `process` 呼び出し中にのみ走る。
- drop は「seq N−D の completion が call N−1 の drain 終了後に reader へ push された」場合に起きる。深度 D=9・10 ms ペースなら子側 latency 上限 ~80–90 ms。Windows runner の stall がこれを超えた（#236）。
- 配送アサート（`matched`/`delivered`/`processed`）は「expected seq の completion が lookahead 窓内に push される」ことに依存し、固定ペースはこれを遅い runner で破る（#237）。
- 可観測性: leg 側 counter（`audio_counters`）は `process` 呼び出し中にのみ進むため、**call 間に poll できる進捗は leg counter には存在しない**。非同期に動くのは `lifecycle_state` mirror・notification・reader 側 `results_dropped` のみ。

## Requirements

1. `l1_proxy_e2e.rs` の壁時計 sleep 同期を、有界 deadline 付き readiness/barrier 待機へ置き換える。sleep 量を増やす対処はしない（仮定そのものを消す）。
2. 各 wait 点で「条件を満たせば早期に進む／deadline 超過時は diagnostic dump 付きで失敗」にする。
3. アサーションとプロトコルカバレッジは同一の不変条件を検証し続ける（wait 機構のみ変更）。
4. アサーションを弱めない、`#[ignore]`/flaky 許容を付けない、製品コードをテストに合わせて変更しない。
5. Windows CI runner の scheduler 遅延を吸収するため、結果チャネル到達・slot 解放・stderr drain 終了を観測可能な事実に同期させる。

## Acceptance Criteria

- AC1: 全 sleep サイトの監査結果が記録され、同期が必要なサイトは barrier/有界 poll へ、不要なサイトは構造的不変性の根拠つきで除去される。
- AC2: `audio_counters_track_rejected_blocks` は `dropped_submits == 0` が構造的に保証される（barrier が各 submit の slot 解放を先行 drain で確定させる）。
- AC3: `rejected_block_replies_resolve_pass_through` は processed 配送が barrier により決定的になり、`processed >= 1` が latency 非依存になる。
- AC4: `mise run fmt-check` / `check` / `lint` / `test` が green。
- AC5: `cargo test -p ruviform-plugin-host --test l1_proxy_e2e` を複数 iteration・高低 `--test-threads` で繰り返し実行し、同期が運任せでなく robust である証跡を IR に記録する。

## Non-goals

- `src/` 側の製品コード変更（wait 機構は harness 側に閉じる）。
- 他の e2e ファイル（`l1_supervisor_e2e.rs`・`l1_video_e2e.rs`）の同等見直し — Issue 範囲外、別途報告対象。
- CI 実行直列化 — 機構で閉じられるため採用しない（決定は plan に記録）。
