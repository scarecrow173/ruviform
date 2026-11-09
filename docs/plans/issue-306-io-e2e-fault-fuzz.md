---
id: "PLAN-034UtCUZnaLUCtLUqb8nYz"
type: "plan"
status: "approved"
title: "Issue #306 IO E2E slice 2 実装計画"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/306"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-306-io-e2e-fault-fuzz.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-306-io-e2e-fault-fuzz.md"
    - "docs/specs/issue-306-io-composition-e2e.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines:
    - "docs/plans/issue-306-io-composition-e2e.md"
  refined-by: []
  derives-from:
    - "docs/designs/issue-306-io-e2e-fault-fuzz.md"
    - "docs/specs/issue-306-io-e2e-fault-fuzz.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-306-io-e2e-fault-fuzz.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #306 IO E2E slice 2 実装計画 Implementation Plan

## Goal

`ruviform-io-e2e` に Fault/Fuzz・OS/FS 実測・Directory 境界の
fixture を追加し、Issue #306 受入条件の残りを閉じる。

## Scope

単一実装ストリーム・1 plan・2 task。変更は `crates/ruviform-io-e2e/`
の tests 層・dev-deps feature・`evidence/rf306/`・本 doc チェーンに
限定。既存クレートの公開 API・挙動を変えない。

## Upstream Documents

- [docs/specs/issue-306-io-e2e-fault-fuzz.md](../specs/issue-306-io-e2e-fault-fuzz.md)（approved）
- Issue #306 本文（受入正本）
- [docs/designs/issue-306-io-e2e-fault-fuzz.md](../designs/issue-306-io-e2e-fault-fuzz.md)（approved）
- [docs/test-specs/issue-306-io-e2e-fault-fuzz.md](../test-specs/issue-306-io-e2e-fault-fuzz.md)（検証正本）
- 第 1 slice: [spec](../specs/issue-306-io-composition-e2e.md) /
  [design](../designs/issue-306-io-composition-e2e.md) /
  [plan](../plans/issue-306-io-composition-e2e.md) /
  [test-spec](../test-specs/issue-306-io-composition-e2e.md)

## File Map

- `crates/ruviform-io-e2e/Cargo.toml` — `ruviform-io-structured`
  dev-dep features を `json,yaml,toml,csv,xml,file` へ拡張
- `crates/ruviform-io-e2e/tests/common/mod.rs` — `dir_link`/
  `file_link`（io-file tests/common と同型）・ZIP CD
  `external_attrs` patch helper 追加
- `crates/ruviform-io-e2e/tests/structured_formats_staged.rs` —
  yaml/toml write_file→read_file、xml read_file、publish 後 tamper
  → Decode 拒否
- `crates/ruviform-io-e2e/tests/fault_boundary.rs` — write_file
  Create/Publish 失敗分類の層越え保持、recovery handle、合成
  publish 競合
- `crates/ruviform-io-e2e/tests/negative_fuzz.rs` — ZIP symlink
  patch・ZIP bomb budget・XML DOCTYPE/entity・破損 fixture typed
  error・決定的 mutation sweep の publish gate
- `crates/ruviform-io-e2e/tests/os_fs_probe.rs` — durability
  ladder の実 FS probe
- `crates/ruviform-io-e2e/tests/rooted_boundary.rs` — RootedDir
  over publish/extraction tree・link escape 拒否
- `crates/ruviform-io-e2e/README.md` — 責務・検証節の更新
- `evidence/rf306/fault-fuzz-report.md` — 実コマンド・実値・
  OS/FS 実測・NotRun/Unsupported・#219/#294 handoff 記述

## Implementation Sequence

- [x] spec → design → test-spec → plan → task の doc chain 作成
- [ ] F1 fixture 実装（common helper + 5 test file + Cargo.toml）
- [ ] `mise run fmt-check`/`check-io-e2e`/`lint-io-e2e`/`test-io-e2e`
      で crate lane 検証・修正
- [ ] `mise run check`/`lint`/`test` で workspace 非回帰検証
- [ ] evidence report + impl record + README 更新・handoff 明記
- [ ] `git fetch origin main && git merge origin/main` で最新取込・
      lockfile/生成物の再生成確認
- [ ] push・PR 作成（日本語・Refs #306）

## Dependencies and Risks

- 依存: 第 1 slice merge 済み（io-e2e crate・StagedFile API・
  ExtractionSession・RootedDir は main に存在）。
- Risk: Windows での link 作成権限（junction は非特権、
  `symlink_file` は特権依存）→ 失敗時 skip + `eprintln!` 記録。
- Risk: ZIP `external_attrs` patch が writer layout 変更で壊れる
  → offset を assert で検証し fail-loud にする。

## Task Breakdown

| Task | 内容 | 依存 |
| --- | --- | --- |
| F1 | fixture 実装一式（common helper・5 test file・Cargo.toml・README） | — |
| F2 | 正規検証（mise lane）+ evidence report + impl record + handoff 明記 | F1 |

## Verification

- `mise run fmt-check` / `check-io-e2e` / `lint-io-e2e` /
  `test-io-e2e` — 全 pass
- `mise run check` / `lint` / `test`（workspace、必要に応じ
  `CARGO_BUILD_JOBS=2`）— 非回帰 pass
- `mise run test-rt` — RT lane 非回帰（io-e2e は非 RT・対象外
  であることの確認を含む）
- OS/FS: Windows NTFS 実測のみ。macOS/Linux・Network/Cloud FS・
  性能・`Indeterminate`/DirSync 失敗/`TempState::Unknown` の実 OS
  再現は `NotRun`/`Unsupported` として記録

## Open Questions / Gaps

- なし（slice 範囲は spec で確定済み。Consumer 実測・#305 接続は
  対象外として明記済み）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（dispatched worker 内で直接実装）
- Independent work streams: none
- User approval required before dispatch: no（Orca dispatch 済み・
  review-protocol を承認として扱う）
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- fixture が公開 API のみを消費しているか・`vfs`/mock 非依存か。
- 否定系が typed 分類 + 非残存の両方を assert しているか。
- NotRun/Unsupported の記録が実測範囲と一致しているか。
