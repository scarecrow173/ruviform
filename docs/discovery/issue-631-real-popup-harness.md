---
id: "DISC-01M7Y96P5AVRW761HK2XP77XF2"
type: "discovery"
status: "resolved"
title: "Issue #631 real-editor-popup 面 — Issue #630 AC4 共有残課題の real-HWND host harness を real-dock lane に置くための調査"
created: "2026-11-21"
updated: "2026-11-21"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
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

# Issue #631 real-editor-popup 面 — Issue #630 AC4 共有残課題の real-HWND host harness を real-dock lane に置くための調査

## Summary

`docs/specs/issue-630-consumer-oracle.md` が Issue #630 AC4 の
「editor popup の real-HWND host harness / L1 editor 面」を
Issue #631 の real-dock lane に割り当てている。fixture 側の
popup 契約は Issue #855 で確定済みだが、real-dock lane には
popup を駆動・観測する case が存在しなかった。本 slice は
`real-editor-popup` を追加し、`session.send_key` の `VK_P`
toggle で fixture popup を open/close し、その実 HWND を
OS 観測で検証する。

調査の結論:

- 駆動面は `real-editor-input` が既に証明済みの production
  chain（`session.send_key` → `InProcessEditorOps` →
  `HostCommand::SendGuiKey` → `IPlugView::onKeyDown/onKeyUp`）
  をそのまま使える。新規の駆動経路は不要。
- title event-log oracle（`GetWindowTextW` suffix 照合）は
  verb case と共通なので `title_log_check` に集約した（DRY）。
- top-level popup HWND oracle は `ospres::inventory()` の
  `own_windows` で成立するが、`TopLevelWindow` に `title`
  欄がなく fixture の `rvf-test-editor-popup-{seq}` identity
  を OS 観測できなかった — query-only の `GetWindowTextW` 欄を
  1 つ追加する最小拡張で解消した。
- popup は `WS_POPUP` top-level のため
  `EnumChildWindows(container)`（`oschild` の attach 観測）には
  一切現れない。子孫 inventory では発見不能であり、top-level
  inventory が唯一の実 HWND oracle である。

## Fixture 契約（Issue #855 確定済み — 変更不可）

`test_editor` `IPlugView` は `POPUP_VK`（`VK_P` = `0x50`）の
`onKeyDown` で `popup_toggle` を呼ぶ:

- open: `popup_seq` インクリメント → `win32::popup` が
  `RuviformTestEditor` class の `WS_POPUP` top-level HWND を
  screen origin (0,0)・200x100 で作成（owner =
  `GetAncestor(child, GA_ROOT)`、`GW_OWNER` として観測可能）
  → title log に `po:{seq}` を追記。
- close: `win32::destroy` で popup を破棄 → title log に
  `pc:{seq}` を追記。
- popup title は `rvf-test-editor-popup-{seq}`。`popup_open`
  は fixture の論理フラグであり、real-HWND oracle ではない
  （実装注記として fixture に明記済み）。
- `removed()` は popup も一緒に破棄する — teardown での残存は
  契約違反かつ lane 衛生上の危険（(0,0) 200x100 が後続 case の
  観測を遮蔽する）。

## 観測面の既存資産と不足分

| 観測層 | 既存 | 本 slice での使い方 |
| --- | --- | --- |
| `observe_attached_children` | attach 契約 + `GetWindowTextW` title 採取 | open/close 各 toggle 後に title event-log を再採取 |
| `title_log_check` | verb case 内の inline suffix 照合 | 共有 helper に集約し popup の 2 phase で再利用 |
| `ospres::inventory()` | own/foreign top-level 仕分け + class/rect/owner/pid | popup identity の発見層（`own_windows` を絞り込み） |
| `ospres::TopLevelWindow::title` | **不在** | 新規追加（`GetWindowTextW`、query-only） |
| `osmut::observe(child).root` | `GA_ROOT` ancestry 解決 | popup の `GW_OWNER` 期待値（`expected_owner`） |
| `oschild::inventory(hwnd)` | `GWL_STYLE`/`GetParent`/`IsWindow`/rect の query-only 採取 | popup の `WS_POPUP`/`WS_CHILD` clear/alive/rect 検証と close 後の死亡確認 |

## verdict 写像の決定

`editor_op_body` の 4 値写像をそのまま採用する:

| 状況 | verdict |
| --- | --- |
| pre-flight（fixture なし・container lease なし・artifact 不在） | `NotRun`（fail-closed、`environment_blocked`） |
| `send_key` open toggle が `Ops(Unsupported)` で拒否 | `Unsupported`（defined refusal — honest boundary） |
| open/close が受理されたが title log・HWND・消失・teardown のいずれかが不成立 | `Fail`（accepted-but-mismatch — 決して `NotRun` に降格しない） |
| 全 oracle 成立 + `Attached` 維持 + `[Attached{Docked}, Closed]` 収束 + 残存 popup 0 | `Pass` |

補助判断:

- close toggle は open が `Accepted` のときだけ走る。refused/
  failed open のあとに `VK_P` を再打つと fixture は逆に popup
  を「開く」ため、証拠を汚染し未観測の popup を残し得る。
- close toggle が受理後に拒否/失敗した場合は contract surprise
  として `Fail` 記録する（`Refused` landing にはしない — op が
  存在することは open で実証済み）。
- teardown（close 後の残存 popup 確認）は landing に依らず
  supported 環境では常に実行する。`verified` の入力に含め、
  残存があれば `Fail`。

## step 配列

`editor_popup_open`（send_key 受理/拒否）→
`editor_popup_open_log`（title suffix `…|kd:0070:0050:0000,po:1,ku:0070:0050:0000`）→
`editor_popup_open_hwnd`（top-level inventory で popup 1 個・
class/title/owner/alive/`WS_POPUP`/`WS_CHILD` clear/(0,0)200x100）→
`editor_popup_close`（2 回目 send_key）→
`editor_popup_close_log`（`…,pc:1` までの suffix）→
`editor_popup_closed_hwnd`（inventory から消失 + 記録 HWND
`IsWindow==0`）→ `session_state_after_op`（`Attached` 維持）→
`session.close`/`child_hwnd_detached`/`session_concluded`/
`session_events`（既存 `conclude_editor_session`）→
`editor_popup_teardown`（owner 非依存の残存 0）。

## popup identity の絞り込み

`own_windows` を次の 3 条件で絞る — class
`RuviformTestEditor`、`title` が `rvf-test-editor-popup-`
prefix、`owner` == `osmut::observe(child).root`（`GA_ROOT`）。

- class だけでは child editor と popup を区別できない（同一
  class を共有するため `WS_POPUP` style と top-level 列挙が
  判別子）。
- owner が解決不能（`None`）なら 0 件一致 = 記録済み `Fail`。
  wildcard 的に owner 不問で拾わない。
- 残存確認だけは owner 非依存で絞る — owner も anchor chain も
  死んだ leak は owner 一致では発見できないため。

## 件数 pin の波及

real catalog 28 → 29、console 投影 37 → 38。`real_entries`・
`blocked_reports`・`blocked_probe_violations`・`REAL_CATALOG`・
`tests/plugin_dock.rs`・`tests/console.rs`・`lib.rs`/
`real_dock.rs`/`real_catalog.rs` の module doc を同一 diff で
整合させた（catalog の並び順 `real-editor-focus` の直後、
`real-session-crash` の直前）。

## スコープ外（明記）

- Issue #630 本体の consumer 実走 gate（AC2 実行面）は本
  slice では触らない — host harness のみ。
- fixture / plugin-host / production crate は変更しない。
- popup の画素 oracle は対象外 — 判定は title event log と
  HWND inventory のみ（画素を見ない non-pixel family）。
- `docs/**/README.md` の索引は本 PR では同期しない
  （Coordinator が統合時に同期する運用のため）。
