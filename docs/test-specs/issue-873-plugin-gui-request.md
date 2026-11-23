---
id: "TSPEC-873pgGuiReqCh03"
type: "test-spec"
status: "approved"
title: "Issue #873 plugin→host GUI 要求チャンネル — test specification"
created: "2026-11-23"
updated: "2026-11-23"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-873-plugin-gui-request.md"
    - "docs/designs/issue-873-plugin-gui-request.md"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-873-plugin-gui-request.md"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/gui_request.rs"
  implements:
    - "docs/specs/issue-873-plugin-gui-request.md"
    - "docs/designs/issue-873-plugin-gui-request.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-873-plugin-gui-request.md"
  blocks: []
  supersedes: []
  superseded-by: []
---

# Issue #873 test specification

## Feature: plugin→host GUI 要求チャンネル（Issue #873）

検証構成は design D7: headless scripted ops/plugin + scripted arbiter で
roundtrip を完結させ、adapter 側は COM/Rust double で verdict 写像を直接
検証する。real dock lane への影響は非回帰確認（既存行 verdict 不変）。

### Rule TS-873-1: session open で request route が install される

**Example**: arbiter 注入済み `InProcessEditorOps` で `open` 成功 →
plugin の `set_gui_request_sink(Some(_))` が呼ばれ request route live。
`close` で sink が detach（`None`）される。arbiter 未注入の `open` は
従来どおり sink 未接続のまま成功する。

実装: `crates/ruviform-plugin-host/tests/gui_request.rs`
（`l0_open_installs_sink_and_delivers_resize_to_arbiter`/
`l0_close_detaches_the_request_route`/
`l0_open_without_arbiter_installs_no_sink`）。

### Rule TS-873-2: 同期 sink 経路 — accept は arbiter verdict を返す

**Example**: scripted arbiter が `Accepted{granted: Some(320×240)}` を返す
構成で、plugin → sink(`GuiRequest::Resize{640×480}`) → verdict が
`Accepted{granted: Some(320×240)}` と一致。scripted arbiter が `Refused`
を返せば同じ `Refused` が plugin へ戻る。

実装: `gui_request.rs`（`l0_sink_refusal_maps_arbiter_verdict`/
`l0_accepted_resize_roundtrips_through_session_reply`）。

### Rule TS-873-3: queue 経路 — enqueue → drain → adjudicate

**Example**: plugin の scripted queue に積まれた要求が
`drain_gui_requests` で取り出され `dispatch_gui_requests` が同じ
arbiter の verdict を drain 順に返す。queue を持たない plugin の
drain は空を返す。

実装: `gui_request.rs`（`l0_dispatch_drains_queue_and_adjudicates_in_order`）。

### Rule TS-873-4: refuse で plugin 側 verb は非実行

**Example**: arbiter `Refused` 時に plugin 側の GUI verb log
（`set_size`/`set_visibility` 相当）に記録が残らない — refuse を
「実行済み」に見せない。

実装: `gui_request.rs`（`l0_refused_request_notifies_nothing`）。

### Rule TS-873-5: L1 は全 verb `Unsupported`

**Example**: `L1EditorOps` の `drain_gui_requests`/
`adjudicate_gui_request`/`dispatch_gui_requests` は全て
`Err(EditorOpError::Unsupported)` — 既存 verb の拒否不変条件と同じ。

実装: `gui_request.rs`（`l1_ops_refuse_the_whole_request_channel`）。

### Rule TS-873-6: VST3 `resizeView` — verdict 写像と `onSize` 往復

**Example**: `BridgePlugFrame` + COM double `IPlugView`:

- sink `Accepted{granted: Some(320×240)}` → `kResultOk` + `onSize` に
  `(320, 240)` が届く（granted 適用）。
- sink `Accepted{granted: None}` → `kResultOk` + `onSize` に
  requested `(640, 480)`（verbatim 適用）。
- sink `Refused`/sink 未接続/sink cell borrow 中（re-entrant）→
  `kResultFalse`、`onSize` 非呼出。
- null view/null rect → `kInvalidArgument`。

実装: `crates/ruviform-plugin-vst3/src/host.rs` `mod tests`
（`plug_frame_resize_*`）。

### Rule TS-873-7: CLAP `clap_host_gui` — enqueue → drain 写像と bounded queue

**Example**: `BridgeShared` の `HostGuiImpl`:

- `request_resize`/`request_show`/`request_hide` → `Ok(())`（enqueue
  ack）＋ `GuiRequest` が順序通り queue に積まれる。
- `resize_hints_changed`/`closed(was_destroyed)` → 通知として enqueue。
- `drain_gui_request_queue` で全件取り出せ、drain 後の再 pull は空。
- queue が `MAX_GUI_REQUESTS` に達すると `request_*` は `Err`、通知は
  defined drop — 無限増大しない。

実装: `crates/ruviform-plugin-clap/src/host.rs` `mod tests`
（`host_gui_verbs_enqueue_then_drain`/`host_gui_request_queue_bound_refuses`）。

## 検証コマンド対応

| Rule | コマンド |
| --- | --- |
| TS-873-1..5 | `mise run test ruviform-plugin-host` |
| TS-873-6 | `mise run test-l1-real-plugin`（`--features host --lib` 区間） |
| TS-873-7 | 同上（CLAP `--lib` 区間） |
| 全体の非回帰 | `mise run fmt-check` / `check` / `lint` / `test`（対象 crate）・`test-ui-plugin-dock` 系 lane |
