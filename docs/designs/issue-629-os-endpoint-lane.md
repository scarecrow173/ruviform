---
id: "DESIGN-034Zf150QPyFzf8wtSyMhi"
type: "design"
status: "approved"
title: "Issue #629 os endpoint wiring lane registration design"
created: "2026-11-18"
updated: "2026-11-18"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-629-os-endpoint-lane.md"
    - "docs/specs/issue-629-test-runtime-wiring.md"
    - "docs/designs/issue-629-test-runtime-wiring.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "POLICIES.md"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-629-os-endpoint-lane.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-629-os-endpoint-lane.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-629-os-endpoint-lane.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-629-os-endpoint-lane.md"
    - "docs/tasks/issue-629-os-endpoint-lane.md"
    - "docs/test-specs/issue-629-os-endpoint-lane.md"
    - "docs/designs/issue-629-test-runtime-wiring.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  refines:
    - "docs/designs/issue-629-test-runtime-wiring.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-629-os-endpoint-lane.md"
    - "docs/designs/issue-629-test-runtime-wiring.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-629-os-endpoint-lane.md"
  references:
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  defers: []
  deferred-by: []
---

# Issue #629 os endpoint wiring lane registration design

## Context

Issue #629 AC6 残部 slice。AC3 wiring slice
（`docs/designs/issue-629-test-runtime-wiring.md`、merge
`27c53bd5`）で `crates/ruviform-audio-test/tests/
os_endpoint.rs` が追加され、deterministic legs 3 件は既定
`cargo test -p ruviform-audio-test` 経路で実行されるが、
`#[ignore]` leg 2 件（`audio_virtual_endpoint_round_trip`・
`midi_virtual_endpoint_loopback`）には canonical lane が
なかった — merged IR `docs/impl/ir/
issue-629-test-runtime-wiring.md` の Follow-ups に
「wiring test の canonical lane 登録（`mise.toml`）—
後続 slice」と明示された未消化作業である。

本 slice は slice 5（`docs/designs/
issue-629-virtual-endpoint-lane.md` — `test-audio-virtual-
endpoint`）と slice 6（`docs/designs/
issue-629-provisioning-runbook.md` — `test-midi-virtual-
endpoint`）と同型の lane 登録 + docs 同期 + self-run
evidence slice であり、テスト挙動・製品挙動の変更はない。

## Scope

### In Scope

- `mise.toml` への `[tasks.test-os-endpoint-wiring]` 追加
  — run は `cargo test -p ruviform-audio-test --test
  os_endpoint -- --ignored --test-threads=1`（Coordinator
  決定の lane 名・run 文字列）。description に
  provisioning 必須・manual・never CI・fail-closed
  `verdict: NotRun`・evidence 格納先 `evidence/rf629/` を
  明記。配置は `test-midi-virtual-endpoint` ブロック直後
  （Issue #629 lane 群の近傍）。既存 task 行には一切触れ
  ない。
- `docs/development-environment.md` の lane 一覧表（
  `test-midi-virtual-endpoint` 行の次）と opt-in lane
  注記への追記。
- stale 記述の同期: `os_endpoint.rs` module doc、
  `docs/designs/issue-629-test-runtime-wiring.md` の
  canonical lane 未登録記述（reconcile）、
  `docs/designs/issue-629-virtual-endpoint-lane.md`・
  `docs/designs/issue-629-provisioning-runbook.md` への
  dated 追記、`docs/discovery/
  issue-629-virtual-endpoint-survey.md` Evidence 節への
  新規 dated entry（append-only）。
- lane self-run の fail-closed evidence
  （`evidence/rf629/`）と docs chain。

### Out of Scope

- provisioned runner での wiring leg 実走 Pass（AC1/AC2/
  AC5 実測）・実 provisioning・driver install・license
  判断 — 後続 slice / repo 外工程。
- `.github/**` CI/runner 統合（定期 opt-in 実行）— 外部
  gate。
- AC5 capability matrix 実測・採用 endpoint 確定・再現
  Case 実装 — provisioning 実測後。
- consumer 実受入（AC7 — `scarecrow173/synvelis#115`/`#117`/
  `#126`・`scarecrow173/audio-lens#127`）。
- `docs/impl/ir/issue-629-test-runtime-wiring.md`（merged
  IR — リース外。Follow-up 消化は本 slice 側の IR・
  design・survey 追記で記録）。
- production 契約・test 挙動の変更・新規 dependency/
  feature/`unsafe`・docs index 再生成。

## Components and Boundaries

- **`mise.toml` `[tasks.test-os-endpoint-wiring]`** —
  lane の唯一の canonical 入口。`run` は Coordinator 決定
  の `cargo test -p ruviform-audio-test --test os_endpoint
  -- --ignored --test-threads=1`。`--test os_endpoint` で
  wiring test target に絞り、`--ignored` で `#[ignore]`
  leg のみを選択、`--test-threads=1` で実 endpoint/port
  open を直列化する（`test-audio-virtual-endpoint`・
  `test-midi-virtual-endpoint` と同じ規約）。deterministic
  legs 3 件は `#[ignore]` ではないため本 lane では実行さ
  れず、既定 `cargo test -p ruviform-audio-test` 経路で
  実行済み — 本 lane が新たに canonical 化するのは
  `#[ignore]` leg のみ。
- **feature gate なし** — `ruviform-audio-test` の device
  crate 参照（`ruviform-audio-device`・
  `ruviform-midi-device`）は `[dev-dependencies]` の
  path 依存であり test target の compile graph にのみ
  載るため、`--features` フラグは不要（audio-device 系
  lane の `real-device` feature とは事情が異なる —
  `test-midi-virtual-endpoint` と同じ「feature gate なし」
  の記法に従う）。
- **lint coverage** — 専用 lint lane は追加しない。新規
  test target ではなく既存 `tests/os_endpoint.rs` の
  `#[ignore]` leg の canonical 化であり、既定 `lint
  ruviform-audio-test`（`cargo clippy --locked
  --all-targets -p`）が `os_endpoint` target を既に
  coverage 済み（merged IR Validation に記録）。
  Coordinator 判断で確定 — slice 6（`test-midi-virtual-
  endpoint`）先例どおり。
- **`docs/development-environment.md`** — lane 一覧表に
  `test-os-endpoint-wiring` 行を `test-midi-virtual-
  endpoint` 行の次に追加し、opt-in lane 注記節に説明
  bullet を追加。記法は `test-midi-virtual-endpoint` 行に
  従い、provisioned pair 要件・fail-closed NotRun・CI
  対象外・専用 lint lane なし・evidence 格納先を明記。
- **stale 参照の同期先** — `os_endpoint.rs` module doc
  （「no canonical lane exists yet; `mise.toml` is out of
  this slice's scope」）、`docs/designs/
  issue-629-test-runtime-wiring.md` §5 Risks の canonical
  lane 未登録記述、`docs/designs/
  issue-629-provisioning-runbook.md` Deferred Design
  Concerns の「canonical lane 登録は後続 slice」記述、
  `docs/designs/issue-629-virtual-endpoint-lane.md` の
  AC6 関連記述、survey doc Evidence 節の「canonical=mise
  lane 未登録」記述。survey doc の Evidence 節は
  append-only 規約のため既存 entry は編集せず新規 dated
  entry で記録し、旧記述が本 slice で解消された旨を
  明記する。virtual-endpoint-lane design・
  provisioning-runbook design も append-only のため同様
  に dated 追記で同期する。
- **`docs/impl/ir/issue-629-test-runtime-wiring.md`** —
  merged IR はリース外のため変更しない。Follow-up
  「wiring test の canonical lane 登録（`mise.toml`）—
  後続 slice」の消化は本 slice 側の IR・design・survey
  追記で記録する。

## Data and Control Flow

- lane 実行: `mise run test-os-endpoint-wiring` →
  `cargo test -p ruviform-audio-test --test os_endpoint --
  --ignored --test-threads=1` → `#[ignore]` leg 2 件が
  実行される。各 leg は endpoint/port 列挙 → pair 選択
  （candidate ヒューリスティック）→ 未 provisioning では
  `run_metadata` JSON（`result:"not-run"`・
  `level:"ext-l3"`・`environment_kind:"virtual-os-endpoint"`）
  + `verdict: NotRun` を emit して exit 0（fail-closed）。
- evidence 採取: libtest は passing test の stdout を
  capture するため、verdict 行・`run_metadata:` JSON 行を
  evidence に採る場合は `--nocapture` が必要（survey doc
  の先例どおり）。canonical lane の `run` には
  `--nocapture` を含めない — self-run evidence は lane
  実行ログそのものを保存し、verdict 行採取は補助証拠
  `mise exec -- cargo test -p ruviform-audio-test --test
  os_endpoint -- --ignored --nocapture` を併記して
  「canonical = mise lane、直接 cargo は補助証拠」と明記
  する（slice 5/6 先例）。
- deterministic legs 3 件（`for_environment_builds_
  endpoint_case_record`・`pcm_fixture_block_records_
  run_metadata`・`midi_scenario_trace_records_run_
  metadata`）は `#[ignore]` ではなく既定 `cargo test`
  経路で実行済み — 本 lane の対象外であり、lane 登録で
  挙動は変わらない。

## Non-Functional Constraints

- **fail-closed 維持**: lane self-run が未 provisioning
  で `verdict: NotRun`（`NOT_PROVISIONED` 系）を返す
  ことを確認し、Pass を主張しない。lint/compile の
  Pass を `virtual-os-endpoint` の Pass や実 OS L3 実走
  Pass と主張しない。
- **additive のみ**: `mise.toml` の変更は task ブロック
  追加のみで既存 task・3OS 経路・他 OS の検証は不変。
  `Cargo.toml`/`Cargo.lock`・全 crate の src は無変更。
- **3OS lane green**: `os_endpoint.rs` は device crate を
  dev-dep 経由で参照するが両 device crate は 3OS で
  compile 可能（`ruviform-midi-device` は非 Windows で
  `Unsupported` を返す設計）— wiring leg は
  `#[ignore]` + fail-closed `NotRun` で振る舞う。
- **lint**: workspace clippy（pedantic + `-D warnings`）。
  `unsafe`・新規 dependency・feature flag の追加なし。
- **MSRV lane は実行しない**（daily CI 管轄 — task spec
  指定）。

## Risks and Trade-offs

| Risk | 影響 | 対応 |
| --- | --- | --- |
| lane 登録で既存 task を壊す | 3OS lane / opt-in lane の退行 | `mise.toml` は task ブロック追加のみ — 既存 task 定義に触れない。`prek run --all-files` の `check-toml` で TOML 検査。既存 lane（`test-audio-virtual-endpoint`/`test-midi-virtual-endpoint`）の退行確認 self-run を実施 |
| `--ignored` が deterministic legs まで実行する誤解 | lane の対象範囲の誤記 | `--test os_endpoint -- --ignored` は `#[ignore]` 属性付き leg のみを選択する — deterministic legs は既定経路で実行済みであることを description・docs に明記 |
| self-run を Pass と誤記 | 未 provisioning の NotRun を実走 Pass と誤読 | evidence・docs・PR で `verdict: NotRun`・`result:"not-run"`・exit 0 をそのまま記録し、Pass 主張を明示的に否定。`level=ext-l3`/`environment_kind=virtual-os-endpoint` は emit されるが `result` は `not-run` であり Pass ではない |
| append-only 文書の既存記述を誤って編集 | survey/runbook/lane design の履歴改変 | survey Evidence 節・両 design は dated entry/追記のみ — 既存 entry・記述は編集しない。`os_endpoint.rs` module doc と wiring design の stale 記述のみリース範囲内で更新/reconcile |
| merged IR との不整合に見える | Follow-up 消化が記録されない | `docs/impl/ir/issue-629-test-runtime-wiring.md` はリース外 — 本 slice 側の IR・design・survey 追記で消化を記録し、merged IR 本体は変更しない |

## Deferred Design Concerns

- provisioned runner での wiring leg 実走 Pass（AC1/AC2/
  AC5 実測）・実 provisioning・driver install・license
  判断 — 後続 slice / repo 外工程。再開条件: 専用
  runner での provisioning 確立。
- `.github/**` CI/runner 統合（定期 opt-in 実行）—
  後続の外部 gate。POLICIES §5 の opt-in lane 規範に
  対応。
- AC5 capability matrix ❓/⚠️ 項目の実測・採用 endpoint
  確定・再現 Case 実装 — provisioning 実測後。
- consumer 実受入（AC7 — `scarecrow173/synvelis#115`/`#117`/
  `#126`・`scarecrow173/audio-lens#127`）— consumer 側の
  独立 Oracle での実受入判定。
- docs index 再生成 — Coordinator 統合時。

## Verification Notes

- [ ] `mise run fmt-check` / `mise run check
  ruviform-audio-test` / `mise run lint
  ruviform-audio-test` / `mise run test
  ruviform-audio-test` — Pass。
- [ ] `mise run test-os-endpoint-wiring`（本 slice 登録
  lane の self-run）— 未 provisioning のため exit 0 +
  `verdict: NotRun`（`NOT_PROVISIONED` 系）を
  `evidence/rf629/` に記録（Pass を主張しない）。
- [ ] 補助証拠 `mise exec -- cargo test -p
  ruviform-audio-test --test os_endpoint -- --ignored
  --nocapture` — `run_metadata` JSON（`result:"not-run"`・
  `level:"ext-l3"`・`environment_kind:"virtual-os-endpoint"`）
  と `verdict: NotRun` 行を採取。
- [ ] 既存 lane 退行確認 `mise run test-audio-virtual-
  endpoint` / `mise run test-midi-virtual-endpoint` —
  exit 0・挙動不変。
- [ ] `mise exec -- prek run --all-files`（`check-toml` が
  `mise.toml` の TOML 検査を兼ねる）— Pass。
- [ ] MSRV lane — NotRun（daily CI 管轄、task spec 指定
  どおり未実行）。
