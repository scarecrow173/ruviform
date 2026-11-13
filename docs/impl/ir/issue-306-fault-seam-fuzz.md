---
id: "IMPL-034WXT9H5uOuqU6UXmcmuk"
type: "impl"
status: "completed"
title: "Issue #306 G07/G08 fault seam・corpus replay"
created: "2026-11-14"
updated: "2026-11-14"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/288"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/fault_seam.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/corpus_replay.rs"
      - type: "file"
        path: "evidence/rf306/fault-seam-fuzz-report.md"
      - type: "file"
        path: "docs/specs/issue-306-fault-seam-fuzz.md"
      - type: "file"
        path: "docs/designs/issue-306-fault-seam-fuzz-design.md"
      - type: "file"
        path: "docs/test-specs/issue-306-fault-seam-fuzz.md"
      - type: "file"
        path: "docs/plans/issue-306-fault-seam-fuzz-plan.md"
      - type: "file"
        path: "docs/tasks/issue-306-fault-seam-fuzz-task.md"
      - type: "file"
        path: "docs/impl/ir/issue-306-fault-seam-fuzz.md"
      - type: "file"
        path: "docs/impl/exp/issue-306-fault-seam-fuzz.md"
    modified:
      - type: "file"
        path: "crates/ruviform-io-file/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-file/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/mock.rs"
      - type: "file"
        path: "crates/ruviform-io-file/src/staged.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-e2e/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/README.md"
      - type: "file"
        path: "mise.toml"
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
    - "docs/tasks/issue-306-fault-seam-fuzz-task.md"
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-306-io-e2e-slice-2-fault-fuzz-fixture-os-fs-probe-rooted-boundary.md"
    - "docs/impl/ir/issue-295-ruviform-io-file-staged-publish-contract.md"
    - "docs/impl/ir/issue-296-ruviform-io-file-os-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-306-fault-seam-fuzz-plan.md"
    - "docs/designs/issue-306-fault-seam-fuzz-design.md"
    - "docs/specs/issue-306-fault-seam-fuzz.md"
    - "docs/test-specs/issue-306-fault-seam-fuzz.md"
    - "evidence/rf306/fault-seam-fuzz-report.md"
    - "evidence/rf306/fault-fuzz-report.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/test-specs/issue-306-fault-seam-fuzz.md"
  verified-by: []
  references: []
  defers:
    - "docs/tasks/issue-294-t2-consumer-verify-workflow-tuple-ui-lanes.md"
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-306-fault-seam-fuzz.md"
    rejected: []
---

# Issue #306 G07/G08 fault seam・corpus replay

## Summary

Issue #306 残件の G07/G08 を実装した。G07: `ruviform-io-file` に
opt-in `fault-injection` feature と `#[doc(hidden)]` mock facade を
追加し、`ruviform-io-e2e/tests/fault_seam.rs` で `Indeterminate`・
publish 後 DirSync 失敗・`TempState::Unknown`・phase 別
`NotPublished` を公開 API のみで決定的に検証する契約試験 8 件を
整備した。G08: `tests/corpus_replay.rs` に固定 seed・bounded な
corpus replay（`#[ignore]`）と `mise run test-io-e2e-corpus`
opt-in lane を追加した。公開 API・既定 feature build・RT 面は
不変。実 OS で誘発不能な class の `Unsupported` 判定は維持し、
mock-oracle は分類契約の検証として位置づけた。

## Context

- 上流: [spec](../../specs/issue-306-fault-seam-fuzz.md)・
  [design](../../designs/issue-306-fault-seam-fuzz-design.md)
  （approved）・[plan](../../plans/issue-306-fault-seam-fuzz-plan.md)・
  [task](../../tasks/issue-306-fault-seam-fuzz-task.md)・
  [test-spec](../../test-specs/issue-306-fault-seam-fuzz.md)
- 前段: [IR](issue-306-io-e2e-slice-2-fault-fuzz-fixture-os-fs-probe-rooted-boundary.md)
  で `Indeterminate`/DirSync 後失敗/`TempState::Unknown` を
  「実 OS で誘発不能 = Unsupported」と正直に記録した残件を、本
  slice で seam 判定 + 契約試験として回収する。
- 証拠: [evidence/rf306/fault-seam-fuzz-report.md](../../../evidence/rf306/fault-seam-fuzz-report.md)

## Implementation

### Changes

- `crates/ruviform-io-file/Cargo.toml`: `fault-injection = []`
  feature を追加（非 default・additive のみ）。
- `crates/ruviform-io-file/src/lib.rs`: `mod mock` を
  `#[cfg(any(test, feature = "fault-injection"))]` +
  `#[doc(hidden)] pub mod mock` に変更。
- `crates/ruviform-io-file/src/mock.rs`: 既存 `MockFileOps`/
  `MockScript`/`MockPublish`/`MockFs` を再利用し、外部契約試験向け
  facade を追加 — `PublishReport`/`TempReport`（`ApplyOutcome`/
  `TempDisposition` の公開語彙への写像）、`PublishScript`
  （report/temp/error/physically_applied の builder）、
  `FaultScript`（stage 別 fault の builder）、`staged_file`
  （`StagedFile::with_ops` + `MockFs` 観測 handle を返す）。
  facade の公開 signature は std 型と公開 result 型のみで、
  `PlatformFileOps`/`StageOwner`/`RecoveryInfo` 等の private 型を
  出さない。`MockFs` は観測 evidence 用に field を `pub` 化
  （module 自体が cfg/doc(hidden) ゲート内）。
- `crates/ruviform-io-file/src/staged.rs`: `StagedFile::with_ops`
  の cfg を `any(test, feature = "fault-injection")` に拡張
  （`pub(crate)`・signature 不変）。
- `crates/ruviform-io-e2e/Cargo.toml`: dev-dep
  `ruviform-io-file` に `features = ["fault-injection"]` を追加
  （test target のみに影響）。
- `crates/ruviform-io-e2e/tests/fault_seam.rs`: G07 契約試験 8 件。
  `PublishEffect::Indeterminate`・`Published`+`DirectorySync`・
  `TempState::Unknown` 伝播（publish cleanup/abort/recovery
  inspect）・phase 別 `NotPublished`（Validate/FileSync/Close/
  Publish）を検証。assert は公開 result 型と `MockFs` 観測のみ。
- `crates/ruviform-io-e2e/tests/corpus_replay.rs`: G08 bounded
  corpus replay。splitmix64 固定 seed `0x52463346555a5a45`、
  形式別 name_seed 派生、48 cases/format × 9 形式、変異は
  byte flip/truncate/extend/splice、`INPUT_CAP=256KiB`/
  `DECODED_CAP=1MiB`/`DEADLINE=120s`、panic は `catch_unwind`
  で計数し 0 以外で fail、seed 受理の sanity anchor 付き。
  `#[ignore]` で通常 lane 非実行。
- `crates/ruviform-io-e2e/src/lib.rs` / `README.md`: 新 test
  binary・opt-in lane・証拠正本（`fault-seam-fuzz-report.md`）を
  記述。
- `mise.toml`: `[tasks.test-io-e2e-corpus]` を追加
  （`cargo test -p ruviform-io-e2e --test corpus_replay --
  --ignored --nocapture`）。既存 task は無変更。
- `evidence/rf306/fault-seam-fuzz-report.md`: seam 判定の技術
  根拠・契約試験一覧・corpus replay 実測出力・検証 lane 実値・
  NotRun/Unsupported 一覧を記録。

### Decisions During Implementation

- seam の置き場所は `ruviform-io-file` 内の `mock.rs` に限定し、
  facade は同 crate 内で crate-private 型を公開 signature に
  出さない構造とした（`private_interfaces` lint を構造的に回避）。
- `MockPublish.physically_applied` を report から独立した軸として
  残し、「Unknown 報告だが物理的には適用済み」という正直な
  モデルを可能にした（`Indeterminate` + dest 適用済みの検証に使用）。
- corpus replay の panic は握り潰さず `catch_unwind` で計数し、
  合計 0 を assert（panic = 欠陥の検出であり reject ではない）。
  panic hook は計数中のみ無音化し必ず復元する。
- XML seed は writer が存在しない read-only API のため固定
  canonical document とし、その旨をコードコメントに明記。
- io-structured/io-compression の専用 lane は省略: 変更は io-file
  の cfg/feature 追加のみで公開 API 不変、かつ `test-io-e2e` が
  fault-injection 有効の io-file を含む下流一式を compile+test
  済み（POLICIES §5 の比例検証。evidence に判断を記録）。

## Related Experiments

- [exp: Issue #306 G07 fault seam 実現可能性の検討と clippy 修正経緯](../exp/issue-306-fault-seam-fuzz.md)
  — seam 形状（選択肢 D: 既存 mock の opt-in feature 公開）の採用
  根拠と `assigning_clones` 修正の試行経緯を記録。

## Validation

全て `mise` タスクで実走（exit 0、Windows 11 x64 / rustc
1.99.1 / debug test profile）:

- `mise run fmt-check`
- `mise run check-io-file` / `lint-io-file` / `test-io-file`
  （feature 無し = 既定 build 不変の確認）
- `mise run check-io-e2e` / `lint-io-e2e` / `test-io-e2e`
  （fault_seam 8 件 pass・corpus_replay は `#[ignore]` で通常
  lane 非実行を確認）
- `mise run test-io-e2e-corpus`（opt-in・432 cases・panicked=0・
  elapsed≈22.5ms）
- 補助証拠: `cargo check -p ruviform-io-file --features
  fault-injection --all-targets`（canonical = 上記 mise lane）

詳細な実値・出力 transcript は
`evidence/rf306/fault-seam-fuzz-report.md` を正本とする。

## Risks

- seam が opt-in を超えて production/公開面へ漏れると fault 経路
  が本番 build に残る。`fault-injection` 非 default +
  `#[doc(hidden)]` + facade の std-only signature + feature 無し
  lane の検証で封じ込めたが、将来の feature 追加時に再確認が
  必要。
- `MockFs` の `pub` field は観測 evidence 用途に限定されるべき
  で、facade 以外からの利用は想定しない（module が doc(hidden)
  のため doc 面には出ない）。
- corpus replay は小規模 bounded 入口であり、sustained fuzz の
  品質・未知 bug の発見率は主張しない（Deferred）。

## Follow-ups

- 実 OS での `Indeterminate`/DirSync 後失敗/`TempState::Unknown`
  再現は OS capability 外のまま（Unsupported 維持）。実 FS での
  統合証拠が必要になれば別 Issue で扱う。
- G01 実 4GiB・G02 Network/Cloud FS・G03 特殊 FS・実故障操作・
  実測容量/性能閾値・持続 Fuzz は本タスク外（Deferred/外部 gate）。
- consumer 実測（AudioLens #139/#143・Synvelis #82/#84）は
  #219/#294 cross-repo gate の責務（NotRun）。
- merge 後の対象 SHA に対する必須 CI（`test-io-msrv` 含む）の
  証拠確認は Coordinator 側で実施。
