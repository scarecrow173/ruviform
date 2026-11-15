---
id: "PLAN-034XWO1EjjLigZP3nnak6l"
type: "plan"
status: "approved"
title: "Issue #631 Headless Scenario Console 実装計画"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-631-scenario-console.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-631-scenario-console.md"
    - "docs/designs/issue-631-scenario-console.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #631 Headless Scenario Console 実装計画 Implementation Plan

## Goal

merge 済み scenario catalog（`HEADLESS_MATRIX`）の headless consumer
として `plugin-dock-console` bin を追加し、`--list`（capability
projection）と `--run`（selector 付き部分実行 + fail-closed）を
提供する。real lane の catalog metadata は driver-neutral な
`real_catalog` module に分離し、console が全 build 構成で real
selector を正直に識別できるようにする。

## Scope

単一実装 stream — plugin-dock-test 内の新規 module/bin/test +
`real_dock`/`scenario` の narrow な拡張 + doc/evidence。分割不要。
`ruviform-ui-test` は変更しない（bin 側 `cells()` filter で閉じる）。

## Upstream Documents

- `docs/specs/issue-631-scenario-console.md`
- `docs/specs/issue-631-scenario-catalog.md`（前 slice — catalog）
- `docs/specs/issue-631-plugin-dock-scenario.md`（headless lane）
- `docs/specs/issue-631-real-window-dock.md`（real-window lane）
- `docs/designs/issue-464-scenario-runner.md`（runner は 1 関数で
  executor ではない制約）

## Design Inputs

- `docs/designs/issue-631-scenario-console.md`（approved）

## File Map

- `crates/ruviform-plugin-dock-test/src/real_catalog.rs` — 新規。
  `RUN_ID`/`SCENARIO`/`LANE`/`DRIVER` const + `RealCatalogRow` +
  `REAL_CATALOG`（4 行 metadata、非 feature-gated）
- `crates/ruviform-plugin-dock-test/src/lib.rs` — `pub mod
  real_catalog` + `run_selected` re-export 追加
- `crates/ruviform-plugin-dock-test/src/scenario.rs` —
  `run_selected(env, keep)` 追加、`run_all` を select-all 投影に
- `crates/ruviform-plugin-dock-test/src/real_dock.rs` —
  `"real-window-dock"` literal を `real_catalog::LANE` に置換、
  `REAL_CATALOG`↔`real_entries()` 一致 test 追加
- `crates/ruviform-plugin-dock-test/src/bin/plugin-dock-console.rs` —
  新規。`--list`/`--include-real`/`--run`/`--out`、selector 検証
  （real-lane refusal → per-selector unknown → combination empty
  の順で fail-closed）
- `crates/ruviform-plugin-dock-test/tests/console.rs` — 新規。
  `env!("CARGO_BIN_EXE_plugin-dock-console")` で bin を駆動
- `crates/ruviform-plugin-dock-test/README.md` — console 節追記
- `evidence/rf631/plugin-dock-console.jsonl` — `--list`/`--run` の
  実出力を保存
- `evidence/rf631/verification.md` — 本 slice の検証記録追記
- `docs/*/issue-631-scenario-console.*` — doc chain
- `docs/*/README.md` — index 再生成（generator のみ）

## Implementation Sequence

- [ ] `real_catalog.rs` + `lib.rs` 配線
- [ ] `scenario.rs` の `run_selected` + `run_all` 投影化
- [ ] `real_dock.rs` の `LANE` 共有化 + 一致 test
- [ ] `plugin-dock-console.rs` 実装
- [ ] `tests/console.rs` 追加
- [ ] fmt/check/lint/test（default + real-dock）・非回帰 lane・
  boundary guard・prek
- [ ] console `--list`/`--run` 実走 → `evidence/rf631/` 保存・
  verification.md 追記
- [ ] doc chain 完成（ir/exp）・index 再生成

## Dependencies and Risks

- `run_selected` は `run_all` と同一の fresh-context + catalog 順
  sequencing — `run_all` を委譲形にするため lane 挙動は不変のはず
  （`test-ui-plugin-dock` 非回帰で検証）。
- 部分実行で `artifact` 未選択の依存 case は `NotRun` — 意図的な
  正直さ（前提 case が run されなかった事実を記録）。
- `REAL_CATALOG`↔`real_entries()` drift は feature-gated test で
  検出。`LANE` 共有で listing/report の lane label 乖離を防ぐ。
- `--driver` 名は `ScenarioDriver` serde（kebab-case）のみを語彙と
  し、bin に driver literal を置かない。

## Task Breakdown

- T1（`docs/tasks/issue-631-scenario-console.md`）: 全体を 1 task で
  実施。

## Verification

- [ ] `mise run fmt-check`
- [ ] `mise run check|lint|test ruviform-plugin-dock-test`（default
  + `--features real-dock`）
- [ ] `mise run test-ui-plugin-dock`（JSONL verdict 非回帰）
- [ ] `mise run test-ui-scenario-contract`（fake/gpui 両 lane）
- [ ] `mise run check-ui-driver-boundary`
- [ ] `mise exec -- prek run --all-files`
- [ ] console `--list`/`--run --case <one>` 実出力を evidence 保存
  （補助証拠 — canonical = mise）

## Open Questions / Gaps

- `mise.toml` への console lane task は別 Worker 占有中のため本
  slice では追加しない — console の canonical lane 証跡は後続
  slice の「未対応の作業」として PR 本文に記録。

## Self-Review

- [x] spec の acceptance criteria を全て file map / sequence /
  verification に写像済み。
- [x] GPUI console GUI・実 Native Dock Pass・製品 oracle・real
  executor は明示的に out of scope。
- [x] file paths・relations・status values・terminology は既存 doc
  慣行と一致。

## Implementation Handoff

- Recommended mode: inline execution（dispatch worker が直接実装）
- Independent work streams: none
- User approval required before dispatch: no（worker 契約により
  review-protocol を承認として扱う）

## Review Handoff

- verdict 正直性（`--list` は evidence でない、部分実行の
  NotRun は偽装しない、real-lane は拒否して既存 lane へ誘導）と
  GPUI 非依存（`real_catalog` が driver-neutral metadata のみ）が
  reviewer focus。
