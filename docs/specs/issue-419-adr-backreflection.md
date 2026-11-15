---
id: "SPEC-034XPQgOgKyGN5Ydw7c93D"
type: "spec"
status: "proposed"
title: "Issue #419 後継ADR verdict逆反映 — 決裁済み CONDITIONAL 判定の ADR 記録（docs-only slice）"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/419"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/532"
    - "https://github.com/scarecrow173/ruviform/issues/465"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/synvelis/issues/176"
    - "https://github.com/scarecrow173/audio-lens/issues/166"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-419-adr-backreflection.md"
      - type: "doc"
        path: "docs/designs/issue-419-adr-backreflection.md"
      - type: "doc"
        path: "docs/test-specs/issue-419-adr-backreflection.md"
      - type: "doc"
        path: "docs/plans/issue-419-adr-backreflection.md"
      - type: "doc"
        path: "docs/tasks/issue-419-adr-backreflection.md"
      - type: "doc"
        path: "docs/impl/ir/issue-419-adr-backreflection.md"
      - type: "doc"
        path: "docs/impl/exp/issue-419-adr-backreflection.md"
    modified:
      - type: "doc"
        path: "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "doc"
        path: "docs/adr/README.md"
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
      - type: "doc"
        path: "docs/impl/exp/README.md"
  implements: []
  implemented-by:
    - "docs/designs/issue-419-adr-backreflection.md"
    - "docs/plans/issue-419-adr-backreflection.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
    - "docs/specs/issue-262-gpui-verdict.md"
    - "docs/designs/issue-262-gpui-verdict.md"
    - "docs/test-specs/issue-262-gpui-verdict.md"
    - "evidence/rf262-gpui-verdict/verdict-matrix.md"
    - "evidence/rf262-gpui-verdict/report.md"
    - "evidence/rf262-gpui-verdict/handoff-419.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/419"
    - "https://github.com/scarecrow173/ruviform/issues/262"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-419-adr-backreflection.md"
  references:
    - "evidence/rf262-gpui-verdict/verdict-matrix.md"
    - "evidence/rf262-gpui-verdict/report.md"
    - "evidence/rf262-gpui-verdict/handoff-419.md"
    - "evidence/rf262-gpui-verdict/verification.md"
    - "docs/specs/issue-262-gpui-verdict.md"
    - "docs/designs/issue-262-gpui-verdict.md"
    - "docs/test-specs/issue-262-gpui-verdict.md"
    - "docs/plans/issue-262-gpui-verdict.md"
    - "docs/tasks/issue-262-gpui-verdict.md"
    - "docs/impl/ir/issue-262-gpui-verdict.md"
    - "docs/impl/exp/issue-262-gpui-verdict.md"
  defers: []
  deferred-by: []
---

# Issue #419 後継ADR verdict逆反映 — 決裁済み CONDITIONAL 判定の ADR 記録（docs-only slice）

## Problem

Issue #419 は「#262 PoC 判定に基づく後継 ADR の最終採用」を扱う Issue であり、その DoD の一つが「#262 の判定結果を後継 ADR（`docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md`、status `proposed`）へ逆反映する」ことである。#262 の verdict 証拠スライス（`evidence/rf262-gpui-verdict/`、Issue #262 文書チェーン `issue-262-gpui-verdict`）は main に merge 済みで、Coordinator が verdict の採否 outcome を **CONDITIONAL（条件付き）** として決裁した。

現時点で判明している構造的事実:

- **判定 outcome は CONDITIONAL**: Issue #262 の Verdict 規則「全必須 OS/format で D・E が未達なら GPUI 標準 Driver 方針維持、Native Dock の無条件本番採用不可と明記し #419 を accepted にしない」に該当する。Gate D Fail（GPUI 0.2.2 DComp `CreateTargetForHwnd`（`topmost=true`）が child HWND を覆い plugin 画素が非合成。`#532` open Blocker）+ Gate E NotRun が根拠。
- **ADR には逆反映の受け皿が既に規定されている**: `### Confirmation` が「判定結果を本 ADR の More Information へ逆反映する」「この逆反映まで、本 ADR は GPUI の無条件採用を宣言しない」と定め、`## More Information` 末尾に「GPUI 最終採用判定の逆反映欄（未記入）」が置かれている。
- **本 slice の責務は「決裁済み verdict の ADR 本文への記録」のみ**: verdict の再判定・ADR status 変更（`proposed`→`accepted` 等）・Issue #419 へのコメント投稿・Close 判定・仮想優先 Gate（`#630`/`#631`）の証拠深度記載・`#465` Legacy 撤去の独立判断は本 slice の範囲外であり、Coordinator / design authority の権限または後続作業である。

本スライスの成果物は、Coordinator 決裁済み verdict を ADR の Confirmation / More Information に日付付きで記録し、同内容を追跡する docs-only の文書チェーン（spec→design→plan→task→test-spec→impl/ir→impl/exp）を作ることである。

## Audience

- Coordinator / design authority — Issue #419 の正式 status 決裁・残 DoD（仮想優先 Gate 記載・`#465` 独立判断）の判断材料として、決裁済み verdict が ADR に正確に記録されたことの確認
- Issue #419/#262/#532/#465 の担当 — verdict outcome（CONDITIONAL）と「L1 のみ評価済み、L2/L3 非保証」の境界の確認
- Synvelis #176 / AudioLens #166 の製品 GUI Owner — Windows で実際に提供・利用する能力の L2/L3 未達範囲の確認材料
- 以降の docs 監査・index 再生成の担当 — 変更範囲と生成物の対応関係

## Scope

### In Scope

- ADR `docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md` の `## More Information` 末尾「GPUI 最終採用判定の逆反映欄」への決裁済み verdict（outcome = CONDITIONAL）の日付付き記録
- 同 ADR `### Confirmation` への逆反映実施済み dated 記録の追加（既存の「無条件採用を宣言しない」制約は維持）
- 同 ADR frontmatter の `updated` を当日日付へ更新、`references` への `#419`/`#532` 追加（`status: "proposed"` は変更しない）
- 本 slice の文書チェーン 7 ファイル（spec/design/plan/task/test-spec/impl-ir/impl-exp、slug `issue-419-adr-backreflection`）
- 触れた全 dir の管理 index 再生成（`doc-maintenance index apply`・`sync_impl_index.js`・`docs/adr` index）

### Out of Scope

- コード・CI workflow・`Cargo.toml`/`Cargo.lock`/`mise.toml` の変更（docs-only slice）
- ADR `status` の変更（`proposed`→`accepted`/`conditional`/`rejected`）および採否の正式決裁宣言（design authority / Coordinator 権限）
- Issue #419（および #262/#465/#532 等）へのコメント投稿・Close・チェックボックス操作（Coordinator が実施）
- verdict の再判定・`evidence/` 配下（`rf262-gpui-verdict/` を含む）の編集（判定正本は読み取り専用）
- 仮想優先 Gate（`#630`/`#631`）の証拠深度記載・`#465` Legacy 撤去の独立判断（Issue #419 の残 DoD）
- Capability Matrix（ADR L171-188 付近の宣言テンプレート行）の書き換え（宣言と実測の分離を壊さないため原則不変）
- 本 ADR 以外の `docs/adr/*`・`POLICIES.md`・`AGENTS.md`・他 slug の docs 全般の変更

## Requirements

- 逆反映内容は `evidence/rf262-gpui-verdict/handoff-419.md`「#419 への引き渡し」節と `verdict-matrix.md` の判定規則に従う: outcome = CONDITIONAL、「L1 のみ評価済み、L2/L3 非保証」、GPUI 標準 Driver 方針維持・Native Dock 無条件本番採用不可、OS 別 Capability 宣言記録、既知制約/将来 Owner の記録。
- 判定語彙は `POLICIES.md` §3 に従う: 要求状態 `Required`/`Deferred`/`NotApplicable` と実行結果 `Pass`/`Fail`/`NotRun`/`Unsupported` を混同しない。`NotRun`/`Deferred`/`Unsupported`/`Fail` を `Pass` にしない。
- 宣言値（`native_surface_slot`/`ime`/focus release の `Supported`/`Unsupported`）と実測 Pass を混同せず、「宣言」と明記する。
- verdict は「Coordinator 決裁済みの結果を記録した」と書き、Worker が採否を決めたかのような記述・「推奨」を「決裁」に読み替える記述をしない。
- `#532` の未解消（open Blocker）を解消済みのように書かない。「条件付き採用」「L1 のみ評価済み」を「無条件採用」「全面合格」に読める表現にしない。
- ADR からの証拠参照は `docs/adr/` からの相対リンク（`../../evidence/rf262-gpui-verdict/...`）で行う。
- docs index は手で編集せず、`doc-maintenance`/`impl-doc` の生成手順で再生成する。

## Contract outcome

本スライスが生む成果物は「決裁済み verdict の ADR 記録とその文書チェーン」である:

1. **ADR 逆反映**: `driver-neutral-gui-gpui-driver-webview-first-issue-384.md` の More Information 逆反映欄（dated 記録）・Confirmation（dated 実施記録）・frontmatter `updated`/`references` の更新。`status: "proposed"` は不変。
2. **文書チェーン**: `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/issue-419-adr-backreflection.md` の 7 ファイル（本ファイルを正本 spec とする）。
3. **管理 index**: 触れた全 dir の `README.md` を `doc-maintenance index apply` / `sync_impl_index.js` / `docs/adr` index 手順で再生成。

**本スライスが行わないこと（明示）**: ADR status の正式決裁・Issue 投稿・Close・verdict 再判定・`#630`/`#631` 証拠深度記載・`#465` 独立判断。これらは Issue #419 の残 DoD であり、PR は `Refs #419`（`Closes` 禁止）とする。

## Acceptance criteria

本 slice の受入条件（Coordinator 確定）に対応する:

- [ ] `## More Information` 末尾の逆反映欄が「未記入」でなくなり、Coordinator 決裁済み verdict（outcome = CONDITIONAL）・「L1 のみ評価済み、L2/L3 非保証」・GPUI 標準 Driver 方針維持・Native Dock 無条件本番採用不可・OS 別 Capability 宣言記録・既知制約/将来 Owner・`evidence/rf262-gpui-verdict/` への相対リンクが日付付きで記録される。
- [ ] `### Confirmation` に「#262 verdict の逆反映を実施した」旨の日付付き記録と outcome・More Information 記録への相互参照が追加され、無条件採用を宣言しない制約が維持される。
- [ ] frontmatter `status` は `proposed` のまま、`updated` が当日日付、`references` に `#419`/`#532` が追加される（`id`・`created`・既存 `relations` は不変）。
- [ ] slug `issue-419-adr-backreflection` の文書チェーン 7 ファイルが存在し、front-matter の `id`/`type`/`status`/`relations` が規約どおり相互整合する。
- [ ] 触れた全 dir の管理 index が生成手順で再生成され `in-sync` である。
- [ ] `NotRun`/`Deferred`/`Unsupported`/`Fail` を `Pass` にせず、「条件付き採用」を「無条件採用」「accepted」に読める記述がない。

## Evidence

- `docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md` — 逆反映先 ADR（本 slice の唯一の既存文書変更）
- `evidence/rf262-gpui-verdict/verdict-matrix.md` — Gate A〜E の CaseID/expected/observed/verdict/証拠 URL 正本
- `evidence/rf262-gpui-verdict/report.md` — 評価 report・推奨 verdict・既知 gap
- `evidence/rf262-gpui-verdict/handoff-419.md` — #419/#465 引き渡し文面（逆反映内容の起草元）
- `evidence/rf262-gpui-verdict/verification.md` — #262 slice の検証実行記録
- `docs/specs/issue-262-gpui-verdict.md` 他 `issue-262-gpui-verdict` チェーン — 同一 stem 命名・文書規約の先例
- `POLICIES.md` §2/§3/§5 — OS 別検証レベル・判定語彙分離・ローカル検証と CI の責務分離

## Deferred Design Concerns

- ADR `status` の正式決裁（`proposed`→`accepted`/`conditional`/`rejected`）— design authority 権限。Trigger: Issue #419 の残 DoD 完了後の決裁。
- 仮想優先 Gate（`#630`/`#631`）の証拠深度記載 — Issue #419 残 DoD。Trigger: Coordinator の Issue 更新。
- `#465` Legacy WebView 撤去の独立判断 — L1 互換と未実 GUI 製品機能の破壊を分離して判断。Trigger: #465 側の受入条件確定。
- `#532` DComp `topmost=true` 未解消 — Windows Native Dock 画素の根本 Blocker（open）。Trigger: #532 の対策決裁・実装。
- Windows L2 残能力（IME・a11y・DPI 動的・物理入力等）・Gate C 性能・Gate E Consumer Oracle — NotRun/Deferred のまま。Trigger: 利用機能確定・性能予算 Issue・consumer-verify 対象 SHA run。

## Sources and Relations

- Issue #419（本 spec の source・引き渡し先）・#262（verdict 証拠の出所）・#388/#389/#532（Native Dock PoC と DComp Blocker）・#465（Legacy 撤去の独立判断）・#294（両 Consumer 互換）・親 #257/#384（後継 ADR）
- `POLICIES.md` §2/§3/§5（OS 別検証レベル・要求状態と実行結果の分離・ローカル検証と CI の責務分離）
- `docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md`（status `proposed`。本 slice は status を変更しない）
- `evidence/rf262-gpui-verdict/`（#262 verdict 証拠正本・読み取り専用）
- `issue-262-gpui-verdict` 文書チェーン（同一 stem 命名・docs-only slice の先例）
