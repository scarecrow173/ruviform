---
id: "ADR-034T6RXL42NlmST9mgHHgZ"
type: "adr"
status: "proposed"
title: "Driver-neutral GUIアーキテクチャ — GPUI標準Driver方針（仮決定）とWebView-first後継（Issue #384）"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/384"
    - "https://github.com/scarecrow173/ruviform/issues/257"
  changes:
    added:
      - type: "file"
        path: "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
      - type: "file"
        path: "docs/designs/issue-384-driver-neutral-gui-contract.md"
      - type: "file"
        path: "docs/specs/issue-384-driver-neutral-gui-architecture.md"
    modified:
      - type: "file"
        path: "docs/adr/ui-architecture-webview-hybrid.md"
      - type: "file"
        path: "docs/adr/ui-tech-selection-poc.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/issue-384-driver-neutral-gui-architecture.md"
  implemented-by:
    - "../designs/issue-384-driver-neutral-gui-contract.md"
  depends-on:
    - "realtime-contract-and-verification-policy.md"
  blocks: []
  supersedes:
    - "ui-architecture-webview-hybrid.md"
    - "ui-tech-selection-poc.md"
  superseded-by: []
  related:
    - "plugin-architecture-clap-vst3.md"
    - "plugin-gui-embedding.md"
    - "issue-181-multi-window-contract-placement-close-decision-ipc-routing.md"
    - "issue-182-dock-tree-placement-move-contract-and-persistence.md"
    - "../designs/ruviform-architecture-design-baseline.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/384"
    - "https://github.com/scarecrow173/ruviform/issues/257"
    - "https://github.com/scarecrow173/ruviform/issues/385"
    - "../specs/issue-384-driver-neutral-gui-architecture.md"
  derived-by:
    - "../designs/issue-384-driver-neutral-gui-contract.md"
    - "../plans/rf384-rf385-driver-neutral-gui-contract-ruviform-ui-theme-dtcg-interpreter-issue.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/ruviform/issues/386"
    - "https://github.com/scarecrow173/ruviform/issues/387"
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/390"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/synvelis/issues/175"
    - "https://github.com/scarecrow173/synvelis/issues/176"
    - "https://github.com/scarecrow173/synvelis/issues/177"
    - "https://github.com/scarecrow173/audio-lens/issues/166"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Driver-neutral GUIアーキテクチャ — GPUI標準Driver方針（仮決定）とWebView-first後継（Issue #384）

## Context and Problem Statement

Ruviform の GUI 技術方針は 2026-10 の Issue #1 / #28 で **WebView ファースト・ハイブリッド**（[ui-architecture-webview-hybrid.md](ui-architecture-webview-hybrid.md)）と **wry+tao / Solid / WebGL2 実測採用**（[ui-tech-selection-poc.md](ui-tech-selection-poc.md)）として確定し、`crates/ruviform-ui`・`crates/ruviform-ui-webview`・`ui/*` TypeScript パッケージの headless 実装まで進んだ。2026-11-07 の 3repo 横断レビュー（Issue #384・#257・#385 冒頭の確定仕様 v2）で、GUI 方針は **Design First + GPUI Native First + DTCG Design Contract + Native Component Lab** への移行が合意された。

この移行で問題となるのは、現行 `ruviform-ui` の公開契約が WebView の実装前提（JS eval・URL navigation・DOM/Canvas/WebGL の RenderTier）を内部に抱えている点である。GPUI を標準 Driver として採用するにあたり、単に JS を GPUI 呼出しへ置換すると、今度は `gpui::Window/Entity/Context/Hsla/Pixels` が共通契約へ逆流し、Consumer（Synvelis・AudioLens）の通常画面と Domain まで GPUI に結合する。両 Consumer を Driver 交換に耐える構造へ移すため、**GUI の構造的正本を Driver-neutral な契約層へ再配置する決定**が必要になった。

旧 2 ADR はいずれも `accepted` のまま現行コードの採用方針として矛盾するため、明示的な supersede と履歴保存を行い、後継の技術採用判断を実機 PoC 証拠へ接続する正本をここに置く。

## Decision Drivers

* Driver-neutral な共通契約（Design Contract/ResolvedTheme・Component & View Specs・Headless Behavior・Window/Dock/NativeSurface/Plugin Editor Session）を Ruviform が所有し、GPUI はその標準 Driver 実装に限定する（Issue #384 確定仕様 v2 §2・追加合意）。
* 外部プラグイン（VST3/CLAP/AU）の**実ネイティブ GUI を GPUI メイン Dock へ収容する能力**を、初期設計の必須要件・GPUI 採用 Gate とする（Issue #384・#388）。Floating 位置合わせ・Generic Editor は代替であり PASS としない。
* 旧 WebView/TS/SolidJS/Canvas/WebGL の実装成果と headless 契約は履歴として保持し、新 Native 経路が実証されるまで破壊的削除をしない（Issue #384・#257）。
* Consumer の通常 View/Domain/Headless の公開型・Cargo 直接依存に GPUI/DOM/wry/Toolkit/OS handle を持ち込まない。例外は最上位 composition root と隔離された driver-specific extension のみ（Issue #384・#386・#390）。
* GPUI 本体のみを標準 Driver の基本 GUI 依存とし、gpui-base/gpui-component/gpui-kit 等の第三者 Toolkit は Driver 内部への選択的・記録付き導入に限る（Issue #384 最小依存方針）。
* 「契約設計の開始」と「GPUI の技術採用最終承認」を分離する。最終承認は #262（OS/format/host 方式別 Pass/Fail/Unsupported/NotRun）・#386 最小 Host fixture・#388/#389 Native Dock PoC の実機証拠後とし、本 ADR を全 PoC 完成の着手条件にしない（Issue #384 §5）。
* WASM は対象外。

## Considered Options

* **Option A — Driver-neutral 契約層 + GPUI 標準 Driver（本 ADR の選択）**: 共通正本を backend-neutral な型・意味論契約に置き、GPUI・旧 WebView・Fake Driver はその消費者/実装者として並立する。
* **Option B — GPUI 直接採用（契約層を設けない）**: `ruviform-ui` の公開 API を GPUI 型へ直接書き換え、Consumer 画面も GPUI を直参照する。実装は早いが Driver 交換不能・#384 の必須依存方向に違反する。
* **Option C — 旧 WebView-first 維持 + GPUI を別経路で併存**: 2 系統を「同等の本番経路」として長期併存。共通契約の第二正本化・検証コスト倍増・移行終端の不定化を招くため不採用。
* **Option D — 万能 GUI 抽象フレームワークの新設**: 描画全体を抽象化する独自フレームワーク。範囲が無制限に膨らみ、#384 が明示禁止するため不採用。

## Decision Outcome

Chosen option: "**Option A — Driver-neutral 契約層 + GPUI 標準 Driver**"。Ruviform の GUI 構造正本は Driver-neutral 契約であり、GPUI は現時点で選択する標準 Driver 実装に限る。将来の Driver 差替えは原則として Consumer の View/Theme/Behavior 契約の変更なしに行える構造とする（全機能・外観の完全一致や実行時 hot swap は保証しない。Driver 選択は初期はビルド時または起動時で十分）。

### 採用する構造（依存方向）

```text
Synvelis App Shell / AudioLens GUI（Consumer 通常画面・ViewModel・Domain・Headless）
   │  参照: Driver-neutral 型付き契約のみ（GPUI/DOM/wry/OS handle 非依存）
   ▼
Ruviform Driver-neutral 共通契約層（本 ADR の正本）
   ├─ Design Contract → typed ResolvedTheme / Asset / Token Registry   [Issue #385]
   ├─ Component & View Specs + Headless Behavior（Gesture/Receipt/Gen） [Issue #387]
   ├─ Window / Dock / Focus / Input semantic contract + Capability      [Issue #386]
   ├─ NativeSurfaceSlot（SurfaceSlotId・矩形・clip・focus・lifecycle）  [Issue #388]
   └─ PluginEditorSession（L0/L1・Attach/Detach・ParentSlotLease）      [Issue #389]
   ▲                     ▲                              ▲
   │ consume             │ consume                      │ consume
ruviform-ui-gpui     ruviform-ui-webview            Fake Driver
（標準 Driver。       （旧 WebView adapter。         （契約テスト用。
 GPUI 本体のみ基本    移行期間は保持・新規           #390 が所有）
 依存・Toolkit は     本番採用ではない）
 内部限定・記録付き）
   ▲
   │ 選択・組立てのみ許可（推移依存の存在は許容・API 漏出は禁止）
Composition Root（Consumer の app 起動部）+ 明示的 driver-specific extension crate
```

ルール:

1. 共通契約層は **GPUI / DOM / wry / OS native handle / upstream toolkit に依存しない**。`gpui::Entity/Window/Context/AnyElement/Hsla/Pixels/Style`、`HWND/NSView/X11`、JS `eval`/URL を共通公開 API に露出しない。
2. Driver は共通契約の消費者・実装者であり、Driver 固有の Window/Entity/GPU 状態・OS handle は Driver クレート内部に閉じ込める。
3. Consumer 通常画面・View Model・業務 Domain・Headless テストは `gpui` 他の Toolkit を直接参照しない。例外は **(a) app 起動時の composition root**（Driver 選択・組立て）と **(b) 隔離された明示的 driver-specific extension crate** のみ。例外は公開 API へ漏らさず、代替/Capability・Unsupported を明示する。
4. GPUI 本体のみを標準 Driver の基本 GUI 依存とする。gpui-base/gpui-component/gpui-kit 等は「GPUI 本体だけでは実装・保守が合理的に難しい」実需が記録された場合のみ、**当該 Driver クレートの内部依存**として選択導入し、対象機能・推移依存・version/MSRV/OS・license・patch/fork 維持費・性能/サイズ・fixture 影響を記録する。共通契約の Theme/State/Component 正本を第三者 Toolkit へ委譲しない。
5. 万能描画抽象・巨大 GUI フレームワーク・完全汎用 GUI DSL は新設しない。共通 API は両 Consumer に必要な意味的構成と現実的な拡張点に限定する。

### 旧 WebView API → 新共通契約の移行表（正本マッピング）

| 現行 API（`crates/ruviform-ui` 他） | WebView 前提 | 移行先（新共通契約 / WebView adapter 内部） | Owner |
| --- | --- | --- | --- |
| `WindowConfig::url` / `devtools` | `rvf://app` URL・devtools | `WindowSpec { title, logical_size, position, scale_hint?, ... }`（JS URL なし）。URL/devtools は `ruviform-ui-webview` の `WebViewWindowConfig` へ局所化 | #386 |
| `HostHandle::eval` / `WindowHandle::eval` | JS 評価 | 共通契約は typed **Action/Event Port**（`ViewAction`/Window lifecycle）。eval は WebView adapter 専用 port へ隔離 | #386 |
| `HostEvent::Ready { url }` | navigation URL | 共通 `WindowEvent::Ready`（URL フィールドなし）＋ navigation 詳細は WebView adapter 内部 | #386 |
| `HostEvent::Message(String)`（raw IPC） | `window.ipc.postMessage` | 共通 typed Intent/Action。`rvf.ipc.v1` JSON スキーマ（`Intent`/`Event`/`ErrorBody`/`Envelope`/`codes`/`parse_intent`/`SCHEMA_VERSION`）は WebView adapter の decode 層として保持（削除しない）。共通契約側の `Service::call`/`UiRuntime` が返す typed 型は共通 API 面に残るため、WebView 前提の記述・payload 名は個別棚卸し | #386 |
| `RenderTier::{Dom, Canvas2d, Webgl2}` | WebView 内描画方式 | **共通契約から除去**。Driver 内部の描画詳細として WebView adapter に限定（Native Driver は RenderTier を持たない） | #386 |
| `RenderTier::NativeSurface` + `NativeSurfaceHost`/`SurfaceRegion`/`SurfaceId` | WebView 隣接 surface 前提 | `NativeSurfaceSlot`（`SurfaceSlotId`・論理所有・矩形/clip/z-order/focus/input/IME/resize/capability/error/rollback）。OS handle は Driver/OS adapter 内部 | #388 |
| `bridge::dispatch_script` / `bridge::BRIDGE_JS` | JS 文字列生成・注入スクリプト定数 | WebView adapter 内部の実装詳細（共通契約へ露出しない） | #386 |
| `GuiHandle` / `Plugin::open_gui`/`close_gui` / `wire::Command::OpenGui`\|`CloseGui`（`ruviform-plugin-core`/`plugin-host`/`plugin-export`） | raw native handle・L1 `HostError::Unsupported` 現状 | `PluginEditorSession`（`PluginEditorSessionId`・状態機械・Result/Timeout/Unsupported・exactly-once 終端）。L1 wire の GUI 命令は #389 が ParentSlotLease と併せて再設計 | #389 |
| `ui/*` TS パッケージ（core/components/viz/binding/guitest/showcase/browser-test） | DOM/Custom Elements/Canvas | 旧 WebView 実装資産として保持。headless 契約（Gesture/Receipt/binding semantics）は Rust 側共通契約へ移植し fixture で等価検証。縮退順序は後述 | #387/#390 |
| `WindowId` / `ViewInstanceId` / `DockTree` / `DockWorkspace`（2-phase move）/ `MoveToken` / `LayoutSnapshot` | backend-neutral 済み | **そのまま共通契約の正本として維持**（Issue #257 v2 の再利用確定） | #386 |
| `DataRegistry` / `AssetSource` / `Service`/`Job`/`UiRuntime`（Intent/Event 非依存部分） | backend-neutral 済み | 共通契約の一部として維持。WebView 依存記述のある doc/API 面は個別に棚卸し | #386 |
| `GuiParentToken(u64)`（`ruviform-plugin-core`） | raw native handle | `ParentSlotLease` 相当の opaque・generation-bound・lifetime-checked token。raw handle は OS adapter 内部 | #389 |

移行原則: 単純な GPUI 型への置換は禁止。旧挙動の接続試験（Fake Driver fixture）は移行前後で失わない。型名は本表が契約上の方向を示すもので、最終的な Rust シグネチャは各 Owner Issue の実装 PR で確定する。

### DriverCapability Matrix（宣言テンプレート）

Capability は **declared（契約上の宣言）/ Pass / Fail / Unsupported / NotRun / Blocked** を分離して記録する。本 ADR 時点の各行はすべて *declared* であり、実機証拠は #388/#262/#390 の結果で埋める（実証済みと記さない）。

| Capability | GPUI-Win | GPUI-macOS | GPUI-Linux(X11) | GPUI-Linux(Wayland) | WebView(legacy) | Fake |
| --- | --- | --- | --- | --- | --- | --- |
| Window create/destroy・multi-window | declared | declared | declared | declared | Pass(実装済) | Pass |
| Focus/keyboard/IME・text intent | declared | declared | declared | declared | partial | Pass |
| HiDPI/scale factor・multi-monitor | declared | declared | declared | declared | partial | Pass |
| Dock（tabs/split/2-phase move/rollback） | declared | declared | declared | declared | declared | Pass |
| **NativeSurfaceSlot: plugin native GUI dock** | PoC gate | PoC gate | PoC gate | PoC gate | N/A(WebView 経路は非対象) | 契約 fixture のみ |
| ResolvedTheme mapping（色空間/単位/フォント/DPI） | declared | declared | declared | declared | N/A | declared |
| Component/View render・input normalization | declared | declared | declared | declared | N/A | declared |
| Offscreen/screenshot（Golden 用） | declared | declared | declared | declared | declared | Pass |
| a11y intent・OS accessibility bridge | declared | declared | declared | declared | declared | declared |

注: Fake 列の `Pass` は既存 `ruviform-ui-test::fake::FakeHost` が headless テスト基盤として実測で提供する window/event/monitor/inject 等の契約を指し、描画・テーマ解決・実 OS 能力を意味しない。Fake の ResolvedTheme/Component render 行は意図的に `declared`（未実装）のままとする。

### Native Dock 採用 Gate（本 ADR の技術採否条件）

* **PASS 定義**: GPUI メイン Dock の実 layout/clip/tab/resize/input/focus 管理下で、**Plugin 自身の実 nativeEditor** が動作すること。Floating 位置合わせ・Generic Editor・fake/static screenshot/remote screenshot のみでは未達。
* **マトリクス（行を省略しない・#388 が所有）**: Windows VST3/CLAP × L0/L1、macOS VST3/CLAP/AU × L0/L1、Linux X11/XWayland VST3/CLAP × L0/L1、Wayland native VST3/CLAP × capability。各行 Pass/Fail/Unsupported/NotRun＋根拠。未試験は Pass に繰上げない。
* **L1**: 現行実装は `HostError::Unsupported`（#389 が実測）。新設計で L1 を実装可能かは PoC Gate で診断し、L1 を黙って L0 へ退化させない。困難な組合せは失敗要因と安全 fallback を記録する。
* **判定フロー**: #388/#389 の matrix と証拠 → #262 が採否判定（OS/format/capability 範囲の限定採用を含む）→ 判定結果を本 ADR の Confirmation 更新として逆反映し、Synvelis #176 が製品 E2E で確認する。未達の能力は機能・OS サポート境界と次の方針を本 ADR に追記して意思決定する（無条件 PASS 宣言をしない）。

### 旧 Frontend（WebView/TS）の縮退順序

1. 新 Driver-neutral 契約 + GPUI Driver の契約 fixture が green（Fake/GPUI 同一 semantic fixture、#390）。
2. Synvelis（Fader+Tabs+Dock）/ AudioLens（Waveform+Seek）の最小 Slice が GPUI Driver 経由で描画・操作できる（#386/#387 実装）。
3. 実 OS 検証 + Native Dock Gate（#388）+ 両 Consumer 非回帰 Gate（#219/#294）が合格。
4. 上記完了後に限り、旧 WebView API の削除・移動と `ui/*` TS 資産の縮退を**別の明示判断（別 PR）**として実施する。それまでは `ruviform-ui-webview`・`ui/*`・poc/ui の証拠を削除しない。

### 責務マップ（#257 共通責務の整理 — 本 ADR が正本として固定）

| 契約領域 | 正本（Owner Issue） | 内容 |
| --- | --- | --- |
| Design Contract / ResolvedTheme | #385 | DTCG 解釈・alias・theme/mode・override・typed backend-neutral ResolvedTheme・Theme Registry |
| Component & View Specs | #387 | `ViewSpec`/`ComponentSpec`・Props/Variant/State・Composition・GPUI Renderer への射影 |
| Headless Behavior | #387（操作正本は #257 既存合意） | Gesture Begin/Preview/Commit/Cancel・Selection/Zoom・Value Mapping・Receipt・revision/generation/stale |
| Window/Dock/Host | #386 | `WindowSpec`/`WindowId`/`ViewInstanceId`/`DockWorkspace`・Focus/Input/Close veto・Capability negotiation・GPUI Host adapter |
| NativeSurfaceSlot | #388 | `SurfaceSlotId`・矩形/clip/focus/lifecycle・OS/Plugin adapter 境界・Native Dock 実証 |
| PluginEditorSession | #389 | `PluginEditorSessionId`/`ParentSlotLease`・L0/L1・state machine・recovery |
| Contract fixture / Fake Driver / Lab | #390 | DriverContractFixture・Fake Driver・Native Component Lab・依存境界 guard |
| Cross-repo CI / Gate 集約 | #294（workflow owner #294、境界/Fake/parity owner #390） | 3repo SHA tuple・Daily Full + manual/real OS |
| 共通 UI 横断設計正本 | #257 | 本表を含む共通 UI 責務の上位正本（技術選定/移行方針は本 ADR=#384） |

### 3repo 境界（非 GPUI ルール）

* Ruviform 共通クレート（contract/headless/theme/view-spec/dock/plugin-session）: `gpui` 系クレートへの Cargo 依存・公開型露出を禁止。依存境界 guard は #390 の cargo metadata + source import 検査で機械化する。
* Synvelis / AudioLens の通常 View/Domain/Headless: `gpui`/Toolkit/OS handle を import しない。`gpui` への依存はアプリ起動部の composition root と明示 extension に限定される。
* 例外を許可するのは Driver crate・composition root・明示 extension の allowlist のみであり、実行バイナリの推移依存そのものは排除対象としない（#386/#390 の解釈に従う）。

### Consequences

* Good, because: Consumer の View/Theme/Behavior 契約を Driver 交換から保護できる。GPUI 採否を実機証拠（#388/#262）に接続しつつ、契約設計と実装を並行開始できる。旧 WebView 実装の headless 契約・PoC 証拠を履歴として残し、後退経路を失わない。Toolkit 最小依存により推移依存・license/保守リスクを抑える。
* Bad, because: 共通契約の設計コストと移行期間の 2 系統保守が増える。`ruviform-ui` の WebView 前提 API の棚卸しと分離で、既存 headless 実装に移行作業が発生する。Native Dock の実現性は未証明であり、Gate 未達時は GPUI の限定採用・フォールバック方針の再判断が必要になる。

### Confirmation

* 本 ADR の「契約設計開始」は、独立レビュー（review-protocol）で blocker が残っていないこと、および本節の依存方向・移行表・Gate が #385〜#390 の各 Owner Issue と整合することをもって確認する。
* 「GPUI 技術採用の最終承認」は本 ADR では行わない。#262 が #386 最小 Host fixture・#388/#389 Native Dock PoC・#390 境界/parity fixture の OS/format 別証拠を以て採否判定し、その結果（採用範囲・制限・代替方針）を本 ADR の More Information へ逆反映する。**この逆反映まで、本 ADR は GPUI の無条件採用を宣言しない。**
* 機械的確認: #390 の依存境界 guard（cargo metadata + source import）が共通契約層と Consumer 通常画面からの GPUI 漏出を検出すること。#385 が GPUI なしで build/test できること。

## Implementation Plan

* 影響するパス:
  * `docs/adr/` — 本 ADR の新設、旧 2 ADR の supersession 更新、`docs/adr/README.md` index 再生成
  * `docs/designs/issue-384-driver-neutral-gui-contract.md` — 契約詳細（ID 体系・state transition・fixture 対応）
  * `crates/ruviform-ui` — WebView 前提 API の棚卸しと共通契約/adapter 分離（#386 が実装。本 ADR は方向のみ）
  * `crates/ruviform-ui-theme`（新設、#385）・`crates/ruviform-ui-gpui`（暫定名、#386）・Native Slot/Session（#388/#389）
  * `ui/*` TS パッケージ — 変更なし（縮退順序 4 まで保持）
* 従うパターン: `ruviform-audio-device` の adapter パターン（cpal を内部依存に閉じ込め `Box<dyn>` port を実装）を Driver 設計の先例とする。`WindowId`/`ViewInstanceId`/`DockWorkspace` の既存 backend-neutral 契約は再設計せず再利用する。
* 避けるパターン: GPUI 型の共通 API 露出、JS 前提を GPUI イベントへ直接置換、Toolkit への Theme/State 正本委譲、旧 API の新経路未証明での削除、万能描画抽象。
* 依存関係: #385（Design Contract）→ #387（Component/View）← #386（Host）。#388/#389 は契約設計を #386 と並行。Acceptance: #262・#388・Synvelis #176・#294。

## Pros and Cons of the Options

### Option A — Driver-neutral 契約層 + GPUI 標準 Driver（採用）

* Good, because: Driver 交換が Consumer 契約変更なしに可能。GPUI 採否を段階証拠に接続できる。旧実装資産を保持したまま移行できる。Toolkit 依存を最小化できる。
* Bad, because: 契約設計・維持コスト、移行期間の 2 系統保守、抽象化の設計難度。契約が過剰抽象になるリスク（ルール 5 で抑制）。

### Option B — GPUI 直接採用

* Good, because: 初期実装が最速。抽象化コスト不要。
* Bad, because: Consumer 画面・Domain が GPUI に結合し Driver 交換不能。#384 の必須依存方向と最小 Slice 受入に違反。Toolkit 選択の柔軟性を失う。

### Option C — 旧 WebView-first 維持 + GPUI 併存

* Good, because: 既存資産の即時再利用。
* Bad, because: 2 系統の共通契約第二正本化、検証コスト倍増、移行終端が不定化し旧経路が半恒久的に残る。

### Option D — 万能 GUI 抽象フレームワーク

* Good, because: 理論上の最大柔軟性。
* Bad, because: スコープ無制限・実装不可能な完全抽象を要求し #384 が明示禁止。YAGNI 違反。

## Verification

* [ ] 本 ADR と旧 2 ADR の supersedes/superseded-by リンクが双方に明示され、旧 PoC 証拠が削除されていない
* [ ] 依存方向図・移行表・Capability Matrix・Native Dock Gate・縮退順序・責務マップ・3repo 境界が #257/#385〜#390 の Issue 記述と整合する
* [ ] 共通契約層が GPUI/Toolkit/DOM/wry 依存なしで設計記述され、`gpui::` 型・OS handle・JS eval/URL を公開 API ルールで禁止している
* [ ] GPUI の「契約設計開始」と「技術採用最終承認」が分離され、最終承認が #262/#388/#389 実機証拠後であること
* [ ] Native Dock PASS 定義に Floating/Generic/fake/screenshot の代替を含めず、L1 の Unsupported 現状が記録されている
* [ ] 旧 Frontend の縮退順序に「新経路実証前の削除なし」が含まれる

## More Information

* 一次ソース: GitHub Issue [#384](https://github.com/scarecrow173/ruviform/issues/384)（2026-11-07 横断レビュー確定仕様 v2 が旧記述・旧 ADR に優先）、[#257](https://github.com/scarecrow173/ruviform/issues/257)（共通 UI 正本）、[#385](https://github.com/scarecrow173/ruviform/issues/385)（Token 解釈）、[#262](https://github.com/scarecrow173/ruviform/issues/262)（GPUI 採否 Gate）。
* 契約詳細設計: [docs/designs/issue-384-driver-neutral-gui-contract.md](../designs/issue-384-driver-neutral-gui-contract.md)。
* 旧 ADR（superseded、証拠保持）: [ui-architecture-webview-hybrid.md](ui-architecture-webview-hybrid.md)、[ui-tech-selection-poc.md](ui-tech-selection-poc.md)。Synvelis 側の旧 SolidJS 採用 ADR は [Synvelis #175](https://github.com/scarecrow173/synvelis/issues/175) で後継化（別 repo/別 Owner）。Synvelis の製品 Theme・Workspace・GPUI Design Review は [Synvelis #177](https://github.com/scarecrow173/synvelis/issues/177)、AudioLens の Consumer 受入は [AudioLens #166](https://github.com/scarecrow173/audio-lens/issues/166) が所有する。
* 関連 ADR: [プラグインアーキテクチャ](plugin-architecture-clap-vst3.md)、[plugin-gui-embedding](plugin-gui-embedding.md)（旧方針の Plugin GUI 節は本 ADR の NativeSurfaceSlot/Session 契約に読替え）、[issue-181](issue-181-multi-window-contract-placement-close-decision-ipc-routing.md) / [issue-182](issue-182-dock-tree-placement-move-contract-and-persistence.md)（維持する論理契約）。
* **GPUI 最終採用判定の逆反映欄（未記入）**: #262 verdict が確定したら、採用範囲・OS/format 制限・代替方針をここに追記する。現在は判定前であり、GPUI は「標準 Driver 候補として契約設計を開始する仮決定」の位置づけ。
