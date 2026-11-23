---
id: "DESIGN-pgreq873chNeuVrbAjd9x"
type: "design"
status: "draft"
title: "Issue #873 plugin→host GUI 要求チャンネル設計（GuiRequest/GuiRequestSink + session 境界 arbiter）"
created: "2026-11-23"
updated: "2026-11-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/873"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-873-plugin-gui-request.md"
      - type: "file"
        path: "docs/specs/issue-873-plugin-gui-request.md"
      - type: "file"
        path: "docs/designs/issue-873-plugin-gui-request.md"
      - type: "file"
        path: "docs/test-specs/issue-873-plugin-gui-request.md"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/gui_request.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/boundary.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/plugin.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/editor_session.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/host.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/host.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-873-plugin-gui-request.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-389-vst3-l0-editor-attach.md"
    - "docs/designs/issue-388-native-surface-slot.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-873-plugin-gui-request.md"
    - "docs/test-specs/issue-873-plugin-gui-request.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-873-plugin-gui-request.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-873-plugin-gui-request.md"
  references: []
---

# Issue #873 plugin→host GUI 要求チャンネル設計

## Context

Issue #873 は `#389` の AC「plugin-side preferred size/resize 拒否」受入ケースに必要な
plugin→host 方向の GUI 要求チャンネルを設計・実装する slice。現状は host→plugin
方向の verb（`HostCommand` の `OpenGui`/`SetGuiSize` 等）のみ存在し、plugin 発の
要求を host へ届ける経路がない。VST3 `IPlugFrame::resizeView` は無条件
`kResultFalse` の defined refusal、CLAP `clap_host_gui`（`request_resize`/
`request_show`/`request_hide`/`resize_hints_changed`/`closed`）は `HostGui` 未宣言で
plugin 側から到達不能。

設計の核は形式間の非対称:

- **VST3 `resizeView`** — plugin の UI スレッド上で同期的に呼ばれ、その呼出で即答が
  必須。accept なら container effect が確定した後 plugin へ `onSize` で往復完了する
  roundtrip が契約。
- **CLAP `request_*`** — `[thread-safe]` で任意スレッドから呼ばれ得、async ack が
  許容（clack `HostGuiImpl::request_resize` doc: "a successful return value may only
  mean the Host acknowledged the request, and will process it asynchronously later.
  If the request is later found not to be able to be satisfied, then the host will
  call the plugin's `set_size` method to revert the operation"）。
- `resize_hints_changed`/`closed` — verdict 不要の通知。

container geometry の所有は slot 側（`scarecrow173/ruviform#388` の `NativeSurfaceSlot`/
`set_geometry` — `docs/designs/issue-389-vst3-l0-editor-attach.md` D2）。本 slice が
露出するのは session 境界の裁定 seam までで、実際に slot geometry を動かす裁定終端は
ui/ui-gpui driver 側の別範囲。テストでは scripted arbiter が代行する。

## Decisions

### D1 — 責務境界: session 境界 arbiter callback + drain queue 併用を採用

plugin 発要求を受理・裁定する主体は、構成側（GUI driver）が注入する裁定 callback
`GuiRequestArbiter`。`LeaseRedeemer`（`editor_session.rs`）と同型の注入 seam とし、
`InProcessEditorOps` が `Arc<Mutex<dyn FnMut(PluginEditorSessionId, GuiRequest) -> GuiRequestVerdict + Send>>`
として保持する。

- **採用 (A) session 境界の arbiter callback（+ queue 併用）** — 同期 verdict を
  返せるため VST3 `resizeView` の同期応答契約と整合。`Arc<Mutex<>>` 共有にするのは
  plugin 側 sink が `'static` を要求し ops の借用（`'a`）を越えて生存するため。CLAP
  の `[thread-safe]` verb は adapter 側 queue に enqueue → ops の drain → 同じ
  arbiter へ流す併用形。
- **棄却 (B) drain queue 単独** — `drain_events`/`drain_output_reports` と同型の
  pull 経路は async ack 許容の CLAP 系には足りるが、VST3 `resizeView` は呼出
  スレッド上での即答が契約のため queue だけでは honest な `kResultOk` を返せない。
- **棄却 (C) adapter 内裁定** — container geometry は slot 側所有（#388・D2）のため
  adapter は裁定主体にできない。`Accepted` を返すには container effect の確定が
  必要で、adapter に effect 権限はない。

### D2 — 中立語彙は `ruviform-plugin-core::boundary` に置く（条件付き lease 採用）

`GuiRequest`/`GuiRequestVerdict`/`GuiRequestRefusal` は `boundary.rs` に配置する。
`GuiSize`・`GuiKeyEvent`・`ParentSlotLease` と同じ「host・(将来の)wire・plugin 契約が
共有する plain-data 語彙」の置き場であり、次の理由で plugin-host 内に閉じない:

- adapter（`ruviform-plugin-vst3`/`ruviform-plugin-clap`）は plugin-host に依存しない
  が plugin-core には依存する。語彙を plugin-host に置くと adapter が名指せない。
- `scarecrow173/ruviform#872` の将来 wire 化で二重定義を生まない（wire 動詞・
  `WIRE_VERSION` 自体は本 slice で変更しない）。

一方 `GuiRequestSink`（`dyn FnMut` trait object）は plain-data でないため
`plugin.rs` に置く。`boundary.rs` は plain-data のみの規約を維持する。

### D3 — 通知と要求は同一 enum `GuiRequest` に載せる

`ResizeHintsChanged`・`Closed{was_destroyed}` のような裁定不要の通知も同じ enum に
乗せる。要求系と通知系で sink/queue を二重化するだけの意味的利得がなく、通知への
verdict `Accepted` は「受理した（ack）」を意味する。`Refused` は通知に対しても
「経路なし」として発生し得る。

```rust
pub enum GuiRequest {
    Resize { size: GuiSize },        // VST3 resizeView / CLAP request_resize
    Show,                            // CLAP request_show
    Hide,                            // CLAP request_hide
    ResizeHintsChanged,              // CLAP resize_hints_changed（通知）
    Closed { was_destroyed: bool },  // CLAP closed(was_destroyed)（通知）
}
```

### D4 — verdict 語彙と同期/非同期セマンティクス

```rust
pub enum GuiRequestVerdict {
    Accepted { granted: Option<GuiSize> },
    Refused { reason: GuiRequestRefusal },
}

pub enum GuiRequestRefusal {
    Unrouted,   // arbiter/sink 未接続 — 「何も listen していない」
    Rejected,   // arbiter/container の policy 拒否
    Busy,       // 再入・競合で裁定チャンネルが占有中（形式ごとの retry 可）
    Gone,       // editor/session が裁定前に converge した
}
```

- `Accepted` は「container effect 確定」の意味を持つ。effect 未確定のまま
  `Accepted` を返して成功を偽装しない。`granted` は resize 系で container が実際に
  適用した size（補正時 `Some`、そのまま適用時 `None`）。非 resize 要求では
  常に `None`。
- **VST3**: `resizeView` → sink → arbiter 同期 verdict。`Accepted{granted}` →
  adapter が `resizeView` 内で `view.onSize(granted or requested)` を呼んで往復完結
  → `kResultOk`。`Refused`/sink 未接続 → `kResultFalse`（従来の defined refusal と
  同じ honest 応答）。`canResize`/`checkSizeConstraint` は通さない — plugin 自身が
  提案した size であり、container 側制約は arbiter の `granted` で表現される
  （host→plugin `set_gui_size` chain との差異として記録）。
- **CLAP**: `request_*` → 共有 queue へ enqueue し `Ok(())`（enqueue 受理 ack —
  clack/CLAP 契約の async ack そのまま）。`Result` → ABI bool は「enqueue 受理の
  ack」であり裁定 verdict ではない（契約化）。queue は bounded（定数 `MAX_GUI_REQUESTS`）
  で、full 時は `Err` → `false` の honest な背圧。`closed`/`resize_hints_changed` は
  戻り値を持たない通知 — full 時は defined drop。
- drain → arbiter → `Accepted` 後の plugin 側 reply（`session.set_size`/
  `set_visibility`）は driver/consumer の責務。ops が自動で session レベル verb を
  呼ぶと session state（Attached↔Hidden）の二重更新になるため、ops は配送 seam +
  verdict までに留める。

### D5 — Plugin/ErasedPlugin trait hook

```rust
pub type GuiRequestSink = dyn FnMut(GuiRequest) -> GuiRequestVerdict + Send;

// Plugin / ErasedPlugin
fn set_gui_request_sink(&mut self, _sink: Option<Box<GuiRequestSink>>) {}
fn drain_gui_requests(&mut self, _out: &mut Vec<GuiRequest>) {}
```

- `set_gui_request_sink` — host が install/detach する同期 sink（VST3 用）。
  `Send` は Shared slot 経由でも `dyn ErasedPlugin` trait method の引数として
  合法的にするため。デフォルトは sink を drop — 同期経路を持たない plugin
  （CLAP 等）は従来どおり defined refusal を維持する。
- `drain_gui_requests` — queue 側の pull 経路（CLAP 用）。デフォルト no-op。
- `InProcessHost` 側は `set_gui_request_sink(handle, sink) -> Result<(), HostError>` と
  `drain_gui_requests(handle, out) -> Result<(), HostError>` を公開 — Plain 直叩き・
  Shared slot は `shared_verb` 経由で `dyn ErasedPlugin` trait method を呼ぶ。
  `plugin_mut` は `&mut dyn ErasedPlugin` で downcast 不可・`HostCommand` は
  host→plugin plain-data・adapter は自身の handle を知らず registry 鍵を持てない
  ため、trait hook 以外の honest な代替 seam が存在しない。
- 配送は `HostCommand` variant にしない — 方向が逆（命令語彙は host→plugin）。
  要求面は session/ops 経路の別 surface。

### D6 — ops/session 境界の seam

- `EditorSessionOps` trait に 2 動詞を追加（default 付き）:
  - `drain_gui_requests(session, out) -> Result<(), EditorOpError>` — default
    `Ok(())`（queue 経路を持たない adapter は何も drain しない）。
  - `adjudicate_gui_request(session, request) -> Result<GuiRequestVerdict, EditorOpError>`
    — default `Ok(Refused{Unrouted})`。
  - `dispatch_gui_requests(session, out)` — drain + adjudicate の trait 提供
    便利ループ（per-request verdict を drain 順に返す）。
- `L1EditorOps` は両方とも `Err(Unsupported)`（全 verb 拒否の不変条件を維持）。
- `InProcessEditorOps` に `arbiter: Option<GuiRequestArbiterHandle>` を保持。
  `with_redeemer_and_arbiter` 新設（`with_redeemer` は `arbiter: None` に委譲）。
- `open` 成功時: arbiter 設定済みなら `host.set_gui_request_sink` で同期 sink を
  install。sink 内部は arbiter を `try_lock` し、`WouldBlock` → `Refused{Busy}`、
  `Poisoned` → `Refused{Gone}` で re-entrant FFI を panic させない。install 失敗
  （handle 不正・Shared converge）は GUI を rollback close して `Err` —
  「open ⇒ request route live」の不変条件。arbiter 未設定なら sink も install
  しない（従来の `Unrouted` 応答と同等）。
- `close` 成功時: `set_gui_request_sink(None)` で detach。close 失敗時は GUI
  残存 → sink も残存が整合。
- drain 側 reply 完結（`session.set_size`/`set_visibility`）は driver の composition。
  ops レベルの dispatch は verdict までで、session state を動かす session-level
  verb の呼出は呼出側に委ねる。

### D7 — test fixture 発火・real lane 行追加は不採用

- roundtrip 検証の起点は (i) `crates/ruviform-plugin-host/tests/` の scripted
  Plugin（sink capture + scripted queue + verb log）+ real `InProcessHost` +
  real `InProcessEditorOps` + scripted arbiter で headless 完結。test-vst3/
  test-clap fixture の要求発火（title event log token 先例）は採用しない。
- real dock lane の plugin 発要求 real 行追加は本 slice 対象外（`real_catalog.rs`/
  `real_dock.rs` は `scarecrow173/ruviform#874` が占有）。real lane への影響は
  非回帰確認に留める。
- adapter 側テストは headless: VST3 は `BridgePlugFrame` + COM double view で
  `resizeView` の verdict 写像と `onSize` 往復を直接検証、CLAP は `BridgeShared`
  の `HostGuiImpl` + `ClapPlugin::dormant` の queue で enqueue→drain 写像を検証。

### D8 — `docs/designs/issue-389-vst3-l0-editor-attach.md` D2 との reconcile

389 design D2 が定めた「`resizeView` は無条件 `kResultFalse` の defined refusal」を
本契約が更新する: refusal は「request route なし（arbiter 未注入・sink 未接続）」
時のみであり、route live なら arbiter verdict に従う。container geometry が slot
側所有という D2 の責務境界自体は変更しない（本設計の棄却肢 C と同じ根拠）。当該
doc は本 slice の編集範囲外のため直接改訂せず、本 doc で「#873 契約が D2 の
refusal 記述を更新する」旨を宣言する。

## 不変条件・契約

- `Accepted` は container effect 確定のみを意味する — effect 未確定で `Accepted`
  を返さない（resize 成功の偽装禁止）。
- `kResultOk`/`true` の honest 性: VST3 では verdict `Accepted` + `onSize` 完結の
  み `kResultOk`。CLAP では `Ok(())` = enqueue 受理 ack（裁定 verdict ではない）。
- re-entrant FFI は panic でなく定義済み応答 — sink/queue 側は `try_lock`/
  `try_borrow` 系で `Refused{Busy}`/`kResultFalse`/`false` に写像。
- 中立語彙に `ruviform-ui` 型・raw OS handle・plugin-standard 具象型（`ViewRect`/
  `ClapGuiSize`/`IPlugFrame` 等）を出さない。変換は adapter 内部で完結する。
- request 面は `HostCommand` に乗せない — host→plugin 命令語彙と方向が混ざらない。
- wire 動詞・`WIRE_VERSION` は不変（L1 GUI verb は #872 範囲 — L1EditorOps は
  全 verb `Err(Unsupported)` を維持）。
- queue は bounded — 無限増大しない（`MAX_GUI_REQUESTS`）。RT path は関与しない
  （GUI callback は非 RT・queue 操作は非 RT 経路のみ）。

## Consumer 影響分類

- `ruviform-plugin-core`: 新規公開型追加のみ（`GuiRequest`/`GuiRequestVerdict`/
  `GuiRequestRefusal`/`GuiRequestSink` + `Plugin`/`ErasedPlugin` default method）。
  既存 consumer への破壊的変更なし。
- `ruviform-plugin-host`: `EditorSessionOps` に default 付き動詞 3 件追加 — 既存
  impl への破壊的変更なし（`L1EditorOps` は明示 impl を追加）。`InProcessEditorOps`
  に新 constructor 追加、既存 `with_redeemer` は維持。
- `ruviform-plugin-vst3`/`ruviform-plugin-clap`: adapter 内部の接続追加。公開面の
  変更なし。
- ui/ui-gpui driver（consumer）: 本 slice では seam 露出のみ — 実配線は別 slice。

## Deferred Design Concerns

- L1/out-of-process 経由の GUI 要求配送（wire verb 化 — `scarecrow173/ruviform#872`）。
- ui/ui-gpui driver 側の arbiter 実装（slot geometry 実 effect — `scarecrow173/ruviform#388` 系）。
- `closed(was_destroyed)` 受領時の session 自動 converge（driver 側で
  `notify_terminated` 系に写すかは consumer の compose 判断 — 本 slice は配送まで）。
- CLAP `resize_hints_changed` 受領後の `get_resize_hints` 再問合せ（plugin 側の
  `get_resize_hints` verb は neutral surface に未露出 — 必要になった時点で別途）。
- test fixture（test-vst3/test-clap）からの実 ABI 経由の要求発火 + real dock lane
  の plugin 発要求行（`scarecrow173/ruviform#874` 系の後続 slice）。
