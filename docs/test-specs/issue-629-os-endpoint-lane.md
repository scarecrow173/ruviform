---
id: "TSPEC-k5cAm0SZOyBhOgpEHHkVZ4"
type: "test-spec"
status: "approved"
title: "Issue #629 os endpoint wiring lane registration test spec"
created: "2026-11-18"
updated: "2026-11-18"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-629-os-endpoint-lane.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-629-os-endpoint-lane.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-629-os-endpoint-lane.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-629-os-endpoint-lane.md"
    - "docs/tasks/issue-629-os-endpoint-lane.md"
    - "docs/designs/issue-629-test-runtime-wiring.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-629-os-endpoint-lane.md"
    - "docs/designs/issue-629-os-endpoint-lane.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-629-os-endpoint-lane.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 os endpoint wiring lane registration test spec

## Purpose

`test-os-endpoint-wiring` lane 登録と stale 記述同期が満たす
べき契約を固定する。本 spec は lane 定義と docs 同期の正し
さを判定する基準であり、provisioned runner での wiring leg
実走 Pass を代替しない — 未 provision 環境での `NotRun`
記録も本 spec の合格条件に含む。

## Feature

`os_endpoint.rs` の `#[ignore]` OS endpoint wiring leg
（AC3 wiring slice 成果）の canonical opt-in manual lane
登録（AC6 残部 headless）。

## Rules

- **R1 opt-in manual lane**: `test-os-endpoint-wiring` は
  provisioning 依存の manual/opt-in task であり、通常
  Daily Full CI の必須前提に無条件追加しない。
  description に provisioning 必須・manual・never CI・
  fail-closed `verdict: NotRun`・evidence 格納先
  `evidence/rf629/` を明記する。
- **R2 canonical 起動式**: lane の `run` は `cargo test -p
  ruviform-audio-test --test os_endpoint -- --ignored
  --test-threads=1`。`--test os_endpoint` で wiring test
  target に絞り、`--ignored` で `#[ignore]` leg のみを選
  択、`--test-threads=1` で実 endpoint/port open を直列化
  する（`test-audio-virtual-endpoint`・`test-midi-virtual-
  endpoint` と同型）。
- **R3 feature gate なし**: `ruviform-audio-test` の device
  crate 参照は dev-dep で test target のみに含まれるため
  `--features` 不要 — `run` に feature フラグを含めない。
- **R4 対象 leg の限定**: 本 lane が新たに canonical 化す
  るのは `#[ignore]` leg 2 件（`audio_virtual_endpoint_
  round_trip`・`midi_virtual_endpoint_loopback`）のみ。
  deterministic legs 3 件は既定 `cargo test` 経路で実行
  済みであり本 lane の対象外 — lane 定義・docs がこの区別
  を正しく記述する。
- **R5 fail-closed self-run**: 未 provisioning ホストでの
  self-run は exit 0 + `verdict: NotRun`
  （`NOT_PROVISIONED` 系）を返し、`run_metadata` JSON が
  `result:"not-run"`・`level:"ext-l3"`・
  `environment_kind:"virtual-os-endpoint"` を emit する。
  Pass・Skip=Pass・`virtual-os-endpoint` 実走 Pass の主張を
  一切行わない — lint/compile の Pass を実走 Pass と混同
  しない。
- **R6 既存 lane 不変**: `mise.toml` の変更は task ブロッ
  ク追加のみで、既存 task・3OS 経路・他 OS の検証を変え
  ない。既存 opt-in lane（`test-audio-virtual-endpoint`・
  `test-midi-virtual-endpoint`）の退行確認 self-run で挙
  動不変を確認する。
- **R7 lint coverage の一元化**: 専用 lint lane を追加せ
  ず、既定 `lint ruviform-audio-test`（`--all-targets`）
  が `os_endpoint` target を coverage する — この判断が
  design 文書に記録される。
- **R8 stale 参照ゼロ**: 「canonical lane 未登録 / lane
  登録は後続 slice」を示す現行記述は `mise run
  test-os-endpoint-wiring` への参照に更新または dated
  追記で解消される。append-only 文書（survey Evidence・
  両 design 追記）は既存 entry を編集しない。
- **R9 merged IR 非接触**: `docs/impl/ir/issue-629-test-
  runtime-wiring.md`（merged IR）は変更しない — Follow-
  up 消化は本 slice 側の IR・design・survey 追記で記録す
  る。
- **R10 正直な証拠**: 実行していない検証を Pass と書かな
  い。evidence は canonical lane 実行ログを保存し、
  verdict/run_metadata 行採取の直接 cargo 実行は「補助証
  拠（canonical = mise lane）」と明記する。

## Examples

- **E1 lane 定義**: Given `mise.toml` When
  `[tasks.test-os-endpoint-wiring]` を確認 Then `run` が
  `cargo test -p ruviform-audio-test --test os_endpoint --
  --ignored --test-threads=1` と一致し、description が
  provisioning 必須・manual・never CI・fail-closed
  NotRun・evidence 格納先を明記する。
- **E2 self-run 未 provision**: Given 仮想 endpoint pair・
  MIDI loopback pair が無い host When `mise run
  test-os-endpoint-wiring` を実行 Then exit 0 で
  `verdict: NotRun`（`NOT_PROVISIONED` 系）が出力され、
  `evidence/rf629/` に記録される。
- **E3 run_metadata 語彙**: Given `--nocapture` 補助実行
  When `run_metadata:` JSON を確認 Then 両 leg が
  `result:"not-run"`・`level:"ext-l3"`・
  `environment_kind:"virtual-os-endpoint"` を emit し、
  実際に endpoint を stream した run のみが Pass を
  claim し得る契約が維持される。
- **E4 stale 参照**: Given `os_endpoint.rs` module doc・
  wiring design・両 lane design・survey doc When lane 記述
  を確認 Then canonical lane `mise run
  test-os-endpoint-wiring` への参照または dated 追記によ
  る解消記録がある。
- **E5 既存 lane 非回帰**: Given `mise.toml` 変更 When
  `test-audio-virtual-endpoint`・`test-midi-virtual-
  endpoint` を self-run Then 両 lane が exit 0 で既存
  verifier 挙動不変。

## Guarantees

- lane 登録は additive で既存の全 lane・3OS 検証経路を壊
  さない。
- 未 provisioning 環境での self-run は常に fail-closed の
  `NotRun` であり、Pass に昇格しない。
- `#[ignore]` leg のみが本 lane で canonical 化され、
  deterministic legs の既定経路実行は不変。

## Non-goals

- provisioned runner での wiring leg 実走 Pass・実
  provisioning・`.github/**` CI 統合・AC5 実測・consumer
  照合 — 全て後続 slice / 外部 gate。
- `os_endpoint.rs` の leg 本体・`#[ignore]` attribute・
  RunMetadata emit 経路の変更 — doc のみ。
- merged IR（`docs/impl/ir/issue-629-test-runtime-wiring.
  md`）の編集 — リース外。
