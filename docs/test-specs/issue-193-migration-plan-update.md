---
id: "TSPEC-034WZ8iKotT2AuzW4LcRZy"
type: "test-spec"
status: "approved"
title: "Issue #193 Migration Plan Update Snapshot 検証"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/193"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-193-migration-plan-update.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-193-migration-execution-plan.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-193-migration-plan-update.md"
  derived-by: []
  verifies:
    - "docs/designs/issue-193-migration-plan-update-design.md"
    - "docs/designs/issue-193-migration-execution-plan.md"
  verified-by: []
  references:
    - "evidence/rf193/issue-state-census-2026-11-14.md"
  defers: []
  deferred-by: []
---

# Issue #193 Migration Plan Update Snapshot 検証

## Purpose

本 test-spec は Issue #193 残件「版付き Snapshot」の
docs 成果物が、spec R-1〜R-7 と設計の Snapshot 規則を
満たすことを検証可能な形で固定する。実装・実 OS・
Consumer 受入は各 Owner Issue の責務であり、本 spec は
それらを検証しない。

## Feature

計画正本 2 文書への `Snapshot-2026-11-14` 節の追記と、
Issue 状態 census 証拠により、M0〜M7 の着手条件・
Headless Slice・Owner/Gate が Issue URL 付きで確定し、
実 OS GUI/Device/Plugin Gate を一律 Blocked by とする
DAG 矛盾が解消されていること。

## Rules

- R-SNAP-1: Snapshot 節は版識別子（`Snapshot-2026-11-14`）
  と基準日を持ち、既存節・表・受入条件を改変しない
  追記型である（spec R-1）。
- R-SNAP-2: M0〜M7 全 Milestone に、開始条件・着手可能
  Headless Slice・完了定義・Integration Gate・責務・
  Blocked by・独立 Owner Issue の 7 項目が Issue URL
  付きで存在する（spec R-2）。
- R-SNAP-3: 各 Milestone の OS-L1 必須範囲と統合 Owner
  が明示され、L2/L3 を要する製品利用条件は
  Deferred/Activation として記録され、計画 Snapshot
  完了と将来実機受入が分離されている（spec R-3）。
- R-SNAP-4: GPUI（#262/#388/#419）・IO（#288/#306）・
  OBS（#275/#281）・Cross-Consumer（#219/#294）・
  Script（#558〜#560）・Creative Branch（SV #277〜#296・
  RF #562）・AL/SV Issue の状態が `gh` 実測に基づき、
  Closed/Deferred/NotRun を実装 Blocked by として
  誤記していないことが検証記録として存在する（spec R-4）。
- R-SNAP-5: AL（f64/長尺/CLI）と SV（f32/DAW/RT）の
  別 Oracle、候補 SHA tuple、既存 Daily/手動 CI、
  OS 別実走証拠の M0〜M7 受入への接続が記録され、
  片方の Consumer Pass を他方へ流用しない（spec R-5）。
- R-SNAP-6: DAG/Owner 表が実装順序の正本として維持さ
  れ、実 Device/OS Native Dock/両 Consumer 実測の未完了が
  Open Gate Issue（#251/#253/#388/#281/#294 等）へ
  接続されている（spec R-6）。
- R-SNAP-7: 差分は編集許可範囲内の文書・証拠・生成
  index のみであり、コード・CI・Issue 本文・GitHub
  依存関係に変更がない（spec R-7）。

## Examples

- Given `gh` で確認した全 Open Issue の
  `着手条件（Blocked by）` が「なし」であること、
  When Snapshot の各 Milestone Blocked by を記述する、
  Then 全 Milestone の Blocked by は「なし」となり、
  #262/#388/#419・#288/#306・#275/#281・#219/#294 は
  Integration Gate/独立 Owner Issue 側にのみ現れる。
- Given #288・#310〜#312・#313・#314・#243・#384〜#387・
  #390・#598・SV #13〜#16・#175・AL #143/#166/#206 が
  CLOSED であること、When M0〜M7 の完了定義を記述する、
  Then これらを「未着手」「Blocked」と記述せず、後続の
  Open Gate（#306・#281・#294 等）を接続先とする。
- Given AL #127（f64/長尺/CLI）と SV #28（f32/DAW/RT）
  が別 Oracle の Owner であること、When M0〜M7 の
  Consumer 証拠接続を記述する、Then 両 Consumer を
  独立に列挙し、候補 SHA tuple（RF #294 運用）を
  接続先とする。
- Given #419（GPUI 最終採用）が OPEN であること、
  When M1〜M6 の headless 作業の Blocked by を記述
  する、Then #419/#262 を着手 Blocker とせず、GUI lane
  の Acceptance 側にのみ位置付ける。

## Guarantees

- 計画正本 2 文書の Snapshot 節の内容が一致し、
  版識別子・証拠パス・Issue URL が解決可能である。
- `evidence/rf193/issue-state-census-2026-11-14.md` に
  Issue 状態の実測方法・対象 Issue・Open/Closed 分類・
  Blocked by 照合結果が記録されている。
- 本タスクの文書チェーン（spec/design/task/test-spec/
  ir/exp）が存在し、front matter relations が相互に
  解決される。
- `git status`/`git diff` で差分が許可範囲内のみで
  あることが確認できる。

## Non-goals

- M0〜M7 の実装・gate 受入・実 OS/実 Device/実 Plugin
  の検証実行。
- GitHub 正式 blockedBy の登録・削除（Coordinator 責務）。
- Issue #193・各 Gate Issue の Close 判定。
- `crates/`・`mise.toml`・`.github/workflows/`・
  `Cargo.toml`/`Cargo.lock` の変更。

## Risk

- Snapshot が陳腐化すると、Closed 済み項目の未着手扱いや
  受入 Gate の着手 Blocker 誤記が再発する。版識別子と
  census 証拠で基準日を固定し、Issue #193 本文更新時の
  同期義務を維持することで緩和する。
- Snapshot を計画再定義や受入 Pass と誤読するリスクは、
  「追記型の状態記録」「M0〜M7 定義を変更しない」
  「実機受入は Deferred/Activation 先の責務」の明記で
  緩和する。
