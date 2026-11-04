---
id: "SPEC-034S312gtUiTestSpecBB22"
type: "spec"
status: "approved"
title: "Issue #312 test-only GUI host lifecycle・secure control adapter・fixture/clock/observation 仕様"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/312"
    - "docs/discovery/issue-312-test-gui-host-lifecycle.md"
  changes:
    added:
      - type: "directory"
        path: "crates/ruviform-ui-test"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-156-gui-host-foundation.md"
    - "docs/adr/ui-architecture-webview-hybrid.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-312-test-gui-host-lifecycle.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-312-test-gui-host-lifecycle.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/310"
    - "https://github.com/scarecrow173/ruviform/issues/311"
    - "https://github.com/scarecrow173/ruviform/issues/313"
    - "https://github.com/scarecrow173/ruviform/issues/243"
  defers: []
  deferred-by: []
---

# Issue #312 test-only GUI host lifecycle・secure control adapter・fixture/clock/observation 仕様

## What Should Be Built

新クレート **`ruviform-ui-test`** — GUI Testing v1.0 / RF-GT-03 の共有テスト基盤。実 Window E2E（および headless 検証）で利用する **分離されたテスト用 Lifecycle/Control ポート** を、本番 `ruviform-ui` / `ruviform-ui-webview` の変更なしに提供する。

1. **`rvf.uitest.v1` 制御プロトコル**: `ruviform_ui::ipc` の versioned envelope（`Intent`/`Event`/`ErrorBody`）を再利用し、`test.*` 名前空間の閉じた op 集合を定義する。`test.hello` handshake（run/session ID・application/ruviform revision・capabilities・backend/window metadata の型付き交換 = ready/failed handshake）、fixture prepare、clock advance、許可 fault injection、read-only observation（windows/receipts）、screenshot（capability 宣言つき）、shutdown/cleanup の各 op。
2. **`TestIo` ポート**: agent が host/app 境界へ到達するための最小 trait（emit・windows・receipts・set_fault・prepare_fixture・screenshot・request_shutdown・capabilities・backend・ready_state・notices）。fake host・実 `run_multi` 配線の双方が実装できる形。
3. **`FakeHost`**: `WindowHandle` 実装の headless ホスト。`tests/multi_window.rs` の fake を一般化し、inject queue・ipc_disconnect 抑制・window metadata・bounded eval backlog を持つ。CI で実プロセス E2E 可能にする。
4. **Transport**: 既定 = 子プロセス stdio フレーム（OS 権限継承・ポート不要）。opt-in = loopback TCP（ephemeral port・run-bound secret・単一接続・loopback 接続元限定）。frame = u32le length + JSON、`MAX_INTENT_BYTES` 上限。
5. **`TestAgent`**: app-under-test 内に埋め込む制御サーバ。`serve(Read,Write)` の transport 非依存ループ + `serve_stdio`/`serve_tcp` helper。driver EOF → cleanup shutdown。`RVF_TEST_DISABLE` 実行時無効化。capability 未宣言 op は fail-closed `unavailable`。
6. **`TestDriver`**: テスト側 client。spawn（stdio/tcp）→ hello → typed op → notification drain → shutdown + child reap + kill-on-drop。
7. **`dummy_app` bin**: `DemoService` + `UiRuntime` + `FakeHost` + `HeadlessIo` を組み合わせた headless ダミー Consumer。test mode で起動 → handshake → fixture → 観測 → 正常/異常終了 → 完全 cleanup を実証する参照実装。

## Why Now

GUI Testing v1.0 の RF-GT-01（#310 browser harness）/ RF-GT-02（#311 IPC conformance）と独立に先行できる RF-GT-03 共有基盤であり、#313（RF-GT-04 OS driver）が実 Window 上で利用する制御契約の正本を先に確定させる必要がある。headless FakeHost 経路で全契約を CI 検証可能にし、実 OS windowed 証拠は #243 手動 gate に分離する。

## Users and Value

- Ruviform 自身の GUI host 契約テスト: 実プロセスを跨いだ lifecycle/handshake/cleanup の決定的 E2E。
- Consumer（Synvelis・AudioLens）の将来 E2E: 同一制御ポートで自アプリを test mode 起動できる。
- RF-GT-04（#313）: windowed host 配線済みの `TestIo`/`rvf.uitest.v1` 契約をそのまま消費する。

## Scope

### In Scope

- `crates/ruviform-ui-test` 新設（利用側層。`ruviform-ui` の公開 API のみに依存。本番クレートへの変更ゼロ）。
- `rvf.uitest.v1` op 集合・handshake・capability・cleanup 契約の型付き定義。
- stdio 既定 + loopback TCP opt-in transport、run-bound secret 認証、loopback 接続元制限、単一接続、実行時無効化。
- `FakeHost`（headless `WindowHandle`）+ `HeadlessIo` + `Journal`/`JournalHandle` 観測 + `ClockDomain::HostTick` advance + allowlist fault（`window_crash`/`ipc_disconnect`）+ fixture port。
- `dummy_app` 参照 bin（`DemoService` 駆動）+ `mise run test` 配下の unit・cross-process E2E テスト。
- 本 issue の doc chain（discovery → spec → ADR → design → plan → test-spec → tasks → impl record）。

### Out of Scope

- 実 Window での screenshot capture・OS イベント注入・DOM 操作・Golden 描画比較（RF-GT-04 #313 / RF-GT-05 / #243 手動 gate）。
- `ruviform-ui-webview` への実 windowed 配線実装（#313 が接続する。本 issue は headless FakeHost で全契約を検証）。
- Playwright/browser 側 harness（RF-GT-01 #310）、record/replay フォーマット（RF-GT-02 #311）。
- Unix domain socket transport、複数同時 control session、driver 再接続 resume。
- 製品 Project Controller / Authorization に関わる意味論 — テスト基盤はそれを迂回しても通常操作 E2E 合格とはみなさない（本ポートは観測・fixture のみ）。

## Requirements

- **Isolation by construction**: `ruviform-ui-test` への依存を持たないビルドには制御面が一切存在しない。`ruviform-ui`/`ruviform-ui-webview` のソース変更は行わない。
- **Explicit opt-in**: agent は app が `serve_*` を明示呼出し + env opt-in（`RVF_TEST_CONTROL`）した場合のみ待受ける。`RVF_TEST_DISABLE=1` は常に優先して fail-closed。
- **Fail-closed**: 未認証 op・secret 不一致・非 loopback 接続・未宣言 capability・`test.hello` 以外の pre-auth op はすべて `unauthenticated`/`unavailable`/`malformed_message` エラーで拒否し、状態を変えない。
- **Closed op set**: wire 上で任意コード・任意 eval・任意ファイル/shell アクセスを表現できない（op は enum、fixture は登録名参照、payload は `MAX_INTENT_BYTES` 上限）。
- **Honest capability reporting**: `capabilities` は実装済み事実のみを返し、`input_injection:false`・`dom_access:false`・未対応 backend の `screenshot:false` を含む。制御可能 clock domain は `host_tick` のみと宣言する。
- **Ownership discipline**: run/session ID と window/job 所有権は session 中一貫。`Destroyed` は window terminal で id 不再利用。driver 切断・restart・異常終了後に孤児 window/port/handle/job を残さない。
- **Non-RT only**: 全 API は非 RT。RT 契約区間から呼び出し不可。
- **`#[non_exhaustive]`**: 公開 enum は ADR の enum policy に従う。
- **Verification**: `mise run fmt-check`/`check`/`lint`/`test` green。

## Acceptance Criteria

- [ ] AC1: `dummy_app` を stdio test mode で spawn → `test.hello` Welcome（session/run ID・revisions・capabilities・backend metadata 取得）→ fixture prepare → `observe.windows`/`observe.receipts` 観測 → `shutdown graceful` → 子プロセス正常終了・孤児 window/port なし、が `mise run test` で再現する。
- [ ] AC2: 2 Window 作成・close veto（`request_close` + `respond_close Veto` で window 生存）・driver 側 restart（新 session ID）・`ipc_disconnect` fault（Message 未到達・clear で回復）・driver 切断（EOF → app cleanup 終了）の各ケースで制御ポート・Handle・Job 所有権が破綻しない（契約テスト）。
- [ ] AC3: `ruviform-ui-test` 非依存のビルド（既存 `check`/`build`/`test`）に制御面が存在しない;`RVF_TEST_DISABLE=1` で agent serve が即時拒否;TCP で secret 不一致・未送信・非 loopback が接続拒否;未宣言 capability op・`test.hello` 前 op が fail-closed エラー。
- [ ] AC4: `ruviform-ui`/`ruviform-ui-webview`/`cli-sample` 等他クレートに差分なし（`git diff` で確認）・`Test Host` を使わない CLI/DSP/RT consumer のビルド・実行に影響なし。
- [ ] AC5: 実 OS 未検証項目（windowed host 配線・実 screenshot・OS 権限限定 pipe 以外の attach 経路）を impl record で NotRun/Unsupported と明記し、RF-GT-04（#313）・#243 への引渡しを記録する。

## Deferred Design Concerns

- 実 windowed host への `TestIo` 配線例（`run_multi` + frame queue + tick dispatch）— #313 の作業時に追加。本 issue では `TestIo` ポート形状をその配線が成立する形に設計済み。
- screenshot capability の真の実装 — OS/backend 固有の capture API が必要。#313/#243。
- 制御 channel の複数接続・再接続 resume — v1 では単一 session・fail-closed。

## Sources and Relations

- Issue: https://github.com/scarecrow173/ruviform/issues/312 （合意済み GUI Testing v1.0 / RF-GT-03。本文の Acceptance が正本）。
- Discovery: `docs/discovery/issue-312-test-gui-host-lifecycle.md` — 既存 host 面の棚卸し・選択肢比較。
- 上位契約: `docs/designs/issue-156-gui-host-foundation.md`（host boundary）、`docs/adr/ui-architecture-webview-hybrid.md`、`docs/designs/ruviform-architecture-design-baseline.md`（層構造 A.2/A.6）。
- 関連 Issue: #310（RF-GT-01）、#311（RF-GT-02）、#313（RF-GT-04 — 本ポートの windowed 消費者）、#243（実 GUI 手動検証 gate）。
