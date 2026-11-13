---
id: "TSPEC-nB3vC7xZ0qW5eR8tY1uI4o"
type: "test-spec"
status: "approved"
title: "Issue #306 IO E2E — OS-L1 CI 証拠と Case 補完 試験仕様"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-306-io-e2e-cases.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-306-io-e2e-cases.md"
    - "docs/designs/issue-306-io-e2e-cases.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-306-io-composition-e2e.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-306-io-e2e-cases.md"
    - "docs/designs/issue-306-io-e2e-cases.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-306-io-e2e-cases.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 IO E2E — OS-L1 CI 証拠と Case 補完 試験仕様

## Feature / Rule / Example

### F-1: 既存 CI の OS-L1 実走証拠（検査・非 test）

- **Rule**: io 系 crate の Native Compile/Link・Headless
  Test の Pass は、CI run の job conclusion ではなく
  job 生ログの test 実行行（crate/binary・passed/failed/
  ignored）で判定する。`deflate` 等の feature 差異は
  lane 単位で分離記録する。
- **Example**: run `43428172820`・SHA `e93c438a` で
  verify 3 OS・compression/structured feature lane・
  msrv lane の io 系 test が 0 failed で実行されたことを
  `evidence/rf306/ci-os-l1-report.md` に記録。

### F-2: post-seal temp rewrite（追加 test）

- **Rule**: seal→publish 間の外部 temp rewrite は publish
  で再検証されず、destination に改竄 bytes が publish
  される（seal 時点保証の境界）。
- **Example**: `post_seal_temp_rewrite_is_not_reverified_
  before_publish` — `std::fs::write(temp, tampered)` 後の
  publish が Ok で `read(dest) == tampered`。

### F-3: post-seal temp delete（追加 test）

- **Rule**: seal→publish 間の外部 temp delete は
  `PublishEffect::NotPublished` で失敗し destination を
  作らず、recovery handle の cleanup が absent temp で
  成功する。
- **Example**: `post_seal_temp_delete_fails_publish_not_
  published` — `Durability::BestEffort` で publish stage
  の fault に限定。

### F-4: recovery cleanup failure→retry（追加 test）

- **Rule**: cleanup 失敗時 `CleanupFailure` が recovery
  handle を返却し、状態復元後の retry `cleanup()` が
  `TempState::Absent` で成功する。
- **Example**: `recovery_cleanup_failure_returns_handle_
  for_retry` — temp を directory に置換して cleanup 失敗
  → handle 返却 → 復元 → retry 成功。incumbent
  destination 無傷。

## 検証方法

- `mise run test ruviform-io-e2e`（canonical）で
  80 tests / 0 failed を確認。3 OS CI（verify lane）での
  実走は merge 後の対象 SHA の CI 証拠に委譲。
- F-1 は test ではなく証拠文書の検査項目。

## Non-Goals

- Indeterminate・TempState::Unknown・dir-sync 失敗の e2e
  （公開 API 到達不能・mock 正本が canonical）。
- rt_ 契約 test（io-e2e は非 RT・rt_ filter 非混入を
  lane 実測で確認済み）。
- post-seal 改竄の検出強化（io-file 設計判断・別 slice）。
