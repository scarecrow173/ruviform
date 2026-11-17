---
id: "IMPL-034Zb3GiG71rPTDG6eZpFv"
type: "impl"
status: "completed"
title: "Issue #629 AC3 Test Runtime wiring — verdict to RunMetadata and deterministic signal/Event supply"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "docs/tasks/issue-629-test-runtime-wiring.md"
    - "docs/plans/issue-629-test-runtime-wiring.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-629-test-runtime-wiring.md"
      - type: "doc"
        path: "docs/designs/issue-629-test-runtime-wiring.md"
      - type: "doc"
        path: "docs/plans/issue-629-test-runtime-wiring.md"
      - type: "doc"
        path: "docs/tasks/issue-629-test-runtime-wiring.md"
      - type: "doc"
        path: "docs/test-specs/issue-629-test-runtime-wiring.md"
      - type: "doc"
        path: "docs/impl/ir/issue-629-test-runtime-wiring.md"
      - type: "doc"
        path: "docs/impl/exp/issue-629-test-runtime-wiring.md"
      - type: "doc"
        path: "docs/impl/exp/issue-629-test-runtime-wiring-exp.jsonl"
      - type: "code"
        path: "crates/ruviform-audio-test/tests/os_endpoint.rs"
      - type: "evidence"
        path: "evidence/rf629/test_runtime_wiring_slice_selfrun-windows-local.txt"
      - type: "evidence"
        path: "evidence/rf629/os_endpoint_wiring_selfrun-windows-local.txt"
    modified:
      - type: "code"
        path: "crates/ruviform-audio-test/Cargo.toml"
      - type: "code"
        path: "crates/ruviform-audio-test/src/metadata.rs"
      - type: "code"
        path: "Cargo.lock"
      - type: "doc"
        path: "docs/designs/issue-629-wasapi-endpoint.md"
      - type: "doc"
        path: "docs/designs/issue-629-virtual-endpoint-lane.md"
      - type: "doc"
        path: "docs/designs/issue-629-provisioning-runbook.md"
      - type: "doc"
        path: "docs/discovery/issue-629-virtual-endpoint-survey.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-629-test-runtime-wiring.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-629-test-runtime-wiring.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-629-test-runtime-wiring.md"
    - "docs/impl/exp/issue-629-test-runtime-wiring.md"
    - "docs/impl/ir/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-629-test-runtime-wiring.md"
    - "docs/plans/issue-629-test-runtime-wiring.md"
    - "docs/designs/issue-629-test-runtime-wiring.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-629-test-runtime-wiring.md"
  verified-by: []
  references:
    - "docs/specs/issue-629-test-runtime-wiring.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-629-test-runtime-wiring.md"
    rejected: []
---

# Issue #629 AC3 Test Runtime wiring — verdict to RunMetadata and deterministic signal/Event supply

## Summary

Issue #629 AC3 Test Runtime 完全結線 slice（headless）。依存方向を
**逆方向 dev-dep**（`ruviform-audio-test` → `ruviform-audio-device` /
`ruviform-midi-device`、test target のみ）で裁定・採用し、
`RunMetadata::for_environment` で verdict→RunMetadata 対応付けを
実装、`crates/ruviform-audio-test/tests/os_endpoint.rs` で deterministic
signal/Event を本番 device API 入口へ供給する wiring test を追加した。
未 provisioning の本ホストでは両 `#[ignore]` leg が `run_metadata`
JSON + `verdict: NotRun` を fail-closed に emit し（audio leg は
host の input device 列挙が空のため `no input devices on this
host`、MIDI leg は `NOT_PROVISIONED — no bidirectional name
match`）、Pass・`virtual-os-endpoint` 実走 Pass は主張しない。

## Context

`docs/tasks/issue-629-test-runtime-wiring.md`（plan
`docs/plans/issue-629-test-runtime-wiring.md`、spec
`docs/specs/issue-629-test-runtime-wiring.md`、design
`docs/designs/issue-629-test-runtime-wiring.md`）に従う。本 slice は
Issue AC3 の headless 部分 — verdict→`RunMetadata` の typed 結線と
deterministic signal/Event の `ruviform-audio-test` 供給 — を担当する。
provisioned runner での実走 Pass（AC1/AC2/AC5 実測）・`.github/**`
CI/runner 統合・AC4 実値記録・consumer 実受入（AC7）は外部 gate であり
対象外。device→audio-test dev-dep は前回不承認のまま不変。

## Implementation

### Changes

- `crates/ruviform-audio-test/Cargo.toml`: `[dev-dependencies]` に
  `ruviform-audio-device`・`ruviform-midi-device`（path dep）を追加。
  lib compile graph・AL/SV consumer の dependency resolution を汚染
  しない test-only 依存。`Cargo.lock` を再生成（dev-dep 辺のみ）。
- `crates/ruviform-audio-test/src/metadata.rs`:
  `RunMetadata::for_environment(case_id, requirement, level,
  environment_kind, oracle, parameters, repo, sha)` を追加。
  `result` は `CaseResult::NotRun` で初期化 — 構築だけでは Pass に
  ならない fail-closed。既存 `deterministic` は
  `OsL1 + DeterministicBackend` の shorthand として維持。
  既存 API・serde 語彙は additive のみ（破壊的変更なし）。
- `crates/ruviform-audio-test/tests/os_endpoint.rs`（新設）:
  - deterministic legs（全ホスト実行）: `for_environment` の
    record 構築、`PcmFixture` block→`RunMetadata`、`MidiScenario`
    trace→`RunMetadata` の 3 test。
  - `#[ignore]` `midi_virtual_endpoint_loopback`: production
    `MidiInputEnumerator`/`MidiOutputEnumerator` で双方向 name
    match の candidate pair を選択（`loopback.rs` と同じ規則）。
    send list は `MidiScenario`→`run_midi_trace` 由来（手組み
    なし）。expected は独立 literal 4-event oracle の ordered
    subsequence。timestamp/stamp 不変条件・close/invalidation・
    終端 `RunMetadata` を emit。pair 不在時は `NotRun
    NOT_PROVISIONED` で return（fail しない）。
  - `#[ignore]` `audio_virtual_endpoint_round_trip`:
    `virtual_endpoint.rs` の virtual-name/stem pairing を再利用し、
    negotiated F32 stream を開き `PcmFixture`/`PcmPattern` の
    deterministic PCM を供給。capture を独立 expected PCM
    （marker 間隔/RMS tolerance 系 oracle）と照合。callback
    count・frame delivery・channel 一致・timestamp monotonic・
    clock-domain・capture/callback ordering・finite sample・
    position continuity・lifecycle を観測 evidence として記録
    （latency/error は evidence のみ、未対応 AC5 挙動を Pass
    条件にしない）。pair 不在時は `NotRun NOT_PROVISIONED`。
  - 両 leg とも `environment_kind=virtual-os-endpoint`・
    `level=ext-l3` の `RunMetadata` を emit するが、これは実際に
    endpoint を stream した candidate run のみ — name match
    だけの Pass は OS-L1 証拠に留める honesty 区別を維持。
    `RUVIFORM_RUN_REPO`/`RUVIFORM_RUN_SHA` で repo/sha を記録
    （未設定は `"unknown"`）。
- `Cargo.lock`: `ruviform-audio-test` の dev-dep 辺追加のみ。
- `docs/designs/issue-629-wasapi-endpoint.md`・
  `docs/designs/issue-629-virtual-endpoint-lane.md`・
  `docs/designs/issue-629-provisioning-runbook.md`: AC3 typed
  結線の Deferred Design Concerns を本 slice の裁定・実装結果で
  更新（解消済み + 逆方向 dev-dep 採用・device→audio-test 不承認
  不変を明記）。
- `docs/discovery/issue-629-virtual-endpoint-survey.md`: Evidence
  節に dated entry を append-only 追記（既存 entry 不変）。
- `evidence/rf629/`: 本 slice の self-run/regression ログ 2 件を
  新設（下記 Validation 参照）。
- docs chain: spec/design/plan/task/test-spec/impl ir・exp
  （+ `*-exp.jsonl`）を `issue-629-test-runtime-wiring` slug で
  新設。

### Decisions During Implementation

- **依存方向 = 逆方向 dev-dep（design §1 で ADR 相当の裁定）**:
  `ruviform-audio-device`→`ruviform-audio-test` dev-dep は
  audio-test が引く `ruviform-audio`/`plugin-bridge`/`graph`
  系が device crate の default `cargo test` compile graph に
  載るため不承認のまま。逆方向は crate cycle を生じず
  （`ruviform-audio` は両 device crate に依存しない）、
  `ruviform-audio-plugin-bridge`→`ruviform-midi-device`・
  `ruviform-plugin-test-vst3`→両 device crate の先例と整合。
  dev-dep に留めるため lib/consumer の解決を汚染しない。
  却下候補（verifier 側で RunMetadata 準拠 JSON を emit する
  schema 重複案）は design に記録。
- **`for_environment` は `NotRun` 初期化**: constructor だけで
  Pass にならない fail-closed を型で保証。`deterministic`
  shorthand の既存 semantics（OsL1+DeterministicBackend）は不変。
- **expected/observed の独立 oracle 維持**: Test Runtime が供給
  するのは expected 側の Fixture/metadata。observed 側は独立
  literal/tolerance oracle — 自己参照 Oracle 禁止を維持。
- **wiring lane は `mise.toml` に登録しない**: 本 slice の
  編集範囲外（Coordinator 確定の lane 追加は後続 slice）。
  canonical 実行は `cargo test -p ruviform-audio-test --test
  os_endpoint`（補助証拠として明記）。
- **docs index 未再生成**: `docs/**/README.md` は生成物で
  Coordinator が統合時に再生成 — PR 本文に明記。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-629-test-runtime-wiring.md`

### Rejected

- なし（依存方向の却下候補は design §1 に記録 — 実装実験としては
  全て採用経路）

## Validation

- `mise run fmt-check` — Pass（exit 0）。
- `mise run check ruviform-audio-device` — Pass（exit 0）。
- `mise run lint ruviform-audio-device` — Pass（exit 0）。
- `mise run test ruviform-audio-device` — Pass（exit 0、6 passed
  + doctests）。
- `mise run check ruviform-midi-device` — Pass（exit 0）。
- `mise run lint ruviform-midi-device` — Pass（exit 0）。
- `mise run test ruviform-midi-device` — Pass（exit 0、
  24+2+2 passed + doctests、`#[ignore]` は収集のみ）。
- `mise run check ruviform-audio-test` — Pass（exit 0）。
- `mise run lint ruviform-audio-test` — Pass（exit 0、
  `--all-targets` が `os_endpoint` target を coverage）。
- `mise run test ruviform-audio-test` — Pass（exit 0、
  `os_endpoint.rs`: 3 passed + 2 ignored、全 suite green）。
- `mise run lint-audio-real-input`（`--all-targets --features
  real-device` の `-D warnings`）— Pass（exit 0）。
- `mise run test-audio-virtual-endpoint` — exit 0、1 passed。
  補助 `--nocapture` で `verdict: NotRun — NOT_PROVISIONED`
  （fail-closed・挙動不変）。
- `mise run test-midi-virtual-endpoint` — exit 0、2 passed。
  `loopback_round_trip` は `verdict: NotRun — NOT_PROVISIONED`、
  `real_device_silent_output_smoke` は `verdict: Pass`
  （GS Wavetable — 仮想証明ではない）。
- `mise run test-midi-real-device` — exit 0、全 `#[ignore]`
  包括 lane の挙動不変（退行なし）。
- `mise exec -- cargo test -p ruviform-audio-test --test
  os_endpoint -- --ignored --nocapture`（補助証拠・canonical=
  mise lane 未登録のため直接 cargo）— exit 0、audio leg が
  `observed:"no input devices on this host"`、MIDI leg が
  `observed:"NOT_PROVISIONED — no bidirectional name match"` の
  `run_metadata` JSON（いずれも `result:"not-run"`,
  `level:"ext-l3"`, `environment_kind:"virtual-os-endpoint"`）
  + `verdict: NotRun` を emit。audio leg の理由文字列は host の
  input device 列挙状態に依存し、device verifier lane
  （`virtual_endpoint.rs`）でも `NOT_PROVISIONED` と
  `no input devices` の両方が観測済み — いずれも同じ
  fail-closed NotRun 系。
- `mise exec -- prek run --all-files` — Pass（exit 0）。
- evidence: `evidence/rf629/test_runtime_wiring_slice_selfrun-
  windows-local.txt`、`evidence/rf629/os_endpoint_wiring_
  selfrun-windows-local.txt`。
- MSRV lane・`*-full` lane — NotRun（daily CI 管轄、task spec
  指定どおり未実行）。
- `virtual-os-endpoint` 実走 Pass・実 OS L3 Pass — **主張しない**
  （未 provisioning のため NotRun が正直な結果）。

## Risks

- `#[ignore]` wiring leg は provisioned runner でのみ実走される
  — 本ホストでは NotRun のため Pass 経路は未検証。provisioning
  確立後の初回実走で初めて `environment_kind=virtual-os-endpoint`
  の Pass/Fail が出る（fail-closed 設計どおり）。
- `ruviform-audio-test` の dev-dep 追加で `cargo test -p
  ruviform-audio-test` の compile graph が device crate を含む
  ようになる — device crate は 3OS で compile 可能（既存
  cross-compile 検証済み）だが、Linux の ALSA 系依存解決が
  test compile に載る点は design に記録済み。
- wiring lane の `mise.toml` 登録は後続 slice — それまで
  canonical lane としては未登録（補助証拠で代替記録）。
- name match は candidate ヒューリスティックであり virtual
  endpoint の provenance 証明ではない — verifier・wiring test
  双方でこの区別を維持。

## Follow-ups

- provisioned runner での実 provisioning・driver install・
  license 判断・両 lane + wiring leg の実走 Pass（AC1/AC2/AC5
  実測）— 後続 slice / repo 外工程。
- wiring test の canonical lane 登録（`mise.toml`）— 後続
  slice（本 slice の編集範囲外）。
- `.github/**` CI/runner 統合（定期 opt-in 実行）— 外部 gate。
- AC5 capability matrix ❓/⚠️ 項目の実測・採用 endpoint 確定・
  再現 Case 実装 — provisioning 実測後。
- consumer 実受入（AC7 — `scarecrow173/synvelis#115`/`#117`/
  `#126`・`scarecrow173/audio-lens#127`）。
- docs index 再生成 — Coordinator 統合時。
