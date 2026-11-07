---
id: "DESIGN-Z3hWk8NpR2tLq6VbYc4MdE"
type: "design"
status: "proposed"
title: "Issue #423 WebView 写像・focus capability 精緻化・GPUI 実窓検証の設計"
created: "2026-11-08"
updated: "2026-11-08"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/423"
  changes:
    added: []
    modified:
      - "crates/ruviform-ui-webview/src/session.rs"
      - "crates/ruviform-ui-webview/src/host.rs"
      - "crates/ruviform-ui-webview/src/lib.rs"
      - "crates/ruviform-ui/src/driver.rs"
      - "crates/ruviform-ui-gpui/src/lifecycle_fixture.rs"
      - "crates/ruviform-ui-gpui/src/render.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/issue-423-webview-dock-verify.md"
  implemented-by:
    - "../plans/implement-issue-423-webview-dock-verify.md"
  depends-on:
    - "../designs/issue-386-driver-neutral-gui-host.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../reports/issue-423-gpui-real-window.md"
  refines:
    - "../designs/issue-386-driver-neutral-gui-host.md"
  refined-by: []
  derives-from:
    - "../specs/issue-423-webview-dock-verify.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../designs/issue-386-driver-neutral-gui-host.md"
  defers: []
---

# Issue #423 WebView 写像・focus capability 精緻化・GPUI 実窓検証の設計

## 1. 方針

- 共通層（`ruviform-ui`）は変更しない。WebView 固有の型・JS・URL は adapter に閉じる。
- 例外は `focus` capability の契約文言（後述 §3）のみで、`ruviform-ui` の型は変えない。
- Phase 1 の写像は移行表 H1–H5 に従い、旧 `HostEvent` / `HostHandle` / `WindowConfig` を
  adapter の実装パスから外す。旧 trait は `ruviform-ui` に残置する（移行表 H1・H2・H5 の方針どおり）。

## 2. WebView adapter の写像（H1–H5）

adapter の実装は次の 2 ファイルに集約した。

| ファイル | 役割 |
| --- | --- |
| `session.rs` | tao loop・`Session`（window registry・close 状態）・`SessionHandle: WindowDriver`・`run_multi` |
| `host.rs` | wry webview 構築。page load → `Ready`、IPC body → `Action(Intent)` の decode（`inbound_message`） |

| 移行表 | 旧 API（adapter から除去） | 新形（adapter 内の写像） | 状態 |
| --- | --- | --- | --- |
| H1 | `HostHandle::eval(script)` | `WindowDriver::post_event` → `Session::post` が `evaluate_script(dispatch_script(event))`。未 `Ready` は有界 backlog | 実装済 |
| H2 | `WindowHandle::eval(window, script)` | H1 と同じ（`post_event`） | 実装済 |
| H3 | `HostEvent::Ready { url }` | wry page load → `WindowLifecycle::Ready`。URL は契約へ出さない（`host.rs` コメント） | 実装済 |
| H4 | `HostEvent::Message(String)` / `rvf.ipc.v1` | `inbound_message` → `parse_intent` → `WindowLifecycle::Action(Intent)`。decode 失敗は当該 window へ `Event::error`（consumer へは渡さない） | 実装済 |
| H5 | `WindowConfig { url, devtools, … }` | `WebViewWindowConfig { spec: WindowSpec, url, devtools }`。neutral `create_window` は `SessionConfig.default_url` / `devtools` を使う | 実装済 |
| H9 (WebView 側) | `run` / `run_multi`（legacy callback） | `run_multi(&SessionConfig, assets, data, FnMut(DriverEvent, &mut dyn WindowDriver))`。GPUI の `run` と同形 | 実装済（下記逸脱） |

### 逸脱: legacy 単一 window `run` の削除

#386 設計の Phase 1 は「旧 `run` は接続試験が通るまで残す」としていた。本 Issue では
`run`（`HostConfig` / `WebHost`）を削除し、単一 window も `run_multi` の 1 window 構成にした。
理由: 旧 `run` の callback は `HostEvent::Message(String)` を受けるため、typed
`Action` を通すには raw body の再構成が必要になり、写像の正本が二系統（`run` と `run_multi`）
になる。WebView の実窓接続は headless で再現できず、残置しても検証できないため、写像の
一本化を優先した。影響範囲は同 crate の 2 example（`demo_app` / `demo_multi`）のみ。
`ruviform-ui` の legacy 型は残置しており、他 consumer への影響はない。

### 例外的な routing（consumer 側の責務）

`Action` は `UiRuntime::dispatch_window_intent`、`Tick` は `drain_jobs_routed`、`CloseRequested`
は `closing` lifecycle の `post_event` で扱う。`Destroyed` は consumer の終了方針で判断する。
この routing は examples に示し、adapter は持たない。

## 3. focus capability の精緻化（決定）

現状: GPUI は `focus` を `Supported` と宣言するが、`focus(_, false)` は
`UiError::Unsupported`（gpui 0.2.2 は window の解放を提供しない）。

検討した選択肢:

- (a) GPUI で release を実装する → gpui 0.2.2 に API がなく、疑似実装は「silent no-op」の禁止に反する。不採用。
- (b) `WindowCapabilities` に `focus_release` 等のフィールドを追加する → 公開 struct の拡張で
  全 driver の構築箇所に影響し、#386 の契約凍結と衝突する。不採用。
- (c) `focus` の宣言の意味を「取得（`focused = true`）」に限定し、release は driver が
  操作単位で `Unsupported` を返しうると契約文書化する → **採用**。根拠: 共通 lifecycle fixture
  （`ruviform-ui-test/src/lifecycle.rs`）が既に「Ok か Unsupported、Host error ではない」を要求しており、
  契約文言だけがそれに追いついていなかった。

WebView adapter は `focus(false)` を `focus_parent()` で実装するため `Supported` のまま正しい。
GPUI の `focus(false)` は `Unsupported` を返し、`lifecycle_fixture.rs` の
`gpui_focus_capability_covers_acquire_and_reports_release_unsupported` で対を固定する。

## 4. Late receipt の描画検証

既存の `lifecycle.rs` は late receipt を inbox 件数（`received == 1`）でしか見ていなかった。
`render.rs` の `rvf_driver_late_receipt_projects_into_view_after_ready` は次を通す。

1. `ViewHost` 上で fader の command を commit し、`command_id` を得る（`commit_on_host`）。
2. `GpuiDriver::create_view_window` で mount し、窓が `Ready` になる前に `result_ok(applied)` を
   `post_event` する（inbox に保持されることを件数で確認）。
3. `settle_for_test` で窓を開き、描画後に inbox が空になり、binding phase が `Applied` であること
   （= 描画経路で projection されたこと）を確認する。

## 5. GPUI 実 window の検証

- `example-ui-gpui-minimal`（実 OS の GPUI window）を起動し、`Created` / `Ready` /
  `Focused(true)` / `Resized` / `Moved` が実機で届くことを確認した（結果は報告書に記録）。
- Dock 2-phase move / detach-reattach / close veto は headless fixture（`ruviform-ui-test`、
  Fake/GPUI 同一 trace）で検証済み。実 window 上での Dock 統合は minimal fixture に Dock UI がないため
  未実施。手順は報告書に文書化する（A3 の「実機手順の文書化」）。

## 6. 検証方針

- 変更した crate の lane を実行する: `check-ui-webview` / `lint-ui-webview` / `test-ui-webview`、
  `test-ui-gpui` / `lint-ui-gpui` / `check-ui-gpui`、`check-ui-driver-boundary`、
  `test-ui-driver-contract`、`test-ui-component-view`、`test-ui-test-msrv`。
- 最終検証は merge 後 HEAD で実施する（実施結果は impl record に記録）。

## 7. 未達・NotRun

- 実 OS lane（DPI・IME・OS focus・実 close・GPU device lost）: NotRun（consumer 受入 #262/#243/#390 系）。
- GPUI 実 window 上の Dock 2-phase 統合: NotRun（手順のみ文書化）。
- WebView の実 WebView2 上での end-to-end（page load → IPC → event 往復）: NotRun
  （この環境では demo を手動実行していない。`cargo run --example demo_app` の手順は既存 README のとおり）。
