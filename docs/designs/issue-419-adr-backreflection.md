---
id: "DESIGN-034XPQhsUWcCgjNEs31H4d"
type: "design"
status: "approved"
title: "Issue #419 後継ADR verdict逆反映 設計（docs-only slice）"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/419"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-419-adr-backreflection.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-419-adr-backreflection.md"
  implemented-by:
    - "docs/plans/issue-419-adr-backreflection.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
    - "docs/specs/issue-419-adr-backreflection.md"
    - "docs/test-specs/issue-419-adr-backreflection.md"
    - "docs/specs/issue-262-gpui-verdict.md"
    - "docs/designs/issue-262-gpui-verdict.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-419-adr-backreflection.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-419-adr-backreflection.md"
  references:
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
    - "POLICIES.md"
    - "evidence/rf262-gpui-verdict/verdict-matrix.md"
    - "evidence/rf262-gpui-verdict/report.md"
    - "evidence/rf262-gpui-verdict/handoff-419.md"
    - "docs/impl/exp/issue-532-dcomp-audit.md"
  defers: []
  deferred-by: []
---

# Issue #419 後継ADR verdict逆反映 設計（docs-only slice）

本書は Issue #419 の「Coordinator 決裁済み verdict を後継 ADR へ逆反映する」docs-only slice の**記録構造と判定語彙の適用規則**を定義する。判定根拠の正本は `evidence/rf262-gpui-verdict/`（読み取り専用）、verdict の決裁は Coordinator が行い本 slice はその結果の記録のみを行う。コード変更・CI 変更・ADR status 変更・Issue 投稿は含まない。

## Context

- 対象 ADR: `docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md`（status `proposed`）。`### Confirmation` が「#262 の採否判定結果を本 ADR の More Information へ逆反映する」「この逆反映まで、本 ADR は GPUI の無条件採用を宣言しない」と規定し、`## More Information` 末尾に「GPUI 最終採用判定の逆反映欄（未記入）」がある。
- verdict 正本: `evidence/rf262-gpui-verdict/verdict-matrix.md`（Gate A〜E の CaseID/expected/observed/verdict/証拠 URL）、`report.md`（評価 report・既知 gap）、`handoff-419.md`（#419/#465 引き渡し文面 — 逆反映内容の起草元）。
- 決裁済み outcome: **CONDITIONAL（条件付き）**。Issue #262 Verdict 規則「全必須 OS/format で D・E が未達なら GPUI 標準 Driver 方針維持、Native Dock の無条件本番採用不可と明記し #419 を accepted にしない」に該当（Gate D Fail = GPUI 0.2.2 DComp `topmost=true` による plugin 画素非合成・`#532` open Blocker、Gate E NotRun）。
- POLICIES.md §3: 要求状態（`Required`/`Deferred`/`NotApplicable`）と実行結果（`Pass`/`Fail`/`NotRun`/`Unsupported`）は別軸。§5: ローカル検証は変更範囲に比例、docs-only 変更は Markdown/リンク等の軽量衛生チェックが原則。

## Decision overview

**記録する verdict（Coordinator 決裁済み。本 slice は適用のみ・再判定しない）**:

| 領域 | 記録する判定 | 根拠の骨子 |
| --- | --- | --- |
| GPUI Native First を標準 Driver とする設計方針 | **維持** | #262/#384 で決裁済み。Driver-neutral 公開契約は Gate A で裏付け済み |
| OS-L1（3 OS Build/Link・Headless・Feature・RT 非干渉） | **Pass（ancestor SHA の CI 証拠）** | run 43428172820/43439434737 全 lane success。HEAD SHA run は統合受入側の証拠 |
| Windows L2 | **能力別に部分実証** | Window/Input/Focus（key 配送）実測 Pass。IME=`Unsupported` 宣言、a11y/DPI 動的/物理入力=`NotRun` |
| Windows L3 / Gate D（Native Dock） | **未達・無条件本番採用不可** | HWND 機構 Pass だが DComp `topmost=true` で plugin 画素非合成（`#532` open Blocker） |
| macOS/Linux L2/L3 | **Deferred** | `native_surface_slot` backend は `Unsupported` 宣言（Capability 欠落として追跡）。NotRun≠Pass |
| Gate C 性能 / Gate E Consumer Oracle | **NotRun** | 定量比較証拠なし・consumer-verify 対象 SHA run なし |

## D1. 逆反映の記録構造

### D1.1 More Information 逆反映欄

`## More Information` 末尾の「GPUI 最終採用判定の逆反映欄（未記入）」を dated 記録へ置き換える。見出しから「未記入」を外し、以下を 1 項目の bullet + サブ箇条書きで記録する:

1. **判定 outcome = CONDITIONAL** とその根拠（Gate D Fail・Gate E NotRun、Issue #262 Verdict 規則への該当）、「判定の決裁は Coordinator が行い本欄はその結果の記録である」旨、および `evidence/rf262-gpui-verdict/` 3 ファイルへの相対リンク（`../../evidence/rf262-gpui-verdict/...`）。
2. **評価範囲の限定**: 「L1 のみ評価済み、L2/L3 非保証」を明示（OS-L1 Pass・Windows L2 部分実証・Windows L3/Gate D 未達・macOS/Linux L2/L3 Deferred・Gate C/E NotRun）。
3. **採用範囲**: GPUI 標準 Driver 方針維持・Native Dock 無条件本番採用不可・条件付き/未受入の明記・ADR status は `proposed` のまま。
4. **OS 別 Capability 記録（宣言値）**: `native_surface_slot` = Windows `Supported` / macOS・Linux `Unsupported`、`ime` = 全 OS `Unsupported`、focus release = `Err(Unsupported)` — いずれも宣言であり実測 Pass と混同しない。
5. **既知制約・将来 Owner**: `#388`/`#389`/`#532` の Windows DComp/Native Dock 未成立（Gate D 画素 Fail は `#532` が open Blocker として所有）・`#243`/`#313` の実 GUI 残件を非 Blocking の既知制約として記録。`#465` Legacy 撤去は独立判断であり L1 採用記録だけで製品 GUI 機能・実 Plugin 表示を保証しない旨を明記。仮想優先 Gate（`#630`/`#631`）証拠深度記載・正式 status 決裁は Issue #419 残 DoD として後続へ残す。

### D1.2 Confirmation への dated 記録

`### Confirmation` の既存 bullet は書き換えず、「#262 verdict の逆反映を実施した」旨の日付付き記録を 1 項目追加する。内容は outcome（CONDITIONAL）・More Information 記録への相互参照・「逆反映後も無条件採用を宣言せず status は `proposed` のまま」・「正式 status 決裁・Issue 投稿は design authority / Coordinator が別途行う」の 4 点。既存の「この逆反映まで、本 ADR は GPUI の無条件採用を宣言しない」制約は維持する（逆反映は無条件採用の宣言ではない）。

### D1.3 frontmatter

- `updated` を当日日付（`2026-11-15`）へ更新。`id`・`created`・`status`・既存 `relations` は変更しない。
- `references` に `#419`・`#532` の Issue URL を追加（`#262` は既存）。
- Capability Matrix（宣言テンプレート表）は本 slice では原則触れない — 宣言→実測の混同を避けるため、宣言記録は逆反映欄に集約する。

### D1.4 触れない範囲

`evidence/` 全域（`rf262-gpui-verdict/` を含む）、`POLICIES.md`、`AGENTS.md`、`mise.toml`、`Cargo.*`、`crates/**`、`ui/**`、`.github/workflows/`、本 ADR 以外の `docs/adr/*`、他 slug の docs、Verification 節チェックリスト（Issue 側の受入管理）。

## D2. 判定語彙・誠実性規則

- **要求状態と実行結果の分離**（POLICIES §3）: verdict-matrix の規範に従い `Required`/`Deferred`/`NotApplicable` と `Pass`/`Fail`/`NotRun`/`Unsupported` を混同しない。ADR 記述でも `NotRun` を `Pass` に、`Unsupported`（宣言）を実測 `Pass` に読み替えない。
- **決裁と推奨の分離**: #262 slice の成果物は「推奨 verdict と根拠表」であり、最終決裁は Coordinator。本 slice は「Coordinator 決裁済みの outcome = CONDITIONAL を記録した」と書き、Worker が採否を決めたかのような記述・「推奨」を「決裁」に読み替える記述をしない。
- **条件付きの明示**: 「L1 のみ評価済み」を「無条件採用」「全面合格」「accepted」と読める表現にしない。`#532` open Blocker を解消済みのように書かない。
- **宣言と実測の分離**: `native_surface_slot`/`ime`/focus release の値は `WindowCapabilities` の宣言値であり、実測 Pass ではない。「宣言」と明記する。
- **証拠参照**: ADR（`docs/adr/`）からの相対リンクは `../../evidence/rf262-gpui-verdict/...`。個別 Case の expected/observed/verdict/証拠 URL の正本は `verdict-matrix.md` であり、ADR には集約結果のみを書く（Case 表の複写はしない）。

## D3. 文書チェーン

slug `issue-419-adr-backreflection` で spec→design→test-spec→plan→task→impl/ir→impl/exp の 7 ファイルを作成する（`issue-262-gpui-verdict` チェーンと同じ same-stem 命名・docs-only slice の先例）。front-matter の `id`/`type`/`status`/`relations` は doc-suite 規約どおり（spec=proposed・design=approved・test-spec=proposed・plan=completed・task=done・ir=completed、exp は `.md` 自由形式記録）。文書は日本語。触れた全 dir の管理 index を生成手順で再生成して同じコミットに含める。

## Risks and Trade-offs

| リスク | 影響 | 対応 |
| --- | --- | --- |
| 「推奨」を「決裁」と読み替える記述 | verdict 権限の偽装 | D2 の決裁/推奨分離規則。spec/design/test-spec/IR で一貫して「Coordinator 決裁済みの記録」と明記 |
| `NotRun`/`Deferred`/`Unsupported`/`Fail` の `Pass` 化 | L2/L3 非保証の隠蔽 | D2 の語彙分離規則。逆反映欄で各領域の result を verdict-matrix の語彙どおり記録 |
| ADR status の誤変更 | 決裁権限の侵害 | `status: "proposed"` 不変を spec/Acceptance・task Done When・review で確認 |
| Capability Matrix 行の不用意な更新 | 宣言/実測の混同 | D1.3 により原則不変。更新が本質的に必要な場合のみ declared/実測を分離したまま該当セルのみ更新し PR 本文に理由を記録 |
| index の手編集 | 生成物のドリフト | `doc-maintenance index apply`・`sync_impl_index.js` のみを mutator とし `index plan` で `in-sync` を確認 |
| Issue #419 の auto-close | 残 DoD の隠蔽 | PR 本文は `Refs #419`（`Closes` 禁止）。本 slice 完結条件に Issue Close を含めない |

## Deferred Design Concerns

- ADR `status` の正式決裁 — design authority 権限。Trigger: Issue #419 残 DoD（仮想優先 Gate 記載・`#465` 独立判断）完了後。
- 仮想優先 Gate（`#630`/`#631`）の証拠深度記載 — Issue #419 残 DoD。
- `#465` Legacy 撤去の独立判断 — L1 互換と未実 GUI 製品機能の破壊を分離。
- `#532` DComp `topmost=true` — Windows Native Dock 画素の根本 Blocker（open）。
- Windows L2 残能力・Gate C 性能・Gate E Consumer Oracle — NotRun/Deferred のまま後続へ。

## Verification Notes

- 本設計の検証は文書照合に限定する: test-spec の Rule/Example 割当・`audit_docs`/`audit_impl_record`/`audit_experiment_log`・`doc-maintenance index plan`（全 dir `in-sync`）・`sync_impl_index`・`mise exec -- prek run --files <変更ファイル>`・`mise run fmt-check`。
- Rust 系 lane（`check`/`lint`/`test`/`build`/`verify`・`*-full`・`*-msrv`・Feature Matrix・3OS 横断・実機 GUI/Native Dock/Consumer Oracle gate）は docs-only 変更のため**不要・NotRun**（POLICIES §5。統合検証は merge 後対象 SHA の CI 証拠）。
- ADR 本文の判定記述は `verdict-matrix.md`/`handoff-419.md`/`report.md` との照合で検証し、推測で補わない。
