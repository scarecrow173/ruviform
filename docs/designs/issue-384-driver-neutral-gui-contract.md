---
id: "DESIGN-034T6S9KVWbAOq8lqyMak4"
type: "design"
status: "approved"
title: "Driver-neutral GUI Contract 詳細設計 — 依存方向・API移行表・DriverCapability・NativeDock Gate（Issue #384）"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/issue-384-driver-neutral-gui-architecture.md"
    - "https://github.com/scarecrow173/ruviform/issues/384"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-384-driver-neutral-gui-architecture.md"
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-384-driver-neutral-gui-architecture.md"
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/designs/issue-385-dtcg-resolved-theme.md"
    - "docs/designs/issue-181-multi-window.md"
    - "docs/designs/issue-182-dock-tree.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-384-driver-neutral-gui-architecture.md"
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/385"
    - "https://github.com/scarecrow173/ruviform/issues/386"
    - "https://github.com/scarecrow173/ruviform/issues/387"
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/390"
  defers: []
  deferred-by: []
---

# Driver-neutral GUI Contract 詳細設計 — 依存方向・API移行表・DriverCapability・NativeDock Gate（Issue #384）

## Context

[spec](../specs/issue-384-driver-neutral-gui-architecture.md) / [ADR: Driver-neutral GUIアーキテクチャ](../adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md) が確定した構造方針（共通契約層 + GPUI 標準 Driver 仮決定）の**契約詳細**を固定する。本書は ADR の実装者向け詳細であり、契約の決定そのものは ADR が正本。Issue #384/#257（2026-11-07 確定仕様 v2）が旧記述・旧 ADR に優先する。本設計は「契約設計開始」であり、GPUI の技術採用最終承認（#262 verdict + #388/#389 実機証拠）は別 phase。

## Scope

### In Scope

- 共通契約の **ID 体系・所有権・ライフサイクル**（Window/View/Component/SurfaceSlot/PluginSession）。
- 共通契約の **公開 API ルール**（持ち込み禁止型・例外 allowlist・Capability/Unsupported 表現）。
- 旧 `ruviform-ui` API の棚卸し移行表（ADR の正本マッピングの実装粒度版）。
- DriverCapability の宣言スキーマと fixture 対応（#390 の Case ID との対応付け）。
- 共通 design/token コードの配置（`ruviform-ui-theme` 新設）と crate 境界。

### Out of Scope

- GPUI Driver の Window/Event loop 実装（#386）、Component Renderer（#387）、Native Dock PoC（#388）、Plugin Session 実装（#389）、Fake Driver/Lab 実装（#390）。
- 旧 `ruviform-ui` API の実コード移動/削除（移行期間中は保持。縮退順序 4 の別判断）。
- GPUI version/MSRV/license の最終採否、Native Dock の実機結果。

## Components and Boundaries

### ID 体系（#257 v2 準拠・既存型再利用）

| ID | 所有 | 意味 | 備考 |
| --- | --- | --- | --- |
| `WindowId` | 共通契約（現行 `ruviform-ui::window` 維持） | Window session の論理 ID | NonZeroU64、destroyed 後は再利用しない |
| `ViewInstanceId` | 共通契約（現行 `ruviform-ui::dock` 維持） | 論理 View（Dock 管理対象） | 2-phase move の対象 |
| `ComponentKey` | 共通契約（#387 で新設） | View 内の stable component key | View rebuild での状態対応付け |
| `SurfaceSlotId` | 共通契約（#388 で新設） | NativeSurface slot の寿命 ID | OS handle は持たない |
| `PluginEditorSessionId` | 共通契約（#389 で新設） | Plugin GUI session の世代付き ID | `PluginInstanceId`/`PluginHandle`（DSP instance）と混同しない |
| `ParentSlotLease`（仮） | 共通契約（#389） | GUI parent の opaque・generation-bound・lifetime-checked token | `GuiParentToken(u64)` の後継 |
| Gesture/Receipt/Generation | 既存 Headless 契約（#257/#261 系） | 操作・receipt・stale 判定 | Rust 側へ移植（TS `ui/binding` の意味論を正本化） |

ルール: 同等型の再定義禁止。新規 ID は上表の所有 Issue でのみ定義する。Plugin Instance / GUI Session / SurfaceSlot / Window / View の ID を相互に使い回さない。

### 公開 API ルール（3repo 共通）

- 共通契約の public signature に出せないもの: `gpui::*`（Entity/Window/Context/AnyElement/Hsla/Pixels/Style）、`wry`/`tao`/DOM 型、`HWND`/`NSView`/X11 Window、GPU handle、`JS eval`・Web URL、`String` での script。
- Consumer 通常画面・View Model・Domain・Headless test: 上記型と `gpui`/Toolkit crate への直接依存を持たない。
- 例外 allowlist（#390 の guard で検査）: (a) Driver crate 自身、(b) Consumer app の composition root、(c) 明示命名の driver-specific extension crate。例外でも共通契約の public API に漏出しない。
- Capability 表現: Driver は `DriverCapabilities`（宣言 struct/flags）を返し、未対応機能は呼出し時に `Unsupported` の typed error を返す。silent fallback・silent degrade を行わない。

### 契約配置（crate 境界）

| 配置 | 内容 | 依存 |
| --- | --- | --- |
| `ruviform-ui`（既存・段階移行） | backend-neutral 済みの論理契約（`WindowId`/`ViewInstanceId`/`DockWorkspace`/`DataRegistry`/`AssetSource`/service-job）、WebView 前提 API は #386 で adapter 側へ分離 | Primitives のみ（現行維持） |
| `ruviform-ui-theme`（**本設計で新設**、#385 が実装） | DTCG interpreter・alias/mode/override resolve・typed `ResolvedTheme`・Theme Registry | serde/serde_json のみ。`ruviform-ui`・GPUI・Toolkit 非依存 |
| `ruviform-ui-gpui`（暫定名、#386 が実装） | 標準 GPUI Driver adapter（Window/Event/Render/Input/Theme mapping・NativeSurfaceSlot 実装） | `ruviform-ui`/`ruviform-ui-theme` + `gpui`（基本依存）・Toolkit は内部任意 |
| `ruviform-ui-webview`（既存・保持） | 旧 WebView adapter（wry/tao）。共通契約の WebView 実装として段階的に適合 | `ruviform-ui` + wry/tao（feature `webview`） |
| Fake Driver（#390、crate 未確定・`ruviform-ui-test` 系または新設） | 契約 fixture 用の in-memory Driver | 共通契約のみ |
| Plugin session contract（#389、`ruviform-plugin-core`/`ruviform-plugin-host` 側に新契約） | `PluginEditorSessionId`/`ParentSlotLease`・L0/L1 capability | 共通契約・plugin 基盤のみ（GUI handle なし） |

- 依存方向は一方向: 共通契約層は Driver に依存しない。Driver→共通契約、Consumer→共通契約（+ composition root→Driver）。
- `ruviform-ui-theme` は host/window 契約を持たないため `ruviform-ui` へ依存しない（DTCG→ResolvedTheme の単独 build/test を保証）。Component/View spec が Theme を消費する場合は #387 側の契約で合成する。

### NativeSurfaceSlot 契約（#388 詳細の共通側骨格）

- `SurfaceSlotId`・論理所有（`WindowId`+`ViewInstanceId`）・矩形（論理座標 + DPI/scale 変換規約）・clip/visibility/z-order・focus/input/IME/resize/reattach/cleanup・capability・error・rollback。
- `HWND`/`NSView`/X11 Window/GPUI Entity/GPU handle は Driver・OS adapter 内部。slot 消滅・stale generation・2-phase move 失敗時は元の View/Plugin Instance/Project/RT 状態を保持する（#182 の rollback 契約を再利用）。
- Floating window・Generic Editor は `NativeDockOutcome` の別結果（明示 fallback）であり、Native Dock PASS ではない。

### PluginEditorSession 契約（#389 詳細の共通側骨格）

- 状態機械 `Closed → Creating → Attached(Docked|Floating) → Hidden|Detaching|Closing → Closed` + `Failed → Recovering|Closed`。request ID/generation・Result/Timeout/Unsupported を各遷移に持つ。終端イベントは exactly-once。
- `ParentSlotLease` は opaque・generation-bound・lifetime-checked。L1 cross-process で親 pointer/address を無検証に再利用しない。現行 `GuiParentToken(u64)`・`InProcessHost::open_gui`・L1 `OpenGui/CloseGui`（`HostError::Unsupported`）の棚卸し対象。
- GUI hang/crash は UI session を終了/復旧しても Project/RT/Graph/Plugin state を暗黙破棄しない。

## Data and Control Flow

```text
Consumer ViewSpec 構築（typed builder / 構造化 Rust 型）
  → ResolvedTheme（#385 の不変 snapshot）+ Component/View spec（#387）
  → Driver へ mount（Window/Dock 契約 #386）
  → Driver が input を正規化 → Headless Behavior（Gesture/Receipt/generation）
  → 共通契約経由で Consumer Command/Query/Binding へ（target/revision/stale は既存契約）
Native Slot attach 要求 → Driver capability 照合 → Supported: slot attach / Unsupported: typed error（silent fallback なし）
```

- stale 判定: 同一 generation の stale result は適用しない。Component 未対応・Native Dock 不足は `Unsupported` で通知し、Floating/Generic/空白へ黙って偽装しない。
- Theme 差分が業務数値・操作状態・Timing を変えないことは #385 の fixture で検証する。

## Non-Functional Constraints

- RT 契約: GUI/UI スレッドから RT スレッドへのブロッキング・allocation・OS GUI API 呼出し禁止（既存 realtime-contract ADR に従う）。GUI close/hang が DSP/Graph/Project/RT を暗黙停止しない。
- 決定性: Theme resolve・fixture oracle・2-phase move は deterministic。Fake Driver fixture と GPUI Driver fixture は同一 semantic oracle。
- 依存最小化: 標準 Driver は GPUI 本体のみ基本依存。Toolkit 導入は Driver 内部・記録付き。
- 検証分離: headless/Fake の green を実 OS/実 Plugin/Native Dock の Pass とみなさない。OS/GPU/GPUI revision/format/plugin binary/L0-L1/process 情報・証拠 path を結果記録に含める。

## Risks and Trade-offs

- **Native Dock 未達リスク（P0 候補）**: GPUI main dock への実 plugin GUI 埋め込みが OS/format 別に困難な場合、#262 verdict で限定採用・代替方針（Host-owned window 等）を決定し本 ADR/design へ逆反映。Gate 未達を PASS としない。
- **契約の過剰抽象化**: ルール「共通 API は両 Consumer の最小 Slice に必要な意味的構成に限定」で抑制。新規 DSL・万能抽象を追加しない。
- **2 系統移行期の drift**: 旧 WebView path は消さず、新契約 fixture で等価性を担保し、縮退順序 4 で明示判断するまで両系統の整合を fixture で監視する。
- **L1 GUI（cross-process native dock）の未実装**: 現行 `HostError::Unsupported` を起点に #389 が診断。設計時点で実装可能と仮定せず、困難な組合せは失敗要因+fallback として記録。

## Deferred Design Concerns

- `ViewSpec`/Component spec の最終シグネチャ・最小 node 集合 → #387（Synvelis Fader+Tabs+Dock / AudioLens Waveform+Seek の slice から決定）。
- `ParentSlotLease` の wire 表現・L1 IPC/negotiation → #389。
- DriverCapability matrix の各セル実値 → #388/#262/#390 の実機・fixture 結果で充填。
- `ruviform-ui` の残存 WebView API の削除/移動 → 縮退順序 4 の別判断。

## Verification Notes

- [ ] 依存方向図・ID 表・crate 配置が ADR と #385〜#390 の Issue 記述に整合する
- [ ] 公開 API ルールが `gpui`/Toolkit/OS handle/DOM/wry 漏出を禁止し、例外 allowlist が composition root + driver extension に限定される
- [ ] `ruviform-ui-theme` が `ruviform-ui`/GPUI 非依存で配置されている（#385 spec/design と整合）
- [ ] Native Dock PASS 定義・L1 Unsupported 現状・未検証を Pass としない記述が ADR と一致する
- [ ] 既存 `WindowId`/`ViewInstanceId`/`DockWorkspace`/`DataRegistry` を再利用対象として列挙し、WebView 前提 API が移行対象として全件棚卸しされている
