---
id: "TASK-svPo0nW2IkDcLaMNhbmbaX"
type: "task"
status: "in-progress"
title: "Issue #630 T1 — VST3 test_gain plugin（ruviform-plugin-test-vst3 first slice）"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "docs/specs/issue-630-test-plugin-suite.md"
  changes:
    added:
      - type: "doc"
        path: "docs/tasks/issue-630-test-plugin-suite.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/issue-630-test-plugin-suite.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-630-test-plugin-suite.md"
    - "docs/specs/issue-630-test-plugin-suite.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-630-test-plugin-suite.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-630-test-plugin-suite.md"
  derived-by: []
  verifies:
    - "docs/test-specs/issue-630-test-plugin-suite.md"
  verified-by: []
  references:
    - "evidence/rf630/verification.md"
  defers: []
  deferred-by: []
---

# Issue #630 T1 — VST3 test_gain plugin（ruviform-plugin-test-vst3 first slice）

## Work

新規 test-only crate `crates/ruviform-plugin-test-vst3` に VST3 `test_gain` を実装する。cdylib+rlib の実 VST3 モジュール（factory + component + controller）を独立実装し、sample-accurate gain law・in-process COM round-trip・本番 host path（`Vst3Library::load`）・`rt_` no-alloc テストを揃える。doc chain・evidence・index 再生成・review-protocol・提出までを含む。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-630-test-plugin-suite.md`
- `docs/specs/issue-630-test-plugin-suite.md`
- `docs/designs/issue-630-test-plugin-suite.md`
- `docs/test-specs/issue-630-test-plugin-suite.md`

## Dependencies

- `vst3 = "0.3.0"`（lockfile 既存）、`ruviform-plugin-core`/`ruviform-audio-core`（workspace path）
- dev-dep `ruviform-plugin-vst3` feature `host`（adapter 本体は変更しない）
- `scarecrow173/ruviform#628` runtime は未 merge — 依存 edge ゼロ（統合は後続 slice）

## Affected Files

- `crates/ruviform-plugin-test-vst3/`（新規ディレクトリ全体・README 含む）
- `Cargo.toml`（root、`members` 1 行のみ）・`Cargo.lock`（再生成）
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/issue-630-test-plugin-suite*.md`、`docs/impl/exp/*-exp.jsonl` companion
- `evidence/rf630/verification.md`
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir,impl/exp}/README.md`（index 再生成のみ）

## Done When

- [ ] `test_gain` cdylib が `GetPluginFactory` + platform hooks を export し、factory が `"Ruviform Test"` vendor・`"test_gain"` component・別 CID で 2 class を登録する
- [ ] in-process COM round-trip テストで factory scan → instantiate → lifecycle → golden PCM process が pass する
- [ ] 本番 host path テスト（`tests/host_path.rs`）で `Vst3Library::load` → `ErasedPlugin` init→prepare→process→teardown が pass する
- [ ] `rt_` prefix `assert_no_alloc` テストが process 経路の allocation を検出しないことを確認し、`mise run test-rt` で収集・実行される
- [ ] sample-accurate automation（frame offset・off-by-one・ゼロフレーム flush・同一 offset last-wins）の golden が pass する
- [ ] `mise run fmt-check`/`check`/`lint`/`test`/`build ruviform-plugin-test-vst3` が exit 0
- [ ] doc chain（spec/design/plan/task/test-spec/ir/exp）と `evidence/rf630/` が整合し、index が再生成されている
- [ ] review-protocol で blocker が残っていない
- [ ] `POLICIES.md`・`AGENTS.md`・`crates/README.md`・`crates/ruviform-plugin-vst3/` ソース・`mise.toml`・`.github/workflows/` に差分がない
