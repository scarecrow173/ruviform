---
id: "IMPL-034YIlnrAlTVIKDGSVNxwW"
type: "impl"
status: "completed"
title: "Issue #629 provisioning runbook and MIDI virtual lane separation"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "docs/tasks/issue-629-provisioning-runbook.md"
    - "docs/plans/issue-629-provisioning-runbook.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-629-provisioning-runbook.md"
      - type: "doc"
        path: "docs/designs/issue-629-provisioning-runbook.md"
      - type: "doc"
        path: "docs/plans/issue-629-provisioning-runbook.md"
      - type: "doc"
        path: "docs/tasks/issue-629-provisioning-runbook.md"
      - type: "doc"
        path: "docs/test-specs/issue-629-provisioning-runbook.md"
      - type: "doc"
        path: "docs/impl/ir/issue-629-provisioning-runbook.md"
      - type: "doc"
        path: "docs/impl/exp/issue-629-provisioning-runbook.md"
      - type: "doc"
        path: "docs/impl/exp/issue-629-provisioning-runbook-exp.jsonl"
      - type: "evidence"
        path: "evidence/rf629/midi_virtual_endpoint_slice6_selfrun-windows-local.txt"
      - type: "evidence"
        path: "evidence/rf629/midi_real_device_slice6_regression-windows-local.txt"
      - type: "evidence"
        path: "evidence/rf629/audio_virtual_endpoint_slice6_regression-windows-local.txt"
    modified:
      - type: "code"
        path: "mise.toml"
      - type: "doc"
        path: "docs/development-environment.md"
      - type: "code"
        path: "crates/ruviform-midi-device/tests/loopback.rs"
      - type: "doc"
        path: "docs/designs/issue-629-wasapi-endpoint.md"
      - type: "doc"
        path: "docs/designs/issue-629-virtual-endpoint-lane.md"
      - type: "doc"
        path: "docs/discovery/issue-629-virtual-endpoint-survey.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-629-provisioning-runbook.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-629-provisioning-runbook.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-629-provisioning-runbook.md"
    - "docs/impl/exp/issue-629-provisioning-runbook.md"
    - "docs/impl/ir/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/issue-629-provisioning-runbook.md"
    - "docs/plans/issue-629-provisioning-runbook.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-629-provisioning-runbook.md"
  verified-by: []
  references:
    - "docs/specs/issue-629-provisioning-runbook.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-629-provisioning-runbook.md"
    rejected: []
---

# Issue #629 provisioning runbook and MIDI virtual lane separation

## Summary

Issue #629 closeout-prep slice。AC6 残部の MIDI 仮想
endpoint 専用 opt-in lane `test-midi-virtual-endpoint` を
`mise.toml` に登録し、`test-midi-real-device` を run 不変の
包括 lane として位置づけ直した。provisioning runbook・
capability matrix 確定版・AC7 consumer 照合を
`docs/designs/issue-629-provisioning-runbook.md` に記録し、
stale 参照（loopback.rs module doc・両 design の Deferred・
survey doc Evidence）を同期した。lane self-run は未
provisioning のため `verdict: NotRun`（fail-closed）を記録し、
Pass は主張しない。production コードの挙動変更なし
（`loopback.rs` は doc 参照のみ）。

## Context

`docs/tasks/issue-629-provisioning-runbook.md`（plan
`docs/plans/issue-629-provisioning-runbook.md`、spec
`docs/specs/issue-629-provisioning-runbook.md`、design
`docs/designs/issue-629-provisioning-runbook.md`）に従う。
本 slice は Issue AC6 の残部（MIDI 仮想 endpoint 専用 lane
分離）、AC4 残部（provisioning runbook 文書化）、AC5 残部の
headless 範囲（capability matrix 確定・実測待ち明記）、
AC7 の headless 範囲（consumer 照合・機材固有未保証範囲の
文書化）を担当する。実 provisioning・driver install・
license 判断・実走 Pass・`.github/**` CI/runner 統合・AC3
typed 結線は対象外であり、後続 slice / 外部 gate へ
hand-off する。

## Implementation

### Changes

- `mise.toml`: `[tasks.test-midi-virtual-endpoint]` を
  `test-midi-real-device` 直後に追加。`run` は Coordinator
  確定どおり `cargo test -p ruviform-midi-device --test
  loopback -- --ignored --test-threads=1`（`loopback`
  target 全体収集 — `loopback_round_trip` 仮想 pair
  verifier + `real_device_silent_output_smoke` GS
  Wavetable smoke の 2 テスト。feature gate なしのため
  `--features` 不要）。description に provisioning 依存・
  manual・never CI・未 provisioning 時の `verdict:
  NotRun` を明記。`test-midi-real-device` は run 不変の
  まま description/コメントで包括 lane との関係を明記。
  専用 lint lane は追加しない（既定 `lint
  ruviform-midi-device` の `--all-targets` が `loopback`
  target を coverage）。
- `docs/development-environment.md`: lane 一覧表に
  `test-midi-real-device`（従来未収録の doc gap）と
  `test-midi-virtual-endpoint` の行を追加し、opt-in lane
  注記節に本 lane の bullet を追記。
- `crates/ruviform-midi-device/tests/loopback.rs`: module
  doc の「`test-midi-real-device` が `#[ignore]` を収集
  するので lane 登録不要」（stale）と `loopback_round_trip`
  doc comment の canonical lane 参照を `mise run
  test-midi-virtual-endpoint` に更新。挙動不変 — doc のみ。
- `docs/designs/issue-629-provisioning-runbook.md`（新設）:
  provisioning runbook 本体 — 専用 Windows runner 要件
  （repo 外工程・開発 PC 無断 driver 導入禁止）・license
  事前確認の必須ゲート化（VB-CABLE donationware/
  loopMIDI 私的非商用限定/VAC 商用 perpetual/WMS loopback
  OS 付属 — survey Open Question #4）・候補別
  install/cleanup/probe 手順（Audio=VB-CABLE/Hi-Fi/VAC、
  MIDI=loopMIDI/loopBE/WMS loopback、例外=SysVAD）・
  capability probe 判定主導フロー・両 lane 検証手順・
  NotRun 解釈・driver 署名/権限/cleanup の記録責任分担
  （probe best-effort vs runner 残件）・capability matrix
  確定版（❓/⚠️ と Open Questions #1-#7 を実測待ちとして
  明記）・AC7 consumer 照合（Synvelis #115/#117/#126・
  AudioLens #127・機材固有未保証範囲）。
- `docs/designs/issue-629-wasapi-endpoint.md`・
  `docs/designs/issue-629-virtual-endpoint-lane.md`:
  Deferred Design Concerns / Verification Notes の
  本 slice 解消項目（MIDI virtual lane 分離・runbook・
  matrix・AC7 headless）を更新。`.github/**` CI/runner
  統合・実走 Pass・AC3 typed 結線は引き続き後続として残す。
- `docs/discovery/issue-629-virtual-endpoint-survey.md`:
  Evidence 節に 2026-11-17 dated entry を追記
  （append-only 規約に従い既存 entry は不変）— lane
  分離・runbook・matrix・AC7 照合と残件を記録。
- `evidence/rf629/`: 3 lane self-run/regression の実行
  ログを新設（下記 Validation 参照）。
- docs chain: spec/design/plan/task/test-spec/impl ir・
  exp（+ `*-exp.jsonl`）を `issue-629-provisioning-runbook`
  slug で新設。

### Decisions During Implementation

- **`loopback` target 全体収集（Coordinator 第一案）**:
  name filter で `loopback_round_trip` のみに絞る案も
  あったが、target 全体収集 + description で両テストの
  依存差を明記する方針を採用。`real_device_silent_output_
  smoke` は実 WinMM output port への silent 送信 smoke
  であり `virtual-os-endpoint` の証拠にならないことを
  description・design・本記録で明記。test target 分割・
  関数移動の構造変更は行わない。
- **`test-midi-real-device` は run 不変の包括 lane**:
  `-p ruviform-midi-device -p ruviform-audio-plugin-bridge
  -- --ignored` の全収集を維持し、description/注記で
  「物理 port 系 + 全 `#[ignore]` 収集の包括 lane、仮想
  pair 専用は `test-midi-virtual-endpoint`」と明記。
  既存 lane の挙動を変えない。
- **専用 lint lane は追加しない（Coordinator 確定）**:
  `loopback.rs` に `cfg(feature)` ゲートがなく、既定
  `mise run lint ruviform-midi-device`（`cargo clippy
  --locked --all-targets -p`）が `loopback` target を
  coverage 済み。5th slice の `lint-audio-real-input`
  流用とは事情が異なり追加 lane 自体が不要 — design
  文書に記録。
- **`test-midi-real-device` の doc gap を併せて解消**:
  同 lane は `docs/development-environment.md` の lane
  表に行を持たず「未収録の opt-in タスク」注記にも
  未掲載だった。新 lane と併記して表に追加した。
- **survey doc Evidence 節は append-only を維持**:
  既存 entry は編集・並替・削除せず新規 dated entry で
  記録。追記 anchor の直後に未読の末尾行があり一時
  orphaned になったため、末尾まで確認して修正した。
- **docs index は未再生成**: `docs/**/README.md` は
  生成物で本 slice の編集範囲外 — Coordinator 統合時に
  再生成される前提で PR 本文の「未対応の作業」に明記。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-629-provisioning-runbook.md`

### Rejected

- なし

## Validation

- `mise run fmt-check` — Pass（exit 0）。
- `mise run check ruviform-midi-device` — Pass（exit 0、
  `--locked --all-targets`）。
- `mise run lint ruviform-midi-device` — Pass（exit 0、
  `--all-targets` が `loopback` target を coverage）。
- `mise run test ruviform-midi-device` — Pass（exit 0、
  既定経路では `#[ignore]` テストは収集されない）。
- `mise run test-midi-virtual-endpoint`（本 slice 新設
  lane の self-run）— exit 0、`2 passed`。canonical
  `run` に `--nocapture` は含めないため verdict 行は
  libtest capture 内 — 補助証拠（`--nocapture` 付き
  直接実行）で `loopback_round_trip` が
  `verdict: NotRun — NOT_PROVISIONED`（0 input ports・
  1 output port — fail-closed）、`real_device_silent_
  output_smoke` が `verdict: Pass`（GS Wavetable Synth
  実 WinMM port — 仮想証明ではない）を確認。
  evidence: `evidence/rf629/midi_virtual_endpoint_slice6_
  selfrun-windows-local.txt`。
- `mise run test-midi-real-device`（退行確認）— exit 0、
  全 `#[ignore]` テスト ok（`loopback_round_trip` は
  NotRun verdict のまま ok）。
  evidence: `evidence/rf629/midi_real_device_slice6_
  regression-windows-local.txt`。
- `mise run test-audio-virtual-endpoint`（既存 lane 挙動
  不変確認）— exit 0、`1 passed`。
  evidence: `evidence/rf629/audio_virtual_endpoint_slice6_
  regression-windows-local.txt`。
- `mise exec -- prek run --all-files` — Pass（`check-toml`
  が `mise.toml` を検査）。
- MSRV lane — NotRun（daily CI 管轄、task spec 指定どおり
  未実行）。

## Risks

- lane 登録は additive で既存 task・3OS 経路を変更しない。
  `prek` の `check-toml` で TOML 妥当性を確認済み。
- `test-midi-virtual-endpoint` の target 全体収集には
  実 device 依存の `real_device_silent_output_smoke` が
  含まれる — GS Wavetable Synth 非搭載環境では当該テストが
  fail し得るが、これは lane の正直な実 OS 依存であり
  description に明記済み。仮想 pair 未 provision でも
  lane 全体は exit 0（NotRun は ok 扱い）を確認済み。
- self-run は未 provisioning の NotRun — Pass・
  `virtual-os-endpoint`・実 OS L3 実走 Pass を主張しない。
- name match は loopback pair の候補ヒューリスティック
  であり provenance 証明ではない — runbook・design・
  verifier doc のいずれでもこの区別を維持。

## Follow-ups

- provisioned runner での実 provisioning・driver install・
  license 判断・verifier 実走 Pass（AC1/AC2/AC5 実測）—
  後続 slice / repo 外工程。
- `.github/**` CI/runner 統合（定期 opt-in 実行）— 外部
  gate。
- AC3 typed 結線（`ruviform-audio-device`→`ruviform-audio-
  test` dev-dep 不承認・逆方向も未裁定 — 'ask' 扱いのまま
  残す）。
- consumer 側の独立 Oracle での実受入判定
  （`scarecrow173/synvelis#115`/`#117`/`#126`・
  `scarecrow173/audio-lens#127`）。
- `real-abi-test-plugin`↔`test-plugin` の serde 語彙 gap
  — #389 占有解除後または Coordinator 調整で後続 slice。
- docs index 再生成 — Coordinator 統合時。
