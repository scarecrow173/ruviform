---
id: "DESIGN-0351ujlVkI9Du0Jm1todAT"
type: "design"
status: "draft"
lifecycle: "draft"
title: "Issue #872 L1 out-of-process plugin editor host 設計（wire GUI 動詞・crash/hang 分離 lane）"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/872"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/115"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/issue-872-l1-oos-plugin-editor-host.md"
      - type: "doc"
        path: "docs/discovery/issue-872-l1-oos-plugin-editor-host.md"
    modified:
      - type: "doc"
        path: "docs/designs/issue-389-plugin-editor-session.md"
      - type: "doc"
        path: "docs/specs/plugin-l1-out-of-process.md"
      - type: "doc"
        path: "docs/specs/issue-389-ime-verbs.md"
      - type: "doc"
        path: "docs/specs/issue-389-real-session-lifecycle.md"
      - type: "doc"
        path: "docs/specs/issue-119-l1-wire-version-policy.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  realizes:
    - "docs/specs/plugin-l1-out-of-process.md"
  supersedes: []
---

# Issue #872 L1 out-of-process plugin editor host 設計（wire GUI 動詞・crash/hang 分離 lane）

## Context

Issue #872 — #389 の `rf631.plugin-dock-real` lane で
`real-session-crash`/`real-session-hang` が verdict `Unsupported`
のまま残っている（`crates/ruviform-plugin-dock-test/src/real_dock.rs`:13235-13288、
evidence marker `session:{crash,hang}:in-process-lane-nonviable`）。
fixture `test_fault` の in-band fault（`abort`/`exit:N`/`hang`）は
plugin の `process()` 内で発火し、in-process lane では plugin
cdylib が host process を共有するため lane 自身が殺される/固まる
— 観測を継続できない。解除には out-of-process（L1）の plugin
editor host、すなわち **wire GUI 動詞 + プロセス隔離**が必要。

本書はその L1 editor host の確定設計である。設計の核心問いは、
Issue #115（CLOSED — approved design
`docs/designs/issue-115-l1-gui-cross-process-parenting.md`）が
確定した「**プロセス境界越しの親ウィンドウ埋め込みは Windows
では条件付き脆弱（入力キュー暗黙結合により子 hang が host UI
へ波及）・macOS/Wayland では越境不可**」という判定と、「子の
crash/hang を観測して lane が生存し続ける」という要求の両立で
ある。結論として本設計は **docked 越境 attach を取らず、child
プロセス所有の toplevel window（`AttachMode::Floating`）を L1
editor のトポロジーとする**。

確定の入力となる現行実装の事実:

- `WIRE_VERSION = 5`（`crates/ruviform-plugin-core/src/wire.rs`:119）。
  v5 は `OpenGui.parent` を raw `u64` から generation-bound
  `ParentLease` へ変えた payload 変更の先例である。
  `Message::Hello{version:u16, capabilities:u32}` は child→parent
  単方向（wire.rs:22 の frame table）で、`capabilities` は
  reserved — `capabilities != 0` は decode が
  `WireError::Malformed` として拒否する（wire.rs Hello decode）。
- wire 語彙は `Command::OpenGui{parent:ParentLease}`（tag 8）・
  `Command::CloseGui`（tag 9）・`Reply::Opened{gui:GuiHandle}`を
  既載 — #115 が keep-for-future と pin した語彙。size /
  visibility / focus / key の GUI 動詞と capability 応答は wire
  に存在しない。
- host 側 `L1Instance::execute` は `OpenGui`/`CloseGui` を wire
  に出さず `Err(HostError::Unsupported)` で short-circuit
  （`crates/ruviform-plugin-host/src/l1/mod.rs`:2995-3035）。
  session adapter の `L1EditorOps` は全 op を
  `EditorOpError::Unsupported` で拒否する refusal-only adapter
  （`crates/ruviform-plugin-host/src/editor_session.rs`:1840-1919）。
- editor session 層は実装済み: `EditorSessionOps` の verb 面
  （`open`/`close`/`set_size`/`set_visibility`/`reattach`/
  `set_focus`/`send_key`/`capabilities` — editor_session.rs:507-593）、
  `AttachMode::{Docked,Floating}`（同 51-56）、10 flag の
  `EditorCapabilities`（同 109-130）、`Terminated{cause}` の
  exactly-once 収束（同 271-274, 1010, 1587）、
  `SessionDeadlinePolicy`/`tick`/`tick_all` の driver 駆動
  deadline モデル（同 444-449, 1070, 1602）。
- L1 の死亡検出語彙は実装済み: `try_wait` crash watch・stdout
  EOF・`command_timeout` → `Err(HostError::Ipc)` → child-scope
  収束（収束 thread 自身が wedged child を `Shared.child` 経由で
  kill）→ `Failed` 遷移・mint した全インスタンスへ exactly-one
  `HostNotification::InstanceFailed{handle, exit, cause}`
  （`crates/ruviform-plugin-host/src/l1/mod.rs` thread model、
  `crates/ruviform-plugin-host/src/composite.rs`:229-243）。
  auto-restart なし。
- 子プロセス側 `ruviform-plugin-export::run` は stdin/stdout
  frame の単一スレッドメッセージループで UI thread/event pump
  を持たないが、`WireCommand::OpenGui`/`CloseGui` の dispatch 腕は
  存在する（`crates/ruviform-plugin-export/src/lib.rs`:593-600 —
  到達すればコマンドループスレッド上で `plugin.open_gui()` を
  呼ぶ）。`Hello` 送出は `capabilities: 0` 固定の一箇所
  （同 230-237）。
- fault 語彙は 2 経路: runner argv の spawn fault
  （`--ruviform-test-fault=abort|exit:N|hang|stderr:N|bad-fixture`
  — `crates/ruviform-plugin-test-vst3/src/bin/
  ruviform-plugin-test-vst3-l1-runner.rs`:30-40。Hello 到達前に
  発火する spawn/handshake 系）と、plugin in-band fault
  （`test_fault` の `Fault` parameter — `abort`/`exit`/`hang` 等が
  `process()` 内で発火。l1_fault E2E が crash/hang 観測を実証済み:
  `real_test_fault_l1_abort_fails_instance`・
  `real_test_fault_l1_hang_converges_on_timeout`・
  `real_test_fault_l1_bundled_crash_fans_out`・
  `real_test_fault_l1_dedicated_isolation` —
  `crates/ruviform-plugin-test-vst3/tests/l1_fault_test_plugin_e2e.rs`）。
- isolation 語彙: `OutOfProcessIsolation::{Dedicated,Bundled}`、
  `PluginHost::kill_child`（child-scope kill の検証支援 API）、
  `register_out_of_process`/`instantiate`/`process` 経路
  （composite.rs）。**Bundled pool は route（plugin id）ごと** —
  異なる descriptor の co-residency は現行構成では不可能
  （`real_test_fault_l1_bundled_crash_fans_out` が pin するのは
  同一 plugin ×2 の fan-out）。
- 現在の crash/hang catalog 行は `required: &[ScenarioCapability::
  NativePluginDock]` のみで、case 本体が明示的に
  `ScenarioVerdict::Unsupported` を記録する（
  `crates/ruviform-plugin-dock-test/src/real_catalog.rs`:259-267、
  real_dock.rs:13235-13288）。`ScenarioCapability`/`ScenarioVerdict`
  は `crates/ruviform-ui-test/src/scenario.rs`:204,253 に定義。

## Scope

### In Scope

- wire GUI 動詞集合・tag 割当・payload/reply・error 語彙の設計
  （wire v6）
- `WIRE_VERSION` 5→6 bump と `Hello.capabilities` GUI bit の確定形
- editor GUI を持つプロセス構成（runner UI thread/pump・window
  所有・docked 越境 attach の採否・Dedicated/Bundled 選択）の設計
- crash/hang 分離 lane の観測 oracle・fault 起動経路・
  verdict/evidence 語彙の設計
- `real-session-crash`/`real-session-hang` の `Unsupported` 解除に
  必要な実装 slice の分割と Blocked-by 関係（起票案自体は PR
  本文 — 本書は分割と依存を確定する）

### Out of Scope

- 実装コード一切 — 本 Issue は設計 Issue であり `.rs`/
  `mise.toml`/CI 定義/evidence は read-only 参照のみ
- L1 docked（プロセス越境埋め込み）の実現 — 本設計はこれを
  Defined `Unsupported` として固定する。docked 設計自体は
  将来の別 Issue（Deferred Design Concerns 参照）
- plugin→host 方向の GUI 要求動詞（#873 系列で別 Worker が実装中
  — wire 語彙の衝突を避けるため本設計は host→child 方向のみを
  確定する）
- video runner（`run_video`）側の GUI 面 — audio runner のみ対象
- GPUI/consumer 側での floating editor window の合成・配置

## 決定サマリ

| 裁定 | 結論 |
| --- | --- |
| editor GUI を持つプロセス構成 | **L1 child 内に lazy 起動の UI thread + message pump を持つ**（`ruviform-plugin-export::run` runner 拡張。editor-host 専用プロセスは新設しない） |
| docked 越境 attach vs 子所有 window | **child 所有 toplevel（`AttachMode::Floating`）のみ**。docked は L1 では Defined `Unsupported`（#115 の入力キュー結合の確定と lane 生存が構造的に両立しない） |
| wire 動詞集合 | tag 8/9 を payload 拡張して流用 + tag 24-27 新設（`SetGuiSize`/`SetGuiVisibility`/`SetGuiFocus`/`SendGuiKey`）。`Reply::Opened` に `capabilities` を追加。`reattach`/`capabilities` 専用動詞は設けない |
| `WIRE_VERSION` | **5→6** — #119 の単一最新版方針の非互換 bump（payload 拡張・新 tag・`capabilities` 有効化を同時に入れる）。v5 の `capabilities != 0` → `Malformed` 規則は v5 とともに凍結 |
| capability 交渉 | `Hello.capabilities` bit0 `GUI_FLOATING`（child→parent 広告のみ。D6 草案の確定版 — per-verb bit でなく機構族 bit）。未広告 child への GUI op は全て `Err(Unsupported)` で現行拒否の意味を維持 |
| crash oracle | `Dedicated` child で `kill_child` または `test_editor` in-band `Fault` abort/exit → `InstanceFailed` exactly-once → `sessions.notify_terminated(HostLost)` → `Terminated{HostLost}` once → `Closed` + lane 生存 |
| hang oracle | `Dedicated` child で `test_editor` in-band `Fault` hang → 非 session probe command → `command_timeout` → `Err(Ipc)` → `Failed{Timeout}` + watchdog kill → `InstanceFailed` → `Terminated{HostLost}` → `Closed` + lane 生存 |
| lane capability | `ScenarioCapability::OutOfProcessPluginHost` 新設（Windows interactive + L1 runner/fixture 解決可能なときのみ claim — fail-closed）。crash/hang 行は `required` に追加 |
| OS 範囲 | **Windows 先行**（`GUI_FLOATING` 広告は `#[cfg(windows)]`）。macOS/Linux は adapter `open_gui` 未実装ゆえの Defined `Unsupported` — 新しい blocker 語彙は不要 |
| 実装分割 | 5 slice — (1) wire v6、(2) runner UI thread/pump、(3) `L1EditorOps` + host routing、(4) `test_editor` in-band `Fault` param、(5) lane verdict flip。Blocked-by は D-followup に確定 |

## 決定

### D-topology: editor GUI を持つプロセス構成とトポロジー

**裁定: editor window は L1 child が所有する toplevel とし、GUI
動詞を捌く UI thread + message pump を `ruviform-plugin-export::run`
内に lazy 起動する。editor-host 専用プロセスは新設しない。
L1 の docked（越境 `SetParent`/`WS_CHILD` 系）attach は Defined
`Unsupported` とする。crash/hang lane は `OutOfProcessIsolation::
Dedicated` の child を使う。**

根拠:

1. **失敗ドメインの同一性** — crash/hang lane が観測したいのは
   「editor を持つ instance が死んだ/固まったとき session と
   lane がどう収束するか」である。editor host を別プロセスに
   分けると失敗ドメインが 2 つになり（editor host 死亡・DSP
   child 生存の組合せが生じる）、`Terminated` の cause 解釈が
   曖昧になる。editor を instance と同じ child に置けば
   **child の死亡/固着 = editor 喪失**で、`Failed`/
   `InstanceFailed` 収束へ 1:1 に写像できる。既存の L1 収束
   機構（l1/mod.rs の driver/reader/stderr-drain 3 thread
   model）を無改変で再利用できる。
2. **lane 生存との構造的両立（核心）** — #115 が確定した
   Windows `SetParent`/`WS_CHILD` 越境埋め込みは AttachThreadInput
   系の入力キュー暗黙結合を伴い、child の hang が host UI
   thread の queue へ波及する。crash/hang lane の目的は「child
   の死亡/固着を host 側が観測して生き残る」ことであり、入力
   キューが結合した越境 docked attach を採れば検証対象の障害が
   観測者自身を殺す — 目的と構造的に矛盾する。child 所有
   toplevel は child 自身の thread queue で動き、child が固まって
   も host の queue は無関係に回り続ける。**これが本設計の軸
   となる裁定**であり、#115 の「越境埋め込みは条件付き脆弱」
   判定を、採用可/不可の二者ではなく「docked = Defined
   `Unsupported`・floating = 採用」の形で解消する。
3. **#115 解除条件の充足** — #115 の解除条件 (a)（wire 版
   枠組み）は #119 で充足済み、(b)（child 側 UI thread/pump
   設計）は本設計の lazy UI thread で充足、(c)（OS 機構の
   再評価）は本 D の裁定自体、(d)（需要の顕在化）は #389 の
   crash/hang AC が該当する。
4. **runner 拡張を選ぶ理由** — `ruviform-plugin-export::run` は
   test runner（`ruviform-plugin-test-vst3-l1-runner`）と将来の
   production adapter runner が共有する単一エントリである。
   UI thread をここに入れれば全 runner が同じ機構を得て、
   GUI 対応 binary を分ける必要がない。**lazy 起動**（最初の
   GUI 動詞到達時にだけ spawn）にすれば、GUI を使わない
   child のコストはゼロのまま — audio-only child の挙動・
   リソースは現行と同一である。
5. **Dedicated を選ぶ理由** — Bundled pool は route（plugin
   id）ごとに独立しており、editor fixture と fault fixture を
   同一 child に同居させるには両者が同一 plugin id である必要
   がある。`test_fault`（fault を持つが editor を持たない）と
   `test_editor`（editor を持つが fault を持たない）は別
   descriptor のため Bundled で同居させられない。そこで lane
   は Dedicated child 1 本で `test_editor` を走らせ、crash には
   `kill_child`（実装済みの検証支援 API）または `test_editor`
   自身への in-band `Fault` param 拡張（D-followup slice 4）を
   使い、hang には同 in-band `Fault` arm を使う（D-oracle）。
   route 横断 pool（異種 plugin の Bundled 同居）は本設計の
   対象外とする。

`AttachMode` との整合:

- `L1EditorOps::open` は `AttachMode::Docked` を **ops 層で早期
  `Err(EditorOpError::Unsupported)`** で拒否する —
  `InProcessEditorOps::open` が non-Docked を早期拒否するのと
  対称の形（editor_session.rs:1690-1703。「要求と異なる mode の
  attach が成功に見える fake success」を防ぐ defined refusal）。
- `open_or_degrade` 経路はそのまま意味を持つ: `open(Docked)` →
  `Err(Unsupported)` → session は `Floating` への explicit
  degrade を試せ、`Degraded{Floating}` event を emit する。
  `Degraded` は Native Dock PASS に数えない既定 honest
  vocabulary（`OpenOutcome::Degraded` — editor_session.rs:465-474）
  であり、consumer が「docked を望むが floating でもよい」を
  表現する正規経路として残る。
- `reattach` は docked-family の再 parent 化であり v6 では ops
  層で `Err(Unsupported)`（wire 動詞自体を設けない —
  D-verbs）。

owner hint（`OpenGui.parent` の意味付け）:

- Floating open の `parent` field（`ParentLease`）は **owner
  hint** として運ぶ。`L1EditorOps` は session から受けた
  `ParentSlotLease` を `LeaseRedeemer` で redeem して得た
  `ParentLease`（payload = host 側 window の HWND）をそのまま
  wire に載せる — `InProcessEditorOps` と同じ redeem seam
  （`LeaseRedeemer` — editor_session.rs:1641）を使うため、
  session 側の lease 契約は変わらない。
- child は `payload != 0` を owner hint として解釈し、
  toplevel に `SetOwner`（`GWLP_OWNER`）相当の association を
  適用する — `SetParent`/`WS_CHILD` では**ない**。owner
  association は Z-order/最小化連動だけを与え、child window は
  child の thread queue に残る（入力キュー結合を伴わない —
  #115 が問題にしたのは child-window 化に伴う AttachThreadInput
  系であり、owned toplevel は対象外。実装 slice での再検証
  項目は Verification Notes を参照）。`payload == 0` は
  owner なし standalone toplevel を意味する。
- これにより lease の opaque 境界は維持される: session/consumer
  面に raw HWND は出ず、`ParentSlotLease` → redeem →
  `ParentLease` の既存の流れの中で owner hint だけが wire を
  越える。

### D-verbs: wire GUI 動詞集合

**裁定: `EditorSessionOps` の verb 面（editor_session.rs D4
相当 — `open`/`close`/`set_size`/`set_visibility`/`reattach`/
`set_focus`/`send_key`/`capabilities`）を、下記の写像で
`wire::Command` 集合へ載せる。keep-for-future の tag 8/9 は
payload 拡張して再利用し、4 動詞を tag 24-27 で新設、
`Reply::Opened`（reply tag 3）を `capabilities` field 追加で
拡張する。`reattach` と `capabilities` には専用 wire 動詞を
設けない。**

写像表:

| session op（`EditorSessionOps`） | `wire::Command` | tag | payload | reply |
| --- | --- | --- | --- | --- |
| `open` | `OpenGui` | 8（拡張） | `mode:u8` + `parent:ParentLease` + `size:GuiSize` | `Opened{gui, capabilities}` |
| `close` | `CloseGui` | 9 | なし | `Ok` |
| `set_size` | `SetGuiSize` | 24（新設） | `size:GuiSize` | `Ok` |
| `set_visibility` | `SetGuiVisibility` | 25（新設） | `visible:u8` | `Ok` |
| `reattach` | 動詞なし | — | — | — |
| `set_focus` | `SetGuiFocus` | 26（新設） | `focused:u8` | `Ok` |
| `send_key` | `SendGuiKey` | 27（新設） | `event:GuiKeyEvent` | `Ok` |
| `capabilities` | 動詞なし | — | — | — |

確定事項:

- **`OpenGui`（tag 8、payload 拡張）**: `{mode:u8,
  parent:ParentLease, size:GuiSize}` — v5 の `{parent}` を拡張
  する（v5 が `parent` の型を変えたのと同じ「bump 時の payload
  変更」先例 — 単一最新版方針下では v5 の decode は残らない）。
  - `mode`: `0 = Floating`（v6 で唯一実装する値）、
    `1 = Docked`（**定義済み・未実装** — decode は受理し、
    child は `Err(HostError::Unsupported)` の defined reply を
    返す。未知の他値は decode `Malformed`）。値の語彙だけを
    先に確定することで、将来の docked 設計が payload 変更なしで
    着地できる。
  - `parent`: floating では owner hint（D-topology）。
  - `size`: 初期 editor size（`GuiSize{width,height:f64}` —
    boundary.rs:155。wire の f64 codec 既存）。`{0,0}` なら
    child は plugin の preferred size で開く。
- **`Reply::Opened`（reply tag 3、payload 拡張）**: `{gui:GuiHandle,
  capabilities:u32}`。
  - `gui`: child が mint する surface token — ≤1 surface /
    instance（#115/#123 の設計規則は維持）。
  - `capabilities`: child が返す GUI capability bitfield —
    **bit 配置は `EditorCapabilities` の field 宣言順**
    （editor_session.rs:109-130）: dock=bit0, float=bit1,
    resize=bit2, hide_show=bit3, reparent=bit4, ime=bit5,
    popup=bit6, close=bit7, input=bit8, focus=bit9。v6 で child
    が立てられるのは `float`/`close`（runner 機構として常時）
    と `resize`/`hide_show`/`input`/`focus`（plugin の reply
    次第 — VST3 `canResize` 等の adapter 判定をそのまま写す）。
    `dock`/`reparent`/`ime`/`popup` は v6 では 0 固定
    （docked-family・IME・popup は対象外）。
- **`CloseGui`（tag 9、payload 不変）**: live surface を持たない
  instance に対しては `Err(HostError::InvalidValue)` の
  defined rejection（値域外 — close 対象の surface が存在
  しない）。
- **≤1-surface の担保**: session 層（state machine）と plugin
  adapter 層（VST3 の `view.is_some()` → `PluginFailed` 先例）
  が既に持つ規則であり、runner は 2 回目の `OpenGui` を
  plugin hook へそのまま渡して decline を返す — runner 独自の
  gate は追加しない（防御の重複を避ける）。runner が持つのは
  reply を instance へ対応付けるための `Option<GuiHandle>` の
  簿記のみ。
- **`SetGuiSize`（tag 24）**: child の UI thread で
  `plugin.set_gui_size` を呼び、container を plugin view に
  合わせる。resize を断る plugin は `PluginFailed`/`Unsupported`
  の defined rejection（L0 と同じ語彙 — session 側では
  `EditorOpError::Plugin`/`Unsupported` に写像）。
- **`SetGuiVisibility`（tag 25）**: child 所有 container の
  show/hide（Windows `SW_SHOW`/`SW_HIDE` 相当）。plugin view
  自身は存続する — L0 VST3 で hide が view detach 相当になる
  のとは意味論が異なるが、session 層の `Hidden`/`Shown`
  語彙（editor 生存・非表示）には container の show/hide が
  正しい実装である。
- **`SetGuiFocus`（tag 26）・`SendGuiKey`（tag 27）**: L0 で
  既載の `set_gui_focus`/`send_gui_key` hook 面を child の UI
  thread で呼ぶ。`GuiKeyEvent` の wire codec は
  `direction:u8`（`GuiKeyDirection::{Down,Up}` — 未知値は
  `Malformed`）+ `key:u32`（BMP 範囲を検証する char —
  boundary.rs:244 の contract どおり supplementary-plane は
  adapter 側 `InvalidValue`）+ `key_code:i16` +
  `modifiers:u16`（`GuiModifiers.bits`）とする。
- **`reattach`/`SetGuiParent`**: wire 動詞を設けない。docked
  前提の再 parent 化であり v6 の floating 語彙には含めない。
  `L1EditorOps::reattach` は ops 層 `Err(Unsupported)`。
  `HostCommand::SetGuiParent` は L0-only のまま — composite の
  `l0_only_gui_verb` guard が維持する唯一の GUI verb になる。
- **`capabilities`**: wire 動詞を設けない。`Hello.capabilities`
  の negotiation bit（protocol/機構級の可否 — D-version）と
  `Reply::Opened.capabilities`（open 時の plugin 個別報告）で
  充足する。`L1EditorOps::capabilities()` は bit 未広告なら
  `Err(Unsupported)`、広告済みなら open 前は `{float:Supported,
  close:Supported}` を、open 後は最後に受け取った
  `Opened.capabilities` を反映した `EditorCapabilities` を返す
  （pre-open に plugin 個別 caps が必要になった場合のみ、将来
  bump で専用動詞を足す — 現時点では YAGNI。L0 も pre-open の
  caps query を持たず「open が発見経路」の先例どおり）。
- **verb の instance scope**: 全 GUI 動詞は envelope の
  `instance:u32` で instance を指し、payload に `GuiHandle` を
  載せない — ≤1 surface 規則の下で instance が surface を一意
  に決める。`GuiHandle` は `Opened` reply の証明語彙のみ。
- **error 語彙**: child→parent の `Err(HostError)` —
  `Unsupported`（未定義値を含む defined 拒否: docked mode・
  未実装 plugin hook・adapter が capability を返さない op）、
  `InvalidValue`（`CloseGui`-with-no-surface・`payload` 域外）、
  `PluginFailed`（plugin hook decline — `guarded`/
  `catch_unwind` 経由で panic もここに収束、export lib.rs:563）、
  `Ipc`（transport/protocol）。decode 不可は wire-level
  `Malformed`。host 側の `EditorOpError` 写像は
  `map_host_error`（editor_session.rs:1665-1678）を再利用する
  — `Failed`→`HostLost`、`Ipc`→`Io`（session では
  `Failed{Transport}` — `fail_cause` editor_session.rs:483-495）、
  `Unsupported`→`Unsupported`。

### D-version: `WIRE_VERSION` bump 方針

**裁定: `WIRE_VERSION` を 5→6 とし、GUI 動詞の追加・payload
拡張・`Hello.capabilities` GUI bit の有効化を同一 bump で入れる。
`Hello.capabilities` の mask は `0x1`（`GUI_FLOATING`）のみ —
D6 草案の確定形として機構族 bit とする。version 一致は厳格
等値のまま（negotiation protocol は導入しない）。**

確定事項:

- **bump が守る変更一式**（全て v6 で同時に入る非互換変更）:
  1. `OpenGui` payload `{parent}` → `{mode,parent,size}`（tag 8）
  2. `Reply::Opened` payload `{gui}` → `{gui,capabilities}`
     （reply tag 3）
  3. `Command` 新 tag 24-27（`SetGuiSize`/`SetGuiVisibility`/
     `SetGuiFocus`/`SendGuiKey`）
  4. `Hello.capabilities` の `0x1` bit を defined にする
     （`capabilities & !0x1 != 0` → `Malformed` を維持 —
     **未知 bit は引き続き拒否**、0 広告も合法）
- **#119 方針との整合**: `docs/specs/issue-119-l1-wire-version-
  policy.md` の単一最新版方針どおり、host/runner は同一
  `WIRE_VERSION` で workspace 一体更新し、v5 の decode/encode
  は残さない。「additive payload/語彙変更は bump を要する」
  「`capabilities` の解放は bump と同時」の 2 規則に、本設計は
  そのまま適用される適用例である（`capabilities != 0` →
  `Malformed` の v5 規則は v5 実装とともに凍結・撤去される
  が、規則の意図 — 未定義値の拒否 — は `!0x1` mask で v6 に
  引き継ぐ）。
- **`Hello.capabilities` の semantics**:
  - 方向: **child→parent のみ**（frame table 上 Hello は child
    が spawn 直後に送る唯一の第一 frame — wire.rs:22、送出は
    `ruviform-plugin-export::run` 冒頭の一箇所、lib.rs:230-237）。
    parent→child の広告経路は存在せず、v6 でも設けない
    （parent 側に伝えたい要件が出た場合は次の bump で
    `Hello` 系を拡張する）。
  - `GUI_FLOATING`（`0x1`）: 「この runner は GUI floating 動詞
    族（`OpenGui`/`CloseGui`/`SetGuiSize`/`SetGuiVisibility`/
    `SetGuiFocus`/`SendGuiKey`）を実装している」を広告する
    機構族 bit。**per-verb bit にはしない** — 動詞族を atomic
    に 1 bit で表し、verb 個別の可否は reply 側の
    `Err(Unsupported)`/`Opened.capabilities` で表現する。
    理由: 機構（UI thread + dispatch）の有無は verb 群共通の
    1 事実であり、bit を verb ごとに割ると「bit はあるが reply
    が Unsupported」の組合せが増えて語彙が不正直になる。
    D6 草案の `GUI_OPEN`/`GUI_LIFECYCLE`/`GUI_REPARENT`
    per-verb 構成はこの機構族 bit 設計に supersede される
    （D6 は非拘束 sketch と明記されていた — 389 design doc の
    D6 注記参照）。
  - 広告条件: runner 側 `capabilities` は **`#[cfg(windows)]`
    でのみ `0x1` を立てる**（v6 の UI thread 実装は Windows
    のみ — D-os）。非 Windows runner は `capabilities: 0` の
    ままで、これは v6 でも合法な広告（= GUI 非対応の honest
    表明）。
  - 交渉 semantics: parent は child の広告 bit を **instance の
    negotiated mask** として保持する（child scope — Bundled
    では同居全 instance が同一 bit を共有）。広告なし
    （mask に bit なし）の child への GUI 動詞は、host 側が
    wire に出す前に `Err(HostError::Unsupported)` で拒否する
    — 現行 `OpenGui`/`CloseGui` short-circuit（l1/mod.rs:3033-3035）
    を「GUI 動詞全般 + negotiated bit gate」に一般化した形で
    **現行拒否の意味をそのまま維持する**。bit 広告済みの
    child では動詞が wire に出て child が dispatch する。
  - 未知 bit（`& !0x1`）は v6 decode が `Malformed` で拒否
    — 前方互換の交渉余地は残さず、「知っている bit 以外を
    名乗る peer は malformed」という #119 の strictness を
    維持する。
- **negotiation protocol を導入しない理由**: 要求されている
  「capabilities 交渉」は「child が機構の有無を広告し、parent
  がそれを honor する」一方向 advertisement で充足する。
  request/response 型の交渉（parent が要求集合を送り child が
  受理集合を返す）は、parent 側に「要求を送る語彙」と
  「部分的受理の verdict」を新設する必要があり、v6 の需要
  （GUI 機構の有無 1 bit）に対して過剰 — YAGNI。version 厳格
  等値のまま 1 bit 広告で済ませる。

### D-oracle: crash/hang の観測 oracle

**裁定: crash は `Dedicated` child への `kill_child`（主経路 —
実装済み）または `test_editor` の in-band `Fault` abort/exit
（代表 fault — slice 4 着地後）を起動し、`InstanceFailed`
exactly-once → `sessions.notify_terminated(handle,
SessionFailCause::HostLost)` → `Terminated{HostLost}` once →
`Closed` を観測する。hang は `test_editor` in-band `Fault` hang
を起動し、非 session probe command の `command_timeout` →
`Err(Ipc)` → child-scope `Failed{Timeout}` 収束 + watchdog
kill → `InstanceFailed` → 同一 `Terminated{HostLost}` 収束を
観測する。観測語彙は既存 L1 収束機構のものだけを使い、新しい
oracle 機構は設けない。**

crash 観測経路（確定）:

1. `Dedicated` child で `test_editor` instance を mint し、
   session `open(Floating)` で `Attached{mode:Floating}` にする
   （+ child pid 帰属の window 存在を OS oracle で確認）。
2. 起動: `PluginHost::kill_child(handle)`（child-scope kill —
   composite.rs の検証支援 API。`Dedicated` なので影響は対象
   instance のみ）**または** `test_editor` の in-band `Fault`
   arm（abort/exit — slice 4 で `FAULT_PARAM`/`decode_fault`
   機構を `test_editor` にも載せる。`process` を駆動して arm
   した後に発火させる — こちらは「plugin コード自身が死ぬ」
   代表 fault として richer な trigger）。
3. 観測（全て既存語彙）: child 死亡 → driver の `try_wait`
   crash watch または stdout EOF → child-scope 収束 →
   instance `Failed` + mint した instance へ exactly-one
   `HostNotification::InstanceFailed{handle,
   exit{code,signal}, cause}` → lane は
   `PluginHost::try_recv_notification` で拾い、
   `sessions.notify_terminated(handle,
   SessionFailCause::HostLost)` を呼ぶ → session は
   `Terminated{cause:HostLost}` を 1 度だけ emit → `Closed`
   （`Terminated` exactly-once — editor_session.rs:271-274,1010,
   1587）。2 回目の notify は無 event。
4. lane 生存: 収束後の lane 固有 window（host 側）への有界
   GUI op が返ること、後続 case が走れること、残った
   notification がないこと（drain 済み）を確認して `Pass`。

hang 観測経路（確定）:

1. attach までは crash と同じ（Dedicated + `test_editor` +
   `open(Floating)` + window oracle）。
2. 起動: `test_editor` の in-band `Fault` `hang`（slice 4 —
   `Initialize`→`Activate`→`StartProcessing`→submit で
   `process` を駆動してから arm → `process()` 内で wedge）。
   child の command loop thread が plugin call 内で wedged に
   なり、wire command/reply と `BlockIn` 処理の双方が止まる
   （child process 自体は生存 — `try_wait` は死を返さない）。
   この wedge の位置（`process` 内）は DSP hang と同型であり、
   editor session 用の別途 wedge 機構は要らない。
3. 観測: lane は **session 経由ではない probe command**（例:
   `host.execute(HostCommand::SaveState{handle})` を fault
   instance に — session event 列を deterministic に保つため
   session op ではなく raw command を probe とする）を出し、
   `command_timeout`（lane 側 `OutOfProcessSpec` は短い timeout
   を指定 — l1_fault E2E の 500ms 先例）の満了で
   `Err(HostError::Ipc)` → child-scope 違反として収束 →
   converging thread が `Shared.child` 経由で wedged child を
   kill → `Failed{cause:FailureCause::Timeout}` +
   `InstanceFailed` once → `Terminated{HostLost}` → `Closed`。
   wedge 中の audio leg が `Ok(CONTINUE)` の passthrough
   substitution を返し続けることは
   `audio_counters.passthrough_submits` の delta で補助観測
   できる（l1_fault の hang 観測と同一語彙）。
4. `Terminated{HostLost}` の cause について: `InstanceFailed`
   の `cause` には `FailureCause::Timeout` が入るが、session
   側の cause は `HostLost` で統一する（session 語彙では
   「host/instance/process が gone」が真実 —
   editor_session.rs:148-149。`Timeout` の詳細は notification
   の `cause`/`exit` field と evidence marker が保持する）。
5. lane 生存は crash と同じ基準で確認。

session 層との整合（確定）:

- session op が wedge の**途中で**飛んだ場合（probe と別に
  session op を出した場合）: ops の呼び出しは wire
  `command_timeout` で `Err(Ipc)` を返し、`fail_cause` 写像で
  session は `Failed{Transport}` event を先に emit し得る —
  その後 notification 経由で `Terminated{HostLost}` に収束する
  （`Failed` から `Terminated` への進行は設計済みの遷移）。
  ただし **lane の主 oracle はこの順序に依存しない** — probe
  を raw command に限定し、session event は
  `[Terminated{HostLost}]` だけを要求する（deterministic oracle
  として pin する）。
- session 側の `SessionDeadlinePolicy`/`tick`/`tick_all`
  （editor_session.rs:444-449,1070,1602）は in-flight session
  request 自身の deadline であり、wire `command_timeout`
  （`execute` の reply 待ち上限）とは独立に存在する — 両者を
  混同しない。lane の hang oracle は wire timeout 経路を使い、
  session deadline は `real-session-timeout`（既存行）側の
  領域のままとする。
- `InstanceFailed` → `notify_terminated` の **delivery
  contract**（確定 — 新機構ではなく利用契約）: notification
  queue（`try_recv_notification`）を読んで session へ
  `notify_terminated(instance, HostLost)` を渡すのは lane /
  GUI driver 側の pump 責務。`L1EditorOps` 自体は
  notification を subscribe せず、session event は
  notification 消化の結果として生じる — production の GUI
  driver は event loop でこの pump を回す（実装 slice 3 で
  `L1EditorOps` または driver 側に helper を置くかを決める）。

fault 経路の使い分け（確定）:

| 経路 | 起動対象 | lane での役割 |
| --- | --- | --- |
| `test_editor` in-band `Fault`（slice 4 で新設） | `process()` 内で abort/exit/hang | **crash の代表 fault・hang の唯一の主経路** — 「plugin コードが死ぬ/固まる」実在 fault |
| `PluginHost::kill_child` | child-scope kill（任意タイミング） | **crash の主経路** — DSP 駆動不要で最小構成（fixture 拡張に依存しない） |
| argv `--ruviform-test-fault` | spawn 直後・Hello 前 | **lane では使わない** — session が存在しない pre-Hello 領域の fault であり、session crash/hang の観測対象ではない（spawn/handshake 系の別 lane 候補として Deferred に記録） |

**#768（l1_fault 系）との整合**: 上記の観測語彙は全て
l1_fault E2E が既に pin している semantics（`Failed` 収束、
`InstanceFailed{cause:Timeout}`、passthrough substitution、
exactly-once notification、Dedicated の sibling 無汚染）と同一
であり、lane は新しい stall/drop 意味論を導入しない。

### D-lane: lane 生存条件と catalog/capability/verdict 語彙

**裁定: `ScenarioCapability::OutOfProcessPluginHost` を新設し、
`real-session-crash`/`real-session-hang` の `required` に
`NativePluginDock` と併記で加える。verdict 語彙（`Pass`/`Fail`/
`NotRun`/`Unsupported`/`Blocked`）と #631 の scenario/case 形式は
そのまま継承し、`Unsupported` は「環境が用意できても adapter/
lane が原理上提供しない」場合にだけ残す。**

新 capability（確定）:

- `ScenarioCapability::OutOfProcessPluginHost`
  （kebab: `out-of-process-plugin-host`）—
  `crates/ruviform-ui-test/src/scenario.rs` の enum に追加する
  （`#[non_exhaustive]`・kebab serde — 既存 variant と同形）。
- **claim 条件（fail-closed — `os_ime::supported()` と同じ
  claim 方式）**: 次の全てを満たすときのみ claim —
  (a) `cfg!(windows)`、(b) interactive real-window session
  （lane 全体の environment gate と同一の判定）、(c) L1
  runner binary（`ruviform-plugin-test-vst3-l1-runner`）と
  fixture cdylib が解決可能、(d) route spawn で `Hello.
  capabilities` の `GUI_FLOATING` bit が実際に広告されること
  を **capability probe で確認**する（機構が名乗るだけでなく
  wire が実際に negotiated することを claim の根拠にする —
  「claim は実証された能力のみ」の lane 規約どおり）。probe
  自体が spawn を伴うため、claim 判定は lane 初期化時に 1 度
  だけ行う（case ごとに再 probe しない — 結果を lane 共通
  state に保持する）。
- **未 claim 時の verdict**: `required` 未充足の行は従来どおり
  規定の verdict を取る。`OutOfProcessPluginHost` が (a)/(b)
  由来で落ちる（非 Windows・非 interactive）場合は lane 全体
  と同じ `NotRun` + `environment_blocked` 系の記録になり、
  (c)/(d) 由来で落ちる（Windows interactive なのに runner
  binary 不在・bit 非広告）場合は `Unsupported` の capability-
  gated verdict — 「環境が揃っても adapter が提供しない」の
  意味で使う（`NotRun` は「そもそも走らせられない環境」、
  `Unsupported` は「走らせようとして adapter が拒否」の
  意味で区別 — scenario.rs:253-263 の verdict 定義と整合）。
- **新 capability が必要な理由**: 既存 `NativePluginDock` は
  in-process real-window lane の dock surface 能力を表し、
  「L1 child で editor host が成立し child 死亡/固着を観測
  できる」ことは含意しない — crash/hang 行が要求するのは
  後者で、capability を分けなければ `NativePluginDock` を持つ
  だけの環境で `Unsupported` の理由が「in-process lane の
  構造限界」と「out-of-process host 未対応」で区別できない。
  capability を分けることで、flip 後の `Pass` が out-of-process
  経路で取れた証拠になる。

case oracle（確定 — D-oracle を verdict 条件へ落としたもの）:

- `real-session-crash`: `Dedicated` route + `test_editor` +
  `open(Floating)` → `Attached{Floating}` + child window 存在
  → `kill_child`（または in-band `Fault` arm — slice 4 着地後
  はこちらを代表 trigger とする）→ `InstanceFailed` once →
  `Terminated{HostLost}` once → `Closed` → lane 生存確認 →
  `Pass`。`Failed{cause:HostLost}`/`PluginFailed` 等の別 cause・
  event 重複・lane 側 hang は `Fail`。
- `real-session-hang`: `Dedicated` route + `test_editor` +
  `open(Floating)` → `Attached{Floating}` → in-band `Fault`
  `hang` arm → probe `SaveState` → `Err(Ipc)`（
  `command_timeout`）→ `Failed{Timeout}` + child kill →
  `InstanceFailed{cause:Timeout}` once → `Terminated{HostLost}`
  → `Closed` → lane 生存確認 → `Pass`。timeout せず無限に
  返らない・session が `Closed` に至らない・lane が固まる場合は
  `Fail`。
- evidence marker（確定案 — 実装 slice で文字列を pin する）:
  `session:crash:out-of-process` / `session:hang:out-of-process`
  系 + `l1:instance-failed` / `l1:watchdog-kill` /
  `session:terminated:host-lost` 相当の marker。現行の
  `session:{crash,hang}:in-process-lane-nonviable` marker は
  flip で消え、新 marker が経路の証拠になる。
- `blocked`/`NotRun` との使い分け: 上記の environment/capability
  由来の非実行はそれぞれの verdict で記録し、`Pass`/`Fail`
  には到達しない — 現行 lane の lattice を変えない。

`Unsupported` を残す場所（確定）:

- `AttachMode::Docked` on L1 session — 設計上の defined refusal
  （未実装ではなく「L1 では提供しない」という意味）。
- `reattach`/`SetGuiParent` — v6 語彙外の defined refusal。
- 非 Windows adapter の `open_gui` — plugin 側未実装の
  defined refusal。
- capability 未広告 child への全 GUI op — protocol-level
  refusal（negotiated bit なし = 提供しない、の意味）。
- これらは全て「実装が追いついていない」ではなく「定義として
  提供しない」の `Unsupported` であり、現行 crash/hang 行の
  「lane が構造的に走れない」boundary `Unsupported` とは意味が
  違う — 後者は flip で消える。

### D-os: OS 適用範囲

**裁定: v6 の `GUI_FLOATING` 広告と crash/hang lane は Windows
のみを対象とし、非 Windows では bit 広告を出さない（=
GUI 非対応の honest 表明）。macOS/Linux の editor GUI は
「child 所有 toplevel のトポロジー自体は OS 中立で再利用可能
だが、adapter `open_gui` と runner UI thread 実装が未着地」の
Defined `Unsupported` として、新しい per-OS blocker 語彙は
設けない。**

根拠と境界:

- **Windows 先行の理由**: `POLICIES.md` が要求する
  OS-L1/GUI-L2 の対象であり、`test_editor`（own-process
  `RuviformTestEditor` class HWND — real_catalog.rs:85-88 の
  HWND oracle が示す実在の window fixture）と Win32 window
  機構（`CreateWindowEx`/`GetMessage`/`SetWindowLongPtr` 系）
  が揃っている唯一の OS。child 所有 toplevel には #115 の
  越境埋め込み判定（X11 旧式機構のみ・Wayland/macOS 不可）は
  **適用されない** — あれは「別プロセス window を host の
  window 内に埋める」機構の判定であり、child が自分の
  toplevel を持つ分には embed 機構を必要としない。
- **macOS/Linux の扱い**: トポロジー（child 所有 toplevel +
  runner UI thread）は OS 中立に移植可能だが、(a) 各 adapter
  の `open_gui`/`set_gui_size` 等が未実装、(b) runner の UI
  thread/pump 実装が Windows 先行、(c) lane が Windows
  interactive 前提 — の 3 点で v6 の対象外。これらは「この
  OS では動かない」ではなく「adapter/runner 実装がまだそこに
  届いていない」の defined `Unsupported` であり、bit 非広告
  → ops `Err(Unsupported)` の一貫した語彙で表現できる。
- **#532 DComp との関係**: `real-dcomp-*` 行は host process の
  compositor 内 pixel gate を検証する。child 所有 toplevel は
  host の DComp visual tree に入らないため、DComp 行の oracle
  には影響しない（これらの行は in-process lane のまま残る —
  Deferred Design Concerns に「floating editor の host 側合成
  表示」を記録するが v6 の範囲外）。
- **interactive 実走の要否**: crash/hang lane は real window +
  real child HWND の GUI-L2 lane であり、**interactive Windows
  session を要する**。headless/CI では lane 全体と同じく
  `NotRun` + `environment_blocked` 系の記録に留まる（D-lane）。
  「headless でも走る crash/hang」の代替 oracle は設けない —
  POLICIES の「headless/fake/offscreen の成功は required real
  window/native dock/plugin ABI 証拠の代替にならない」規則に
  従う。

### D-followup: 後続実装 Issue の分割

**裁定: 実装を 5 slice に分割し、Blocked-by を下記のとおり
確定する。起票案（`.github/ISSUE_TEMPLATE/task.yml` 形式）は
PR 本文に記し、起票・依存登録は Coordinator が統合時に行う。**

| slice | 内容（対象 crate） | Blocked by |
| --- | --- | --- |
| (1) wire GUI verbs + `WIRE_VERSION` 6 | `ruviform-plugin-core/src/wire.rs`: `WIRE_VERSION=6`、`OpenGui`/`Opened` payload 拡張、tag 24-27 新設、`GuiSize`/`GuiKeyEvent`/`capabilities` bitfield codec、`Hello.capabilities` mask `0x1`（`capabilities & !0x1 != 0` → `Malformed`）、codec round-trip/malformed test | Issue #872（本設計） |
| (2) runner UI thread/pump + GUI dispatch | `ruviform-plugin-export/src/lib.rs`: lazy UI thread + Windows message pump + child 所有 container toplevel、GUI verb を UI thread へ marshal（reply 待ちは unbounded — bound は parent 側 `command_timeout`）、`gui_handle` 簿記、`Hello.capabilities` に `#[cfg(windows)]` `0x1` 広告、Shutdown で window 破棄+thread join | slice (1) |
| (3) `L1EditorOps` + composite/host routing | `ruviform-plugin-host`: `L1EditorOps` 実装（`LeaseRedeemer` 対称構成、`Docked`/`reattach`/未広告の早期 `Unsupported`、`capabilities` の negotiated/`Opened` 反映）、`l0_only_gui_verb` guard を `SetGuiParent` のみに縮小、`L1Instance::execute` の GUI gate を negotiated bit 化、`HostReply::Opened` に `capabilities`、notification→`notify_terminated` の pump helper、editor_session/adapter test | slice (1), (2) |
| (4) `test_editor` in-band `Fault` param | `ruviform-plugin-test-vst3`: `test_fault` の `FAULT_PARAM`/`decode_fault` 機構を `test_editor` にも載せる（`process()` 内で abort/exit/hang を発火できるようにする — hang 経路に必須、crash の代表 fault にもなる） | Issue #872（(1)-(3) と並列可） |
| (5) lane verdict flip | `ruviform-plugin-dock-test` + `ruviform-ui-test`: `ScenarioCapability::OutOfProcessPluginHost` 追加、catalog `required` 更新、case 本体を D-oracle どおり実装（Dedicated spawn + `open(Floating)` + trigger + `Terminated{HostLost}` oracle + lane 生存確認 + evidence marker + capability claim gate）、blocked probe 行の更新 | slice (3), (4) |

分割の根拠:

- (1)→(2)→(3) は wire contract → child 機構 → host 側経路の
  依存順で素直に並ぶ。(3) は (2) の runner なしでは E2E 検証
  が組めないため Blocked-by に含める（(1) のみなら実装自体は
  始められるが、verification 前提で依存を明記する）。
- (4) は fixture のみの独立 slice — `test_editor` に `Fault`
  param を載せる作業は wire/runner 実装と並列できるが、
  `real-session-hang` の唯一の起動経路になるため (5) より前に
  必須（crash 側は `kill_child` で `test_editor` 拡張なしに
  先行 flip も可能 — 実装時に (5) 内で「hang は (4) 待ち、
  crash は `kill_child` で先行」の段階付けをしてもよい）。
- (5) が最終の verdict flip — `real-session-crash`/
  `real-session-hang` が `Pass`/`Fail` を取れるようになる
  統合 slice であり、`#389` の crash/hang AC 残件を止める。

## Components and Boundaries

- `ruviform-plugin-core/src/wire.rs` — wire 語彙・codec・
  `WIRE_VERSION`・`Hello.capabilities` mask の所有点。
  `GuiSize`/`GuiKeyEvent`/`capabilities` bitfield の codec を
  ここに追加する（`ParentLease`/`GuiHandle` codec は既存）。
- `crates/ruviform-plugin-host/src/composite.rs` — `HostCommand`
  → `wire::Command` の `to_wire` 写像・`l0_only_gui_verb`
  guard・`HostNotification`/`ChildExit` の surface。
  `SetGuiParent` だけが L0-only に残る。
- `crates/ruviform-plugin-host/src/l1/mod.rs` — `L1Instance::
  execute` の negotiated-bit GUI gate・command/reply 経路・
  `command_timeout`→child-scope 収束・watchdog kill・
  `InstanceFailed` 通知。
- `crates/ruviform-plugin-host/src/editor_session.rs` —
  `L1EditorOps` の実装点（`InProcessEditorOps` と対称の
  ops adapter）。`map_host_error`/`fail_cause` は再利用。
- `crates/ruviform-plugin-export/src/lib.rs` — runner の
  UI thread/pump・container window・GUI dispatch marshal・
  `Hello.capabilities` 広告。
- `crates/ruviform-plugin-test-vst3` — fixture cdylib
  （`test_editor`/`test_fault`/`test_spawn_fail`）と
  l1-runner binary。slice 4 で `test_editor` に `Fault` param
  を追加する点。
- `crates/ruviform-ui-test/src/scenario.rs` —
  `ScenarioCapability::OutOfProcessPluginHost` の追加点。
- `crates/ruviform-plugin-dock-test` — `real_catalog.rs` の
  `required` 更新と `real_dock.rs` の crash/hang case 本体
  （lane 実行 + verdict 記録）。

## Data and Control Flow

open の正常系（`open(Floating)`）:

1. session driver が `EditorSessionOps::open(session, lease,
   size, AttachMode::Floating)` を呼ぶ — `L1EditorOps` は
   negotiated bit を確認し、無ければ `Err(Unsupported)`、
   mode が `Docked` でも `Err(Unsupported)`（early refusal）。
2. `L1EditorOps` は `LeaseRedeemer` で `ParentSlotLease` を
   redeem して `ParentLease`（owner hint の host HWND）を得、
   composite 経由で `wire::Command::OpenGui{mode:0, parent,
   size}` を対象 instance の child に送る。
3. parent 側 `L1Instance::execute` は negotiated bit gate を
   通して wire に出し、`command_timeout` 内の reply を待つ。
4. child の command loop は dispatch で `OpenGui` を検出し、
   lazy 起動した UI thread へ marshal — UI thread は child
   所有の container toplevel を作り `plugin.open_gui`
   （`ParentLease` payload から取り出した owner hint HWND）
   を呼び、`size` を適用、`GuiHandle` を mint して
   `Reply::Opened{gui, capabilities}` を返す。
5. parent は `Opened` を `L1EditorOps` 経由で
   `Ok(EditorCapabilities)` に写像 → session は
   `Attached{mode:Floating}` event を emit → `Attached`。

close/size/visibility/focus/key の正常系は同じ経路で
`CloseGui`/`SetGuiSize`/`SetGuiVisibility`/`SetGuiFocus`/
`SendGuiKey` が UI thread へ marshal され、plugin hook または
window op を適用して `Ok`/`Err` を返す。

crash 収束（lane 観測）:

1. `kill_child` または `test_editor` in-band `Fault` 発火 →
   child process 死亡。
2. driver thread の `try_wait` crash watch（または reader の
   stdout EOF）が検出 → child-scope 収束 → instance `Failed` →
   mint した instance へ `InstanceFailed{handle, exit{code,
   signal}, cause}` を exactly-once で enqueue。
3. lane/driver の notification pump が `try_recv_notification`
   で拾い `sessions.notify_terminated(handle, HostLost)` →
   `Terminated{HostLost}` once → `Closed`。child が死ぬとその
   window も道連れになる（child 所有ゆえ — host 側に orphan
   HWND は残らない）。

hang 収束（lane 観測）:

1. `test_editor` in-band `Fault` `hang` 発火 → child command
   loop が plugin call 内で wedged（child 生存・応答なし）。
2. lane が非 session probe `SaveState`（raw `host.execute`）を
   出す → child が返さない → `command_timeout` → `Err(Ipc)`。
3. converging thread が `Shared.child` 経由で wedged child を
   kill → `Failed{Timeout}` → `InstanceFailed{handle, exit,
   cause:Timeout}` once → `notify_terminated` →
   `Terminated{HostLost}` → `Closed`（wedge 中の audio leg は
   `Ok(CONTINUE)` passthrough substitution — 補助観測）。

## Non-Functional Constraints

- **RT/非 RT 境界**: session op と GUI wire 動詞は全て非 RT —
  UI thread/コマンド thread 側の責務。`process`/audio leg の
  RT 性を変えない（hang 中の passthrough substitution も既存
  機構）。
- **lane 生存**: crash/hang の観測中に host process / lane
  driver / host UI queue が殺されない・固まらないことを
  verdict 条件に含める（観測者が検証対象に巻き込まれない
  ことが lane の本質）。
- **determinism**: session event 列は `[Terminated{HostLost}]`
  のみを要求し、probe は raw command に限定して順序揺れを
  排除する。
- **honesty**: `Pass` には「negotiated bit 実測 + window 存在 +
  収束 event + lane 生存」の全てが要る — capability claim と
  verdict は実証ベースで、fake success / `Degraded` の混同を
  しない。

## Risks and Trade-offs

- **marshal reply 待ちの設計**: UI thread が plugin hook 内で
  wedge すると marshal reply が返らず command loop も止まる
  — これは故意に unbounded とし、bound は parent 側
  `command_timeout` に一本化する（child 内で独自の GUI-op
  deadline を持たない）。理由: wedged UI thread は window も
  plugin hook も捌けず、child が `Err` を返しても editor は
  失われている — 早い返答より「wedged child は parent が殺す」
  という既存収束との一貫性を取る。trade-off: wedge 検出が
  `command_timeout` 分だけ遅れる（観測 oracle では timeout
  自体が検証対象なので問題にならない）。
- **docked を残さない trade-off**: L1 editor は host dock
  layout に直接入らない — consumer が docked 見た目を要する
  場合は floating を「dock のように振る舞う owner 付き
  toplevel」として扱うか、将来の docked 設計（別 Issue）に
  委ねる。これは #115 の脆弱性判定と lane 生存の矛盾を解消
  するための意図的な限定であり、欠落ではなく裁定である。
- **`Hello.capabilities` の一方向性**: parent→child 広告が
  ないため「parent が GUI を要求する能力」を child が知る
  経路はない — v6 では不要（child は動詞到達時に機構が
  あれば応えるだけ）だが、将来「child 側が host 能力を見て
  挙動を変える」需要があれば bump で拡張が要る。
- **Bundled 同居の制約**: 異 plugin の同居ができないため、
  「editor plugin と fault source を別 plugin で分けたい」
  構成（fault を sibling に追いやる案）は取れない — lane
  は Dedicated + `test_editor` 自身の fault arm に依存する。
  これは観測したい収束（instance=editor=1 child）には最も
  素直な形でもある。
- **SetOwner の hint 性**: owner hint の適用失敗を `Err` に
  するか無視するか — v6 は `payload != 0` の hint を
  best-effort で適用し（適用不能でも open は成立）、厳密な
  owner 検証はしない。厳格さが要る需要（例: owner 必須の
  modal 的 editor）が出たら `Err(InvalidValue)` への昇格を
  別途設計する。
- **version bump の blast radius**: 単一最新版方針のため
  runner/host/test の全 wire touch が同一 bump にまとまる —
  #119 が定める運用どおり workspace 一体で上げる。

## Deferred Design Concerns

- **L1 docked（越境埋め込み）** — `mode:1` と `SetGuiParent`
  語彙は確定済みで payload 変更なく有効化できるが、実現は
  別 Issue。再着手条件: #115 の条件付き脆弱性を避ける越境
  attach 機構（OS 機構の再評価または non-queue-coupled
  attach）の設計が確定したとき。
- **plugin→host GUI 要求動詞**（`requestShow`/`resize`
  要求系 — #873 系列）— 別 Worker の lease 範囲。本設計の
  tag 24-27 と衝突しないよう、将来 bump で別 tag に載せる。
- **spawn/handshake fault lane** — argv
  `--ruviform-test-fault` の pre-Hello fault は session を持た
  ない別観測面（`SpawnFailed`/`Handshake`/`Timeout` oracle —
  `real-session-*` とは別の `real-spawn-*` 行候補）。本設計の
  lane では使わない。
- **非 Windows editor GUI** — adapter `open_gui` + runner
  UI thread の per-OS 実装（macOS `NSView` 所有 toplevel、
  X11 toplevel。Wayland は transient-hint の語彙設計が要る）。
- **floating editor の host 側合成表示** — child window を
  host の DComp visual tree / dock layout に見せる合成機構
  （`real-dcomp-*` 系の pixel gate には入らない — 別の
  表示設計領域）。
- **owner hint の厳格化** — `payload` 検証・owner 必須 mode・
  `Err(InvalidValue)` 昇格は需要が出た段階で。
- **Bundled 異 plugin 同居** — route 横断 pool が必要になった
  場合の設計（現行 lane は Dedicated で充足）。
- **`GuiCapabilities` 専用動詞** — open 前の plugin 個別 caps
  query が必要になったら次 bump で追加。

## Verification Notes

- [ ] **wire codec**: `mise run test ruviform-plugin-core` で
  tag 8/9・24-27・reply tag 3・`Hello.capabilities` mask の
  round-trip と、`Malformed` 規則（未知 mode・未知 bit・
  supplementary-plane `key`）の拒否を pin — slice (1)。
- [ ] **runner**: `mise run test ruviform-plugin-export` /
  l1-runner 系で、`GUI_FLOATING` 広告（`#[cfg(windows)]`）、
  lazy UI thread 起動、container toplevel 生成、GUI verb の
  marshal/reply、wedged hook → 無応答 → parent 側 timeout
  収束を pin — slice (2)。
- [ ] **host ops**: `mise run test ruviform-plugin-host` で
  `L1EditorOps` の early `Unsupported`（Docked・reattach・
  未広告）・`open(Floating)`→`Opened` 写像・capabilities
  反映・notification→`notify_terminated` pump・`map_host_error`
  再利用の写像を pin — slice (3)。
- [ ] **lane flip**: `mise run test-ui-plugin-dock-real`（
  interactive Windows）で `real-session-crash`/
  `real-session-hang` が `Pass`/`Fail` を取り、headless では
  `NotRun`/`environment_blocked` に留まり、`real_catalog_
  matches_entries` が green — slice (5)。
- [ ] **`SetOwner` 非結合の再検証**: `SetOwner`/`GWLP_OWNER`
  が input queue を結合しないことを実装 slice で Windows
  ドキュメント/挙動で再確認（#115 の対象は child-window 化に
  伴う AttachThreadInput であり owned toplevel は別機構だが、
  実装時に事実確認を残す）。
- [ ] **`Opened.capabilities` の bit 写像**: child が立てる
  bit（`float`/`close` 常時 + plugin reply）と host 側
  `EditorCapabilities` 反映の一致を unit test で pin。

## Sources and Relations

- `docs/discovery/issue-872-l1-oos-plugin-editor-host.md` —
  本設計の検討過程・却下案の記録。
- `docs/designs/issue-115-l1-gui-cross-process-parenting.md` —
  越境埋め込み採否の approved 設計（解除条件 (a)-(d) と
  入力キュー結合の確定事実 — 本設計の入力）。
- `docs/designs/issue-389-plugin-editor-session.md` — session
  層の approved 設計。D6 の capabilities 草案は本設計が確定版
  として supersede する（D6 本体に注記済み）。
- `docs/specs/issue-119-l1-wire-version-policy.md` — 単一最新版
  方針の正本（本設計の bump はその適用例）。
- `docs/specs/plugin-l1-out-of-process.md` — L1 の spec（L1 GUI
  の Deferred Design Concerns を本設計が引き取る）。
- `docs/specs/issue-389-ime-verbs.md`・
  `docs/specs/issue-389-real-session-lifecycle.md` — L1 GUI
  動詞/crash-hang out-of-process 化を対象外と明記していた spec
  — 本設計への相互参照を追記済み。
- `docs/specs/issue-389-plugin-editor-session.md` — session
  API の spec 正本。
- `docs/discovery/issue-389-real-session-lifecycle.md` —
  crash/hang が out-of-process を要するという discovery 記録。
- `evidence/rf631/plugin-dock-real-scenario.jsonl` — 現行
  `Unsupported` verdict と marker の canonical 帳簿（本 slice
  では参照のみ）。
- `crates/ruviform-plugin-core/src/wire.rs`・
  `crates/ruviform-plugin-host/src/l1/mod.rs`・
  `crates/ruviform-plugin-host/src/composite.rs`・
  `crates/ruviform-plugin-host/src/editor_session.rs`・
  `crates/ruviform-plugin-export/src/lib.rs`・
  `crates/ruviform-plugin-dock-test/src/real_dock.rs`・
  `crates/ruviform-plugin-dock-test/src/real_catalog.rs`・
  `crates/ruviform-plugin-test-vst3/src/bin/
  ruviform-plugin-test-vst3-l1-runner.rs`・
  `crates/ruviform-plugin-test-vst3/tests/
  l1_fault_test_plugin_e2e.rs` — 設計確定事項の根拠コード。
