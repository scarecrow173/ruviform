---
id: "IMPL-034NU3jXjckCIx8sPNr1Wz"
type: "impl"
status: "completed"
title: "Issue #116 L1 自動再起動・supervisor ポリシー — implementation"
created: "2026-10-29"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/116"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/supervisor.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_supervisor_e2e.rs"
      - type: "file"
        path: "docs/impl/ir/issue-116-supervisor-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/bin/l1-test-worker.rs"
      - type: "file"
        path: "docs/specs/issue-116-supervisor.md"
      - type: "file"
        path: "docs/plans/implement-issue-116-supervisor.md"
      - type: "file"
        path: "docs/test-specs/issue-116-supervisor-core-tests.md"
      - type: "file"
        path: "docs/test-specs/issue-116-supervisor-e2e-tests.md"
      - type: "file"
        path: "docs/tasks/l1-test-worker-fault-flags.md"
      - type: "file"
        path: "docs/tasks/l1-supervisor-core.md"
      - type: "file"
        path: "docs/tasks/l1-supervisor-e2e.md"
      - type: "file"
        path: "docs/tasks/l1-supervisor-final-verification.md"
      - type: "file"
        path: "docs/designs/issue-116-supervisor.md"
      - type: "file"
        path: "docs/adr/issue-116-supervisor-placement.md"
      - type: "file"
        path: "docs/discovery/issue-116-supervisor.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/l1-supervisor-final-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-116-supervisor.md"
    - "docs/specs/issue-116-supervisor.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-116-supervisor.md"
    - "docs/adr/issue-116-supervisor-placement.md"
    - "docs/discovery/issue-116-supervisor.md"
    - "docs/impl/ir/issue-117-bundle-child-implementation.md"
    - "docs/impl/ir/issue-114-real-adapter-final-verification-closeout-task-5-implementation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-116-supervisor.md"
    - "docs/specs/issue-116-supervisor.md"
    - "docs/designs/issue-116-supervisor.md"
    - "docs/adr/issue-116-supervisor-placement.md"
    - "docs/discovery/issue-116-supervisor.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-116-supervisor-core-tests.md"
    - "docs/test-specs/issue-116-supervisor-e2e-tests.md"
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/31"
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/113"
    - "https://github.com/scarecrow173/ruviform/issues/114"
    - "https://github.com/scarecrow173/ruviform/issues/117"
    - "https://github.com/scarecrow173/ruviform/issues/119"
    - "https://github.com/scarecrow173/ruviform/issues/120"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #116 L1 自動再起動・supervisor ポリシー — implementation

## Summary

Issue #116 T4（`l1-supervisor-final-verification`）完了 — T1–T3 で着地した supervisor 実装（`l1-test-worker` fault flags・`src/supervisor.rs` opt-in util 層 `Supervisor`/`SupervisedId`/`RestartPolicy`/`RestartRecipe`/`SupervisionEvent`/`SupervisionError`/`RestoreOutcome` + watch/poll state machine・attempt replay/cleanup/lingering・exponential backoff + `stable_window` episode reset・checkpoint bound・`usize` host binding・`tests/l1_supervisor_e2e.rs` 公開 API 経由 E2E）に対し、plan actions の最後段として **Full Gate を 1 回実施**し、AC1–AC10 + R-V1–R-V6 の証拠写像・canonical closeout（IR・status 同期・索引整合）を行った。

Full Gate **全コマンド初回 pass — 補正・rerun は一切不要**。`test` は **60 test binaries・1294 passed・0 failed・0 ignored**（#117 計測 59/1239 から +1 binary・+55 — supervisor unit 33 + supervisor E2E 22）、`test-rt` は `rt_` name filter で **実一致 137・0 failed**（#117 計測 132 から +5 — `rt_l1_supervised_process_no_alloc` 等 supervisor RT 行追加）、`l1_supervisor_e2e` **22/22**・`l1_proxy_e2e` **100/100 無 diff green**・lib supervisor unit **33/33**・opt-in lane `test-l1-real-plugin` は 3 段全 green（build → `--lib --bins` CLAP 63+bin 2・VST3 47+bin 2 → `--ignored` CLAP 3+VST3 3・実子プロセス経路）。`cargo doc -p ruviform-plugin-host --no-deps`（`RUSTDOCFLAGS="-D warnings"`）は **初回から exit 0・warning 0**（新規公開 API doc — sole-drain WARNING 含む — が全て解決済み）。既知負荷依存 flake（`wedged_child_write_converges_and_teardown_is_bounded`・`hang_converges_on_command_timeout` 系）は **未発生 — 1 発目から全 pass**。

literal sweep 最終確認: `git diff merge-base...HEAD -- crates/` は **`src/supervisor.rs`（新規 2292 行）・`tests/l1_supervisor_e2e.rs`（新規 1387 行）・`src/lib.rs`（+16 re-export）・`src/bin/l1-test-worker.rs`（+84/-10 flag 追加）の 4 file のみ** — `composite.rs`・`src/l1/`・`src/host.rs`・`crates/ruviform-plugin-core/src/wire.rs` は **byte-level 無 diff**（AC1/AC4 の機械的証明）。`supervisor.rs` の `use` は公開 API 型のみ（`crate::l1`/`composite`/`pub(crate)` 内部型への参照ゼロ — `pub(crate)` 言及は module doc の設計契約記述のみ）・`l1_supervisor_e2e.rs` は `#[path]`/`mod l1` 非使用（`pub(crate)` 非到達の compile-level 証明が維持）・core/l1/export/host.rs への `supervisor`/`Supervisor` 語彙混入ゼロ。

closeout として **AC1–AC10 + R-V1–R-V6 ↔ task/test-spec ↔ 実 test/証跡の写像表**を本 IR に記録し、spec `draft → implemented`・plan `approved → completed`・2 test-spec `proposed → approved`・4 task `todo → done`・索引 5 件（tasks・plans・specs・test-specs・impl/ir README）を同期。design Verification Notes の 12 checkbox は task doc の「checkbox 証拠化」条項どおり実績 annotate で `[x]` 化。doc audit は最終編集後に `blocking: 0`・`findings: []` を確認。**GitHub merge・issue close は lead の authoritative `gh` で別途実施 — 本 IR・本 commit は PR merged / issue closed の主張を含まない。**

## Context

approved plan `docs/plans/implement-issue-116-supervisor.md`（PLAN-034NOXmaWmUZjtJxQeNUhL）の T4（`docs/tasks/l1-supervisor-final-verification.md` — Full Gate・AC/R-V 証拠収集・IR・status 同期）。upstream: spec `docs/specs/issue-116-supervisor.md`（draft・AC1–AC10 + R-V1–R-V6）・approved design `docs/designs/issue-116-supervisor.md`（API sketch・遷移表・poll アルゴリズム・失敗分類表が正本・Verification Notes L588-603 が AC/R-V の証拠 oracle）・accepted ADR `docs/adr/issue-116-supervisor-placement.md`（util 層配置・core 無変更境界）・2 test-spec（新規 `issue-116-supervisor-{core,e2e}-tests`）。T1 `l1-test-worker-fault-flags`（`db80ead`）・T2 `l1-supervisor-core`（`47fef7c` + T2 review fix `637abae`）・T3 `l1-supervisor-e2e`（`e0b31f3` + determinism hardening `9a82dca`）は全て実装完了であり、本 task は全 task 完了後の **Full Gate 1 回のみ** + canonical closeout を担う（T1–T3 task 文書は planning 時点の `todo` のまま残っていたため、本 task で実装実績への status 同期を兼ねる）。Issue #117 `l1-bundle-child-final-verification`（T6）・#114 `real-adapter-final-verification`（Task 5）が先例 — Full Gate 実績・AC-to-test 写像・status 同期の同型 closeout。

## Implementation

### Changes

動作を変えるコード変更はゼロ — 検証記録 + 帳簿 + 正本同期のみの task。#117 T6 と異なり `cargo doc` を含む全 gate が初回 pass したため、doc-only 補正を含む source/docs への fix も不要だった。

- **Full Gate 実施（全コマンド初回 exit 0）**: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`bench-smoke`・opt-in lane `lint-l1-real-plugin`/`test-l1-real-plugin`・`cargo doc -p ruviform-plugin-host --no-deps`（`RUSTDOCFLAGS="-D warnings"` — 新規公開 API doc）・`mise exec -- prek run --all-files`・doc audit（`doc_status.js audit --json`）+ focused E2E 3 本（`l1_supervisor_e2e`・`l1_proxy_e2e`・`--lib` supervisor unit）。結果は Validation 表。
- **literal sweep 最終確認（task doc 項目 4）**:
  - `git diff fc02103...HEAD -- crates/ruviform-plugin-host/src/composite.rs crates/ruviform-plugin-host/src/l1/ crates/ruviform-plugin-host/src/host.rs crates/ruviform-plugin-core/src/wire.rs` — **0 行**（merge-base `fc02103` 対比の byte-level 無 diff。`main` は `fc02103` で merge-base と一致 — AC1/AC4 の機械的証明）。
  - `git diff --stat fc02103...HEAD -- crates/` — `l1-test-worker.rs` +84/-10・`lib.rs` +16・`supervisor.rs` +2292（新規）・`l1_supervisor_e2e.rs` +1387（新規）の 4 file のみ。
  - `grep -rn "supervisor\|Supervisor" crates/ruviform-plugin-core/ crates/ruviform-plugin-host/src/l1/ crates/ruviform-plugin-host/src/composite.rs crates/ruviform-plugin-export/ crates/ruviform-plugin-host/src/host.rs` — **0 hit**（core/l1/export/host への語彙混入なし）。
  - `grep` `supervisor.rs` — `use` は `std`・`ruviform_plugin_core::{LifecycleState, ProcessConfig, wire}`・`crate::{EndpointBounds, HostCommand, HostError, HostNotification, HostReply, PluginHandle, PluginHost}` のみ（supervisor.rs:35-42 — 全て `lib.rs` 公開面）。`pub(crate)`/`crate::l1`/`crate::composite` 参照なし（`pub(crate)` の文字は L20 module doc の設計契約記述のみ）。
  - `grep "#\[path\]\|mod l1" tests/l1_supervisor_e2e.rs` — **0 hit**（L12 の module doc「No `#[path]` harness」宣言のみ — `pub(crate)` 非到達が compile-level で維持されている）。
- **AC1–AC10 + R-V1–R-V6 証拠 matrix** — 後述 coverage 表（plan Verification Matrix を正本として使用・design Verification Notes L588-603 を oracle として照合）。
- **status 同期**: spec `draft → implemented`（実装完了・全 AC 証跡あり・`> **DRAFT**` banner を `IMPLEMENTED` 記述へ補正 — OPEN 行は全て design で確定済み、照合は後述 Decisions）・plan `approved → completed`（body `> status:` 行を全 task 完了記録へ更新）・2 test-spec `proposed → approved`（実測証跡を持つ）・4 task `todo → done`（T1–T3 は実装完了済みで planning 時点の `todo` が残っていたため本 task で一括同期 — T4 は `implemented-by` に本 IR を登録し Done When/Verification checkbox を実績 annotate で `[x]` 化）。
- **design Verification Notes checkbox 証拠化** — task doc 項目 6 の「design Verification Notes の checkbox 証拠化を含む」条項どおり、design L588-603 の 12 行を `[x]` + 実績（test 名/証跡）へ annotate（#117 では tick しなかった先例を、本 task doc が明確に要求するため対象化 — design 本文・契約記述は変更せず checkbox 行のみ）。
- **索引同期**: `docs/tasks/README.md`（4 行 `todo → done`）・`docs/plans/README.md`（`approved → completed`）・`docs/specs/README.md`（`draft → implemented`）・`docs/test-specs/README.md`（2 行 `proposed → approved`）・`docs/impl/ir/README.md`（本 IR 行追加 — 末尾追記の時系列順 convention に従う）・design/ADR/discovery は `updated:`-tick 簿記（status 行不変 — design `approved`・ADR `accepted`・discovery `active` のまま #117 先例どおり）。

### Whole-issue scope（4 task 集約）

| Task | Task 文書 | scope | Commit |
| --- | --- | --- | --- |
| 前提 | —（discovery・draft spec・approved design・accepted ADR・plan・test-spec×2・task×4 文書パッケージ） | — | `5254df2`・`f9a8c48`・`98c9f66` |
| T1 | `l1-test-worker-fault-flags.md` | `--fail-restore`（`RestoreState` → `Err(PluginFailed)` 不 commit）・`--state-unsupported`（`Err(Unsupported)`・併用時優先）・`--fail-initialize`（`Created` 残留）・`--fail-start-processing`（`Activated` 残留）+ argv doc table・`&Flags` dispatch 整理 | `db80ead` |
| T2 | `l1-supervisor-core.md` | `src/supervisor.rs`（2292 行 — 公開型群・entry state machine・poll Phase 0-3・attempt/replay/cleanup/lingering・backoff・checkpoint bound・`usize` host binding・`#[non_exhaustive]` enum+struct variant・unit test 33）+ `lib.rs` re-export | `47fef7c` + `637abae`（T2 review fixes — `Orphaned` variant 追加・adopt `max_attempts=0` gate・`set_checkpoint` 検証順序・`clear_checkpoint` `#[must_use]` 等） |
| T3 | `l1-supervisor-e2e.md` | `tests/l1_supervisor_e2e.rs`（1387 行・22 tests — `#[path]` harness 不使用の別 test target。実子 restart・restore 4 分類・bundle burst cap・mid-create 死・lingering・adopt・crash-loop `GaveUp`・opt-in 無退行・`prepare_endpoint`・`Send` witness・`rt_` supervised） | `e0b31f3` + `9a82dca`（determinism hardening） |
| T4 | `l1-supervisor-final-verification.md` | Full Gate・AC/R-V 証拠・IR・status 同期 | 本 docs commit |

### Acceptance criteria coverage（spec AC1–AC10 + R-V1–R-V6 ↔ task/test-spec ↔ 実 test/証跡）

| AC / R-V | spec 受入基準 | task / test-spec | 実 test・証跡 |
| --- | --- | --- | --- |
| AC1 | `ruviform-plugin-host` に supervisor モジュール・`PluginHost` 公開 API のみで動作・`pub(crate)` L1 machinery/`composite.rs`/`l1/` 内部型に非接触・core 型/`execute`/`try_recv_notification` 语义無変更 | T2/T3/T4 / `issue-116-supervisor-core-tests` | literal sweep — `composite.rs`/`l1/`/`host.rs`/`wire.rs` が merge-base `fc02103` 対比 **byte-level 無 diff**・`supervisor.rs` の `use` は `lib.rs` 公開面のみ（supervisor.rs:35-42）・`crate::l1`/`pub(crate)` 内部型参照ゼロ・`l1_supervisor_e2e.rs` が `#[path]`/`mod l1` 非使用（別 test target が `pub(crate)` 非到達を compile-level で強制 — AC1 の構造的証明）・`grep` supervisor 語彙 core/l1/export/host.rs 0 hit |
| AC2 | `InstanceFailed` → `Destroy{failed}` → `Instantiate{plugin_id}` 同 route・新 `PluginHandle` mint・isolation 継承・他 route `plugin_id` 非発生 | T3 / `issue-116-supervisor-e2e-tests` | `restarted_dedicated_replays_recipe_and_rewires`（`tests/l1_supervisor_e2e.rs:347` — Dedicated `kill_child` → `Restarted{old,new}`・新 mint generation bump・旧 handle `Err(InvalidHandle)`）・`restarted_bundled_reaches_each_recipe_state`（404 — Bundled 同経路・minimal/activated/processing recipe 各到達状態）・`poll_restarts_pending_entry`/`poll_restart_replays_recipe`（`src/supervisor.rs:1945,1972` — `Instantiate{recipe.plugin_id}` で登録 route を継承） |
| AC3 | Bundled `kill_child`（1 子死亡）→ N supervised `InstanceFailed` 同一 `ChildExit` → 各々 `Destroy`→`Instantiate` 1 回ずつ・全員新 handle 復旧・supervised 外は `Unsupervised`・N 子強制しない | T3 / `issue-116-supervisor-e2e-tests` | `bundle_burst_cap_defers_and_unsupervised`（`l1_supervisor_e2e.rs:1142` — Bundled 4 supervised + 1 unwatched `kill_child` → 同一 `ChildExit` N 通知 drain → `max_attempts_per_poll=2` cap で残りは次 poll 繰越、全員 `Restarted`・unwatched は `Unsupervised` pass-through・子数は pool 判定で fresh 1 子収束）・`poll_caps_attempts_and_orders_ready`（supervisor.rs:2108 — ready 順 SupervisedId 昇順 determinism + cap） |
| AC4 | pre-failure `checkpoint` blob が `Restarted` 前に `RestoreState{bytes}` 適用・`Err(Unsupported)` は復旧を妨げず skip（attempt 不消費）・未登録は `RestoreState` 非発行 | T1/T3 / `issue-116-supervisor-e2e-tests` + `issue-116-supervisor-core-tests` | `checkpoint_applied_roundtrips_state`（`l1_supervisor_e2e.rs:467` — `set_checkpoint` blob が新 instance へ `RestoreState` 適用 `restored=Applied`・round-trip で前 state 観測）・`checkpoint_unsupported_skips_without_attempt`（521 — `--state-unsupported` → `restored=Skipped`・attempt counter 不消費で `Restarted`）・`NotRequested`（`restarted_*` の minimal recipe 行 — checkpoint 未登録は `RestoreState` 非発行）・`checkpoint_pluginfailed_burns_attempts_to_gaveup`（558 — `--fail-restore` `Err(PluginFailed)` は attempt burn → budget 尽きて `GaveUp`）・`poll_restart_checkpoint_outcomes`/`poll_restore_rejection_burns_attempts`（supervisor.rs:2024,2052 — `Applied`/`Skipped`/`NotRequested`/`PluginFailed` burn 4 分類）・`set_checkpoint_wire_bound`（1580 — `WIRE_MAX_PAYLOAD` 超過 blob は `Err(CheckpointTooLarge)` で wire 到達前拒否・`NotWatched` 優先の検証順序 pin） |
| AC5 | `max_attempts` 超過で attempt 停止 + `GaveUp`・backoff 中は新 attempt 非発行（`Instant` 注入）・attempt カウンタ respawn 継承 | T2/T3 / `issue-116-supervisor-core-tests` + `issue-116-supervisor-e2e-tests` | `poll_backoff_gates_attempt_timing`（supervisor.rs:2089 — `Instant` 注入で deadline 未到達は attempt 非発行・到達で発行）・`delay_grows_exponentially_and_clamps`（1658 — `min(base·2^(n-1), max)` 数学）・`crash_loop_bounded_gaveup`（`l1_supervisor_e2e.rs:804` — Dedicated `--crash-after-msgs=3` で restart 毎に子死亡する crash-loop が `max_attempts` で停止 `GaveUp{attempts,last_error}`・attempts は respawn/generation 越しに episode 継承）・`stable_window_crash_starts_fresh_episode`（1064 — **design 回帰 pin**: `healthy_since + stable_window` 越しの散発 crash が drain 時点 pre-check で `failures=0` reset → `GaveUp` せず `Pending` → `Restarted`）・`drain_resets_stable_window_before_give_up`/`poll_window_reset_sweeps_live_entries`（supervisor.rs:1750,2158 — drain/sweep 両経路の reset）・`zero_max_attempts_adopt_gives_up_immediately`（`l1_supervisor_e2e.rs:1038`）+ `drain_zero_max_attempts_gives_up`/`adopt_zero_max_attempts_gives_up`（supervisor.rs:1801,1821 — drain 経路と adopt `Pending{next:None}` 経路の両方で `GaveUp{0,None}` — T2 review fix P1 で Phase-3 発行前検査を追加）・`inert_policy_rejected`（`l1_supervisor_e2e.rs:1316` — `max_attempts_per_poll=0` は `Supervisor::new` で `Err(InertPolicy)`）・`policy_validation_and_defaults`（supervisor.rs:1365） |
| AC6 | `GaveUp` 時に mint 済みなら `Destroy` で slot 解放・`HostError::Failed` に畳まない別語彙 | T3 / `issue-116-supervisor-e2e-tests` | `crash_loop_bounded_gaveup`（`l1_supervisor_e2e.rs:804` — `GaveUp` は `SupervisionEvent` の独立 variant・`Failed` 语义と非混同）・`gaveup_lingering_late_death_reemits`（948 — `GaveUp` 後に lingering residue が後発死した場合 `attempts`/`last_error` 不変で `GaveUp` 再 emit・`(id,attempts)` dedupe で初回と区別）・`drain_gave_up_lingering_reemits`（supervisor.rs:1899）・`cleanup_unwinds_by_mirror_state`（2209 — mirror 状態別 unwind:`StopProcessing`/`Deactivate`/`Destroy`）・entry は `unwatch` まで terminal 保持（design 確定 — `lingering_residue_unwatch_returns_handle` 917 が `Option<PluginHandle>` 返却を pin） |
| AC7 | `Instantiate` `Err(Ipc)`/`Err(InstanceLimit)`・lifecycle 再生 `Err` → attempt 失敗 + mint 済み `Destroy` 後片付け・残留リソースなし・`RestoreState` `Err(PluginFailed)`/`Err(Ipc)`/`Err(Failed)` 同分類 | T1/T3 / `issue-116-supervisor-e2e-tests` + `issue-116-supervisor-core-tests` | `mid_create_death_attempt_failed_without_mint`（`l1_supervisor_e2e.rs:725` — Bundled `--crash-after-msgs=4` が attempt 中の `CreateInstance` frame で死亡 → `Instantiate` `Err(Ipc)`・handle 未 mint・cleanup 不要・同居 sibling は別 entry で復旧）・`fail_initialize_attempt_cleans_created`（615 — `--fail-initialize` `Err(PluginFailed)` → `Created` 残留を unwind `Destroy` で除去）・`fail_start_processing_attempt_unwinds_activated`（647 — `--fail-start-processing` → `Activated` 残留を `Deactivate`→`Destroy` で除去）・`activate_over_limit_attempt_fails_prepare`（682 — `Err(Prepare)` → attempt 失敗）・**lingering 全行**: `lingering_residue_retry_destroy_clears`（849 — `--fail-start-processing --crash-after-msgs=6` が unwind `Deactivate`=6 着信死 → `Err(Ipc)`・minted `Failed` 収束 → 次 poll Phase 2 retry `Destroy` が `Failed` から合法で clear）・`lingering_residue_external_destroy_clears`（886 — 外部 `Destroy` で `Err(InvalidHandle)` → lingering clear）・`lingering_residue_unwatch_returns_handle`（917 — `unwatch` が residue handle を返却し orphan させない）・`poll_retries_lingering_cleanup`/`store_lingering_retries_occupied_slot`（supervisor.rs:2183,2256 — occupied slot の二重失敗角は `Orphaned` emit — Decisions 参照）・`Err(InstanceLimit)` は design 確定どおり公開面到達不能（`pool_eligible` が満杯子を除外 — `composite.rs` L1092-1095）のため注入せず `Instantiate` `Err` → attempt 失敗 arm で review-covered |
| AC8 | `poll(&mut host, now)` 駆動・内部スレッド/タイマー/async なし・`PluginHost` `!Send`/`!Sync` 適合・全復旧は caller 非 RT スレッド上で同期実行 | T2/T3 / `issue-116-supervisor-core-tests` + `issue-116-supervisor-e2e-tests` | `assert_impl_all!(Supervisor: Send, Sync)`・`assert_not_impl_any!(PluginHost: Send, Sync)`（supervisor.rs:1169-1171 — `bound_host` は `usize`（pointer→usize cast・deref なし）で `Send` 可能データのみ）・`host_binding_rejects_foreign_host`/`poll_binds_host_on_first_call`（1468,1513 — 初回 `poll`/`watch` で host 束縛・異なる host は `Err(WrongHost)`/`WrongHost` event）・`wrong_host_rejected_without_drain`（`l1_supervisor_e2e.rs:1238` — 他 host 通知を消費しない drain 前拒否）・`poll(&mut host, now: Instant)` signature が thread/timer を作らない単一同期呼出（全 test が同一スレッド `Instant` 注入で駆動） |
| AC9 | `Restarted{id,old,new,bounds,restored}` で `prepare_endpoint`/`latency_frames`/`lifecycle_state` が呼べる・`#[non_exhaustive]` enum event 語彙 | T2/T3 / `issue-116-supervisor-core-tests` + `issue-116-supervisor-e2e-tests` | `restarted_dedicated_replays_recipe_and_rewires`（`l1_supervisor_e2e.rs:347` — `Restarted.new` で `prepare_endpoint(bounds)`・`lifecycle_state`・block 配信再開を実子で検証）・`restarted_bundled_reaches_each_recipe_state`（404）・`#[non_exhaustive]` enum レベル（`SupervisionEvent`/`SupervisionError`/`RestoreOutcome`）+ `SupervisionEvent` struct variant レベル両方（supervisor.rs enum 定義 — `lint` `-D warnings` で `exhaustive_enums` warn=0）・`Unsupervised` pass-through（`drain_unwatched_passes_through` supervisor.rs:1922 + `bundle_burst_cap_defers_and_unsupervised`）・**`Failed` handle adopt**: `adopt_failed_handle_recovers`（`l1_supervisor_e2e.rs:986` — `lifecycle_state→Some(Failed)` handle の `watch` → `Destroy` で slot 解放（キュー残通知は落ちて二重復旧しない）→ `Pending` → `Restarted`）・`watch_registers_live_handle`/`watch_rejects_stale_handle`/`watch_rejects_unregistered_plugin_id`/`watch_rejects_duplicate_match_set`（supervisor.rs:1392,1405,1419,1434）・`unwatch_returns_handle_by_state`/`current_handle_prefers_current_then_residue`（1526,1628）・`set_recipe_replaces_and_validates`/`clear_checkpoint_reports_presence`/`poll_restart_replays_recipe`（1547,1612,1972）・`poll_l0_host_is_inert`（2202 — L0 host は通知を emit しないため inert） |
| AC10 | `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` 全パス・`doc_status.js audit` blocking 0・findings なし | T4 | 本 Full Gate — Validation 表（11 gate + focused E2E 3 本・全初回 exit 0）・doc audit 最終 `blocking: 0`・`findings: []` |
| R-V1 | bundle crash fan-out（1 `kill_child` → N `InstanceFailed` → poll → N `Restarted`） | T3 | `bundle_burst_cap_defers_and_unsupervised`（`l1_supervisor_e2e.rs:1142` — 実子 `kill_child` で N=4 supervised + unwatched fan-out → cap 繰越で N `Restarted`） |
| R-V2 | mid-`CreateInstance` 死亡 → `Err(Ipc)` で attempt 失敗・通知なし | T3 | `mid_create_death_attempt_failed_without_mint`（`l1_supervisor_e2e.rs:725` — `--crash-after-msgs=4` が attempt 中 `CreateInstance` frame で死亡・waiter `Err(Ipc)`・未 mint handle 通知なし） |
| R-V3 | `RestoreState` 拒否（`Err(PluginFailed)`/`Err(Unsupported)`）が確定分類どおり | T1/T3 | `checkpoint_unsupported_skips_without_attempt`（`l1_supervisor_e2e.rs:521` — `Err(Unsupported)` → `Skipped` skip・attempt 不消費 — design が spec OPEN を確定した方向）・`checkpoint_pluginfailed_burns_attempts_to_gaveup`（558 — `Err(PluginFailed)` → attempt burn）・`poll_restore_rejection_burns_attempts`（supervisor.rs:2052） |
| R-V4 | backoff — `Instant` 注入 deadline 跨ぎ・`max_attempts` `GaveUp`・attempt 継承・成功 reset | T2/T3 | `poll_backoff_gates_attempt_timing`（supervisor.rs:2089）・`delay_grows_exponentially_and_clamps`（1658）・`crash_loop_bounded_gaveup`（`l1_supervisor_e2e.rs:804`）・`stable_window_crash_starts_fresh_episode`（1064 — drain 時点 pre-check reset 回帰 pin）・`drain_resets_stable_window_before_give_up`/`poll_window_reset_sweeps_live_entries`（1750,2158） |
| R-V5 | AC4 無退行 — supervisor 未使用 `l1_proxy_e2e` 全テスト不変 | T3/T4 | `l1_proxy_e2e` **100/100 無 diff green**（`l1_proxy_e2e.rs` は merge-base 対比 zero-diff — test file 自体が未変更のまま pass）・`plain_host_reports_failed_no_autorestart`（`l1_supervisor_e2e.rs:1285` — supervisor 非使用の素 host は `Failed` + 通知のみ・自動再起動しない AC4 pin） |
| R-V6 | `Restarted` の `new` で `prepare_endpoint` が動く | T3 | `restarted_dedicated_replays_recipe_and_rewires`（`l1_supervisor_e2e.rs:347` — `config: Some` で `Activated` 到達 recipe の `Restarted.bounds` echo で `prepare_endpoint` 成功・RT process 経路 `rt_l1_supervised_process_no_alloc`（1334 — `assert_no_alloc` 窓内 supervised endpoint `process`）も green） |

### Per-task gate evidence（各 commit・本 gate 実測）

- T1（`db80ead`）: `l1-test-worker` fault flags — 4 flag が argv doc table + parse + dispatch に実装（`src/bin/l1-test-worker.rs` L29-32 doc・L172-179 parse）。T3 全系の注入前提として本 gate で実 pass（`--fail-restore`/`--state-unsupported`/`--fail-initialize`/`--fail-start-processing` が e2e で発火 — coverage 表）。flag 未指定 worker の挙動は `l1_proxy_e2e` 100/100 で非退行。
- T2（`47fef7c`+`637abae`）: `src/supervisor.rs` + `lib.rs` re-export — module 内 unit test **33/33 pass**（`--lib supervisor` 実測・`policy_validation_and_defaults`〜`store_lingering_retries_occupied_slot`）。review fix commit が `Orphaned` variant・adopt `max_attempts=0` 発行前検査・`set_checkpoint` 検証順序・`clear_checkpoint` `#[must_use]` を追加。
- T3（`e0b31f3`+`9a82dca`）: `tests/l1_supervisor_e2e.rs` — **22/22 pass**（実子プロセス `kill_child`/`--crash-after-msgs` 注入・`#[path]` harness 不使用の別 test target で `pub(crate)` 非到達を compile-level 強制・`l1_proxy_e2e.rs` 無 diff 維持）。
- T4（本 commit）: Full Gate 全コマンド初回 exit 0 — Validation 表。

### Spec/design/test-spec relation state

- `docs/specs/issue-116-supervisor.md` — `status: "draft" → "implemented"`・`> **DRAFT**` banner を `IMPLEMENTED` 記述へ補正（design/ADR で全 OPEN 確定済みの旨を記述 — 照合は下記 Decisions）。AC checkbox は #113/#114/#117 先例どおり未チェックのまま（証跡は本 IR coverage 表）。`verified-by` は test-spec 2 件のまま変更なし（IR は `verified-by` edge の対象外 — test-spec が verify を担う convention）。
- `docs/designs/issue-116-supervisor.md` — `status: "approved"` 維持・`updated:`-tick。契約レベルの design 差異は T2 review fix（`Orphaned` variant は review commit で design API sketch・遷移表・poll pseudocode・AC5 verification note にも反映済み — commit `637abae` が design を authoritative に保持したと message に記録）を除き観測されず。Verification Notes L588-603 の 12 checkbox は task doc 条項どおり実績 annotate で `[x]` 化（#117 では未 tick だったが本 task doc が明示要求）。
- `docs/plans/implement-issue-116-supervisor.md` — `approved → completed`・body `> status:` 行を全 task 完了記録へ更新。
- 2 test-spec（`issue-116-supervisor-core-tests`・`issue-116-supervisor-e2e-tests`）— `proposed → approved`（実測証跡: coverage 表の実 test 名・gate 実績）。
- `docs/adr/issue-116-supervisor-placement.md` — `accepted` 維持・`updated:`-tick（util 層・core 無変更の最終確認は literal sweep で実証）。
- `docs/discovery/issue-116-supervisor.md` — `active` 維持・`updated:`-tick（#117 先例どおり discovery は status 不変）。
- plan `implemented-by` → 4 task・task `implements` → plan・T4 task `implemented-by` → 本 IR — 相互 link 完備。

### Decisions During Implementation

- **spec OPEN → design 確定値の照合（task doc 要求行）**: spec Deferred Design Concerns の全 9 OPEN 行が design で確定済み — (1) handle→plugin_id 逆引き: **util 帳簿採用**（`watch` で caller が `plugin_id` を宣言・core 無変更 — spec 推奨どおり）、(2) `Err(Unsupported)` 分類: **attempt 不消費の skip 確定**（`restored=Skipped` — spec 推奨どおり）、(3) backoff 確定値: `RestartPolicy{max_attempts=3, base_delay=250ms, max_delay=30s, stable_window=60s, max_attempts_per_poll=8}` 既定 + `stable_window` episode reset、(4) handle 変化追跡: caller `unwatch`/`set_recipe` 規則 + `rebind` deferred、(5) `EndpointBounds` recipe 内包: `RestartRecipe.endpoint_bounds` → `Restarted.bounds` echo、(6) 1 poll attempt 上限: `max_attempts_per_poll` cap + 繰越、(7) route 単位 watch: deferred、(8) **`SupervisedId`（u64 opaque key）採用 — spec 推奨の `PluginHandle` を design が覆した**（restart 毎にキーが変わるため handle は不安定 — design 決定 2）、(9) mid-`DestroyInstance` 通知消失: `lifecycle_state` 走査 fallback 却下（YAGNI — deferred 表）。
- **`--hang-after-msgs` は未使用（計画内 deviation）** — design L598 が lingering 注入に `--crash-after-msgs`/`--hang-after-msgs` の両選択肢を記したが、T3 は `--crash-after-msgs=6`（unwind `Deactivate` frame 着信時に子死亡 → `Err(Ipc)` → minted `Failed` 収束 → retry `Destroy` 合法）を採用。crash は応答停止より厳しい経路（子死 → `Failed` 遷移 → `Destroy` が `Failed` から合法）を確率的ゆらぎなく通るため、hang 変種（コマンド timeout 待ち → 同じ `Err(Ipc)` 収束）は冗長として未使用のまま — flag 自体は T1 前から存在する fixture の一部で、本 Issue の対象外。
- **`--fail-stop-processing` は supervisor unwind で到達不能（documented）** — unwind が `StopProcessing` を発行するには minted mirror `Processing` が必要だが、それは `StartProcessing` `Ok` = attempt 成功（cleanup なし）時のみ成立する。design L598 明記どおり注入不能であり、flag は既存 `--fail-stop-processing` テスト（commit-then-advisory 形状の pin）のみで使用 — supervisor 関連 diff なし。
- **`SupervisionEvent::Orphaned` variant は T2 review で追加（design sketch にも反映済み）** — lingering slot が占有されたまま新 residue が発生する二重失敗角で、`Option<PluginHandle>` 単一 slot を通せない新 handle を silent drop する代わりに `Orphaned{id, handle}` event で caller 簿記へ委ねる確定契約（commit `637abae` P2・design cleanup-corner 行）。**`Orphaned` は公開 API 経由で生起不能** — emit arm（supervisor.rs:838）は `store_lingering` が占有 slot への inline retry を `Err` で返す二重失敗角のみが到達し、minted residue の unwind verb（`Deactivate`/`Destroy`）には L0 fixture で失敗する plugin hook がないため unit も `Err` arm に到達しない（`store_lingering_retries_occupied_slot` supervisor.rs:2256 が inline retry 成功 → `Orphaned` 非 emit の reachable arm を pin — test 内 comment が「`Err` arm は L1 E2E territory」と明記）。E2E では `lingering_*` 系が residue 経路を全網羅し `Orphaned` event は観測されない（variant 不在を前提としない `find_map`/`_ =>` 構造のため event 到達時に検出可能 — 生起しないことが実測で確認済み）。
- **T1–T3 task 文書の `todo` 残留は本 task で一括同期** — T1–T3 commit は task 文書の status 同期を行わなかった（#117 は各 task で `done` 化していた先例だが、本 plan は「status 同期は T4 の範囲」を T4 task doc に明記）。4 task 全て `done` へ同期。
- **gate log ファイルは生成しない** — plan `changes.generated`/`docs/impl/exp/` への追加分が本 task スコープに列挙されていないため（#117 は task スコープに log 保存が明記されていた）。実出力は本 IR Validation 表に記録。
- **既知 flake 未発生** — `l1_proxy_e2e` の負荷依存 flake（`wedged_child_write_converges_and_teardown_is_bounded`・`hang_converges_on_command_timeout`）は本 gate で再発せず（100/100 初回 pass）— 孤立 rerun は不要だった。
- **IR `verifies` は空・`verified-by` は test-spec 群** — #119/#117 IR の convention に準拠。

## Related Experiments

### Adopted

なし — 検証記録 + 帳簿のみの task（実験ログなし・新規 fix なし — 全 gate 初回 pass のため before/after 記録対象も発生しなかった）。

### Rejected

- **design Verification Notes checkbox を #117 先例どおり tick しない案** — 本 task doc が「design Verification Notes の checkbox 証拠化を含む」と明示的に scope 内へ列挙しており（#117 と異なり要求行あり）、証拠化対象として採用した。
- **gate 実測 log を `docs/impl/exp/data/` へ保存する案** — plan/task doc の `changes.generated` が `[]` で log 保存が scope 外のため、実出力は Validation 表への記録に留めた（#117 は task スコープに log 保存が明記されていた差分）。

## Validation

| gate | コマンド | 結果 |
| --- | --- | --- |
| fmt-check | `mise run fmt-check` | exit 0（`cargo fmt --all -- --check` 差分なし） |
| check | `mise run check` | exit 0（`cargo check --workspace --all-targets` dev・default 構成） |
| lint | `mise run lint` | exit 0（`cargo clippy --workspace --all-targets -- -D warnings` clean — `exhaustive_enums`/`unsafe_code` warn ゼロ） |
| test | `mise run test` | exit 0（**60 test binaries・1294 passed・0 failed・0 ignored** — `l1_proxy_e2e` 100・`l1_runner_e2e` 21・`l1_supervisor_e2e` 22・supervisor unit 33 含む。`#[ignore]` E2E は `host` feature 未 compile で default 経路に出ない） |
| test-rt | `mise run test-rt` | exit 0（`cargo test --workspace rt_` — **実一致 137・0 failed**・37 binaries 内で一致。`rt_l1_supervised_process_no_alloc` 追加で #117 計測 132 から +5・無退行） |
| bench-smoke | `mise run bench-smoke` | exit 0（`cargo bench -p ruviform-math -p ruviform-rt -- --test` 計測なし 1 回） |
| lint-l1-real-plugin | `mise run lint-l1-real-plugin` | exit 0（clippy `-p ruviform-plugin-clap -p ruviform-plugin-vst3 --all-targets --features host -D warnings` — 0 warning） |
| test-l1-real-plugin | `mise run test-l1-real-plugin` | exit 0 — **3 段全 green**: `cargo build --features host`（fixture cdylib + runner bin 生成）→ `--lib --bins`（CLAP lib 63+bin 2・VST3 lib 47+bin 2）→ `--ignored`（CLAP 3 + VST3 3 — `real_adapter_l1_full_series`/`real_adapter_l1_kill_reports_failed_once`/`real_adapter_l1_missing_runner_is_ipc_rejection` 各 ×2・実子プロセス経路） |
| cargo-doc | `env RUSTDOCFLAGS="-D warnings" mise exec -- cargo doc -p ruviform-plugin-host --no-deps` | **初回から exit 0・warning 0**（新規公開 API doc — `Supervisor`/`SupervisedId`/`RestartPolicy`/`RestartRecipe`/`SupervisionEvent`/`SupervisionError`/`RestoreOutcome` + `poll` sole-drain WARNING が全て解決済み） |
| prek | `mise exec -- prek run --all-files` | exit 0（全 hook pass — `check illegal windows names` は no files で Skipped） |
| doc-audit | `node .agents/skills/doc-status/scripts/doc_status.js audit --json` | exit 0・`blocking: 0`・`findings: []`（baseline でも最終編集後でも同値 — 392 documents） |
| focused E2E | `cargo test -p ruviform-plugin-host --test l1_supervisor_e2e` | exit 0（**22/22 pass** — 実子プロセス全系） |
| focused E2E | `cargo test -p ruviform-plugin-host --test l1_proxy_e2e` | exit 0（**100/100 pass・無 diff green** — R-V5。既知 flake 未発生） |
| focused unit | `cargo test -p ruviform-plugin-host --lib supervisor` | exit 0（**33/33 pass** — supervisor unit 全列） |
| literal-sweep | `git diff fc02103...HEAD`・grep 系 | `composite.rs`/`l1/`/`host.rs`/`wire.rs` 0 行 diff・`supervisor.rs` が `pub(crate)`/`crate::l1` 非参照・`l1_supervisor_e2e.rs` が `#[path]` 非使用・core/export への supervisor 語彙 0 hit |

対象 test-spec: `docs/test-specs/issue-116-supervisor-core-tests.md`・`issue-116-supervisor-e2e-tests.md`（`proposed → approved` — 実測証跡あり）・`plugin-l1-proxy-contract-tests.md`（既存 `approved` — `l1_proxy_e2e` 無退行の `verified-by` edge）。

## Risks

- **opt-in lane は実行されなければ腐る** — `#[ignore]` + `required-features="host"` 分離の性質上、`l1-real-verify` の手動 dispatch と `mise run test-l1-real-plugin` 明記で緩和（#114/#117 Risks と同記述）。本 task で実測済みのため現時点では lane は健在。
- **既知負荷依存 flake の継続観測** — `l1_proxy_e2e` の `wedged_child_write_converges_and_teardown_is_bounded`・`hang_converges_on_command_timeout`（500ms spawn budget 競合・pre-existing）は本 gate 未発生だが残存リスク（再発時は孤立 rerun で切り分け）。
- **caller の sole-drain 契約違反は型で防げない** — `poll` が bound host の唯一の drain 点であり、caller が `poll` 外で `try_recv_notification` を drain すると supervised `InstanceFailed` が silent unrecoverable loss になる（entry が `Live` のまま滞留）。`poll` doc の WARNING・design 決定 8・Risks 表で pin 済みだが契約は social — `lifecycle_state` 走査 fallback は design deferred（YAGNI）。
- **Bundled 共有子上の blast radius** — design Risks 表の確定トレードどおり、A の crash-loop は共有子の反復死亡で同居 B/C の `Failed` fan-out を引き、各 entry の `failures` を削り得る。隔離強度が必要な route は `Dedicated` を選ぶ運用緩和（isolation は登録時指定）。
- **`Orphaned` corner は accepted として観測可能化のみ** — 二重 unwind 失敗で lingering slot が占有済みの場合、新 residue は追跡対象外となり `Orphaned{id, handle}` event で caller 簿記へ委ねる（instance は殺さず後発死は `Unsupervised` で届く）。公開 API 経由では生起が極めて困難だが、event として契約化済み。
- **`recipe.plugin_id` misdeclare → cross-plugin resurrection** — `PluginHost` に handle→plugin_id 逆引き面がないため `watch` の `Scan` 検証は「id 登録済み」の証明のみ（design Risks 表・`watch` doc で caller 責務として明記）。
- **`rt_` name filter は substring match** — `sho**rt_**input` 系の偶然一致を含む件数 caveat（#126/#130/#117 Risks と同一 — 無退行判定は before/after 同一条件の件数比較で成立: 132 → 137 の +5 は supervisor `rt_` 行）。

## Follow-ups

- Issue #118 — video/NONRT データ面（本 supervisor の `prepare_endpoint` 再配線が関係する将来の plugin 種別）。
- Issue #120 — 共有リーパー/io 多重化（poll 型 supervisor は競合なし・design で確認済み）。
- design Deferred 表: route 単位 watch・`rebind` API・`lifecycle_state` 走査 fallback・per-entry `RestartPolicy` 上書き・`BundleFailed` variant・pull 型 checkpoint。
- **caller 直接 `RestoreState`/`SaveState` reply の oversized blob（pre-existing core gap）** — `set_checkpoint` は bound 拒否で防御済みだが、supervisor 非経由の `RestoreState`・plugin 側 `SaveState` reply（`reader_main` が同じ codec bound で converge）には同種の穴が残る — design Deferred 表で follow-up 候補として記録済み（`ruviform-plugin-core` wire/`ruviform-plugin-host` 層の別 Issue 級）。
- **GitHub merge・issue close は lead の authoritative `gh` のみ** — 本 task では merge を実施しない（証跡収集と doc 同期に留まる）。

## Sources and Relations

- Issue [#116](https://github.com/scarecrow173/ruviform/issues/116): L1 自動再起動・supervisor ポリシー — `ruviform-plugin-host` opt-in util 層（`Supervisor`/`SupervisedId`/`RestartPolicy`/`RestartRecipe`/`SupervisionEvent`/`SupervisionError`/`RestoreOutcome`・watch/poll state machine・bounded retry + backoff + `stable_window` reset・checkpoint bound・lingering/orphan・host binding）の全実装と検証。core `PluginHost`/`composite.rs`/`l1/`/wire は byte-level 無 diff（AC4「自動再起動なし」維持）。
- Spec [issue-116-supervisor.md](../specs/issue-116-supervisor.md)（`implemented` — AC1–AC10 + R-V1–R-V6）・Design [issue-116-supervisor.md](../designs/issue-116-supervisor.md)（approved — API sketch・遷移表・Verification Notes L588-603 が証拠 oracle）・ADR [issue-116-supervisor-placement.md](../adr/issue-116-supervisor-placement.md)（accepted — util 層配置・core 無変更）・Plan [implement-issue-116-supervisor.md](../plans/implement-issue-116-supervisor.md)（`completed`）・Tasks [l1-test-worker-fault-flags.md](../tasks/l1-test-worker-fault-flags.md)・[l1-supervisor-core.md](../tasks/l1-supervisor-core.md)・[l1-supervisor-e2e.md](../tasks/l1-supervisor-e2e.md)・[l1-supervisor-final-verification.md](../tasks/l1-supervisor-final-verification.md)（全 `done`）・Discovery [issue-116-supervisor.md](../discovery/issue-116-supervisor.md)（`active`）。
- 検証 oracle: [issue-116-supervisor-core-tests](../test-specs/issue-116-supervisor-core-tests.md)・[issue-116-supervisor-e2e-tests](../test-specs/issue-116-supervisor-e2e-tests.md)・[plugin-l1-proxy-contract-tests](../test-specs/plugin-l1-proxy-contract-tests.md)。
- 先行記録: [issue-117-bundle-child-implementation](issue-117-bundle-child-implementation.md)（`Failed` 粒度・fan-out・pool の前提実装 + 同型 closeout 先例）・[issue-114-real-adapter-final-verification-closeout-task-5-implementation](issue-114-real-adapter-final-verification-closeout-task-5-implementation.md)（opt-in lane 実測 closeout 先例）。
- Refs: Issue [#31](https://github.com/scarecrow173/ruviform/issues/31)・[#76](https://github.com/scarecrow173/ruviform/issues/76)・[#113](https://github.com/scarecrow173/ruviform/issues/113)・[#114](https://github.com/scarecrow173/ruviform/issues/114)・[#117](https://github.com/scarecrow173/ruviform/issues/117)・[#119](https://github.com/scarecrow173/ruviform/issues/119)・[#120](https://github.com/scarecrow173/ruviform/issues/120)。

## Review Handoff

- AC 写像表の test 名・file:line が HEAD と一致するか・literal sweep（`composite.rs`/`l1/`/`host.rs`/`wire.rs` 無 diff・`supervisor.rs` の `use` 制限・`l1_supervisor_e2e.rs` `#[path]` 非使用）が merge-base `fc02103` 対比で再現するか・`Orphaned`/`--hang-after-msgs`/`--fail-stop-processing` の deviation 記述が design の確定記述と整合するか・doc audit の最終 `findings: []` が編集後再実施であるかを確認されたい。**GitHub issue close・merge 判定は本 IR の scope 外 — push/PR は行っていない。**
