---
id: "IMPL-034YqH5uEzCRjkxjkzRS3V"
type: "impl"
status: "draft"
title: "Issue #631 real-row 拡充 実装記録"
created: "2026-11-16"
updated: "2026-11-16"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/631"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-631-real-row.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-631-real-row.md"
    - "docs/designs/issue-631-real-row.md"
    - "docs/plans/issue-631-real-row.md"
    - "docs/test-specs/issue-631-real-row.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/impl/exp/issue-631-real-row-exp.jsonl"
    - "evidence/rf631/verification.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #631 real-row 拡充 実装記録

## Summary

`rf631.plugin-dock-real` の real-window カタログを 5 行から 12 行へ拡充した。
AC1 の残件（2 window focus 移譲・実 OS pointer/keyboard 入力・DPI・IME・
window close）と AC3 の非画素 editor lifecycle 行（resize・re-dock）を追加し、
headless host が real session を開かずに fail-closed することを証明する
`--blocked-probe` lane（`test-ui-plugin-dock-real-headless`）を新設した。
verdict 語彙は `Pass`/`Fail`/`NotRun`/`Unsupported` を維持し、観測できない
能力は `Unsupported`、効果 oracle のない受理は `NotRun`、環境阻害は
`NotRun`+`observed.environment_blocked` として正直に記録する。

## Context

`docs/tasks/issue-631-real-row.md` の slice。前段で 5 行
（mount / native-slot / editor-attach / capture / dcomp-pixel）が実装済みで、
本 slice は AC1 残りと AC3 非画素行、headless fail-closed 証明を担当する。
`ruviform-ui-gpui` / `ruviform-plugin-host` / `ruviform-ui-test` の公開契約は
変更せず、test 専用 crate `ruviform-plugin-dock-test` 内で完結させた。

## Implementation

### Changes

- `crates/ruviform-plugin-dock-test/src/real_catalog.rs`: `REAL_CATALOG` に
  7 `RealCatalogRow` を追加し、metadata と executable entry が pairwise
  一致する 12 行構成にした。
- `crates/ruviform-plugin-dock-test/src/real_dock.rs`:
  - `real_entries()` を 12 entry に拡張。window-only case は
    `ctx.window`（window A）を使い、focus/os-input case のみ window B を
    参照する。
  - `real-multiwindow-focus`: 2 window mount → `request_focus` →
    `settle_until(window_focused)` で A→B→A の focus 移譲を観測。
    focus release は記録のみで verdict にしない。
  - `real-os-input`: `os_input_preflight(PointerAndKeyboard)` の family
    別 effect を確認後、`plugin.dock` panel が canvas を paint しないため
    `plugin.editor` の painted bounds を得るべく実 native slot を
    declare→bind→project→lease し、pointer move（物理座標 + cursor
    read-back）・mouse down/up・`Tab` を注入。`OsInputOutcome` を verbatim
    に step row へ写像し、worst landing で verdict を決める。
  - `real-window-dpi` / `real-window-close`: 実 session 内で公開 API 非存在を
    `Unsupported` として記録（close request/respond・scale-factor 観測は
    adapter seam に無い）。
  - `real-window-ime`: `Ime` capability を要求し preflight が正直に
    `Unsupported` を返す（adapter は IME を claim しない）。
  - `real-editor-resize` / `real-editor-redock`: `attempt_real_attach` の
    `observe_open` を再利用し、本番 `InProcessEditorOps` 経由で
    `set_size` / `reattach` を試行。`OpLanding{Refused,Accepted,Other}` で
    着地点を表現し、定義済み `EditorOpError::Unsupported` は lifecycle・
    event 系列・子 HWND 収束を検証した上で `Unsupported`、受理されたが
    非画素 effect oracle が無い場合は `NotRun` とした。pixel capture は
    呼ばない。
  - `blocked_reports` / `blocked_probe_violations` を pub 化し、bin の
    `--blocked-probe` から利用。
- `crates/ruviform-plugin-dock-test/src/bin/plugin-dock-real-scenario.rs`:
  `--blocked-probe` flag を追加。session を開始せず全 catalog 行へ
  `NotRun`+`observed.environment_blocked`+非 real `environment_kind` の
  report を生成し、違反があれば exit 1。lane 名は
  `test-ui-plugin-dock-real-headless`。通常 lane は `Fail` のみ exit 1。
- `crates/ruviform-plugin-dock-test/tests/console.rs`: real 行 12・合計 20・
  case 順序を検証するよう更新。
- `mise.toml`: `test-ui-plugin-dock-real-headless` task を追加し
  `evidence/rf631/plugin-dock-real-headless.jsonl` へ出力。
- `crates/ruviform-plugin-dock-test/README.md`、`src/lib.rs`、
  `docs/development-environment.md`: 12 行構成・AC1/AC3 挙動・blocked-probe・
  verdict 規則・対話 desktop 要件を記述。
- `docs/specs|designs|plans|tasks|test-specs/issue-631-real-row.md`:
  doc chain を canonical front matter で整備。
- `evidence/rf631/plugin-dock-real-headless.jsonl`（新規）と
  `plugin-dock-real-scenario.jsonl`（実 lane 再生成）を記録。

### Decisions During Implementation

- pointer aim 対象: `plugin.dock` panel は canvas を paint しないため
  `component_bounds` に現れず、初回実 lane では `pointer_move` が
  `NotRun`（no painted bounds）だった。aim 対象を実 native slot を
  bind した `plugin.editor`（実際の dock 領域）へ変更し、slot chain の
  step row も証拠として残す設計にした。これにより実 lane で
  pointer move（cursor read-back 付き）・click・`Tab` が全て `Injected`
  となり `Pass` を観測した。
- `OpLanding` enum: resize/redock の着地点判定で bool の組み合わせが
  複雑化したため `Refused`/`Accepted`/`Other` の 3 値に集約し、
  `struct_excessive_bools` lint と verdict 写像の明示性を両立した。
- `real-dcomp-pixel` の `Fail` は DComp topmost occlusion（Issue #532）に
  よる既知の正直な regression signal であり、Pass に変換しない。
  lane exit 1 はこの 1 件が原因で、他 11 件は Pass/Unsupported/NotRun の
  正直な結果である。
- review-protocol（独立 2 reviewer + coordinator 判定）の CONFIRMED 指摘を
  反映: `OpLanding::Accepted` は `verified` 必須（検証失敗は `Fail`）、
  pointer aim 非 `Injected` 時は click を発火せず `not_run` 記録、
  focus settle timeout は `not_run`、未知 `OsInputOutcome` variant は
  `not_run`、multiwindow は A baseline focus を先に観測、
  `blocked_probe_violations` に case id/driver/environment_kind 検査を追加。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-631-real-row-exp.jsonl` seq 1-4: 公開面調査・
  OpLanding 設計・catalog/entries 拡張の決定を採用。

### Rejected

- なし。

## Validation

- `cargo fmt --check`: pass
- `cargo clippy -p ruviform-plugin-dock-test --features real-dock
  --all-targets -- -D warnings`: pass
- `cargo clippy -p ruviform-plugin-dock-test --all-targets -- -D warnings`
  （default feature）: pass
- `cargo test -p ruviform-plugin-dock-test --features real-dock`: 3 unit +
  14 console + 4 plugin-dock + doc tests 全 pass
- `cargo test -p ruviform-plugin-dock-test`（default feature）: pass
- `mise run test-ui-plugin-dock-real-headless`: pass（exit 0）。
  `evidence/rf631/plugin-dock-real-headless.jsonl` は 12 case 全て
  `not_run` + `observed.environment_blocked` +
  `environment_kind=deterministic-backend`、real-window claim 無し。
- `mise run test-ui-plugin-dock-scenario` / `plugin-dock-console`:
  non-regression pass
- `mise run check-ui-driver-boundary`: pass（dep-graph/signature/allowlist）
- `mise run test-ui-plugin-dock-real`（対話 desktop）: 実 window を開き
  12 case を実行。`real-dcomp-pixel` が既知の DComp occlusion で `Fail`
  （lane exit 1）。`real-os-input` は slot bind 後に全注入 `Injected` で
  `Pass`。`real-window-dpi`/`ime`/`close`/`editor-resize`/`editor-redock`
  は `Unsupported`。残りは `Pass`。

## Risks

- `real-dcomp-pixel` の `Fail` は Issue #532 の未解決 occlusion が原因で、
  本 slice の範囲外。lane が exit 1 を返すのは意図どおりだが、CI/定期
  実行で赤く見える点は周知が必要。
- `real-os-input` の `Pass` は transport-level（注入が実 window に届いた）
  証拠であり、component-level の semantic effect は oracle 不在のため
  claim しない。
- `real-editor-resize`/`redock` は fixture plugin が op を `Unsupported`
  で拒否する現状に依存する。将来 op が受理されると `NotRun` に変わる
  （Fail/Pass にはならない）設計。

## Follow-ups

- Issue #631 は open のまま維持（GUI Test Lab / Scenario Console GUI 本体は
  本 slice の範囲外）。
- Issue #532（DComp occlusion）・#389（IME）・window close/DPI API は
  別 issue の範囲。
