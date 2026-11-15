---
id: "DESIGN-034XJpQL8GJGCAfS6f2zQK"
type: "design"
status: "approved"
title: "Issue #193 Migration M0〜M7 受入表 Virtual-first Owner 対応付け設計"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "docs/specs/issue-193-virtual-first-owner-remap.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-193-virtual-first-owner-remap.md"
      - type: "doc"
        path: "docs/designs/issue-193-virtual-first-owner-remap-design.md"
      - type: "doc"
        path: "docs/test-specs/issue-193-virtual-first-owner-remap.md"
      - type: "doc"
        path: "docs/plans/issue-193-virtual-first-owner-remap-plan.md"
      - type: "doc"
        path: "docs/tasks/issue-193-virtual-first-owner-remap-task.md"
      - type: "doc"
        path: "docs/impl/ir/issue-193-virtual-first-owner-remap.md"
      - type: "doc"
        path: "docs/impl/exp/issue-193-virtual-first-owner-remap.md"
    modified:
      - type: "doc"
        path: "docs/designs/issue-193-migration-execution-plan.md"
      - type: "doc"
        path: "docs/plans/issue-193-migration-execution-plan.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-193-virtual-first-owner-remap.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-193-virtual-first-owner-remap.md"
    - "docs/designs/issue-193-migration-execution-plan.md"
    - "docs/plans/issue-193-migration-execution-plan.md"
    - "POLICIES.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-275-observability-integration-design.md"
    - "docs/designs/issue-275-obs-owner-map-design.md"
    - "docs/designs/issue-627-virtual-first-verification-contract-design.md"
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-193-virtual-first-owner-remap.md"
    - "docs/designs/issue-193-migration-execution-plan.md"
    - "docs/plans/issue-193-migration-execution-plan.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-193-virtual-first-owner-remap.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/251"
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/532"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/synvelis/issues/28"
    - "POLICIES.md"
  defers: []
  deferred-by: []
---

# Issue #193 Migration M0〜M7 受入表 Virtual-first Owner 対応付け設計

## Context

`docs/designs/issue-193-migration-execution-plan.md` と
`docs/plans/issue-193-migration-execution-plan.md` は
Issue #193 の Migration Execution Plan の設計・計画
canonical であり、両者の `## Snapshot-2026-11-14` 節は
「同名 Snapshot 節（内容は一致させる）」の約束で同期
されている（実測: 両節の差異は designs 側が設計根拠
リンク 1 行、plans 側が設計正本側 2 文書への言及を
含む版識別ヘッダのみで、本文は一致）。

Issue #193 本文の 2026-11-15「Virtual-first正式受入
Owner整理」は、M0〜M7 受入表に物理機材 Gate ではなく
3OS OS-L1 と Windows 使用 GUI（#631）・Audio/MIDI
実 OS 仮想 Endpoint（#629）・実 Plugin ABI（#630）を
それぞれの Owner へ紐付けることを現行方針として
要求する。一方、`Snapshot-2026-11-14` 節の M0〜M7
受入表・`### Deferred/Activation 記録` 表・M3/M4/
M5/M7 Integration Gate 行・`### DAG と Owner の表` は
実装/実機 owner（RF #251/#253/#388/#389/#532）のみを
指したままで、#628〜631 Harness への受入 owner
対応付けが未記録である。

正本方針は `POLICIES.md` §2.1（2026-11-15 決定
「仮想優先の受入と共通テスト基盤」）であり、同型の
remap は Issue #275 OBS 設計正本の 2026-11-15 追記
（Deferred 範囲の Owner/Trigger・Consumer Gate 割当）
として `92e40aea` で merge 済み。本設計は「どこに
何を書くか」と「節の構造・語彙規則」だけを定める。

## Scope

### In Scope

- 両 canonical ファイルへの同一内容の新規 dated 節
  `## Snapshot-2026-11-15 — Virtual-first owner
  対応付け（版付き）` の追記（`Snapshot-2026-11-14`
  節の直後、designs は `## Deferred Design Concerns`
  の前、plans は `## Implementation Handoff` の前）。
- 新規節の構造: 版識別・状態語彙・Issue 状態照合
  （2026-11-15 実測）・Virtual-first 受入 owner
  対応表・旧 owner 記述との関係（dated 上書き宣言）・
  維持区分・Verification Notes。
- `issue-193-virtual-first-owner-remap` 文書チェーン。
- 参照 Issue の `gh issue view` 実測照合。
- 触った dir の生成 index の再生成（`doc-maintenance`）。

### Out of Scope

- コード・CI・manifest・`POLICIES.md`・`AGENTS.md` の
  変更。
- `Snapshot-2026-11-14` 節を含む既存本文の書き換え・
  削除（append-only。dated 節の追加で更新を表現）。
- `evidence/` 配下の新規作成（lease 外。検証記録は
  doc chain に残す）。
- RF #628〜631 の実装・Case 実走、GitHub blockedBy
  の登録・削除。

## Components and Boundaries

### 追加する節（両 canonical 内・完全一致）

両ファイルに**バイト一致**で挿入する。相対 markdown
リンクは使わず、Issue 参照は GitHub URL、文書参照は
repo ルート相対パスの inline code とし、ファイル間で
パス解決が変わらない形にする。

#### 節ヘッダと版識別

`## Snapshot-2026-11-15 — Virtual-first owner 対応付け
（版付き）`。版識別子 `Snapshot-2026-11-15`、本節が
受入 owner 対応付けの記録であり M0〜M7/G0〜G7/RB0〜
RB6・GUI lane・Cross-Consumer 受入の定義を変更しない
こと、既存本文を履歴 snapshot として保持し本節を
2026-11-15 以降の受入 owner 正本とすることを明記。
方針正本 `POLICIES.md` §2.1 と precedent
（`docs/designs/issue-275-observability-integration-
design.md` の 2026-11-15 追記）を参照する。

#### `### Issue 状態照合の結果（2026-11-15 実測）`

`gh issue view` 実測値を snapshot として記録する:

- RF #627 CLOSED（POLICIES §2.1 の契約正本 — 詳細
  設計と #628〜631 実装を取り違えない旨を明記）
- RF #628/#629/#630/#631 OPEN（未実装 Harness。
  RF-VTEST-01〜04 の役割を 1 行ずつ記す）
- RF #251/#253/#275/#294/#388/#389/#532 OPEN
  （実装・mechanism・製品 owner として残存）
- AL #127・SV #28 OPEN（Consumer Oracle）

#### `### Virtual-first 受入 owner 対応表`

per-milestone の対応表。列は「Milestone/領域・
OS-L1・GUI-L2/EXT-L3 受入 owner（2026-11-15〜）・
残存 owner」の 4 列とし、Snapshot-2026-11-14 の
M0〜M7 表の各 Integration Gate と Deferred/
Activation 表の Required 行を網羅する。対応付けの
語彙規則:

- **OS-L1**: 変更なし。3OS Native Build/Link/
  Headless/RT/CLI・必要 Feature・Consumer 独立
  Oracle は既存 Daily/手動 CI と #294 tuple・
  AL #127/SV #28 が担う（Snapshot-2026-11-14 の
  記述を引き継ぐ）。
- **GUI-L2（Windows 使用 GUI）**: 受入 owner =
  RF #631（GPUI 実 Window Scenario Runner）。
- **EXT-L3 Audio/MIDI（実 OS 仮想 Endpoint・本番
  API 通過）**: 受入 owner = RF #629。
- **EXT-L3 実 Plugin ABI（VST3/CLAP Test Plugin・
  Native Editor）**: 受入 owner = RF #630。Native
  Dock の GUI-L2 側は RF #631 と併記。
- **共通基盤**: RF #628（deterministic test
  runtime — VirtualClock/PCM/Fault）を全 Harness
  Case の土台として併記。
- **残存 owner**: RF #251/#253/#388/#389/#532 は
  実装・EXT-L3 mechanism owner、SV #115/#117/
  #126/#127/#176/#177・AL #127/#189 は製品 Owner
  として残り、削除・格下げしない。

per-milestone の対応（Snapshot-2026-11-14 の各
Integration Gate との対応）:

| Milestone/領域 | OS-L1 | GUI-L2/EXT-L3 受入 owner | 残存 owner |
| --- | --- | --- | --- |
| M0 Baseline | #219/#294 tuple・AL #127/SV #28 | なし（L2/L3 対象外） | — |
| M1 Graph | SV #106・#294 | なし | — |
| M2 Audio Runtime | SV #106/#109・SV #28/AL #127・#294（RT budget は L1 CI） | なし | — |
| M3 Time/Event | #253 の OS 非依存部分は headless 継続 | EXT-L3（OS MIDI API・VST3 翻訳）: RF #629 + RF #630 | RF #253（mechanism）、SV #126/#127（製品） |
| M4 Plugin | plugin host L1 継続 | GUI-L2/EXT-L3（Native Dock・実 ABI）: RF #631 + RF #630 | RF #388/#389/#532（mechanism）、SV #176/#127（製品） |
| M5 Device/Reader | Mock Input headless 継続 | EXT-L3（Audio Input Device）: RF #629 | RF #251（real backend）、SV #115/#117（製品） |
| M6 Output/Offline | #306 合成 Gate | なし（OS/FS は OS-L1 範囲） | — |
| M7 UI/Probe・GUI lane | headless/fake 継続 | GUI-L2: RF #631、Native Dock EXT-L3: RF #630 + RF #631 | RF #262/#388/#389/#419/#465/#532（採否・mechanism）、SV #112/#114/#176/#177・AL #127/#189（製品） |

#### 旧 owner 記述との関係（dated 上書き宣言）

`Snapshot-2026-11-14` 節の `### Deferred/Activation
記録（L2/L3 が必要な製品利用条件）` 表・M3/M4/M5
Integration Gate 行・M7 Integration Gate/責務行・
GUI lane・`### DAG と Owner の表` が指す RF
#251/#253/#388/#389/#532 を名指しで列挙し、これらが
実装・EXT-L3 mechanism owner および製品 Owner と
して**引き続き有効**（削除・格下げしない）である
こと、2026-11-15 以降の**受入 owner 正本は本節**
であることを宣言する。issue-275 design doc の
2026-11-15 追記と同型の precedent に従う。

#### 維持区分

- Windows で提供・利用する機能の GUI-L2/EXT-L3 は
  Required のまま維持（Required→Deferred の格下げ
  禁止）。macOS/Linux L2/L3 の Deferred 既存分類と
  未実走 Case の `NotRun` 正直記録を維持。
- 既存実装/CI に新たな `Pass` を付与しない。未実装
  の仮想テスト（#628〜631 の Harness Case）は
  `NotRun`、DComp 既知 Fail（RF #532 関連）は維持。
- Headless 実装 Slice の着手を L2/L3 `NotRun` で
  止めず、製品提供の最終受入では `Fail`/`NotRun`
  を維持する区分を明記。
- 物理機材・市販 Plugin の一括 Required 化は課さない
  （POLICIES §2.1）。個別機材の正式サポート宣言・
  不具合調査時のみ物理 Case を定義する。

#### `### Verification Notes（Snapshot 節の検証）`

本節の検証項目を checkbox で列挙する（完全一致・
Issue 実測照合・非昇格・append-only・docs-only）。

## Data and Control Flow

文書のみの変更であり、データフローは存在しない。
記述の流れは:

1. `gh issue view` で参照 Issue の状態を実測し、
   新規節の「Issue 状態照合の結果」に snapshot と
   して固定する。
2. `Snapshot-2026-11-14` 節の M0〜M7 受入表・
   Deferred/Activation 表・Integration Gate 行・
   DAG 表を読み、Virtual-first 受入 owner 対応表へ
   per-milestone で写像する。
3. 同一内容の節を両 canonical に挿入し、抽出 diff
   で完全一致を確認する。
4. `issue-193-virtual-first-owner-remap` 文書チェーン
   が derives-from/implements 関係を記録し、
   test-spec が検証規則を固定する。

## Non-Functional Constraints

- **完全一致**: 両 canonical の新規節はバイト一致。
  相対リンクを使わず GitHub URL / repo 相対パスで
  記述し、ファイル間でリンク解決が変わらない形に
  する。
- **append-only**: `Snapshot-2026-11-14` 節を含む
  既存本文に差分を作らない。挿入位置は designs
  `## Deferred Design Concerns` 直前、plans
  `## Implementation Handoff` 直前。
- **正確性**: Issue 番号・役割・open/closed 状態は
  全て `gh issue view` 実測（2026-11-15）に基づく。
  存在しない Issue/文書を根拠にしない。
- **非昇格**: `NotRun`/`Fail`/`Unsupported` を
  `Pass` に置き換えない。Required→Deferred の
  格下げをしない。job/lane 定義の存在から Pass を
  推論しない。
- **役割分離**: #627（方針基盤・CLOSED）と #628〜631
  （実装 Harness・OPEN）、#251/#253/#388/#389/#532
  （実装・mechanism owner）と #628〜631（受入
  owner）、RF 側 Harness と AL #127/SV #28（製品
  Oracle）の分界を混同しない。
- **スコープ**: `docs/` のみ。`crates/`・manifest・
  workflow・`mise.toml`・`evidence/` に差分を作ら
  ない。

## Risks and Trade-offs

- **Issue 状態の陳腐化**: #628〜631・#251/#253/
  #388/#389/#532・AL #127/SV #28 の状態は変動する。
  対策: 実測日付（2026-11-15）を明記し、将来の状態
  変化は次回 dated Snapshot で再照合する旨を記す。
- **旧 owner 記述との二重正本化**: 旧 Snapshot 節の
  Owner 列と新節の受入 owner が併存し読み手が混同
  する余地。対策: dated 上書き宣言で「2026-11-15
  以降の受入 owner 正本は本節」と明示し、旧 owner
  は mechanism/製品 Owner として残存すると役割を
  分離する。
- **対応表の肥大化**: M0〜M7 全行に詳細を書くと
  Snapshot-2026-11-14 節と drift する。対策: L2/L3
  対象のある Milestone のみ owner を記し、OS-L1
  のみの Milestone は「変更なし」と要約する。
- **Harness 未実装の誤読**: #628〜631 は全て OPEN で
  受入 Case は未実走。対策: `NotRun` を明記し、
  対応表が「実行済み」を意味しないことを状態語彙
  と併記する。

## Deferred Design Concerns

- RF #628〜631 Harness の実装・Case 実走 | 各 Issue
  が Owner であり本 slice は対応付けのみ | #628〜631
  の実装 Issue 群の実行 | 対応表が「実行済み」を
  意味すると誤読されるリスク（`NotRun` 明記で緩和）。
- macOS/Linux の GUI-L2/EXT-L3 | 利用要件発生まで
  Deferred（POLICIES） | 各利用機能の Owner Issue・
  具体的利用決定 | 実機条件下の問題の顕在化遅延。
- 物理機材・市販 Plugin の個別適合 Case | 原則必須
  Gate から外す（POLICIES §2.1） | 正式サポート宣言・
  個別不具合調査時に物理 Case を定義 | 仮想 Pass が
  物理機材を保証しない境界の見落とし。
- Issue #193 本文への今後の追記の docs への同期 |
  本書は 2026-11-15 snapshot | #193 本文の更新検知時
  に次回 dated Snapshot で再照合 | 計画正本の陳腐化
  （Snapshot-2026-11-14 節の Deferred Design
  Concerns と同じ継続課題）。

## Verification Notes

- [ ] 両 canonical の新規節が抽出 diff で完全一致
      することを確認する。
- [ ] 新規節の Issue 参照（#627/#628/#629/#630/#631/
      #294/#251/#253/#388/#389/#532・AL #127・SV #28）
      が `gh issue view` 実測の番号・状態と一致する
      ことを確認する。
- [ ] `NotRun`→`Pass` 置換・Required→Deferred 格下
      げ・既存本文への差分がないことを diff で確認
      する。
- [ ] `mise run fmt-check`・`mise exec -- prek run
      --all-files`・`doc_status` lint/audit・index
      再生成の整合を確認する。
