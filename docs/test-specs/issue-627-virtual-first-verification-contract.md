---
id: "TSPEC-034XAEUQyTZ4N0vdNcRAlI"
type: "test-spec"
status: "approved"
title: "Issue #627 仮想優先検証契約のテスト仕様"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/627"
  changes:
    added:
      - type: "doc"
        path: "docs/test-specs/issue-627-virtual-first-verification-contract.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-627-virtual-first-verification-contract-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-627-virtual-first-verification-contract-plan.md"
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-627-virtual-first-verification-contract-plan.md"
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "docs/designs/issue-627-policies-revision-proposal.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "docs/designs/issue-627-policies-revision-proposal.md"
  verified-by: []
  references:
    - "evidence/rf627/policies-baseline-comparison.md"
    - "evidence/rf627/verification.md"
  defers: []
  deferred-by: []
---

# Issue #627 仮想優先検証契約のテスト仕様

## Purpose

Issue #627 の検証契約（`docs/designs/issue-627-virtual-first-verification-contract-design.md`）と POLICIES 照合記録（`docs/designs/issue-627-policies-revision-proposal.md`）が、三 repo の受入に必要な性質を実際に満たすことを検証可能にする。本 spec は docs-only slice であるため「テスト」は文書・証拠の照合手順を指し、#628〜#631 実装後は各実装が本 spec の Rule/Example を Case として具現化する。契約が supersede されるか #628〜#631 が完了するまで有効。

## Feature

仮想優先の3repo検証契約 — OS-L1/GUI-L2/EXT-L3 の各レベルで、`environment_kind` により環境の実体を区別し、要求状態と実行結果を直交させ、独立 Oracle と証拠で Case の Pass を判定する横断契約。

## Rules

- **R1（要求状態×実行結果の直交）**: 各 Case は `requirement ∈ {Required, Deferred, NotApplicable}` と `result ∈ {Pass, Fail, NotRun, Unsupported}` を別フィールドで持つ。`Required` かつ `NotRun`/`Fail` は未受入であり、`Unsupported` は技術的非対応が文書化された場合に限る。
- **R2（必須記録項目）**: 全 Case は `case_id`・`feature`・`level`・`environment_kind`・`os`/`arch`・`repo`/`sha`・`resolved_ruviform_sha`・`oracle`・`expected`/`observed`・`evidence`・`fixture_version` を記録する。欠落した Case は Gate 判定に使えない。
- **R3（環境種別の識別）**: `environment_kind` は `deterministic-backend` / `virtual-os-endpoint` / `test-plugin` / `real-window` / `physical-device` のいずれかで、Case の証拠がどの環境実体で得られたかを一意に示す。
- **R4（境界A: 本番OS-API通過）**: `virtual-os-endpoint` の Pass は、製品が実際に本番 WASAPI/WinMM 等の API を通して Endpoint を open/stream した記録を要求する。ユーザ空間への直接注入・列挙のみの記録は無効。
- **R5（境界B: 実ABIバイナリ）**: `test-plugin` の Pass は、正規 VST3/CLAP ABI を実装した実バイナリを本番 Host の探索/Load/Process/State/Unload 経路で実行した記録を要求する。mock/fake は代用にならない。
- **R6（境界C: 実画素Native Dock）**: `real-window` の Pass は、実子 HWND/native editor を実 Window に dock し、実画素・実入力・フォーカス・リサイズを観測した記録を要求する。headless/offscreen/generic/floating は代用にならない。
- **R7（独立Oracle）**: `expected` は被検証系と独立した実装・データ・解析器が生成する。AudioLens は f64/長尺/PTS/missing-data/CLI/visualization、Synvelis は PCM/RT/Project/録音・MIDI/plugin state/native dock をそれぞれ独立 Oracle で検証する。
- **R8（Pass非転移）**: ある repo の fixture/Runtime/Oracle の Pass を別 repo の Case の Pass 根拠にしない。三 repo はそれぞれ自分の `sha` と `resolved_ruviform_sha` で評価する。
- **R9（証拠なき緑は無効）**: CI job の定義・green のみでは Case の Pass にならない。`evidence` に RunID/URL と Oracle 比較結果が必要。
- **R10（仮想≠物理）**: `virtual-os-endpoint`/`test-plugin` の Pass は物理機材 Driver・ADC/DAC・市販 Plugin 固有互換・直結モニタリングを要求する Case の Pass ではない。それらは `physical-device` の別 Case であり初回受入の必須条件ではない。
- **R11（旧物理Requiredの扱い）**: 仮想 Case の Pass は旧物理 Required Case の `requirement`/`result` を自動書き換えしない。状態遷移は各 repo のレビューで行い、二重計上しない。
- **R12（POLICIES非変更の証明）**: 本 slice の差分に `POLICIES.md`・`docs/development-environment.md`・`mise.toml`・`.github/workflows/ci.yml`・`ci/summary/`・`crates/` を含まない。
- **R13（OS別深度の維持）**: OS-L1 は三 OS Required、Windows の GUI-L2/EXT-L3 は利用機能に限り Required、macOS/Linux の GUI-L2/EXT-L3 は Deferred を維持する。

## Examples

- **E1（R1/R9）**: Given `requirement=Required` の EXT-L3 Case（仮想 WASAPI Endpoint 経由の録音）が CI で green、When Case 記録に `evidence`（RunID）と `oracle`（独立 PCM 解析）の比較結果がない、Then その Case は `NotRun` であり `Pass` ではない。
- **E2（R4）**: Given Synvelis が仮想 Endpoint を列挙して format を返す、When 製品の audio graph がその Endpoint を source として open/stream していない、Then `environment_kind=virtual-os-endpoint` の証拠として無効であり `deterministic-backend` へ格下げまたは `NotRun`。
- **E3（R5）**: Given Test Plugin が host 内部オブジェクトとして振る舞いを再現するだけで実バイナリ load がない、When `environment_kind=test-plugin` の Pass を主張する、Then 無効。正規 ABI の実バイナリ + 独立 ABI Validator が必要。
- **E4（R6）**: Given plugin editor が offscreen buffer に描画され実 Window に dock されない、When `real-window` の Pass を主張する、Then 無効。実子 HWND + 実画素/入力/フォーカス証拠が必要。
- **E5（R7/R8）**: Given AudioLens が f64/長尺 fixture で Pass、When Synvelis の PCM/RT Case の Pass 根拠に流用しようとする、Then 禁止。Synvelis は自分の `sha`/`resolved_ruviform_sha`/Oracle で独立に評価する。
- **E6（R10/R11）**: Given RF #251 の旧物理 Required（実機 WASAPI record）が NotRun、When 対応する仮想 Case が `environment_kind=virtual-os-endpoint` で Pass、Then 旧 Case は自動的に Pass にならず、移管台帳の「物理維持/opt-in」ルールに従い別管理される。
- **E7（R12）**: Given 本 slice の `git diff`、When `POLICIES.md`・`mise.toml`・`crates/` 等の範囲外ファイルに差分がある、Then R12 違反として slice は未完了。
- **E8（R13）**: Given macOS/Linux で GUI-L2/EXT-L3 の Case、When `requirement=Required` として計上しようとする、Then Deferred 方針に反するため不可（利用要件発生まで Deferred）。

## Guarantees

- 三 repo の Case 記録が同一 schema で比較可能であり、`environment_kind` により「何を証明し何を証明しないか」が一意に読める。
- 仮想環境の Pass が物理適合・市販 Plugin 互換・旧物理 Required の Pass として誤読される経路が構造的に閉じている。
- 独立 Oracle により、production 実装の自己採点による false green を防げる。
- #628〜#631 の実装が本契約の `environment_kind`・Case schema・境界規則に従って進められる。

## Non-goals

- #628〜#631 の実装物（Test Runtime・仮想 Endpoint・Test Plugin・GUI Runner）の動作検証 — それらは各 Issue の test-spec が担う。
- 物理機材・市販 Plugin の実適合検証 — `physical-device` opt-in Case の範囲。
- `POLICIES.md` 本文の内容検証 — 本 slice は非変更の証明（R12）のみ。
- CI workflow の実行可能性 — docs-only slice のため `prek run --all-files` の衛生チェックのみ。

## Risk

- 本 spec の Rule が #628〜#631 実装で形骸化するリスク → 各実装 Issue の doc-chain で本 spec を `verifies` 関係に含め、Case schema の必須項目を証拠契約として維持する。
- `environment_kind` の値が実装側で独自拡張されるリスク → schema の値集合を閉じた列挙として固定し、追加は契約改訂として扱う。
- 移管台帳が「提案」に留まり旧 Case の実状態が更新されないリスク → 各 repo の Issue/PR で状態遷移を明示的に行う運用を前提とし、本 slice では台帳の確定までを担う。
