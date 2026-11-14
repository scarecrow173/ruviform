---
id: "SPEC-034XAreKCCKojmpwC7OvJR"
type: "spec"
status: "approved"
title: "Issue #627 仮想優先の3repo検証契約・物理Gate置換・POLICIES整合と独立Oracle"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "https://github.com/scarecrow173/ruviform/issues/251"
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/532"
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "https://github.com/scarecrow173/synvelis/issues/23"
    - "https://github.com/scarecrow173/synvelis/issues/114"
    - "https://github.com/scarecrow173/synvelis/issues/115"
    - "https://github.com/scarecrow173/synvelis/issues/117"
    - "https://github.com/scarecrow173/synvelis/issues/126"
    - "https://github.com/scarecrow173/synvelis/issues/127"
    - "https://github.com/scarecrow173/synvelis/issues/128"
    - "https://github.com/scarecrow173/synvelis/issues/176"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-627-virtual-first-verification-contract.md"
      - type: "doc"
        path: "docs/designs/issue-627-virtual-first-verification-contract-design.md"
      - type: "doc"
        path: "docs/designs/issue-627-policies-revision-proposal.md"
      - type: "doc"
        path: "docs/test-specs/issue-627-virtual-first-verification-contract.md"
      - type: "doc"
        path: "docs/plans/issue-627-virtual-first-verification-contract-plan.md"
      - type: "doc"
        path: "docs/tasks/issue-627-virtual-first-verification-contract.md"
      - type: "doc"
        path: "docs/impl/ir/issue-627-virtual-first-verification-contract.md"
      - type: "doc"
        path: "evidence/rf627/policies-baseline-comparison.md"
      - type: "doc"
        path: "evidence/rf627/verification.md"
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
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "docs/designs/issue-627-policies-revision-proposal.md"
    - "docs/plans/issue-627-virtual-first-verification-contract-plan.md"
  depends-on:
    - "docs/specs/issue-219-consumer-compat-contract.md"
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "docs/designs/issue-627-policies-revision-proposal.md"
    - "docs/test-specs/issue-627-virtual-first-verification-contract.md"
    - "POLICIES.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/627"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-627-virtual-first-verification-contract.md"
  references:
    - "evidence/rf627/policies-baseline-comparison.md"
    - "evidence/rf627/verification.md"
    - "https://github.com/scarecrow173/ruviform/blob/main/POLICIES.md"
    - "https://github.com/scarecrow173/synvelis/blob/main/POLICIES.md"
    - "https://github.com/scarecrow173/audio-lens/blob/main/POLICIES.md"
  defers: []
  deferred-by: []
---

# Issue #627 仮想優先の3repo検証契約・物理Gate置換・POLICIES整合と独立Oracle

## Problem

Issue #627 は、三リポジトリ（Ruviform / Synvelis / AudioLens）の受入を、初回から物理 Audio/MIDI 機材・市販 Plugin を前提としない「仮想優先」検証へ移行するための横断検証契約を定義する Issue である。

**前提の更新（2026-11-15 判明）**: Issue 起票時点では三 repo の `POLICIES.md` は 2026-11-13 版（Windows の利用機能に実物理機材・実 Plugin の EXT-L3 を要求し、「Mock Plugin は実 Plugin 動作の代用にならない」と規定）であり、本 Issue は改訂「案」をレビュー可能にすることを目的としていた。しかし作業中に、三 repo すべてで仮想優先改訂が **既に main へ merge 済み** であることを確認した:

- ruviform: PR #634（commit `3b8b7322`、main `origin/main` HEAD）
- synvelis: PR #449（commit `8aa5cb0`）
- audio-lens: PR #212（commit `eb62440`）

三 repo の改訂は共通節（§1〜§5）で同一内容（各 +21/-10）であり、§2.1「仮想優先の受入と共通テスト基盤（2026-11-15決定）」を新設し、EXT-L3 の定義・OS 別表・§3 証拠項目・§5 末尾・§6 を仮想優先へ整合させている。差異は repo 名・§6 固有節・audio-lens の `\` 改行スタイルのみ。

したがって本スライスの責務は「未適用テキストへの改訂案」ではなく、**(a) 適用済み改訂の3 repo 行単位照合と #627 契約との適合確認、(b) Case schema・境界 Case・独立 Oracle・受入マトリクスを定義する検証契約の確定、(c) 既存 Issue のゲート移管台帳、(d) 残差分があればその改訂メモ**である。`POLICIES.md` 本文は本スライスでも変更しない。

残るリスクは、(a) 改訂済みポリシーが参照する「環境種別」「独立Oracle」「証拠」項目を満たす正式な Case schema が未定義であり Issue ごとの記録粒度がばらつくこと、(b) 旧物理 Required で NotRun だった Case を仮想 Case 成功でどう扱うかの移管規則が未確定なこと、(c) 仮想 Endpoint/Test Plugin の Pass が物理機材・市販 Plugin の適合を保証しない境界を Case 単位で反証可能にする記述がないこと、の3点である。

## Why Now

- RF-VTEST シリーズ（#628 Test Runtime → #629 仮想 OS Endpoint → #630 Test Plugin → #631 GPUI Scenario Runner）の実装が #627 の契約を前提として開始可能な状態にある必要がある。
- 三 repo の `POLICIES.md` 仮想優先改訂は merge 済みだが、改訂本文が参照する「環境種別」「独立 Oracle」「証拠」の記録形式を満たす Case schema は未定義のままである。Issue #627 がこの契約を確定しないまま #628〜#631 の実装が進むと、証拠の記録粒度と Gate 判定が実装ごとにばらつく。
- 旧物理 Required で NotRun/未達だった Case（RF #251/#253 の実機録音・実MIDI、#388/#389/#532 の実Plugin Dock、SV #115/#117/#126/#127/#128/#176 の製品受入、AL #127 の継続追従）について、仮想環境での置き換え先と「仮想 Pass を物理適合の Pass と読み替えない」境界を台帳として固定する必要がある。
- 物理機材・市販 Plugin を初回必須としない方針は、開発者が機材を持たない環境でも 3 repo の受入を前進させられるようにするための合意済みの設計意図であり、契約として固定しないと各 Consumer が独自解釈で Gate を運用する危険がある。

## Audience

- **Coordinator / Issue Owner** — #627 の契約・照合記録・移管台帳のレビューと、残件（Case schema の運用・旧物理 Required の状態遷移）の判断材料
- **RF-VTEST シリーズ（#628〜#631）の実装担当** — Test Runtime・仮想 Endpoint・Test Plugin・GUI Runner が満たすべき Case schema・Oracle・Evidence 契約の正本
- **Synvelis / AudioLens の受入 Owner** — 製品固有 Oracle（SV: PCM/RT/Project/Plugin/GUI、AL: f64/長尺/CLI）と Ruviform 共通 Fixture の責務分担、独立受入マトリクスの参照先
- **既存 Gate Owner（RF #251/#253/#388/#389/#532/#294、SV 各 Issue、AL #127）** — 旧物理 Required の移管先と、仮想 Case 成功を物理環境依存の受入条件の Pass として計上してはならないルールの確認先

## Scope

### In Scope

- 三 repo の現行 `POLICIES.md`（2026-11-15 仮想優先改訂適用後）の行単位比較と、適用済み変更箇所・非変更箇所・repo 間残差分を整理した照合記録の確定（`POLICIES.md` 本文は変更しない）
- `Required`/`Deferred`/`NotApplicable`（要求状態）と `Pass`/`Fail`/`NotRun`/`Unsupported`（実行結果）を直交させた検証 Case schema の定義。`environment_kind`、OS/Arch/Device/Plugin/SDK、Repo/SHA と resolved Ruviform SHA、Feature、独立 Oracle、Evidence URL/RunID を必須項目として含める
- Gate taxonomy の拡張: OS-L1 / GUI-L2 / EXT-L3 を維持しつつ、EXT-L3 の「仮想適合性」サブケースを `environment_kind` で区別する規則（適用済み §2.1 の「環境種別」記述を形式化）
- 反証可能な境界 Case の記述: Device 仮想化と本番 OS-API 通過の境界、Plugin Fake と実バイナリ ABI の境界、Native Dock 実画素と Fake Surface の境界
- 独立 Oracle の規則: production と同じ実装が自分自身の正解値を生成しないこと（既知数学波形・固定 MIDI event・外部 ABI Validator・独立 PCM 解析）
- AudioLens（f64/長尺/CLI）と Synvelis（PCM/RT/Project/Plugin/GUI）の独立検証を両立させ、一方の改善が他方を暗黙に劣化させない受入マトリクス
- Ruviform Test Harness の Dogfooding 責務分担（#628〜#631 が提供する共通部品と、各 Consumer が所有する製品 Oracle）
- 既存 Issue のゲート移管表と旧物理 Required の扱いの記録（RF #251/#253/#388/#389/#532/#294、SV #23/#114/#115/#117/#126/#127/#128/#176、AL #127）
- 明示ルール: 仮想 Case の成功を、それが代替しない物理環境依存の受入条件の Pass として扱ってはならない（移行期の旧 Required Case を含む）

### Out of Scope

- `POLICIES.md` 本文の変更（三 repo とも改訂適用済み。本スライスは照合・契約・台帳のみ）
- `docs/development-environment.md`、`mise.toml`、`.github/workflows/ci.yml`、`ci/summary/`、`crates/` の変更
- RF-VTEST-01〜04（#628 Test Runtime・#629 仮想 OS Endpoint・#630 Test Plugin・#631 GPUI Scenario Runner）の実装 — 本 Issue は契約のみを定義し、実装は後続 Issue の責務
- 物理機材・市販 Plugin の初回購入・常設義務、未実装 Backend を Pass 扱いすること、既存の実画素 DComp 問題（#532）の Close
- 3 repo の `POLICIES.md` への追加改訂（適合確認で残差分が見つかった場合は改訂メモとして記録し、適用は別 PR・別レビュー）
- macOS/Linux の GUI-L2/EXT-L3 の Required 昇格（利用要件発生まで Deferred を維持）

## Requirements

1. **POLICIES.md 行単位比較**: 三 repo の現行 `POLICIES.md`（2026-11-15 仮想優先改訂適用後）を行単位で比較し、物理機材/市販 Plugin、Windows の使用機能の L2/L3、Deferred/NotRun、3OS L1 の各変更箇所（2026-11-13 版からの差分）と非変更箇所、repo 間の残差分を整理した照合記録を確定すること。比較の根拠（使用したファイルパス・commit SHA・取得日）を証拠として記録すること。
2. **Case schema**: `Required`/`Deferred`/`NotApplicable` と `Pass`/`Fail`/`NotRun`/`Unsupported` を直交させた Case schema を定義し、少なくとも `environment_kind`、OS/Arch、Device、Plugin、SDK、Repo/SHA、resolved Ruviform SHA、Feature、独立 Oracle、Evidence URL/RunID を項目として持つこと。
3. **境界 Case**: Device 仮想化と本番 OS-API 通過の境界、Plugin Fake と実バイナリ ABI の境界、Native Dock 実画素と Fake Surface の境界を、それぞれ反証可能な（Fail になり得る）Case として記述すること。
4. **独立 Oracle**: AudioLens の f64/長尺/CLI、Synvelis の PCM/RT/Project/Plugin/GUI をそれぞれ独立した Oracle で検証し、一方の改善が他方を暗黙に劣化させない受入マトリクスを完成させること。
5. **ゲート移管表**: 既存 RF #251/#253/#388/#389/#532/#294、SV #23/#114/#115/#117/#126/#127/#128/#176、AL #127 の各 Issue について、旧物理 Required の移管先（RF-VTEST workstream または維持）と扱いを記録すること。
6. **仮想/物理境界ルール**: 仮想 Case の成功を、それが代替しない物理環境依存の受入条件（物理機材 Driver・ADC/DAC・市販 Plugin 固有互換等）の Pass として扱ってはならないこと、および移行期の旧 Required Case の扱いを明示ルールとして記述すること。
7. **POLICIES.md 非変更**: 本スライスでは `POLICIES.md` 本文・本番コード・CI 設定を一切変更しないこと。

## Acceptance Criteria

- [ ] AC1: 三 repo の現行 `POLICIES.md`（2026-11-15 仮想優先改訂適用後）の行単位比較結果と、適用済み変更箇所・非変更箇所・repo 間残差分を整理した照合記録が `docs/designs/issue-627-policies-revision-proposal.md` に確定している。
- [ ] AC2: `Required`/`Deferred`/`NotApplicable` と `Pass`/`Fail`/`NotRun`/`Unsupported` を直交させた Case schema が `docs/designs/issue-627-virtual-first-verification-contract-design.md` に定義され、`environment_kind`、OS/Arch/Device/Plugin/SDK、Repo/SHA・resolved Ruviform SHA、Feature、独立 Oracle、Evidence URL/RunID を含む。
- [ ] AC3: Device 仮想化 vs 本番 OS-API、Plugin Fake vs 実バイナリ ABI、Native Dock 実画素 vs Fake Surface の3境界が、それぞれ反証可能な Case として記述されている。
- [ ] AC4: AudioLens の f64/長尺/CLI と Synvelis の PCM/RT/Project/Plugin/GUI を独立検証する受入マトリクスが完成し、一方の改善が他方を暗黙に劣化させない構造になっている。
- [ ] AC5: 既存 RF #251/#253/#388/#389/#532/#294、SV #23/#114/#115/#117/#126/#127/#128/#176、AL #127 のゲート移管表と旧物理 Required の扱いが記録されている。
- [ ] AC6: 「仮想 Case の成功を、それが代替しない物理環境依存の受入条件の Pass として扱わない」明示ルールが設計文書に記述されている。
- [ ] AC7: `POLICIES.md`・`docs/development-environment.md`・`mise.toml`・`.github/workflows/ci.yml`・`ci/summary/`・`crates/` に変更がない（`git diff` で確認）。
- [ ] AC8: doc-chain（spec/design×2/test-spec/plan/task/IR）と管理 index が整合し、`mise exec -- prek run --all-files` が Pass する。

## Deferred Design Concerns

| 後送り項目 | 理由 | 追跡先 | 無視した場合のリスク |
| --- | --- | --- | --- |
| `POLICIES.md` 本文の追加改訂 | 三 repo の仮想優先改訂は適用済み。適合確認で残差分が見つかった場合のみ別 PR で改訂 | Issue #627（残差分の有無を記録） | ポリシーと契約 schema の間に未記録の意味論差異が残る |
| RF-VTEST-01〜04（#628〜#631）の実装 | 本 Issue は契約のみ。実装は後続 Issue の責務 | Issue #628/#629/#630/#631 | 契約未確定のまま実装が先行し、schema の再設計が必要になる |
| 物理機材/市販 Plugin の個別適合 Case | 初回必須から外すが、製品が当該機材を正式サポートする際に受入 Case を別途定義する | 各製品 Issue の Activation 条件 | 特定機材での問題が仮想環境に還元されず、サポート範囲の主張が曖昧になる |
| macOS/Linux GUI-L2/EXT-L3 の Required 昇格 | 利用要件発生まで Deferred を維持（現行方針と同じ） | 各 OS/機能の利用決定 Issue | macOS/Linux での実 GUI/実機器利用時に受入 Case が未定義のまま進む |

## Sources and Relations

- **Issue #627 本文・コメント**: 2026-11-15「横断Issue監査・物理実機ゲート置換台帳」のドラフト表を本 spec の設計で精緻化した。
- **現行 `POLICIES.md`**: 三 repo とも 2026-11-15 仮想優先改訂適用済み（RF `3b8b7322` / SV `8aa5cb0` / AL `eb62440`）。旧版（Effective: 2026-11-13）との行単位差分と比較根拠は `evidence/rf627/policies-baseline-comparison.md` に記録。
- **関連 spec**: `docs/specs/issue-219-consumer-compat-contract.md`（Consumer 互換契約）、`docs/specs/issue-294-cross-consumer-candidate-gate.md`（候補 tuple Gate・証拠契約）。
- **関連 Issue**: RF #251/#253/#388/#389/#532/#294、RF #628〜#631（RF-VTEST シリーズ）、SV #23/#114/#115/#117/#126/#127/#128/#176、AL #127 — いずれも 2026-11-15 付で「Virtual-first 移行（#627 ポリシー整合後に有効）」の暫定契約が追記済み。
