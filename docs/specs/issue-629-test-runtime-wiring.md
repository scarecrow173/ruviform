---
id: "SPEC-e9Hp1wpFJhAUcwotfrV2Sh"
type: "spec"
status: "approved"
title: "Issue #629 AC3 Test Runtime complete wiring — verdict to RunMetadata and deterministic signal/Event supply (headless slice)"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "docs/specs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "POLICIES.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-629-test-runtime-wiring.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-629-test-runtime-wiring.md"
  depends-on:
    - "docs/specs/issue-629-virtual-endpoint-lane.md"
    - "docs/designs/issue-629-virtual-endpoint-lane.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-629-test-runtime-wiring.md"
    - "docs/tasks/issue-629-test-runtime-wiring.md"
    - "docs/test-specs/issue-629-test-runtime-wiring.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-629-virtual-endpoint-lane.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-629-test-runtime-wiring.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 AC3 Test Runtime 完全結線 — headless slice spec

## 目的

Issue `scarecrow173/ruviform#629` の AC3「Test Runtime で生成した
信号・Event を本番 Backend 入口へ投入し、別 Observer/Trace で
Expected/Observed を照合する」の **headless 部分**を完結させる。
前 slice（`issue-629-virtual-endpoint-lane`）で `EnvironmentKind`
語彙と opt-in lane は merge 済みだが、verdict→`RunMetadata` の
typed 結線と deterministic signal/Event の `ruviform-audio-test`
供給は device→audio-test dev-dep 不承認により deferred のまま
残っている。本 slice は依存方向を design で裁定し、採用方向に沿って
結線を実装する。

## スコープ

### 対象

- `ruviform-audio-test` から `ruviform-audio-device` /
  `ruviform-midi-device` への **dev-dependency 追加**（依存方向の
  裁定が逆方向採用の場合に限る）と、device wiring test の新規
  test target。
- verdict→`RunMetadata` 対応付け: endpoint 系 test が
  `case_id`/`requirement`/`result`/`level`/`environment_kind`/
  `oracle`/`parameters`/`repo`/`sha`/`os`/`arch`/`expected`/
  `observed`/`evidence` を Case Schema v1 語彙で emit する結線。
- deterministic signal/Event 供給: `PcmFixture` /
  `oracle::pcm_reference_block` / midi trace API を使い、
  Test Runtime 生成側の Fixture/metadata を expected 側へ供給し、
  観測側の独立 oracle で照合する経路。
- `RunMetadata` への additive API 追加（非 deterministic-backend
  環境の Case record 構築用。既存 API・serde 語彙は不変）。
- 既存 verifier（`virtual_endpoint.rs`/`loopback.rs`）・probe・
  lane の挙動不変の維持と、stale 参照の文書同期。
- `evidence/rf629/` への self-run 記録（未 provisioning のため
  `verdict: NotRun` を正直に記録。Pass を主張しない）。

### 対象外（外部 gate / 後続 slice）

- provisioned Windows runner での実走 Pass（AC1/AC2/AC5）。
- `.github/**` の CI/runner 統合、定期 opt-in 実行（AC6）。
- capability matrix ❓/⚠️ の実測と採用 endpoint 確定（AC5）。
- consumer 実受入（AC7）。
- `mise.toml` の lane 追加・変更（本 slice では wiring test の
  canonical lane を新設しない — 直接 cargo 実行は補助証拠として
  記録する）。
- docs index（`docs/**/README.md`）の再生成（Coordinator が
  統合時に実施）。

## 受入条件（本 slice が直接対象とするもの）

- AC3 headless: `ruviform-audio-test` 側に device wiring test が
  あり、deterministic signal/Event を本番 Backend 入口へ投入し、
  独立 oracle/trace で Expected/Observed を照合した結果を
  `RunMetadata` へ対応付けて emit する。
- verdict→`RunMetadata` 対応付けの spec が design に記録され、
  `Pass`/`Fail`/`NotRun`/`Unsupported`・`environment_kind`・
  `level` の Case Schema v1 語彙が維持される。
- 未 provisioning 環境では `result: NotRun` が fail-closed に
  記録され、実行していない検証を Pass と記録しない。
- `ruviform-audio-test` の既存公開 API・serde 語彙に破壊的変更が
  なく、追加は additive のみ。
- 本番 device 契約（`src/`）に変更がなく、依存方向の裁定結果が
  design 文書と PR 本文に明記される。

## honesty rules（本 slice で維持する不変条件）

- name-match は candidate であって virtual endpoint の証明では
  ない。実際に endpoint を stream した run のみが
  `environment_kind=virtual-os-endpoint` の証拠になり得る。
- `virtual-os-endpoint` の `Pass` は本番 WASAPI/WinMM 経路が
  実際に open/stream した場合に限る。列挙や format 取得のみでは
  Pass にしない。
- deterministic-backend の成功は OS-L1 証拠に留まり、実 OS API・
  GUI・plugin ABI・物理デバイスの証拠と混同しない。
- `physical-device` evidence と `virtual-os-endpoint` evidence は
  分離する。
- lint/compile の Pass を virtual endpoint の実走 Pass と主張
  しない。

## 前提（merge 済み）

- `ruviform-audio-test` API: `PcmFixture`（`src/pcm.rs`）、
  `oracle::pcm_reference_block`（`src/oracle.rs`）、
  `RunMetadata` JSON save/load（`src/metadata.rs`）、
  midi event/trace（`src/midi.rs`）、`VirtualClockConfig`。
- `EnvironmentKind::VirtualOsEndpoint` / `PhysicalDevice` は
  5th slice で追加済み。`real-abi-test-plugin`↔`test-plugin`
  語彙 gap は `scarecrow173/ruviform#725` Phase B で解消済み。
- 既存 lane: `test-audio-virtual-endpoint`、
  `test-midi-virtual-endpoint`、`test-midi-real-device`、
  `lint-audio-real-input` は merge 済みで挙動を変えない。
