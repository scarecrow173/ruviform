---
id: "DESIGN-034L2Cjm1Zjxj8oQl2jINm"
type: "design"
status: "approved"
title: "OFX ホストアダプタ詳細設計（Issue #98）"
created: "2026-10-24"
updated: "2026-10-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/98"
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/ofx-host-adapter.md"
    - "../adr/ofx-host-video-contract-and-c-abi.md"
    - "../discovery/ofx-host-adapter-boundary-and-bindings.md"
  derived-by:
    - "../plans/implement-issue-98-ofx-host-adapter.md"
    - "../plans/implement-issue-124-ofx-window-lease.md"
    - "../specs/issue-124-ofx-cross-window-h-exposure.md"
    - "../test-specs/video-plugin-plain-contract-tests.md"
    - "../test-specs/ofx-host-native-contract-tests.md"
  verifies: []
  verified-by:
    - "../test-specs/video-plugin-plain-contract-tests.md"
    - "../test-specs/ofx-host-native-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
---

# OFX ホストアダプタ詳細設計（Issue #98）

> status: approved（Cycle 1 BLOCKED 改訂 → Cycle 2 Judge APPROVED — ACCEPT WITH NON-BLOCKING NOTES）。コード・依存追加は含まない。
>
> **Approval handoff（Judge settled implementation obligation — 未検証 bugfix ではなく承認条件としての持越）**: adapter 所有の**単一 ClipPrefs validation helper** を activation 時と各 render で（local preflight 後・`BeginSequenceRender` 前・無条件に）呼ぶ。unsupported/failed prefs は reject — Begin/Render/出力 pixels/metadata/padding は不変。valid な typed param setter の rollback は不要（後続フレームで検証・render する）。実 C slave Bool/Choice fixture の counter+sentinel+recovery oracle は将来 plan/testspec に引き継ぐ。dirty flag 状態・caller reactivation・新 core API は採らない。実装・検証は plan/tests 側の責務 — ここで完了主張しない。

## Architecture

- **新クレート `crates/ruviform-plugin-ofx`** — OFX adapter。依存方向は adapter → `ruviform-plugin-core` / `ruviform-video-core` / `ruviform-media-core` のみ
- **`plugin-core` への新規追加**: `src/descriptor.rs` に借用 `PluginDescriptorView<'a>` + 既存 `PluginDescriptor.view()`（identity 用・依然必要）、新規 `src/plain_param.rs`（下記）、`src/video.rs`（`VideoProcessConfig`・`VideoPluginInstance`）。core lib exports/Cargo.toml、README/allowed-deps 索引・baseline OFX 行/DAG annotation・Lifecycle Processing の RT doc 明確化も対象。**旧 core `param.rs`/event 形式は変更しない**
- C bridge: pinned 公式ヘッダ @ `c3f1c2dc…` + `cc` 1.9.0 + `libloading` 0.8.9（locked 値）

### plain_param.rs（新規・中立 typed plain 制御チャネル）

```rust
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PlainParamValue { Double(f64), Integer(i32), Boolean(bool), Choice(u32) }

#[derive(Copy, Clone)]
pub struct PlainParamInfo<'a> {
    pub id: ParamId, pub name: &'a str, pub unit: &'a str,
    pub min_value: f64, pub max_value: f64,
    pub display_min_value: f64, pub display_max_value: f64,
    pub default_value: PlainParamValue, pub flags: ParamFlags,
    pub choices: &'a [String],
}
```

- `PlainParamValue` の Double 入力は caller が非有限を構築し得る — **コンストラクタが有限を強制すると主張しない**。返却/保存される native 値は finite・型・range を検証済みとする
- kind enum は別途設けない（variant が type を符号化）
- `choices` は adapter 所有の snapshot（ABI 型/ static leak なし）。非 Choice は空。Choice 選択肢は非空・default/current index は `0..len`・C int に収まる。Boolean bounds は 0/1。Integer の論理 bounds は i32 整数範囲内（metadata は f64 表現・正確な i32 範囲）。Double bounds は有限かつ ordered の**端点**で、`max-min` が overflow するだけでは reject しない。default は hard bounds 内
- display bounds は有限・ordered の **UI hint** で、hard domain を暗黙に制限しない。UI は plain 数値編集のみ必須、slider 投影は optional/presentation のみ。**native 値の get/set は ParamValue を経由しない**
- 動的な param 構造/kind/choice 選択肢の変更は初期スコープ外。値・UI enabled・label の通常更新は SDK どおり許容し、control action 後に snapshot を refresh。sample-accurate automation/keyframe 主張なし

### Core API（署名の正本 — `plugin-core/src/video.rs`・`plugin-core → video-core/media-core`）

```rust
pub struct VideoProcessConfig { /* private: format: PixelFormat, width: u32, height: u32,
                                   frame_rate: FrameRate */ }
impl VideoProcessConfig {
    pub fn new(format: PixelFormat, width: u32, height: u32, frame_rate: FrameRate)
        -> ruviform_media_core::Result<Self>;   // 非ゼロ dims のみ検証
    pub fn format(&self) -> PixelFormat;        // Copy getters
    pub fn width(&self) -> u32;
    pub fn height(&self) -> u32;
    pub fn frame_rate(&self) -> FrameRate;
}

pub trait VideoPluginInstance {
    type Error: std::error::Error + 'static;
    fn state(&self) -> LifecycleState;
    fn descriptor(&self) -> PluginDescriptorView<'_>;
    fn parameters(&self) -> &[ParamId];
    fn parameter(&self, id: ParamId) -> Option<PlainParamInfo<'_>>;
    fn parameter_value(&self, id: ParamId) -> Result<PlainParamValue, Self::Error>;
    fn set_parameter(&mut self, id: ParamId, value: PlainParamValue) -> Result<(), Self::Error>;
    fn initialize(&mut self, config: &VideoProcessConfig) -> Result<(), Self::Error>;
    fn activate(&mut self) -> Result<(), Self::Error>;
    fn start_processing(&mut self) -> Result<(), Self::Error>;
    fn render(&mut self, input: &VideoFrame, output: &mut VideoFrame, at: MediaTime)
        -> Result<(), Self::Error>;
    fn stop_processing(&mut self) -> Result<(), Self::Error>;
    fn deactivate(&mut self) -> Result<(), Self::Error>;
    fn destroy(&mut self) -> Result<(), Self::Error>;
}
```

- 全操作は明示的に **NONRT**（`render` 含む）。共通 Lifecycle の activity は RT 保証を意味しない
- audio `PluginProcessor`・既存 no-alloc RT テスト・ABI は変更しない

### Lifecycle 遷移（delegated common Lifecycle — adapter 側 duplicate 遷移表なし）

| API | 遷移 | 失敗時 |
| --- | --- | --- |
| `initialize` | Created → Initialized（project format 保存 → CreateInstance） | InitFailed → Created のまま retryable |
| `activate` | Initialized → Activated（ClipPrefs 確認/確保は commonActivate commit **前**） | ActivateFailed → Initialized のまま |
| `start_processing` | Activated → Processing | — |
| `render` | Processing のみ（commonProcess self-loop） | — |
| `stop_processing` | Processing → Activated | — |
| `deactivate` | Activated → Initialized | — |
| `destroy` | Created/Initialized → Destroyed・terminal | — |

- 違法遷移は foreign call・mutation 前に reject。成功した Create なし → **NativeDestroy は呼ばない**。NativeDestroy は高々 1 回・失敗でも terminal + uncertain resource は session 所有で Unload まで隔離
- wrapper drop は Processing → stop、Activated → deactivate、Initialized → destroy の順で正規化し physical な native Library unload まで DLL・suite・registry を生かす — `NativeDtor` retry はしない
- NativeCreate/Destroy 中の callback 可否は SDK の action/liveness 契約に従う — 「Rust Created/Destroyed なら SDK callback 不可」の blanket ルールではない。公開 Created staging・Destroyed 値 reject は別の制御面として別途定義する

- setter は wrong variant・Double NaN/inf・hard bounds 外・invalid Choice index を **mutation/native action 前に**構造化エラーで reject。**plain チャネルに暗黙の clamp/round はない**
- native C setter は同一の validation authority を使うが再帰的な InstanceChanged 通知はしない。host UI の readonly は plugin 自身の readonly/UI-disabled 出力更新を妨げない

## State policy（A2 確定形）

- `state`/`descriptor`/`parameters`/`parameter` metadata 読出しは Destroyed を含む全状態で可（owned cache・read-only・foreign action なし）
- `parameter_value`/`set_parameter` は **Created でも合法**（cached typed staged values）。setter は cache するのみ — **Create 前に native InstanceChanged を発行しない**
- `initialize(config)` は staged values を attempt registry にコピーしてから NativeCreate。成功 → Initialized。失敗 → host attempt リソースは破棄し Created の staged values を保持（retry 可）。foreign-owned alloc の cleanup は過保証しない
- Initialized/Activated/Processing では同期 render 呼出しの合間に get = 実 native typed state 読出し・set = 検証後 Begin/InstanceChanged/End（SDK optional ReplyDefault）+ metadata snapshot refresh
- `&mut self` + thread 限定で Rust setter/render の並行なし。Destroyed での値 get/set は `InvalidState` reject
- native callback paramSet は実 state を更新するが再帰通知しない。Lifecycle への state policy 複写はしない

## Loading / Admission（A3a — deliberate product behavior）

```rust
impl OfxHost {
    pub fn new() -> Result<Self, OfxError>;                       // 1 process 1 session
    pub unsafe fn load_library(&mut self, path: &Path) -> Result<OfxLibrary, OfxError>;
}
impl OfxLibrary {
    pub fn plugin_count(&self) -> usize;                          // admitted 件数
    pub fn plugin(&self, admitted_index: usize) -> Option<PluginDescriptorView<'_>>;
    pub fn create_instance(&self, admitted_index: usize) -> Result<OfxInstance, OfxError>;
    pub fn rejections(&self) -> &[PluginRejection];
}
pub struct PluginRejection { /* private */ }
impl PluginRejection { pub fn native_index(&self) -> usize; pub fn identifier(&self) -> Option<&str>; pub fn error(&self) -> &OfxError; }
```

- **admitted index ≠ raw native index・persistence ID**。admitted は library の採用済み list の添字
- library レベルの IO/symbol/optional SetHost refusal/invalid count は load_library 失敗。valid SDK エントリで unsupported API/context/param/profile・per-entry Load/Describe error は **per-entry rejection** として記録し successful Load は一度だけ Unload で cleanup（balanced）。同 bundle の valid Filter entry は使える
- admitted list が空でも成功の Library + rejections を返す（fake 'compatible' にしない）
- context/param チェックは per-entry で admitted list 公開前に完了。initialize の config profile チェックは CreateInstance 前。`VideoProcessConfig` は汎用 geometry の非ゼロ検証のみ、OFX 側 `initialize` は Rgba8・dimensions・**算出した staging stride/size** の C 制約を先に検査する。実際の input/output plane の stride・mutability・data extent は render 時にのみ得られる — `BeginSequenceRender` 前の preflight で検査し、フレーム未存在時に「実フレーム stride を検査済み」とは主張しない。既存 `VideoFrame` の `set_pts`/`set_duration`/`set_color` で足り、追加の video-core metadata API は不要
- `Rc` session が native DLL+suite host を保持し、OfxHost/OfxLibrary wrapper drop 後も instance 生存中は unload しない。canonical path cache が immutable な admission 結果を共有し重複 Load なし・filesystem autoscan なし
- cache された control metadata は C メモリへの借用を含まない

## Private boundary design

- C bridge が pinned SDK suite テーブル・stable heap `OfxHost`・typed accessor を所有
- 公開 `Rc` 制御面は `!Send`/`!Sync` で、synchronized な Send-safe callback registry / native state とは分離。callback は plugin 作成 thread から来得るため **Rc/RefCell/制御 metadata snapshot に触れない** — callback データは mutex 保護の owned state / stable buffer のみ。Mutex や借用 registry guard を foreign call を跨いで保持しない。`!Send` が foreign callback 安全性を証明するとは言わない
- opaque generation token は never-reused・owner/kind 検査（unknown/stale → `BadHandle` を deref 前に）。実際の MemorySuite `void*` 確保は managed owned pointer で、fake generation ID ではない。MemorySuite/`imageMemory` の allocation は明示 **16 バイト alignment の real 実装**で、alloc 失敗は `ErrMemory`。callback/host が渡す C 文字列は SDK の合法な使用期間より長生きする stable 所有で、temporary `CString.as_ptr` の lifetime 依存は禁止
- SDK がサポートする single-instance property は `kOfxImageEffectPluginPropSingleInstance` を成功 NativeCreate 前に enforce。複数 Created wrapper は可・live native instance がある間の 2 回目 initialize は Create せず失敗。`InstanceChunkCount` のような存在しない property は発明しない
- `NativeDestroy` は成功作成済み instance に高々 1 回・terminal。失敗時は uncertain resource を session 所有で Unload まで隔離し、破壊済みという誤前提で別 SingleInstance ネイティブを再 admission しない。native crash isolation は約束しない
- unsafe は FFI boundary module に局在し各 unsafe 操作に SAFETY コメント + scoped expect/allow（workspace ポリシー）。core や FFI 上位の crate 全体 `allow(unsafe_code)` は置かない
- SDK ordering 確定: optional DLL `OfxSetHost` → `OfxGetNumberOfPlugins`/`OfxGetPlugin` → per-plugin mandatory `setHost`/non-NULL main entry → `Load` → `Describe`/`DescribeInContext` → `Create`。Choice varargs は `int*`（`char**` ではない）。`OfxTime` は fractional frames 許容。`clipGetImage` の region は `const OfxRectD*`・full-frame RoD の superset は clipping 後に許容。missing/off-time image → `Failed`+NULL。live image handle は action-scoped で全経路 cleanup（`imageMemory` 確保とは別 lifetime）。`kOfxImageEffectHostPropNativeOrigin`（値 `kOfxHostNativeOriginBottomLeft` 等・literal `k` 接頭の SDK マクロ定数を使用し文字列名を発明しない）は UI hint のみ — 物理行方向の証明ではなく、負 stride コピーロジックは不変
- ParamV1 テーブル（compact かつ exact・全 slot 提供・all-OK stub なし）: get/set scalar values 実装・`getAtTime` = current static 値・`NumKeys`→0/OK・`KeyTime`/`DeleteKey`→BadIndex・`KeyIndex`→-1/Failed・`DeleteAll`→OK（empty）・`SetAtTime`/`Derivative`/`Integral`/`Copy`/`EditBegin`/`EditEnd`→明示 Unsupported（no mutation・unknown/stale handle は先に BadHandle）。restricted static profile であり general conformance ではない。variadic C typed 変換は Kind（Double vs int 他 scalar）に一致。未知/未対応 suite は NULL
- capability は正直に: hostname `org.ruviform.ofx-host`・label `Ruviform`・host version = workspace `0.1.0`・ImageEffect plugin API version 1・hostAPIVersion `[1,0]` baseline（全最新 SDK feature 主張ではない）。contexts=Filter・components=RGBA・depths=Byte のみ。Temporal/Tiles/MultiRes/Overlays/multipleDepths/multiplePARs なし・OFX custom interact なし・animation flag 0・sequentialStatus/interactive/RenderQualityDraft=0・FieldNone・scale=1。**plugin が未対応 feature を要求（REQUIRE）する場合は reject するが、より多機能をサポートするだけの plugin は reject しない**
- host suite・文字列・opaque handle は合法な native 使用より長生き。Rust callback は panic を catch し SDK status に変換（C 境界を unwind しない）

## Image / Time semantics

- adapter 所有の aligned staging buffer（Rust audio slice の alias ではない）。native 突入前に `Rgba8`/dims と `plane_mut` 一意性を確認
- OFX 側へは bottom-left 露出（最終物理行 pointer + 負の checked-i32 `rowBytes`・`bounds[0,0,w,h]`）。native staging stride は 16 でもよい（fixture の source padding を native plane 契約にしない）
- フレームごと `BeginSequenceRender`（`frameRange=[t,t]`・step1・scale[1,1]・fieldNone）→ `Render`（full window）→ `EndSequenceRender` を Processing 内で括る。Render 失敗でも Begin 成功分は必ず閉じる・primary error 保持・全シーケンス成功時のみ commit — 失敗時は出力 pixels+metadata 不変
- `clipGetImage` は active call・current requested time のみ。image handle の release/lifetime 均衡
- `t = at.to_seconds_f64() * f64::from(fps.numerator()) / f64::from(fps.denominator())`（finite 検査）。`to_seconds_f64` は docs 上 display-only API で OFX ABI は f64 表現が不可避 — **極端な時刻で厳密一致を保証しない**。アルゴリズム上の exactness 主張なし
- requested time ≠ source input.pts。入力は requested time の source、出力 pts=`at`。source の ColorInfo/duration 保持。shared output/no mutation/native 直 call は reject。`ClipPrefs` は非 RGBA/Byte/PAR1/fieldNone/unpremultiplied を reject

## Param semantics（typed plain チャネル）

- native descriptor/runtime metadata は `String` を所有し中立コアへは借用ビューで投影。`ParamId` は define order で安定
- get は typed native host state を読む（initialized 以降）・set は検証済み値のみ native に流す
- UI へは normalized ではなく typed metadata（labels/defaults/ranges）+ plain 値を出し、generic UI は OFX 型なしで consume — normalized writeback を強制しない

## AC / Test matrix（文書上の要求 — 未実施・Cycle 2 入力）

| # | 検証 |
| --- | --- |
| 1 | 実物所有 C fixture（pinned ヘッダでビルド・`OfxGetNumberOfPlugins`/`GetPlugin`/`setHost`）を temp DLL/SO/dylib の実 path からロード。mock-only ではない |
| 2 | lifecycle 順序・違法呼出で foreign callback ゼロ・失敗 Create/activate の retry/cleanup・plugin ID/API/null entry/unsupported context/type error |
| 3 | property scalar/N/error・vararg get/set scalar double/int/bool/choice + 未対応型/NULL・未知 suite/version → NULL |
| 4 | image memory alloc/lock/unlock/free/alignment error |
| 5 | **plain bridge gain**: 論理 range ±1e20・display [0,2] で default/current=1 — get→set 同一 1 が actual C GetValue/AtTime を通して保持。distinct 0 と 1 が collapse しない。setDouble 3（hard bounds 内・display 外）は受理/保持。wrong kind/NaN/inf/hard range 外は native action 前に reject・不変。generic UI は typed metadata/choice label/default + plain 値を OFX 型なしで consume・normalized writeback 必須でない |
| 6 | `PlainParamValue::Double(0.5)` の gain が出力画素に反映（normalized gain ではない）|
| 7 | Created での setter は native action を発行せず staged value が Create で見える。render 間の setter 合法。Destroyed での値 access reject。host UI readonly write は reject だが native plugin 所有値更新は合法 |
| 8 | 混在 DLL（Filter 可 + Generator 除外）: 可 entry は利用可能・rejections は構造化。optional DLL SetHost 順序・per-effect setHost/null 検査 |
| 9 | SingleInstance enforce（initialize 時）。NativeDestroy 失敗は terminal・重複呼出なし・uncertain resource は session cleanup まで隔離 |
| 10 | 2×2 RGBA orientation oracle: source 物理 top `[10,20,30,255],[40,50,60,255]`・bottom `[70,80,90,255],[100,110,120,255]`・padded core stride 12・gain 0.5 → top `[5,10,15,255],[20,25,30,255]`・bottom `[35,40,45,255],[50,55,60,255]`（RGB 半減・alpha 不変）。Failure/End 失敗で pixel+metadata+padding sentinel 不変 |
| 11 | `Render` 成功後 `EndSequenceRender` 失敗 → 出力 sentinel 不変（transaction 原子性） |
| 12 | interface が `Box::leak` で native `String` を永続化しない・新 core view が owner を越えて生存しない |
| 13 | 1 session/creator-thread 制限・canonical 同一 DLL 重複 Load/unload なし・root/library drop 後も instance 生存中は render 動作。**auxiliary `libloading` Library が DLL を loaded に保ってアサーションを隠さないよう、lifetime assert 前に補助 DLL handle は閉じる**。test-only の process-lifetime callback counter で観測可（production logging ではない）。第三者プラグイン実行なし |
| 14 | 時間写像 `t=1.001s @ 30000/1001 → ≈30`（小値ケースの absdiff ≤ 1e-9 のみ・worst-case 保証ではない） |
| 15 | MSRV 1.89 で lib+tests+C bridge コンパイル・既存 audio RT ゲート不変 |
| 16 | scoped doc checks のみ主張・platform CI は実走範囲のみ記録 |

## Risks / Triggered follow-ups

- pixel format/context/param kind 拡大・animation 対応（本 profile 外）
- native host worker scheduling・generic video registry（実 consumer 需要時）・L1（#76）非主張
- C1: 既存 core `param.rs:223-246` overflow 系は別 defect として留保（本 causal path 外・未修正）
- 全量最終 gate は plan/task 起票後の一回のみ
