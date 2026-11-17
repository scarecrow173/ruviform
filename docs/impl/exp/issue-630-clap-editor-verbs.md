---
id: "EXP-630clapEditorVerbs08"
type: "impl"
status: "completed"
title: "Issue #630 CLAP editor verb 実バイナリ coverage slice — 実験記録"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/impl/ir/issue-630-clap-editor-verbs.md"
    - "docs/impl/exp/issue-630-clap-editor-verbs-exp.jsonl"
  changes:
    added:
      - type: "file"
        path: "docs/impl/exp/issue-630-clap-editor-verbs.md"
      - type: "file"
        path: "docs/impl/exp/issue-630-clap-editor-verbs-exp.jsonl"
  implements:
    - "docs/impl/ir/issue-630-clap-editor-verbs.md"
  implemented-by: []
  depends-on:
    - "docs/impl/ir/issue-630-clap-editor-verbs.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #630 CLAP editor verb 実バイナリ coverage slice — 実験記録

詳細な時系列イベントは companion JSONL
`docs/impl/exp/issue-630-clap-editor-verbs-exp.jsonl`
（`experiment_event.v1`）を参照。

## 要約

1. discovery で `test_editor` の resize-capable 化を決裁
   （VST3 `ruviform#754` 対称 — `set_size` success 経路を実
   バイナリで検証するには fixture の resize 受理が必要）。
2. `src/editor.rs` に `GuiState.size`・`can_resize=true`・
   `set_size`（created gate + `win32::resize`）を実装し、
   `tests/host_path.rs` に非 ignore refusal 2 件と Windows
   `#[ignore]` 実機層 1 件を追加。
3. canonical 検証: fmt-check/check/test/build 全 exit 0。
   `lint ruviform-plugin-test-clap` は scope 外既存破損
   （`ruviform-plugin-clap` host feature の clippy 4 件）で
   Fail — stash 検証で本 slice 起因でないことを確認。
4. ignored 実機層を interactive window station で実走し Pass —
   open_gui → 実 child HWND → GetWindowRect 800x600 resize →
   WS_VISIBLE bit hide/show → reparent PluginFailed → close 破棄
   → double-close Unsupported を実測。
5. `test-plugin-abi-validator` を pin 済み validator で再実走し
   verdict Passed — run.json の artifact SHA-256・repo.head を
   `a3804b04` 時点の値に更新。
