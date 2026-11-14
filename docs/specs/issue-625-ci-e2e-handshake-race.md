---
id: "SPEC-034XqAdHQcP0noGrfdbRqY"
type: "spec"
status: "approved"
title: "Issue #625 verify(ubuntu-latest) test-full 失敗 — e2e_process handshake refusal の error 分類 race を修正する"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/625"
    - "https://github.com/scarecrow173/ruviform/actions/runs/43540652567"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-625-ci-e2e-handshake-race.md"
      - type: "file"
        path: "docs/designs/issue-625-ci-e2e-handshake-race-design.md"
      - type: "file"
        path: "docs/plans/issue-625-ci-e2e-handshake-race-plan.md"
      - type: "file"
        path: "docs/tasks/issue-625-ci-e2e-handshake-race.md"
      - type: "file"
        path: "docs/test-specs/issue-625-ci-e2e-handshake-race.md"
      - type: "file"
        path: "docs/impl/ir/issue-625-ci-e2e-handshake-race.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements: []
  implemented-by:
    - "docs/plans/issue-625-ci-e2e-handshake-race-plan.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-625-ci-e2e-handshake-race-design.md"
    - "docs/test-specs/issue-625-ci-e2e-handshake-race.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-625-ci-e2e-handshake-race.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #625 verify(ubuntu-latest) test-full 失敗 — e2e_process handshake refusal の error 分類 race を修正する

## Problem

2026-11-14 の `ci` workflow run([run 43540652567](https://github.com/scarecrow173/ruviform/actions/runs/43540652567)、sha `2613b2388e0413af15913032421c57857e393682` = PR #619 merge commit)で `verify (ubuntu-latest)` のみが失敗した。失敗 step は `mise run test-full`(`cargo test --workspace --profile ci`)で、`crates/ruviform-ui-test/tests/e2e_process.rs:325` の `e2e_disable_env_kills_active_transport` が次の panic で終了した。

```
expected handshake refusal, got Io(Os { code: 32, kind: BrokenPipe, message: "Broken pipe" })
```

同 binary の他 17 test は pass し、他の必須 job(rt-verify・compression-verify・icc-verify・io-structured-verify・ui-gpui-verify ×2OS・bench・verify-macos・l1-real-verify ×3OS・verify windows-latest・msrv-verify・ui-verify ×3OS・ui-gpui-offscreen-verify)は全て success だった。失敗 sha は docs のみの変更で src/test コードに差分がなく、merge 起因の regression ではない — test lane 内の潜在 race/flaky がこの run で初顕在化した。

## Diagnosis(変更前に診断する)

- **child 側の挙動は正しい**: `RVF_TEST_DISABLE=1` のとき `dummy_app` は `serve_from_env` → `disabled_from_env()` で `ServeOutcome::Disabled` を返し、stderr に `dummy_app: Disabled { reason: "RVF_TEST_DISABLE is set" }` を出して exit 0 する(失敗 run のログにも同出力あり)。serve を拒否して即終了するのが仕様どおりの挙動。
- **driver 側の分類が非対称**: `TestDriver::handshake`(`crates/ruviform-ui-test/src/driver.rs`)は `test.hello` を `write_frame` で書き(`?` で `DriverError::Io` がそのまま漏れる)、その後 `read_frame` の `FrameRead::Eof` で `Handshake("control channel closed before welcome")` を返す。
- **同一条件が 2 経路で観測される**: 「welcome 前に channel が閉じた」は、write が child exit 前に pipe buffer に入れば read 側 EOF → `Handshake`、child exit が先なら Linux では stdin pipe の read end が閉じ write が EPIPE → `Io(BrokenPipe)` となる。どちらが先に観測されるかは OS・timing 依存で、ubuntu で `Io(BrokenPipe)`、windows で `Handshake`(成功)という結果分布と整合する。
- **同種の前例**: `f02f0bc6`(wrong-secret テストの Windows RST race を FIN-only + socket 保持で解消)、`63d3d126`(認証拒否テストで graceful close(FIN)を送信し RST 起因の EOF 判定差異を解消)はいずれも「同一の channel 終了条件が OS で異なる error に化ける」class の修正だった。本件はその driver 側対称形 — child 側を直すのではなく、hello 交換中の transport error を handshake refusal へ正規化するのが責務的に正しい境界。

## What Should Be Built

`TestDriver::handshake` が所有する「hello 交換が完了しなかった」分類を決定的にする。「welcome 前に control channel が閉じた/失敗した」という同一条件が、write 側の `BrokenPipe`/`ConnectionReset` 系・read 側の `Eof`/reset・frame 途中切断のいずれで観測されても、同じ `DriverError::Handshake`(test が期待する "closed before welcome" を含む message)に分類されること。channel 断以外の io error は従来どおり `DriverError::Io` として区別を維持する。

## Why Now

`verify (ubuntu-latest)` は必須 job であり、この flaky が main の CI を不定期に落とし続ける。失敗 sha が docs-only で regression でないことが確認済みで、原因は test driver の error 分類契約の穴 — 放置すると同種の flake が別 test・別 OS で再発する。

## Users and Value

- Ruviform の CI 利用者・レビューア: `verify (ubuntu-latest)` の信頼性回復。
- `ruviform-ui-test` の driver API 利用者(gpui windowed lane 等): handshake refusal の分類が OS・timing に依らず一意になり、呼び出し側の error 分岐が予測可能になる。

## Scope

### In Scope

- `crates/ruviform-ui-test/src/driver.rs` の `TestDriver::handshake` における hello 交換中の transport error 分類正規化。
- 同一方針の適用として `spawn_tcp` accept loop の child-exit 観測経路の分類整合(driver.rs 内)。
- 分類契約を固定する決定的な unit test(driver 内の fake reader/writer による error 経路の強制)。
- doc-chain(spec/design/plan/task/test-spec/IR)と `evidence/rf625/` の root cause 証跡。

### Out of Scope

- `dummy_app` / `serve_from_env` / `TestAgent` 等 child 側の挙動変更(disable → serve 拒否 → exit 0 は正しい)。
- `DriverError` の公開 variant 集合・`call()` 等 post-handshake 経路の分類変更。
- `e2e_process.rs` の assertion 緩和・test の skip/ignore 化・variant 受容拡大(症状隠しは禁止)。
- ci-issue-tracker 管理 Issue #625 の本文・marker comment 編集、手動 close。
- `*-msrv` 系タスク、他 crate の変更(変更が波及した場合は POLICIES.md に従い対象 package 引数で追加検証)。

## Requirements

1. 「welcome 前に channel が閉じた」同一条件の `DriverError` 分類が OS・timing に依らず決定的になること。write 側 EPIPE/reset・read 側 EOF/reset・frame 途中切断のいずれでも `Handshake` に分類される。
2. channel 断を意味しない io error(例: `InvalidData` 等)は `DriverError::Io` の区別を維持し、異常を握り潰さないこと。
3. 既存の refusal 契約を維持すること: `e2e_disable_env_kills_active_transport` は `Handshake` + "closed before welcome"、`e2e_fail_init_reports_failed_handshake` は `Handshake` + "not ready" を返し続ける。`looks_environment_blocked`(ruviform-ui-gpui)が扱う分類集合を壊さない。
4. 修正は分類を所有するレイヤー(`TestDriver::handshake`)で行い、test 側の assertion 緩和で症状を隠さないこと。
5. `2613b238..origin/main` に既修正が無いことを確認した上で実装し、最新 main を取り込んで doc-chain・管理 index を再生成してコミットすること。

## Acceptance Criteria

- AC1: `TestDriver::handshake` の hello 交換で、channel-death 系 transport error(`BrokenPipe`/`ConnectionReset`/`ConnectionAborted`/`UnexpectedEof`、および child exit 確認済みの任意 io error)が `DriverError::Handshake`("control channel closed before welcome" を含む)に分類される。
- AC2: 上記以外の io error は `DriverError::Io` のまま返る(過剰分類しない)。
- AC3: `spawn_tcp` の accept error 経路でも、child exit が確認できる場合は既存の WouldBlock 経路と同じ `Handshake`("app exited before TCP handshake")に分類される。
- AC4: driver 内の決定的 unit test が EPIPE/reset/EOF/途中切断/非 channel error/child-exit fallback の各経路の分類を固定する。
- AC5: `mise run fmt-check`、`mise run check ruviform-ui-test`、`mise run lint ruviform-ui-test`、`mise run test ruviform-ui-test --test e2e_process`、`mise run test ruviform-ui-test`、`mise run test-full` が全て Pass する(実行したもののみ Pass と記録)。
- AC6: PR 本文に `Refs #625` を記し、検証コマンド/OS/exit code・NotRun・review-protocol 結果・未対応作業を記載する。Issue 本文・marker comment は編集せず手動 close もしない。

## Non-goals

- ci-issue-tracker / workflow / 他 lane の変更。
- handshake の timeout 追加・stdio `--stall before-serve` の hang 対策(別の潜在問題として evidence に記録のみ)。
- `DriverError` API の破壊的変更、新 variant 追加。
- ubuntu 固有 EPIPE 経路のローカル実機再現(Windows ホストでは困難 — CI 一次ログ + 決定的 unit test で契約を検証)。
