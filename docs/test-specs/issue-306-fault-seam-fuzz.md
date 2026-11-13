---
id: "TSPEC-034WXOTmrti91Jpbt5kIxV"
type: "test-spec"
status: "approved"
title: "Issue #306 G07/G08 fault seam・corpus replay テスト仕様"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/fault_seam.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/corpus_replay.rs"
      - type: "file"
        path: "evidence/rf306/fault-seam-fuzz-report.md"
      - type: "file"
        path: "docs/test-specs/issue-306-fault-seam-fuzz.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-306-fault-seam-fuzz-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines:
    - "docs/test-specs/issue-306-io-e2e-fault-fuzz.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-306-fault-seam-fuzz.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-306-fault-seam-fuzz.md"
    - "docs/designs/issue-306-fault-seam-fuzz-design.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 G07/G08 fault seam・corpus replay テスト仕様

## Purpose

Issue #306 G07/G08 の受入を保証するテスト仕様。G07 は publish
fault 分類契約を公開 API 経由で決定的に検証し、G08 は bounded・
再現可能な corpus replay の入口を保証する。実 OS で誘発不能な
class は `Unsupported` のまま残し Pass へ繰り上げない。

## Feature

```gherkin
Feature: Fault seam contract and bounded corpus replay
  publish pipeline の分類契約と、reader への corpus replay が
  決定的・bounded に成立すること。
```

## Rules

- G07-R1: `ApplyOutcome::Unknown`（publish report = Unknown）は
  `PublishEffect::Indeterminate` に分類され、temp は自動削除
  されず `RecoveryHandle` が所有権を保持する。
- G07-R2: `Applied` 後の `sync_directory` 失敗は
  `PublishEffect::Published` + `PublishPhase::DirectorySync` +
  achieved `DurabilityEvidence::FileSynced` で報告される
  （`NotPublished` に偽装しない）。
- G07-R3: `TempDisposition::Unknown`（cleanup/inspect 経路）は
  `TempState::Unknown` に伝播し、`Absent`/`Retained` へ誤分類
  しない。
- G07-R4: validate/sync_file/prepare/readback/publish/directory
  sync の各 fault は対応する `PublishPhase` +
  `PublishEffect::NotPublished`（publish 不明時を除く）+
  `PublishFailureKind` で型付き報告される。
- G07-R5: `Indeterminate` temp は `RecoveryHandle` の
  inspect/cleanup が明示的に成功し、temp bytes が残存/消滅の
  期待どおりになる。
- G07-R6: seam は opt-in feature + `#[doc(hidden)]` のみで、
  既定 feature build・公開 API signature・RT 面を変えない。
- G08-R1: corpus replay は固定 seed・case 数上限・byte/decoded
  byte budget・deadline を持ち、同一 seed で決定的に再現する。
- G08-R2: replay 対象は writer 正規 bytes を種にした mutation
  corpus で、各 reader は panic せず Ok または typed error
  （budget 拘束含む）で終了する。
- G08-R3: 形式別除外（deflate/WAV/secure extraction/publish FS
  経路/持続 fuzz）は理由付きで記録され、NotRun/Unsupported を
  Pass に繰り上げない。
- G08-R4: 通常 `test-io-e2e` lane では実行されず、opt-in task
  経由でのみ走る。

## Examples

- G07 publish report=Unknown・物理適用あり → `Indeterminate`、
  temp Retained、RecoveryHandle 経由で cleanup 成功。
- G07 publish=Applied・sync_directory_error → `Published` +
  `DirectorySync` phase の failure、dest bytes は publish 済み。
- G07 cleanup report=Unknown → `TempState::Unknown` が
  publish failure / recovery inspect の双方で観測される。
- G08 seed 固定 → 同一 corpus で同一 outcome histogram、
  deadline 超過時は fail-loud。

## Guarantees

- `cargo test -p ruviform-io-e2e --test fault_seam` が
  deterministic に pass する。
- `cargo test -p ruviform-io-e2e --test corpus_replay -- --ignored`
  が bounded に完走し、outcome histogram を出力する。
- `cargo check/clippy/test -p ruviform-io-file`（feature 無し）が
  既定 build 不変で pass する。

## Non-goals

- 実 OS での Indeterminate/DirSync 後失敗/TempState::Unknown の
  再現（OS capability 外・Unsupported 維持）。
- structured/compression/archive/wav 層への seam 波及。
- 長時間・持続 fuzz、外部 fuzzing 基盤、minimized reproducer の
  git 管理 corpus。
- G01–G03・実故障・容量/性能閾値（Issue #306 の外部 gate）。

## Risk

- seam が test-only を超えて production/公開面に漏れると、
  fault 経路が本番 build に残り契約を偽る。feature +
  doc(hidden) + `publish = false` の三重封じ込めと、feature 無し
  lane の検証で防ぐ。
- corpus replay が無制限になると opt-in lane が CI/ローカルを
  圧迫する。固定 seed・case/byte/deadline budget・`#[ignore]`
  で防ぐ。
