---
id: "SPEC-U5ytIWyq22nGIRlXTz2LIN"
type: "spec"
status: "approved"
title: "Issue #251 Stage W closeout — 実機 lane 証拠化と残存受入項目の検証・記録"
created: "2026-11-11"
updated: "2026-11-11"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/251"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "https://github.com/scarecrow173/ruviform/issues/251"
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-251-audio-device-real-backend-input-plumbing-headless-slice.md"
    - "docs/impl/ir/issue-565-wasapi-input-frame-position-slice.md"
    - "docs/designs/issue-187-audio-input-port-capture-clock.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "crates/ruviform-audio-device/tests/real_input_e2e.rs"
    - "../../AGENTS.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
---

# Issue #251 Stage W closeout: 実機検証・証拠記録

## 背景と問題

Issue #251（real backend input plumbing・実機検証）は、headless slice（`docs/impl/ir/issue-251-audio-device-real-backend-input-plumbing-headless-slice.md`）で cpal `build_input_stream` 実 open + `InputDataCallback` 経路を実装し、PR #567（Issue #565）で WASAPI-native input path（`frame_position` 供給・`capture` timestamp `QPCPosition`・`input_latency` 実値応答・`AUDCLNT_E_DEVICE_INVALIDATED`→`StreamInvalidated`・default endpoint 変更の再経路・`PositionTracker` 連続性検査）を main に統合済み。実機 lane `mise run test-audio-real-input` も PR #567 時点で `some=194/194, gaps=0` の Pass 実績がある。

残るのは **closeout** であり、実装ではない: Issue #251 の受入形に沿って実機 lane を実行して証拠を記録し、post-merge lane が未網羅の残存項目を実測で埋め、`docs/impl/ir/` の NotRun matrix を「Stage W（Windows+WASAPI）Pass / Stage M,L は NotRun — Issue 全体の Cross OS 完了ではない」と明記できる形に更新する。

## 要件

1. 実機 environment（Windows 11 + WASAPI + Audient iD4）で `mise run test-audio-real-input` を実行し、input stream open→capture→handoff の動作を証拠化する。実行不能な項目は理由付き `NotRun` として正直に記録する。
2. `ruviform-audio-device` の既存 mock/headless テストが回帰しないことを `mise run test`/`test-rt` で証拠化する。
3. 実機依存テストは opt-in lane（Issue #114 `l1-real-verify` パターン準拠 = 既存 `test-audio-real-input` lane）で動作する。新 lane は追加しない。
4. 残存項目の実測（post-merge lane が未網羅のものだけ追加）:
   - `input_latency()` の実測値と `StreamLatency` 契約の観測記録（WASAPI-native path は `IAudioClient::GetStreamLatency` 由来の `Ok(Some)`）。`output_latency()` / per-callback `callback - capture` 実効レイテンシも観測する。
   - real same-clock duplex の同一 clock domain 実測（`real_duplex_streams_agree_on_clock_domain` の実走記録）と、非同一 domain 時の rejection/fallback の観測（同一物理装置では非同一 domain を構成不能なため、contract 判定 `same_clock_domain(domain, UNKNOWN) == false` を実機上で観測・記録する）。
   - Format 不一致の明示検証（非 F32・範囲外 config → `Err(UnsupportedConfig)`）。
   - Device 切断→typed error 通知（`StreamInvalidated` 等）の実機観測。物理抜去は完全自動化不能なため、抜去指示を console 表示して error callback の到達を観測する対話式テストを lane 内に置き、未実施なら手動手順つきで `NotRun` と記録する。
   - xrun/underrun の観測（発生しなければ「観測なし」と正直に記録）。
5. 結果を `docs/impl/ir/` の NotRun matrix に反映する。Stage M/L（macOS/Linux）は本 slice では実行しない・`NotRun` のまま残す。

## 受入条件（Issue #251 本 slice 分）

- [ ] 実機 environment で input stream open→capture→handoff が動作（`mise run test-audio-real-input` 実走記録）。
- [ ] 既存 mock/headless テストが回帰しない（`mise run test`/`test-rt` Pass）。
- [ ] 実機依存テストが opt-in lane（`test-audio-real-input`）で動作し、追加した検証行も同一 lane 内で実行可能（対話式の device 抜去検証は env var opt-in の二重ゲート）。
- [ ] 要件 4 の各項目が実測済みか、理由つき `NotRun`/`NOT-EXERCISED` で記録済み。
- [ ] `docs/impl/ir/` の NotRun matrix が更新され、「Stage W Pass / Stage M,L NotRun — Cross OS 完了ではない」と記載できる。

## スコープ外

- `crates/ruviform-audio-core`（port 契約不変）。他クレート・`.github/workflows/`・`mise.toml`・evidence 配下は変更しない。
- 実機検証で欠落が判明しない限り、adapter の実装変更を行わない（判明時のみ `src/wasapi_input.rs`・`src/stream.rs`・`src/device.rs`・`src/error.rs` の小規模補修を許容）。
- Stage M/L（macOS・Linux）での同契約検証、異 clock 複数 device の完全 drift 補正（#315）、Synvelis consumer 側の受入（#115/#117）。
- Permission denied の実機再現（OS プライバシー設定の手動切替が必要 — 本 slice では手順のみ記録し `NotRun` とする）。
