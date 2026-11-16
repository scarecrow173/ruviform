---
id: "TSPEC-byCAXVcNUJp9U972Dd16XL"
type: "test-spec"
status: "approved"
title: "Issue #629 provisioning runbook test spec"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-629-provisioning-runbook.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-629-provisioning-runbook.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-629-provisioning-runbook.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-629-provisioning-runbook.md"
    - "docs/tasks/issue-629-provisioning-runbook.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-629-provisioning-runbook.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-629-provisioning-runbook.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 provisioning runbook test spec

## Purpose

`test-midi-virtual-endpoint` lane 登録・provisioning
runbook・capability matrix 確定版・AC7 consumer 照合が
満たすべき契約を固定する。本 spec は lane 定義と docs
記録の正しさを判定する基準であり、provisioned runner
での実走 Pass を代替しない — 未 provision 環境での
`NotRun` 記録も本 spec の合格条件に含む。

## Feature

MIDI 仮想 endpoint loopback verifier の opt-in manual
lane 登録（AC6 残部）と、provisioned runner 向け
provisioning runbook・capability matrix 確定版・consumer
照合の headless 文書化（AC4/AC5/AC7 残部）。

## Rules

- **R1 opt-in manual lane**: `test-midi-virtual-endpoint`
  は provisioning 依存の manual/opt-in task であり、通常
  Daily Full CI の必須前提に無条件追加しない。
  description に provisioning 必須・manual・never CI を
  明記する。
- **R2 canonical 起動式**: lane の `run` は `cargo test -p
  ruviform-midi-device --test loopback -- --ignored
  --test-threads=1`。feature gate が無いため
  `--features` は不要。`--test loopback` は
  `loopback_round_trip` と `real_device_silent_output_
  smoke` の 2 テストを収集し、その依存差を description
  で正直に区別する。
- **R3 fail-closed self-run**: 未 provisioning ホストでの
  self-run は `verdict: NotRun`（NOT_PROVISIONED）を返す。
  Pass・Skip=Pass・`virtual-os-endpoint` 主張を一切行わ
  ない。name match は candidate であり virtual の証明で
  はない。
- **R4 既存 lane 不変**: `test-midi-real-device` の run
  コマンド・`mise.toml` の他 task・3OS 経路は不変。
  description/コメントの同期は実態を正直に記述するのみ。
- **R5 lint coverage の一元化**: 専用 lint lane を追加
  せず、既定 `lint ruviform-midi-device`（`--all-targets`）
  が `loopback` target をカバーする — この判断が docs に
  記録される。
- **R6 候補非決め打ち**: runbook は「名前・インストール
  方法を決め打ちしない」Issue 方針に従い候補別手順 +
  capability probe 判定主導フローとし、採用 endpoint を
  確定しない。
- **R7 license 必須確認**: install 前に各候補の license
  が専用 test runner への install を許容するかの確認を
  必須チェック項目とし、条項の不明点は install せず記録
  する。
- **R8 実測待ちの明記**: capability matrix 確定版は
  ❓/⚠️ 項目と Open Questions #1-#7 を「provisioning
  実測待ち」として明記し、未実測を確定済みと主張しない。
- **R9 consumer 照合の誠実さ**: AC7 記録は headless 照合
  であり consumer 側の独立 Oracle での実受入判定を
  主張しない。機材固有未保証範囲（物理 USB 抜去・ADC/DAC
  特性・機種固有 driver・物理 Direct Monitoring・UMP/MPE
  future）を明記する。
- **R10 正直な証拠**: 実行していない検証を Pass と書か
  ない。lint/compile の Pass を `virtual-os-endpoint`
  の Pass や実 OS L3 実走 Pass と主張しない。
- **R11 stale 参照ゼロ**: `loopback.rs` の「lane 登録不
  要」記述・両 design の Deferred・survey doc の旧記述は
  本 slice の解消範囲に同期される。survey doc Evidence
  節は append-only 規約に従い新規 dated entry で記録する。

## Examples

- **E1 lane 定義**: Given `mise.toml` When
  `[tasks.test-midi-virtual-endpoint]` を確認 Then `run`
  が記録済み設計の cargo test 起動式と一致し、
  description が provisioning 必須・manual・never CI・
  2 テストの依存差を明記する。
- **E2 self-run 未 provision**: Given 仮想 MIDI loopback
  pair が無い host When `mise run test-midi-virtual-
  endpoint` を実行 Then loopback verifier が `verdict:
  NotRun`（NOT_PROVISIONED）を返し、`evidence/rf629/` に
  記録される。`real_device_silent_output_smoke` は GS
  Wavetable Synth の有無で Pass/NotRun を正直に返す。
- **E3 runbook 網羅性**: Given design doc の Provisioning
  Runbook 節 When 内容を確認 Then 専用 runner 要件・
  license 事前確認・候補別 install/cleanup/probe・判定
  フロー・両 lane 検証手順・NotRun 解釈・記録責任分担が
  揃う。
- **E4 matrix 実測待ち**: Given Capability Matrix 確定版
  When ❓/⚠️ 項目を確認 Then survey Open Questions
  #1-#7 が実測待ちとして列挙され「確定済み」と主張する
  記述がない。
- **E5 AC7 照合**: Given design doc の AC7 節 When
  consumer 対応表を確認 Then Synvelis #115/#117/#126・
  AudioLens #127 それぞれの要求と #629 側対応・未保証範囲
  が記録され、consumer Pass を主張しない。
- **E6 stale 参照**: Given `loopback.rs` module doc・両
  design の Deferred・survey doc When lane/解消範囲の記述
  を確認 Then canonical lane `mise run test-midi-virtual-
  endpoint` と本 slice の解消済み範囲に同期されている。

## Guarantees

- lane 登録は additive で既存の全 lane・3OS 検証経路を
  壊さない。`test-midi-real-device` の run は不変。
- runbook は候補を決め打ちせず、license 確認を install の
  前提条件として構造化する。
- 未 provisioning 環境での self-run は常に fail-closed の
  `NotRun` であり、Pass に昇格しない。
- capability matrix・AC7 記録は実測・consumer 受入を
  先取りしない。

## Non-goals

- provisioned runner での実 provisioning・実走 Pass・
  `.github/**` CI 統合・AC3 typed 結線・consumer 実受入 —
  全て後続 slice / 外部 gate / consumer 責務。
- 採用 endpoint の確定・再現 Case の実装 — provisioning
  実測後。
