---
id: "IMPL-034WSQTRYaDQlCGdESELFh"
type: "impl"
status: "completed"
title: "Issue 388 OS L1 CI evidence"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/388"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-388-os-l1-ci-evidence.md"
      - type: "file"
        path: "evidence/rf388-os-l1-ci/report.md"
      - type: "file"
        path: "evidence/rf388-os-l1-ci/handoff-262-419.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/ci-os-l1-gpui-macos.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue 388 OS L1 CI evidence

## Summary

Issue #388 の OS-L1 受入 slice として、対象 SHA `e93c438adc4af5b498e10926fe859ac1338ac390`（main HEAD・PR #599 merge）に対する既存 Full CI run `43428172820`（`ci.yml` workflow_dispatch、conclusion=success、`ci-summary` verdict=PASS）の証拠を照合し、`evidence/rf388-os-l1-ci/report.md` に受入条件 ↔ lane/test ↔ Run URL ↔ verdict の対応表を記録した。併せて #262/#419 向け引き渡し文面を `evidence/rf388-os-l1-ci/handoff-262-419.md` に作成した。コード変更はなく、文書のみの slice である（文書チェーンは本 IR に集約 — spec/design 等の重複生成は行わない）。

## Context

Issue #388（RF-GPUI-05 NativeSurfaceSlot / GPUI Dock 実 GUI 埋め込み PoC・採用 Gate）の現行受入条件のうち、本 slice が担うのは「Windows/macOS/Linux の既存 CI 内で公開契約・GPUI/OS private handle 境界・Fallback/Detach/RT 非干渉の Compile/Link/Headless 試験を対象 SHA で照合」「macOS/Linux L1 のみ必須」「検証 Scope 明記・新規 Workflow 不作成」「#262/#419 への L1 完了可否と実 Native Dock 非保証の引き渡し」の 4 項。Windows L2/L3 実 Plugin Dock 画素（#532 DComp 決裁待ち）と実 Plugin 受入（SV #176/#389 Owner）は対象外で、Issue は open 継続が前提。検証レベルの規範は `POLICIES.md` §2〜§5 に従う。

## Implementation

### Changes

#### `evidence/rf388-os-l1-ci/report.md`（新規）

- Full CI run `43428172820` の全 20 job（19 lane job + `ci-summary`。`needs` の 15 declared lane のうち 4 件は 2 OS matrix で 19 leg に展開）を一覧化。
- `checkout-sha-*` artifact 19 leg を全件取得し、実測 `git rev-parse HEAD` が対象 SHA `e93c438a…` で一致することを確認（`ci-summary` の event/summary/workflow 3 者一致も確認）。
- 受入条件 4 項それぞれに対し、lane/test 名・実行 OS・観測結果（cargo test の `test result` 集計・step success）・verdict（Pass/Deferred/NotRun）を対応表化。個別 Case の Pass は run log でテスト名・件数を確認できるものに限定し、CI green を個別 Case Pass と混同しない（POLICIES §4）。
- 採用 Feature・OS/Device/Format・Run URL/Oracle を「検証スコープと Oracle」節に明記（AC「検証Scope」対応）。
- 既知ギャップとして `ruviform-poc-native-dock` の `gpui`/`vst3` feature が CI で compile されない点を注記（修正はしない — 本 slice 範囲外）。
- NotRun/Deferred 行（Windows L2/L3 実 Plugin Dock、macOS/Linux 実 GUI/物理 Device/実 Plugin、consumer-verify の本 SHA run なし）を Pass と分離して列挙。

#### `evidence/rf388-os-l1-ci/handoff-262-419.md`（新規）

- #262/#419 への引き渡し文面。L1 完了可否（OS-L1 は 3 OS で Pass）・実 Native Dock 非保証（DComp 画素問題 #532 未解決・実 Plugin Dock は Deferred）・Floating/Generic/Mock を実 Native Dock Pass として扱わないことを明記。Issue コメント投稿は Coordinator が行うため文面のみを成果物とする。

#### `docs/impl/ir/issue-388-os-l1-ci-evidence.md`（本書）

- 実施内容・証拠・判定を IR として記録。

### Decisions During Implementation

- **証拠 run の選定**: タスク前提の dispatch run `43428172820` が唯一の同 SHA run であり、完了を待って採用した（`gh run watch` で `ui-gpui-verify (windows-latest)` の完了を確認後、全 job + `ci-summary` PASS を取得）。同一 SHA の別 completed run は存在しなかった。
- **case-level 証拠の粒度**: job/step success に加え、主要 lane（`ui-gpui-verify` ubuntu/macos/windows・`rt-verify`・`msrv-verify`・`ui-gpui-offscreen-verify`・`verify (windows-latest)`）の job log を `gh api .../jobs/<id>/logs` で取得し、`native_slot_contract` 13 件・`test-ui-gpui` の `windowed_test_app` bin compile/link・rt-verify の strict `rt_` 166 tests・offscreen の real_window 3 tests 等の実測値を report に記録した。
- **文書チェーンの集約**: 本 slice は証拠記録が主体のため、共通契約の「spec → design → plan → task」完全チェーンは生成せず IR を正とした（タスク仕様の許容どおり）。
- **branch**: Orca worktree の既定 branch `scarecrow173/rf388-evidence` から、タスク指定の `docs/388-os-l1-ci-evidence` を作成して作業した（同一 SHA `e93c438a` 起点・コード差分なし）。

## Related Experiments

### Adopted

- なし（実験 log は作成していない — 証拠収集と文書作成のみの機械的 slice）。

### Rejected

- なし。

## Validation

実施済み:

- `gh run view 43428172820` / `gh api repos/.../actions/runs/43428172820/artifacts` / `gh api .../jobs/<id>/logs` — run 状態・job 結果・artifact・log の実測収集（補助証拠: GitHub API 実測。canonical な検証は CI run 自体）。
- `gh run download 43428172820 -p 'checkout-sha-*'` → 19 leg 全件の実測 SHA が `e93c438adc4af5b498e10926fe859ac1338ac390` で一致することを確認。
- `ci-summary` job log で verdict=PASS・15 declared lane 全 Pass・19 checkout leg の sha verdict=Pass を確認。
- `gh run list --workflow consumer-verify.yml` で本 SHA の Consumer Oracle run がないこと（最新は別 SHA `3692c6fa`）を確認し、NotRun として記録。
- `mise.toml` / `.github/workflows/ci.yml` / `crates/ruviform-poc-native-dock/Cargo.toml` の静的確認で、採用 feature・lane 構成・poc feature gap を裏付け。
- 文書のみの slice のため Workspace build/test は未実施（`POLICIES.md` §5: Markdown/リンクの軽量チェックが原則）。prek pre-commit hook（`prek run --all-files` 相当の commit 時 hook）で Markdown/構成ファイルの軽量チェックを実施予定。
- `sync_impl_index.js --kind ir` で `docs/impl/ir/README.md` を reconcile 予定。

## Risks

- **証拠の経時劣化**: `checkout-sha-*` artifact の retention は 30 日。report は run URL と実測値を本文に転記済みのため、artifact 失効後も判定根拠は残る。
- **poc-native-dock feature gap**: `gpui`/`vst3` feature の CI compile 欠落は既知事項として report に注記したが、consumer 層 PoC の feature-enabled 経路は CI 証拠がない状態が継続する（修正は別 Issue 判断）。
- **ci-summary verdict の解釈**: verdict=PASS は「宣言 lane が全て success で対象 SHA が一致」の fail-closed 判定であり、個別 Case（例: 実 Dock）の Pass を意味しない。本 report はこの区別を維持した。

## Follow-ups

- Coordinator による #262/#419 への引き渡し文面の Issue コメント投稿（`handoff-262-419.md` の内容を使用）。
- Issue #388 の受入判定（Coordinator が証拠を見て実施。Windows L2/L3 残件があるため open 継続見込み）。
- `ruviform-poc-native-dock` の `gpui`/`vst3` feature を CI feature matrix に載せるかの判断（別 Issue）。
- `docs/impl/ir/README.md` index の reconcile（`sync_impl_index.js --kind ir --apply`）。
