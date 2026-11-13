---
id: "TSPEC-034WYgclyB5Fcmkva75gsf"
type: "test-spec"
status: "approved"
title: "Issue #192 TA Invariant Map — 対応表・非循環監査の検証規約"
created: "2026-11-13"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/192"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-192-ta-invariant-map.md"
      - type: "file"
        path: "docs/specs/issue-192-ta-invariant-map.md"
      - type: "file"
        path: "docs/designs/issue-192-ta-invariant-map-design.md"
      - type: "file"
        path: "docs/plans/issue-192-ta-invariant-map-plan.md"
      - type: "file"
        path: "docs/tasks/issue-192-ta-invariant-map-task.md"
      - type: "file"
        path: "docs/test-specs/issue-192-ta-invariant-map.md"
      - type: "file"
        path: "docs/impl/ir/issue-192-ta-invariant-map.md"
      - type: "file"
        path: "docs/impl/exp/issue-192-ta-invariant-map.md"
    modified:
      - type: "file"
        path: "docs/designs/issue-192-synvelis-driven-target-architecture-migration.md"
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
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
  implements: []
  implemented-by: []
  depends-on:
    - "docs/plans/issue-192-ta-invariant-map-plan.md"
    - "docs/specs/issue-192-ta-invariant-map.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-192-ta-invariant-map.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-192-ta-invariant-map-design.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-192-ta-invariant-map.md"
    - "docs/designs/issue-192-ta-invariant-map-design.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #192 TA Invariant Map — 対応表・非循環監査の検証規約

## Purpose

Issue #192 TA Invariant Map の文書成果物（discovery 正本対応表・依存
非循環性監査節・索引側参照・文書チェーン）が、spec の Acceptance Criteria
を満たすことを検証する規約を定める。文書監査タスクであり、検証対象は
Markdown 文書とその front matter relations であって、コードの振る舞いでは
ない。本規約は実機・実 Consumer・CI 実走の受入を検証しない（各 Open Gate
Issue の責務）。

## Feature

「TA Invariant Map 文書チェーンが、spec/design が定める監査成果物の
完全性・証拠等級の分離・relation 解決・軽量文書チェックを満たす」こと。

## Rules

- **MAP-001（網羅性）**: discovery 正本の対応表は umbrella §7 の
  TA-001〜TA-012 全 12 項目を行として持ち、各行に「現行 API/crate・
  設計判断・実装 Issue・既存 OS-L1 テスト・Headless Gap→Open Gate」の
  5 軸が記録されている。
- **MAP-002（依存監査の再現性）**: 依存非循環性監査節は対象 SHA・
  実行コマンド（`mise exec -- cargo metadata`、default/--all-features）・
  exit code・観測値（member 数・edge 数・循環数・target 条件 edge 数）・
  違反有無を記録し、manifest 宣言と resolver 活性を区別する。
- **MAP-003（証拠等級の分離）**: 「実測（cargo metadata・grep 棚卸し）」
  「既存テスト記録（ファイル・テスト名の引用）」
  「未実行（実機/実 Plugin/実 Consumer/CI 実走）」が文中で区別され、
  未実行を実行済みと記載しない。
- **MAP-004（Gap の接続）**: 実機・実 Consumer・CI 実走に依存する未達項目
  は、対応表の Gap 欄で具体的な Open Issue 番号へ接続され、本スライスの
  完了条件に含めない旨が明記される。
- **MAP-005（索引参照）**: `docs/designs/issue-192-synvelis-driven-target-architecture-migration.md`
  に discovery 正本への参照が追記され、既存の設計判断・capability map・
  Owner 記述は変更されない。
- **MAP-006（文書チェーン）**: spec→design→plan→task→test-spec→
  impl-ir/impl-exp の front matter relations が相互に解決し、status が
  各 type の規約（spec=proposed、design/plan=approved、task=todo→done、
  test-spec=draft、impl=in-progress→completed）に整合する。
- **MAP-007（軽量検証）**: 変更ファイルに対し `mise run fmt-check` と
  `mise exec -- prek run --files <変更ファイル>` が exit 0 で通り、
  doc-maintenance による index 再生成と doc-status 監査が整合する。

## Examples

- **MAP-001 の例**: TA-010 の行に `ruviform-io-wav`（`WavEncoder`/
  `WavMuxer`/`WavReceipt`）・`media-io-backend-abstraction` ADR・
  #191/#198/#287/#305・`io-wav/tests/roundtrip.rs` の往復テスト名・
  「IO E2E → #306 OPEN」が記録されている。
- **MAP-002 の例**: 依存監査節に「default: ws→ws normal+build 98 /
  dev 26 / 循環 0、all-features: 103 / 26 / 0、target 条件 path 依存 0、
  違反なし」と SHA・コマンド付きで記録されている。
- **MAP-003 の例**: TA-012 の「L1 テスト」列は `legacy_golden.rs` と
  `test-l1-real-plugin` lane（fixture）を引用し、Gap 列は
  「両 Consumer 独立受入未完 → #219/#281/#294 OPEN」と実測でないことを
  区別している。
- **MAP-006 の例**: spec の `derives-from` が discovery を指し、
  test-spec の `verifies` が spec/design を指し、task の `verified-by`
  が test-spec を指す。

## Guarantees

- 正しい実装（文書成果物）は、TA-001〜012 の全項目について現行 API・
  Issue・既存 L1 テスト・Gap の対応を一意に追跡可能にする。
- 依存監査結果は同一 SHA・同一コマンドで再算出可能であり、違反の有無が
  明示される。
- 実機・実 Consumer 受入の未完了が Open Gate Issue へ漏れなく接続され、
  headless 監査の完了を過大に主張しない。

## Non-goals

- TA-001〜012 invariant 自体の正しさの再証明（既存テストの再実行・
  CI 実走は行わない）。
- TA gate の正式 Close 判定・Issue #192/#193 の Close。
- 実機 RT 認証・実 Plugin 互換・両 Consumer 独立受入の検証。
- `cargo metadata` 以外の build/test/lint の実行。
- コード・manifest・CI 設定の変更に対する検証。

## Risk

- 対応表が不完全だと、受入管理で「未対応の invariant」を見落とす。
- 依存監査の記録が曖昧だと、feature×dev/build×target の組合せでの
  循環や禁止方向違反を見逃す。
- 証拠等級を混同すると、headless の静的確認を実機/実 Consumer 受入の
  証拠と誤認し、POLICIES の検証レベル規範に反する。
- 文書チェーンの relations が壊れると、doc-driven-dev-graph の routing
  と index 再生成が失敗し、後続タスクの起点を失う。
