---
id: "SPEC-034SmylRtM9crIdjdInbvd"
type: "spec"
status: "approved"
title: "Issue #360 l1_proxy_e2e heir flake — mint 窓内 exit を post-mint へ因果順序付け"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/360"
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
    - "docs/designs/plugin-l1-out-of-process.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-238-l1-proxy-e2e-flake.md"
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #360 l1_proxy_e2e heir flake — mint 窓内 exit を post-mint へ因果順序付け

## What Should Be Built

`crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs` で `--leave-heir-after-msgs=2` を使う 4 テストサイトについて、fixture worker の自発 `exit(0)` が **mint commit（`flip_live`）解決後にのみ起こり得る** 構造へ変更する。観測対象の不変条件（`ExitObserved` 収束・heir が保持する pipe と drain deadline・stderr capture）は維持する。

## Why Now

Issue #360: `composite_e2e::heir_stderr_after_drain_stop_is_never_read` が PR #358（observability 追加のみの無関係変更）の初回 `verify (ubuntu-latest)` で `SpawnFailed { cause: ExitObserved, diagnostics_id: 1 }` を返し `unwrap` panic。windows / macos / rt-verify は pass、rerun で green。変更と無関係な PR の `verify` / `ci-gate` を稀に fail させる残留 flake。

## Diagnosis（変更前に診断する）

`l1-test-worker` の `--leave-heir-after-msgs=N` は「N 個目の parent→child message を回答し終えた後、stdio 継承の heir を spawn して `exit(0)`」という意味。`Instantiate` の handshake は `Scan`(msg 1) → `CreateInstance`(msg 2) なので、`=2` は **create reply を書いた直後・mint window の内側** で child を終了させる。

host 側の mint は: reply demux → waiter 完了（reader thread）→ caller thread の `flip_live`（`inner` lock 下で `terminal.is_none() && phase == Creating` を確認し `Live` へ flip）。並行して driver thread は `DRIVER_POLL_INTERVAL`(200 µs) 毎に `try_wait` を poll し、exit を観測すれば `converge_child(ExitObserved)` で `terminal = Failed` + `Creating` → `Failed` flip + waiter `Err(Ipc)` drain を行う。

両者に順序保証はない。`flip_live` が先なら minted notification 付きの正規シナリオ、`converge_child` が先なら `create_rollback_error` → `SpawnFailed{ExitObserved}`（mid-mint death の **defined contract** — `flip_live` の `went_live` ゲート自体がこの折りたたみのために存在する）。通常は reply→flip が heir spawn + exit より速いため mint が勝つが、heir spawn が失敗して即 exit した場合や caller thread が runner 上で遅延した場合に収束が先に来る。ubuntu CI runner でのみ観測されるのは scheduler/contention 依存のため。

症状（`Instantiate` が `SpawnFailed`）と原因（exit が mint window 内に配置されている）は別物であり、破られた不変条件は **テストシナリオ側の「mint 成功後の mid-life 自発 exit」という前提** — reply 送出済みは mint commit を意味しない。

## Users and Value

- CI `verify (ubuntu-latest)` / `ci-gate` lane と全 PR 作者: 無関係変更での false failure・rerun コストの除去。
- 同パターンを今後書くテスト作者: 「reply sent ≠ mint committed」の落とし穴を fixture flag の契約として明示。

## Scope

### In Scope

- `tests/l1_proxy_e2e.rs` の `--leave-heir-after-msgs=2` 4 サイト（`destroy_detaches_reader_when_heir_holds_stdout` / `leave_heir_yields_exit_observed_and_bounded_drain` / `heir_stderr_inside_grace_lands_in_record` / `heir_stderr_after_drain_stop_is_never_read`）の exit 順序付け。
- 上記に伴う当該テストのコメント（誤った前提を記述している箇所）と、必要なら `l1-test-worker.rs` の flag 契約コメント。

### Out of Scope

- `src/l1/` 等 product code: mid-mint death → `SpawnFailed{cause}` は `flip_live`/`create_rollback_error` が実装する defined contract であり、欠陥ではない（ADR 参照）。
- 意図的な mid-mint death を検証する既存サイト（`--crash-after-msgs=2` 等、reply **前** に exit するため `SpawnFailed` が決定的）。
- `.github/workflows/` 等 CI ファイル（別 worker が #364/#366 を担当）。
- sleep・retry 追加、失敗握り潰し、テスト無効化。

## Requirements

1. 4 サイトで child の自発 exit が mint window 内に起こり得ない構造にする。順序は wall-clock/sleep ではなく message の因果順序（host が mint 完了まで送り得ない 3rd message への leave）で担保する。
2. 3rd message の結果 oracle は `{Ok(_), Err(HostError::Ipc)}` とする — reply 送出後の exit 収束が demux に先行し得る残存 race を defined outcome として明示し、defined rejection は引き続き失敗とする。
3. 各テストの検証不変条件（notification `ExitObserved`、`exit.code == 0`、drain deadline/eof/unread/stderr bytes、`SpawnPhase::Live` record、bounded teardown）は変更しない。
4. 意図的 mid-mint death サイト（`--crash-after-msgs=2`）は変更しない。

## Acceptance Criteria

- [ ] 4 サイトの `--leave-heir-after-msgs` が handshake 完結（`Scan`+`CreateInstance`）より後の message を指し、各テストが mint 成功後にその message を 1 回送信する構造になっている（exit が mint window に非可能）。
- [ ] 3rd message の結果を `Ok(_) | Err(HostError::Ipc)` に限定する assert があり、それ以外は fail する。
- [ ] `mise run fmt-check` / `mise run lint` / `mise run test` が green（Windows 11 + Rust 1.99.0）。
- [ ] `cargo test -p ruviform-plugin-host --test l1_proxy_e2e heir`（`mise exec` 経由）の複数回反復で `SpawnFailed` を観測しない。
- [ ] Ubuntu CI での非再現は本変更のマージ後にのみ確かめられるため、報告で未検証である旨を明示する。

## Deferred Design Concerns

- `--spawn-heir` / `--crash-after-msgs` 等他の exit 系 fixture knob の同種リント（handshake message count を超える leave 条件の静的検査）は本件の因果範囲外。必要なら別 issue で検討する。

## Sources and Relations

- Issue #360（失敗 run / rerun の両方の CI 証跡つき）。
- `docs/specs/issue-238-l1-proxy-e2e-flake.md` — 同ファイルの sleep→barrier 置換の前例。「タイミング仮定ではなく因果順序で同期する」方針を継承。
- `docs/designs/plugin-l1-out-of-process.md`、`docs/test-specs/plugin-l1-proxy-contract-tests.md` — L1 contract の正本。
