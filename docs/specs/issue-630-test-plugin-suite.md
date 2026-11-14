---
id: "SPEC-034XC2yDsqDluVXBOAyKnC"
type: "spec"
status: "approved"
title: "Issue #630 Ruviform製VST3/CLAP Test Plugin Suite — first slice: VST3 test_gain plugin"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "https://github.com/scarecrow173/synvelis/issues/101"
    - "https://github.com/scarecrow173/synvelis/issues/127"
    - "https://github.com/scarecrow173/synvelis/issues/128"
    - "https://github.com/scarecrow173/synvelis/issues/176"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-630-test-plugin-suite.md"
      - type: "doc"
        path: "docs/designs/issue-630-test-plugin-suite.md"
      - type: "doc"
        path: "docs/test-specs/issue-630-test-plugin-suite.md"
      - type: "doc"
        path: "docs/plans/issue-630-test-plugin-suite.md"
      - type: "doc"
        path: "docs/tasks/issue-630-test-plugin-suite.md"
      - type: "doc"
        path: "docs/impl/ir/issue-630-test-plugin-suite.md"
      - type: "doc"
        path: "evidence/rf630/verification.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
  implements: []
  implemented-by:
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/plans/issue-630-test-plugin-suite.md"
  depends-on:
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
    - "docs/specs/issue-219-consumer-compat-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-630-test-plugin-suite.md"
    - "docs/test-specs/issue-630-test-plugin-suite.md"
    - "POLICIES.md"
    - "crates/ruviform-plugin-vst3/README.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/630"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-630-test-plugin-suite.md"
  references:
    - "evidence/rf630/verification.md"
    - "https://github.com/scarecrow173/ruviform/blob/main/POLICIES.md"
  defers: []
  deferred-by: []
---

# Issue #630 Ruviform製VST3/CLAP Test Plugin Suite — first slice: VST3 test_gain plugin

## What Should Be Built

Issue #630 は、Ruviform の本番 Plugin Host が通常の探索・Load/Process/State/Editor ABI で扱える、決定的な VST3/CLAP **実バイナリ**のテストプラグイン群を構築する Issue である。Mock Plugin や Fake Plugin Port ではなく、実ファイルを製品 Host の本番パスから Discover/Instantiate/Process/Uninstantiate できることを目的とし、独立 Oracle による期待値検証・RT 保証・State 挙動・Native Editor 検証へ段階的に拡張する。

本スライス（first slice）はその最初の垂直断面として、**VST3 `test_gain` プラグインのみ**を新規 test-only crate `crates/ruviform-plugin-test-vst3` に実装する:

- `[lib] crate-type = ["cdylib", "rlib"]` の実 VST3 モジュール（`GetPluginFactory` + プラットフォーム hook を export）
- vendor `"Ruviform Test"`・component 名 `"test_gain"`・adapter fixture（`"Gain"`/`"Ruviform"`）とは別 CID の Audio Module Class + Component Controller Class
- n チャンネル入力 × 正規化 gain パラメータの決定的 DSP（`out[c][n] = in[c][n] × gain(n)`、パラメータイベントは frame offset に対し sample-accurate に適用）
- `ruviform-plugin-vst3` の本番 host bridge（`Vst3Library::load` → `ErasedPlugin` init→prepare→process→teardown）による in-process L0 検証と、crate 自身の `GetPluginFactory` からの in-process COM round-trip 検証
- `assert_no_alloc` による `rt_` prefix の RT non-allocation 検証

## Why Now

- #627（Closed、2026-11-15 仮想優先改訂）で `POLICIES.md` に「実 Plugin ABI と Mock を分離する」受入・証拠契約が確定し、実 ABI バイナリを要求する Case が正式に定義された
- #628（Test Runtime）・#629（仮想 OS Endpoint）が進行中だが、本 slice はそれらへ依存 edge を持たず先行して plugin 側の実 ABI 土台を提供できる
- 既存 `ruviform-plugin-vst3` の adapter fixture（`"Gain"`）は adapter 自身の dogfood 用であり、「test plugin と host を同じ adapter 実装だけで組み立てる自己検証ループ」を禁止する Issue #630 の独立実装要件を満たさない — plugin 側を独立実装する必要がある

## Users and Value

- Ruviform 本番 Plugin Host（`ruviform-plugin-vst3` host bridge / 将来の L1 runner / consumer 側 host）の検証者：市販 Plugin・ライセンス・物理機材なしに、実 ABI の Discover/Instantiate/Process/Automation/State/Editor パスを決定的に検証できる
- Synvelis / AudioLens 等の独立 Consumer：Ruviform Test Plugin を oracle として自 repo の host/adapter 検証に利用できる（AudioLens への新規依存は強制しない）
- 本 crate 自体は出荷物ではなく test fixture — product distribution に含めない

## Scope

### In Scope

- 新規 crate `crates/ruviform-plugin-test-vst3`（cdylib+rlib、公開 Rust API なし、`vst3` binding 型は非公開）
- `test_gain`: stereo main bus pair 宣言・1〜2ch/bus の process 対応、単一 gain パラメータ（plain domain `[0.0, 2.0]`・default `1.0`・normalized `[0.0, 1.0]` 線形写像、`kCanAutomate`）
- sample-accurate パラメータ適用: イベント frame offset を境界に区間ごとの gain を適用（off-by-one を含む golden 検証）
- in-process COM round-trip テスト（factory scan → createInstance → lifecycle → process golden）
- 本番 host path テスト（`Vst3Library::load` で自 crate の cdylib を dlopen → `ErasedPlugin` 経由）
- `rt_` prefix `assert_no_alloc` テスト
- crate README + lib docs の test-only 宣言、workspace `Cargo.toml` members 追加、`Cargo.lock` 再生成
- 本 doc chain（spec/design/plan/task/test-spec/impl/ir/impl/exp）と `evidence/rf630/` 記録

### Out of Scope

- `test_instrument` / `test_stateful` / `test_fault` / `test_editor` の各 plugin class（後続 slice）
- CLAP 版 test plugin crate（format 別クレート分割は design doc で規約化するが実装は後続）
- `crates/ruviform-audio-test`（`scarecrow173/ruviform#628` runtime）への依存 — 未 merge のため依存 edge ゼロ
- L1 out-of-process runner 経路、Native Editor/GUI-L2/EXT-L3 実走、外部 SDK validator 実走、CI lane 統合、consumer oracle（Synvelis/AudioLens）照合
- State Save/Reload/Preset、Tempo/Loop/MIDI 入力、Crash/Hang/Timeout/Isolation — 後続 plugin class の責務（本 slice では `kNotImplemented` を正直に返す）
- `crates/ruviform-plugin-vst3` 本体の変更（dev-dependency edge のみ許可）

## Requirements

- 実 `.vst3`/`.dll` ファイルを `Vst3Library::load` → `ErasedPlugin` init→prepare→process→teardown の本番 host path で Discover/Instantiate/Process/Uninstantiate できること（Fake Plugin Port 非経由）
- 既知 PCM 入力・frame 数・automation イベントに対し、独立に計算した golden 出力と sample 単位で一致すること（frame offset・off-by-one 境界を含む）
- RT process 経路で allocation/blocking/I/O を行わないこと（`assert_no_alloc` で機械検証）
- adapter fixture と同一 host scan 内で共存・区別できること（別 CID・別 vendor/name）
- test-only であることを README/lib docs に明記し、production crate が本 crate に依存しないこと
- `scarecrow173/ruviform#628` runtime への依存 edge を持たないこと

## Acceptance Criteria

- [ ] VST3: 自 crate の cdylib を本番 host path（`Vst3Library::load` → `factories()` → `create()` → `init`/`prepare`/`process_host`/teardown）で Discover/Instantiate/Process/Uninstantiate — L0 in-process で実施（CLAP・L1 は後続 slice = NotRun）
- [ ] 既知 PCM/Frame/Automation: fixed golden vectors + event frame offset の sample-accurate 適用を検証（Tempo/Loop/MIDI/State/Crash-Hang-Timeout-Isolation は後続 = NotRun）
- [ ] RT non-blocking: `rt_` prefix `assert_no_alloc` テストで process 経路の allocation 禁止を機械検証
- [ ] 独立実装 cross-check を文書化（plugin 側実装 ≠ adapter fixture 実装。共有する `vst3` binding 層は SDK 相当の共通層として記録）。外部 SDK validator は NotRun/Deferred
- [ ] test-only crate 宣言を README/lib docs に記載

## Deferred Design Concerns

- `test_instrument`/`test_stateful`/`test_fault`/`test_editor` plugin class | Issue #630 の残受入は本 slice の範囲外 | 後続 slice の dispatch | なし（本 slice は gain のみで受入可能）
- CLAP test plugin crate | format 別クレート分割は design doc で pin 済み | `-clap` crate の後続 slice | なし
- `scarecrow173/ruviform#628` runtime 統合 | 未 merge のため依存 edge を持てない | #628 merge 後の統合 slice | plugin が runtime 非依存のまま残る（意図どおり）
- 外部 VST3 SDK validator（Steinberg validator 等） | License・実行環境の確保が必要 | 別途 validator 導入 slice | adapter 自己検証のみでは外部 ABI 適合を主張できない（文書で明記済み）
- L1 out-of-process / Native Editor / consumer oracle | 本 slice は L0 in-process のみ | #631 scenario runner・consumer 側作業 | in-process Pass を L1/実機 Pass と誤認しないよう証拠に NotRun を明記

## Sources and Relations

- Issue `scarecrow173/ruviform#630`（本 spec の source）
- `docs/specs/issue-627-virtual-first-verification-contract.md` — `environment_kind`/`test-plugin` 環境種別・Case Schema v1・Test Depth・「実 Plugin ABI と Mock を分離」の契約正本
- `POLICIES.md` §2.1（2026-11-15 仮想優先改訂）— OS-L1/GUI-L2/EXT-L3、Pass/Fail/NotRun/Unsupported の証拠規範
- `crates/ruviform-plugin-vst3/` — 本番 host bridge（`src/host.rs`）と adapter fixture の先例（変更しない）
- `crates/ruviform-plugin-core` / `crates/ruviform-audio-core` — neutral 契約（`PluginProcessor`/`Event`/`ParamInfo`/`PlanarBufferView`）
