---
id: "IR-Rf242SpawnHeirBarrier01"
type: "impl"
status: "completed"
title: "Issue #242 実装記録 — 残存 L1 E2E suite の wall-clock settle を `--spawn-heir` 決定的 converge barrier へ置換"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/242"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_supervisor_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/bin/l1-test-worker.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-238-l1-proxy-e2e-flake-implementation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/issue-238-l1-proxy-e2e-flake.md"
    - "docs/plans/issue-238-l1-proxy-e2e-flake.md"
    - "docs/test-specs/issue-238-l1-proxy-e2e-sync.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/impl/ir/issue-238-l1-proxy-e2e-flake-implementation.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "crates/ruviform-plugin-host/src/l1/mod.rs"
    - "crates/ruviform-plugin-host/src/supervisor.rs"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #242 実装記録 — 残存 L1 E2E suite の wall-clock settle を決定的 converge barrier へ置換

## Summary

残存 3 suite の sleep 監査の結果、真の wall-clock ペーシングは `l1_supervisor_e2e.rs` の `settle_notification()`（`thread::sleep(50 ms)`、7 call site）のみだった。これは「`wait_for_failed` が mirror 反転を観測してから driver reap が `Pending → Sent` を完了するまで」の ~200 µs ウィンドウを覆っていた。公開 API には `Sent` を消費せず観測する手段が存在しないため、代わりに **Pending ウィンドウ自体を消去** する: worker fixture に `--spawn-heir` 旗を追加し、heir が stdout pipe を保持することで reader の EOF converge を不成立にし、driver `try_wait` reap を唯一の converge にする。`converge_child` の first-converge は `exit: Some` 経路で `notify = Sent` + send を mirror 反転と**同一 critical section** で行うため、`wait_for_failed` が `Failed` を観測した時点で notification は必ず queued — 決定的 barrier。assertion 式は 1 行も変更していない。

## Context

- 正本要件: Issue #242（<https://github.com/scarecrow173/ruviform/issues/242>）
- 前提: Issue #238 の実装記録 `docs/impl/ir/issue-238-l1-proxy-e2e-flake-implementation.md`（`depends-on`/`derives-from`）の Follow-ups に挙がっていた sibling-suite 監査を本 Issue で完了したもの。
- 関連文書: Issue #238 の task/plan/test-spec（`docs/tasks/issue-238-l1-proxy-e2e-flake.md`、`docs/plans/issue-238-l1-proxy-e2e-flake.md`、`docs/test-specs/issue-238-l1-proxy-e2e-sync.md`、`related`）— 本 Issue はそれらの残存 suite への適用。
- 参照実装: `crates/ruviform-plugin-host/src/l1/mod.rs`（`converge_child`）、`crates/ruviform-plugin-host/src/supervisor.rs`（`references`）。

## Diagnosis（実証済み）

- `settle_notification` の目的: `converge_child`（`l1/mod.rs:870-960`）は death を2段階で処理する — reader の stdout EOF が先に来ると `inner.terminal = Failed` + `notify = Pending`（exit 不明）+ mirror 反転、続いて driver loop 末尾の `try_wait` reap が `Pending → Sent` を完了する（real exit + cause を乗せて送信）。`wait_for_failed` は mirror しか見ないため Sent を保証しない。
- `Sent` の観測可能性: `try_recv_notification` は consume する（pop は supervisor の `poll` 専用 drain — consume すると watch 側に届かない）。`lifecycle_state`/`audio_counters` は Sent 以前の状態。`Destroy` は `remove_instance` で flush するが facade drop で channel も死ぬ（watched entry には通知が届かない）。`freeze()`/`drain_cv` は stderr drain thread の完了を見るので notification send とは無関係。公開 API で「consume せず post-Sent を観測」する手段は存在しない。
- 一方 `converge_child` の first-converge（`exit: Some`、driver `try_wait` 経路）は `notify = Sent` + `notify_tx.send` を mirror 反転と同じ `inner` critical section で実行する。**reader の EOF converge を潰せば、mirror 観測 = Sent = queued が不変条件になる。**
- EOF を潰す機構は既に fixture にある: `--leave-heir-after-msgs` が spawn する stdio 継承 heir は parent の stdout write-end を握り続け、reader の `read` が EOF を返せなくする（drain/heir テストで実績あり）。同 heir を startup に spawn して serve を継続させれば、後続の `kill`/crash knob 死亡でも同じことが起きる — それが `--spawn-heir`。

## Implementation

### Changes

変更対象は test fixture 1 件と E2E suite 2 件（うち 1 件は監査のみで無変更）。

#### `crates/ruviform-plugin-host/src/bin/l1-test-worker.rs`（test fixture）

- `--spawn-heir` 旗を追加: startup で `--sleep-ms=15000` の stdio 継承 heir を spawn してから通常 serve を継続。既存の heir spawn を `spawn_heir(&Flags)` に抽出して `leave_heir_after_msgs` 経路と共有。既存旗の挙動・既定 argv は不変（strictly additive）。
- argv doc 表に `--spawn-heir` 行を追加。

#### `crates/ruviform-plugin-host/tests/l1_supervisor_e2e.rs`

- `settle_notification()` 削除（7 call site すべて除去）。
- `--spawn-heir` を death-bearing host の argv に追加 — `wait_for_failed` が「全 co-resident notification queued」を保証:
  - `mid_create_death_attempt_failed_without_mint`（bundled, torn drain が `(next_attempt_at, id)` 試行順を反転させ得た）
  - `gaveup_lingering_late_death_reemits`（dedicated, lingering match が Phase-2 retry に先立つ必要あり）
  - `adopt_failed_handle_recovers`（bundled, slot-order pop が `a` を先に取る必要あり）
  - `stable_window_crash_starts_fresh_episode`（dedicated, Phase-1 drain reset が Phase-2 sweep に先立つ pin）
  - `bundle_burst_cap_defers_and_unsupervised`（bundled, 5+4 fan-out が torn drain 不可、unwatched は real exit payload 必須）
- `lingering_residue_retry_destroy_clears` は settle のみ削除（旗なし）— retry か lingering match のどちらが先に来ても `current_handle == None`・`is_empty`・`unwatch == None` は同一結果で、named path（Phase-2 retry `Destroy`）は `poll_until_cleared` 初回 poll で必ず走る。
- `wait_for_failed` doc に `--spawn-heir` 時の通知 barrier 不変条件を記録。各 call site の settle 説明コメントを `--spawn-heir` 不変条件の説明へ置換。

#### `crates/ruviform-plugin-host/tests/l1_video_e2e.rs` / `crates/ruviform-plugin-export/tests/l1_runner_e2e.rs`

- **変更なし（監査結論）**。残る `sleep` は全て deadline 有界 poll 内の retry backoff（`wait_failed`/`wait_notification`/`assert_no_notification`/`MiniHost::wait_exit`・spawn/handshake deadline loop）— issue-238 plan/test-spec が明示的に保持した INV-2/INV-5 の正しい形。video は notification を consume する pop 型 wait のみで mid-flight drain hazard がなく、runner は `try_wait` poll のみ。wall-clock ペーシングは存在しない。

### Decisions During Implementation

- `Sent` を consume せず観測する公開 API が存在しないため、観測ではなく Pending ウィンドウ自体を消去する方針を採用 — worker fixture に `--spawn-heir` を追加し、heir が stdout pipe を保持することで reader の EOF converge を不成立にし、driver `try_wait` reap を唯一の converge にした。
- 既存の heir spawn を `spawn_heir(&Flags)` に抽出して `--leave-heir-after-msgs` 経路と共有 — 既存旗の挙動・既定 argv は不変（strictly additive）。
- `lingering_residue_retry_destroy_clears` は旗なしで settle のみ削除 — retry/lingering どちらが先に来ても assertion は同一結果で、named path（Phase-2 retry `Destroy`）は `poll_until_cleared` 初回 poll で必ず走る。
- `l1_video_e2e`/`l1_runner_e2e` は監査の結果変更なし — 残る sleep は全て deadline 有界 poll 内の retry backoff で、wall-clock ペーシングは存在しない。
- 直列化・並列度調整は採用せず — `--spawn-heir` が convergence 順序を決定的にする（Verification 節の反復 green が裏付け）。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

実測環境: Windows 11 x64 worktree。

| 検証 | 結果 |
| --- | --- |
| `mise run fmt-check` | PASS |
| `mise run check`（`--workspace --all-targets`） | PASS |
| `mise run lint`（clippy `-D warnings`） | PASS |
| `mise run test`（workspace 全量） | 下表参照 |

### Repeated target-suite evidence（`mise exec -- cargo test ...`）

| Suite | threads | 反復 | 結果 |
| --- | --- | --- | --- |
| `l1_supervisor_e2e` | default | 4（初回 + 3 連続） | 23/23 PASS 全回（~2.3 s/回） |
| `l1_supervisor_e2e` | 1 | 1 | 23/23 PASS（7.35 s） |
| `l1_supervisor_e2e` | 16 | 1 | 23/23 PASS（2.25 s） |
| `l1_video_e2e` | default | 10（初回除く） | 36/36 PASS 全回（~3.1 s/回） |
| `l1_runner_e2e` | default | 4 | 21/21 PASS 全回（~0.5 s/回） |

- 変則記録: `l1_video_e2e` の最初の実行のみ `31 pass/5 fail` — worker binary 直後ビルドの cold-spawn 一過性（以降 10 連続 green）。video suite は本変更の対象外・旗非使用で挙動不変。
- 直列化・並列度調整は採用せず — `--spawn-heir` が convergence 順序を決定的にし、各 test は固有 child/process を持つ。
- `git merge origin/main`（`2a1b309` 取込、conflict なし）後に `mise run fmt-check`・`l1_supervisor_e2e`（23/23）を再確認 — 全 PASS。

### Review（review-protocol）

- 客観チェック: fmt/check/lint/test + 反復実行（上表）— 全 PASS。
- Reviewer A（cooperative）: `--spawn-heir` の不変条件（heir が stdout write-end を保持 → reader EOF 不成立 → driver `try_wait` が唯一の converge → `converge_child` exit-Some 経路で `Sent`+send が mirror 反転と同一 critical section）は `l1/mod.rs:870-960` の converge 実装と整合。`wait_for_failed` 観測 ⇒ queued は critical-section ordering で証明される。
- Reviewer B（adversarial）: (a) `Destroy` flush で channel が死ぬ経路・`try_recv` consume・`freeze()` の非適合性を独立に排除 → `Pending→Sent` の non-consuming 観測が公開 API に存在しないことを確認、(b) `--spawn-heir` は既存旗・既定 argv に無影響（additive）、(c) 957 は旗なし削除だが retry/match 両経路で同一 assertion、(d) heir は 15 s で死滅、reader は detached で leak なし。
- Judge: P0/P1 なし → **ACCEPT**。非 blocking 注記: fixture への追加は「product-code 非変更」制約に対して test-only binary（`src/bin/l1-test-worker.rs` は E2E fixture で library に含まれない）であることを明記 — 本 doc の Changes 節に記録済み。

## Risks

- `--spawn-heir` の不変条件は `converge_child` が first-converge（`exit: Some` 経路）で `Sent`+send を mirror 反転と同一 critical section で行うことに依存する。converge 実装を変える変更で barrier が静かに無効化され得る。
- heir は `--sleep-ms=15000` で死滅する。15 s を超えて収束しない停滞・長時間テストでは stdout write-end が解放され、reader の EOF converge が復活して flake が再発し得る。
- `l1_video_e2e` の最初の実行で worker binary 直後ビルドの cold-spawn 一過性と思われる 31 pass/5 fail を 1 度観測した（以降 10 連続 green、対象外 suite で旗非使用・挙動不変）。

## Follow-ups

- `--spawn-heir` の不変条件は `converge_child` が first-converge で `Sent`+send を同 section で行うことに依存 — converge 実装を変える変更時は本 doc/test 側 invariant を再参照すること。
- `l1_video_e2e`/`l1_runner_e2e` は現状 pacing sleep なし（本監査で確認）— 将来 drain-during-flight 型の hazard を持つテストを足す場合は同じ `--spawn-heir` 機構を使える。
- issue-238 record の Followups に挙がっていた sibling-suite 監査は本 Issue で完了。
