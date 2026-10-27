---
id: "DESIGN-034N3J1C5vystrbJl69R1q"
type: "design"
status: "draft"
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

## 決定サマリ（Issue スコープ → 本書の結論）

| 判断 | 結論 |
| --- | --- |
| 現行 `Unsupported` 拒否を維持するか実装するか | **defer — `Err(HostError::Unsupported)` 維持を確定**（host-side short-circuit のまま。解除条件は「境界の pin」節） |
| `OpenGui`/`CloseGui` wire 動詞・`Reply::Opened`・`GuiHandle`/`GuiParentToken` | **keep-for-future** — L0 コマンド集合との parity と L0 GUI 将来実装経路のため存置。除去は `WIRE_VERSION` bump を要する破壊的変更で利益なし |
| `gui_open` write-only field（`host.rs` Instance 簿記）の使途 | **keep-for-future** — #123 の sub-item は「フィールド除去」でなく「double-`OpenGui` の `GuiHandle` orphan ゲート」を実装対象とする（理由は「境界の pin」節） |
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
- ただし: (a) wry の Linux 子 attach は `WebViewBuilderExtUnix::build_gtk` — **同一プロセス内の `gtk::Container`** を要求し（#57 Linux 検証は親 `gtk::Fixed`・in-process）、`GtkSocket` ベースの越境 attach 経路は wry に存在しない。(b) `GtkSocket`/`GtkPlug` は GTK3 系機構で GTK4 で除去済み — wry が依存する WebKitGTK 4.1 / GTK 系の主流では越境経路の土台が消えている。(c) **Wayland は越境埋め込みを設計上提供しない** — `xdg-foreign` は toplevel 間の transient-parent hint のみで、クライアント間サーフェス分離がプロトコル原則
- 判定: X11 セッション限定なら XEmbed 理論上成立。現行 wry/GTK4/Wayland 経路では**実質不可**

### macOS — `NSView` 越境

- `NSView` 階層はプロセス内のオブジェクトグラフであり、別プロセスの view を子にする公認 API は**存在しない**。VST3 type `"NSView"`・CLAP `clap_window` cocoa は同一プロセス内ポインタ受け渡しを前提とする
- 代替としての別プロセス `NSWindow` child 化・`IOSurface`/layer 越境合成は「埋め込み」ではなく合成経路であり、hit-testing・resize・lifecycle を全て自前で再実装する領域 — プラグイン GUI 契約の単純な写像として成立しない
- 判定: **不可**（公認機構なし）

### 調査まとめ

| OS | 越境機構 | wry/現行経路との整合 | 判定 |
| --- | --- | --- | --- |
| Windows | `SetParent`/`WS_CHILD`（入力キュー結合付き） | `build_as_child` の越境指定は未検証・WebView2 foreign-parent は undocumented | 条件付き（脆弱） |
| X11 | XEmbed（`GtkSocket`/`GtkPlug`） | `build_gtk` は in-process コンテナ前提・GTK4 で機構消滅 | 旧式機構のみ |
| Wayland | なし（`xdg-foreign` は toplevel hint のみ） | — | 不可 |
| macOS | なし（`NSView` 越境 embed 公認 API なし） | — | 不可 |

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
- **(c) per-OS 実現可能性の再評価** — Windows: 越境 `SetParent` + WebView2 foreign-parent の実機検証 / Linux: X11 セッション限定 XEmbed の受容 or Wayland 新機構の出現 / macOS: 公認の越境 view 機構の出現。少なくとも 1 OS で robust な経路が立つことが最低条件
- **(d) 需要の顕在化** — GUI 必須かつクラッシュ隔離が同時に要求されるプラグイン利用形態の出現（L0 で代替不能なケース）

### `gui_open` field の fate（Issue #123 sub-item への確定回答）

**keep-for-future。** `host.rs` `Instance::gui_open` は `OpenGui` 成功時 `true`・`CloseGui` 成功時 `false` に書かれる host 側簿記で、「1 インスタンスにつき同時に開く GUI surface は高々 1 つ」という不変条件の保持材料である。本書の defer 判定は L1 で動詞が成功しないことを意味するが、**L0（in-process）で `open_gui` を実装するプラグインが現れた瞬間にこの不変条件は必要になる**。現状 write-only なのは読み手となる gate が未導入なだけであり、#123 の sub-item は「フィールド除去」ではなく **double-`OpenGui` に対する orphan ゲート**（`gui_open == true` での再 `OpenGui` を defined rejection — `GuiHandle` は Drop-less トークンのため二度目の `Opened` は先の handle を leak させる）を実装対象とするのが正しい解である。フィールド除去は L0 GUI 実装時に同一簿記を再導入するコストを生み、guard 対象の不変条件自体は backend 非依存で残るため不採用。

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
- `gui_open` double-open ゲート | #123 sub-item として既にトリアゲ済み・本書は keep+gate の結論のみ供給 | #123 実施時 | 未ゲートのままでは L0 GUI 実装時に `GuiHandle` orphan 可能

## Verification Notes

- [ ] 採否（defer）と判定基準 4 項目が本書に記録され、reviewer の approve を得た
- [ ] `Err(HostError::Unsupported)` の現行意味（host-side short-circuit・wire 非送信）が本判定で変化していない（`l1/mod.rs` 参照・コード変更なし）
- [ ] `gui_open` keep-for-future + orphan ゲート方針が #123 sub-item へ本書経由で引き渡されている（`references`/`deferred-by` 関係）
- [ ] wire 語彙存置・`#[non_exhaustive]` 圧力なし・capability 不触が明記されている
- [ ] doc 監査 findings 0
