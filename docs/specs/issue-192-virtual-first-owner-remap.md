---
id: "SPEC-C2oZ7zQDj9Mj2XANYlamgo"
type: "spec"
status: "approved"
title: "Issue #192 TA DAG L2/L3 受入 Owner 欄の Virtual-first 再指向"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "POLICIES.md"
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
  implements: []
  implemented-by:
    - "docs/designs/issue-192-virtual-first-owner-remap-design.md"
    - "docs/plans/issue-192-virtual-first-owner-remap-plan.md"
  depends-on:
    - "docs/specs/issue-192-synvelis-driven-target-architecture-migration.md"
    - "docs/designs/issue-192-synvelis-driven-target-architecture-migration.md"
    - "docs/discovery/issue-192-ta-invariant-map.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-192-ta-invariant-map.md"
    - "docs/specs/issue-627-virtual-first-verification-contract.md"
    - "docs/designs/issue-275-observability-integration-design.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-192-synvelis-driven-target-architecture-migration.md"
    - "https://github.com/scarecrow173/ruviform/issues/192"
  derived-by:
    - "docs/designs/issue-192-virtual-first-owner-remap-design.md"
    - "docs/plans/issue-192-virtual-first-owner-remap-plan.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-192-virtual-first-owner-remap.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/627"
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/251"
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/243"
    - "https://github.com/scarecrow173/ruviform/issues/313"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/synvelis/issues/28"
  defers: []
  deferred-by: []
---

# Issue #192 TA DAG L2/L3 受入 Owner 欄の Virtual-first 再指向

## What Should Be Built

Issue #192 本文「2026-11-15 Virtual-first正式受入Owner整理」
と `POLICIES.md` §2.1（2026-11-15 決定「仮想優先の受入と
共通テスト基盤」。方針基盤 Issue は RF #627 CLOSED）に基づき、
headless に可能な文書スライスとして次を作成・更新する。

1. **tracking canonical への dated addendum 追記**:
   `docs/designs/issue-192-synvelis-driven-target-architecture-migration.md`
   の末尾に当日日付の追記節を追加し、設計 DAG の L2/L3
   受入 Owner 欄の remap を記録する。既存の 2026-11-09
   snapshot 表は履歴として保持し append-only とする。
2. **対応表への dated addendum 追記**:
   `docs/discovery/issue-192-ta-invariant-map.md` の
   「Headless Gap → Open Gate 接続一覧」節に当日日付の
   追記を入れ、領域別の受入 Owner 再指向を記録する。
   既存の 2026-11-14 snapshot 表は履歴として保持する。
3. **文書チェーンと検証記録**: 本 spec を起点に design /
   test-spec / plan / task / impl(ir/exp) の文書チェーンを
   `issue-192-virtual-first-owner-remap` 名で作成し、
   実行した検証と参照 Issue の実在照合結果を
   `docs/impl/exp/` 側に記録する（本 slice は `evidence/`
   を作らない）。

## Why Now

- Issue #192 は TA v0.7.1 Dependency DAG / Graph / State
  Ownership の perpetual tracker であり、tracking canonical
  （2026-11-09 snapshot）と対応表（2026-11-14 snapshot）は
  固定済みだが、設計 DAG の L2/L3 受入 Owner 欄は物理
  Device 一律必須を前提とした旧記述のままである。
- 2026-11-15 の Issue 本文決定と POLICIES §2.1 により、
  Windows で利用する GUI-L2/EXT-L3 は実 Window・本番 OS
  API を通る仮想 Endpoint・正規 ABI Test Plugin 等で受入
  し、物理機材・市販 Plugin は通常必須ではない方針が
  確定した。受入 Owner 欄を #628〜631 共通 Harness と
  #294/AL #127/SV #28 へ再指向する記録が必要である。
- Issue 状態は変動する（例: #243/#313 は 2026-11-14
  snapshot では OPEN だったが現行 CLOSED）。`gh issue
  view` 実測に基づく snapshot として本 slice で固定し、
  gate 段階の Owner が参照できるようにする。

## Users and Value

- **Issue #192 Owner / gate 担当**: TA gate・Headless Gap
  の受入 Owner がどの Issue・Harness に委譲されたかを
  dated addendum から一意に辿れる。
- **#628〜631 共通 Harness Owner**: deterministic test
  runtime・OS virtual endpoint・ABI Test Plugin・real-GUI
  runner が受入 Owner として登録されたことを確認できる。
- **#294・AL #127・SV #28 Owner**: OS-L1/両 Consumer 独立
  検証の移管先を確認できる。
- **#251/#253/#388/#389 Owner**: 実装 Owner としての
  役割が維持され、受入経路だけが #629/#630/#631 へ
  再指向されたことを区別できる。

## Scope

### In Scope

- `docs/designs/issue-192-synvelis-driven-target-architecture-migration.md`
  末尾への dated addendum 追記（`updated` の当日日付更新を
  含む。既存本文・snapshot 表は不変）。
- `docs/discovery/issue-192-ta-invariant-map.md`
  「Headless Gap → Open Gate 接続一覧」節への dated
  addendum 追記（`updated` の当日日付更新を含む。
  既存 snapshot 表・指定外の行は不変）。
- `issue-192-virtual-first-owner-remap` 名の文書チェーン
  （spec/design/test-spec/plan/task/impl ir・exp）。
- 触った dir の生成 index の `doc-maintenance` による再生成。
- 参照 Issue の `gh issue view` 実在照合と検証記録
  （`docs/impl/exp/`）。

### Out of Scope

- `crates/`・`Cargo.toml`・`Cargo.lock`・`mise.toml`・
  `.github/workflows/`・`POLICIES.md`・`AGENTS.md`・
  `.devin/` の変更（コード・ポリシー変更なし）。
- `docs/designs/synvelis-driven-target-architecture-and-migration.md`
  （umbrella 設計正本・read-only）や他 slug の docs の変更。
- `evidence/` ディレクトリの作成（検証記録は
  `docs/impl/exp/` に残す）。
- #628〜631・#294・Consumer repo での実装・実走・受入
  （各 Issue が Owner。本 slice は Owner 欄の記録のみ）。
- GitHub blockedBy の登録・削除、新規 Issue 起票。
- 既存実装/CI への追加 Pass の付与、Required→Deferred の
  降格、未検証 Case の Pass 化。

## Requirements

- tracking canonical の addendum は末尾への追記のみとし、
  既存の 2026-11-09 TA gate 表・3 者 Ownership 表・
  capability map を書き換えない（append-only）。
- 対応表の addendum は「Headless Gap → Open Gate 接続一覧」
  節への追記のみとし、既存の 2026-11-14 snapshot 表と
  指定外の行（#262/#465・#219/#281/#294・#306/SV#84・
  Consumer 継続追従行）を変更しない。
- remap の対応は次を網羅する: Windows 実 WASAPI/WinMM
  仮想 Endpoint → RF #629、正規 Plugin ABI → RF #630、
  Windows 使用 GUI（実 Window runner）→ RF #631、
  OS-L1/両 Consumer 独立検証 → RF #294・AL #127・SV #28。
  共通基盤として RF #628（deterministic test runtime）と
  方針基盤 RF #627（CLOSED・POLICIES §2.1 の設計正本）を
  併記する。
- 各参照は Issue 番号・役割・現行状態（open/closed）を
  明記し、`gh issue view` の実測と一致させる。
- 対応表 addendum では「受入 Owner（acceptance）」と
  「実装 Owner」を区別して記し、#251（実 Audio Input）・
  #253（OS MIDI）・#388/#389（Native Dock/Plugin Editor）
  は実装 Owner として維持する。#243/#313 が CLOSED に
  なった旨を日付根拠付きで明記する。
- 「仮想 Endpoint の Pass は物理機材の Driver/ADC-DAC を
  保証しない」「未検証 Case は `NotRun` と正直に記録」
  「物理機材・市販 Plugin の一括 Required 化は課さない」
  「design-Close と impl 受入を分離」の原則を dated note
  に添え、Required→Deferred の降格を行わない。
- 既存実装/CI に追加の Pass を付与せず、未実装の仮想
  テストは `NotRun`、DComp 既知 Fail 等の既存結果を維持
  する。RF #627 の詳細設計と #628〜631 の実装を取り違え
  ない。

## Acceptance Criteria

- [ ] tracking canonical 末尾に当日日付の追記節があり、
      remap 対応（#629/#630/#631/#294/AL #127/SV #28・
      共通基盤 #628・方針基盤 #627）と各 Issue の
      番号・役割・状態が明記されている。
- [ ] 対応表「Headless Gap → Open Gate 接続一覧」節に
      当日日付の追記があり、領域別の受入 Owner/実装
      Owner の再指向と #243/#313 CLOSED の明記、指定外
      行の不変が確認できる。
- [ ] Required→Deferred 降格・Pass 付与・NotRun の
      Pass 化がなく、維持すべき原則が dated note に
      記されている。
- [ ] `issue-192-virtual-first-owner-remap` の文書
      チェーン 7 ファイルが作成され、front matter の
      `status`/`relations` が実態どおりである。
- [ ] 触った dir の managed index が `doc-maintenance`
      の `index apply` で再生成され、`index plan` で
      in-sync である。
- [ ] 実行した検証（コマンド・exit code）と参照 Issue の
      実在照合結果が `docs/impl/exp/` の記録に残り、
      cargo test 等を実行していないこと・docs-only で
      あることが正直に記されている。
- [ ] 変更が `docs/` 配下と生成 index のみであり、
      `crates/`・manifest・`mise.toml`・`POLICIES.md`・
      `AGENTS.md`・umbrella 設計正本に差分がない。

## Deferred Design Concerns

- #628〜631 共通 Harness の未構築により、再指向後の
  受入 Owner が要求する Required Case は `NotRun` の
  まま残る | 各 Harness Issue（#628〜631）が Owner |
  Harness 構築・Case 実施時 | 受入経路が記録されても
  実走証拠がなく機能が未受入のまま滞留するリスク。
- Issue 状態 snapshot の陳腐化 | gate 段階での再照合 |
  状態変動時 | 旧 OPEN/CLOSED 記述を現状と誤読する
  リスク（日付明記で緩和）。
- 物理機材・市販 Plugin の個別適合 | POLICIES §2.1 の
  限界規定どおり原則 Gate 外 | 正式サポート宣言・
  不具合調査時に対象 Case を定義 | 仮想 Pass を物理
  保証と誤認するリスク。

## Sources and Relations

- 正本 Issue: [#192](https://github.com/scarecrow173/
  ruviform/issues/192)（perpetual tracker。本文
  「2026-11-15 Virtual-first正式受入Owner整理」が本
  slice の決定根拠。`Refs #192` で参照し Close しない）。
- 方針基盤: [#627](https://github.com/scarecrow173/
  ruviform/issues/627)（CLOSED）・`POLICIES.md` §2.1・
  `docs/specs/issue-627-virtual-first-verification-contract.md`。
- 受入 Owner 候補: RF #628/#629/#630/#631（OPEN）・
  #294（OPEN）・AL #127（OPEN）・SV #28（OPEN）。
- 実装 Owner（維持）: #251/#253/#388/#389（OPEN）、
  #243/#313（CLOSED）。
- 更新対象正本: `docs/designs/issue-192-synvelis-driven-
  target-architecture-migration.md`（tracking canonical）・
  `docs/discovery/issue-192-ta-invariant-map.md`
  （対応表）。
- 表現先例: `docs/designs/issue-275-observability-
  integration-design.md` §7 の Deferred Owner/Trigger
  表および 2026-11-15 dated note。
- 文書チェーン: `docs/designs/issue-192-virtual-first-
  owner-remap-design.md`・`docs/test-specs/issue-192-
  virtual-first-owner-remap.md`・`docs/plans/issue-192-
  virtual-first-owner-remap-plan.md`・`docs/tasks/
  issue-192-virtual-first-owner-remap-task.md`・
  `docs/impl/ir/issue-192-virtual-first-owner-remap.md`・
  `docs/impl/exp/issue-192-virtual-first-owner-remap.md`。
