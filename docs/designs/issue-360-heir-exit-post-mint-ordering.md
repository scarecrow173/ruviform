---
id: "DESIGN-034SmBuqxSVw74rlBganJD"
type: "design"
status: "approved"
title: "Issue #360 設計 — heir 系テストの exit を post-mint へ message 順序で固定"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/360"
    - "docs/specs/issue-360-l1-proxy-e2e-heir-flake.md"
    - "docs/adr/issue-360-mint-child-exit-test-scenario.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/bin/l1-test-worker.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-360-l1-proxy-e2e-heir-flake.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-360-l1-proxy-e2e-heir-flake.md"
    - "docs/adr/issue-360-mint-child-exit-test-scenario.md"
    - "docs/designs/plugin-l1-out-of-process.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-238-l1-proxy-e2e-flake.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-360-l1-proxy-e2e-heir-flake.md"
    - "docs/adr/issue-360-mint-child-exit-test-scenario.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #360 設計 — heir 系テストの exit を post-mint へ message 順序で固定

## Context

`Instantiate` の handshake は `Scan`(msg1) → `CreateInstance`(msg2)。`--leave-heir-after-msgs=2` は msg2（create reply）回答直後に heir spawn + `exit(0)` するため、child の exit が mint window（reply 着陸〜caller `flip_live` commit）の内側に配置される。host 側で `flip_live`（caller thread、`inner` lock 下で `terminal.is_none() && phase==Creating` 時のみ `Live`）と driver の `try_wait`→`converge_child`（`terminal=Failed` + `Creating`→`Failed` flip + waiter `Err(Ipc)`）は無順序に競合し、後者が先なら `SpawnFailed{ExitObserved}`（defined contract）。

## Scope

### In Scope

- `tests/l1_proxy_e2e.rs` の 4 サイトで `--leave-heir-after-msgs` を `3` にし、mint 復帰後に 1 command を送信して leave を発火させる。
- 3rd command の結果 oracle `{Ok(_), Err(HostError::Ipc)}`。
- `l1-test-worker.rs` の `--leave-heir-after-msgs` flag 契約コメントに「N ≦ handshake message count は mint window 内 exit」を明記。
- 上記に伴う誤った前提のテストコメント修正。

### Out of Scope

- `src/l1/` product code（`flip_live`/`converge_child`/`create_rollback_error` — defined contract、変更しない）。
- 意図的 mid-mint death サイト（`--crash-after-msgs=2` — reply 前 exit で `SpawnFailed` 決定的）。
- workflow/CI ファイル。

## Components and Boundaries

- `l1-test-worker`（fixture）: `--leave-heir-after-msgs=N` は「N 個目の parent→child message 回答後・次 loop iteration 先頭で heir spawn + `exit(0)`」。flag 契約は変更せず、値の選び方だけ直す。
- テスト（caller）: `instantiate`/`L1Instance::spawn` 復帰 = `flip_live` 解決済み。その後の最初の送信 message が msg3 であり、host がそれより早く msg3 を送る手段は存在しない — **exit は mint commit に因果的に後続**。
- host（`src/l1`）: 既存 contract に委ねる。3rd command の reply は exit 収束と demux が競合し得るため `{Ok, Err(Ipc)}` を defined outcome とする。

## Data and Control Flow

変更後の直列（全 4 サイト共通）:

1. spawn → `Hello` → `Scan`(msg1) → `CreateInstance`(msg2) → reply `Ok` → `flip_live` → `Instantiate`/`spawn` 復帰（mint committed, `Creating`→`Live`）。
2. テストが 1 command（`Initialize` = msg3）を送信 → child は回答後に heir spawn + `exit(0)`。
3. driver の `try_wait` が exit を観測 → `converge_child(ExitObserved)` → minted entry は `Notify::Sent`（notification に exit status を載せる既存経路）。
4. 既存 assertion 群（`wait_host_failed`/`wait_for_failed` → `wait_host_notification`/`wait_notification` → stderr record flags）に到達。

msg2 reply 送出〜exit の heir spawn 区間に残る demux race（3rd command の `Err(Ipc)`）を、結果 oracle で defined outcome として扱う。`Initialize` を選ぶのは Created→Initialized の自然な次 lifecycle で、`Ok` 時は commit 済み状態を汚さず、`Err(Ipc)` 時も scenario（Failed 収束・notification・record）は不変だから。

## Non-Functional Constraints

- 新規の wall-clock/sleep 同期は導入しない（#238 方針の継承）。
- `Initialize` 追加で各テストの runtime は +数 ms 程度（heir の 15 s 残留は既存どおり detached）。

## Risks and Trade-offs

- 3rd command が defined rejection（`Lifecycle` 等）を返す退化は `Ok|Ipc` oracle が fail させるため取りこぼさない — `Err(Ipc)` 許容は握り潰しではなく、exit が因果的に msg3 後である以上 command 結果が scenario に影響しないことの明示。
- `--leave-heir-after-msgs` の新たな `=2` 使用は flag 契約コメントで防ぐ（仕組みで強制はしない — fixture の汎用性を保つ）。

## Deferred Design Concerns

- exit 系 fixture knob（`leave-heir`/`crash-after`/`stop-reading`/`hang-after`）の handshake 内回数を静的に検査する lint は本件の因果範囲外 — 必要なら別 issue。

## Verification Notes

- [x] `cargo test -p ruviform-plugin-host --test l1_proxy_e2e heir`（4 サイト）が pass。
- [x] `mise run fmt-check` / `lint` / `test` green（Windows 11 + Rust 1.99.0）。
