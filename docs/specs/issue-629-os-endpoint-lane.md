---
id: "SPEC-034ZeZCKCdRoly8j8VOIc9"
type: "spec"
status: "approved"
title: "Issue #629 os endpoint wiring lane registration (AC6 residual)"
created: "2026-11-18"
updated: "2026-11-18"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/specs/issue-629-test-runtime-wiring.md"
    - "docs/designs/issue-629-test-runtime-wiring.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
    - "POLICIES.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-629-os-endpoint-lane.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-629-os-endpoint-lane.md"
    - "docs/plans/issue-629-os-endpoint-lane.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-629-os-endpoint-lane.md"
    - "docs/test-specs/issue-629-os-endpoint-lane.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-629-test-runtime-wiring.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "docs/designs/issue-629-test-runtime-wiring.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-629-os-endpoint-lane.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 os endpoint wiring lane registration (AC6 residual)

## What Should Be Built

Issue #629 の AC6 残部 slice。AC3 wiring slice
（`docs/designs/issue-629-test-runtime-wiring.md`、PR
`scarecrow173/ruviform#751`）で追加された
`crates/ruviform-audio-test/tests/os_endpoint.rs` の `#[ignore]`
wiring leg 2 件（`audio_virtual_endpoint_round_trip`・
`midi_virtual_endpoint_loopback`）の canonical opt-in lane を
`mise.toml` に登録し、stale 記述を同期し、lane self-run の
fail-closed evidence を記録する。3 部構成:

- **AC6 opt-in lane 登録** — `mise.toml` に
  `[tasks.test-os-endpoint-wiring]` を追加する。
  `cargo test -p ruviform-audio-test --test os_endpoint --
  --ignored --test-threads=1` を実行する manual/opt-in task
  で、provisioned virtual audio endpoint pair / MIDI loopback
  pair 必須・never CI・未 provisioning では `verdict: NotRun`
  （fail-closed）を返すことを description に明記する
  （`test-audio-virtual-endpoint` / `test-midi-virtual-endpoint`
  の先例に従う）。feature gate はなく `--features` 不要 —
  device crate は dev-dep で test target のみに載る。専用
  lint lane は追加しない — 既定 `lint ruviform-audio-test`
  （`--all-targets`）が `os_endpoint` target を coverage 済み
  （Coordinator 確定）。`docs/development-environment.md` の
  lane 一覧表と opt-in lane 注記に追記する。
- **stale 記述の同期** — `os_endpoint.rs` module doc の
  「no canonical lane exists yet」記述、
  `docs/designs/issue-629-test-runtime-wiring.md` の
  「canonical lane がない / lane 登録は後続 slice」記述、
  `docs/designs/issue-629-provisioning-runbook.md` の
  「canonical lane 登録は後続 slice」記述、
  `docs/discovery/issue-629-virtual-endpoint-survey.md` の
  Evidence 節を、本 slice で登録した canonical lane
  （`mise run test-os-endpoint-wiring`）に同期する。
- **lane self-run + evidence + docs chain** — 本ホストで
  `mise run test-os-endpoint-wiring` を 1 回実行し、未
  provisioning のため `verdict: NotRun`
  （`NOT_PROVISIONED` 系）の正直な evidence を
  `evidence/rf629/` に記録する（Pass を主張しない）。
  spec→design→plan→task→test-spec→impl ir/exp の docs
  chain を作成する。

本 slice が直接対象とするのは Issue AC6 の headless 残部
（`#[ignore]` wiring leg 用 canonical lane の `mise.toml`
登録と lane 説明の文書化）のみ。provisioned runner での
wiring leg 実走 Pass（AC1/AC2/AC5 実測）・`.github/**`
CI/runner 統合・AC4 実値記録・AC5 実測項目・consumer 実受入
（AC7）は外部 gate であり、fail-closed の NotRun を維持する。

## Why Now

- merged IR `docs/impl/ir/issue-629-test-runtime-wiring.md`
  の Follow-ups に「wiring test の canonical lane 登録
  （`mise.toml`）— 後続 slice」と明示された未消化作業で
  あり、headless で完結する最後の AC6 残部である。
- AC3 wiring slice 時点では `mise.toml` が編集範囲外で
  あり、`os_endpoint.rs` module doc・wiring design・
  provisioning runbook design・survey Evidence に「lane
  未登録 / 後続 slice」記述が残っている。lane 登録と
  同期を 1 slice で閉じる。
- 先行する 2 本の仮想 endpoint lane
  （`test-audio-virtual-endpoint`・`test-midi-virtual-
  endpoint`）と同型の確立済みパターンであり、provisioning
  非依存で登録・self-run できる。

## Users and Value

- **provisioned runner 到着時の後続 slice**:
  `mise run test-os-endpoint-wiring` 一発で wiring leg を
  実行でき、lane 説明が `docs/development-environment.md`
  に揃う。
- **証拠の誠実さ**: lane self-run が未 provisioning 環境で
  `verdict: NotRun`（`run_metadata` の
  `result:"not-run"`/`level:"ext-l3"`/
  `environment_kind:"virtual-os-endpoint"`）を返すことを
  canonical lane 経由で確認し、Pass 偽装の経路が無いことを
  記録で固定する。
- **docs 一貫性**: wiring leg の canonical 起動入口が
  module doc・design・runbook・survey の全記述で一意に
  なる。

## Scope

### In Scope

- `mise.toml` への `[tasks.test-os-endpoint-wiring]` 追加
  （opt-in manual lane、`test-midi-virtual-endpoint` 先例の
  構造・description 記法。配置は同 task 直後）。
- `docs/development-environment.md` の lane 一覧表・opt-in
  lane 注記への追記。
- `crates/ruviform-audio-test/tests/os_endpoint.rs` module
  doc の canonical lane 参照への更新（doc のみ —
  `#[ignore]` attribute・leg 本体・fail-closed 契約・
  RunMetadata emit 経路は不変）。
- `docs/designs/issue-629-test-runtime-wiring.md` の
  canonical lane 未登録記述の reconcile（追記/reconcile の
  み）。
- `docs/designs/issue-629-virtual-endpoint-lane.md`・
  `docs/designs/issue-629-provisioning-runbook.md` への
  dated 追記（append-only — 既存記述は編集しない）。
- `docs/discovery/issue-629-virtual-endpoint-survey.md`
  Evidence 節への新規 dated entry 追記（append-only 規約）。
- `evidence/rf629/` への lane self-run 記録
  （canonical lane 実行ログ + `--nocapture` 補助証拠）。
- docs chain（spec/design/plan/task/test-spec/impl ir・exp
  + `*-exp.jsonl`）。

### Out of Scope

- provisioned runner での wiring leg 実走 Pass（AC1/AC2/
  AC5 実測）・実 provisioning・driver install・license
  判断（repo 外工程）。
- `.github/**` の CI/runner 統合（定期 opt-in 実行）—
  後続の外部 gate。
- AC5 capability matrix ❓/⚠️ 項目の実測・採用 endpoint
  確定・再現 Case 実装 — provisioning 実測後。
- consumer 実受入（AC7 — `scarecrow173/synvelis#115`/`#117`/
  `#126`・`scarecrow173/audio-lens#127`）。
- `crates/ruviform-audio-test/src/` を含む全 crate の src
  変更・新規 dependency/feature/`unsafe`・`Cargo.toml`/
  `Cargo.lock` 変更 — 本 slice は lane 登録 + 文書・
  evidence 同期のみでテスト挙動・製品挙動は不変。
- docs index（`docs/**/README.md`）の再生成 — 生成物で
  あり Coordinator が統合時に再生成するため本 slice では
  触れない。

## Requirements

- 新規 lane は manual/opt-in — 通常 Daily Full CI の必須
  前提に無条件追加しない（Issue #629 本文 CI 節・
  POLICIES.md §5「Windows専用の仮想 Endpoint/実Window/実
  ABI Test Pluginの手動・opt-in・定期実行レーン」）。
  3OS lane は green を維持し、既存 lane・他 OS の経路を
  変えない（`mise.toml` の変更は task ブロック追加のみ）。
- lane の description は provisioning 依存・manual・never
  CI を明記し、未 provisioning で `verdict: NotRun`
  （fail-closed）を返す契約を記述する。`--test-threads=1`
  で実 endpoint/port open を直列化する既存規約を踏襲する。
- RunMetadata / fail-closed 契約を維持する:
  `RunMetadata::for_environment` は NotRun 初期化であり、
  実際に endpoint を stream した run のみ
  `level=ext-l3`・`environment_kind=virtual-os-endpoint` を
  claim 可。仮想 endpoint や provisioned leg の偽 Pass は
  禁止。
- 実行していない検証を Pass と書かない。lane self-run の
  NotRun や lint/compile の Pass を
  `virtual-os-endpoint` の Pass や実 OS L3 実走 Pass と
  主張しない。証拠語彙は Case schema
  （Required/Deferred/NotApplicable × Pass/Fail/NotRun/
  Unsupported）と `environment_kind` に従う。
- `unsafe`・新規 dependency・feature flag の追加はない。
- 文書・コメント・報告は日本語。PR 本文・コミット
  メッセージで他 Issue/リポジトリを参照するときは
  auto-reference を誘発しない inline code 形式を使い、
  対象 Issue への参照は `Refs #629`（`Closes` 不可）。

## Acceptance Criteria

- [ ] `mise.toml` に `[tasks.test-os-endpoint-wiring]` が
  `test-midi-virtual-endpoint` 先例の構造で登録され、
  run が `cargo test -p ruviform-audio-test --test
  os_endpoint -- --ignored --test-threads=1`、description に
  provisioning 必須・manual・never CI・fail-closed NotRun・
  evidence 格納先が明記される。既存 task 行は不変。
- [ ] 専用 lint lane は追加されず、既定 `lint
  ruviform-audio-test`（`--all-targets`）が `os_endpoint`
  target を coverage する旨が design 文書に記録される。
- [ ] `docs/development-environment.md` の lane 一覧表と
  opt-in lane 注記に `test-os-endpoint-wiring` が追記される。
- [ ] `os_endpoint.rs` module doc・wiring design・
  virtual-endpoint-lane design・provisioning-runbook
  design・survey doc の stale な「lane 未登録 / 後続
  slice」記述が canonical lane `mise run
  test-os-endpoint-wiring` に同期される（append-only
  文書は dated entry で記録）。
- [ ] `mise run test-os-endpoint-wiring` の本ホスト
  self-run が exit 0 + `verdict: NotRun`
  （`NOT_PROVISIONED` 系）と `run_metadata`
  （`result:"not-run"`・`level:"ext-l3"`・
  `environment_kind:"virtual-os-endpoint"`）を
  `evidence/rf629/` に記録し、Pass を主張しない。
- [ ] Issue #629 の残受入（provisioned 実走 Pass・
  `.github/**` 統合・AC4 実値記録・AC5 実測・consumer
  照合）は本 slice 対象外として docs/PR に明記される。

## Deferred Design Concerns

- provisioned runner での wiring leg 実走 Pass（AC1/AC2/
  AC5 実測）・実 provisioning・driver install・license
  判断 — 後続 slice / repo 外工程。再開条件: 専用
  runner での provisioning 確立。
- `.github/**` CI/runner 統合（定期 opt-in 実行）— 後続の
  外部 gate。POLICIES §5 の opt-in lane 規範に対応。
- AC5 capability matrix ❓/⚠️ 項目の実測・採用 endpoint
  確定・再現 Case 実装 — provisioning 実測後。
- consumer 実受入（AC7 — `scarecrow173/synvelis#115`/`#117`/
  `#126`・`scarecrow173/audio-lens#127`）— consumer 側の
  独立 Oracle での実受入判定。
- docs index 再生成 — Coordinator 統合時。
- 実体は `docs/designs/issue-629-os-endpoint-lane.md` の
  Deferred Design Concerns に記録する。

## Sources and Relations

- `scarecrow173/ruviform#629`（本 Issue、open のまま — PR は
  `Refs`）、`scarecrow173/ruviform#627`（Case Schema v1・
  `environment_kind`・fail-closed NotRun の契約元）、
  `scarecrow173/ruviform#628`（Test Runtime — wiring leg の
  供給側）。
- `docs/impl/ir/issue-629-test-runtime-wiring.md`（merged
  IR — Follow-ups「wiring test の canonical lane 登録
  （`mise.toml`）— 後続 slice」の消化対象。本 slice では
  リース外のため変更しない）。
- `docs/designs/issue-629-test-runtime-wiring.md`（AC3
  wiring design — verdict→`RunMetadata` 対応付け・
  fail-closed 契約の正本）、`docs/specs/
  issue-629-test-runtime-wiring.md`。
- `docs/designs/issue-629-virtual-endpoint-lane.md`（5th
  slice — audio 側 lane 登録の先例）・
  `docs/designs/issue-629-provisioning-runbook.md`
  （closeout slice — MIDI 側 lane 登録と provisioning
  runbook の正本）。
- `docs/discovery/issue-629-virtual-endpoint-survey.md`
  （1st slice 成果 — capability matrix・`--nocapture`
  注意・Evidence append-only 規約）。
- `POLICIES.md` §2.1（仮想優先の受入・証拠要件）・§5
  （ローカル検証と Daily CI の責務分離・opt-in lane
  規範）。
- 先例: `mise.toml` の `test-audio-virtual-endpoint` /
  `test-midi-virtual-endpoint`（opt-in manual lane の構造・
  description 記法）、`docs/development-environment.md`
  （lane 一覧表・opt-in lane 注記の記法）。
