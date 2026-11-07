---
id: "DISC-c2G2365EomTupKRjR3zoqt"
type: "discovery"
status: "resolved"
title: "Issue #388/#389 Native Plugin Dock 現行実装監査（GuiParentToken / L1 Unsupported / wire / Dock 契約の棚卸し）"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/384"
    - "https://github.com/scarecrow173/ruviform/issues/262"
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
  related:
    - "docs/designs/issue-115-l1-gui-cross-process-parenting.md"
    - "docs/designs/issue-181-multi-window.md"
    - "docs/designs/issue-182-dock-tree.md"
    - "docs/adr/plugin-gui-embedding.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-115-l1-gui-cross-process-parenting.md"
  derived-by:
    - "docs/specs/issue-388-native-surface-slot.md"
    - "docs/specs/issue-389-plugin-editor-session.md"
    - "docs/designs/issue-388-native-surface-slot.md"
    - "docs/designs/issue-389-plugin-editor-session.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/115"
    - "https://github.com/scarecrow173/ruviform/issues/386"
    - "https://github.com/scarecrow173/synvelis/issues/176"
  defers: []
  deferred-by: []
---

# Issue #388/#389 Native Plugin Dock 現行実装監査

## Summary

RF388（Driver-neutral `NativeSurfaceSlot` 契約・GPUI Dock 実 GUI 埋込
PoC/採用 Gate）と RF389（Driver-neutral Plugin Editor Session・L0/L1
Native Dock/Floating/Recovery Adapter）の設計入力として、現行の
GUI 関連公開 API・wire・Dock 契約・L1 実装を監査した。

**監査確定事実（コード・テスト・approved 設計書の実測値）**:

1. `GuiParentToken(u64)` は「HWND や `NSView *` を caller が整数へ縮約した
   raw native parent」を運ぶ旧 Boundary 語彙であり、発行・世代・寿命の
   検証機構を持たない（`crates/ruviform-plugin-core/src/boundary.rs`）。
2. L1 の `OpenGui`/`CloseGui` は host-side short-circuit で
   `Err(HostError::Unsupported)` を返す — wire に出ず子プロセスにも
   届かない（`crates/ruviform-plugin-host/src/l1/mod.rs`
   `L1Instance::execute`、e2e `gui_defined_rejection` が pin）。
   **L1 Native GUI は現時点で未実装。**
3. L0 の `InProcessHost::open_gui` は `plugin.open_gui(parent)` へ
   dispatch する live 経路を持つが、`Plugin::open_gui` の既定実装は
   `Err(HostError::Unsupported)` であり、CLAP/VST3 bridge は現時点で
   `open_gui` を実装していない — **全バックエンドで実 GUI open は
   実質不可能**（adapter 実装は別 Issue 級）。
4. wire 語彙は GUI を既に保持する — `wire::Command::OpenGui{parent}`（tag 8）
   /`CloseGui`（tag 9）/`wire::Reply::Opened{gui}`（tag 3）は
   Issue #115 で keep-for-future 確定済み。`WIRE_VERSION = 3` で
   `Hello.capabilities` は非 0 を `Malformed` 拒否（hardening）。
   新動詞の追加は `WIRE_VERSION` bump を要する（Issue #119 確定）。
5. 子プロセス側 `ruviform-plugin-export` runner は `WireCommand::OpenGui`
   の dispatch 腕を持つが stdin/stdout 単一スレッドループで GUI
   メインスレッド意味論を満たせず、host 側も転送しないため到達不能。
6. `#123` で `gui_open` 簿記 field は除去済み — ≤1 surface/インスタンス
   強制は「保持すべき field ではなく設計指針」として本 Issue へ
   hand-off 済み。
7. `ruviform-ui` には `WindowId`/`ViewInstanceId`/`DockWorkspace`
   （2-phase `begin_move`/`commit_move`/`abort_move` + `MoveToken` +
   `ViewBusy`）と旧 `NativeSurfaceHost`（`create/move/destroy` の
   WebView-adjacent 前提 port、実装は `ruviform-gpu` 候補・未実装）が
   存在する。
8. アーキテクチャベースライン確定規則: **plugin 系と ui 系の相互依存は
   禁止（プラグイン GUI 統合は利用側で配線する）**。
9. RF386 側 `ruviform-ui-gpui` クレートは骨格のみ（`gpui = "0.2.2"`
   を非既定 `gpui` feature で pin、driver 実装・最小 Window fixture は
   未着地）— 実 Native Dock PoC は本時点で未着手（Blocked on RF386）。

## Exploration Goal

- 現行公開 API・wire・実装の「GUI 関連の真の状態」を、設計合意（新
  契約）と実装済み・検証済みを混同せずに固定する。
- 新契約（`SurfaceSlotId`/`ParentSlotLease`/`PluginEditorSessionId`/
  `PluginEditorSession`/`NativeSurfaceSlot` port）が置かれるべき
  クレート境界を依存方向規則から導く。
- 既存契約（`DockWorkspace` 2-phase move・`WindowId`/
  `ViewInstanceId`・wire GUI 語彙）の再利用可否を確定する。

## Findings

### F1: `GuiParentToken` の実像（旧 Boundary 語彙）

`boundary.rs` の定義と語彙集合:

```rust
pub struct PluginHandle { slot: u32, generation: u16 }   // generation 付き
pub struct GuiHandle(u32);                               // 生 token
pub struct GuiParentToken(u64);                          // 生 token
```

- `GuiParentToken::new(u64)` は public `const fn` — caller が任意の u64
  を作れる。doc は "a HWND or `NSView` pointer narrowed to an integer
  by the caller" で、in-process 意味論（プロセス局所ポインタを整数化）
  が埋め込まれている（issue-115 design の「非対称性」指摘どおり）。
- `PluginHandle` は `{slot, generation}` で stale 検出可能だが、
  `GuiParentToken`/`GuiHandle` は世代なし裸値 — 「opaque,
  generation-bound, lifetime-checked」な `ParentSlotLease` 相当が
  存在しない。
- wire は `GuiParentToken` を `u64` LE で encode/decode するのみで
  解釈しない（`wire.rs` `encode_gui_parent_token`/`decode_gui_parent_token`）。

### F2: L1 GUI の実装状態 = defined rejection（未実装）

- `l1/mod.rs` `L1Instance::execute` は `wire::Command::OpenGui{..}`/
  `CloseGui` を `Err(HostError::Unsupported)` で short-circuit —
  `Failed`/`Ipc` チェック後・Activate 上限チェック前で、wire へも
  子プロセスへも出さない。
- 実子プロセス E2E（`l1_proxy_e2e.rs` `gui_defined_rejection`、
  clap/vst3 `l1_real_adapter_e2e.rs`）がこの拒否を pin 済み。
- `composite.rs` の `to_wire`/`host_reply` には `OpenGui`→`Opened`
  の写像があるが L1 経路では到達不能。
- Issue #115 design（approved）は defer を確定: Windows 越境
  `SetParent` は入力キュー結合で「条件付き（脆弱）」、X11 は
  XEmbed 旧式機構のみ、Wayland/macOS は原理的に不可。
  解除条件 (a)–(d) のうち (a) wire 版枠組みのみ確定済み。

### F3: L0 `open_gui` 経路は live だが adapter 未実装

- `HostCommand::OpenGui{handle, parent}`/`CloseGui{handle}` と
  `HostReply::Opened{gui}` は command surface に存在。
- `InProcessHost::open_gui` → `plugin.open_gui(parent)` → 既定
  `Err(Unsupported)`。CLAP host bridge（`ruviform-plugin-clap`）と
  VST3 host bridge（`ruviform-plugin-vst3`）は `open_gui` を既定の
  まま実装していない（`unsupported_state_and_gui_verbs_are_defaults`
  系テストが pin）。
- つまり現状で実プラグイン GUI を開ける経路は**どのバックエンドにも
  ない**。`plugin-gui-embedding` ADR（wry `build_as_child`、Windows
  実機 pass）は `poc/plugin-gui` 独立 WS の実証であり、本番クレートの
  `open_gui` 実装ではない。

### F4: wire の GUI 面と negotiation 余地

- `OpenGui`/`CloseGui`/`Opened` 動詞は WIRE_VERSION 3 で既存
  （keep-for-future）。
- `Hello { version: u16, capabilities: u32 }` — `capabilities != 0`
  を decode が `Malformed` 拒否（hardening）。機能ネゴシエーション
  機構は現行 wire に存在せず、追加は #119 の bump 政策の対象。
- GUI lifecycle の実動詞（`set_size`/`show`/`hide`/`set_scale`/
  reparent/detach 相当）は wire に**存在しない** — L1 GUI を実装する
  場合は bump 前提の語彙追加が必要（issue-115 design と一致）。

### F5: UI 側の既存契約（再利用対象）

- `window.rs`: `WindowId(NonZeroU64)`（単調採番・破棄後再利用しない
  論理 ID）、`WindowEvent`/`RoutedEvent`/`CloseDecision`/
  `WindowHandle` port、`MonitorInfo`。
- `dock.rs`: `ViewInstanceId(NonZeroU64)`、`DockWorkspace`
  （`begin_move`→`commit_move`/`abort_move` の 2-phase、`ViewBusy`
  transfer lock、`MoveAborted`/`MoveCommitted` イベント、
  `WindowCloseDisposition` 3 ポリシー）。純粋モデルで headless 検証可能。
- `render.rs`: `RenderTier::{Dom,Canvas2d,Webgl2,NativeSurface}` と
  `NativeSurfaceHost`（`create_surface`/`move_surface`/`destroy_surface`
  + `SurfaceRegion` 物理 px + `SurfaceId`）— **WebView 隣接前提の旧
  port** で、RF386 仕様どおり新 `NativeSurfaceSlot` 意味論への写像先。
  trait の実装は存在しない（`ruviform-gpu` 候補・未実装）。
- `ruviform-ui-test`: fake host・fixture/clock/fault/観測 port 群を持つ
  test-only 基盤（本番クレートから依存されない）。
- GPUI Driver（`ruviform-ui-gpui`）は RF386 worktree に骨格のみ存在し
  未着地 — 本監査時点で `NativeSurfaceSlot` の実 Driver 実装は存在しない。

### F6: 依存方向の確定規則（配置の決定根拠）

`docs/designs/ruviform-architecture-design-baseline.md` 確定値:

- `ruviform-plugin-core` → `ruviform-plugin-host` → CLAP/VST3 adapter。
- `ruviform-ui` → `ruviform-ui-webview`（port 定義元への adapter 依存）。
- **plugin 系と ui 系の相互依存は禁止（プラグイン GUI 統合は利用側で
  配線する）**。
- 境界 trait は port 定義元クレートに置き、adapter はそれを実装する
  （media-io / audio-core / ui webview の確立パターン）。

導かれる配置:

| 型・契約 | 配置 | 根拠 |
| --- | --- | --- |
| `SurfaceSlotId`/`SlotLease`/`SurfaceSlotRegistry`/`NativeSurfaceSlot` port | `ruviform-ui` | Window/Dock と同層の Driver 境界定義元 |
| `PluginEditorSessionId`/`ParentSlotLease` | `ruviform-plugin-core::boundary` | Host↔Driver 共有語彙（`PluginHandle` と同じ slot+generation 形） |
| `PluginEditorSession`/`EditorSessionOps` port/`EditorCapabilities` | `ruviform-plugin-host` | セッション所有・L0/L1 統合点 |
| lease→OS handle 解決・GuiParentToken への写像 | GUI Driver / 利用側 composition | plugin↔ui 相互依存禁止のため共有型をまたぐ変換は消費側 |

### F7: OS/format 機構の基礎知見（issue-115 design からの承継）

- Windows: 同一プロセスなら `WS_CHILD` 親子で成立（#30 PoC 実機）。
  越境 `SetParent` は入力キュー暗黙結合で脆弱 — crash isolation と
  構造的に緊張。
- macOS: 同一プロセス `NSView` attach は規格上成立。越境 NSView
  reparent の公認 API は存在せず、`CAContext`/`CALayerHost` 系は
  入力転送を持たない合成専用 → L1 越境 attach は不可。
- Linux X11: XEmbed（`GtkSocket`/`GtkPlug`、GTK3 では deprecated だが
  現存）経由で理論上成立 — VST3 `"X11EmbedWindowID"`・CLAP
  `clap_window.x11` は XID 前提。
- Wayland: `xdg-foreign` は toplevel 間 hint のみでクライアント間
  サーフェス埋込機構を提供しない → native embed は Unsupported。

## Decisions Recorded

- 新契約のクレート配置は F6 の表どおり確定（依存方向規則から一意）。
- L1 GUI は「現行 Unsupported のまま維持」し、L1 対応を黙って L0 へ
  退化させない — session/capability 語彙で L0/L1 を分離して表現する。
- `GuiParentToken`/wire GUI 語彙は keep-for-future を維持し、公開
  破壊（削除・意味変更）を行わない — session API は新 `ParentSlotLease`
  を取り、raw `GuiParentToken::new(u64)` の呼出は OS adapter 内部へ
  隔離する設計指針を design で確定する。
- GPUI 実機 Dock PoC は RF386 の最小 GPUI Window fixture 到着まで
  Blocked — 本 dispatch は契約・モデル・fake 検証とマトリクス
  事前評価を進め、実機行は NotRun/Blocked で記録する。

## Open Questions

- RF386 の `NativeSurfaceSlot` 消費点（GPUI Driver が lease を mint
  する具象境界）— RF386 側 contract 着地後に合流確認が必要。
- 実プラグイン binary fixture（固定/可変サイズ・別 toolkit の
  VST3/CLAP）の入手経路とライセンス — 未入手時は当該行 NotRun。
- `open_gui` の CLAP/VST3 adapter 実装本体は別 Issue 級（本 dispatch
  のスコープ外、session capability で `unsupported` を返す）。

## Next

`docs/specs/issue-388-native-surface-slot.md`・
`docs/specs/issue-389-plugin-editor-session.md` へ spec として
引き継ぐ。
