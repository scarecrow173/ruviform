---
id: "PLAN-vK3u33xgmjJerBC8LrUewZ"
type: "plan"
status: "approved"
title: "Issue #307 attach 時 frame_base 検証 context — 実装計画"
created: "2026-11-04"
updated: "2026-11-04"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/307"
    - "docs/specs/issue-307-attach-frame-base-verified-context.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-307-attach-frame-base-verified-context.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-307-attach-frame-base-verified-context.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-307-attach-frame-base-verified-context.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-307-attach-frame-base-verified-context.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #307 attach 時 frame_base 検証 context — 実装計画

## Goal

`PluginNodeControl`/`PluginNode` へ attach 時 `frame_base` の検証済み
context（読取アクセサ + 構築束縛 + 型付き照合）を追加し、異値 base の
plan が node へ届く前に `InstallPlanError::FrameBaseMismatch` で拒否する
単一の authoritative 境界を実装する。test-spec の oracle で検証し、
既存呼出の移行と consumer impact 材料を記録して完了とする。

## Scope

`ruviform-audio-plugin-bridge` のみ。1 task（小規模・単一整合変更）+
doc チェーン + index 再生成。`install_plan` 戻り値型の変更を含むため
in-repo 呼出箇所の移行確認を同一変更で行う。

## Upstream Documents

- `docs/specs/issue-307-attach-frame-base-verified-context.md`（approved; AC-1..6）
- `docs/designs/issue-307-attach-frame-base-verified-context.md`（approved; API 形態決定・境界決定・棄却案）

## File Map

| ファイル | 変更 |
| --- | --- |
| `crates/ruviform-audio-plugin-bridge/src/control.rs` | `frame_base` フィールド・`frame_base()`・`check_frame_base()`・`plan_builder()`・`install_plan` 新戻り値・`FrameBaseMismatch`/`InstallPlanError` 新設 |
| `crates/ruviform-audio-plugin-bridge/src/attach.rs` | `PluginNodeControl::new` へ `cfg.frame_base` 供給 |
| `crates/ruviform-audio-plugin-bridge/src/node.rs` | `PluginNode::frame_base()`（scheduler 委譲） |
| `crates/ruviform-audio-plugin-bridge/src/lib.rs` | `FrameBaseMismatch`/`InstallPlanError` re-export |
| `crates/ruviform-audio-plugin-bridge/tests/frame_base.rs` | 新規: test-spec の検証一式 |
| `docs/specs・designs・plans・test-specs・tasks/…307…` | doc チェーン（本計画自身を含む） |
| `docs/impl/ir/issue-307-…` | Implementation Record |
| `docs/*/README.md` | `doc_maintenance apply` で generated-index 再生成 |

## Task Decomposition

| Task | 内容 | 依存 |
| --- | --- | --- |
| T1 | API 実装（control/attach/node/lib）+ `tests/frame_base.rs` + 呼出移行 + 検証一式 | — |

単一の整合変更（戻り値型変更と利用箇所移行は分離不可）のため 1 task。

## Verification Matrix

| AC | 検証 |
| --- | --- |
| AC-1 | `tests/frame_base.rs`: accessor 往復一致 + verified base plan の絶対 frame 配送 E2E |
| AC-2 | 同: 異値 plan install → `FrameBaseMismatch`・report 空・後続正常 install |
| AC-3 | 同: `check_frame_base` ok/err |
| AC-4 | 同: `device_restart` 後の `frame_base()` 不変 |
| AC-5 | `mise run check`/`lint`/`test` 全通過（全既存呼出移行） |
| AC-6 | IR に consumer impact 材料・Not Run 明示 |

## Risk Register

- `install_plan` 戻り値変更: in-repo は `.unwrap()` のみで非破壊。
  Synvelis は未導入（#101）のため移行負担は将来実装時の契約提示のみ。
- workspace `test` は全クレート実行で時間要。開発中は
  `mise exec -- cargo test -p ruviform-audio-plugin-bridge` で反復し、
  最終検証のみ canonical `mise run test` を使う。
- `#[non_exhaustive]` 規約: `InstallPlanError` は enum-level 付与必須。

## Rollback

変更は新規フィールド/メソッド/型 + `install_plan` 戻り値のみ。
revert で `QueueFull` 直接返却に戻せる（旧型に 1:1 対応）。

## Closeout

- `mise run fmt-check` → `check` → `lint` → `test` → `test-rt` 実行・記録。
- IR を completed へ更新（検証証跡・consumer impact 材料・Not Run 明記）。
- `doc_maintenance apply --type …` で各 index 再生成、`doc_status` で audit。
- Conventional Commits・論理単位（docs チェーン → 実装+テスト → IR/index 同期）。
