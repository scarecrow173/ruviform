---
id: "ADR-034Mz2Lq8Vd4Tn6Rc1Xb9Qe"
type: "adr"
status: "proposed"
title: "legacy host trait（HostHandle/WindowHandle/HostEvent/WindowConfig）の Phase 2 削除は保留する"
created: "2026-11-08"
updated: "2026-11-08"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/431"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../designs/issue-386-driver-neutral-gui-host.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-431-webview-gpui-real-verify.md"
    - "../designs/issue-431-webview-gpui-real-verify.md"
    - "../reports/issue-431-real-verify.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/issue-386-driver-neutral-gui-host.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../specs/issue-423-webview-dock-verify.md"
  defers: []
---

# legacy host trait の Phase 2 削除は保留する

## Context and Problem Statement

Issue #386 で共通契約 `WindowDriver`/`DriverEvent`/`WindowLifecycle`（`ruviform-ui::driver`）が導入され、
旧 API は並存させてきた。移行段階の Phase 2 は「Synvelis/AudioLens の native 最小 Slice・実 OS・
Native Dock・両 Consumer gate 合格後に、旧 API（H1〜H5・H9・H10・H11・H13）の削除/移動を判断する」と
定められている（`docs/designs/issue-386-driver-neutral-gui-host.md` の移行段階 3）。

Issue #431 はこの判断を求められている。対象は `ruviform-ui` の `HostHandle`（`event.rs`）、
`WindowHandle`（`window.rs`）、`HostEvent`（`event.rs`）、`WindowConfig`（`window.rs`）である。
今すぐ削除するか、保留するかを、影響範囲から決める。

## Decision Drivers

- Phase 2 の gate（Consumer の native 最小 Slice・実 OS・Native Dock）が満たされているか
- 各型の実装者・呼び出し元が repo 内と consumer のどこにあるか
- 削除が公開 API の破壊になるか（`ruviform-ui` は公開 crate）
- 実 WebView/GPUI 検証（#431）が、旧 API の残存で妨げられているか

## Considered Options

1. **A: 4 型をすべて今削除する**
2. **B: 4 型をすべて保留し、gate と再評価条件を記録する**
3. **C: `HostHandle` のみ先行削除し、他 3 型は保留する**

## Decision Outcome

Chosen option: **B（保留）**。本 PR では 4 型のいずれも削除・移動しない。

理由:

- Phase 2 gate が未達である。Synvelis は AGENTS.md の表で「設計段階」と定義されており、
  audio-lens の `al-gui` は現在も `HostEvent` を経由して `UiRuntime::handle_window` を呼んでいる。
- `HostEvent` と `WindowHandle` は現行の runtime・テスト・consumer の経路上にあり、
  削除すると repo 内の `ruviform-ui`・`ruviform-ui-test`・`ruviform-ui-gpui`（lab binding）と
  consumer 双方を同時に書き換える必要がある。#431 の範囲を超える。
- `HostHandle` は実装者 0 件だが、公開 API として再 export されており（`lib.rs`）、
  削除は consumer への互換通知を伴う公開 API 変更である。そのため C も本 PR では採らない。

### 使用箇所の棚卸し（判断の根拠）

| 型 | 定義 | 実装者（repo 内） | 呼び出し元（repo 内） | consumer（read-only 確認） |
| --- | --- | --- | --- | --- |
| `HostEvent` | `event.rs:21` | なし（enum） | `UiRuntime::handle`/`handle_window`、`ruviform-ui-gpui/src/lab_binding.rs:107`、`ruviform-ui-test` の headless/fake/journal、`ruviform-ui` の tests | synvelis: `workspace.rs:469-502`、`synvelis-app/src/bin/synvelis.rs:149`。audio-lens: `al-gui/src/desktop.rs:204,330-337` |
| `WindowHandle` | `window.rs:214` | `ruviform-ui-test/src/fake.rs:234`（FakeHost）、`ruviform-ui-test/src/journal.rs:163`、`ruviform-ui/tests/{dock,multi_window}.rs` | `ruviform-ui-test`、`ruviform-ui` tests | synvelis: `WorkspaceManager<H: WindowHandle>`（`workspace.rs:47`、`preset.rs:561`）、`synvelis-app/src/host.rs`（`ShellWindowHost`）、`synvelis-workspace/src/host.rs`（`FakeWindowHost`） |
| `WindowConfig` | `window.rs:146` | なし（struct） | `WindowHandle::create_window` の引数 | synvelis: `workspace.rs:193`（`WindowConfig` 組み立て） |
| `HostHandle` | `event.rs:78` | **0 件** | なし（trait 宣言と `lib.rs` の再 export のみ） | synvelis-app `app.rs:228,334` は doc コメントの参照のみ |

WebView adapter（`ruviform-ui-webview`）は legacy 型を使わない（`src/lib.rs:26`、`src/session.rs:22`、
README「旧 `HostEvent` / `HostHandle` / `WindowConfig` はこの crate で使わない」）。
GPUI adapter は gpui 型（gpui の `WindowHandle` を含む）を公開面に出さず、consumer は
`ruviform_ui::WindowDriver`/`DriverEvent` だけに依存する（`ruviform-ui-gpui/src/lib.rs:1-8`）。

### 再評価の条件（Phase 2 gate）

次をすべて満たしたとき、削除の可否を再評価する。

1. Synvelis が `WindowDriver` 経路で native 最小 Slice を動かし、`WindowHandle`/`HostEvent` の利用を外している
2. audio-lens の `al-gui` が `WindowLifecycle` を直接消費し、`HostEvent` 変換（`map_lifecycle`）を外している
3. 実 OS lane（`example-ui-gpui-lifecycle`・`example-ui-webview-e2e`）が両 consumer の経路で通っている
4. `HostHandle` については、公開 API 破壊として扱う場合の semver 方針と consumer への通知を決めている

`HostHandle` だけは実装者 0 件のため、条件 4 のみで単独に削除を検討できる。その判断は別 Issue で行う。

## Pros and Cons of the Options

### A: 今すぐ全削除

- Good: 旧 API の二重化が解消する
- Bad: consumer（synvelis・audio-lens）が同時に壊れる。gate 未達で、互換の出口がない

### B: 保留（採用）

- Good: 現行の runtime・テスト・consumer を壊さない。再評価条件を明示できる
- Bad: 旧 API と新 API の二重化が続く。Phase 2 までの期間、保守対象が増える

### C: `HostHandle` のみ削除

- Good: 実装者 0 件の型だけを減らせる
- Bad: 公開 API の破壊であり、consumer の doc 参照を壊す。削除理由が「未使用」だけなら、単独の Issue で行うべき

## Consequences

- 旧 API は本 PR 後も残る。`docs/designs/issue-386-driver-neutral-gui-host.md` の Phase 2 は未決のままとする。
- 本 ADR の status は `proposed`。PR レビューで採否を確定する。
- #431 の実機検証は旧 API に依存しない経路（`WindowDriver` と `run_multi`）で実施した。
