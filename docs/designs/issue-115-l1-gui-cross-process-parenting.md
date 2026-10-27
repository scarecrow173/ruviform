---
id: "DESIGN-034N3J1C5vystrbJl69R1q"
type: "design"
status: "approved"
title: "L1 GUI — プロセス境界越しの親ウィンドウ埋め込みの採否設計（Issue #115）"
created: "2026-10-27"
updated: "2026-10-27"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/115"
    - "https://github.com/scarecrow173/ruviform/issues/76"
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
    - "docs/adr/plugin-architecture-clap-vst3.md"
    - "docs/adr/ui-architecture-webview-hybrid.md"
    - "docs/adr/ui-tech-selection-poc.md"
    - "docs/designs/issue-114-real-adapter-l1-e2e.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/plugin-l1-out-of-process.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/discovery/plugin-l1-out-of-process.md"
    - "docs/specs/plugin-gui-poc.md"
    - "docs/designs/plugin-gui-poc.md"
    - "docs/adr/plugin-gui-embedding.md"
    - "docs/designs/plugin-isolation.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/30"
    - "https://github.com/scarecrow173/ruviform/issues/57"
    - "https://github.com/scarecrow173/ruviform/issues/118"
    - "https://github.com/scarecrow173/ruviform/issues/119"
    - "https://github.com/scarecrow173/ruviform/issues/123"
    - "https://github.com/tauri-apps/wry/pull/1762"
    - "https://github.com/novonotes/wxp"
  defers: []
  deferred-by:
    - "docs/specs/plugin-l1-out-of-process.md"
    - "docs/designs/plugin-l1-out-of-process.md"
---

# L1 GUI — プロセス境界越しの親ウィンドウ埋め込みの採否設計（Issue #115）

## Context

Issue #115 — #76 で L1 インスタンスへの `OpenGui`/`CloseGui` 要求を `Err(HostError::Unsupported)` とする defined rejection を選択した（`crates/ruviform-plugin-host/src/l1/mod.rs` `L1Instance::execute` の host-side short-circuit — wire に出さず子プロセスへも転送しない）。L1 spec は本判断を Out of Scope で「defined rejection とするか明示除外するかは design が確定」と委譲し、L1 spec/design 双方の Deferred Design Concerns に「L1 GUI（プロセス境界越し親ウィンドウ埋め込み）」行として残った。本書はその委譲を受けて**採否（実装するか `Unsupported` 維持か）を確定する**。

確定の入力となる現行実装の事実:

- wire 語彙は既に GUI 対応を含む — `wire::Command::OpenGui { parent: GuiParentToken }`（tag 8）・`CloseGui`（tag 9）・`wire::Reply::Opened { gui: GuiHandle }`（`crates/ruviform-plugin-core/src/wire.rs`）。`GuiParentToken` は `u64` の opaque 値型で「a HWND or `NSView` pointer narrowed to an integer by the caller」と doc 済み（`crates/ruviform-plugin-core/src/boundary.rs`）
- `Plugin::open_gui`/`close_gui` の既定実装は `Err(HostError::Unsupported)`（`plugin.rs`）。CLAP/VST3 bridge は現時点で `open_gui` を実装せず既定のまま（`unsupported_state_and_gui_verbs_are_defaults` テストが pin）
- L1 host 側は `execute()` が `OpenGui`/`CloseGui` を `Err(Unsupported)` で short-circuit（wire 非送信）。Issue #114 の real-adapter L1 E2E も `Err(Unsupported)` 返却を実子プロセスで pin 済み
- 子プロセス側 `ruviform-plugin-export` runner は **stdin/stdout frame の単一スレッドメッセージループ**（`src/lib.rs` — "single-threaded message loop"）。`WireCommand::OpenGui`/`CloseGui` の dispatch 腕は存在するが（`lib.rs` `Runner::dispatch`）、仮に到達すればコマンドループスレッド上で `plugin.open_gui()` を呼ぶ — GUI イベントポンプも UI スレッドも持たない
- GUI 関連の先行知見はすべて**インプロセス**前提: #30 PoC（wry `build_as_child` → ホスト提供 HWND・Windows 実機）・ADR `plugin-gui-embedding`（素 wry 採用・accepted）・#57 follow-up（同一親内 wry 8 個 60fps・Linux は `build_gtk` on `gtk::Fixed` = 同一プロセス GTK コンテナ）・`ui-tech-selection` ADR（wry PR #1762 未 merge・wxp はインプロセスライフサイクルパッチの参照実装）
- 直交する別平面として Issue #118（OFX/NONRT video の L1 経路・`VideoPluginInstance`）が追跡中 — video port は本書が扱う GUI 動詞面（`OpenGui`/`CloseGui`/`set_parent` 意味論）とは別の wire/port 設計であり、本書の採否・解除条件は video 経路を拘束しない（逆も同様）

## 決定サマリ（Issue スコープ → 本書の結論）

| 判断 | 結論 |
| --- | --- |
| 現行 `Unsupported` 拒否を維持するか実装するか | **defer — `Err(HostError::Unsupported)` 維持を確定**（host-side short-circuit のまま。解除条件は「境界の pin」節） |
| `OpenGui`/`CloseGui` wire 動詞・`Reply::Opened`・`GuiHandle`/`GuiParentToken` | **keep-for-future** — L0 コマンド集合との parity と L0 GUI 将来実装経路のため存置。除去は `WIRE_VERSION` bump を要する破壊的変更で利益なし |
| `gui_open` write-only field（`host.rs` Instance 簿記）の使途 | **removed — #123 で除去済み** — 宣言・初期化・4 write に対し read 0 件の dead field と確証され、「orphan ゲート追加」ではなく「除去」が採択・実施された（理由は「境界の pin」節）。本書の keep-for-future 判定は wire 語彙側にのみ残る |
| 採否確定後の後続 | 本書 approve で design ゲート完了。[impl] Issue は解除条件の顕在化時に起票（現時点で起票しない） |

## OS 別メカニズム調査（プロセス境界越しの親ウィンドウ埋め込み）

規格側が要求するのは「子プロセス内のプラグインが作るネイティブビューを、ホストプロセス所有の親ウィンドウの子として埋め込む」こと。OS ごとの実現可能性:

### Windows — 越境 `SetParent`/`WS_CHILD`

- HWND はセッション内グローバルなカーネルハンドルであり、別プロセスの HWND を親とする `SetParent`/`WS_CHILD` 作成は**技術的に動作する**（実世界先例: 32↔64bit ブリッジ型プラグインホストや DAW の sandbox 経路）
- ただし越境親子は**入力キューを暗黙結合**し（`AttachThreadInput` 相当の attached queue）、DPI awareness mismatch・メッセージポンプ停止の相互伝播・z-order/フォーカス規則の弱保証を伴う。子プロセスのハングや応答停止が**ホスト UI スレッドへ直接波及**し得る — L1 の存在理由である crash isolation と構造的に緊張する
- wry 経路: #30 で実機検証したのは `build_as_child` → **同一プロセス**の親 HWND。外部プロセス HWND 指定は内部的に同じ親子化経路を通るはずだが未検証であり、WebView2 の子 HWND ツリー（`Chrome_WidgetWin_*`）を他プロセス窓へ下げた場合の shutdown・IME・DPI 挙動は undocumented 領域。`RESULTS.md` の実証は越境ケースをカバーしない
- 判定: **条件付きで成立（脆弱）**。Windows 限定・isolation 弱体化を受け入れる場合のみ

### X11 — XEmbed 越境

- X プロトコル上の Window はサーバ側オブジェクト（プロセス非依存）のため越境埋め込みが原理的に可能。正規機構は **XEmbed**（`XEMBED` プロトコル・`GtkSocket`/`GtkPlug` 実装）であり、VST3 `IPlugView` の Linux platform type `"X11EmbedWindowID"` はまさに XEmbed ID を期待する
- ただし: (a) wry の Linux 子 attach は `WebViewBuilderExtUnix::build_gtk` — **同一プロセス内の `gtk::Container`** を要求し（#57 Linux 検証は親 `gtk::Fixed`・in-process）、`GtkSocket` ベースの越境 attach 経路は wry に存在しない。(b) `GtkSocket`/`GtkPlug` の存否は GTK 世代で段階的: GTK3 では deprecated だが現存する機構であり、wry の現行依存 WebKitGTK 4.1 は GTK3 系 — つまり機構自体は依存木内に残るが wry API は expose しない（(a) の通り）。除去されるのは GTK4 以降で、Wayland にはそもそも存在しない。(c) **Wayland は越境埋め込みを設計上提供しない** — `xdg-foreign` は toplevel 間の transient-parent hint のみで、クライアント間サーフェス分離がプロトコル原則
- 判定: X11 セッション限定なら XEmbed 理論上成立。現行 wry/GTK4/Wayland 経路では**実質不可**

### macOS — `NSView` 越境

- `NSView` 階層はプロセス内のオブジェクトグラフであり、別プロセスの view を子にする公認 API は**存在しない**。VST3 type `"NSView"`・CLAP `clap_window` cocoa は同一プロセス内ポインタ受け渡しを前提とする
- 出荷済みの越境機構として CoreAnimation の `CAContext`/`CALayerHost`/`CARemoteLayer` 系（及び deprecated の `NSRemoteView`）が存在するが、これは**合成（compositing）専用経路** — 他プロセスの layer ツリー描画を自プロセスへホストできる一方、公認の入力転送機構を持たず hit-testing・responder chain・focus は越境できない。よって `set_parent(NSView*)` の「子 NSView として機能する埋め込み」意味論を満たせず、却下の結論は変わらない。別プロセス `NSWindow` child 化・`IOSurface` 合成も同種の合成経路で、hit-testing・resize・lifecycle を全て自前で再実装する領域 — プラグイン GUI 契約の単純な写像として成立しない
- 判定: **不可**（越境 view embed の公認機構なし — `CAContext`/`CALayerHost` 系は入力転送を持たない合成専用）

### 調査まとめ

| OS | 越境機構 | wry/現行経路との整合 | 判定 |
| --- | --- | --- | --- |
| Windows | `SetParent`/`WS_CHILD`（入力キュー結合付き） | `build_as_child` の越境指定は未検証・WebView2 foreign-parent は undocumented | 条件付き（脆弱） |
| X11 | XEmbed（`GtkSocket`/`GtkPlug` — GTK3 では deprecated だが現存・GTK4 で除去） | `build_gtk` は in-process コンテナ前提・`GtkSocket` 経路を expose しない | 旧式機構のみ |
| Wayland | なし（`xdg-foreign` は toplevel hint のみ） | — | 不可 |
| macOS | `CAContext`/`CALayerHost`/`CARemoteLayer` + deprecated `NSRemoteView`（合成専用・公認入力転送なし） | — | 不可 |

## 規格 GUI 意味論・runner 制約との衝突

- **GUI thread affinity**: CLAP `clap_plugin_gui` の全メソッドは `[main-thread]` 呼出し規約（`create`/`destroy`/`set_parent`/`set_size`/`show`/`hide`/`set_scale`/…）、VST3 `IPlugView` も UI/main thread 規約。L1 ではプラグインは**子プロセス**内に生きるため、GUI 呼出しは「子プロセスの UI スレッド + メッセージポンプ」でサービスする必要がある
- **現行 runner との不整合**: `ruviform-plugin-export` は stdin/stdout frame の単一スレッドループで、GUI を動かすスレッドモデルを持たない。仮に host 側 short-circuit を外して動詞を通しても、コマンドループスレッド上で `open_gui` を呼ぶ構造は規格の main-thread 意味論もイベントポンプ要件も満たさない — 実装には runner の再設計（UI スレッド起動・コマンドの UI スレッド marshaling・pump 統合）が前提になる
- **wire 語彙不足**: 現行 wire は `OpenGui`/`CloseGui` のみで GUI lifecycle の実動詞（`set_size`/`show`/`hide`/`set_scale`/`adjust_size` 相当）を持たない。追加は wire policy 上 `WIRE_VERSION` bump が前提（`wire.rs` — "new verbs arrive only with a [`WIRE_VERSION`] bump"）であり、その枠組み自体が Issue #119 の未決事項
- **`GuiParentToken` の非対称性**: `u64` トークンは HWND（カーネルハンドル・セッション共有可）や XID（サーバグローバル ID）を運べるが、`NSView *` は**プロセス局所ポインタ**で他プロセスには意味を持たない。トークン doc が HWND/NSView pointer を例示する時点で in-process 意味論が埋め込まれており、この非対称性自体が「全 OS で統一できる越境意味論が存在しない」ことを裏付ける

## 採否判定基準と判定

| # | 基準 | 評価 |
| --- | --- | --- |
| 1 | 対象 3 OS で成立する機構があるか（部分 OS 限定機能は L1 契約の一貫性を損なう） | **不合格** — Windows のみ条件付き、X11 は旧式機構のみ、Wayland/macOS は原理的に不可 |
| 2 | crash isolation 不変条件を維持するか（L1 の存在理由） | **不合格** — Windows 越境親子は入力キュー結合で子のハングをホスト UI へ波及・子死亡時の孤児窓/リペアレント処理を要する |
| 3 | 既存 wire・スレッドモデルへの侵入度 | **大** — wire 動詞拡張（#119 前提）+ child runner の UI スレッド/pump 再設計 + per-OS shim（Win32 越境・XEmbed・macOS 代替なし）|
| 4 | L0 で代替可能か | **回避経路あり** — L0 では `open_gui` がプラグイン実装へ到達する経路が既に live で、in-process attach 機構は #30/ADR で確定・#57 で複数 GUI 共存も実証済み（adapter 側 `open_gui` 実装自体は未着手・別 Issue 級） |

**判定: defer — `Err(HostError::Unsupported)` 維持が正解。** 4 基準のうち 1–3 を同時に満たさず、唯一成立し得る Windows 経路は isolation を弱体化し、回避経路（L0）は既に存在する。「defer」は調査不足による保留ではなく、OS 機構・規格 thread 意味論・runner 構造の 3 層で積極的に裏付けた確定判断である。

## 境界の pin（deferred の確定形）

### `Unsupported` のまま残るもの

- `wire::Command::OpenGui { .. }`/`CloseGui` → `L1Instance::execute` が `Err(HostError::Unsupported)` を返す host-side short-circuit（`crates/ruviform-plugin-host/src/l1/mod.rs`）。wire に出ず子プロセスにも届かない — Issue #114 real-adapter E2E が pin する意味と同一
- この拒否は `Failed`/`Activate` 上限超過と同じ「定義済み拒否」カテゴリであり、wire codec・runner・adapter 側に変更を要求しない

### 解除条件（re-engagement triggers — 全て揃った時点で [impl] Issue 起票を再検討）

- **(a) wire versioning 枠組みの確定** — Issue #119。GUI lifecycle 動詞（size/show/hide/scale 系）の追加は `WIRE_VERSION` bump と capability 設計を前提とし、現行 `capabilities != 0` を `Malformed` 拒否する hardening とも整合させる必要がある
- **(b) child runner の UI スレッド + event pump 設計** — 単一スレッド stdin/stdout ループと規格 main-thread 意味論の両立設計（コマンドの UI スレッドへの marshaling・`process` 脚との分離維持）
- **(c) per-OS 実現可能性の再評価** — Windows: 越境 `SetParent` + WebView2 foreign-parent の実機検証 / Linux: X11 セッション限定 XEmbed の受容 or Wayland 新機構の出現 / macOS: 公認の越境 view 機構の出現（既存 `CAContext`/`CALayerHost` 系への入力転送追加を含む）。少なくとも 1 OS で robust な経路が立つことが最低条件
- **(d) 需要の顕在化** — GUI 必須かつクラッシュ隔離が同時に要求されるプラグイン利用形態の出現（L0 で代替不能なケース）

### `gui_open` field の fate（Issue #123 で「除去」として解決済み）

**removed — sibling Issue #123 は本書初稿の keep-for-future 回答と異なる解決（除去）を採択・実施済みであり、本節はそれに整合するよう訂正する。** `host.rs` `Instance::gui_open` は宣言・初期化・`open_gui`/`close_gui` 内の 4 write を持つが、全クレート範囲で read 0 件の dead field と確証され（`rg gui_open crates/` = 0 件 — spec `docs/specs/issue-76-followup-l1-internal-cleanup.md`・IR `docs/impl/ir/issue-123-l1-internal-cleanup-implementation.md`）、「double-`OpenGui` orphan ゲート追加」ではなく**除去**として実施された。gate が一度も存在しなかったため除去は挙動中立であり、wire の `OpenGui`/`CloseGui` defined rejection は不変。

#123 reviewer の判定要点（本書はこれを受け入れる）: (i) keep+gate は unreachable-path の投機的状態になる — 現状は全ての `open_gui` が `Err(Unsupported)` を返すため gate は発火し得ない、(ii) #76 Task 1 IR は `gui_open` について「使途を決定 or 除去」を許容選択肢として記録済み、(iii) 将来の実ゲートが欲するのは bare bool ではなく `Option<GuiHandle>` 型の状態、(iv) wire 面（動詞・`GuiHandle`・`GuiParentToken`）は存置で、失われたのは dead な host 側簿記 field のみ。

**本書に残る判定**: wire 語彙（`OpenGui`/`CloseGui`/`Reply::Opened`/`GuiHandle`/`GuiParentToken`）の keep-for-future は不変であり、orphan ゲートの意図（1 インスタンスにつき同時に開く GUI surface は高々 1 つ — `GuiHandle` は Drop-less トークンのため二度目の `Opened` は先の handle を leak させる）は**保持すべき field としてではなく設計指針として存続**する。将来 L0/L1 で GUI を実装する Issue は `Option<GuiHandle>` 型の ≤1-surface 強制 + destroy 時 auto-close 相当を含む自身の設計済み状態を導入する — #123 IR Follow-ups が「Issue #115（GUI 設計）は `gui_open` とは別の、設計済みの状態/invariant を導入すること」と hand-off を記録済み。

### wire / `#[non_exhaustive]` 互換ノート

- **wire 変更ゼロ**: `OpenGui`/`CloseGui`/`Reply::Opened`/`GuiHandle`/`GuiParentToken` は wire 語彙・公開型として存置。理由は (i) L0 コマンド集合との parity（公開面「L0 と同一」要件）、(ii) 将来の L0 GUI 実装経路・L1 解除時の再利用、(iii) 除去は `WIRE_VERSION` bump を要する破壊的変更で利益なし、の 3 点
- **新規公開面ゼロ**: `Err(HostError::Unsupported)` は既存 variant で完結し、新 variant・新 enum・`#[non_exhaustive]` 圧力は発生しない。`HostError` は既に `#[non_exhaustive]` 済みで将来 `GuiUnavailable` 系 variant を additive に足せる余地も残る
- **capability 空間は不触**: 現行 `Hello.capabilities` は `WIRE_VERSION=1` で非 0 を `Malformed` 拒否 — GUI capability を切るなら #119 の capability bit 設計領域であり、本判定はその前提を変えない

## Risks and Trade-offs

| リスク / トレードオフ | 評価と緩和 |
| --- | --- |
| GUI 持ちプラグインが L1 で使えない状態が続く | L0 で代替可能（既定バックエンド）。defined rejection は利用側が L0 へフォールバックする材料として既に機能する |
| 「defer」が無期限の事実上の却下になる | 解除条件 (a)–(d) を明示し、(a) は既に #119 として issue 化済み。(b)–(d) は需要・OS 機構の顕在化トリガであり、定期再評価ではなく条件発生駆動とする |
| 将来 Windows だけ先行実装したくなる | 判定基準 1（3 OS 一貫性）と基準 2（isolation 弱体化）を再確認。単一 OS 例外を認めるなら「Windows-only L1 GUI」という明示的な契約限定が別途 design 級 |
| wire 動詞の存置が「使わない語彙を抱える」負債に見える | wire の所有は plugin-core・L0 語彙との同一性が公開契約。削除利益より parity 破壊と bump コストの方が大きい |

## Deferred Design Concerns

- L1 GUI 実装本体（越境埋め込み + runner UI スレッド + wire 動詞拡張） | 上記 4 基準を同時に満たさず YAGNI/構造上の非整合 | 解除条件 (a)–(d) の揃い | GUI 必須プラグインを L1 で使えない（L0 選択で回避可能）
- child runner の UI スレッド/event pump 設計 | L1 GUI 採否未確定の間は設計対象にならない | L1 GUI [impl] Issue 起票時 | runner 再設計は大規模 — 早期着手の利益なし
- L0（in-process）GUI 実装 | 本 Issue の境界は L1 採否まで。L0 経路は #30/ADR で attach 機構が確定済みだが実装は別 Issue 級 | engine/利用側からの GUI 需要 | `open_gui` 既定 `Unsupported` のままでは全バックエンドで GUI 不可
- `gui_open` double-open ゲート | #123 で field 除去として解決済み（gate 非採用 — 発火し得ない unreachable-path の投機的状態を避けるため）。orphan 回避の意図は保持すべき field ではなく設計指針へ移行 | 将来 GUI を実装する Issue（L0 GUI [impl] 等）が `Option<GuiHandle>` 型の ≤1-surface 強制 + destroy 時 auto-close 相当の状態を設計・導入する時 | 未強制のままでは L0 GUI 実装時に `GuiHandle` orphan 可能

## Verification Notes

- [ ] 採否（defer）と判定基準 4 項目が本書に記録され、reviewer の approve を得た
- [ ] `Err(HostError::Unsupported)` の現行意味（host-side short-circuit・wire 非送信）が本判定で変化していない（`l1/mod.rs` 参照・コード変更なし）
- [ ] `gui_open` が #123 で除去済みである事実・wire 語彙の keep-for-future 維持・将来 GUI 実装時に独自の設計済み状態（`Option<GuiHandle>` 型の ≤1-surface 強制等）を導入する指針が本書に記録され、#123 spec（`docs/specs/issue-76-followup-l1-internal-cleanup.md`）/ IR（`docs/impl/ir/issue-123-l1-internal-cleanup-implementation.md`）と整合している
- [ ] wire 語彙存置・`#[non_exhaustive]` 圧力なし・capability 不触が明記されている
- [ ] doc 監査 findings 0
