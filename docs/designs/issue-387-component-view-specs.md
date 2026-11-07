---
id: "DESIGN-034T6RWQr3Kn5Xp8bC7yF1"
type: "design"
status: "proposed"
title: "Issue #387 Component/View Specs・Headless Behavior・GPUI Renderer 設計"
created: "2026-11-07"
updated: "2026-11-07"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/387"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/issue-387-component-view-specs.md"
  implemented-by: []
  depends-on:
    - "../designs/issue-384-driver-neutral-gui-contract.md"
    - "../designs/issue-385-dtcg-resolved-theme.md"
    - "../designs/issue-386-driver-neutral-gui-host.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../discovery/issue-387-component-view-inventory.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/issue-387-component-view-specs.md"
  derived-by:
    - "../plans/implement-issue-387-component-renderer.md"
  verifies: []
  verified-by:
    - "../test-specs/issue-387-component-renderer.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #387 設計: Component/View Specs・Headless Behavior・GPUI Renderer

## 設計判断サマリ

| ID | 判断 | 理由 |
| --- | --- | --- |
| D1 | 契約は `ruviform-ui` の `view`/`behavior` 2 module に置く。新 crate は作らない | #384 契約層配置。crate 追加は共有 Cargo.toml/lock への侵入口を増やす |
| D2 | `ViewHost` を唯一の authoritative projection engine とし、fake/gpui の両 layer がこれを通す | reconcile/gesture 意味論を 1 実装に集約(DRY)。gpui renderer は spec→element の写像と入力 normalize のみ |
| D3 | TS canonical(`interaction.ts`/`value.ts`/`binding` contract)を Rust 型へ port。DSL 写経はしない | 正本意味論の同値が Issue AC。port は意味論のみ |
| D4 | viz data は解決済み `WaveformBin` 列を spec payload として保持。query/LOD/cancel は consumer 側(ruviform-viz) | 契約層に port trait を置かない。bounded data の projection のみが renderer 責務 |
| D5 | `ruviform-ui` は viz 非依存のまま。`WaveformBin { start,end,min,max,rms,count }` を契約層の payload 型として宣言(`ruviform-viz::ViewBin`/`LodBucket` と同一 field 意味)。viz→contract 変換は adapter/consumer 責務(viz README「結合は adapter 側の責務」と整合) | `ruviform-ui` の「Primitives のみ依存」宣言を維持。bin の知識(LOD 構築/bounded query/cancel)は引き続き viz に一元、契約層は resolved payload の形だけを規定 |
| D6 | `readOnly`/`disabled`/`constant` は `ValueDescriptor` の宣言 flag で availability 正本。`ComponentVisualState` は `error` のみ(visual+a11y、gesture 非 block) | canonical の descriptor 直持ちと一致。二重正本を作らない |
| D7 | spec 更新で descriptor 等値変更なし → committed/gesture 保持。descriptor 変更 → `unregister`(detach reason "unmount")+ `register`(TS `InteractionDispatcher` 意味論) | TS canonical は begin-while-active を reject するため「safe begin」は存在しない。rebuild は unregister 経路 |
| D8 | ThemeMap は canonical semantic token 名(AudioLens 語彙底本)を `ResolvedTheme::color()`/`length()` で引く。未定義 token は literal default fallback | 第二 Theme 正本を作らない。token 名は #387 contract の一部 |

## 型構成(`ruviform-ui`)

```text
view/   … ComponentKey, ComponentSpec, ViewSpec, A11ySpec,
          ComponentVisualState, ProjectionOp, ViewEvent,
          ViewError, ViewHost
behavior/… ValueDescriptor, ValueMapping, StepDomain,
          OutOfRangePolicy, ValueMapper, DisplayValue,
          ControlValue, ControlInput, InteractionEvent,
          InteractionControl, InteractionDispatcher,
          CommandIntent/Receipt/Status/Phase,
          StateProjection, CommandBinding
```

### view module

```rust
pub struct ComponentKey(String);          // View 内 stable key
pub enum ComponentSpec {                  // #[non_exhaustive]
    Panel(PanelSpec), Tabs(TabsSpec), Fader(ControlSpec),
    Seek(ControlSpec), Waveform(WaveformSpec),
}
pub struct ControlSpec {                  // Fader/Seek 共通
    pub key: ComponentKey, pub descriptor: ValueDescriptor,
    pub orientation: Orientation, pub visual: ComponentVisualState,
    pub a11y: A11ySpec, pub control_id: Option<ControlId>,
    pub detach_policy: DetachPolicy,
}
pub struct ComponentVisualState { pub error: Option<String> }
// readOnly/disabled/constant は descriptor 宣言 flag(availability 正本)
pub struct WaveformBin {               // ruviform-viz::ViewBin+LodBucket 意味
    pub start: u64, pub end: u64,
    pub min: f64, pub max: f64, pub rms: f64, pub count: u64,
}
pub struct WaveformSpec {
    pub key: ComponentKey, pub sample_rate: u32, pub channels: usize,
    pub frames: u64, pub viewport: Range<u64>,
    pub channel_bins: Vec<Vec<WaveformBin>>,  // ≤ MAX_WAVEFORM_BINS
    pub visual: ComponentVisualState, pub a11y: A11ySpec,
}
pub struct A11ySpec { pub role: Option<A11yRole>, pub label: Option<String>,
    pub orientation: Option<Orientation>,
    pub value_min: Option<f64>, pub value_max: Option<f64>,
    pub value_text: Option<String> }
pub enum A11yRole { Slider, TabList, Tab, Group, Img }  // non_exhaustive
pub struct ViewSpec { pub root: ComponentSpec }
```

### `ViewHost`

```rust
pub struct ViewHost { /* spec tree, dispatcher, binding, generation */ }
impl ViewHost {
    pub fn new(view: ViewInstanceId) -> Self;
    pub fn mount(&mut self, spec: ViewSpec) -> Vec<ProjectionOp>;
    pub fn update(&mut self, spec: ViewSpec)
        -> Result<Vec<ProjectionOp>, ViewError>;
    pub fn unmount(&mut self) -> Vec<ProjectionOp>;
    pub fn reattach(&mut self) -> u64;        // generation++
    pub fn dispatch_input(&mut self, key: &ComponentKey,
        input: ControlInput) -> Vec<InteractionEvent>;
    pub fn observe(&self, key: &ComponentKey)
        -> Option<ComponentSnapshot>;         // display/phase/committed
    pub fn drain_events(&mut self) -> Vec<ViewEvent>;
    pub fn attach_sink(&mut self, sink: Box<dyn FnMut(CommandIntent)>);
}
```

- reconcile: 前回 spec tree を key map 化し flatten 比較。
  新規 key → `Mounted`、同一 key で props 差分 → `Updated`、
  消えた key → `Removed`(active gesture は safe cancel)、
  `unmount` → 全 key `Unmounted` + `detach_all`。
- generation: `mount`/`reattach` で採番した generation を
  `dispatch_input`/`update` が持ち回る — 呼び出し側が stale
  generation を渡した場合 `ViewError::StaleGeneration`。
- `ViewEvent`: `Mounted|Updated|Removed|Unmounted`(op echo)、
  `Gesture { key, event }`(InteractionEvent echo)、
  `Intent(CommandIntent)`。drain して consumer/test が検証。

### behavior module(TS port の対応表)

| TS canonical | Rust |
| --- | --- |
| `ValueDescriptor` | 同名 struct(`mapping: ValueMappingSpec`) |
| `mapping.linear/log/custom` | `ValueMappingSpec::{Linear,Log,Custom{id}}`、custom 実体は `Arc<dyn CustomValueMapping>` を `ValuePorts` 経由注入(canonical log は自然対数のみ) |
| `stepDomain.plain/.normalized` | `StepDomain::{Plain,Normalized}` |
| `outOfRange.clamp/.reject` | `OutOfRangePolicy::{Clamp,Reject}` |
| `GestureId`/`ControlId`(open string) | `String` type alias(canonical open union を保持。newtype 化しない) |
| `InteractionControl` | 同名 struct + `dispatch(&mut self, ControlInput) -> InteractionEvent` |
| `beginGesture/endGesture/setAvailability/detach/observe` | 同名メソッド |
| `detachPolicy` | `DetachPolicy::{Cancel,Commit}` — spec 毎 |
| `InteractionDispatcher` | `HashMap<ControlId, InteractionControl>` + rebuild 規則 |
| `intentFromGesture` | `pub fn intent_from_gesture(&InteractionEvent) -> Option<CommandIntent>` |
| `Binding`(state 機械部) | `CommandBinding`: minted `cmd-N`、in-flight set、receipt rank(`in_flight < accepted|queued < applied < terminal`)、projection revision guard、`detach()`→stale+generation++、`observe()`→`BindingViewState` |
| `ViewportQuery`/`ViewBinding` | 本 slice では spec payload 化のため port しない(D4) |

`UnavailableReason` は canonical 集合 `{Disabled, ReadOnly,
Unsupported}`。`InputSource`/`CancelReason` は open union なので
`String` newtype + 既知名 constant。`MAX_INBOX` 等の gpui 側 bounded
定数を render module から使う場合は `pub(crate)` 化する(現状 private)。

### GPUI renderer(`ruviform-ui-gpui`, `#[cfg(feature="gpui")]`)

mount seam は backend 非依存の `ViewMount` + `create_view_window`:

```rust
pub struct ViewMount {                     // crate public・gpui 型なし
    pub host: ViewHost,                    // authoritative projection
    pub theme: Option<ResolvedTheme>,      // 一方向 theme 消費
}
impl GpuiDriver {
    pub fn create_view_window(&mut self, spec: &WindowSpec,
                              mount: ViewMount) -> Result<WindowId, UiError>;
}
```

- `render` module は **crate-private**。`RvfView`/`ViewPalette` 等の
  gpui 型を持つ item は内部限定 — boundary guard の signature 検査は
  adapter の公開 API に backend 型が現れることを漏出とみなすため、
  consumer は `ViewMount` で mount し renderer entity を直接扱わない。
- window 構成: `create_window`/`create_view_window` は共通の
  `reserve_window` → `Op::Create{mount}` → `open_window`。
  root は常に `AppRoot` entity(window lifecycle subscription を
  集約)で、content は `AnyView`: mount なし→fixture `RvfRoot`、
  mount あり→`RvfView::mounted(host, theme, sink, inbox, …)`。
- intent 経路: `RvfView` 内の `intent_handler` = `IntentSink::send`
  → `WindowLifecycle::Action(Intent{id:command_id,
  op:"rvf.ui.command.v1", params:{command_id, expect_receipt, intent}})`
  → consumer イベント handler。
- event 経路: `post_event` の `Event` は bounded inbox(`MAX_INBOX`)
  に積まれ、**render 冒頭の `drain_inbox`** が host へ投影:
  - `Event::Result{id, ok, data, error}` → `CommandReceipt`:
    `id`=`command_id`(correlation)。`ok` は transport ack に過ぎず、
    semantic status は `data.status` を canonical receipt 語彙
    (`accepted|queued|applied|rejected|unsupported`)で decode する:
    status 欠落の bare `ok` は `Accepted`(gateway が受領しただけで
    domain 反映ではない — head は引き続き ack bound 対象)、
    未知/非文字列 status は `Rejected`+raw を detail へ(正直な失敗で
    あって偽 `Applied`/既定成功へ落とさない)、`ok=false` は `Rejected`
    (`error.message`/`data.detail`→detail)。`data.revision`→receipt
    revision。`ViewHost::apply_receipt` が dispatch 時の
    `command_index`(`command_id`→`control_id`)で owner binding へ route。
  - `Event::Notify{data}` → `{control_id, revision, value,
    provenance?}` を `StateProjection<serde_json::Value>` として
    `ViewHost::apply_projection` へ route。
  - `Job`/`Lifecycle`/`Error` は view route なし(driver 層の責務)。
- `CommandBinding::mint` は namespace 付き
  (`format!("view-{view_id}:a{attach_epoch}:m{mount_epoch}:{control_id}:{n}")`)—
  receipt は `Intent.id` のみで相関するため同一 view 内で一意である
  必要がある。2段の epoch が所有境界を分離する:
  - `attach_epoch`(`AttachEpoch`)は **runtime instance** 識別 —
    `ViewHost` 構築時に外部注入で不変。`ViewInstanceId` は move/
    layout restore を跨ぐ **logical** 識別なので、host drop→new で
    `mount_epoch`=0 へ戻り旧 receipt が新 head に相関する衝突は
    logical id 単独では閉じられない(root review P1 で確証)。
    epoch の発行権は logical id ライフサイクルの所有者
    (consumer の view session)が持つ `AttachEpochAllocator`
    — `u64` 単調・枯渇は panic(2^64 は実用上到達不能で、
    黙って再利用するより正直)。一意性の保証範囲は session
    (allocator lifetime)内: command id と receipt は inbox 経由で
    しか届かないため process/session restart では旧 session の
    receipt が新 host に届く経路自体が存在しない。`AttachEpoch::new`
    literal は単発 host の test/fixture では合法。
  - `mount_epoch` は `ViewHost` 所有で fresh `mount` 毎に増加し
    `unmount` を跨いで持続する — binding は unmount で破棄・
    remount で counter/gen=0 へ再生成されるため、同一 runtime
    instance 内の stale receipt を吸収する。
  `update` 経由の remove/re-add は remount ではなく binding が生存し
  counter が継続するため両 epoch とも据置 — epoch 境界は
  `unmount`(mount)と host drop(attach)のみ。consumer は
  command_id を opaque として扱うことが前提で、format 追加は
  typed adapter(SV182 系)の意味を変えない。
- `render` で spec tree を `AnyElement` へ写像:
  - `Panel` → `div` + children `flex_col`、`Tabs` → header row
    (active tab = accent underline、hairline border)+ active child
  - `Fader`/`Seek` → `canvas` quad(track/fill/thumb)+ pointer
    normalize → `ControlInput::{Begin,Preview,Commit}`、wheel→`Adjust`
    (Shift=fine)。`descriptor.disabled|read_only`→disabled 色、
    `visual.error`→error 色、Seek thumb→viz cursor 色
  - `Waveform` → `canvas` で `WaveformBin` quad 描画
    (baseline/min-max/rms/gap skip、theme の `viz.*` token)
  - window 非活性化 → 全 active drag に `ControlInput::FocusLost`
    (canonical lifecycle input、seq drop 免除)
- `drag` は bounds store(canvas prepaint が window 座標を記録)で
  normalized 計算。window 全体の `on_mouse_up` が drag 継続/終端を保証。

#### NativeSlot glue(#387 × #388、PR399 merge 後)

`ComponentSpec::NativeSlot(NativeSlotSpec)` は native child surface が
描画する layout 領域を宣言する**非対話 leaf**。runtime の
`SurfaceSlotId` は spec に含めない — `declare_slot` が live window
を要求するため spec は先に存在できず、また id は runtime registry
発行で serialize 契約に載せない。関連付けは driver 側 binding:

```rust
impl GpuiDriver {
    pub fn surface(&self) -> SharedSurface;   // canonical 共有 opaque handle
    pub fn bind_native_slot(&mut self, window: WindowId,
                            key: ComponentKey, slot: SurfaceSlotId)
        -> Result<(), UiError>;   // window/spec.window/live 検査
    pub fn unbind_native_slot(&mut self, window: WindowId,
                            key: &ComponentKey) -> Result<(), UiError>;
}
```

- `GpuiDriver` は canonical `GpuiSurfaceAdapter` を `new` で生成し
  `SharedSurface`(`Rc<RefCell<Adapter>>` を内包する opaque cloneable
  handle)として共有する。**per-call borrow が境界契約**: consumer
  には `Rc`/`RefCell`/`RefMut` を公開しないため、consumer が任意長の
  borrow guard を保持して renderer の push を恒久的に starve する
  反例(既に `Visible` の slot が `Hidden`/geometry/clip push 失敗で
  stale `Visible` に留まる露呈クラス、root review P1)は公開 API
  からは構成不能になる。borrow 拒否は reentrancy のみで到達可能で
  `SlotError::Backend` として typed に報告される(panic/silent
  success ではない)。`SharedSurface` は `NativeSurfaceSlot` trait
  自体も実装するため trait-object 文脈でも同一 ledger を共有でき、
  `state`/`spec`/`container_token` は `Result<Option<…>, SlotError>`
  (「ledger を参照できない」Err と「slot 不明」`Ok(None)` を混同
  しない)。adapter は instance 毎に独立 `SurfaceSlotRegistry`
  ledger を持つため、consumer は必ず `driver.surface()` 経由で
  declare/lease/redeem する(`GpuiSurfaceAdapter::new` は別 ledger を
  持つ standalone 用途 — PoC/fixture 用)。二重 OS ledger や別経路
  hide による補償は行わない — 修復は borrow 所有権の境界にある。
- `Slot`(driver window record)に `slot_bindings:
  Rc<RefCell<BTreeMap<ComponentKey, SurfaceSlotId>>>` を持たせ、
  `open_window` が `SlotMount{adapter, bindings}` として `RvfView`
  へ注入。consumer は `bind_native_slot` で window+slot 有効性を
  検査済みの関連付けだけを登録できる。
- renderer は frame 毎に: ① render 冒頭で `placeable` を clear、
  ② `element_for` が bound `NativeSlot` を `placeable` 登録 + canvas
  prepaint で `bounds`(window coords)→`set_geometry`、
  `window.content_mask()`→`set_clip`(viewport 全体なら `None`)、
  visibility を dedupe ledger 経由で push、
  ③ tree 構築後 `(bindings ∪ applied) − placeable` を `Hidden` へ
  suppress(inactive tab/removed/unbind = 未描画)。
- dedupe ledger(`applied`): aspect 別 last-pushed 値を保持し同一値は
  port 呼び出ししない。`Backend`/`SlotBusy` は retry、恒久的
  error(`Unsupported`/`Released`/`StaleGeneration` 等)は aspect を
  `failed` 化して毎 frame の hammering を防ぎ `last_error` に記録。
- visibility は **fail-closed**: `Visible` は ledger の geometry が
  現在 frame の要求値と一致 かつ clip が現在要求値と一致する間のみ。
  refused borrow・transient error・必須 op の永久拒否では ledger が
  要求値を保持しないため `Hidden` を維持/復帰する(unclip 済みでない
  child・旧 geometry の child を祖先 clip 外へ露出させない)。
  `clip=None` 要求は ledger `clip=None`(未 push=port 初期状態)で
  充足 — `set_clip` を push すらしないため clip 非対応 driver でも
  clip 不要 frame では表示できる(必要条件の契約区別。clip 解除不能
  は上流 #404 の責務であり、ここでは黙示補償しない)。
- slot lifecycle(declare/release/lease/redeem/busy)は consumer が
  adapter 経由で所有 — glue は layout→port 投影のみ担い、
  RF388/389 の責務を複製しない。unbind 後の suppress は renderer が
  `Hidden` + `clip=None` を push して port を fresh-ledger 不変条件
  (hidden AND unclipped)へ戻してから ledger から drop する —
  これを省くと rebind した slot の fresh `clip:None` ledger が
  `None` 要求を vacuous 充足し、port が stale rect で clip した
  まま `Visible` になる(同一の露出クラス)。
- preview 中は `ViewHost::observe().display` を thumb 位置に使う
  (controlled display = preview。committed 描画と混同しない)。

### 所有境界の確認

- `driver.rs`/`dock.rs`/theme crate/`driver_lab.rs` 本体は無変更。
- `cases.json`/`stories.json` は binding 行の status/mechanism/rerun
  更新のみ(schema 語彙内)。
- `Cargo.lock`: 新規 crate edge なし(既存 member のまま)。

### リスク

| リスク | 対応 |
| --- | --- |
| gpui `TestAppContext::add_window_view` が Windows headless で動かない | 試行→失敗なら gpui binding は `not_run`+実機 lane と正直に記録 |
| drag 中の window-level mouse move 取得 API が 0.2.2 で非公開 | `InteractiveElement::on_mouse_move` + `on_mouse_up_out` で近似、pointer 外 drag は viewport clamp で処理(documented limitation) |
| `WaveformBin` と `ruviform-viz::ViewBin` の field 意味 drift | field 名を `ViewBin`/`LodBucket` に一致させ、consumer 側の変換を 1 関数に集約して drift を抑止 |
