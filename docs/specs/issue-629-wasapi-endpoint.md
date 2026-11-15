---
id: "SPEC-034YaVh88vUiMap5dBbjln"
type: "spec"
status: "approved"
title: "Issue #629 WASAPI streaming verifier and endpoint probe extension (4th slice)"
created: "2026-11-16"
updated: "2026-11-16"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/specs/issue-629-midi-loopback.md"
    - "POLICIES.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-629-wasapi-endpoint.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-629-wasapi-endpoint.md"
    - "docs/plans/issue-629-wasapi-endpoint.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-629-wasapi-endpoint.md"
    - "docs/test-specs/issue-629-wasapi-endpoint.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/specs/issue-629-midi-loopback.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-629-wasapi-endpoint.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #629 WASAPI streaming verifier and endpoint probe extension (4th slice)

## What Should Be Built

Issue #629 の 4th slice。Audio 側の仮想 Endpoint に対して「本番
Backend 入口を実際に open/stream して signal が往復するか」を
検証する経路を、headless 範囲で実装する。3 部構成:

- **WASAPI streaming verifier
  （`crates/ruviform-audio-device/tests/virtual_endpoint.rs` 新設、
  `#![cfg(feature = "real-device")]` + `#[ignore]`-gated）** —
  production `default_host()`/`DeviceEnumerator` で output/input
  device を列挙 → `endpoint_probe` と同一の name heuristic で仮想
  pair candidate を選定 → 本番 `Device::open_output_stream` /
  `open_input_stream` で stream を開き F32 format を交渉 →
  deterministic な試験信号（sine + impulse marker）を render
  callback から出力 → capture callback で観測 PCM を収集 →
  独立 oracle（marker 位置・level・基音スペクトルを tolerance
  比較。cable 内部 SRC 48kHz 前提のため bit-exact 禁止）と照合 →
  timestamp/clock domain/error 観測 → close/lifecycle 確認 →
  verdict と環境 metadata を出力。pair candidate が無い環境では
  fail-closed の `NotRun`（`NOT_PROVISIONED`）で早期 return し、
  Skip=Pass にしない。
- **`endpoint_probe.rs` 拡張（AC4 記録 + AC1 補助）** —
  render↔capture の virtual pair candidate を方向横断で表示する
  pair 表を追加し、verifier の選定規則と対応づける。AC4 の未実装
  残件として driver 署名（`Get-AuthenticodeSignature` 等の
  PowerShell best-effort — SetupAPI/driver-store 系の windows
  feature は無い）、capture 権限（microphone privacy の registry
  照会 best-effort）、cleanup（probe が永続状態を作らないことの
  明記）を verdict/記録に追加する。provisioning runbook・verdict
  文言を本 slice の実装に合わせて更新し、Windows 実機で再実行して
  `evidence/rf629/endpoint_probe-windows-local.txt` を更新する。
- **AC3 Test Runtime 結線の headless 範囲 + lane 設計記録** —
  #628 Test Runtime（`ruviform-audio-test`）の deterministic
  generator・PCM fixture・independent oracle・RunMetadata との
  結線点を本 spec/design で決める。coordinator 判断により
  `ruviform-audio-device` → `ruviform-audio-test` の dev-dep 追加は
  不承認（2026-11-15 承認済み依存リスト外）のため、verifier の
  oracle はテスト内の独立 deterministic generator +
  Expected/Observed（marker/level/tolerance）照合で自足させ、
  #628 fixture/RunMetadata との結線 IF を docs に記録して
  provisioned-runner 後続 slice へ hand-off する。AC6 の opt-in
  実走 lane（mise task + lint lane）の設計を docs に記録するのみ
  で、`mise.toml` 自体は編集しない。

本 slice が直接対象とするのは Issue AC1 の WASAPI render→capture
streaming verifier の headless 実装単位と、AC3 の独立 oracle 結線・
AC4 の記録拡張の headless 範囲。実 provisioning・実走 Pass は
provisioned runner 到着まで外部 gate のまま。

## Why Now

- 1st slice の capability probe（`endpoint_probe.rs` +
  `docs/discovery/issue-629-virtual-endpoint-survey.md`）と 3rd
  slice の MIDI loopback verifier（`tests/loopback.rs`）により
  「列挙 → candidate 選定 → fail-closed NotRun」の型が確立した。
  Audio 側に対応する streaming verifier が無いと、provisioning
  確立後に AC1 の証拠を取る経路が無い。
- `endpoint_probe` の verdict は「open/stream proof は next
  slice」のまま stale になっている — verifier 実装に合わせて
  更新し、pair candidate の可視化と AC4 記録（driver 署名・
  権限・cleanup）を補完する必要がある。
- AC3 は「Test Runtime で生成した信号を本番 Backend 入口へ投入し
  別 Observer で Expected/Observed を照合する」を要求する。
  dev-dep 不承認の判断を受け、独立 oracle の自足実装と #628 結線
  IF の記録で headless 範囲を閉じる。

## Users and Value

- **後続 slice（#629 残受入）**: VB-CABLE/VAC 系の provisioning が
  確立した時点で、verifier を `real-device` feature + `#[ignore]`
  経由で実行するだけで AC1 の render→capture streaming 証拠を
  取得できる。
- **probe の再利用者**: pair candidate 表により provisioning 判定時に
  「どの render endpoint がどの capture endpoint と対になるか」の
  見通しが立ち、driver 署名・権限・cleanup の記録が AC4 の証拠
  要件を補う。
- **Ruviform 自身**: #251 の実 WASAPI backend（cpal +
  `wasapi_input.rs` native input path）を仮想 endpoint 経由で
  dogfooding する最初の経路。production 契約は無変更のまま使う。

## Scope

### In Scope

- `crates/ruviform-audio-device/tests/virtual_endpoint.rs` 新設
  （`#![cfg(feature = "real-device")]` + `#[ignore]`-gated
  verifier。既存 `real_input_e2e.rs` と同じ feature 流用 — 新規
  feature は追加しない）。
- `crates/ruviform-audio-device/examples/endpoint_probe.rs` の拡張
  （pair candidate 表・driver 署名/権限/cleanup の best-effort 記録・
  verdict 文言更新）。
- docs chain（spec/design/plan/task/test-spec/impl ir・exp +
  `*-exp.jsonl`）と `evidence/rf629/` の検証記録、
  `docs/discovery/issue-629-virtual-endpoint-survey.md` の解消・
  確定した Known Code Gap/Open Question の更新。
- AC6 opt-in lane の設計記録（docs のみ — `mise.toml` 編集は
  後続 slice / Coordinator）。
- AC3 の #628 Test Runtime 結線 IF の docs 記録（実結線は
  dev-dep 承認後の後続 slice）。

### Out of Scope

- loopMIDI/VB-CABLE/VAC 等の実 provisioning・driver install・
  `virtual-os-endpoint` evidence の取得と provisioned runner での
  実走 Pass（本 slice は verifier 実装と未 provision 環境での
  fail-closed NotRun 記録まで）。
- reconnect/xrun/permission 再現の実測（AC5 本体 — Capability
  Matrix 確定は runner provisioning 後）。
- AC6 の lane 登録本体（`mise.toml` 編集 — 後続 slice /
  Coordinator）と `.github/**` の CI/runner 統合。
- Test Runtime 完全結線・User-mode controller/runner（#628/#629
  後続 slice）。`ruviform-audio-test` への dev-dep 追加は
  coordinator 不承認のため行わない。
- MIDI `attach()` 配線（`scarecrow173/ruviform#630` slice 管轄）・
  MIDI 側 cancellation（`scarecrow173/ruviform#628` 内部 Deferred）。
- consumer 照合（AC7 — `scarecrow173/synvelis#115`/`#117`/`#126`・
  `scarecrow173/audio-lens#127`）。
- production 契約の変更（`DeviceEnumerator`/`Device`/stream trait は
  read-only 参照 — AC1「既存 #251 実 Backend code のまま」）。
- 新規 dependency・feature flag・`unsafe` の追加（verifier は
  safe code のみ — production port API のみを使い Win32 FFI を
  持ち込まない）。

## Requirements

- verifier は `#[ignore]`-gated で、`real-device` feature 有効時のみ
  compile される（`real_input_e2e.rs` 先例）。canonical 実行入口は
  `cargo test -p ruviform-audio-device --features real-device
  --test virtual_endpoint -- --ignored --test-threads=1` の直接実行
  （補助証拠）— 専用 mise lane は本 slice では登録しない
  （`test-audio-real-input` は `--test real_input_e2e` 固定のため
  本 test を収集しない。lane 設計は docs に記録）。
- fail-closed: host 列挙失敗・virtual pair candidate 0 件・
  非対応環境では assert せず `verdict: NotRun`（`NOT_PROVISIONED`
  等）を記録して early return。Skip=Pass にしない。name match は
  candidate であり `virtual-os-endpoint` の証明ではない
  （`tests/loopback.rs`・#627 `environment_kind` 契約を踏襲）。
- 期待値は被試験 adapter 外の独立 oracle から生成する（自己参照
  Oracle 禁止）。試験信号は deterministic（sine + impulse
  marker）で、比較は cable 内部 SRC（48kHz）を前提に tolerance/
  level/marker 系とし bit-exact を要求しない。
- stream open/start/stop・format 交渉・callback 発火・timestamp
  （callback/capture の順序・clock domain 一致）・error 観測・
  lifecycle（stop 後の query・再 start）を本番契約どおり確認する。
  `StreamInvalidated` は device removal が必要なため本 slice では
  NotRun として正直に記録する（interactive unplug は AC5/後続）。
- probe は引き続き手動 example であり CI で実行しない。Win32/
  PowerShell 依存は `#[cfg(windows)]` の後ろに置き、非 Windows
  では compile 通過 + `NotRun` 出力のみ（3OS lane green）。
- probe の driver 署名・権限・cleanup 記録は best-effort —
  失敗時は `NotRun` note であって probe を落とさない
  （`midi_endpoint_probe.rs` の WMS availability 検出が先例）。
- 実行していない検証を Pass と書かない。証拠語彙は Case schema
  （Required/Deferred/NotApplicable × Pass/Fail/NotRun/
  Unsupported）と `environment_kind` に従う。
- クレート外から到達可能な全公開 enum に enum レベルの
  `#[non_exhaustive]`（本 slice は公開 API を追加しない見込み —
  追加する場合は遵守）。
- `unsafe` は追加しない（verifier は safe code のみ。probe の
  既存 `#![expect(unsafe_code)]` module 内に PowerShell 呼出を
  追加するのみで新規 unsafe block は増やさない — `Command` は
  safe API）。

## Acceptance Criteria

- [ ] `tests/virtual_endpoint.rs` が production
  `DeviceEnumerator`/`Device`/`open_output_stream`/
  `open_input_stream` のみで virtual pair candidate を選定・open・
  stream し、deterministic 信号の render→capture を独立 oracle
  （marker/level/tolerance）で照合する `#[ignore]`-gated verifier
  として実装される。pair 未 provision 環境では fail-closed の
  `NotRun` を記録し、`virtual-os-endpoint` Pass を主張しない。
- [ ] `endpoint_probe` が render↔capture の virtual pair candidate
  を方向横断で表示し、driver 署名・権限・cleanup の best-effort
  記録を verdict/出力に含め、provisioning runbook/verdict 文言が
  本 slice の実装と一致する。Windows 実機で再実行し
  `evidence/rf629/endpoint_probe-windows-local.txt` を更新する。
- [ ] AC3 の結線点（#628 `PcmFixture`/`oracle`/`RunMetadata` を
  使う場合の dev-dep・feature・lane 設計）が spec/design/plan に
  記録され、dev-dep 不承認の判断と代替（テスト内独立 oracle +
  結線 IF の hand-off）が docs と PR 本文に明記される。
- [ ] AC6 の opt-in 実走 lane 設計（mise task 名・対象 test・
  lint lane・provisioned runner 前提）が docs に記録され、
  `mise.toml` は変更されない。
- [ ] `docs/discovery/issue-629-virtual-endpoint-survey.md` の
  本 slice で解消・確定した Known Code Gap/Open Question が更新
  され、新規 evidence が `evidence/rf629/` に記録される。
- [ ] Issue #629 の残受入（実 provisioning・`virtual-os-endpoint`
  evidence・reconnect/xrun・consumer 照合・lane 登録）は本 slice
  対象外として docs に明記される。

## Deferred Design Concerns

- VB-CABLE/VAC/loopMIDI/WMS の実 provisioning と
  `virtual-os-endpoint` evidence — 後続 slice（専用 runner 運用が
  前提）。再開条件: provisioning 確立。
- reconnect/xrun/permission 再現の実測（AC5）・Test Runtime 完全
  結線・User-mode controller/runner・consumer 照合（AC7）— 後続
  slice / 別 Issue。
- AC6 lane 登録本体（`mise.toml` + `.github/**`）— 後続 slice /
  Coordinator。再開条件: verifier merge 後。
- #628 `PcmFixture`/`oracle::pcm_reference_block`/`RunMetadata`
  との実結線 — `ruviform-audio-test` dev-dep 承認が前提。
  再開条件: coordinator による依存承認。
- 実体は `docs/designs/issue-629-wasapi-endpoint.md` の Deferred
  Design Concerns に記録する。

## Sources and Relations

- `scarecrow173/ruviform#629`（本 Issue、open のまま — PR は
  `Refs`）、`scarecrow173/ruviform#627`（Closed — Case Schema v1・
  `environment_kind`・Test Depth の契約元）、
  `scarecrow173/ruviform#628`（Test Runtime — AC3 結線対象）、
  `scarecrow173/ruviform#251`（実 WASAPI backend — probe/verifier
  が検証対象とする production 契約）。
- `docs/discovery/issue-629-virtual-endpoint-survey.md`（1st slice
  成果 — capability matrix・Known Code Gap #4・`IsFormatSupported`
  quirk・`GetStreamLatency` NOT_INITIALIZED 記録）。
- `docs/specs/issue-629-midi-loopback.md` + design/plan/task/
  test-spec（3rd slice chain — `#[ignore]` verifier 型・fail-closed
  NotRun・独立 oracle の先例）。
- `docs/designs/issue-627-virtual-first-verification-contract-design.md`
  （Case schema・`environment_kind` taxonomy の正本）。
- `POLICIES.md` §2.1（仮想優先の受入・証拠要件）・§5（ローカル検証と
  Daily CI の責務分離）。
- 先例: `crates/ruviform-audio-device/tests/real_input_e2e.rs`
  （`real-device` feature + `#[ignore]` + watchdog + atomic 観測の
  real-device lane 先例）、`crates/ruviform-midi-device/tests/
  loopback.rs`（pair candidate 選定・verdict 出力・fail-closed
  NotRun の verifier 先例）、`examples/endpoint_probe.rs` と
  `crates/ruviform-midi-device/examples/midi_endpoint_probe.rs`
  （probe・PowerShell best-effort 検出の先例）。
