---
id: "TASK-034UwcoMpAnO2xGUgLMC7x"
type: "task"
status: "done"
title: "W2 Issue #305 io-e2e WAV golden/fault 検証・handoff"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/305"
  changes:
    added:
      - type: "file"
        path: "evidence/rf305/wav-golden-fault-report.md"
      - type: "file"
        path: "docs/impl/ir/issue-305-io-e2e-wav-byte-golden-fault-fixtures.md"
    modified:
      - type: "file"
        path: "docs/specs/issue-305-io-e2e-wav-golden-fault.md"
      - type: "file"
        path: "docs/designs/issue-305-io-e2e-wav-golden-fault.md"
      - type: "file"
        path: "docs/test-specs/issue-305-io-e2e-wav-golden-fault.md"
      - type: "file"
        path: "docs/plans/issue-305-io-e2e-wav-golden-fault.md"
      - type: "file"
        path: "docs/tasks/issue-305-w1-fixture-extension.md"
      - type: "file"
        path: "docs/tasks/issue-305-w2-verification-handoff.md"
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
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-305-io-e2e-wav-golden-fault.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-305-io-e2e-wav-golden-fault.md"
    - "docs/tasks/issue-305-w1-fixture-extension.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "evidence/rf306/fault-fuzz-report.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/test-specs/issue-305-io-e2e-wav-golden-fault.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-305-io-e2e-wav-golden-fault.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# W2 Issue #305 io-e2e WAV golden/fault 検証・handoff

## Work

W1 の fixture を正規 lane で検証し、証拠と handoff を記録する。

- `mise run fmt-check` / `check-io-e2e` / `lint-io-e2e` /
  `test-io-e2e` / `check` / `lint` / `test` / `test-rt` を実行し
  exit status を記録する。
- 補助証拠（canonical = mise、補助と明記）: `cargo tree
  -p ruviform-media-io` で Domain Core3 のみ依存を確認、
  `StagedOutput`/media-io 旧 `PublishOptions` の workspace 参照
  grep ゼロを確認。
- `evidence/rf305/wav-golden-fault-report.md` に実コマンド・
  実値・OS/FS 実測範囲・NotRun/Unsupported・旧 API 証拠・
  DAG 確認・#294/#219 handoff 条件を記録する。
- `docs/impl/ir/issue-305-io-e2e-wav-golden-fault.md` に実装記録
  を作成し、doc status/index を更新する。
- review-protocol で docs+code を再レビューし、blocker がない
  ことを確認する。`git fetch origin main && git merge
  origin/main` で最新を取り込み、生成物を再生成してから push・
  PR 作成（日本語・`Refs #305`・Issue は Close しない）。

## Classification

- `normal-plan-task`

## Upstream Documents

- [docs/plans/issue-305-io-e2e-wav-golden-fault.md](../plans/issue-305-io-e2e-wav-golden-fault.md)
- [docs/test-specs/issue-305-io-e2e-wav-golden-fault.md](../test-specs/issue-305-io-e2e-wav-golden-fault.md)
- [docs/tasks/issue-305-w1-fixture-extension.md](../tasks/issue-305-w1-fixture-extension.md)
- `evidence/rf306/fault-fuzz-report.md`（evidence 形式の先例）

## Dependencies

- W1 fixture 実装の完了。

## Affected Files

- `evidence/rf305/wav-golden-fault-report.md`（新規）
- `docs/impl/ir/issue-305-io-e2e-wav-byte-golden-fault-fixtures.md`（新規）
- `docs/{specs,designs,plans,test-specs,tasks,impl/ir}/README.md`
  （index 再生成）

## Done When

- [ ] 全正規検証 lane が pass（実 exit status を evidence に記録）
- [ ] 旧 API 非存在・DAG 維持の補助証拠を記録
- [ ] NotRun/Unsupported（macOS/Linux・Published/Indeterminate
      失敗 effect・DirSync 後失敗・TempState::Unknown・実 4GiB
      ファイル publish）が正直に列挙される
- [ ] review-protocol で blocker なし・origin/main merge 済み・
      PR 作成済み（`Refs #305`）

## Verification

- [ ] `mise run fmt-check` / `check` / `lint` / `test` /
      `test-io-e2e` / `check-io-e2e` / `lint-io-e2e` / `test-rt`
      全 pass
