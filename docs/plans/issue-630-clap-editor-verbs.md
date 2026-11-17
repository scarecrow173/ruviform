---
id: "PLAN-630clapEditorVerbs04"
type: "plan"
status: "approved"
title: "Issue #630 CLAP editor verb 実バイナリ coverage slice — 実装計画"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-630-clap-editor-verbs.md"
    - "docs/designs/issue-630-clap-editor-verbs.md"
  changes:
    added:
      - type: "file"
        path: "docs/plans/issue-630-clap-editor-verbs.md"
  implements:
    - "docs/specs/issue-630-clap-editor-verbs.md"
    - "docs/designs/issue-630-clap-editor-verbs.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-630-clap-editor-verbs.md"
    - "docs/designs/issue-630-clap-editor-verbs.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 CLAP editor verb 実バイナリ coverage slice — 実装計画

1. fixture 更新: `src/editor.rs` の `GuiState` に `size` 追加・
   `Default` 手書き化、`can_resize=true`、`set_size` 実装 +
   `win32::resize` helper、module doc・unit test 追従、
   `README.md` 追従。
2. host_path 試験追加: 非 ignore 層 2 件（editor no-gui refusal・
   non-editor 4 class refusal）と Windows `#[ignore]` 実機層
   1 件（open → child HWND → resize → hide/show → reparent 拒否 →
   close → double-close）。
3. canonical 検証: `mise run fmt-check` / `check` / `lint` / `test`
   （`ruviform-plugin-test-clap`）、adapter 非回帰
   `check`/`lint`/`test`（`ruviform-plugin-clap`）、
   `build`（cdylib 生成）、`-- --ignored` 実走。
4. validator 簿記: `CLAP_VALIDATOR`/`VST3_VALIDATOR` を pin して
   `mise run test-plugin-abi-validator` 実走、run.json・raw txt
   更新。binary 不在なら NotRun 記録。
5. 証拠・文書: 検証 md（`evidence/rf630/`）、doc chain 後半
   （tasks/test-spec/adr/impl ir/exp/jsonl）作成。
6. `mise exec -- prek run --all-files` → review-protocol →
   commit → push → PR（`Refs #630`）。

`crates/` の production crate・`Cargo.toml`/`Cargo.lock`・
`mise.toml`・`.github/**`・docs index・`evidence/` の他 Issue 分は
変更しない。`*-msrv`/`*-full` lane は実行しない。
