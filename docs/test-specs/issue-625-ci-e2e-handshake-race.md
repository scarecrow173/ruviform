---
id: "TSPEC-034XqAxz6XasjXM9geCdT7"
type: "test-spec"
status: "approved"
title: "Issue #625 テスト仕様 — handshake channel-closed 分類の決定性と既存 refusal 契約の維持"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/625"
    - "docs/specs/issue-625-ci-e2e-handshake-race.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-625-ci-e2e-handshake-race.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/test-specs/README.md"
  implements:
    - "docs/plans/issue-625-ci-e2e-handshake-race-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-625-ci-e2e-handshake-race-plan.md"
    - "docs/specs/issue-625-ci-e2e-handshake-race.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-625-ci-e2e-handshake-race-design.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-625-ci-e2e-handshake-race-plan.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-625-ci-e2e-handshake-race.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #625 テスト仕様 — handshake channel-closed 分類の決定性と既存 refusal 契約の維持

## Purpose

「welcome 前に control channel が閉じた」条件の `DriverError` 分類が OS・timing に依らず `Handshake` で一意になることを保証し、同時に channel 断以外の io error が `Io` として区別され続けること(異常の握り潰しがないこと)を固定する。

## Feature

handshake refusal 分類 — `TestDriver::handshake` の `test.hello` 交換における transport error の `DriverError` 写像。

## Rules

- INV-1(channel-death kind → Handshake): hello write/read で `BrokenPipe`/`ConnectionReset`/`ConnectionAborted`/`UnexpectedEof` が返った場合、`DriverError::Handshake` で message が "control channel closed before welcome" を含む。
- INV-2(child exit fallback → Handshake): 上記 kind 以外の io error でも `child.try_wait()` が exit を確認できた場合は同じ `Handshake` に分類される(errno mapping が名前を持たない OS error をカバー)。
- INV-3(非 channel error → Io 維持): 上記いずれにも該当しない io error(例: `InvalidData`)は `DriverError::Io` のまま返る。
- INV-4(既存 refusal 契約): `e2e_disable_env_kills_active_transport` は `Handshake` + "closed before welcome"、`e2e_fail_init_reports_failed_handshake` は `Handshake` + "not ready" を維持。`looks_environment_blocked`(ruviform-ui-gpui)が依存する message prefix を壊さない。
- INV-5(検証意図の維持): test の skip/ignore 化・assertion 緩和・error variant 受容拡大による症状隠しを行わない。

## Examples

- hello write が `BrokenPipe` で失敗 → `Handshake("control channel closed before welcome: …")`(run 43540652567 の観測経路を fake writer で決定的に再現)。
- hello read が `ConnectionReset` で失敗 → 同じ `Handshake` 分類(TCP reset 経路)。
- hello read が frame 途中で `UnexpectedEof` → 同じ `Handshake` 分類。
- hello read が clean EOF → 従来どおり `Handshake`("closed before welcome")。
- hello write が `InvalidData`/その他非 channel kind で失敗 + child 未 exit → `Io` 維持。
- 非 channel kind の io error + child exit 確認済み → `Handshake`(fallback)。

## Guarantees

- `handshake()` が返す error の分類は、同一の「channel closed before welcome」条件に対して OS・timing に依らず一意。
- post-handshake(`call()`)の `Io`/`Disconnected` 区別は不変。
- `spawn_tcp` accept error で child exit が確認できる場合、WouldBlock 経路と同じ `Handshake("app exited before TCP handshake")` に分類される。

## Non-goals

- ubuntu 固有 EPIPE 経路の実機再現(race 条件の実発生は CI 任せ — 契約は決定的 unit test で固定)。
- handshake timeout・stdio stall-before-serve hang の検証(別問題)。
- child 側 serve/agent の挙動検証(既存 e2e が担保)。

## Risk

- kind 集合の過不足: channel-death 以外を `Handshake` に写像すると異常が見えなくなる。kind 集合 + child-exit 確認の 2 段で限定し、非該当は `Io` を維持することで抑止する。
