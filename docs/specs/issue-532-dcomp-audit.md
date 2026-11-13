---
id: "SPEC-034WZDrkNL8aP1DM6LjaVq"
type: "spec"
status: "approved"
title: "Issue #532 DComp 合成制約 — 監査・再開条件・調査手順の文書化"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-532-dcomp-audit.md"
      - type: "file"
        path: "docs/designs/issue-532-dcomp-audit-design.md"
      - type: "file"
        path: "docs/designs/issue-532-dcomp-native-dock.md"
      - type: "file"
        path: "docs/plans/issue-532-dcomp-audit-plan.md"
      - type: "file"
        path: "docs/tasks/issue-532-dcomp-audit-task.md"
      - type: "file"
        path: "docs/test-specs/issue-532-dcomp-audit.md"
      - type: "file"
        path: "docs/impl/ir/issue-532-dcomp-audit.md"
      - type: "file"
        path: "docs/impl/exp/issue-532-dcomp-audit.md"
      - type: "file"
        path: "docs/discovery/README.md"
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
      - type: "file"
        path: "docs/impl/exp/README.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/discovery/README.md"
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
  implements: []
  implemented-by:
    - "docs/plans/issue-532-dcomp-audit-plan.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-532-dcomp-native-dock.md"
    - "docs/designs/issue-532-dcomp-audit-design.md"
    - "docs/designs/issue-532-dcomp-native-dock.md"
    - "docs/test-specs/issue-532-dcomp-audit.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-532-dcomp-native-dock.md"
  derived-by:
    - "docs/designs/issue-532-dcomp-audit-design.md"
    - "docs/designs/issue-532-dcomp-native-dock.md"
    - "docs/plans/issue-532-dcomp-audit-plan.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-532-dcomp-audit.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/ruviform/issues/419"
    - "https://github.com/scarecrow173/ruviform/issues/465"
    - "https://github.com/scarecrow173/synvelis/issues/176"
  defers: []
  deferred-by: []
---

# Issue #532 DComp 合成制約 — 監査・再開条件・調査手順の文書化

## What Should Be Built

Issue #532（GPUI 0.2.2 の DirectComposition `CreateTargetForHwnd(topmost=true)` が GPUI window 配下の child HWND を覆い、`NativeSurfaceSlot` 内の実 plugin 画素が画面に合成されない制約）の残件を処理する文書成果物。正本の監査記録 `docs/discovery/issue-532-dcomp-native-dock.md` に、(a) 現行 L1（Fake/`NativeSurfaceSlot` 契約・Compile/Headless 試験・公開 API/RT 非回帰）と既存 CI lane の同候補 SHA 対応付け、(b) L2/L3 利用要件発生時の再開条件と方式 A/B/C の調査手順・担当、(c) Blocked by 除外提案を記録する。実画素試験・実 Plugin・実 GUI は対象外。

## Why Now

Issue #532 の先行 Slice（DComp 制約の検知と #388 への Partial/Unsupported 記録）は PR #501 で完了済み。残る未チェック受入項目は headless に可能な監査・文書スライスであり、実機 L2/L3 調査は利用要件発生まで Deferred。L1 証拠の CI 対応付け・再開条件・調査手順を先に固定することで、将来の実機調査が「何を・どの条件で・どの順で」行うかを曖昧さなく開始できる。

## Users and Value

- Coordinator / Issue #532・#388・#262 の Owner: L1 証拠の CI 対応付けと Blocked by 状態を確認でき、実機調査の再開条件を判断できる。
- Ruviform の後続実装者: 方式 A/B/C の調査手順と契約維持条件（lease/epoch・2-phase re-dock・private 境界）を参照して実機検証を開始できる。
- Synvelis #176 の Owner: 製品実 Plugin Native Dock の OS 受入に必要な Ruviform 側の前提条件を把握できる。

## Scope

### In Scope

- `docs/discovery/issue-532-dcomp-native-dock.md` の作成（正本監査記録）。
- `docs/designs/issue-532-dcomp-native-dock.md` の作成（再開条件・方式 A/B/C 調査手順の設計記録）。
- 文書チェーン（spec/design/plan/task/test-spec/impl-ir/impl-exp）の作成。
- 触った `docs/<type>/` dir の index 再生成（`doc-maintenance` の `index apply`。`docs/impl/ir` は同 apply で再生成、`docs/impl/exp` の .md レポート一覧は手動追記）。

### Out of Scope

- 実画面・実 Plugin・実 GUI の L2/L3 試験（Windows 実機・実 VST3/CLAP/AU editor）。
- `crates/` 配下のコード変更（契約・adapter・CI workflow・Cargo manifest/lockfile）。
- GitHub の正式依存（blocked-by）の登録・削除（Coordinator 責務）。
- 方式 A/B/C の採否判定・実装（#262 ADR の入力として将来決定）。
- Issue #532 自体の Close。

## Requirements

- R1: 現行 L1 で必要な Fake/`NativeSurfaceSlot` 契約・Compile/Headless 試験の結果と公開 API/RT 非回帰を、既存 CI（Windows/macOS/Linux の利用可能 lane）の同候補 SHA に対応付けて記録する。実画素試験は要求しない。
- R2: L2/L3 利用要件が発生した場合の再開条件（対象 OS・実 Plugin・Window/DPI・対応目的・測定環境）と、方式 A（GPUI 内最小修正）→B（Windows private 合成）→C（代替 Native 経路）の調査手順・担当（#388/#389/SV #176）を記録する。
- R3: 本 Issue を現行リリース/GPUI 採用/Legacy 撤去の Blocked by から除外する提案を記録する。実 Native Dock 未成立を Pass/Unsupported（未確証）に偽装しない。
- R4: Windows L2/L3（利用機能のみ必須）: Windows で外部 Plugin Native Dock を利用する場合、実 VST3/対象形式による DComp 画素合成・入力・Focus・Resize・ReDock を L2/L3 必須として扱い、未解消なら当該機能の受入は Fail/Blocked とする。利用しない形式・OS まで必須化しない。
- R5: macOS/Linux L1 のみ必須: 両 OS の既存 CI で Native Build/Link・必要 Feature/Headless/RT を対象 SHA で検証。実 GUI/物理 Device/実 Plugin は使用開始時まで Deferred（NotRun≠Pass）。
- R6: 検証 Scope: Windows でも未利用デバイス・Plugin 形式・追加環境は必須とせず、採用/利用する機能・OS/Device/Format・Run URL/Oracle を明記。新規 CI Workflow は作らない。

## Acceptance Criteria

- [ ] `docs/discovery/issue-532-dcomp-native-dock.md` が正本として作成され、F1（制約）・F2（L1 被覆棚卸し）・F3（CI lane 対応付け）・F4（依存状態）・再開条件・方式 A/B/C 手順・Blocked by 除外提案を含む。
- [ ] `docs/designs/issue-532-dcomp-native-dock.md` が再開条件と方式 A/B/C 調査手順の設計として作成される。
- [ ] 文書チェーン（spec/design/plan/task/test-spec/impl-ir/impl-exp）が front matter relations で相互に解決可能。
- [ ] 触った `docs/<type>/` dir の index が `doc-maintenance` で再生成される（`docs/impl/ir` は `index apply` で再生成、`docs/impl/exp` の .md 一覧は手動追記）。
- [ ] `mise run fmt-check` と `mise exec -- prek run --files <変更ファイル>` が exit 0。
- [ ] 実 Native Dock 未成立が「未確証」として明記され、Pass/Unsupported への偽装がない。

## Deferred Design Concerns

- 方式 A/B/C の技術的採否と実装設計 | L2/L3 実機調査が前提 | Windows L2/L3 利用要件発生時 | 採否未決のまま実装に入ると契約違反のリスク。
- `ruviform-poc-native-dock` の `gpui`/`vst3` feature の CI compile 追加 | 既知ギャップ・別 Issue 判断 | feature matrix 見直し時 | consumer 層 PoC の compile/link 証拠が手動 lane のみのまま。
- `consumer-verify.yml` の Consumer Oracle run | workflow_dispatch 専用・本 SHA run なし | 次回 dispatch 時 | Consumer 側の統合検証が NotRun のまま。

## Sources and Relations

- `docs/discovery/issue-532-dcomp-native-dock.md` — 正本監査記録（derives-from）。
- `docs/designs/issue-532-dcomp-native-dock.md` — 再開条件・方式 A/B/C 調査設計。
- Issue #532/#388/#389/#262/#419/#465・Synvelis #176/#114 — 関連 Issue。
- `evidence/rf388-native-dock-windowing/report.md`・`evidence/rf388-os-l1-ci/report.md` — 実機・CI 証拠。
