---
id: "IMPL-At0dLb2kB9niysH7FtgPk8"
type: "impl"
status: "completed"
title: "Issue #306 汎用 I/O 合成 E2E — ruviform-io-e2e 試験クレートと RF #305 候補 scratch 証拠 (IO-IMP-12)"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/288"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-e2e/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-e2e/README.md"
      - type: "file"
        path: "crates/ruviform-io-e2e/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/common/mod.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/staged_lifecycle.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/structured_staged.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/archive_compression_staged.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/extraction.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/wav_staged.rs"
      - type: "file"
        path: "docs/specs/issue-306-io-composition-e2e.md"
      - type: "file"
        path: "docs/designs/issue-306-io-composition-e2e.md"
      - type: "file"
        path: "docs/plans/issue-306-io-composition-e2e.md"
      - type: "file"
        path: "docs/test-specs/issue-306-io-composition-e2e.md"
      - type: "file"
        path: "docs/tasks/issue-306-t1-crate-scaffold.md"
      - type: "file"
        path: "docs/tasks/issue-306-t2-file-structured-boundary.md"
      - type: "file"
        path: "docs/tasks/issue-306-t3-archive-compression-boundary.md"
      - type: "file"
        path: "docs/tasks/issue-306-t4-wav-boundary.md"
      - type: "file"
        path: "docs/tasks/issue-306-t5-mise-task-verification.md"
      - type: "file"
        path: "docs/tasks/issue-306-t6-candidate-scratch-evidence.md"
      - type: "file"
        path: "evidence/rf306/report.md"
      - type: "file"
        path: "docs/impl/ir/issue-306-io-composition-e2e.md"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
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
        path: "docs/discovery/README.md"
  implements:
    - "docs/tasks/issue-306-t1-crate-scaffold.md"
    - "docs/tasks/issue-306-t2-file-structured-boundary.md"
    - "docs/tasks/issue-306-t3-archive-compression-boundary.md"
    - "docs/tasks/issue-306-t4-wav-boundary.md"
    - "docs/tasks/issue-306-t5-mise-task-verification.md"
    - "docs/tasks/issue-306-t6-candidate-scratch-evidence.md"
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-295-ruviform-io-file-staged-publish-contract.md"
    - "docs/impl/ir/issue-296-ruviform-io-file-os-backend.md"
    - "docs/impl/ir/issue-300-ruviform-io-compression-gzip-raw-deflate-compression-core.md"
    - "docs/impl/ir/issue-301-io-compression-zstd.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-306-io-composition-e2e.md"
    - "docs/designs/issue-306-io-composition-e2e.md"
    - "docs/specs/issue-306-io-composition-e2e.md"
    - "docs/test-specs/issue-306-io-composition-e2e.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers:
    - "docs/tasks/issue-294-t2-consumer-verify-workflow-tuple-ui-lanes.md"
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected:
      - "serde_json は非有限 f64 をエラーにせず null に写すため、encode 失敗 fixture は custom Serialize impl 失敗に切替"
      - "WAV 未 finalize drop は backend が best-effort finalize するため placeholder 仮定を捨て、incomplete 試験は truncated bytes の独立 decode 不一致で検証"
---

# Issue #306 汎用 I/O 合成 E2E — ruviform-io-e2e 試験クレートと RF #305 候補 scratch 証拠

## Summary

test-only クレート `ruviform-io-e2e` を新設し、main 上の既存公開
API（`StagedFile`・Structured・Compression・Archive・`WavMuxer`）の
層横断境界を 31 件の合成試験で検証した。RF #305 候補
`82256cff…` は scratch worktree（TEMP 配下・検証後除去）で旧
API 不在と `WavMuxer<&mut StagedFile>` フローの実走を確認。

## Context

- 上流: [spec](../../specs/issue-306-io-composition-e2e.md)・
  [design](../../designs/issue-306-io-composition-e2e.md)・
  [plan](../../plans/issue-306-io-composition-e2e.md)・
  [test-spec](../../test-specs/issue-306-io-composition-e2e.md)
- 実施タスク: T1–T6（`docs/tasks/issue-306-t*.md`）
- 実行証拠: `evidence/rf306/report.md`（実コマンド・実値・
  NotRun/Blocked の明記）

## Implementation

- `staged_lifecycle.rs`: type-state 境界（create→write→seal→
  readback→publish→reopen、CreateNew 競合→`NotPublished`+
  `DestinationExists`+先入主保全、Replace owner 差替え、
  abort/drop temp 回収、readback seek）。
- `structured_staged.rs`: json/csv → `&mut StagedFile` → seal →
  独立 decode → publish。`write_file` の seal+publish compose、
  Serialize 失敗→不 publish、truncated JSON→strict decode 失敗→
  abort、`Publish` 失敗で `PublishFailure` 保持。
- `archive_compression_staged.rs`: TAR→gzip/zstd→`&mut StagedFile`
  の 3 層合成 round-trip（各層 finish/verified EOF）。encoder
  未 finish drop・truncated member → verified EOF 不成立→
  abort。declared size overrun は write 時点拒否、streamed
  budget は declared hint 非依存で拘束、ZIP indexed layout on
  staged Seek。
- `extraction.rs`: TAR/ZIP secure extraction。byte-exact 手工
  fixture（writer API が emit しない hostile entry）で `..`/
  nested traversal/absolute/drive/symlink/hardlink/special
  refuse、case-fold canonical collision、total-bytes budget、
  失敗時 staging 回収 or `ExtractionRecoveryHandle`。
- `wav_staged.rs`: `WavMuxer<&mut StagedFile>` → `finalize` →
  `into_sink` 借用解除 → `seal` → readback を symphonia で独立
  decode → `publish` → 再 decode。truncated WAV の独立 decode
  不一致→abort、二重 finalize 拒否。

## Verification

全て実走・実値は `evidence/rf306/report.md` 正本。要約:

- `mise run fmt-check` / `check-io-e2e` / `lint-io-e2e` /
  `test-io-e2e`: Pass（31/31 tests）。
- `mise run check` / `lint`（workspace、CARGO_BUILD_JOBS=2）:
  Pass。
- `CARGO_BUILD_JOBS=2 mise run test`（`cargo test --workspace`、
  merge head `a99e953`）: **Pass (exit 0)・2773 passed / 0 failed /
  1 ignored**。main `ccf61a2` 既知の plugin-host 起動 Timeout
  flake は本 run 未発生（失敗未観測のため分類対象なし。
  同一性は推測しない）。
- RF #305 候補 `82256cf…` scratch run 2（正規 mise lane）:
  `mise run check-io-e2e` / `lint-io-e2e` / `test-io-e2e` 全
  Pass・**31/31 tests が候補 revision 上で同一緑**（fixture を
  候補 worktree へ uncommitted 接続し実走）。run 1 の旧 API
  grep 0 件・io-wav 38/38 は直接 cargo による補助証拠であり
  正規 Pass ではない。
- 独立 review: runtime subagent Reviewer A/B → Validator →
  Judge。採用/REJECTED/維持の disposition は report の
  disposition 表に全件記録（実バグ 1 件=ustar version を含む
  採用指摘を反映し再実走で緑維持）。
- NotRun: io-compression/io-structured feature matrix lane、
  両 Consumer 実 runner（Blocked: AL127 修復・SV e815384 候補
  旧 API 参照残存 — coordinator 管理）、実 fault 強制再現
  （io-file mock adapter 層で既検証）、macOS/Linux。

## Notes

- test-only crate は公開 API を持たず dev-deps のみ。依存追加
  理由は `Cargo.toml` コメントに記録。
- `PublishFailureKind` の実 variant 名は `DestinationExists`
  （docs 初稿の `AlreadyExists` を全訂正済み。issue-295/296/304
  系旧文書の同名残存は範囲外・報告のみ）。
- managed index は `doc_maintenance.js apply` で再生成（追加
  のみ、削除なし）。
