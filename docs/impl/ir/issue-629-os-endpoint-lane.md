---
id: "IMPL-034Zfqq9gdwjWSsYVJXdgk"
type: "impl"
status: "completed"
title: "Issue #629 os endpoint wiring lane registration"
created: "2026-11-18"
updated: "2026-11-18"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/tasks/issue-629-os-endpoint-lane.md"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-629-os-endpoint-lane.md"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/development-environment.md"
      - type: "file"
        path: "crates/ruviform-audio-test/tests/os_endpoint.rs"
      - type: "file"
        path: "docs/designs/issue-629-test-runtime-wiring.md"
      - type: "file"
        path: "docs/designs/issue-629-virtual-endpoint-lane.md"
      - type: "file"
        path: "docs/designs/issue-629-provisioning-runbook.md"
      - type: "file"
        path: "docs/discovery/issue-629-virtual-endpoint-survey.md"
      - type: "file"
        path: "evidence/rf629/os_endpoint_wiring_lane_selfrun-windows-local.txt"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-629-os-endpoint-lane.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/issue-629-os-endpoint-lane.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-629-os-endpoint-lane.md"
    - "docs/designs/issue-629-os-endpoint-lane.md"
    - "docs/test-specs/issue-629-os-endpoint-lane.md"
    - "docs/impl/ir/issue-629-test-runtime-wiring.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-629-os-endpoint-lane.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-629-os-endpoint-lane.md"
  references:
    - "docs/designs/issue-629-test-runtime-wiring.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-629-os-endpoint-lane.md"
    rejected: []
---

# Issue #629 os endpoint wiring lane registration

## Summary

Issue #629 AC6 残部 slice — AC3 wiring slice
（`docs/impl/ir/issue-629-test-runtime-wiring.md`）で追加された
`crates/ruviform-audio-test/tests/os_endpoint.rs` の
`#[ignore]` leg 2 件に canonical opt-in manual lane
`test-os-endpoint-wiring` を `mise.toml` に登録し、stale な
「canonical lane 未登録」記述を同期した。テスト挙動・製品挙動・
依存関係の変更はない（lane 登録 + docs 同期 + evidence のみ）。

## Context

merged IR `docs/impl/ir/issue-629-test-runtime-wiring.md` の
Follow-ups に「wiring test の canonical lane 登録（`mise.toml`）
— 後続 slice」と明示された未消化作業を消化する。slice 5
（`test-audio-virtual-endpoint`）・slice 6
（`test-midi-virtual-endpoint`）と同型の lane 登録 + docs 同期
+ self-run evidence slice。対象は `docs/tasks/
issue-629-os-endpoint-lane.md`・`docs/plans/
issue-629-os-endpoint-lane.md`・`docs/designs/
issue-629-os-endpoint-lane.md`・`docs/specs/
issue-629-os-endpoint-lane.md`・`docs/test-specs/
issue-629-os-endpoint-lane.md`。

## Implementation

### Changes

- `mise.toml` — `[tasks.test-os-endpoint-wiring]` を
  `test-midi-virtual-endpoint` ブロック直後に追加。`run` は
  `cargo test -p ruviform-audio-test --test os_endpoint --
  --ignored --test-threads=1`（Coordinator 決定の lane 名・
  run 文字列）。description に provisioning 必須・manual・
  never CI・fail-closed `verdict: NotRun`・evidence 格納先
  `evidence/rf629/` を明記。既存 task 行は一切無変更 —
  diff は task ブロック追加のみ。
- `docs/development-environment.md` — lane 一覧表に
  `test-os-endpoint-wiring` 行を `test-midi-virtual-
  endpoint` 行の次に追加し、opt-in lane 注記節に説明 bullet
  を追加。provisioned pair 要件・fail-closed NotRun・CI 対象
  外・専用 lint lane なし・evidence 格納先を明記。
- `crates/ruviform-audio-test/tests/os_endpoint.rs` —
  module doc の「no canonical lane exists yet; `mise.toml`
  is out of this slice's scope」を canonical lane
  `mise run test-os-endpoint-wiring` 参照に更新（doc のみ —
  `#[ignore]` attribute・leg 本体・fail-closed 契約・
  RunMetadata emit 経路は無変更）。
- `docs/designs/issue-629-test-runtime-wiring.md` — §5
  Risks の「`#[ignore]` wiring test には canonical lane が
  ない」「lane 登録は後続 slice」記述を、後続 slice で
  `test-os-endpoint-wiring` として登録済みである旨に
  reconcile（リース範囲内の追記/reconcile のみ）。
- `docs/designs/issue-629-virtual-endpoint-lane.md` —
  末尾に「追記（2026-11-18）— AC6 残部の解消」節を追加
  （append-only、既存記述は無編集）。
- `docs/designs/issue-629-provisioning-runbook.md` — 末尾に
  「10. 追記（2026-11-18）— wiring leg canonical lane の
  登録」節を追加（append-only）。本 runbook の provisioning
  手順・capability probe 判定・NotRun 解釈が本 lane にも適用
  される旨を記録。
- `docs/discovery/issue-629-virtual-endpoint-survey.md` —
  Evidence 節に「### 2026-11-18 — AC6 residual slice」
  dated entry を append（append-only — 既存 entry は無編集）。
  2026-11-17 AC3 wiring entry の「canonical=mise lane 未登録」
  「wiring test の canonical lane 登録 — 後続 slice」が本
  slice で解消された旨を記録。
- `evidence/rf629/os_endpoint_wiring_lane_selfrun-
  windows-local.txt` — canonical lane self-run + 補助
  `--nocapture` 実行の記録（後述 Validation）。
- docs chain 新規作成 — spec/design/plan/task/test-spec/
  本 IR/exp(+jsonl)。docs index（`docs/**/README.md`）は
  生成物のため無変更 — Coordinator が統合時に再生成。

### Decisions During Implementation

- **feature gate なし**: `ruviform-audio-test` の device
  crate 参照（`ruviform-audio-device`・`ruviform-midi-
  device`）は `[dev-dependencies]` の path 依存であり test
  target の compile graph にのみ載るため `--features` 不要
  （audio-device 系 lane の `real-device` feature とは事情が
  異なる — `test-midi-virtual-endpoint` と同じ記法）。
- **専用 lint lane なし**: 新規 test target ではなく既存
  `tests/os_endpoint.rs` の `#[ignore]` leg の canonical
  化であり、既定 `lint ruviform-audio-test`
  （`cargo clippy --locked --all-targets -p`）が `os_endpoint`
  target を既に coverage（merged IR Validation に記録）。
  Coordinator 判断で確定 — slice 6 先例どおり。
- **merged IR 非接触**: `docs/impl/ir/issue-629-test-
  runtime-wiring.md` はリース外のため変更せず、Follow-up
  消化は本 slice 側の IR・design・survey 追記で記録。
- **append-only 文書の扱い**: survey Evidence 節・両 lane
  design は dated entry/追記のみで既存 entry・記述を編集
  しない。`os_endpoint.rs` module doc と wiring design の
  stale 記述のみリース範囲内で更新/reconcile。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-629-os-endpoint-lane.md` — lane 登録の
  決定（feature gate なし・lint lane なし・append-only 同期
  方針）と self-run の fail-closed 観測を記録。

### Rejected

- なし。

## Validation

全て Windows 11 `10.0.26300.9974` x86_64（`DESKTOP-NH00KUI`、
未 provisioning）で実行。canonical = mise lane、直接 cargo
実行は補助証拠。

- `mise run fmt-check` → exit 0。
- `mise run check ruviform-audio-test` → exit 0。
- `mise run lint ruviform-audio-test` → exit 0。
- `mise run test ruviform-audio-test` → exit 0（runtime
  tests 9 + doctests 3 Pass — deterministic legs 3 件を含む
  既定経路は不変）。
- `mise run test-os-endpoint-wiring`（本 slice 登録 lane の
  self-run）→ exit 0、`#[ignore]` leg 2 件とも `ok`（2
  passed・3 filtered out — deterministic legs は lane 対象外
  であることを確認）。未 provisioning のため libtest capture
  内に `verdict: NotRun`（fail-closed）— Pass を主張しない。
- 補助証拠 `mise exec -- cargo test -p ruviform-audio-test
  --test os_endpoint -- --ignored --nocapture` → exit 0。
  audio leg `observed:"NOT_PROVISIONED — no render↔capture
  virtual name-stem pair"`、MIDI leg `observed:"NOT_PROVISIONED
  — no bidirectional name match"`、いずれも `run_metadata`
  JSON（`result:"not-run"`, `level:"ext-l3"`,
  `environment_kind:"virtual-os-endpoint"`）+ `verdict:
  NotRun` を emit。evidence:
  `evidence/rf629/os_endpoint_wiring_lane_selfrun-windows-local.txt`。
- 既存 lane 退行確認 `mise run test-audio-virtual-endpoint`
  → exit 0（1 passed）、`mise run test-midi-virtual-
  endpoint` → exit 0（2 passed）— 既存 verifier 挙動不変。
- `mise exec -- prek run --all-files` → exit 0（`check-toml`
  が `mise.toml` の TOML 検査を兼ねる）。
- MSRV lane — NotRun（daily CI 管轄、task spec 指定どおり未
  実行）。

## Risks

- lane 登録で既存 task を壊すリスクは task ブロック追加のみ
  の additive 変更で回避 — `prek` の `check-toml` + 既存
  lane self-run で非回帰を確認済み。
- `--ignored` が deterministic legs まで実行する誤解は
  self-run の `3 filtered out` で否定済み — lane 対象は
  `#[ignore]` leg のみ。
- self-run の `verdict: NotRun` を Pass と誤記するリスクは
  evidence・docs・PR で `result:"not-run"`・exit 0 をそのま
  ま記録し Pass 主張を明示的に否定することで回避。

## Follow-ups

merged IR `docs/impl/ir/issue-629-test-runtime-wiring.md` の
Follow-up「wiring test の canonical lane 登録（`mise.toml`）
— 後続 slice」は本 slice で消化（本 IR・design・survey 追記
で記録、merged IR 本体はリース外で無変更）。Issue #629 の
残受入は全て外部 gate: provisioned runner での実 provisioning
・driver install・license 判断・wiring leg 実走 Pass
（AC1/AC2/AC5 実測）・`.github/**` CI/runner 統合・AC4 実値
記録・AC5 実測項目・consumer 実受入（AC7 — `scarecrow173/
synvelis#115`/`#117`/`#126`・`scarecrow173/audio-lens#127`）。
docs index 再生成は Coordinator 統合時。
