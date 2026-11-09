---
id: "SPEC-akGFe19N27efIRTF1LTQdg"
type: "spec"
status: "approved"
title: "Issue #306 IO E2E slice 3 — IO MSRV lane・DAG/RT 境界 audit 証跡"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/288"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-306-io-msrv-dag-rt-audit.md"
      - type: "file"
        path: "docs/designs/issue-306-io-msrv-dag-rt-audit.md"
      - type: "file"
        path: "docs/test-specs/issue-306-io-msrv-dag-rt-audit.md"
      - type: "file"
        path: "docs/plans/issue-306-io-msrv-dag-rt-audit.md"
      - type: "file"
        path: "docs/tasks/issue-306-a1-msrv-dag-rt-audit.md"
      - type: "file"
        path: "docs/impl/ir/issue-306-io-msrv-dag-rt-audit.md"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "evidence/rf306/report.md"
      - type: "file"
        path: "crates/ruviform-io-e2e/README.md"
      - type: "file"
        path: "docs/development-environment.md"
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
    - "https://github.com/scarecrow173/ruviform/issues/306"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-306-io-e2e-fault-fuzz.md"
    - "docs/plans/issue-306-io-e2e-fault-fuzz.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
  refines:
    - "docs/specs/issue-306-io-composition-e2e.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-306-io-composition-e2e.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 IO E2E slice 3 — IO MSRV lane・DAG/RT 境界 audit 証跡

## What Should Be Built

Issue #306（IO-IMP-12 共通 I/O 合成 E2E・Fault/Fuzz）の第 3
slice。第 1・2 slice（`docs/specs/issue-306-io-composition-e2e.md`・
`docs/specs/issue-306-io-e2e-fault-fuzz.md`）で `ruviform-io-e2e`
77 tests を含む合成試験と証拠 report は main 統合済み。本 slice
は Issue 受入条件の残行のうち headless で誠実に実行可能な
**「MSRV1.89」「DAG/RT 監査」「License 状態記録」**に限定して
閉じる。

追加するもの:

1. **IO 系 crate の MSRV lane**: `test-ofx-msrv`/
   `test-ui-test-msrv`（`mise.toml` 既存 MSRV task）と同じ
   `mise exec rust@1.89.0` 方式で、`ruviform-io-file`・
   `ruviform-io-archive`・`ruviform-io-structured`・
   `ruviform-io-compression`・`ruviform-io-wav`・
   `ruviform-io-e2e` の 6 crate を MSRV 1.89 toolchain で
   build・test する `test-io-msrv` task。`ruviform-io-e2e` を
   同一 `-p` 集合に含め、dev-dependencies 経由の feature
   unify（io-structured の json,yaml,toml,csv,xml,file・
   io-compression の gzip,zstd）で E2E が実際に消費する
   feature 組合せをそのまま検証する。io-compression の
   `deflate` は io-e2e が消費しないため別コマンドで補完する。
2. **DAG/RT 境界 audit 証跡**: `cargo metadata` / `cargo tree`
   による機械確認 — (a) `ruviform-io-e2e` が leaf（本番・
   dev いずれの依存辺も持たない）、(b) `ruviform-media-io`
   の path 依存が Domain-Core3（`ruviform-media-core`・
   `ruviform-audio-core`・`ruviform-video-core`）のみで逆方向
   依存がない、(c) `ruviform-rt` が io/blocking 依存を持た
   ない — を実行コマンド・実測出力・判定として
   `evidence/rf306/` の既存 report に追記する。
3. **License 状態の記録**: `license = "MIT"` は配布方針確定
   までの仮置きであり license 未決定である旨を証跡へそのまま
   記録する（発明・確定扱いしない）。

## Why Now

Issue #306 受入条件の最終行「…MSRV1.89/Feature/License/DAG/RT
監査を統合」のうち、MSRV lane・DAG/RT 監査・License 記録は
headless で誠実に実行可能であり、本 slice で閉じられる。
残る受入行（実 OS/実ネットワーク FS・consumer 実測・4 GiB
実測・Issue close）は headless 不可のため NotRun/Unsupported
として記録に留める。

## Users and Value

- Issue #306 受入監査: 「MSRV1.89/DAG/RT/License 監査」の
  実測証跡を既存 evidence report に集約する。
- 後続 consumer 受入（#219/#294）: IO 系 crate が MSRV 1.89 で
  green であること、io-e2e が leaf・media-io が Domain-Core3
  方向・rt が io/blocking 非依存であることの機械確認済み根拠。

## Scope

### In Scope

- `mise.toml` への `test-io-msrv` task 追加（既存 MSRV lane
  の命名・`mise exec rust@1.89.0`・`CARGO_TARGET_DIR` 分離の
  先例に倣う）。
- `cargo metadata` / `cargo tree` による DAG/RT audit の実行と
  結果の `evidence/rf306/report.md` への追記（実コマンド・
  実出力要約・判定・License 状態・NotRun 一覧）。
- `crates/ruviform-io-e2e/README.md` 検証節と
  `docs/development-environment.md` opt-in task 一覧への
  `test-io-msrv` 追記。
- 本 doc チェーン（spec/design/test-spec/plan/task/impl）と
  managed docs index の再生成。

### Out of Scope

- `Indeterminate`・DirSync 後失敗・`TempState::Unknown` を公開
  API 経由で trigger する fixture の捏造 — `#[cfg(test)]`
  MockFileOps の canonical テストが正本であり、重複・迂回実装
  を作らない。
- 実 OS/実ネットワーク FS・consumer 実走（AudioLens/Synvelis）・
  4 GiB 実測・Issue #306 の close — headless 不可のため
  NotRun/Unsupported として記録のみ。
- 新規 YAML/CI framework・証拠 schema・Dashboard・独立
  workflow（Issue #306 が禁止）。
- IO crate の公開 API・実装の変更、feature 追加、依存追加。

## Requirements

- MSRV lane は `mise.toml` タスクとして定義し、cargo を直接
  実行せず `mise run test-io-msrv` を唯一の入口とする
  （`docs/development-environment.md` の検証入口規約）。
- audit は `cargo metadata` / `cargo tree` の実コマンドと実
  出力を証跡に残し、目視確認のみで済ませない。
- License は現行値（`license = "MIT"` 仮置き）をそのまま記録
  し、決定済み・承認済みとは書かない。
- NotRun/Unsupported は実測範囲と一致させ、未実施を pass と
  記録しない。

## Acceptance Criteria

- [ ] `mise run test-io-msrv` が `rust@1.89.0` toolchain で
      exit 0（6 crate・io-e2e 統合 feature・deflate 補完を含む）。
- [ ] `evidence/rf306/report.md` に slice 3 節として、実行
      コマンド・実測結果・判定・License 状態・NotRun が追記
      されている。
- [ ] DAG/RT audit の 3 判定（io-e2e leaf・media-io
      Domain-Core3 方向・rt 非 io/blocking）が機械確認済み。
- [ ] 公開 API で trigger 不可能な fault class の偽装 fixture
      を追加していない。
- [ ] `mise run fmt-check` / `check` / `lint` / `test` が pass、
      managed docs index が再生成済み。

## Deferred Design Concerns

- macOS/Linux 実機・Network/Cloud FS での MSRV/IO 実測 |
  headless Windows のみ | 実機 dispatch | OS 差異が未検出の
  まま残る。
- Consumer 実測（AudioLens #139・Synvelis #82） | #219/#294
  cross-repo gate の責務 | consumer-verify lane 整備後 |
  片側 pass を両 Consumer 合格と誤認するリスク。
- `deflate` を含む io-compression 全 feature 組合せの MSRV
  網羅 | lane は E2E 消費 feature + deflate 補完に限定 |
  feature 追加時の再検討 | 将来 feature が増えた場合に
  MSRV 網羅が陳腐化する。

## Sources and Relations

- Issue #306 本文（受入条件の正本）・Issue #288 IO-06 v0.1・
  Issue #282 横断設計正本。
- 前 slice 文書チェーン: `docs/{specs,designs,plans,tasks,
  test-specs,impl/ir}/issue-306-*.md`（slice 1・2）— 本 slice
  はその残存受入行の一部を担う。
- `docs/development-guidelines.md` MSRV clippy 検証節
  （Issue #223）・`docs/development-environment.md` 検証入口。
- `evidence/rf306/report.md`・`evidence/rf306/fault-fuzz-
  report.md` — 既存証跡の形式正本。
