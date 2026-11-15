---
id: "DESIGN-SIWa8Wf4wBd9hAQ0iI6kUE"
type: "design"
status: "approved"
title: "Issue #192 Virtual-first 受入 Owner remap 設計"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "docs/specs/issue-192-virtual-first-owner-remap.md"
    - "docs/designs/issue-192-synvelis-driven-target-architecture-migration.md"
    - "docs/discovery/issue-192-ta-invariant-map.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-192-virtual-first-owner-remap.md"
      - type: "file"
        path: "docs/designs/issue-192-virtual-first-owner-remap-design.md"
      - type: "file"
        path: "docs/test-specs/issue-192-virtual-first-owner-remap.md"
      - type: "file"
        path: "docs/plans/issue-192-virtual-first-owner-remap-plan.md"
      - type: "file"
        path: "docs/tasks/issue-192-virtual-first-owner-remap-task.md"
      - type: "file"
        path: "docs/impl/ir/issue-192-virtual-first-owner-remap.md"
      - type: "file"
        path: "docs/impl/exp/issue-192-virtual-first-owner-remap.md"
    modified:
      - type: "file"
        path: "docs/designs/issue-192-synvelis-driven-target-architecture-migration.md"
      - type: "file"
        path: "docs/discovery/issue-192-ta-invariant-map.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements:
    - "docs/specs/issue-192-virtual-first-owner-remap.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-192-virtual-first-owner-remap.md"
    - "docs/designs/issue-192-synvelis-driven-target-architecture-migration.md"
    - "docs/discovery/issue-192-ta-invariant-map.md"
    - "docs/specs/issue-192-synvelis-driven-target-architecture-migration.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/designs/issue-275-observability-integration-design.md"
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-192-virtual-first-owner-remap.md"
    - "docs/specs/issue-192-synvelis-driven-target-architecture-migration.md"
  derived-by:
    - "docs/plans/issue-192-virtual-first-owner-remap-plan.md"
    - "docs/test-specs/issue-192-virtual-first-owner-remap.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-192-virtual-first-owner-remap.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/synvelis/issues/28"
  defers: []
  deferred-by: []
---

# Issue #192 Virtual-first 受入 Owner remap 設計

## Context

tracking canonical
`docs/designs/issue-192-synvelis-driven-target-architecture-migration.md`
（以下「tracking canonical」）は Issue #192 の受入管理面を
記録する approved 正本であり、対応表
`docs/discovery/issue-192-ta-invariant-map.md`（以下
「対応表」）は TA-001〜012 の invariant ↔ 現行 API/Issue/
L1 テスト対応と Headless Gap → Open Gate 接続を記録する
active 正本である。両者の Issue 状態記述は 2026-11-09 /
2026-11-14 の snapshot で固定されており、設計 DAG の
L2/L3 受入 Owner 欄は物理 Device・実機・実 Plugin・実 GUI
を前提とした旧来の割当のままである。

一方、Issue #192 本文「2026-11-15 Virtual-first正式受入
Owner整理」と `POLICIES.md` §2.1（2026-11-15 決定
「仮想優先の受入と共通テスト基盤」。方針基盤 Issue は
RF #627 CLOSED）で、Windows で実際に提供・利用する
GUI-L2/EXT-L3 は実 Window・本番 OS API を通る仮想
Endpoint・正規 ABI Test Plugin 等で受入し、物理機材・
市販 Plugin は通常必須ではない方針が確定した。設計 DAG
の L2/L3 受入 Owner 欄を RF #628〜631 共通 Harness の
本番契約検証へ委譲し、OS-L1/両 Consumer 独立検証を
#294・AL #127・SV #28 へ移管する決定である。

本設計は「どこに何を書くか」と「表の語彙・記述規則」
だけを定める。umbrella 設計正本
`docs/designs/synvelis-driven-target-architecture-and-migration.md`
の本文・TA gate 定義・領域別正本の詳細は変更しない。
表現スタイルの先例は
`docs/designs/issue-275-observability-integration-design.md`
§7 の Deferred Owner/Trigger 表および 2026-11-15 dated
note（merge commit `92e40aea` で main 入り済み）とする。

## Scope

### In Scope

- tracking canonical 末尾への dated addendum 追記
  （`## 2026-11-15 追記` 節。`updated` を当日日付へ更新）。
- 対応表「Headless Gap → Open Gate 接続一覧」節への
  dated addendum 追記（`updated` を当日日付へ更新）。
- `issue-192-virtual-first-owner-remap` 文書チェーン
  （spec/design/test-spec/plan/task/impl ir・exp）と
  触った dir の生成 index 再生成。

### Out of Scope

- コード・CI・manifest・ポリシー文書の変更。
- umbrella 設計正本・領域別正本・他 slug docs の本文
  変更（read-only 境界の維持）。
- #628〜631・#294・Consumer repo での実装・実走・受入、
  `evidence/` の作成、GitHub blockedBy の登録・削除、
  新規 Issue 起票。
- 既存 snapshot 表の書き換え・再構成（append-only）。

## Components and Boundaries

### 追加位置 1: tracking canonical 末尾

`## Verification Notes` 節の後（ファイル末尾）に
`## 2026-11-15 追記 — TA DAG L2/L3 受入 Owner 欄の
Virtual-first 再指向` 節を新設する。構成:

- 決定根拠の明記（Issue #192 本文 2026-11-15 節・
  POLICIES §2.1・方針基盤 RF #627 CLOSED）。
- remap 対応表（後述の語彙規則に従う 1 表）。
- 「設計 Issue は実装・実走の Owner ではない」旨と
  OS-L1/両 Consumer 独立検証の #294・AL #127・SV #28
  への移管の明記。
- 原則の再掲（Required→Deferred 降格禁止・NotRun 正直
  記録・物理機材/市販 Plugin の一括 Required 化なし・
  design-Close と impl 受入の分離・既存 Pass 付与なし）。
- 対応表側 addendum と本チェーン spec への参照。

既存節（Context・Scope・3 者 Ownership・capability
map・TA-001〜012 表・G0〜G7/E2E/GUI lane・ADR/互換
表・Consumer 接続面・Data and Control Flow・
Non-Functional Constraints・Design Done/Implementation
Done・Risks・Deferred Design Concerns・Verification
Notes）は一切変更しない。

### 追加位置 2: 対応表「Headless Gap → Open Gate 接続一覧」節

節内の 2026-11-14 snapshot 表の直後（`## Key Issues and
Assumptions` の前）に dated 追記を挿入する。構成:

- 追記の根拠と範囲（対象行・対象外行の明記）。
- remap 表（「受入 Owner（再指向後）」と「実装 Owner
  （維持）」の 2 列で領域別に記す）。
- 原則の再掲と tracking canonical 追記・本チェーン spec
  への参照。

対象行は次の 5 領域 + 共通基盤:

| 領域 | 受入 Owner（再指向後） | 実装 Owner（維持） |
| --- | --- | --- |
| 実 Audio Input / duplex・実機 capture | RF #629（OPEN — Windows 実 WASAPI/WinMM 仮想 Endpoint） | #251 OPEN |
| OS MIDI backend・VST3 翻訳 | RF #629（OPEN — 同上） | #253 OPEN |
| 実 GUI | RF #631（OPEN — 実 Window scenario runner） | #243 CLOSED・#313 CLOSED（2026-11-14 snapshot では OPEN — 陳腐化した旧受入参照） |
| Plugin Editor / Native Dock | RF #630（OPEN — 正規 ABI Test Plugin） | #388 OPEN・#389 OPEN |
| OS-L1 / 両 Consumer 独立検証 | RF #294 OPEN・AL #127 OPEN・SV #28 OPEN | — |
| 共通基盤 | RF #628 OPEN（deterministic test runtime）・RF #627 CLOSED（POLICIES §2.1 設計正本） | — |

対象外行（#262/#465・#219/#281/#294・#306/SV#84・
Consumer 継続追従行）は変更しない旨を明記する。

### 参照表記の語彙規則

- 各 Issue 参照は「番号 + 役割 + 現行状態
  （OPEN/CLOSED）」を揃えて記す（例: `RF #629 OS
  virtual endpoint survey/probe（OPEN — …）`）。
- #627 は CLOSED だが POLICIES §2.1 の設計正本として
  出典を併記する。#628〜631・#294・AL #127・SV #28 は
  2026-11-15 `gh issue view` で OPEN を照合済み。
- 「RF #」接頭は Ruviform repo、「AL #」「SV #」は
  それぞれ audio-lens・synvelis repo を指す既存表記に
  合わせる。
- Issue 状態は変動する snapshot であり、日付
  （2026-11-15 照合）を明記して恒久的事実として
  扱わない。

### 原則の記述規則

addendum には次を必ず含める:

- 本 remap は受入 Owner 欄の再指向であり、Windows で
  利用する機能の L2/L3 を Required から Deferred へ
  降格しない。
- 「仮想 Endpoint の Pass は物理機材の Driver/ADC-DAC
  を保証しない」「未検証 Case は `NotRun` と正直に
  記録」「物理機材・市販 Plugin の一括 Required 化は
  課さない」「design-Close と impl 受入を分離」。
- #251/#253/#388/#389 は実装・EXT-L3 mechanism の
  Owner として維持する。
- 既存実装/CI に追加の Pass を付与しない。未実装の
  仮想テストは `NotRun`、DComp 既知 Fail 等は維持。
  RF #627 の詳細設計と #628〜631 の実装を取り違えない。

## Data and Control Flow

文書のみの変更であり、データフローは存在しない。記述の
流れは:

1. Issue #192 本文決定 + POLICIES §2.1 → 本 spec が
   要求を固定。
2. 本 design が追記位置・表語彙・原則を固定。
3. tracking canonical・対応表へ dated addendum を追記
   し、`issue-192-virtual-first-owner-remap` 文書
   チェーンが derives-from/implements 関係を記録。
4. test-spec が addendum の検証規則を固定し、impl
   ir/exp が実施記録と検証証拠を保持する。

## Non-Functional Constraints

- **append-only**: 既存 snapshot 表・節構造・本文を
  書き換えない。追記は dated note として識別可能にする。
- **正確性**: Issue 番号・状態・役割は `gh issue view`
  実測（2026-11-15）に基づく。存在しない Issue・文書・
  lane を根拠にしない。
- **非昇格**: `NotRun`/`Fail`/`Unsupported` を `Pass`
  に置き換えない。lane・job 定義の存在から Pass を
  推論しない。
- **権威の非侵害**: umbrella 設計正本・POLICIES・
  AGENTS・`mise.toml`・`Cargo.toml` を変更しない。
- **スコープ**: `docs/` 配下と生成 index のみ。
  `evidence/`・`crates/`・manifest・workflow に差分を
  作らない。

## Risks and Trade-offs

- **Issue 状態の陳腐化**: #243/#313 の CLOSED 化を含め
  状態は変動する。対策: 照合日（2026-11-15）と
  snapshot 根拠（2026-11-14 比較）を明記し、将来の
  状態変化は gate 段階で再照合する旨を記す。
- **二重権威化**: addendum が既存表を書き換えると
  snapshot の履歴性が失われる。対策: append-only を
  設計制約として明記し、既存表を「履歴として保持」と
  記す。
- **Owner 混同**: 受入 Owner と実装 Owner を混同すると
  #251/#253/#388/#389 の役割を誤認する。対策: 対応表
  addendum で列を分け、原則節で維持を明記する。
- **降格の誤読**: remap を Required→Deferred と誤読
  すると受入が空洞化する。対策: 原則節で降格禁止を
  明示する。

## Deferred Design Concerns

- #628〜631 共通 Harness の構築・Case 実施 | 各 Issue
  が Owner。本 slice は Owner 欄の記録のみ | Harness
  Issue 群 | 再指向後の受入経路があっても実走証拠が
  なく Required Case が `NotRun` に留まる。
- Consumer 側 Windows L2/L3 Case の具体化 | AL #127・
  SV #28・#294 が Owner | Consumer 受入段階 | 製品機能
  Owner の Case 定義なしに L2/L3 Required が宙に浮く。
- TA gate の正式 Close 判定と #192 自体の Close | #192
  Owner と gate 証拠の棚卸し時 | 各 gate の証拠が揃った
  時点 | 証拠未完のまま tracker を閉じるリスク。

## Verification Notes

- [ ] 両 addendum が dated・append-only であり、既存
      snapshot 表に差分がないことを diff で確認する。
- [ ] remap 表の全行が 2026-11-15 `gh issue view` 実測の
      Issue 状態と一致することを `docs/impl/exp/` の
      記録と突合する。
- [ ] `NotRun`→`Pass` の置換・Required→Deferred 降格が
      ないことを diff で確認する。
- [ ] `mise run fmt-check`・`mise exec -- prek run
      --all-files`・doc index `index plan`/`apply`・
      `doc-status` audit・type 別 lint・impl 監査が
      成功すること。
