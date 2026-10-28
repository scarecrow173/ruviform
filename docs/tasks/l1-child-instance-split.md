---
id: "TASK-034NlQlLZkVZRtEVfF8Sdp"
type: "task"
status: "todo"
title: "L1 子構造再編（Shared/Inner→L1Child+InstShared split・demux・converge split・Destroying gate）"
created: "2026-10-28"
updated: "2026-10-28"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/117"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/mod.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/shm.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/l1/data.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/composite.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-117-bundle-child-process.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-117-bundle-child-process.md"
    - "docs/tasks/l1-wire-v2-codec-cutover.md"
  blocks:
    - "docs/tasks/l1-route-pool-and-isolation-surface.md"
    - "docs/tasks/l1-bundle-e2e-and-granularity-verification.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/l1-proxy-control-plane.md"
    - "docs/tasks/l1-shm-data-plane-and-rt-legs.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-117-bundle-child-process.md"
    - "docs/designs/issue-117-bundle-child-process.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-117-bundle-failed-granularity-tests.md"
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/113"
  defers: []
  deferred-by: []
---

# L1 子構造再編（Shared/Inner→L1Child+InstShared split・demux・converge split・Destroying gate）

## Work

wire v2（T1 で確立）上に host proxy の子共有構造を実装する。design L199-250（責務写像・converge split・Destroying doorbell — doorbell window は L195）・L304-320（demux 判定表）を正本とし、`Shared`/`Inner` → child 共有 + `Inner.instances`→`InstShared` の split・`L1Child`・driver/reader demux・converge split・`Destroying` gate・install/uninstall・last-ref teardown を行う。外側からは依然 dedicated 相当（1 子 1 instance）のまま — pool/isolation 面は T3。

1. `Shared`/`Inner` → `L1Child` split（`l1/mod.rs`）— `L1Child{child, driver_join, reader_join, driver_tx, spawn 簿, live set}` を新設。`Shared{inner: Mutex<Inner>, child: OnceLock<L1Child への backref>, next_cmd_seq}` を child-wide 共有とし、`Inner{waiters, instances: HashMap<u32, InstShared>, block_in_flight: HashSet<(u32,u64)>, terminal, shutdown_initiated, seen_eof}` へ再編。`InstShared{rt_state, mirror, notify_tx, notify, staging, shm: SegmentInfo, result_tx}` を per-instance に分離。`L1Instance` は `Arc<Shared>` + `Arc<L1Child>` + `instance: u32` + `RtLeg` + `Segment` + `notify_rx` を保持。`Waiter{instance, tx}`・`block_in_flight` を `(u32,u64)` キーへ。lowest-free wire handle mint（`Inner.instances` keyset から最小空きを採番）を `Inner` 排他内で実装
2. driver multi-drain — `submits: HashMap<u32, Consumer<Submit>>`・`views: HashMap<u32, shm::View>` を driver 側に保持。`DriverCmd::AddInstance{instance, submit_producer, shm_key}`/`RemoveInstance{instance}` で install/uninstall。各 poll で live instance 全員の submit を drain し `BlockIn{instance, seq}` 化。instance ごとの staging を `InstShared` 経由で受け取る
3. reader demux + 判定表 — `views: HashMap<u32, shm::View>` を lazy attach（初回 `BlockDone{i}` で `InstShared.shm.key` から attach・失敗 → 子 scope converge）。Reply/BlockDone/`Command`（host 発）を `(instance, seq)` で解決する demux 関数を 1 箇所に集約し、design L304-320 判定表を逐語実装: 子 scope 終端（death/kill/EOF/write-fail/decode 不能/unknown kind/2nd Hello/帰属不能違反/timeout）→ `converge_child(exit)`、帰属可能 instance-scope（live i に対する未知 block seq の `BlockDone`・flagged `Reply`）→ `converge_instance(i)`、inert（registered-but-not-live・`i ∉ instances`）→ 静黙 drop、post-`Shutdown` frame → non-event、per-command `Err(HostError)` → defined rejection（`Failed` にしない）。instance=`NONE` の Reply/BlockDone・`i ≥ 64` の着信も子 scope 違反。`live` 判定 = `InstShared` が存在し `Destroying` でない・`terminal` でない
4. converge split — `converge_child(exit)`: `Inner.terminal = true` + 全 live `InstShared` を `Failed` mirror + 全 waiter `Err(Ipc)` + in-flight doorbell `mark_failed` + per-instance `notify_tx.send(InstanceFailed{handle, exit})`（同一 `exit` fan-out）+ thread join + reap。`converge_instance(i)`: `InstShared` のみを対象に `Failed` mirror + waiter/doorbell/result 解放 + `notify_tx.send(InstanceFailed{handle, exit:{None,None}})`（子は生存・兄弟は継続）— `Inner.terminal` 等の子 state を変更しないことを invariant とする
5. `Destroying` gate + doorbell window — `Destroy`/`DestroyInstance` 送信 serialize → `InstShared` を `Destroying` 状態に遷移 → 新 submit は `mark_failed` → in-flight doorbell は完了を待つ（drain された `BlockDone{i,seq}` は normal 処理）→ reply 受領で entry 撤去・残 seq は `mark_failed` 解放。`Shutdown` は live=0 判定時のみ送信（T3 の shutdown-on-empty と接続）
6. install/uninstall + 死亡時意味論 — `Inner.instances` に `CreateInstance` 送信前に `InstShared` を仮登録（registered-but-not-live）。mid-`CreateInstance` 死亡 → ローカル unwind（entry 撤去・`Err(Ipc)`）で通知なし（handle 未 mint）。mid-`DestroyInstance` 死亡 → entry を `Failed` に残し per-instance `InstanceFailed{handle,{None,None}}` を通知 + `Destroy` は `Ok`。fresh-spawn 失敗 → 子ごと teardown（T3 pool remove と接続）・shared-child `CreateInstance` 失敗 → instance entry rollback のみ（兄弟は生存）
7. last-ref teardown — `L1Instance::drop` / route teardown で最後の参照が外れた子を bounded teardown（`Shutdown`+grace または kill→reap→join）。`L1Instance` は `Arc<Shared>`/`Arc<L1Child>` を持つため drop 時に live set を検査して最終 referrer が子を回収する
8. dedicated pub(crate) seam — `L1Child::spawn` + `CreateInstance` の dedicated spawn helper を `pub(crate)` で残し、`#[path]` harness（`l1_proxy_e2e`）が dedicated 子を 1 発で立てられる形にする。`L1Instance::spawn(spec)` は本 helper 経由
9. `l1/shm.rs` doc — `--ruviform-shm-key` 記述（`l1/shm.rs:8`・`:181`）を `CreateInstance` payload 経路へ更新（code 変更なし）
10. demux 判定表の unit test — `l1/mod.rs` 内 `#[cfg(test)]` で、合成 `Message` frame + 構築済み `Inner` を demux 関数へ入力し、判定表全行（子 scope/instance scope/inert/non-event/defined rejection）を pin。`views` lazy attach の失敗 → 子 scope 行も含める
11. `test_fixture` 移行 — `L1Instance::test_fixture`（`l1/mod.rs:1860-1899`）が singleton `Inner`/`Shared` を literal 構築している — `InstShared`/`L1Child` split 後の新構造へ必ず追随させる（`composite.rs:1209`/`1347` の `l1_instance()` helper と `#[path]` harness が利用 — T5 grep では拾えない構造依存のため本 task で明示）
12. l1 内 doc の構造追随 — `l1/mod.rs:28` モジュール doc「## Thread model (per instance = 1 child + 2 threads)」を per-child（1 子 N instance + driver/reader 2 スレッド維持）へ、`l1/data.rs:926`「a clone of `Shared::rt_state`」を `InstShared::rt_state`（`rt_state` は InstShared へ移る）へ更新。T5 の grep パターン（`per-instance spawn`・`1 子`・`singleton`）では拾えない言及のため T2 で処理（code 変更なし）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-117-bundle-child-process.md](../plans/implement-issue-117-bundle-child-process.md)（T2 — host 構造再編）
- [../designs/issue-117-bundle-child-process.md](../designs/issue-117-bundle-child-process.md) L199-250（Shared/Inner/InstShared/L1Child 責務写像・converge split）・L195（Destroying doorbell window）・L304-320（demux 判定表）
- [../adr/issue-117-bundle-child-failed-granularity.md](../adr/issue-117-bundle-child-failed-granularity.md)（Failed 粒度契約 — `converge_child`/`converge_instance` の意味は本 ADR が正本）
- [../specs/issue-117-bundle-child-process.md](../specs/issue-117-bundle-child-process.md) AC2-AC5/AC9/AC11/AC12 の機構側
- [issue-117-bundle-failed-granularity-tests.md](../test-specs/issue-117-bundle-failed-granularity-tests.md)（判定表 oracle — T4 で E2E 検証と接続）

## Dependencies

- 先行タスク: [l1-wire-v2-codec-cutover.md](l1-wire-v2-codec-cutover.md) — `instance` envelope・`CreateInstance`/`DestroyInstance`・`Inner` v2 形が前提
- 後続タスク: [l1-route-pool-and-isolation-surface.md](l1-route-pool-and-isolation-surface.md)（T3 が `L1Child`/live set を pool 化）

## Affected Files

- `crates/ruviform-plugin-host/src/l1/mod.rs` — `Shared`/`Inner`/`L1Child`/`InstShared` split・demux・converge split・`Destroying` gate・install/uninstall・teardown・`test_fixture` 移行・モジュール doc
- `crates/ruviform-plugin-host/src/l1/shm.rs` — `--ruviform-shm-key` doc 参照のみ
- `crates/ruviform-plugin-host/src/l1/data.rs` — `Shared::rt_state` doc 参照のみ（`InstShared::rt_state` へ）
- `crates/ruviform-plugin-host/src/composite.rs` — `L1Instance::spawn` call-site を `pub(crate)` helper 経由へ適合（pool は T3）

## Done When

- [ ] `Inner.instances: HashMap<u32, InstShared>` が存在し、`InstShared` が per-instance の `rt_state`/`mirror`/`notify`/`staging`/`shm`/`result` を保持する
- [ ] `block_in_flight: HashSet<(u32,u64)>`・`Waiter{instance,tx}`・lowest-free mint が `Inner` 排他内で動作する
- [ ] driver が `submits`/`views` の `HashMap` を持ち `AddInstance`/`RemoveInstance` で install/uninstall される
- [ ] demux が design L304-320 判定表を単一関数で実装し、inert/non-event/defined rejection/attributable/child-scope を区別する
- [ ] `converge_instance(i)` が `Inner.terminal` 等の子 state を変更しない（invariant を test pin）
- [ ] `Destroying` gate が doorbell window（新 submit `mark_failed` + in-flight 完了待ち + reply 撤去）を実装する
- [ ] mid-`CreateInstance` 死亡 → 通知なし `Err(Ipc)`・mid-`DestroyInstance` 死亡 → 通知 + `Ok`・last-ref teardown が動作する
- [ ] `pub(crate)` dedicated spawn helper が `#[path]` harness から呼べる
- [ ] `test_fixture` が split 後の `L1Child`+`InstShared` 構造を構築し、`composite.rs` の `l1_instance()` helper（`:1209`/`:1347`）が green に留まる
- [ ] `l1_proxy_e2e` が dedicated 経路で green に留まる

## Verification

- [ ] `cargo test -p ruviform-plugin-host --lib` — demux 判定表 unit test 全行 pass
- [ ] `cargo test -p ruviform-plugin-host --test l1_proxy_e2e` — dedicated 経路非回帰
- [ ] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` が全 pass
- [ ] demux 判定表の各行（live/registered-but-not-live/post-removal/child-scope terminal/instance-scope attributable/non-event/defined rejection）が 1 つの関数に集約されている
