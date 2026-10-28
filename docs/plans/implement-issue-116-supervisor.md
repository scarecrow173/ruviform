---
id: "PLAN-034NOXmaWmUZjtJxQeNUhL"
type: "plan"
status: "approved"
title: "Issue #116 実施計画 — L1 supervisor（自動再起動 opt-in util 層）"
created: "2026-10-28"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/116"
    - "https://github.com/scarecrow173/ruviform/issues/31"
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/117"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/supervisor.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_supervisor_e2e.rs"
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
        path: "docs/impl/ir/issue-116-supervisor-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/bin/l1-test-worker.rs"
      - type: "file"
        path: "docs/specs/issue-116-supervisor.md"
      - type: "file"
        path: "docs/designs/issue-116-supervisor.md"
      - type: "file"
        path: "docs/adr/issue-116-supervisor-placement.md"
      - type: "file"
        path: "docs/discovery/issue-116-supervisor.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-116-supervisor.md"
  implemented-by:
    - "docs/tasks/l1-test-worker-fault-flags.md"
    - "docs/tasks/l1-supervisor-core.md"
    - "docs/tasks/l1-supervisor-e2e.md"
    - "docs/tasks/l1-supervisor-final-verification.md"
  depends-on:
    - "docs/designs/issue-117-bundle-child-process.md"
    - "docs/designs/plugin-l1-out-of-process.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/issue-116-supervisor-placement.md"
    - "docs/plans/implement-issue-117-bundle-child-process.md"
    - "docs/designs/issue-117-bundle-child-process.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
    - "docs/designs/plugin-isolation.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/designs/engine-plugin-host-integration.md"
    - "docs/discovery/issue-116-supervisor.md"
    - "AGENTS.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-116-supervisor.md"
    - "docs/designs/issue-116-supervisor.md"
    - "docs/adr/issue-116-supervisor-placement.md"
    - "docs/discovery/issue-116-supervisor.md"
  derived-by:
    - "docs/tasks/l1-test-worker-fault-flags.md"
    - "docs/tasks/l1-supervisor-core.md"
    - "docs/tasks/l1-supervisor-e2e.md"
    - "docs/tasks/l1-supervisor-final-verification.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-116-supervisor-core-tests.md"
    - "docs/test-specs/issue-116-supervisor-e2e-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #116 実施計画 — L1 supervisor（自動再起動 opt-in util 層） Implementation Plan

> status: approved — Refs #116。approved design（`docs/designs/issue-116-supervisor.md`）+ accepted ADR（`docs/adr/issue-116-supervisor-placement.md`）を正本として、`PluginHost` 公開 API のみで動く opt-in supervisor util（`src/supervisor.rs`）を 4-task DAG で実装する。T1 は `l1-test-worker` の失敗分類 fixture flag、T2 は supervisor 本体（watch/poll state machine・attempt replay・backoff・checkpoint bound・host binding）、T3 は実子プロセス E2E（別 test target — `pub(crate)` 非到達が「公開 API のみ使用」の compile-level 証明）、T4 は Full Gate + AC/R-V 証拠 + IR + status 同期。spec は `draft` のまま — status 同期は T4 の範囲。core `PluginHost`（AC4「自動再起動なし」）・`composite.rs`/`l1/`/wire/adapter/runner は無 diff が確定契約。

## Goal

`ruviform-plugin-host` に `supervisor` モジュールを新設し、`HostNotification::InstanceFailed` → drain 時 `Destroy` → 同 route `Instantiate{plugin_id}` → lifecycle replay（`Initialize` → `RestoreState`(checkpoint 時) → `Activate{config}` → `StartProcessing`）→ `SupervisionEvent::Restarted{id,old,new,bounds,restored}` の機械的復旧を、`RestartPolicy`（bounded retry + exponential backoff + `stable_window` episode reset + `max_attempts_per_poll`）と `GaveUp`/`Unsupervised`/`WrongHost` event 報告のもと実装する。`PluginHost` core の型・语义・通知契約は一切変更しない（AC4 維持 — supervisor は caller が `watch` した entry のみを `poll(&mut host, now)` で駆動する opt-in util）。

## Non-Goals

- core `PluginHost`/`composite.rs`/`l1/`/`host.rs`/wire/`HostNotification`/`HostError`/`ChildExit` の変更（AC4 保護境界 — AC1 で無 diff を検証）
- `prepare_endpoint` の supervisor 実行（caller 責務 — `Restarted.bounds` echo のみ。`PluginEndpoint` は RT owner への move が前提の構造的理由）
- pull 型 checkpoint（`poll` での `SaveState` 駆動）・route 単位 watch・`rebind` API・`lifecycle_state` 走査 fallback・`GaveUp` 後自動 unwatch・`BundleFailed` variant・per-entry `RestartPolicy` 上書き（全て design Deferred 表）
- caller 直接 `HostCommand::RestoreState` / `SaveState` reply の oversized blob bound gap（pre-existing core gap — design Deferred 表の follow-up 候補・本 plan の `set_checkpoint` bound 防御とは別物）
- GUI（`OpenGui` — L1 `Err(Unsupported)` 不変）・RT 処理経路（復旧中も pass-through/無音）・core 側の自動再起動
- 新規依存（`std::time::{Instant,Duration}`・`BTreeMap`・既存公開型のみ。dev-dep `static_assertions` は既存のものを使用）

## Scope and Split Boundary

単一 plan・4 task。分割軸は「コンパイル可能で独立に検証できる最小整合単位」:

- **T1 は fixture のみ（`src/bin/l1-test-worker.rs` 1 file）** — design Verification Notes が明記する fixture gap（`RestoreState` が常に `Ok` で `Err(Unsupported)`/`Err(PluginFailed)`/lifecycle hook 失敗を生起不能）を解消する 4 flag 追加。supervisor API に非依存のため T2 と完全並行可能だが、T3 の restore 分類・lifecycle 失敗注入の前提
- **T2 は supervisor 本体（`src/supervisor.rs` 新設 + `lib.rs` re-export）** — design の公開 API sketch・遷移表・poll アルゴリズムを正本とする単一モジュール + module 内 unit test。同 crate 内のため `pub(crate)` 面はコンパイラが遮らない — 「公開 API のみ使用」は設計契約 + review 観点 + T3 の別 target で担保
- **T3 は E2E（`tests/l1_supervisor_e2e.rs` 新設）** — `l1_proxy_e2e.rs` の `#[path]` harness には乗らない。別 test target は `pub(crate)` L1 machinery を resolve 不能にする（compile-level の公開面強制）かつ `l1_proxy_e2e.rs` を無 diff に保つ（R-V5「既存 suite 無退行」の zero-diff 証明）
- **T4 は closeout** — Full Gate・AC1-AC10 + R-V1–R-V6 証拠・IR・status 同期（spec `draft → implemented` 等）。新機能は追加しない

cross-file 依存の形: T1 は bin 内で完結、T2 は `src/` 内で完結、T3 は `tests/` 内で完結 — T1∥T2・（T1+T2）→T3→T4 の直鎖。

## Files / Ownership

| task | owner crate | primary files | scope |
| --- | --- | --- | --- |
| T1 `l1-test-worker-fault-flags` | plugin-host(bin fixture) | `crates/ruviform-plugin-host/src/bin/l1-test-worker.rs` | `--fail-restore`・`--state-unsupported`・`--fail-initialize`・`--fail-start-processing` flag + argv doc table + dispatch flag 経路整理 |
| T2 `l1-supervisor-core` | plugin-host(src) | `crates/ruviform-plugin-host/src/supervisor.rs`（新規）, `crates/ruviform-plugin-host/src/lib.rs` | 公開 API 面・entry state machine・poll Phase 0-3・attempt/replay/cleanup・backoff・checkpoint bound・host binding・`#[non_exhaustive]`・unit test |
| T3 `l1-supervisor-e2e` | plugin-host(tests) | `crates/ruviform-plugin-host/tests/l1_supervisor_e2e.rs`（新規） | 実子 restart・restore 分類・backoff/episode・bundle burst cap・attempt 失敗全段・lingering・adopt・GaveUp・opt-in 無退行・`Send` witness・`prepare_endpoint` |
| T4 `l1-supervisor-final-verification` | 全列 | `docs/impl/ir/issue-116-supervisor-implementation.md` + index 群 | Full Gate・AC/R-V 証拠・IR・status 同期 |

## Ordering Rationale

1. **T1 が T3 の oracle 前提** — design Verification Notes の fixture gap 行（`RestoreState` 常時 `Ok`）により、`Err(Unsupported)` skip・`Err(PluginFailed)` budget burn・`Initialize`/`StartProcessing` hook 失敗の E2E は flag なしに書けない。T2 の unit test 経路は flag を使わないため T1/T2 は独立
2. **T2 が API surface** — T3 のテストは `Supervisor` 公開面が確定してからでないと書けない（`watch`/`poll`/event 語彙は design sketch 確定形を T2 が着地させる）
3. **T3 は T1+T2 の両方に依存** — fault flag 注入と supervisor API の交点
4. **T4 が収束点** — 全 task 完了後の Full Gate + closeout

並行 group: `{T1 ∥ T2}` → `{T3}` → `{T4}`。

## Task DAG

```
T1 l1-test-worker-fault-flags          (fixture — T3 の注入前提・T2 とは独立)
  └─ T3 l1-supervisor-e2e
T2 l1-supervisor-core                  (supervisor 本体 — AC1/AC5/AC8/AC9 機構面)
  └─ T3 l1-supervisor-e2e              (AC2/AC3/AC4/AC6/AC7 + R-V1–V6)
       └─ T4 l1-supervisor-final-verification  (AC10 + 全 AC/R-V 収束)
```

| task | one-line scope |
| --- | --- |
| T1 `l1-test-worker-fault-flags` | `--fail-restore`（`RestoreState` → `Err(PluginFailed)`・不 commit）・`--state-unsupported`（`RestoreState` → `Err(Unsupported)`）・`--fail-initialize`（`Initialize` → `Err(PluginFailed)`・`Created` 残留）・`--fail-start-processing`（`StartProcessing` → `Err(PluginFailed)`・`Activated` 残留）を追加する |
| T2 `l1-supervisor-core` | `src/supervisor.rs` に `Supervisor`/`SupervisedId`/`RestartPolicy`/`RestartRecipe`/`SupervisionEvent`/`SupervisionError`/`RestoreOutcome`（design 確定形・`#[non_exhaustive]`）と watch/poll state machine・attempt replay/cleanup・backoff・checkpoint bound・`usize` host binding を実装し `lib.rs` から再公開する |
| T3 `l1-supervisor-e2e` | `tests/l1_supervisor_e2e.rs`（`#[path]` harness 不使用の別 target）で実子 `kill_child` → `Restarted` 全系・restore 分類・bundle burst・失敗段階注入・adopt・lingering・`prepare_endpoint`・`Send` witness・opt-in 無退行を pin する |
| T4 `l1-supervisor-final-verification` | Full Gate・AC1-AC10+R-V1–R-V6 証拠・literal sweep・IR・status 同期で closeout する |

## Test Oracles

| test-spec | role |
| --- | --- |
| `issue-116-supervisor-core-tests.md`（新規） | API/state machine oracle — policy 検証・`watch` 検証（live/stale/未登録/dup/`Failed` adopt）・`unwatch`/`set_recipe`/`current_handle`・host binding・`delay` 数学・episode reset・checkpoint bound・drain 契約・`Unsupervised`・`Send`/`Sync` witness・`#[non_exhaustive]` |
| `issue-116-supervisor-e2e-tests.md`（新規） | 実子 E2E oracle — Dedicated/Bundled `Restarted`・restore 分類（`Applied`/`Skipped`/`NotRequested`/`PluginFailed` burn）・bundle burst cap 繰越・mid-create/mid-replay 死・lingering・adopt・crash-loop `GaveUp`・opt-in 無退行・`prepare_endpoint` |

## Verification Matrix（spec AC1–AC10 + R-V1–R-V6 → task/test-spec）

| AC / R-V | task | 検証 |
| --- | --- | --- |
| AC1 opt-in util・core 無変更 | T2/T3/T4 | review（`supervisor.rs` が `pub(crate)`/`l1::`/`composite` 内部型に触れない）+ T3 別 target が compile-level で公開面のみ・`composite.rs`/`l1/`/`host.rs`/`wire.rs` 無 diff（T4 literal sweep） |
| AC2 同 route 限定再 spawn | T3 | Dedicated/Bundled `kill_child` → `Restarted{old,new}`・新 mint generation bump・旧 `Err(InvalidHandle)`・`Instantiate{plugin_id}` が登録 route を継承（e2e test-spec） |
| AC3 bundle fan-out（+ R-V1） | T3 | Bundled N supervised + 1 unwatched `kill_child` → 同一 `ChildExit` N 通知全 drain → per-entry `Pending` → cap 繰越で全員 `Restarted`（N 子強制せず）・unwatched は `Unsupervised`（e2e test-spec） |
| AC4 checkpoint/restore（+ R-V3） | T1/T3 | `set_checkpoint` blob が `RestoreState{bytes}` で適用 `restored=Applied`・`--state-unsupported` → `Skipped`（budget 不消費）・未登録 `NotRequested`・`Err(CheckpointTooLarge)`（core + e2e test-spec） |
| AC5 bounded retry（+ R-V4） | T2/T3 | `Instant` 注入 backoff deadline・`max_attempts` → `GaveUp`・`stable_window` drain 時 pre-check reset 回帰 pin・`max_attempts=0`・`InertPolicy`（core + e2e test-spec） |
| AC6 give up 语义 | T3 | mint 済み unwind+`Destroy` 済み・entry は `unwatch` まで保持・`GaveUp` は `HostError::Failed` に畳まない別 variant・lingering 後発死の再 emit（`attempts`/`last_error` 不変・`(id,attempts)` dedupe）（e2e test-spec） |
| AC7 attempt 中の失敗（+ R-V2） | T1/T3 | mid-`CreateInstance` 死 `Err(Ipc)`（handle 未 mint・cleanup 不要）・`Err(InstanceLimit)` は公開面到達不能（`pool_eligible` が満杯の子を除外 — `composite.rs` L1092-1095）のため `Err(Ipc)` と同じ `Instantiate` `Err` → attempt 失敗 arm で review-covered・`--fail-initialize`/`Err(Prepare)`/`--fail-start-processing`/mid-replay 死の cleanup unwind・`--fail-start-processing`+`--crash-after-msgs=6`（Dedicated — StartProcessing=5 不 commit `Err` → unwind `Deactivate`=6 着信死 → `Err(Ipc)`・minted `Failed` 収束）で lingering 露出 + 次 poll の lingering retry `Destroy`（e2e test-spec） |
| AC8 driver 契約 | T2/T3 | `poll(&mut host, now)`・内部 thread/timer/async なし・`bound_host: usize`・`assert_impl_all!(Supervisor: Send, Sync)`・`PluginHost` `!Send` 維持・host 束縛 `WrongHost` 拒否（core + e2e test-spec） |
| AC9 event 報告 | T2/T3 | `Restarted{id,old,new,bounds,restored}` で `prepare_endpoint`/`latency_frames`/`lifecycle_state` が呼べる・`#[non_exhaustive]`（enum + struct variant）・`Unsupervised` pass-through・`Failed` adopt・`unwatch` の `Option<PluginHandle>`（core + e2e test-spec） |
| R-V5 AC4 無退行 | T3/T4 | supervisor 未使用 `l1_proxy_e2e` suite が無 diff green・素 host drain では `Restarted` 相当が出ない |
| R-V6 endpoint 再配線 | T3 | `Restarted.new`（`config: Some` で `Activated` 到達 recipe）で `prepare_endpoint(bounds)` が動く |
| AC10 CI/監査 | T4 | `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` + `bench-smoke` + `cargo doc` + opt-in lane + `doc_status.js audit` blocking 0 |

## Risks / Readiness

| risk | 対処 |
| --- | --- |
| `pub(crate)` L1 machinery への手出し（同 crate 内では型システムが遮らない — design 明示の設計契約領域） | `supervisor.rs` は `use` を公開 API 型に限定する規則を task doc で pin・T3 を `#[path]` harness 不使用の別 test target にして `pub(crate)` 非到達を compile-level で強制・T4 literal sweep で `l1::`/`composite` 内部型参照なしを確認 |
| `l1-test-worker` flag 増加で dispatch signature が太る（bool 位置引数 5 連の誤配線リスク） | T1 で `&Flags`（または verb 用小 struct）渡しへ整理 — bin 内部の整合 change（argv/外部契約不変） |
| `Instant` 注入を忘れて実時刻依存の backoff test を書く → wall-clock flake | deadline は全て `poll` 引数 `now` で解決（design 確定）— test は `Instant` を直接進める規則を task/test-spec で pin・reply-wait だけ `NOTIFY_TIMEOUT` 流の bounded wait |
| `stable_window` reset の drain 時 pre-check 抜け（散発 crash が旧 episode `failures` を背負い早期 `GaveUp`） | design 遷移表（drain 行 — give-up 判定前に `healthy_since + stable_window` を評価）を単一の drain 処理へ集約し、R-V4 回帰行を E2E で明示 pin |
| `find_watched` match-set（current/failed/lingering）の取り違え → lingering を `Destroy` で殺す or 通知を誤帰属 | design の match-set 規則（lingering 一致のみ `lingering = None`・`failed` 一致は idempotent `Destroy` で無害）を unit test の match-set 行で pin |
| worker flag の注入形状が実 runner と乖離（commit-then-`Err` を誤用） | 不 commit `Err` を既定形状とし、commit-then-advisory は既存 `--fail-stop-processing` のみ（実 runner の `stop_processing` 形状に合わせた旗）— T1 task doc に明記 |
| `poll` 外 `try_recv_notification` drain（caller 契約違反）が silent unrecoverable loss になる | 型では防げない — `poll` doc の WARNING + sole-drain 契約を core test-spec で pin・T4 で doc 文言の最終確認 |
| 既存 `l1_proxy_e2e` 負荷依存 flake が混入 | supervisor suite は別 target — flake 切り分けがファイル境界で自然（T4 で孤立 rerun 記録） |

## Rollback Strategy

- **commit 単位**: 各 task は独立した buildable 境界（T1=bin fixture・T2=`src/` 2 file・T3=test file・T4=docs）。各 task 末は `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` green 必須 — red は同 task 内 fix、大きな欠落はその task の差し戻し（planning 基点 `5254df2` への `git revert`）
- **supervisor は additive opt-in 面** — `mod supervisor;` + `pub use` の除去で公開面を完全撤去できる（core 無 diff が確定契約のため、rollback は追加分の除去のみで他へ波及しない）
- **fixture flags は additive** — flag 未指定の worker 挙動は現行と同値のため T1 revert は flag 除去のみ
- **planning artifact のみの段階** — 本 plan 時点では doc のみ。doc rollback は新規 doc 削除 + index/関係行の除去で完結

## Implementation Sequence

- [ ] T1 `l1-test-worker-fault-flags`（T2 と並行可）
- [ ] T2 `l1-supervisor-core`（T1 と並行可）
- [ ] T3 `l1-supervisor-e2e`（T1+T2 後）
- [ ] T4 `l1-supervisor-final-verification`（全 task 後）

## Dependencies and Risks

- 前提: `docs/designs/issue-116-supervisor.md`（approved — API sketch・遷移表・poll アルゴリズム・失敗分類表が正本）・`docs/adr/issue-116-supervisor-placement.md`（accepted — util 層配置・core 無変更境界）・`docs/specs/issue-116-supervisor.md`（draft — AC/R-V の oracle。status 同期は T4）・`docs/designs/issue-117-bundle-child-process.md`（implemented — fan-out 粒度・pool の前提）・`PluginHost` `!Send`/`!Sync`（`poll` を host 所有スレッドで呼ぶ駆動形の型制約）
- 検証列は全 task 共通: `mise run fmt-check` → `check` → `lint` → `test` → `test-rt`。T4 で `bench-smoke`・`mise exec -- cargo doc -p ruviform-plugin-host --no-deps`（warning 0 — 新規公開 API doc）・`prek run --all-files`・`doc_status.js audit` を追加。opt-in lane `mise run lint-l1-real-plugin`・`mise run test-l1-real-plugin` は wire/runner/adapter 無 diff のため既定 gate 外だが、host 公開面の additive sanity として T4 で実行する
- `unsafe` 新規導入は不要 — `host as *const PluginHost as usize` は pointer→usize cast で `unsafe` ではない（deref しない・`usize` としてのみ保持）。workspace lint `unsafe_code` に抵触しない
- `#[non_exhaustive]` は `SupervisionEvent`/`SupervisionError`/`RestoreOutcome` の enum レベル + `SupervisionEvent` struct variant レベルの両方に必須（公開 enum 方針 + additive field 拡張余地 — design L325）
- 依存追加なし — `std::time::{Instant,Duration}`・`BTreeMap`・既存公開型（`PluginHost`/`PluginHandle`/`HostCommand`/`HostReply`/`HostError`/`HostNotification`/`EndpointBounds`・`ProcessConfig`・`wire::WIRE_MAX_*`）のみ。`static_assertions` は dev-dep として既存（`Send`/`Sync` witness に使用 — Cargo.toml 変更不要）

## Task Breakdown

- `docs/tasks/l1-test-worker-fault-flags.md` — T1
- `docs/tasks/l1-supervisor-core.md` — T2
- `docs/tasks/l1-supervisor-e2e.md` — T3
- `docs/tasks/l1-supervisor-final-verification.md` — T4

## Verification

- [ ] `mise run fmt-check` が差分なしで終了する
- [ ] `mise run check`・`mise run lint`・`mise run test`・`mise run test-rt` が全て成功する
- [ ] `mise run bench-smoke`・`mise exec -- cargo doc -p ruviform-plugin-host --no-deps` が warning 0 で完了する
- [ ] `mise run lint-l1-real-plugin`・`mise run test-l1-real-plugin` が pass（opt-in lane — host 公開面 additive の sanity。wire/runner/adapter 無 diff のため default gate 外）
- [ ] `node .agents/skills/doc-status/scripts/doc_status.js audit --json` が `blocking:0`・`findings:[]` を返す
- [ ] spec AC1–AC10 + R-V1–R-V6 全行が Verification Matrix の task/test-spec に対応付けられている
- [ ] `git diff` で `crates/ruviform-plugin-host/src/composite.rs`・`crates/ruviform-plugin-host/src/l1/`・`crates/ruviform-plugin-host/src/host.rs`・`crates/ruviform-plugin-core/src/wire.rs` が無 diff（AC1/AC4 の機械的証明）・`supervisor.rs` が `crate::l1`/`pub(crate)` 内部型を参照しない
- [ ] `cargo test -p ruviform-plugin-host --test l1_proxy_e2e` が無 diff green・`cargo test -p ruviform-plugin-host --test l1_supervisor_e2e` が全 pass

## Open Questions / Gaps

- **E2E 配置は別 test target を採用** — `l1_proxy_e2e.rs` 内 `mod supervisor_e2e` ではなく `tests/l1_supervisor_e2e.rs` とする（`#[path]` harness 不使用 → `pub(crate)` 非到達が AC1「公開 API のみ使用」の compile-level 証明 + `l1_proxy_e2e.rs` 無 diff が R-V5 の zero-diff 証明）。worker spawn/helper（`OutOfProcessSpec`・`WORKER_DESCRIPTOR`・`NOTIFY_TIMEOUT`）は最小複製 — design が「`#[path]` harness をそのまま利用可能」と述べるのは可能性の言及であり必須ではない
- **L0 経路での `Failed` adopt pin は optional** — design は adopt 経路で L0 `Shared` slot poisoned `Failed` に言及するが、L0 `Failed` mirror の deterministic な生起は fragile（`Plain` slot panic は bookkeeping を残す）。adopt 検証は L1 実子 `kill_child` → `lifecycle_state → Some(Failed)` 経路で pin し、L0 adopt は unit test で到達可能なら追加（必須行ではない）
- **`l1-test-worker` flag の dispatch signature 形は実装裁量** — `&Flags` 渡しを推奨（bool 位置引数 5 連の誤配線防止）するが、argv/外部契約が不変なら内部表現は強制しない
- **spec は `draft` のまま開始** — OPEN 行は全て design で確定済み。spec の `draft → implemented` 同期は T4 で、同期時に design 確定値と spec OPEN 行の照合（design が覆した spec 推奨 — `SupervisedId` 採用・`Err(Unsupported)` skip 確定 等）を IR に記録する
- **矛盾報告なし** — design/ADR/spec/discovery の現行版で本文書作成時に相互矛盾は確認されなかった（発見時は変更せず報告する契約）
