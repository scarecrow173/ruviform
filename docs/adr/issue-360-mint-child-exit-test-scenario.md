---
id: "ADR-034SmAdWeS6quuMM29Wgfx"
type: "adr"
status: "accepted"
title: "Issue #360 mint 窓内 child exit の責務境界 — 順序付けは test scenario 側に置く"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/360"
    - "docs/specs/issue-360-l1-proxy-e2e-heir-flake.md"
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
  implemented-by: []
  depends-on:
    - "docs/specs/issue-360-l1-proxy-e2e-heir-flake.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-360-heir-exit-post-mint-ordering.md"
    - "docs/designs/plugin-l1-out-of-process.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-360-l1-proxy-e2e-heir-flake.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# Issue #360 mint 窓内 child exit の責務境界 — 順序付けは test scenario 側に置く

## Context and Problem Statement

`--leave-heir-after-msgs=2` を使う `l1_proxy_e2e` テストは「fixture worker が `Scan`(msg1)・`CreateInstance`(msg2) を回答した直後に heir を残して自発 exit」する。host 側では reply demux → waiter 完了 → caller の `flip_live`(`inner` lock) と、driver の `try_wait` poll(200 µs)→`converge_child` が競合し、exit の観測が `flip_live` に先行すると mint は mid-mint death として `SpawnFailed{ExitObserved}` に折りたたまれる（defined contract）。テストの意図は「mint 成功後の mid-life 自発 exit」なので、この contract を正しく使うか contract 自体を見直すかが決定点。

## Decision Drivers

- mid-mint death → `SpawnFailed` は `flip_live`/`create_rollback_error` が実装する設計済み contract で、dead child 上に live facade を mint しないためのもの。両順序とも内部整合的で product はどちらでも正しい。
- テストが検証したいのは「live な instance を残したままの自発 exit + heir が pipe を保持」という drain deadline シナリオであり、mint window 内 exit はそのシナリオではない。
- 因果順序（msg が host に送れる条件）で同期する手法は Issue #238 の sleep→barrier 置換と同型で、wall-clock 仮定を持ち込まない。
- 変更は最小限・一箇所の責務に収めること（AGENTS.md「正しい境界で修正する」）。

## Considered Options

- **Option A（採用）**: test scenario 側で exit を post-mint へ因果順序付け。`--leave-heir-after-msgs` を `3` に上げ、mint 成功後に host から 1 command（`Initialize`）を送る — 3rd message は `Instantiate`/`L1Instance::spawn` 復帰（= `flip_live` 解決）まで送り得ないため、exit が mint window 内に起こることが構造的に不可能になる。
- **Option B**: product 側で「`Ok` reply 着陸済みなら exit 観測が先行しても mint 成功扱い（live mint + `InstanceFailed` notification）」に変更。**却下** — `flip_live` の `went_live` ゲートが意図的に実装する defined contract を反転させ、dead child 上の live facade を `Instantiate` 成功として返すことになる。`SpawnFailed` で失敗を知りたい既存の呼び出し側期待（mid-mint death 系テスト群・`create_rollback_error` の cause 分類）を壊す。
- **Option C**: driver の `try_wait` 収束を `Creating` entry が解決するまで遅延。**却下** — 収束経路に mint 待ちを入れると waiter 起床・強制 kill・`Failed` 遷移が遅れ、wedged child の bounded teardown 保証を損なう。責務の分散を悪化させる。
- **Option D**: テストの oracle を `{Instantiated, SpawnFailed{ExitObserved}}` に緩和。**却下** — `SpawnFailed` では instance が mint されず drain deadline シナリオ自体が成立しないため、後続 assertion 全部が空洞化しテストの存在意義を失う（失敗握り潰しに等しい）。
- **Option E**: sleep・retry・spawn 間隔の調整。**却下** — timing 仮定の置き換えであり、Issue #238 で排除した flake 構造の再発。タスク上も禁止。

## Decision Outcome

Chosen option: **Option A** — 自発 exit の順序付けは「exit させたいシナリオを作る test scenario の責務」とする。`--leave-heir-after-msgs` の値を handshake 完結（`Scan`+`CreateInstance`）後の 3rd message にし、mint 復帰後に 1 command を送って leave を発火させる。product code は変更しない。

## Implementation Plan

* Affected paths: `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs`（4 サイト）、`crates/ruviform-plugin-host/src/bin/l1-test-worker.rs`（flag 契約コメントのみ）。
* Constraints: 既存 assertion 不変条件は維持。3rd command の結果 oracle は `{Ok(_), Err(HostError::Ipc)}` — reply 送出後の exit 収束が demux に先行し得る残存 race を defined outcome として明示する。

## Verification

* [x] exit が mint window 内に起こり得ない構造であることを message 順序で説明できる（design 参照）。
* [x] `mise run fmt-check` / `lint` / `test` green（Windows 11 + Rust 1.99.0）。
