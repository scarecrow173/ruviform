---
id: "TSPEC-034YtAq6r0vJ8IFSxWqurc"
type: "test-spec"
status: "approved"
title: "Issue #389 editor-session real-session-terminate テスト仕様"
created: "2026-11-16"
updated: "2026-11-16"
owners: []
relations:
  source:
    - "docs/specs/issue-389-editor-session.md"
    - "docs/designs/issue-389-editor-session.md"
  changes:
    added:
      - type: "doc"
        path: "docs/test-specs/issue-389-editor-session.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-389-editor-session.md"
    - "docs/designs/issue-389-editor-session.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-389-editor-session.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-389-editor-session.md"
    - "docs/designs/issue-389-editor-session.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-389-editor-session.md"
  verified-by: []
  references:
    - "evidence/rf389/"
  defers: []
  deferred-by: []
---

# Issue #389 editor-session real-session-terminate テスト仕様

## Purpose

`real-session-terminate` case が「attach 中の host 破棄 →
`Terminated` exactly-once → child HWND teardown → host 存続」を
real OS window 上で正直に判定することを保証する。本仕様は
Issue #389 の real-window session lifecycle L2 検証の最小 1 case
を固定し、後続の move/degraded/crash/recovery case が再利用する
観測・verdict 規律の型を残す。

## Feature

real-window lane（`rf631.plugin-dock-real`）の session 終了経路:
attach 済み `test_editor` session に対し host instance を破棄し、
session registry が `Terminated{HostLost}` をちょうど 1 回だけ
emit して `Closed`+`concluded` に収束し、実 child HWND が OS
観測で teardown され、host が存続することを判定する。

## Rules

- R1: catalog 一致 — `REAL_CATALOG` と `real_entries()` は
  case/scenario/required が pairwise 一致し、両者 13 行。
  `real-session-terminate` の `required` は
  `&[ScenarioCapability::NativePluginDock]`。
- R2: attach 前提 — case 本体は `attempt_real_attach` の
  `observe_open` seam でのみ動き、refused open は defined
  `Unsupported`、それ以外の失敗は `Fail`、setup 失敗は `Fail`、
  全 step 成立のみ `Pass`。
- R3: host 破棄順序 — instance は `Activated` のため
  `HostCommand::Deactivate` → `HostCommand::Destroy` の順で
  実行し、各 command の結果を個別 step row に記録する。
- R4: session 収束 — `sessions.notify_terminated(handle,
  SessionFailCause::HostLost)` 後、session は `Closed`+
  `concluded` で、drain した event 列はちょうど
  `[Attached{Docked}, Terminated{HostLost}]`（`Closed` event は
  graceful close 専用のため含まれない）。
- R5: exactly-once — `by_instance` binding 除去済みのため 2 回目
  の `notify_terminated` は no-op で、追加 event を生まない。
- R6: child HWND teardown — attach 中に記録した全 child HWND が
  `IsWindow(child)==0`（destroyed）であること。
- R7: host 存続 — `HostCommand::Scan` が成功し、破棄済み handle
  への command が `HostError::InvalidHandle` で拒否されること
  （command surface 生存 + instance 死亡の同時観測）。
- R8: fail-closed — headless/非対応環境では全行
  `NotRun`+`observed.environment_blocked`・
  `environment_kind=real-window` 不請求。`EnvironmentKind::
  RealWindow` は実 session 内のみ emit。
- R9: 非回帰 — 既存 real 12 case・headless lane 8 case・console
  `--list` projection の verdict/行数を本 slice で動かさない
  （console は 20→21 行の意図的追従を除く）。

## Examples

- E1（実走・全 step 成立）: Given test_editor が Native Dock で
  `Attached{Docked}` 済み、When `Deactivate`+`Destroy`→
  `notify_terminated(HostLost)`→再 notify→child 再列挙→
  `Scan`+破棄 handle command、Then events =
  `[Attached{Docked}, Terminated{HostLost}]`、session
  `Closed`+`concluded`、全 child `IsWindow==0`、`Scan` 成功・
  破棄 handle は `InvalidHandle`、case `Pass`・
  `environment_kind=real-window`。
- E2（refused open）: Given editor open が defined
  `Unsupported` を返す環境、When case が走る、Then verdict は
  `Unsupported` で host 破棄 step には進まない。
- E3（headless blocked）: Given 実 Window を持たない host、
  When `--blocked-probe` または real lane が走る、Then 新行を
  含む全 13 行が `NotRun`+`observed.environment_blocked` で
  violation 0・exit 0。

## Guarantees

- `Terminated{HostLost}` がちょうど 1 回だけ emit され、2 回目の
  notify・late callback で重複しない。
- graceful close（`session.close()`）とは区別され、event 列に
  `Closed` event が混入しない。
- child HWND は破棄前に記録され、破棄後に `IsWindow==0` で
  OS レベル teardown を確認する。
- host/GPUI window は instance 破棄後も存続し、command surface
  が応答する。
- verdict は実走した step のみ `Pass`、未実行を `Pass` にしない。

## Non-goals

- session move/degraded/crash/hang/recovery、tick/deadline 実駆動、
  実 Window RT 非干渉、adapter verb 実装、`real-dcomp-pixel`
  画素、headless lane catalog の変更、Issue close。

## Risk

- exactly-once・teardown・host 存続のいずれかの保証が落ちると、
  host loss 経路の session lifecycle が「見かけ上 close したが
  実は生きている/二重通知する」状態を検出できず、後続の
  crash/recovery case の土台が崩れる。
- verdict を実走なしに `Pass` にすると POLICIES.md §2.1 の
  fail-closed 規範に違反し、headless/fake 証拠を実 Window
  Pass に転用する誤りを生む。
