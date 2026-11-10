---
id: "SPEC-034Vg7TrJqZWl7EeaAK5yu"
type: "spec"
status: "approved"
title: "Issue #314 slice 4 follow-up — canonical real-scenario lane + story matrix residual cells"
created: "2026-11-10"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/314"
    - "https://github.com/scarecrow173/ruviform/issues/390"
    - "https://github.com/scarecrow173/ruviform/issues/426"
    - "https://github.com/scarecrow173/ruviform/issues/552"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-314-real-scenario-lane.md"
      - type: "file"
        path: "docs/designs/issue-314-real-scenario-lane.md"
      - type: "file"
        path: "docs/plans/issue-314-real-scenario-lane.md"
      - type: "file"
        path: "docs/tasks/issue-314-real-scenario-lane.md"
      - type: "file"
        path: "docs/test-specs/issue-314-real-scenario-lane.md"
      - type: "file"
        path: "docs/impl/ir/issue-314-real-scenario-lane.md"
      - type: "file"
        path: "evidence/issue-314-real-scenario/driver-lab-evidence.jsonl"
      - type: "file"
        path: "evidence/issue-314-real-scenario/golden-evidence.jsonl"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "crates/ruviform-ui-gpui/tests/real_window.rs"
      - type: "file"
        path: "crates/ruviform-ui-test/fixtures/driver-lab/stories.json"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-314-real-window-adapter.md"
    - "docs/impl/ir/issue-314-real-window-adapter.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #314 slice 4 follow-up — canonical real-scenario lane + story matrix residual cells

## 背景

PR #554（slice 4）で `RealOsGuiTest`・`real-scenario` feature・
`tests/real_window.rs` が merge 済みだが、IR
（`docs/impl/ir/issue-314-real-window-adapter.md`）が記録する残件が
2 件残っている。

1. `real-scenario` 相当の canonical mise lane が未追加（当時
   mise.toml が編集範囲外だったため補助 `cargo test` のみ）。
2. real-os story matrix は declared theme × scale 1.0 のみ実走。
   scale 2.0 cell・各 story 第 2 theme（manifest `themes` のもう一方）
   は `stories.json` note で未実施と明記されたまま。

本 spec はこの残件だけを対象とする。Issue #314 自体は consumer gate・
OS input（#313）・macOS/Linux capture backend・Plugin Dock snapshot
が別 scope のため open のまま。

## 要件

1. `mise.toml` に real-window scenario lane を 1 本追加する。
   命名は既存 `test-ui-*` / `example-ui-gpui-real-os-*` の規約に
   合わせ、`tests/real_window.rs` の `real-scenario` feature を呼ぶ。
   対話 desktop session 前提の real-OS lane であり、CI 必須 lane
   にはしない（env 不在で無条件失敗させない）。
2. 実 desktop session で lane を実行し、Pass と証拠（driver-lab /
   golden JSONL・PNG/meta）を `evidence/` に保存する。env 前提が
   満たせない場合は既存 real-OS lane の流儀に倣い
   `environment_blocked` を GUI Fail と区別して記録する
   （`os_input::preflight` の verdict 区分と同じ考え方。実 window
   lane は SendInput を使わないため preflight 本体は適用対象外 —
   区分の考え方のみ適用する）。
3. story matrix の残 cell（scale 2.0・各 story 第 2 theme）を実
   desktop session で実走し、結果を `stories.json` と golden
   evidence に正直に反映する。実行できない cell は NotRun のまま
   note を更新するだけとし、Fake/headless の成功で繰り上げない。
4. mise.toml の変更は本 lane 追加のみ — 既存 task の変更・削除は
   しない。無関係な既存変更は保持する。

## 対象外

consumer 製品 fixture の実接続、OS-level input 本体（#313）、
macOS/Linux capture backend、Plugin Dock snapshot（#388/SV#176）、
`stories.json` の drivers 定義の大幅再設計、`docs/` の Issue 名を
持たない管理文書（他 Worker 占有）の更新、Issue #314 の Close。

## 受入条件

- canonical lane で再現可能な実行経路が確立され、env 証拠が
  verdict/JSONL に記録される（Issue #314 転記行）。
- `stories.json` の `drivers.real-os` が実行実績と一致する
  （matrix cell の pass/NotRun が正直に反映される）。
- 既存検証 lane（`fmt-check`/`check`/`lint`/`test`/`test-ui-*` 系）
  が退行しない。

## Deferred Design Concerns

- consumer gate・OS input・macOS/Linux・Plugin Dock snapshot —
  Issue #314 の別 scope として継続 open。
