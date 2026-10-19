---
id: "TASK-034IRZk613ql8oL3Y2Yxli"
type: "task"
status: "done"
title: "README.md の Issue #1 ベースライン向け差分更新"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/apply-issue-1-architecture-baseline-to-repo.md"
  implemented-by: []
  depends-on:
    - "../plans/apply-issue-1-architecture-baseline-to-repo.md"
  blocks:
    - "verify-issue-1-repo-reflection.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../plans/apply-issue-1-architecture-baseline-to-repo.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# README.md の Issue #1 ベースライン向け差分更新

## Work

plan `PLAN-034IR3qOr15LGNY9mEindJ` のスライス T-README（Phase R-1〜R-5）。`README.md` の「#1 で決定」系の陳腐化した記述を、approved 済み design ベースラインの決定・採否に整合させ、design への導線と RT 契約注意書きを追加する。最終文案は design が逐語確定していないため、plan Design Inputs が示す design 引用元節（採否/分類表・RT 契約）の範囲で執筆する（plan Open Questions / Gaps）。

- [ ] R-1. 「### 未確定（#1 で決定）」節（L29–L31）を「### 決定済み・未確定」に改題して再構成する: 対応プラットフォーム（Windows/macOS/Linux・x86_64＋aarch64）は platform ADR・design で決定済みとして「決定済み」側に列挙し、公開 API・サポート形式・ライセンス・最終的なクレート分割（design に DAG 設計案として記録・実装 Issue で確定）・edition/MSRV（専用確認タスクへ defer・`Cargo.toml` 現行値は仮置き）は「未確定」側に列挙して区別して記述する
- [ ] R-2. 「### 検討中（採否は #1 で決定）」節（L25–L27）のラベルと内容を design の採否に整合させる: WebView UI（UI ADR で WebView ファースト・ハイブリッドが方針決定・具体技術は要 PoC）、`wgpu`（エンジン層 opt-in adapter・採否は要 PoC）、ネイティブ UI（要 PoC）、プラグイン連携（plugin ADR・design でアーキテクチャ決定・CLAP/VST3 同格）の区分を反映する
- [ ] R-3. 冒頭段落の「詳細設計は #1 の後続タスクで確定します」を設計ベースライン approved の現状に更新し、「## Documentation」節に `docs/designs/ruviform-architecture-design-baseline.md` への導線（Issue #1 由来の設計ベースラインである旨）を追加する。あわせて同節の Issue #1 行（L96）「プロダクト本体の詳細設計（後続タスク）」を「詳細設計ベースライン approved・実装は後続 Issue」等の現状記述へ更新する
- [ ] R-4. RT 契約・保証範囲の注意書きを追加する: RT 契約は API 単位で定義・明示、保証は Ruviform 自身の RT 対応実装に限定、CI 成功は完全証明ではない、OS レベルのハードリアルタイム保証は提供しない旨（design「RT 契約（API 単位の保証定義）」「RT 違反検出の分担」の明記要求）
- [ ] R-5. ラベル監査を行う: 「実在・検証済み」「構想」「検討中」「決定済み」「未確定」各区分のラベルが保持され、未確定項目を確定済み・未実装を実装済みと記述していないことを確認する

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/apply-issue-1-architecture-baseline-to-repo.md](../plans/apply-issue-1-architecture-baseline-to-repo.md) — `PLAN-034IR3qOr15LGNY9mEindJ`（approved）。Phase T-README / V-4・V-5 の正本
- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — `DESIGN-034IPynWe2V30FLubzQenb`（approved）。採否/分類表・「RT 契約（API 単位の保証定義）」（L420–L425）・「RT 違反検出の分担」（L432–L437）の引用元
- [../specs/architecture-design-baseline-deliverables.md](../specs/architecture-design-baseline-deliverables.md) — `SPEC-034INYy2aDLxse9dy8MgGZ`（approved）。Requirements B の確定制約と禁止記載の正本（V-5）
- [../adr/platform-and-toolchain-targets.md](../adr/platform-and-toolchain-targets.md) — `ADR-034IO01ZSFQcu2jEt2p0Tb`（accepted）。対応プラットフォーム決定の根拠
- [../adr/ui-architecture-webview-hybrid.md](../adr/ui-architecture-webview-hybrid.md) — WebView ファースト・ハイブリッド方針の根拠
- [../adr/plugin-architecture-clap-vst3.md](../adr/plugin-architecture-clap-vst3.md) — CLAP/VST3 同格の根拠
- [../adr/realtime-contract-and-verification-policy.md](../adr/realtime-contract-and-verification-policy.md) — `ADR-034IO00JjhxeTXUbCZtcDX`（accepted）。RT 契約・CI 成功は完全証明でない旨の根拠

## Dependencies

- 共有前提: plan S-1（変更前ベースライン記録）はいずれのタスクにも属さない pre-dispatch ステップとして、本タスク着手前に実施済みであること
- 後続をブロック: `verify-issue-1-repo-reflection.md`（V-4・V-5 の内容監査）
- T-PUB・T-IDX とは互いに独立（S-1 後に並行可能）。T-AGENTS とも相互独立だが、両方の完了が T-VERIFY の前提
- design の採否・分類表に載っていない項目を誤って確定/棄却扱いしない。対応付け不能な項目は「未確定」側に残す

## Affected Files

- `README.md` — 「未確定」節再構成・「検討中」節ラベル整合・冒頭案内と Documentation 節の導線更新・RT 契約注意書き追加（section-edit）

## Done When

- [ ] 「決定済み・未確定」の区別（対応プラットフォーム=決定・公開 API/サポート形式/ライセンス/最終クレート分割/edition/MSRV=未確定）が design の採否分類と一致している
- [ ] 「検討中」節の各項目が design の採否・ADR 決定（WebView ファースト・wgpu opt-in・ネイティブ UI 要 PoC・CLAP/VST3 同格）に対応付けられている
- [ ] design ベースラインへの導線と Issue #1 行の現状記述が追加されている
- [ ] RT 契約注意書き（API 単位保証・CI 成功≠完全証明・OS ハード RT 保証なし）が記載されている
- [ ] 未確定項目を確定済み・未実装を実装済みと記述した箇所がない

## Verification

- [ ] `git grep -n "#1 で確定\|#1 で決定\|後続タスク" -- README.md` が一致を返さない（S-1 ベースラインと比較）
- [ ] `grep -n "ruviform-architecture-design-baseline" README.md` が design への導線を返す
- [ ] Manual: design 採否/分類表との突合（V-4）および spec Requirements B との整合・禁止記載なし（V-5）の目視監査
