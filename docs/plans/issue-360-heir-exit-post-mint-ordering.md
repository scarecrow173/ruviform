---
id: "PLAN-034SmCWRZx5srJB6yelbn6"
type: "plan"
status: "approved"
title: "Issue #360 実施計画 — heir 系 4 サイトの exit を post-mint へ固定"
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
      - type: "file"
        path: "crates/ruviform-plugin-host/src/bin/l1-test-worker.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-360-l1-proxy-e2e-heir-flake.md"
  implemented-by:
    - "docs/tasks/issue-360-t1-heir-exit-post-mint-ordering.md"
  depends-on:
    - "docs/specs/issue-360-l1-proxy-e2e-heir-flake.md"
    - "docs/designs/issue-360-heir-exit-post-mint-ordering.md"
    - "docs/adr/issue-360-mint-child-exit-test-scenario.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-360-heir-exit-post-mint-ordering.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-360-l1-proxy-e2e-heir-flake.md"
    - "docs/designs/issue-360-heir-exit-post-mint-ordering.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #360 実施計画 — heir 系 4 サイトの exit を post-mint へ固定 Implementation Plan

## Goal

`--leave-heir-after-msgs` を `3` に上げて mint 復帰後の 1 command で leave を発火させ、4 サイトの自発 exit を mint commit に因果的に後続させる。product code・assertion 不変条件は不変。

## Scope

単一実施ストリーム（test-file-only 変更 + fixture flag コメント）。1 task。

## Upstream Documents

- `docs/specs/issue-360-l1-proxy-e2e-heir-flake.md`
- `docs/adr/issue-360-mint-child-exit-test-scenario.md`

## Design Inputs

- `docs/designs/issue-360-heir-exit-post-mint-ordering.md`（approved）

## File Map

- `crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs` — 4 サイトの flag 値 `2`→`3` + mint 後 1 command（`Initialize`）+ `{Ok|Err(Ipc)}` oracle + 前提コメント修正。
- `crates/ruviform-plugin-host/src/bin/l1-test-worker.rs` — `--leave-heir-after-msgs` flag 契約コメントに「handshake message count 以下は mint window 内 exit」を明記（code 変更なし）。

## Implementation Sequence

- [ ] `destroy_detaches_reader_when_heir_holds_stdout`（`L1Instance::spawn` 直接経路）: flag `3` + `instance.execute(wire::Command::Initialize)` + oracle。
- [ ] `leave_heir_yields_exit_observed_and_bounded_drain` / `heir_stderr_inside_grace_lands_in_record` / `heir_stderr_after_drain_stop_is_never_read`（composite 経路）: flag `3` + `host.execute(HostCommand::Initialize { handle })` + oracle。
- [ ] `l1-test-worker.rs` の flag doc に mint window の注意を追記。
- [ ] `mise run fmt-check` → `mise run lint` → `mise run test` + heir 反復。

## Dependencies and Risks

- 3rd command の reply が exit 収束に demux で先行される残存 race は `{Ok|Err(Ipc)}` oracle で defined outcome 化（design 参照）。
- `HostCommand::Initialize` は `Created` で legal（既存テストで使用済み）。composite `Err` 伝播は `HostError` をそのまま返す。

## Task Breakdown

- `docs/tasks/issue-360-t1-heir-exit-post-mint-ordering.md` — 上記 Implementation Sequence 全体。

## Verification

- [ ] `mise exec -- cargo test -p ruviform-plugin-host --test l1_proxy_e2e heir`（4 サイト、複数回）。
- [ ] `mise run fmt-check` / `mise run lint` / `mise run test`。
- [ ] Ubuntu CI での非再現はマージ後のみ確認可能 — 報告に明記。

## Open Questions / Gaps

- なし。
