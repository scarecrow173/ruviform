---
id: "SPEC-034XCi8anHlk3T9rYA7ysb"
type: "spec"
status: "draft"
title: "Issue #629 first slice — Windows virtual endpoint survey and capability probe"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "https://github.com/scarecrow173/ruviform/issues/251"
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/synvelis/issues/115"
    - "https://github.com/scarecrow173/synvelis/issues/117"
    - "https://github.com/scarecrow173/synvelis/issues/126"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-629-virtual-endpoint-survey.md"
      - type: "doc"
        path: "docs/designs/issue-629-virtual-endpoint-survey.md"
      - type: "doc"
        path: "docs/test-specs/issue-629-virtual-endpoint-survey.md"
      - type: "doc"
        path: "docs/plans/issue-629-virtual-endpoint-survey.md"
      - type: "doc"
        path: "docs/tasks/issue-629-virtual-endpoint-survey.md"
      - type: "doc"
        path: "docs/impl/ir/issue-629-virtual-endpoint-survey.md"
      - type: "doc"
        path: "docs/impl/exp/issue-629-virtual-endpoint-survey.md"
      - type: "doc"
        path: "crates/ruviform-audio-device/examples/endpoint_probe.rs"
      - type: "doc"
        path: "crates/ruviform-midi-device/examples/midi_endpoint_probe.rs"
      - type: "doc"
        path: "evidence/rf629/endpoint_probe-windows-local.txt"
      - type: "doc"
        path: "evidence/rf629/midi_endpoint_probe-windows-local.txt"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/designs/issue-629-virtual-endpoint-survey.md"
    - "docs/plans/issue-629-virtual-endpoint-survey.md"
  depends-on:
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-629-virtual-endpoint-survey.md"
    - "https://github.com/scarecrow173/ruviform/issues/629"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-629-virtual-endpoint-survey.md"
  references:
    - "evidence/rf629/endpoint_probe-windows-local.txt"
    - "evidence/rf629/midi_endpoint_probe-windows-local.txt"
    - "https://github.com/scarecrow173/ruviform/blob/main/POLICIES.md"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/629"
  deferred-by: []
---

# Issue #629 first slice — Windows virtual endpoint survey and capability probe

## Problem

Issue #629 は Windows 実 OS 仮想 Audio/MIDI Endpoint を通じた本番 WASAPI/WinMM Backend の検証基盤を求めるが、(a) どの仮想 endpoint 技術を採用できるかの根拠がなく、(b) 仮想 endpoint が存在しない環境で「Pass」と誤判定される危険があり、(c) runtime-driven controller を含む完全実装は大きすぎて一度にレビューできない。

本 slice は Issue #629 の **first slice（investigation + capability-probe）** であり、runtime-driven controller・本番 Backend 配線・CI lane 統合・Private Adapter/Provisioning 実装本体は対象外とする。目的は「仮想 endpoint 候補の比較根拠」「capability probe PoC」「fail-closed な provisioning 判定出力」「後続 slice への引き継ぎ事項の記録」を揃えることである。

## Why Now

- RF-VTEST シリーズ（`scarecrow173/ruviform#628` Test Runtime → #629 仮想 OS Endpoint → #630 Test Plugin → #631 GPUI Scenario Runner）は #627 contract を前提に順次実装される。#629 の endpoint 選定根拠がないまま #628 の fixture 設計が進むと、再現不能な capability を前提にした契約になる危険がある。
- `POLICIES.md`（2026-11-15 仮想優先）は仮想 OS endpoint を標準環境としつつ「未 Provision 環境は NotRun/Unsupported で fail-closed」を要求する。probe がこの判定を機械的に出力できることが、後続の evidence 記録の前提になる。
- driver 導入は専用 runner 前提であり、開発機に無断導入してはならない。採用候補の比較（署名・ライセンス・install・capability）を先に文書化しないと、provisioning runbook が後付けになる。

## Users and Value

- **Coordinator / Issue Owner（#629）** — endpoint 採用判断・後続 slice の scope 確定に使う調査結果と、probe の fail-closed 判定が正しく動くことの確認
- **RF-VTEST 後続 slice の実装担当（#628 Test Runtime・#629 残件）** — runtime controller が駆動すべき endpoint の capability 境界・既知コードギャップ（MIDI output 経路不在・`MidiPortOrigin::Unknown`・windows feature 不足）・provisioning runbook の入力
- **専用 Windows test runner の provisioning 担当** — どの endpoint を opt-in で導入し、cleanup をどう記録するかの runbook 素材

## Scope

### In Scope

- 署名済み仮想 Audio Endpoint 候補（VB-CABLE / VB-CABLE Hi-Fi / VAC / その他合法配布物）の比較: 署名状態・ライセンス・install 方法・管理者権限・再起動要否・WASAPI 可視性・制御可能性（hotplug / format switch / device-loss / xrun / latency）
- WinMM から列挙可能な仮想 MIDI loopback 候補（loopMIDI / loopBE / Windows MIDI Services in-box loopback）の比較: OS 版・導入状態依存・WinMM 双方向可視性・動的 port 生成
- SysVAD 系 fallback のスコーピングと採用判定基準の記録（実装はしない）
- Provisioning 判定 + runbook の方針記録（採用 endpoint・manual opt-in install・専用 runner 要件・cleanup・fail-closed provisioning 記録）
- Capability matrix: virtual hotplug / format switch / device-loss / permission denial / xrun / loopback latency × 各候補 → 再現可能と `NotRun` の境界
- Capability probe PoC 2 本（手動実行 example、CI 非実行）:
  - `crates/ruviform-audio-device/examples/endpoint_probe.rs` — WASAPI/MMDevice 列挙・endpoint state/id・format 交渉 query・period/latency query・virtual candidate 検出・provisioning verdict
  - `crates/ruviform-midi-device/examples/midi_endpoint_probe.rs` — WinMM `midiIn*`/`midiOut*` caps 列挙・interface-id・loopback pair 検出・production port view・provisioning verdict
- Probe 出力の証拠保存（`evidence/rf629/`）と、実行した環境の OS/arch/build 記録
- 既知コードギャップの記録（MIDI output 経路不在・`MidiPortOrigin::Unknown`・`midiOutMessage` interface-id 不安定・windows feature 不足）

### Out of Scope

- runtime-driven endpoint controller・Test Runtime との配線・自動 provisioning（#628/後続 slice）
- 本番 Backend への signal/MIDI 生成経路の接続（`Signal/MIDI Generator → 仮想 Endpoint → 本番 Backend` の実配線）
- 専用 Windows test runner の CI/手動タスク統合（workflow・mise task 新設なし）
- Private Adapter/Provisioning の実装本体（OS Driver 固有部・kernel code は分離方針の記録のみ）
- driver の開発機への導入・`Cargo.toml`/`Cargo.lock`/`mise.toml`/`.github/workflows/`/`src/` の変更
- Synvelis/AudioLens consumer 照合の実施（影響範囲の記録のみ）
- Issue #629 の受入チェックボックスの充足判定（本 slice は partial — AC4 recording side・AC6 matrix side のみ）
- macOS/Linux の仮想 endpoint（本 Issue は Windows のみ）

## Requirements

1. **候補比較の記録**: Audio・MIDI・fallback の各候補について、署名状態・ライセンス/配布条件・install 方法（admin/再起動の有無）・WASAPI/WinMM 可視性・制御可能な capability を一次情報に基づき比較し、名前・install 方法を決め打ちしない形で `docs/discovery/` に記録すること。
2. **Capability matrix**: virtual hotplug / format switch / device-loss / permission denial / xrun / loopback latency の各 capability について、候補ごとに「再現可能」「条件付き」「不可/非現実」「未確認（probe 要）」を区別して記録すること。
3. **probe PoC**: 既存の公開 adapter 境界（`ruviform_audio_device`・`ruviform_midi_device`）と enabled な windows feature のみを使い、OS API 面の列挙・format 交渉・period/latency query を行う手動 example を 2 本追加すること。`unsafe` は crate 先例どおり局所許可 + SAFETY コメント。非 Windows では `NotRun` を報告して compile を通すこと。
4. **fail-closed 判定**: probe は provisioning されていない環境・未検出の仮想 endpoint・未対応 capability を `NotRun`/`Unsupported` として報告し、いかなる場合も列挙成功を `virtual-os-endpoint` の Pass と主張しないこと。OS Version/Architecture/Driver/署名/権限/Provisioning/cleanup の記録項目を出力または記録すること。
5. **証拠保存**: probe を実際に実行した環境での出力を `evidence/rf629/` に保存し、未実行環境はその旨を記録すること。
6. **既知ギャップの記録**: `midiOut*` 送信経路不在・`MidiPortOrigin::Unknown`・`midiOutMessage` interface-id 不安定・windows feature 不足（SetupAPI/OS version 取得）を discovery doc に記録し、後続 slice の入力とすること。
7. **本番 API 不変**: 既存 `ruviform_audio_device`/`ruviform_midi_device` の公開 API・src を変更しないこと。OS 固有型を公開面に出さないこと。
8. **3OS lane 維持**: `#[cfg(windows)]` 分離により非 Windows lane が green を維持すること。

## Acceptance Criteria

- [ ] AC1: `docs/discovery/issue-629-virtual-endpoint-survey.md` に Audio 候補（VB-CABLE/Hi-Fi/VAC 等）・MIDI 候補（loopMIDI/loopBE/Windows MIDI Services）・SysVAD fallback の比較と capability matrix が記録され、各候補の署名・ライセンス・install・可視性・制御可能性が一次情報 URL と紐付いている。
- [ ] AC2: `crates/ruviform-audio-device/examples/endpoint_probe.rs` と `crates/ruviform-midi-device/examples/midi_endpoint_probe.rs` が追加され、`cargo check`/`clippy`（workspace lint）を通り、Windows で実行すると OS/arch・endpoint 列挙・state・format/period/latency query・provisioning verdict を出力する。非 Windows では `NotRun` を出力する。
- [ ] AC3: probe 出力が未 provisioning 環境で `NotRun`（fail-closed）を報告し、`virtual-os-endpoint` evidence を主張しないことが実機出力（`evidence/rf629/`）で確認できる。
- [ ] AC4: 既知コードギャップ（MIDI output 経路・`MidiPortOrigin`・`midiOutMessage`・windows feature）が discovery doc に記録され、後続 slice の入力として参照可能である。
- [ ] AC5: `Cargo.toml`/`Cargo.lock`/`mise.toml`/`.github/workflows/`/`src/`・`POLICIES.md`・`AGENTS.md`・`docs/development-environment.md` に変更がない（`git diff` で確認）。driver の導入もない。
- [ ] AC6: doc-chain（spec/design/test-spec/plan/task/IR/exp）と管理 index が整合し、canonical 検証（`mise run fmt-check`、両 crate の `check`/`lint`/`test`、`prek run --all-files`）が Pass する。

## Deferred Design Concerns

| 後送り項目 | 理由 | 追跡先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| runtime-driven endpoint controller・本番 Backend 配線 | 本 slice は investigation + probe のみ。controller は #628 Test Runtime と後続 slice の責務 | `scarecrow173/ruviform#629` 残件・`scarecrow173/ruviform#628` | probe が手動実行のまま残り、再現可能な evidence 収集が自動化されない |
| 採用 endpoint の確定（VB-CABLE vs VAC vs SysVAD、loopMIDI vs WMS） | provisioning 実測（専用 runner）が前提。名前・install を決め打ちしない Issue 方針 | 後続 ADR 候補 | 採用根拠なき endpoint 固定で capability 不足が後から判明する |
| `midiOut*` 送信経路・`MidiPortOrigin` 識別の adapter 変更 | 公開 API 変更を伴うため本 slice 対象外。probe は既存面のみ使用 | 後続 slice / adapter 設計 | MIDI loopback の send→receive 検証が実装不能のまま残る |
| 専用 Windows runner の CI/手動タスク統合 | workflow/mise task 変更は別 Worker リース中・本 slice 範囲外 | 後続 slice | probe が手動実行のまま CI 証拠に乗らない |
| Synvelis/AudioLens consumer 照合の実施 | consumer 影響範囲の記録のみが本 slice の責務 | `scarecrow173/synvelis#115`/`#117`/`#126`、`scarecrow173/audio-lens#127` | 仮想 Pass を consumer 受入の Pass と誤読する余地が残る |

## Sources and Relations

- **Issue #629 本文・受入条件**: first slice の範囲（investigation + capability-probe）と禁止事項（driver 無断導入・決め打ち禁止・Mock を `os_virtual_endpoint` と称さない）を本 spec の Scope/Requirements に反映。
- **Issue #627 contract**: `environment_kind`・Case schema・fail-closed 規則の正本 — `docs/specs/issue-627-virtual-first-verification-contract.md`・`docs/designs/issue-627-virtual-first-verification-contract-design.md`。
- **POLICIES.md**: 2026-11-15 仮想優先改訂 — 仮想 OS endpoint は標準環境、driver 導入は opt-in、物理/市販 Plugin は初回必須でない。
- **Discovery**: `docs/discovery/issue-629-virtual-endpoint-survey.md` — 候補比較・capability matrix・コードギャップ・probe 実測の正本。
- **証拠**: `evidence/rf629/endpoint_probe-windows-local.txt`・`evidence/rf629/midi_endpoint_probe-windows-local.txt`（Windows 11 10.0.26300.9974 x86_64 での実測出力）。
- **関連 Issue**: `scarecrow173/ruviform#628`（Test Runtime）、`scarecrow173/ruviform#251`/`#253`（実 backend）、`scarecrow173/ruviform#294`（consumer 証拠集約）、`scarecrow173/synvelis#115`/`#117`/`#126`、`scarecrow173/audio-lens#127`。
