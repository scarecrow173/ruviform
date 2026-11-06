---
id: "TESTSPEC-npkv0TXtBSoMiqPQl8ClVv"
type: "test-spec"
status: "proposed"
title: "Issue #386 Driver-neutral GUI Host 接続試験仕様"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/386"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/issue-386-driver-neutral-gui-host.md"
    - "../plans/implement-issue-386-driver-neutral-gui-host.md"
  derived-by: []
  verifies:
    - "../specs/issue-386-driver-neutral-gui-host.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #386 接続試験仕様

## Feature

Driver-neutral Window/Host 契約（`ruviform-ui::driver`）が
Fake Driver と GPUI Driver で**同一の意味論**を持つこと、および
`ruviform-ui` 本体が GPUI 非依存のまま headless 検証できること。

## Rule / Example

### R1: 契約型の構築（共通・headless）

- `WindowSpec::default()` は `title="ruviform"`、
  `logical_size = 1280x800`、`position = None`、`resizable = true`。
- `WindowCapabilities::default()` は全項目 `Unsupported`。
- `UiError::unsupported` は `Unsupported` variant を生成し、
  `Display` が `unsupported capability` を含む。
- `DriverEvent`/`WindowLifecycle`/`WindowDriver` の公開シグネチャは
  `gpui`/`wry`/`tao`/OS handle 型を含まない
  （`cargo check --workspace` が GPUI feature なしで完走すること
  で間接検証）。

### R2: Fake Driver の lifecycle 意味論（headless）

- `create_window(spec)` は単調 `WindowId` を払い出し、
  `Created` → `Ready` を emit する。
- `request_close(w)` は `CloseRequested` を emit するだけで
  window を破棄しない。`respond_close(w, Veto)` は close を
  キャンセルし window を残す。`respond_close(w, Approve)` は
  `Destroyed` を emit し `WindowId` を退役させる。
- 未回答の `CloseRequested` は**対象 window のみ** `Approve` を
  既定とし、loop 全体や他 window には影響しない。
- 2 window 同時作成・別々の close flow で `WindowId` が交差しない。
- `post_event(w, &Event)` は宛先 window の event inbox に届く。
- `resize`/`focus`/`destroy_window` は不明/破棄済み `WindowId`
  で `UiError::Host`、capability 未対応要求は `UiError::Unsupported`。

### R3: GPUI Driver の同一 meaning（feature `gpui`）

- `#[gpui::test]` + `TestAppContext` で、同一シナリオ
  （create 2 window → Ready → request_close → Veto → Approve →
  Destroyed）を GPUI Driver で再現する。
- `open_window` 受理 → `Created`、初回 frame → `Ready` の順序。
- `on_window_should_close` で `CloseRequested` を emit し、
  consumer の `respond_close` 回答が実 close に反映される。
- `post_event` が `Entity` の view inbox に届く。
- `windows()` が live `WindowId` 集合を返す。

### R4: 接続試験の等価性

- Fake/GPUI 双方で同一シナリオの event 系列
  （variant 順序・WindowId 相関・終端 `Destroyed`）が一致する。
- `Action(Intent)` が `UiRuntime::dispatch_window_intent` で
  `dispatch_for` と同じ result を返す（typed action port）。

### R5: 明示的な残 Gate（本試験では検証しない）

- 実 OS window 表示・実入力 slot・IME・DPI 変更・a11y・
  GPU device lost・実 plugin Native Dock・Dock 2-phase move・
  両 Consumer gate。これらは #262/#388/#389/#390 の実機
  fixture で別途検証する。

## 検証入口

- `mise run test`（workspace・headless — R1/R2）
- `mise run test-ui-gpui`（feature lane — R3/R4）
- `cargo check --workspace`（R1 の GPUI 非依存性）
