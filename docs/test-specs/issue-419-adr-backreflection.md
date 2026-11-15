---
id: "TSPEC-034XPQypgsINSAsr2QivQO"
type: "test-spec"
status: "proposed"
title: "Issue #419 後継ADR verdict逆反映 検証（docs-only slice）"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/419"
  changes:
    added:
      - type: "doc"
        path: "docs/test-specs/issue-419-adr-backreflection.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-419-adr-backreflection.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-419-adr-backreflection.md"
    - "docs/designs/issue-419-adr-backreflection.md"
    - "docs/test-specs/issue-262-gpui-verdict.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-419-adr-backreflection.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-419-adr-backreflection.md"
    - "docs/designs/issue-419-adr-backreflection.md"
  verified-by: []
  references:
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
    - "evidence/rf262-gpui-verdict/verdict-matrix.md"
    - "evidence/rf262-gpui-verdict/report.md"
    - "evidence/rf262-gpui-verdict/handoff-419.md"
    - "POLICIES.md"
  defers: []
  deferred-by: []
---

# Issue #419 後継ADR verdict逆反映 検証（docs-only slice）

## Purpose

Issue #419 の「Coordinator 決裁済み verdict を後継 ADR へ逆反映する」docs-only slice が、生み出す ADR 記録と文書チェーンが満たすべき不変条件を固定する。本 test-spec は実行可能テストではなく、ADR 逆反映記述の**正しさの契約**を定義する。実装（文書）の正しさは本 spec の Rule/Example に照らして監査し、verdict の実測正しさは `evidence/rf262-gpui-verdict/` の証拠と `verdict-matrix.md` が担保する。本 slice の記録が Issue #419 の正式 status 決裁・残 DoD の判断材料として再利用される限り有効である。

## Feature

**決裁済み GPUI 採否 verdict（CONDITIONAL）の ADR 逆反映記録**

#262 の verdict 証拠（`evidence/rf262-gpui-verdict/`）に基づき Coordinator が決裁した outcome = CONDITIONAL を、後継 ADR（`driver-neutral-gui-gpui-driver-webview-first-issue-384.md`、status `proposed`）の Confirmation / More Information / frontmatter へ、判定語彙・誠実性規則・記録構造どおりに日付付きで記録し、追跡用の docs-only 文書チェーンを完成させる。

## Rules

### R-RECORD: 逆反映記録の存在と構造

- **R-RECORD-01**: ADR `## More Information` 末尾の逆反映欄は「未記入」を含まず、日付（`YYYY-MM-DD`）付きで outcome = CONDITIONAL・判定根拠（Gate D Fail・Gate E NotRun・Issue #262 Verdict 規則該当）・「決裁は Coordinator・本欄はその結果の記録」旨を記録しなければならない。
- **R-RECORD-02**: 同欄は「L1 のみ評価済み、L2/L3 非保証」・GPUI 標準 Driver 方針維持・Native Dock 無条件本番採用不可（条件付き/未受入）・ADR status は `proposed` のまま、を明示しなければならない。
- **R-RECORD-03**: 同欄は OS 別 Capability 宣言記録（`native_surface_slot` = Windows `Supported` / macOS・Linux `Unsupported`、`ime` = 全 OS `Unsupported`、focus release = `Err(Unsupported)`）を「宣言」と明記して含めなければならない。
- **R-RECORD-04**: 同欄は既知制約/将来 Owner（`#388`/`#389`/`#532` の Windows DComp/Native Dock 未成立・`#532` open Blocker・`#243`/`#313` の実 GUI 残件・`#465` 独立判断境界・`#630`/`#631` 仮想優先 Gate と正式 status 決裁の残 DoD）を非 Blocking として記録しなければならない。
- **R-RECORD-05**: ADR `### Confirmation` は既存 bullet を保持したまま「#262 verdict の逆反映を実施した」旨の日付付き記録を追加し、outcome・More Information 記録への相互参照・無条件採用を宣言しない制約の維持を含めなければならない。
- **R-RECORD-06**: ADR からの証拠参照は `docs/adr/` からの相対リンク（`../../evidence/rf262-gpui-verdict/verdict-matrix.md`・`report.md`・`handoff-419.md`）でなければならない。

### R-VOCAB: 判定語彙の分離

- **R-VOCAB-01**: 要求状態 `Required`/`Deferred`/`NotApplicable` と実行結果 `Pass`/`Fail`/`NotRun`/`Unsupported` を混同してはならない（POLICIES.md §3・verdict-matrix 凡例）。
- **R-VOCAB-02**: `NotRun`/`Deferred`/`Unsupported`/`Fail` を `Pass` に読み替えてはならない。OS-L1 の Pass・Windows L2 の部分実証・Windows L3/Gate D の未達・macOS/Linux L2/L3 の Deferred・Gate C/E の NotRun は verdict-matrix の判定どおりに記録する。
- **R-VOCAB-03**: Capability 宣言値（`Supported`/`Unsupported`）は実測 Pass と区別し、「宣言」と明記しなければならない。

### R-SCOPE: 変更範囲の限定

- **R-SCOPE-01**: 既存文書の変更は対象 ADR 1 ファイルのみとし、`evidence/`・`POLICIES.md`・`AGENTS.md`・`mise.toml`・`Cargo.*`・`crates/**`・`ui/**`・`.github/workflows/`・他 `docs/adr/*`・他 slug docs を変更してはならない。
- **R-SCOPE-02**: ADR frontmatter の `status` は `proposed` のまま変更してはならない。`id`・`created`・既存 `relations` も変更しない。`updated` の当日日付化と `references` への `#419`/`#532` 追加のみ許可する。
- **R-SCOPE-03**: ADR の Decision Outcome・判定フロー・縮退順序・責務マップ・Capability Matrix 宣言テンプレート行・Verification チェックリストの既存記述を書き換えてはならない（Capability Matrix は宣言/実測分離のため原則不変）。
- **R-SCOPE-04**: 新規文書は slug `issue-419-adr-backreflection` の 7 ファイル（spec/design/plan/task/test-spec/impl-ir/impl-exp）のみとし、front-matter の `id`/`type`/`status`/`relations` は doc-suite 規約に従う。
- **R-SCOPE-05**: docs index（`docs/adr/README.md`・各 dir `README.md`）は `doc-maintenance index apply`・`sync_impl_index.js`・`docs/adr` index 手順の生成物のみとし、手編集してはならない。

### R-HONESTY: 決裁記録の誠実性

- **R-HONESTY-01**: verdict は「Coordinator 決裁済みの結果を ADR に記録した」と記述し、Worker が採否を決めたかのような記述・「推奨」を「決裁」に読み替える記述をしてはならない。
- **R-HONESTY-02**: 「条件付き採用」「L1 のみ評価済み」を「無条件採用」「全面合格」「accepted」に読める表現にしてはならない。ADR status は `proposed` のままと明記する。
- **R-HONESTY-03**: `#532` の未解消（open Blocker）を解消済みのように書いてはならない。
- **R-HONESTY-04**: Issue #419 の正式 status 決裁・Issue 投稿・Close・`#630`/`#631` 仮想優先 Gate 記載・`#465` 独立判断を本 slice の成果として記述してはならない（残 DoD として正直に残す）。

## Examples

### EX-1: CONDITIONAL outcome の記録

- **Given**: `handoff-419.md` が「L1 のみ評価済み、L2/L3 非保証」「GPUI 標準 Driver 方針維持・Native Dock 無条件本番採用不可」「#419 を accepted にしない」条件付き判定材料を定め、Coordinator が outcome = CONDITIONAL を決裁した。
- **When**: ADR More Information 逆反映欄を記入する。
- **Then**: outcome = CONDITIONAL・Verdict 規則該当（Gate D Fail + Gate E NotRun）・「L1 のみ評価済み、L2/L3 非保証」・標準 Driver 方針維持・Native Dock 無条件採用不可・status `proposed` 維持が日付付きで記録され、「未記入」は残らない。

### EX-2: 宣言 Capability と実測の分離

- **Given**: `native_surface_slot` は `BACKEND_AVAILABLE = cfg!(target_os = "windows")` により Windows のみ `Supported` を返し、macOS/Linux は `Unsupported` を宣言する。`ime` は全 OS `Unsupported` 宣言、focus release は `Err(Unsupported)` を返す。
- **When**: ADR 逆反映欄に OS 別 Capability を記録する。
- **Then**: 各値が「宣言」と明記され、実測 Pass・Windows L2 部分実証・Gate D 画素 Fail と混同されない。

### EX-3: Confirmation の dated 実施記録

- **Given**: `### Confirmation` に「この逆反映まで、本 ADR は GPUI の無条件採用を宣言しない」制約がある。
- **When**: 逆反映実施の記録を追加する。
- **Then**: 既存 bullet を保持したまま「2026-11-15: #262 verdict（outcome = CONDITIONAL）を More Information 逆反映欄へ記録」旨が追加され、無条件採用を宣言しない制約と status `proposed` が維持される。

### EX-4: 範囲外変更の不作為

- **Given**: 本 slice は docs-only であり、ADR status 決裁・Issue 投稿・`evidence/` 編集・他 ADR/POLICIES/コード変更は Coordinator/別 slice の権限。
- **When**: 変更ファイル一覧を監査する。
- **Then**: 変更は対象 ADR 1 ファイル + 新規 7 文書 + 生成 index のみであり、`evidence/rf262-gpui-verdict/`・他 ADR・`status` frontmatter に差分がない。

## Guarantees

- ADR の逆反映欄・Confirmation・frontmatter が R-RECORD-01〜06 を満たし、判定語彙が R-VOCAB-01〜03 に従う。
- 変更範囲が R-SCOPE-01〜05 に限定され、status `proposed`・`id`/`created`/既存 `relations` が不変である。
- verdict の記述が R-HONESTY-01〜04 に従い、「決裁済み CONDITIONAL の記録」であり「無条件採用」「accepted」「解消済み」を含まない。
- 文書チェーン 7 ファイルの front-matter relation が相互整合し、触れた全 dir の管理 index が `in-sync` である。

## Non-goals

- verdict の再判定・新規証拠の収集・実機/CI 検証の実施。
- ADR status の正式決裁・Issue #419 へのコメント投稿・Close・チェックボックス操作。
- 仮想優先 Gate（`#630`/`#631`）証拠深度記載・`#465` Legacy 撤去の独立判断・Capability Matrix 行の更新。
- コード・CI workflow・依存・`evidence/` の変更。

## Risk

逆反映記述の不変条件が破られると、Issue #419 の正式 status 決裁が「L1 のみ評価済み」の範囲を超えて L2/L3 を保証したと誤認される。特に CONDITIONAL を「採用/accepted」と読み替える誤り、`NotRun`/`Fail`/`Unsupported` を `Pass` に塗り替える誤り、`#532` Blocker の解消済み化、宣言 Capability を実測 Pass と混同する誤りは、製品 GUI 機能や実 Plugin 表示の不存在を隠蔽する。R-RECORD/R-VOCAB/R-SCOPE/R-HONESTY の保証が落ちると、#262 Verdict 規則と Coordinator 決裁の記録としての ADR の信頼性が損なわれる。
