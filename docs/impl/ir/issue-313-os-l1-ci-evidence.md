---
id: "IMPL-034WT5GBcSlZWbHVBMkaQo"
type: "impl"
status: "completed"
title: "Issue #313 OS-L1 既存CI証拠照合・Capability記録・Timeout headless gap 補完"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/313"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-313-os-l1-ci-evidence.md"
    modified:
      - type: "file"
        path: "crates/ruviform-ui-test/src/bin/dummy_app.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/tests/e2e_process.rs"
      - type: "file"
        path: "evidence/rf313-os-capability-matrix/results.jsonl"
      - type: "file"
        path: "evidence/rf313-os-capability-matrix/report.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-313-real-os-input-driver-slice.md"
    - "docs/impl/ir/issue-313-windowed-testio-slice.md"
    - "docs/impl/ir/ci-os-l1-gpui-macos.md"
    - "evidence/rf313-os-capability-matrix/report.md"
    - "evidence/rf313-os-capability-matrix/results.jsonl"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #313 OS-L1 既存CI証拠照合・Capability記録・Timeout headless gap 補完

## Summary

Issue #313 の OS-L1 slice: 対象 SHA `e93c438a`（main HEAD・PR #599
merge）の Full CI run `43428172820` から既存証拠を照合し、
`evidence/rf313-os-capability-matrix/` に #313 scoped の
Capability/Timeout 記録を追加する。あわせて headless
`rvf.uitest.v1` 経路の `DriverError::Timeout` gap を
`dummy_app` の stall モードと `e2e_process.rs` の assert で補完する。
Issue #313 は open 継続（Windows L2/L3 実機・macOS/Linux 実機 lane
は別 scope）。

## Context

- 受入条件（本 slice 転記）: Test-only Host/Driver 中立 API・
  Fake/Headless OS Capability/Receipt/Timeout/Unsupported の不変条件を
  3 OS の既存 CI で Compile/Link/Test し、実行 SHA・Feature・結果を
  照合する。GUI Driver の OS 固有実装がない場合は Build 成功と
  Driver 機能対応を混同せず欠落 Capability の Owner と将来
  Activation を記録する。macOS/Linux は L1 のみ必須で L2/L3 は
  Deferred（NotRun を Pass 扱いしない）。新 CI Workflow は作らない。
- 編集範囲: `evidence/rf313-os-capability-matrix/`、本 IR、
  `crates/ruviform-ui-test/src/bin/dummy_app.rs`、
  `crates/ruviform-ui-test/tests/e2e_process.rs` のみ。
- 対象 SHA は `e93c438adc4af5b498e10926fe859ac1338ac390` で固定。
  Coordinator dispatch 済み Full CI run `43428172820` を採用
  （同 SHA の最新 completed run・conclusion=success）。

## Implementation

### Changes

- `crates/ruviform-ui-test/src/bin/dummy_app.rs`
  - `--stall before-serve|after-serve` を追加（`StallPhase` enum +
    `park_forever`）。`before-serve` は `serve_from_env` の前に、
    `after-serve` は serve 終了後（`Shutdown`/`DriverGone`/
    `Disabled`/`Failed` のいずれの outcome でも）に永久 park する
    非終了 fixture。未知 phase・引数不足は exit 2 で fail-closed。
    既存の `--fail-init`/`--windows` 契約は不変。
- `crates/ruviform-ui-test/tests/e2e_process.rs` — Timeout assert
  3 テスト追加（既存 15 テストの契約は不変）:
  - `e2e_stall_after_serve_wait_exit_times_out`: `test.shutdown` を
    `d.call(ops::SHUTDOWN)` で送り `bye:true` +
    `cleanup.exited:true`（host-loop フラグ。OS process 終了では
    ない）を確認後、`wait_exit(500ms)` が `DriverError::Timeout`
    を返すこと、`kill`+`try_exit_status` で reap されることを
    assert。`driver.shutdown()` 経由だと固定 `EXIT_TIMEOUT`(15s)
    待ちになるため、同一 `wait_exit` 契約を caller bound で直接
    検証する形にした。
  - `e2e_stall_before_serve_tcp_connect_times_out`:
    `TestDriver::spawn_tcp` が dial しない child を
    `CONNECT_TIMEOUT`(15s) で `DriverError::Timeout` として kill
    することを assert（orphan なし）。
  - `e2e_stall_after_driver_eof_child_survives_until_killed`:
    manual spawn + stdin drop（driver EOF = `DriverGone` 経路）でも
    stalled child が生存し続け、明示 kill で reap されることを
    assert。
- `evidence/rf313-os-capability-matrix/results.jsonl`
  - stale 行 `scenario-driver-realos-adapter`（status=Unsupported
    「adapter 未実装」）を OS 別 3 行に訂正 — `RealOsGuiTest` は
    `real-scenario` feature で実装済み。#243 行（旧 lines 11-18、
    現 13-20）は無変更。
  - #313 scoped 行を 19 行追加: `ci-os-l1-verify-test-full` ×3 OS、
    `ci-ui-gpui-verify-gpui-test-uitest` ×3 OS、
    `ci-offscreen-xvfb-real-scenario`（Linux）、`os-input-backend`
    ×3、`visual-capture-backend` ×3、`windowed-uitest-lane` ×3、
    `headless-driver-timeout-e2e` ×3。
- `evidence/rf313-os-capability-matrix/report.md` — 末尾に
  「Issue #313 OS-L1 slice」節を追加（対象 SHA/Run、lane×OS
  カバレッジ、Capability OS 別表、Timeout 補完、NotRun/Unsupported
  owner、実測と仮定の区別）。既存節は無変更。
- `docs/impl/ir/README.md` — `doc-maintenance index apply` で再生成
  （本 IR 追加分）。

### Decisions During Implementation

- Timeout 検証は driver 実装（`driver.rs`/`WindowedIo`）を変更せず
  既存 public API（`call`/`wait_exit`/`spawn_tcp`/`kill`/
  `try_exit_status`）のみで行った — `WindowedIo::JOB_TIMEOUT` は
  `ruviform-ui-gpui` 側で編集範囲外のため対象外とし、IR/report に
  明記。
- stall は「serve 前」と「serve 後」の 2 phase に分けた —
  `before-serve` が TCP `CONNECT_TIMEOUT` accept loop を、
  `after-serve` が `wait_exit` の non-exit 契約をそれぞれ実演する
  最小構成。`--stall` は outcome を問わず park する（失敗時も
  非終了という契約を単純に保つ）。
- `test.shutdown` は `d.shutdown()` でなく `d.call(ops::SHUTDOWN)`
  経由 — `shutdown()` は内部で `wait_exit(EXIT_TIMEOUT=15s)` を
  呼び Timeout 時に report を捨てるため、caller bound の
  `wait_exit` で同一契約を短時間で assert した。
- Capability 記録は「Build/Link 成功 ≠ Driver 機能対応」を守る
  ため、compile+link のみの lane（CI の `windowed_uitest` env-gated
  no-op、macOS の cfg 空 test target）を Pass にせず NotRun とした。

## Related Experiments

### Adopted

なし — Experiment Log は作成していない（機械的な証拠照合+小
テストの slice で、採用設計は task spec と既存 IR で固定済み）。

### Rejected

なし。

## Validation

### CI 証拠照合（対象 SHA `e93c438a`、run 43428172820 conclusion=success）

- `gh run view 43428172820` → status=completed/conclusion=success、
  headSha=`e93c438adc4af5b498e10926fe859ac1338ac390`。
- `gh run view --log` で job log を取得し lane/test 単位で照合:
  - `verify-ubuntu`/`verify-windows`/`verify-macos` `test-full`:
    `ruviform-ui-test` 全 target 実走・0 failed（`e2e_process.rs`
    15 tests 各 OS）。
  - `ui-gpui-verify`(ubuntu/windows)+`ui-gpui-verify-macos`:
    `check-ui-driver-boundary` + fixture lanes + `test-ui-gpui`
    （features `"gpui-test uitest"`）。`windowed_test_app` bin が
    3 OS で Compile+Link+unittest harness 実行を確認。各 job の
    checkout が `e93c438` で一致。
  - `ui-gpui-offscreen-verify`: `real-scenario` feature で
    `real_window` 3/3 pass（105s）。`real_os_input_scenario` は
    Linux で `verdict=unsupported` を正直に記録。

### ローカル検証（Windows 11 Pro x86_64、branch
`test/313-os-l1-evidence-timeout`、コード commit `20762e0a`）

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run fmt-check` | 0 | `cargo fmt --all -- --check` Pass |
| `mise run check ruviform-ui-test` | 0 | `cargo check` Pass |
| `mise run lint ruviform-ui-test` | 0 | `cargo clippy` Pass |
| `mise run test ruviform-ui-test` | 0 | 全 target pass。`e2e_process.rs` **18/18**（既存 15 + 新規 stall/timeout 3）、finished in 15.11s |

補助証拠（canonical = mise）: 上記は全て mise task 経由。cargo
直接実行は行っていない。

レビュー: review-protocol を 2 pass（cooperative / adversarial）+
validation + judgment で実施。CONFIRMED 指摘 2 件を修正済み
（Linux/macOS の per-target test 件数を cargo 並列出力の混在で
誤帰属し得るため suite 合格のみに修正、`--stall` + handshake 失敗
時の orphan 可能性を dummy_app の契約コメントに明記）。残存は
P2/P3 の non-blocking のみ（EOF 生存窓の理論上の false-pass、
TCP timeout による suite +15s/OS — ともに Risks に記録）。
blocker なし。

## Risks

- 新規 timeout テストの Linux/macOS 実走は未実施（merge 後の
  `ci.yml` verify lane が初回）。`--stall` の park は
  `std::thread::park` loop でプラットフォーム差はないが、TCP
  `CONNECT_TIMEOUT` の 15s 実測は Linux/macOS で同じ wall time を
  要する（test suite +15s/OS）。
- `e2e_stall_after_driver_eof_child_survives_until_killed` の
  「500ms 生存」窓は EOF 処理が異常に遅い環境では stall 未到達でも
  pass し得るが、`park_forever` は一度入れば永久に抜けないため
  false-pass は「buggy exit が 500ms 超遅延」する場合に限る —
  主契約の assert は `wait_exit`→`Timeout` 側のテストが担う。
- evidence の capability 判定は e93c438a 時点の CI/lane に基づく。
  merge 後の main SHA で lane 構成が変われば results.jsonl の
  revision 記述と齟齬し得る（統合検証は merge SHA の CI が正本）。

## Follow-ups

- Issue #313 open 継続: Windows L2/L3 実機ケース（#243/SV #114
  委譲）、macOS/Linux 実機 lane、IME/DPI 動的ケース、
  `WindowedIo::JOB_TIMEOUT` の assert（`ruviform-ui-gpui` 側・
  別 slice）。
- merge 後: merge SHA の `ci.yml` で新規 timeout テスト 3 件が
  3 OS verify lane で実走したことを確認し、`results.jsonl` の
  `headless-driver-timeout-e2e` linux/macos 行を実測値で更新する
  のが望ましい（Coordinator の統合検証フローに委譲）。
