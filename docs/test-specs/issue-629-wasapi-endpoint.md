---
id: "TSPEC-034Ybd5eFlJYcz1k7fLguo"
type: "test-spec"
status: "approved"
title: "Issue #629 WASAPI streaming verifier test spec"
created: "2026-11-16"
updated: "2026-11-16"
owners:
  - "scarecrow173"
relations:
  source:
    - "docs/specs/issue-629-wasapi-endpoint.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-629-wasapi-endpoint.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-629-wasapi-endpoint.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-629-wasapi-endpoint.md"
    - "docs/tasks/issue-629-wasapi-endpoint.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-629-wasapi-endpoint.md"
    - "docs/designs/issue-629-wasapi-endpoint.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-629-wasapi-endpoint.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 WASAPI streaming verifier test spec

## Purpose

Issue #629 4th slice の WASAPI streaming verifier
（`crates/ruviform-audio-device/tests/virtual_endpoint.rs`）と
`endpoint_probe` 拡張が満たすべき契約を固定する。本 spec は
verifier 実装と probe 出力の正しさを判定する基準であり、
provisioned runner での実走 Pass を代替しない — 未 provision 環境
での `NotRun` 記録も本 spec の合格条件に含む。

## Feature

仮想 Audio Endpoint に対する WASAPI render→capture streaming
round-trip の fail-closed 検証（OS-L1 実 OS API 証拠）と、
endpoint capability probe の pair/記録拡張。

## Rules

- **R1 fail-closed NotRun**: host 列挙失敗・virtual pair
  candidate 0 件・非対応環境では assert せず `verdict: NotRun`
  （`NOT_PROVISIONED` 等）を記録して return する。Skip=Pass に
  しない。
- **R2 candidate ≠ proof**: name heuristic（`is_virtual_name` +
  方向語除去 stem 一致）で選んだ pair は candidate であり、
  `virtual-os-endpoint` の証明ではない。verdict/証拠は OS-L1
  「実 OS API 往復」証拠に留め、`environment_kind=
  virtual-os-endpoint` を主張しない。
- **R3 production port のみ**: verifier は
  `DeviceEnumerator`/`Device`/`open_*_stream`/stream trait のみを
  使い、Win32 FFI・cpal 直参照・テスト専用ロジックで endpoint を
  開かない。production 契約は無変更。
- **R4 独立 oracle**: 期待値（試験信号・marker 位置・期待
  level/spectral 特性）は被試験 adapter 外のテスト内独立
  deterministic generator が生成する。adapter は期待値を生成
  しない（自己参照 Oracle 禁止）。比較は cable 内部 SRC
  （48kHz）を前提に marker/level/tolerance 系とし bit-exact を
  要求しない。
- **R5 open+start+callbacks 必須**: stream を開き start しても
  callback が 0 件なら Fail。「開けた」だけでは Pass にしない。
- **R6 callback 契約**: data callback は allocation-free・
  bounded・non-blocking・non-panicking。capture は pre-allocated
  buffer への bounded copy + 観測可能な overflow counter のみ。
  timestamp 単調性・clock domain 一致・frame_position 連続性を
  観測し、契約違反は Fail、未観測項目は正直に記録する。
- **R7 lifecycle**: stop 後の query 応答・再 start/stop・drop を
  確認する。`StreamInvalidated` は device removal 経路のみのため
  本 slice では NotRun 記録（AC5/後続）。
- **R8 probe の誠実さ**: probe は query-only のまま（stream
  open/Initialize/registry 変更なし）。driver 署名・権限・cleanup
  の記録は best-effort で失敗時 `NotRun` note — probe を落とさ
  ない。pair candidate 表は verifier の選定規則と対応づく。
- **R9 証拠語彙**: Case schema（Required/Deferred/NotApplicable ×
  Pass/Fail/NotRun/Unsupported）と `environment_kind` に従う。
  実行していない検証を Pass と書かない。
- **R10 lane**: verifier は `real-device` feature + `#[ignore]`
  で default test lane に載らない。canonical 実行は直接 cargo
  実行（補助証拠）— 専用 mise lane は本 slice では登録しない
  （設計は docs に記録）。

## Examples

- **E1 未 provision 環境**: Given 仮想 name を持つ endpoint が
  無い When `virtual_endpoint` test を `--ignored` で実行 Then
  `verdict: NotRun — NOT_PROVISIONED` が出力され assert/panic
  なく return する（Fail でも Pass でもない）。
- **E2 provisioned pair**: Given VB-CABLE 系の render/capture
  pair が存在 When verifier を実行 Then deterministic 信号が
  render され capture callback で観測され、marker 間隔・level・
  基音スペクトルが tolerance 内で一致し、callback 契約
  （frames>0・timestamp 非減少・domain 一致・frame_position
  連続）を満たせば `verdict: Pass` — ただし証拠は OS-L1 で
  `virtual-os-endpoint` ではない。
- **E3 pair はあるが往復しない**: Given name match の pair が
  存在するが signal が観測されない When verifier を実行 Then
  verdict 付き panic（Fail）— candidate の不成立を Pass に
  しない。
- **E4 probe 拡張**: Given Windows host When `endpoint_probe` を
  実行 Then 全 state 列挙・mix format/IsFormatSupported/
  period/latency・port view に加え、pair candidate 表・driver
  署名・microphone 権限・cleanup の記録が出力され、verdict が
  実装状態と一致する。PowerShell/registry 失敗時は `NotRun`
  note で probe は exit 0。
- **E5 非 Windows**: Given 非 Windows host When verifier/probe
  を compile・実行 Then compile は通り、verdict は `NotRun`
  のみ（3OS lane green）。

## Guarantees

- verifier の Pass は「実 OS API で open→start→callback→stop が
  成立し、独立 oracle の照合を通過した」ことのみを意味する。
- `NotRun`/`Unsupported`/`Deferred` は常に理由付きで記録され、
  Pass に昇格しない。
- probe は従来の列挙・format query 能力を維持しつつ、追加記録が
  best-effort で probe 自体を失敗させない。
- production crate（`ruviform-audio-device` src・
  `ruviform-audio-core`・`ruviform-audio-test`）の契約・依存方向
  は無変更。

## Non-goals

- 実 provisioning・driver install・`virtual-os-endpoint` evidence
  の取得（provisioned runner 後続 slice）。
- reconnect/xrun/permission-denial の再現実測（AC5 — Capability
  Matrix 確定は provisioning 後）。
- `StreamInvalidated` の interactive unplug 確認。
- #628 `PcmFixture`/`RunMetadata` との実結線（dev-dep 承認待ち —
  結線 IF は docs 記録のみ）。
- AC6 lane 登録本体（`mise.toml`/`.github/**`）。
- bit-exact PCM 比較・SRC 位相合わせ。

## Risk

- name/stem match の偽陽性 → signal 未観測で Fail になるが、
  verdict に全 candidate を列挙し人間判別を可能にする。
- SRC による marker/level 劣化 → 閾値は provisional で実測値を
  全記録し、provisioned runner 初回実走で締め直す。
- `Mutex::try_lock` の頻繁な失敗 → overflow counter で観測可能化、
  silent loss にしない。
- PowerShell/registry 照会の環境差 → best-effort `NotRun` で
  probe を落とさない。
