---
id: "DESIGN-034Mclz5y96MrTg7R2BNCx"
type: "design"
status: "approved"
title: "エンジン/利用側へのプラグイン統合（RT endpoint 所有分離 + グラフ接着）"
created: "2026-10-25"
updated: "2026-10-26"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/113"
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/74"
    - "https://github.com/scarecrow173/ruviform/issues/114"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/engine-plugin-host-integration.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/rt-foundation-primitives-implementation.md"
    - "docs/adr/plugin-isolation-strategy.md"
    - "docs/adr/ofx-host-video-contract-and-c-abi.md"
    - "docs/designs/rt-verification-lane-design.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/engine-plugin-host-integration.md"
    - "docs/adr/plugin-rt-ownership-split.md"
    - "docs/discovery/engine-plugin-host-integration.md"
    - "docs/designs/audio-engine.md"
    - "docs/adr/audio-engine-snapshot-swap.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/designs/plugin-host-l0.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
  derived-by:
    - "docs/plans/implement-issue-113-engine-plugin-integration.md"
    - "docs/test-specs/plugin-endpoint-contract-tests.md"
    - "docs/test-specs/plugin-graph-e2e-tests.md"
    - "docs/test-specs/plugin-rt-retirement-tests.md"
    - "docs/discovery/issue-114-real-adapter-l1-e2e.md"
    - "docs/specs/issue-114-real-adapter-l1-e2e.md"
    - "docs/designs/issue-114-real-adapter-l1-e2e.md"
    - "docs/plans/implement-issue-114-real-adapter-l1-e2e.md"
  verifies: []
  verified-by:
    - "docs/test-specs/plugin-endpoint-contract-tests.md"
    - "docs/test-specs/plugin-graph-e2e-tests.md"
    - "docs/test-specs/plugin-rt-retirement-tests.md"
  references: []
  defers:
    - "docs/specs/issue-114-real-adapter-l1-e2e.md"
  deferred-by: []
---

# エンジン/利用側へのプラグイン統合（RT endpoint 所有分離 + グラフ接着）

## Context

Issue #113 — #76 で host API として完成した L1 アウトプロセス経路（`PluginHost` コンポジット）を、audio エンジンの pull グラフから実際に消費できるようにする。本書は discovery `docs/discovery/engine-plugin-host-integration.md` の Open Questions 全項目と spec `docs/specs/engine-plugin-host-integration.md` の 11 受入基準を、ADR `docs/adr/plugin-rt-ownership-split.md` の確定分岐（**transferable RT endpoint + non-RT 制御所有分離**）に沿って確定する。

上流確定事項（再決定しない）: `PluginHost` は `!Send`/`!Sync` で non-RT 制御側専有・RT 側はインスタンス毎の `Send` endpoint をノードが所有・engine は plugin 型を公開 API に露出しない（baseline 依存方向ルール）・全域 `Send` bound は不採（opt-in capability）・退役 plugin リソースを RT コールバック上で drop しない・video は NONRT 境界の明文化 + trigger 付き延期・自動再起動なし。

## 決定サマリ（discovery Open Questions → 本書の結論）

| # | 論点 | 決定 |
| --- | --- | --- |
| a | endpoint の公開形 | **`PluginHost::prepare_endpoint(handle, bounds) -> Result<PluginEndpoint, HostError>`**（non-RT verb）。`Processor::prepare → Rt` の型分離と同形 — endpoint は 1 度だけ `Send` でノードへ移される |
| b | L0 `Send` 処理オブジェクト | **opt-in capability + 共有セル**: `PluginFactory::create_sendable() -> Option<Box<dyn ErasedPlugin + Send>>`（default `None`）。Send 対応インスタンスは `SharedSlot { core: Arc<Mutex<SendCore>>, mirror: Arc<AtomicU8>, attached: Arc<AtomicU64> }` スロットで host と endpoint が共有し、non-RT verb は到達可能のまま（全 verb が `core.lock()` 内で transition commit + mirror store + plugin hook を atomic に実行）。attach は epoch CAS。非対応は `Err(HostError::EndpointUnavailable)` で defined rejection |
| c | endpoint の RT 実行形 | endpoint = `{ EndpointInner(L0/L1), bound, in_scratch, latency_frames }` — L0 leg = `{ core, mirror, attached, epoch }`、L1 leg = `{ leg: Option<RtLeg>, mirror, arm_rx, arm_retirer, return_tx }`。`process` は mirror → 定義済み出力表（下表）で駆動。per-block gate は activated 契約を検査し、`bound` は scratch 上限のみ |
| d | L1 `Arc<Staging>` arm 経路 | **per-attach SPSC arm channel（capacity 2）+ endpoint 側 `Retirer`（capacity 4）**。channel pair は `prepare_endpoint` 内で鋳造し、現在の `Inner::staging` を seed arm として push。`Activate` 成功時に control が `try_push`（`Full` は `pending_arm` 退避・**全 `&mut` non-RT entry point** で再送 — `&self` query は defined non-drain）。endpoint は `process` 冒頭で drain し generation 単調増のみ apply — drain した arm は適用・不適用を問わず全て `Retirer` へ。snapshot `slot` は初回 read の per-thread debt alloc（warm-up 例外）があるため不採 |
| e | 退役 offload のシーム | **`plan.rs` に `Retirer<Arc<GraphPlan>>` を配線**（最小の整合的修正）。`RtPlan.cached` 差替時の旧 `Arc` と `RtPlan` 自身の `Drop` を retire 経路へ。collector は `PlanSwap` が保持し `swap`・`drain_retired`・teardown の**全 non-RT entry point** で drain（`retired_overflows()` で飽和を観測）。全 node/endpoint/plugin リソースが collector スレッドで drop される |
| f | ノード接着の crate 配置 | **engine は `AudioNode` の `FnMut` blanket impl、plugin-host は `PluginEndpoint::process_audio`**。consumer が endpoint を move した `Send` closure を `graph.push` する — 相互に型を名指さない（baseline A.2 整合） |
| g | `Io`/バッファ写像 | **plugin-core が `BufferView`/`BufferViewMut` を audio-core `AudioBufferView`/`AudioBufferViewMut` に impl**（`PlanarBufferView*` への既存 impl と同じ seam）。`process_audio` は `out` を per-block 実フレーム数で組んだ scratch view（`AudioBufferViewMut::try_new` — `buffer.rs` L535）へ複写し in-place transform として処理。block が activated `[min_frames, max_frames]` 外なら `Err(Prepare(UnsupportedConfig))` で `out` 不変更 |
| h | non-RT verb 到達（graph-attached） | L0 共有セル経由で **全 verb が到達可能のまま**（`lock()` は non-RT 側のみ・RT 側は `try_lock` → busy は defined bypass）。L1 は wire verb 不変・`process` は detached → `Err(EndpointUnavailable)` |
| i | 遅延報告 API | **`latency_frames() -> u32`（frames 整数）**を `PluginHost`（handle 経由・`Option`）と `PluginEndpoint` の双方に露出。秒変換は `frames / sample_rate` を doc に明記 |
| j | graph E2E 配置 | **`crates/ruviform-plugin-host/tests/` に dev-dep `ruviform-audio`**（循環なし — audio の prod dep は不変で AC3 の Cargo.toml 非露出を厳格に満たす） |
| k | video 境界 | audio `process` 経路のみ。`VideoPluginInstance` 全操作 NONRT・`prepare_endpoint` は audio instance のみを対象とし、L1 video wire 非確立・trigger 付き延期を Deferred に明記 |

### spec 修正記録（本書レビューで確定した spec 差分）

- **AC6・遅延式の修正**: spec の `lookahead_blocks × block_frames / sample_rate` は実パイプライン量子と不整合 — ブロック長は可変で `block_frames` は定数でないため、報告量子は activated `max_frames`（wire/staging が dimension する上限）とする。**`latency_frames = lookahead_blocks × activated max_frames`（frames 整数。秒換算は `frames / sample_rate`）**へ spec `docs/specs/engine-plugin-host-integration.md` の AC6・What/Scope/Requirements の同式を修正済み。実装時に `composite.rs` の latency doc（同じ旧式を記載）も揃える。
- **`bounds.channels` の prepare-time bus 等値の緩和（v1 修正 — 本書レビューで確定）**: 当初の決定サマリ・bounded 検証は「`bounds.channels` は plugin の activated メイン bus チャンネル数と一致必須・不一致は `Err(Prepare(UnsupportedConfig))`」としていたが、**plugin の channel 照会 API が存在しないため prepare 時点では照合不能**（`Activated` config は frames/rate を持つが bus チャンネル数を持たない）。v1 では prepare-time 等値を外し、代わりに (i) `bounds.channels` 非 0・`≤ WIRE_MAX_CHANNELS`、(ii) per-block `out.channels() == bound.channels` を `process_audio` bound clip で強制（不一致は `Err(Prepare(UnsupportedConfig))`・`out` 不変更）、(iii) `in_scratch` を `bound.channels × bound.max_frames` で bound-shaped 確保、の 3 条件で拘束する。**実 channel query・activated bus との reconcile は Issue #114 系の deferred contract work**（真の channel query 導入後に prepare-time 整合を再評価）。`EndpointBounds.channels` の field コメント・bounded 検証・拒否表に反映済み — spec 側の AC4 表現（「bounds 検証」）は文言上この緩和と矛盾しないため spec 本文の改訂は不要・本記録で pin する。

## Architecture

### クレート構成と依存方向（baseline A.2 適合）

| クレート | 追加要素 | 追加依存 |
| --- | --- | --- |
| `ruviform-plugin-core` | `PluginFactory::create_sendable`・`erase_sendable`・`BufferView`/`BufferViewMut` の `AudioBufferView`/`AudioBufferViewMut` への impl・`HostError::EndpointUnavailable` variant | なし（`ruviform-audio-core` は既存依存） |
| `ruviform-plugin-host` | `endpoint` モジュール（`PluginEndpoint`/`EndpointBounds`・L0/L1 leg）・`PluginHost::prepare_endpoint`/`latency_frames`・L0 共有セル + mirror・L1 leg 移出 + arm channel + leg 返却 channel | `ruviform-audio-core`（dev-dep → 正式 dep へ昇格。Domain foundations で A.2 許可範囲） |
| `ruviform-audio` | `AudioNode` の `FnMut` blanket impl・`plan.rs` 退役シーム（`Retirer<Arc<GraphPlan>>` + `PlanSwap` collector + `drain_retired`） | なし（`ruviform-rt` は既存依存。**plugin-host 型・依存の露出ゼロ** — AC3） |
| `ruviform-rt` | なし（`channel`/`retire_channel` を再利用 — 新規機構・新規外部依存なし） | — |

**接着は利用側（consumer）配線**: engine が plugin 型を知らず、host が engine 型を知らない。`Send` な closure が両者の唯一の接点であり、`Send` 証拠は closure capture の型が静的に担う。

### 所有分離と endpoint 型

```rust
// plugin-host::endpoint（概念形 — シグネチャの正本）
pub struct PluginEndpoint {
    inner: EndpointInner,          // 非公開 — L0/L1 の層差は内部化（呼出し側は層を意識しない）
    bound: EndpointBounds,         // 構築時の capacity 契約 — scratch 上限のみ。per-block gate は activated 契約が担う
    in_scratch: AudioBuffer<f32>,  // process_audio の入力複写先（bound.channels × bound.max_frames で事前確保）
    latency_frames: u32,           // L0 = 0、L1 = lookahead × activated max_frames
}
enum EndpointInner { L0(L0Endpoint), L1(L1Endpoint) }
struct L0Endpoint {                     // 共有セル参照 3 本 + 自身の attach epoch
    core: Arc<Mutex<SendCore>>,         // plugin + context + activated — RT 側は try_lock のみ
    mirror: Arc<AtomicU8>,              // lifecycle mirror（L1 rt_state と同型）
    attached: Arc<AtomicU64>,           // attach epoch flag — Drop が CAS(epoch→0) で返却
    epoch: u64,                         // attach 時に CAS した自身の epoch 値
}
struct L1Endpoint {                     // 移出脚 + arm/退役/返却経路 — mutex・shm・child を含まない
    leg: Option<RtLeg>,                 // Drop で return_tx へ返すため Option（生存中は常時 Some）
    mirror: Arc<AtomicU8>,              // Shared::rt_state の Arc 化を clone 共有
    arm_rx: Consumer<Arc<Staging>>,     // per-attach arm channel（capacity 2）
    arm_retirer: Retirer<Arc<Staging>>, // 退役 ring → L1Instance 側 DropCollector（capacity 4）
    return_tx: Producer<RtLeg>,         // leg 返却 channel（capacity 1）
}

pub struct EndpointBounds {
    pub max_frames: u32,   // ≥ activated ProcessConfig::max_frames — 非 0・両層共通で ≤ WIRE_MAX_FRAMES（scratch 確保上限）
    pub channels: u32,     // グラフ側バス幅 — 非 0・≤ WIRE_MAX_CHANNELS（v1: activated bus との等値は要求しない — plugin の channel query が存在しないため。per-block `out.channels() == bound.channels` を process_audio bound clip で強制・bound-shaped scratch を供給・違反は Err(Prepare(UnsupportedConfig))。実 channel 照会/reconcile は Issue #114 系の deferred contract work）
}

impl PluginEndpoint {
    // host 契約形 — PluginHost::process と同じ語彙
    pub fn process(&mut self, inputs: &[HostInput<'_>], outputs: &mut [HostOutput<'_>],
                   events: EventList<'_>, transport: Option<TransportInfo>)
        -> Result<ProcessResult, HostError>;
    // engine 向け接着面 — out を input scratch へ複写して in-place transform。
    // 定義済み非出力時は out を変更しない（Err 返却 or bypass）
    pub fn process_audio(&mut self, out: &mut AudioBufferViewMut<'_, f32>,
                         events: EventList<'_>, transport: Option<TransportInfo>,
                         ctx: &RtContext) -> Result<ProcessResult, HostError>;
    pub fn latency_frames(&self) -> u32;
    pub fn bounds(&self) -> EndpointBounds;
}
// PluginEndpoint: Send — !Send な制御状態（PluginHost/L1Instance/裸の ErasedPlugin/segment）を含まない
```

**`process_audio` の borrow 分割と per-block view（機構の固定）**: `process_audio` は `self` を `let Self { inner, bound, in_scratch, .. } = self` で destructure し、`&mut in_scratch`（slab）と `&mut inner`（dispatch）を disjoint borrow として扱う — `&self.in_scratch` と `&mut self` の衝突は構造上発生しない。処理順序を固定する: **(1) bound clip** — `out.channels() == bound.channels` かつ `out.frames() ≤ bound.max_frames` を検査（違反は `Err(Prepare(UnsupportedConfig))` — scratch slice の前提であり activated gate より先に必要）、**(2) per-block view 構築** — `in_scratch.data_mut()[..ch*fr]` に対し `AudioBufferViewMut::try_new(&mut slab[..ch*fr], ch, fr)`（audio-core `buffer.rs` L535 — `data.len() == channels * frames` を要求する借用構築・alloc なし）で**実ブロック寸法の view を毎ブロック組み**、`out` → view へ channel-major に複写、**(3) `inner.process(bound, &inputs, &mut outputs, events, transport)`** — `inputs = [HostInput(&view)]`、`outputs = [HostOutput(out)]`。activated 契約の gate（`min_frames ≤ frames ≤ max_frames`）は `inner.process` 側が各 variant の情報源で行う（L0 = lock 内 `SendCore.activated`、L1 = armed staging の `frame_capacity` を上限として検査・`min_frames` は child 側 plugin 契約に委譲）。**bound 系・activated 系いずれの違反も `Err(HostError::Prepare(PrepareError::UnsupportedConfig))`** — 「activated 契約外のブロック = unsupported config」で、L0 に wire は存在せず `Err(Ipc)`（transport 失敗の意味）は誤り。`Err(Ipc)` は L1 内部の「wire が表現不能なブロック」意味に留める。L0 で `activated == None` かつ mirror ∈ `{Activated, Processing}` は下記の commit 順序規則と lock 内再読みにより起こり得ない（`debug_assert` + 防御的 `Err(Lifecycle)` をバックストップ — mirror の u8 順では `Failed`=4/`Destroyed`=5 ≥ `Activated`=2 なので `≥` 比較は不変条件として使わない）。

`prepare_endpoint` は `Processor::prepare` と同じ責務境界を持つ non-RT verb — ライフサイクル（instantiate→initialize→activate→start-processing）を呼出し側が `execute` で完了した後に呼ぶ。`RtProcessor` trait 自体は `Io` 関連型が `&mut` 借用の erased bus view 列（`&[HostInput]`/`&mut [HostOutput]`）と適合しないため直接 impl せず、**同じ prepare→RT 型分離のパターンを host 語彙で踏襲**する（`process_audio` は `&RtContext` を要求し RT 区間 capability 規律を守る。`process` は `PluginHost::process` と同形のため ctx を取らない — host API の一貫性を優先）。

```rust
impl PluginHost {
    pub fn prepare_endpoint(&mut self, handle: PluginHandle, bounds: EndpointBounds)
        -> Result<PluginEndpoint, HostError>;
    // 構成面の遅延照会 — L0 = Some(0)、L1 = Some(lookahead × activated max_frames)、未 Activate = None、無効 handle = None
    pub fn latency_frames(&self, handle: PluginHandle) -> Option<u32>;
}
```

**単一 endpoint 規則**: 1 インスタンスにつき attach できる endpoint は 1 本（同じインスタンスを複数ノードが per-block 駆動する意味がなく、二重 `process` を API 形で黙認しない）。L1 は leg の移出（`Option::take` — `self.rt.is_none()` の間は `Err(EndpointUnavailable)`）で型強制、L0 は **epoch flag** `SharedSlot::attached: Arc<AtomicU64>` で拒否する。`prepare_endpoint` は host 側 `SharedSlot::next_epoch`（attach 毎に単調増・初期値 1）を採番して `attached.compare_exchange(0, epoch, AcqRel, Acquire)` — 成功なら attach、非 0 なら `Err(HostError::EndpointUnavailable)`。endpoint `Drop` は `attached.compare_exchange(epoch, 0)` で**自分の epoch のみ**を返却する — collector スレッド上の遅い async drop が後続の attach が立てた新しい epoch を掴み消すことは CAS により構造上あり得ない。ノード除去（= endpoint 退役 drop、collector スレッド）後の再 attach は可能。

**Send 証拠**: `fn expect_send<T: Send>()` 系の compile-time テスト（`#[cfg(test)]` inline — crate-private 型は `src/` 内配置・外部 `tests/` には置かない）で `PluginEndpoint`・具象 `L0Endpoint`/`L1Endpoint` 型（**enum variant 自体は型ではない**ため `EndpointInner::L0`/`L1` の variant 名は assert 引数に書かない）・endpoint を capture したノード closure（`FnMut(&mut AudioBufferViewMut<'_, f32>, &RtContext) + Send` の具体的 closure 型）・`GraphPlan`/`RtPlan`/`OutputDataCallback` コールバック capture が `Send` であることを静的に示す（AC2）。`PluginHost`/`L1Instance`/`Box<dyn ErasedPlugin>`（無印）は参照が RT 側に存在しないことを型で担保し、**負側（`!Send`）は `static_assertions` dev-dep の `assert_not_impl_any!` で `Rc`/`RefCell`/`PluginHost`/`L1Instance`/裸 `ErasedPlugin` 参照の capture を固定**する（`expect_not_send` 型関数は `!Send` を compile 時に表明できないため不適・trybuild なし）— `unsafe impl Send` は使わない。

### L0 endpoint（opt-in `Send` capability + 共有セル）

- **plugin-core 契約追加**: `PluginFactory::create_sendable(&self) -> Option<Box<dyn ErasedPlugin + Send>>`（default `None`）と `erase_sendable<P: Plugin + Send + 'static>(P) -> Box<dyn ErasedPlugin + Send>`。`Erased<P>` は `P: Send` なら `Send` — 既存の erase 構造をそのまま用い、全域 `Send` bound は掛けない（ADR 確定範囲）。非対応 factory のインスタンスは従来どおり `create()` で plain に保持され、`prepare_endpoint` は `Err(EndpointUnavailable)` で defined rejection — **L0 out-of-graph 利用を損なわない**。**`InProcessHost` の適用範囲**: v1 の endpoint 面は `PluginHost` コンポジットのみに提供する — standalone `InProcessHost` 利用者は従来どおり `plugin_mut`/`process` の直接呼出しを使い、endpoint API を L0 単独 host へ配線する要求は v1 では持たない。
- **L0 スロット**: `Instance` の plugin/context 保持を `enum` 化する — `Plain { plugin: Box<dyn ErasedPlugin>, context: ProcessContext }` と `Shared(SharedSlot)`。`Instantiate` で `create_sendable()` が `Some` なら `Shared`、なければ `create()` で `Plain`。`SharedSlot`/`SendCore` の形は次のとおり（endpoint 接面を 3 本の `Arc` + host 簿記に限定して一括に束ねる）。**命名**: 構造型は `SharedSlot` とする — `l1::Shared`（`l1/mod.rs` L249 — `Mutex<Inner>`・child・`notify_tx` を束ねる proxy state）との同名衝突を避けるためで、slot variant 名は `Shared` のまま `Shared(SharedSlot)` とする（以下「Shared スロット」はこの variant を指し、`l1::Shared` とは別物）:

```rust
struct SharedSlot {                   // Shared スロット固有の endpoint 接面 + host 側 attach 簿記
    core: Arc<Mutex<SendCore>>,       // plugin + context + activated — RT は try_lock のみ
    mirror: Arc<AtomicU8>,            // lifecycle mirror（L1 rt_state と同型）
    attached: Arc<AtomicU64>,         // attach epoch — 0 = 空き、n>0 = epoch n が attach 中
    bound: Option<EndpointBounds>,    // host 側簿記 — 最新 attach の bound（Activate 上限検査に使用）
    next_epoch: u64,                  // host 側採番器 — attach 成功毎に +1（初期値 1）
}
struct SendCore {                     // try_lock の対象 — RT 側が読む値は全てこの中
    plugin: Box<dyn ErasedPlugin + Send>,
    context: ProcessContext,          // endpoint が process_host に渡すため shared cell 内（Send 前提 — 検証対象）
    activated: Option<ProcessConfig>, // Activate commit で書込・Deactivate/Destroy でクリア
}
```

  `activated`/`bound`/`next_epoch` を `SharedSlot`/`SendCore` 内に置き `Instance` 直下の汎用 field にしない理由: endpoint attach の簿記は `Shared` スロットにのみ意味を持ち、`Plain` に dead field を残さない。`activated` は **RT 側が `try_lock` 内で読む唯一の正本**として `SendCore` 内に置く — 別の atomic 経路を増やさず、再 Activate の新値も同じ lock 経由で RT 可視になる（`Plain` スロットは endpoint を持てず `activated` を必要としない — `prepare_endpoint` が非 Send 対応を先に拒否するため）。
- **non-RT verb 経路と commit 順序（順序規則の固定）**: `Shared` スロットで `plugin`/`context` に触れる **全 verb** — lifecycle verb（`init`/`activate`/`start_processing`/`stop_processing`/`deactivate`/`destroy`）に限らず `SaveState`/`RestoreState`/`OpenGui`/`CloseGui`（`composite.rs` L593-617 の dispatch が示す通りこれらも plugin hook を呼ぶ）も含む — は **`core.lock()` を transition の合法性検査 + commit + `set_state`（mirror store）+ plugin hook の全体にまたがって保持**する（`SaveState` 等の非 lifecycle verb は transition 部分が空の同じ critical section）。mirror store は critical section 内・transition commit の後に行う — L1 の `set_mirror` が `shared.lock()` 内で `inner.mirror` 更新と `rt_state` store を一体化する形（`l1/mod.rs` L292-296、呼出し側 L1105-1114）と同じ規則。これにより **`try_lock` が観測するのは常に完全 commit 済みの状態のみ**で、「mirror だけ進んだ」half-committed 状態は endpoint から観測不能 → **`mirror ∈ {Activated, Processing} ⟹ activated.is_some()`** が endpoint 側の不変条件になる。注意: mirror の u8 順（`l1/data.rs` `mirror_encode` — `Created`0/`Initialized`1/`Activated`2/`Processing`3/`Failed`4/`Destroyed`5）では `Failed`/`Destroyed` も `≥ Activated` なので **`≥` 比較は不変条件として使わない** — 列挙で書く（`{Activated, Processing}` は `activated.is_some()` のときに限り store される値であり、`Failed`/`Destroyed` の terminal store 時点では `activated` は既に `None` または不変条件の適用外）。失敗系（hook `Err` → ロールバック遷移）も同じ lock 内で最終 state を mirror へ書く。`plugin_mut` は `Shared` スロットで `None`（`Mutex` 越しに `&mut` を貸せない — L1 スロットと同じ defined 挙動）。host 側 `process` は `try_lock` — busy は `Err(EndpointUnavailable)`（RT-suitable を維持し blocking `lock()` はしない）。**verb 経路の lock 形態と `Failed` regression の排除（順序規則の一部）**: verb 経路は `l1::Shared::lock`（`unwrap_or_else(PoisonError::into_inner)` — `l1/mod.rs` L270-275）や `PlanSwap::swap`（`plan.rs` L82）のような poison-tolerant lock を **使わない**。verb 実行列を 3 段に固定する: **(1) entry pre-check** — mirror を Acquire 読みし、`Failed` なら lock せず即座に収束（`lifecycle` へ `LifecycleEvent::Failed` + `Err(HostError::Failed)`）; **(2) `core.lock()` を `match` で分岐** — `Err(PoisonError)` なら同じ収束（poison =「lock 保持中に unwind が抜けた」痕跡 — verb CS 内の plugin hook panic が主な発生源で、endpoint の `catch_unwind` 経路では poison は作られない）; **(3) lock 内で mirror を Acquire 再読み** — `Failed` なら hook・transition・`set_state` を一切実行せず解放して同じ収束を行い、非 `Failed` なら transition 合法性検査 + commit + `set_state` + plugin hook を一続きの CS として実行する。この二段構えにより「verb の in-CS `set_state` が endpoint の `Failed` store を上書きする `Failed→Activated` regression」は構造的に不可能になる — endpoint の `Failed` store（CS 内 catch 系統 (i)・poisoned 観測系統 (ii)）は、verb の CS が非 `Failed` store を行う直前検査を必ず通る（(i) は mutex で直列化されるため verb の再読みが必ず先行・後続を正確に分岐し、(ii) は verb の lock 自体が `Err` になる）。逆に verb の非 `Failed` store が先行するのは正常列のみ（後続の endpoint `Failed` store が正しく勝つ — regression ではなく panic 後の収束）。`destroy` のみ例外: poisoned lock でも teardown を完了させるため `unwrap_or_else(PoisonError::into_inner)` で guard を取得し、**host 側の `Failed→Destroyed` transition + mirror=`Destroyed` store + core 内容物の drop のみ**を行う — panic 済み plugin の destroy hook は呼ばない（defined: hook を呼べる健全性がないため、リソース解放は destructor 経路に限定する）。**`&self` 照会点の union は observation のみ**: `InProcessHost::lifecycle_state(&self)`（`host.rs` L156）と `PluginHost::lifecycle_state(&self)`（`composite.rs` L353）は既存 PUBLIC `&self` API で、`lifecycle.transition`（`&mut`）を実行できない — したがって `&self` 照会では **`mirror == Failed` なら `Some(Failed)` を返す観測 union のみ**を行い（commit しない pure read — `lifecycle.state()` は変えない）、`LifecycleEvent::Failed` の **commit** は `execute`/host `process`/`prepare_endpoint`/`destroy` の `&mut` entry point で行う。「`Failed` への再報告は illegal」に対し、観測 union は transition を伴わないので再報告を構成しない。**`Shared`/`SharedSlot` スロットの `lifecycle_state` は `mirror` atomic の Acquire 読みのみを行い、`core.lock()` は決して取らない** — `InProcessHost::lifecycle_state` は RT-suitable/no blocking と doc 契約済み（`host.rs` L156 の RT contract 記載）・`PluginHost::lifecycle_state` は L0 slot で const-cheap（`composite.rs` L353）と規定されており、blocking lock を取る実装はその公開契約を破る。したがって `&self` 照会点の `Failed` union は **`mirror` の read-only union** である。**blocking `lock()` による serialization と wedge**: Shared スロットの制御 verb は全て `core.lock()` で直列化される（verb 同士・verb vs hook）— Plain スロットの `&mut self` 直列と等価で許容。wedged な shared-core plugin は自身の制御 verb も wedge させる — `try_lock` + retry は「遅い hook」と「本物の wedge」を区別できず新たな livelock 面を増やすため不採（Plain の wedged hook が `execute` を塞ぐ現状と等価の失敗面であり、blocking lock は non-RT verb の契約内）。
- **RT lifecycle mirror と `Failed` 収束（mirror の第 2 書き手）**: L0 に L1 の `rt_state` と同型の `Arc<AtomicU8>` mirror を追加する。`lifecycle.transition` 成功のたびに `mirror_encode(state)` を store する薄い `set_state` ヘルパで一元化（失敗系ロールバック後も最終 state を反映 — 上記の critical-section 内 store）。`mirror_encode`/`mirror_decode` は `l1::data` に残し、L0 側は `crate::l1::data::` 経路で参照する crate 内共有とする（`src/l1/` 木は `crate::`/`ruviform_plugin_host::` パス禁止の crate-free import discipline — `l1/mod.rs` L19-26・`tests/l1_proxy_e2e.rs` が `#[path]` で同木を include する制約のため crate 直下 `src/mirror.rs` へ移設すると l1 側から到達不能になる — `src/mirror.rs` は新設しない）。**mirror の書き手は host verb と endpoint の 2 系統**: (i) endpoint が自身の `process_host` 呼出しで panic を `catch_unwind` で捕捉した場合（下記 — lock 保持のまま CS 内 store）、(ii) endpoint が poisoned `try_lock` を観測した場合（主な poison 源は verb CS 内の plugin hook panic — unwind が verb の lock guard を抜けて poison 化する。endpoint 自身は `catch_unwind` で poison を作らない）、endpoint は自身で `mirror` へ `mirror_encode(Failed)` を **fetch_max**（Release）して `Ok(CONTINUE)` + 無音書込みへ収束する — L1 の「crash は制御面の事実で RT エラーにしない」規則と同じ終端で、以後のブロックは mirror 読みで short-circuit し mutex を触らない。(ii) は全 mirror store の中で **唯一の CS 外 store** — 単調増加（`fetch_max`）なので先に書かれた terminal 値を上書きしない（`fetch_max(Failed)` は `Destroyed`(5) を決して 4 へ戻さない — poisoned 観測と poisoned-lock `destroy` の `Destroyed` store が競合しても `Destroyed` が勝つ）。(i) は CS 内 store で、verb の in-CS store と同じ規則に従う。`PluginHost` は Shared スロットの照会・verb で endpoint 書込みの mirror を union して `Failed` gating に使う（`&self` 照会では上記の観測 union、`&mut` entry point では `mirror == Failed` かつ `lifecycle.state() != Failed` なら `LifecycleEvent::Failed` で収束 — `Created`/`Initialized`/`Activated`/`Processing` から合法、`Failed` への再報告は illegal のため state 確認が前提。non-RT verb の `core.lock()` が `Err(PoisonError)` を返した場合も同じ収束を行い `Err(HostError::Failed)` を返す — 上記 lock 形態規則）。
- **endpoint process 経路**: `mirror` Acquire 読み → 定義済み出力表（下表）→ `core.try_lock()` → **lock 内で `mirror` を Acquire 再読み** → `activated` gate（`min_frames ≤ frames ≤ max_frames`・`channels == bound.channels`）→ `plugin.process_host(...)`。**lock 前の mirror 読みは fast-skip のみ**（`Failed`/非 `Processing` の早期 short-circuit で mutex に触れない最適化）で、authoritative な状態判定は `try_lock` 成功後の **lock 内再読み**が担う — mirror store は全て同じ critical section 内（唯一の例外は endpoint の `fetch_max(Failed)` — 上記）なので、lock 保持中の再読みは commit 済みの最新値を必ず観測し、pre-lock 読みで得た古い値（read 後に別 verb が commit した `Failed`/`Deactivated` 等を見逃す）に依存しない。再読みの結果で同じ出力表を駆動し、`activated` gate・`process_host` 呼出しは再読みが `Processing` を示した場合にのみ行う（上記不変条件 `mirror ∈ {Activated, Processing} ⟹ activated.is_some()` は lock 内再読みの観測値に対して成立）。**`process_host` は `std::panic::catch_unwind(AssertUnwindSafe(...))` で囲んで CS 内で呼ぶ** — plugin panic が unwind として `try_lock` guard を抜けて RT callback を突き抜けることは device 契約違反（`OutputDataCallback` は panic 禁止 — audio-core `device.rs` L475-477「a panic on the backend RT thread can lead to undefined behavior or abnormal process termination」）なので、catch して mirror=`Failed`（CS 内 store — (i) 系統）+ `Ok(CONTINUE)` + 無音書込みへ収束する。`AssertUnwindSafe` は `&mut` plugin/context を跨ぐ捕捉の標準形 — hook の unwind-safety は plugin 契約側の責務として境界を明示する。`try_lock` 失敗（`WouldBlock`）は一時的 busy として `out` 不変更で返す（defined bypass）。poisoned は上記の `Failed` 収束（回復しない — panic 後の状態を RT で踏まない）。

### L1 endpoint（leg 移出 + arm 経路 + leg 返却）

- **`L1Instance.rt: Option<RtLeg>`** に変更し、`prepare_endpoint` が `take` で endpoint へ移す。detach 後の host 側 `process` は `Err(EndpointUnavailable)`（定義済み — 処理面が endpoint に移ったことを示す拒否で、wire 障害 `Ipc` と区別する）。`self.rt.is_none()` の間の再 `prepare_endpoint` も `Err(EndpointUnavailable)` — leg が `return_rx` drain で復元されるまで attach 不能。**drain 順序の固定**: `L1Instance::process` は post-attach で `Err(EndpointUnavailable)` を返す drain entry point — `return_rx`/`pending_arm` の drain（本点は RT-suitable 契約のため **`arm_collector.drain()` も `clear_endpoint_fields` も除く** wait-free 部分 — `self.rt = Some(leg)` の restore のみ。下記 `&mut` tier 規則）を **`self.rt.is_none()` の early-return より前**に実行し、early exit が drain を skip しないことを固定する。**wait-free restore は非 terminal（mirror ≠ `Destroyed`）gate** — Destroy 済みインスタンスへは復元せず、queued leg は channel ends と共に teardown で drop する bounded 経路。
- **mirror**: `Shared::rt_state: AtomicU8` を `Arc<AtomicU8>` 化し `Shared` と endpoint が clone 共有する（`set_mirror` の更新点は不変 — `Arc` 越しに同じ atomic へ store）。endpoint が保持するのは **`Arc<AtomicU8>` のみ** — `Arc<Shared>` は持たない（`Shared` は `Mutex<Inner>`・child・`notify_tx` を含み endpoint から不可触）。endpoint は mutex/Inner に触れない — **lock-free mirror のみ**を保持する設計を維持。
- **arm 経路（d の確定 — 5 点の固定）**: `Activate` 成功時に作られる `Arc<Staging>` を non-RT → RT へ届ける経路。
  1. **channel は per-attach で鋳造**: `prepare_endpoint` が `channel::<Arc<Staging>>(capacity 2)` の pair を作り、現在の `Inner::staging`（attach 時点で `Activate` 済みなら存在）を clone して **seed arm として `try_push`** — endpoint は常に arm 経由で ring を得る（staging install 点を `arm_apply` 一本に統一）。**seed arm の apply は generation 規則に従う — 常に no-op ではない**: seed arm は **`seed.generation > leg の現装着 staging.generation` のときのみ apply され、generation が一致する場合に限り正規化 no-op（apply されず unapplied のまま retire）** となる。attach は `Activated`/`Processing` を要求するため detach される leg は既に arm 済み（`Activate` 時点の `self.rt.arm` または先行 arm 経路で装着）で、leg は装着中の `staging`/`frame_capacity`/seq 簿記を**そのまま保持して移動**する（clear しない）— 通常経路（`Activate`→`prepare_endpoint`）では seed arm は同じ generation（`Inner::staging` の clone = leg の装着 ring と同一 `Arc`）を持つため endpoint の `generation > 現装着` 判定を通らず retire される。**detached-Activate**（attach 生存中の `Deactivate`→`Activate` で `Inner::staging` が新 generation へ進んだ場合）の自己修復は **host 側 non-RT drain が担う** — leg が `return_rx` drain で還流した際、復元 leg の装着 generation が `Inner::staging` より古ければ `rt.arm(inner.staging)` で re-arm する（そのままでは以後の submit が全て driver 側 `armed_for` で drop され恒久的 pass-through になるため）。wait-free restore から次の `&mut` drain までの一過性 stale submit は pass-through で自然回復する。還流後の再 attach の seed は常に同 generation の no-op で、endpoint 側 `generation > 現装着` の apply が実際に新 ring を装着するのは endpoint 生存中に arm channel へ届いた arm のみ。seed の存在意義は「attach 後の install 変更経路を `arm_apply` 一本に揃える」正規化で、通常経路の初回装着状態を作るものではない。
  2. **arm 送信**: `Activate` 成功時に control が `try_push`。`Full` なら `pending_arm: Option<Arc<Staging>>` へ退避 — **latest-wins** で新しい arm が退避中の古い arm を置き換える（staging は世代累積なので中間世代の喪失は問題にならない）。
  3. **再送点（`&mut` tier — `&self` query は defined non-drain）**: `pending_arm` の再送試行・`return_rx` の drain（leg 復元）・arm-`DropCollector` の drain は **`&mut` entry point** の冒頭で行う — `L1Instance` 面では `execute`・`process`（wait-free 部分のみ — 下記）・`prepare_endpoint`・`destroy`/teardown、`PluginHost` 面ではこれに `try_recv_notification(&mut self)`（`composite.rs` L433 — slot を `iter_mut` で触るため `L1Instance::drain_endpoint_channels` を呼べる）が加わる（`try_push`/`try_pop` は wait-free で RT-suitable 契約を壊さない）。**`&mut` 制約の理由と interior-mutability の不採**: `try_push`/`try_pop`/`retire`/`drain` は全て `&mut` シグネチャ（`spsc.rs` L91・L183、`retire.rs` L157・L251）であり、`L1Instance::lifecycle_state`/`try_recv_notification` は `&self`（`l1/mod.rs` L1233・L1264）、`PluginHost::lifecycle_state(&self)` は既存 PUBLIC API（`composite.rs` L353）、本書が pin する `latency_frames` も `&self` なので、これらの `&self` query 点では drain/resend は行えない — **defined non-drain observation point とする**（観測のみ・leg 復元先 `self.rt` の更新自体も `&mut` が要るため、`&self` で pop しても置き場がない）。代替案の検討: (i) `return_rx`/`pending_arm`/`arm_tx`/`DropCollector` を `Shared` の `Mutex<Inner>` 内に移す案は **不採** — drain 対象の host 側 `process`（`L1Instance::process` — RT-suitable: 「no locks」契約）が blocking lock を取れず、`DropCollector::drain` の destructor 実行も RT 契約上その経路では不可で、一様な「全 entry point drain」はどのみち実現できない; (ii) 当該メソッドを `&mut` 化する案も **不採** — 既存 PUBLIC `PluginHost::lifecycle_state(&self)`/`InProcessHost::lifecycle_state(&self)`（`composite.rs` L353・`host.rs` L156）のシグネチャを変えてしまい、leg 復元は結局別の `&mut` 点が要るので益がない。`&mut` entry point は `L1Instance::drain_endpoint_channels(&mut self)`（`pending_arm` resend + `return_rx` pop → `self.rt` 復元 + `arm_collector.drain()` + leg 復元の仕上げ — 復元 leg が `Inner::staging` より古い generation なら `rt.arm` で re-arm し、per-attach field `arm_tx`/`pending_arm`/`return_rx`/`arm_collector`/`endpoint_bound` を全て clear）を冒頭で呼ぶ共通形とし、RT-suitable な host 側 `process` のみ **wait-free 部分**（`try_push` resend + `try_pop` → `self.rt` 復元 **のみ** — `arm_collector.drain()` も `clear_endpoint_fields` も含まない: 復元は destructor ゼロで、stale channel ends は bounded で inert — non-RT drain が片付ける。destructor 実行は RT 契約違反）を行う。verb 系のみの再送だと verb が来ない間 endpoint が旧 ring に滞留するため、滞留窓は「次の `&mut` entry point」までに圧縮される（`&self` query だけが呼ばれ続ける運用では resend は行われない — leg・arm は喪失せず次の `&mut` 呼出しで必ず処理される defined 挙動）。
  4. **generation 単調 apply**: endpoint は `process` 冒頭で `arm_rx` を drain し、`arm.generation > 現装着 staging.generation` の arm のみを `arm_apply` で apply する（`staging_generation` は再 Activate で単調増 — 決して regress しない）。それ以下の arm は未装着のまま `arm_retirer.retire()` し、apply した arm への差替で退役する旧 ring も同じ `Retirer` へ — **drain した arm は適用・不適用（superseded）を問わず全て `Retirer` 経由**で non-RT 側 `DropCollector`（`L1Instance` 保持・上記 `&mut` entry point tier のうち RT-suitable 契約を持たない点 — `execute`/`try_recv_notification`/`prepare_endpoint`/`destroy`-teardown — で `arm_collector.drain()`）へ送る。**staging の dealloc が RT 上で起きない**ことが型で担保される。
  5. **capacity**: arm channel = **2**、endpoint `Retirer` = **4**。根拠: 1 drain pass の retire 数は **worst ≤3** — drain ループの pop 回数は 1 pass あたり `arm_capacity + 1 = 3` に上限化する（`Consumer::pop_latest` と同じ bounded-loop 規則 — `spsc.rs` L218-227 の `for _ in 0..capacity` 先例。pass 開始時に queue 済みの arm ≤ 2 + drain 中に producer の `try_push` が完了し得る racing push ≤1 を構造的にカバーし、producer 速度に依らず上界が成立する）。各 popped arm が退役させる ring は **≤1** — 不適用（superseded/stale）なら arm 自身、適用なら差替で外れる装着 ring（装着 ring の退役は pass 内最後の適用 1 回のみ、superseded intermediate は arm 自身の retire として計上済み）→ retire/pass ≤ 3 に対し capacity 4 は余裕 1 で維持する。`ruviform_rt::slot`（snapshot）を採らなかった理由: 初回 `read()` の per-thread debt-node alloc が RT 初回適用ブロックで発生し warm-up 例外が要るのに対し、SPSC+Retirer は全操作が wait-free で例外ゼロかつ既存 data plane と同形。
- **`arm_apply` の簿記分割（no-alloc 化）**: 現行 `RtLeg::arm`（`data.rs` L491-506）は `last_output` を `resize` で伸長し得る上に `last_pushed`/`cleared`/`held`/`submit_dims` をリセットする — そのままの RT 適用は alloc と in-flight 不変条件の両方で不可。endpoint 向けに `RtLeg::arm_apply(&mut self, arm: Arc<Staging>, retirer: &mut Retirer<Arc<Staging>>)` を新設し、**ring 束縛の状態だけをリセットする**: `staging` 差替・`frame_capacity` 更新・`last_output` 中身のクリア＋`resize(frame_capacity × WIRE_MAX_CHANNELS)`（**capacity 事前確保済みで realloc しない** — 下記）・`last_frames`/`last_channels`/`have_output` リセット・旧 `staging` を `retirer` へ。一方、**seq→shm-slot 空間を追う `last_pushed`/`cleared`/`held`/`submit_dims`/`next_block`/`depth`/`lookahead` は arm を跨いで持続する** — 旧 ring 下で submit 済み・done 未着の block が残り得るため、`cleared[i] ≥ last_pushed[i] ⇒ slot i は空` の strict-alternation 不変条件を維持し、stale な done が再 arm 後の seq と alias しない（`next_block` の単調性は現行 doc どおり）。`last_output` は `prepare_endpoint`（`RtLeg` 移出時の non-RT 点）で `bound.max_frames × WIRE_MAX_CHANNELS` へ `reserve` — `frame_capacity ≤ bound.max_frames` が Activate 時 gate（下記）で保証されるため `arm_apply` の `resize` は常に capacity 内。`staging.frame_capacity > bound` の arm は control 側が `Activate` 時に `Err(Prepare(UnsupportedConfig))` で拒否するのを本線とし、endpoint 側の guard（`frame_capacity > bound` の ring は apply せず retire + `leg.pass_through(outputs)` + `Ok(CONTINUE)` で収束 — `Err(Ipc)` を公開面に出さない）をバックストップとする。
- **leg 返却 channel**: endpoint `Drop` で `leg.take()` した `RtLeg` を `return_tx.try_push`（capacity 1）で host へ返す。`Producer::try_push` の失敗形は `TryPushError::Full` のみで **disconnected variant は存在しない**（`spsc.rs` L294-299 — enum は `#[non_exhaustive]` で単一 variant `Full`）— `Full` は channel に先行 leg が残る場合のみで、`into_inner()` で回収した leg は **endpoint を drop したスレッド上でそのまま drop** される。「`Full` のみ」の pin は版固定の事実として扱い、`#[non_exhaustive]` による将来の variant 追加への備えは 2 点で担保する: (a) `TryPushError::into_inner` 自身が内部で exhaustive `match`（`spsc.rs` L315-319 — `Self::Full(value) => value` のみ）を持つため、新 variant 追加はまず ruviform-rt 内で compile error になり API 意味の再決定を強制する — leg 返却側が黙って意味変化を踏むことはない、(b) leg 返却経路は `try_push` の `Err` を `into_inner()` 経由で回収する unit test を pin し、「Full 回収 → drop スレッド drop」の契約を直接検証する。正しさの根拠: (i) endpoint の drop は必ず non-RT — plan 退役 offload が collector スレッドへ逃がし（AC8）、leg 内の dealloc が RT 上で走らない、(ii) `L1Instance` の **`&mut` non-RT entry point**（上記 tier 規則 — `execute`/`try_recv_notification`/`prepare_endpoint`/host `process`/teardown）が `return_rx.try_pop` を drain し `self.rt` を復元するので、**plan swap が一度も走らない session でも `&mut` entry point 呼出し毎に leg が還る** — `&self` query 点（`lifecycle_state`/`latency_frames`）は defined non-drain だが、attach の根拠となる `prepare_endpoint` 自身が `&mut` で drain するため再 attach が不定に塞がれることはない、(iii) attach 可否の判定は leg 実体の返却（`Option::take` の復元）が唯一の根拠 — L0 の epoch flag のような代用 flag は L1 側に持たないため、leg の帰還経路は `&mut` entry point drain で担保する。leg を喪失した場合（host 側 channel endpoint が既に drop）はそのインスタンスは再 attach 不能の defined 終端 — instance teardown が先行する場合に限る。
- **driver/reader・segment は host に残る**: endpoint は submit/result channel endpoint + `Arc<AtomicU8>` mirror + arm/retire/leg 返却経路のみを持ち、`Shmem`（`!Send`）・child・I/O スレッド・`notify_rx` は `L1Instance` 側に留まる。`Destroy`/teardown は従来どおり — mirror を終端 state（`Destroyed`/`Failed`）へ書き、channel 切断後の endpoint は定義済み無音/pass-through へ収束する。

### グラフ接着（f/g の確定）

- **engine 側シーム**: `ruviform-audio` は `AudioNode` に **`FnMut` blanket impl** を 1 本追加する。

```rust
impl<F> AudioNode for F
where F: FnMut(&mut AudioBufferViewMut<'_, f32>, &RtContext) + Send {
    fn pull(&mut self, out: &mut AudioBufferViewMut<'_, f32>, ctx: &RtContext) { (self)(out, ctx) }
}
```

  `Send` closure がそのまま `Box<dyn AudioNode>` として `graph.push` できる — named node newtype（`FnNode` 案）は同一意味で儀式的なため不採。`Send` でない capture を持つ closure はコンパイル時に弾かれ、`Send` 契約が接着点そのものになる。
- **バッファ写像（g）**: `process_audio` は `out` の内容を endpoint 内 `in_scratch`（`bound.channels × bound.max_frames` で事前確保）の先頭 `ch × fr` 領域へ channel-major で複写し、per-block view `AudioBufferViewMut::try_new(&mut in_scratch.data_mut()[..ch*fr], ch, fr)` を `HostInput(&view)` として、`HostOutput(out)` と組んで `process` へ渡す。**transform 意味**: ノードは `out` の上流内容を入力として処理し結果を `out` へ書き戻す（source 的 plugin は入力を無視して上書き — graph の「sources overwrite, transforms modify in place」に整合）。
- **source-position の Err 意味（adapter 契約の使い分け）**: `process_audio` の `Err` は「`out` 不変更」のみを意味し、transform 位置では上流内容の bypass になる。**source 位置（上流入力を持たない・無視する）のノードでは、bypass が残す上流内容は stale になり得るため、closure が `process_audio` 呼出し前に `out.data_mut().fill(0.0)` で `out` をゼロ化する**のを adapter 契約の必須慣行（document-required convention）とする — `Err`/bypass 時は stale でなく無音が出る。`Ok` 時は plugin が `out` を上書きするのでゼロ化の実害は bounded memset の固定コストのみ。v1 はこの慣行のみで node-level の role flag は設けない（`FnMut` ノードへ role メタデータを増やす API 面に対し consumer 側 1 行で足りる — YAGNI）。
- **plugin-core の view impl**: `impl BufferView for AudioBufferView<'_, f32>` + `impl BufferView/BufferViewMut for AudioBufferViewMut<'_, f32>` を追加（`processor.rs` の `PlanarBufferView*` impl と同じ seam・channel-major なので `channel(i) = &data[i*frames..]` の const 実装）。self-referential な `&[&[f32]]` 構成を避けられ、copy・alloc なし。
- **consumer 配線（非 RT・全て構築時）**:

```rust
// 構成（全て non-RT）:
host.register_factory(factory);                       // or register_out_of_process(spec)
let h = host.execute(HostCommand::Instantiate { plugin_id })?;
for cmd in [Initialize, Activate { config }, StartProcessing] { host.execute(cmd)?; }
let mut ep = host.prepare_endpoint(h, EndpointBounds { max_frames, channels })?;
// グラフ配線:
graph.push(Box::new(move |out: &mut AudioBufferViewMut<'_, f32>, ctx: &RtContext| {
    let _ = ep.process_audio(out, EventList::empty(out.frames() as u32), None, ctx);
    // Err は定義済み非出力 — out はそのまま（上流信号の bypass）
}));
```

### lifecycle/preparation と bounded 検証

- **prepare シーケンス**: consumer は `Instantiate`→`Initialize`→`Activate { ProcessConfig { sample_rate, min_frames, max_frames } }`→`StartProcessing` を `execute` で完了後に `prepare_endpoint` を呼ぶ。`Activated`/`Processing` 以外の state では `Err(Lifecycle)` — 未 prepare 状態の RT 利用は型・gate 双方で排除される。
- **bounded 検証（`prepare_endpoint` — 全て `Err(Prepare(UnsupportedConfig))` で defined rejection）**: `bounds.max_frames == 0`・`bounds.channels == 0` は無効 config として拒否。`bounds.max_frames` は activated `max_frames` 以上必須（未満は同 variant — wire 上限拒否と同じ族）。`bounds.channels` は **v1 では activated メイン bus との prepare-time 等値を要求しない**（修正記録参照 — plugin の channel query が存在しないため照合不可能）。代わりに (i) `bounds.channels` は非 0・`≤ WIRE_MAX_CHANNELS`、(ii) per-block の `out.channels() == bound.channels` を `process_audio` の bound clip（step 1）で強制（不一致は同 `Err(Prepare(UnsupportedConfig))` — `out` 不変更）、(iii) `in_scratch` は `bound.channels × bound.max_frames` で bound-shaped に事前確保する、の 3 条件で形状を拘束する（暗黙の padding/truncation は採らない — 黙った形状変換を避け caller が bound を宣言して整合させる）。**実 channel 照会・activated bus との reconcile は Issue #114 系の deferred contract work**（真の channel query 導入後に prepare-time 整合を再評価 — 多 bus・padding は v1 範囲外の deferred 論点）。cap は**両層共通**で `bounds.channels ≤ WIRE_MAX_CHANNELS`（16）・`bounds.max_frames ≤ WIRE_MAX_FRAMES`（8192）を endpoint 上限とする。L0 への `max_frames` cap 拡張の根拠: `in_scratch` は `bound.channels × bound.max_frames` で事前確保する **caller-controlled alloc** であり、cap なしでは u32::MAX 級の巨大確保（16ch × u32::MAX × 4B ≈ 274 GB → alloc failure abort）を API 面で許してしまう — L0 に wire 上限の必然こそないが、(i) `EndpointBounds` は「wire 上限」ではなく scratch 供給契約であり語彙・上限値を層を跨いで同一に保つ、(ii) scratch の裏付けは `AudioBuffer::try_new`（audio-core `buffer.rs` L212-217 — `checked_slab_len` が usize 桁溢れ・slab バイト数 `isize::MAX` 超過を `Err(InvalidValue)` で弾く shape check）で、その shape-Err は boundary で `Err(Prepare(UnsupportedConfig))` へ写像する（constructor が shape で panic しない）— cap は「上限内だが巨大すぎる確保」への唯一の defined rejection として両層に適用する（cap 後の最大 scratch は 8192 × 16 × 4B = 512 KiB）。副次効果: consumer 配線の `EventList::empty(out.frames() as u32)`（`event.rs` L192 — `block_frames: u32`）の usize→u32 縮退窓も閉じる — 受理ブロックは常に `frames ≤ 8192` となり `as u32` が値を変えない。
- **per-block gate**: `process_audio` の bound clip（`channels == bound.channels`・`frames ≤ bound.max_frames` — scratch slice の前提検査）の後、`inner.process` が **activated 契約**を検査する: `activated.min_frames ≤ frames ≤ activated.max_frames`・channels 一致（L0 は lock 内 `SendCore.activated` の `min_frames`/`max_frames`/channels、L1 は armed staging の `frame_capacity` を上限 + `bound.channels` を等値として検査 — `min_frames` は armed `Staging` が保持しないため child 側 plugin 契約に委譲する deliberate divergence）。**gate する `frames`/`channels` の情報源を固定**: `block_data` と同じ **in-bus-first 規則**（`data.rs` L630-631 — `frames` は `inputs.first()` の `frames()`、in-bus が無ければ `outputs.first()` の `frames()`; `channels` は両 bus 存在時 `min(in,out)`、片側のみなら存在側）を使う — `bound` は上限定数の供給のみで gate の寸法情報源ではない。`process_audio` 経由では `out` が唯一の bus で、`out.frames()`/`out.channels()` がそのまま使われる（in-view は `out` と同寸法で組まれるため in/out で一致する）。違反は **`Err(HostError::Prepare(PrepareError::UnsupportedConfig))`** — panic/UB/alloc なし。`Err(Ipc)` は「wire が表現不能なブロック」の L1 内部意味に留め、endpoint 公開面の bound 違反には使わない（L0 に transport は存在しない）。
- **再 Activate**: endpoint attach 済みインスタンスへの `Deactivate`→`Activate` は、attach flag が生存中（L0 = `attached` epoch ≠ 0、L1 = leg 移出中）かつ `new_config.max_frames > bound.max_frames` のとき `Err(Prepare(UnsupportedConfig))` で拒否（scratch 契約の維持 — 比較対象は attach 時に `SharedSlot::bound`/host 簿記へ保持した bound）。bound 内の再 Activate は L1 で arm 経路（generation+1 の `Arc<Staging>`）により新 ring が endpoint へ届き、L0 は `SendCore.activated` の差替が同じ `core.lock()` 経由で RT 可視になる。

### エラー・拒否面（定義済み拒否の一覧）

| 条件 | 結果 |
| --- | --- |
| `prepare_endpoint` — state が `Activated` 未満（`Created`/`Initialized`） | `Err(HostError::Lifecycle(_))`（`Failed` インスタンスは既存規則どおり `Err(HostError::Failed)`、`Destroyed`/stale handle は `Err(InvalidHandle)`） |
| `prepare_endpoint` — L0 非 Send 対応・2 本目の endpoint・L1 leg 未返却 | `Err(HostError::EndpointUnavailable)` — **新 variant**（`#[non_exhaustive]` で non-breaking。**local-only variant と宣言**: wire 経路には乗らない — `encode_host_error` を `Result<(), WireError>` 化し wildcard arm で `Err(WireError::UnsupportedVariant)` を返す（`encode_transport` の non-exhaustive variant 先例 `wire.rs` L1260-1269 と同形。wildcard で `Ipc`/`Unsupported` へ潰さない — host-local の topology 事実を transport/plugin 起因と誤記させないため）。**伝播のため `pub fn encode`（`wire.rs` L650）も `Result<(), WireError>` 化する** — `Message::Reply` の `Err` arm は `REPLY_ERR` tag を `encode_host_error` 呼出し**前**に push する（`wire.rs` L676-677）ため、encoder 単体の fallible 化では partial frame が残り伝播不能 — `encode` は失敗時 `out.truncate(start)` で追加済みバイトを巻き戻す **all-or-nothing append 規則**とする（partial frame は残さない）。`encode_reply`（`wire.rs` L889）は `HostError` に到達しないため **infallible のまま**（`Err` arm 内の `encode_host_error` のみが `?` を要求）。`encode` は公開 API で **signature 変更の開示**が要る — pre-1.0 として許容（`EndpointUnavailable` を `Err(UnsupportedVariant)` で正しく返すための最小変更）。代替案 — encode 前に `is_wire_encodable(error)` pre-tag 検査＋partial-frame 規則 — は variant→encodable 知識を encoder と predicate で二重化し `#[non_exhaustive]` 追加時に drift するため不採（fallible encoder は variant 表の単一権威を維持）。`Display` への arm 追加と roundtrip 用手動 variant リスト（`wire.rs` L1611-1627・L1958-1971 — wire 表現可能 variant のみが対象）の更新が必須。test 更新: `roundtrip` ヘルパ（`wire.rs` L1500-1503）は `encode(...)` の戻りを `unwrap` し、`field_roundtrip`（`wire.rs` L1505 — `encode: impl Fn(&T, &mut Vec<u8>)` の infallible 形は維持）は `host_error` 側 call site を `|e, buf| encode_host_error(e, buf).unwrap()` で wrap、`Reply{Err(EndpointUnavailable)}` の `Err(UnsupportedVariant)` + `out` 不変（truncate 規則）を assert する negative case を追加） |
| `prepare_endpoint` — `bounds` 非 0 検査（`max_frames`/`channels`）・`bounds.max_frames < activated`・cap 超過（`channels > WIRE_MAX_CHANNELS`・`max_frames > WIRE_MAX_FRAMES` — 両層共通・`u32::MAX` 級も同 variant — v1 では `channels` の activated bus 等値は要求しない: channel query 未存在のため。per-block `out.channels() != bound.channels` は `process_audio` bound clip で同 variant — `out` 不変更） | `Err(HostError::Prepare(UnsupportedConfig))` |
| endpoint `process` — mirror が `Failed`（lock 前 fast-skip または lock 内再読みで観測）・poisoned `try_lock`・`catch_unwind` が `process_host` panic を捕捉 | `Ok(CONTINUE)` + **無音書込み**（終端条件 — L1 と同じ規則。L0 endpoint は自ら mirror=`Failed` を `fetch_max` store する第 2 の書き手 — 以後 mirror 読みで short-circuit） |
| endpoint `process` — mirror が `Processing` 以外 | `Err(HostError::Lifecycle(_))` → ノードは `out` 不変更（上流 bypass） |
| endpoint `process` — L0 `try_lock` busy（non-RT verb 保持中） | `Err(HostError::EndpointUnavailable)` → `out` 不変更の一時 bypass — ハングしない |
| endpoint `process` — block が bound 超過/channels 不一致（bound clip）・activated gate 違反（L0 = `activated.min_frames ≤ frames ≤ activated.max_frames`・channels 一致、L1 endpoint gate は `frames ≤ armed staging.frame_capacity` + `channels == bound.channels` — `min_frames` は armed `Staging` が保持しないため child 側 plugin 契約に委譲する deliberate divergence。`frames > frame_capacity` 検査は現行 `block_data` と同じ upper-bound 規則 — `data.rs` L632-636） | `Err(HostError::Prepare(UnsupportedConfig))` → `out` 不変更（`Err(Ipc)` は L1 内部の wire 表現不能に留め公開面には出さない） |
| endpoint `process` — L1 結果未着/slot busy | L1 規則の pass-through（直前受信 else 無音）— エラーでなくフォールバック |
| host 側 `process` — L1 detached leg・L0 shared busy | `Err(HostError::EndpointUnavailable)` |
| non-RT verb — L0 SharedSlot の entry pre-check/lock 内再読みで mirror=`Failed`、または `core.lock()` が `Err(PoisonError)` | `Err(HostError::Failed)` + `LifecycleEvent::Failed` 収束（endpoint 収束と同じ `Failed` union — 既存「`Failed` 以後は `Destroy` のみ」規則へ収束。`destroy` のみ `into_inner` で teardown を継続 — 上記 lock 形態規則） |
| `Failed` 以後のコマンド（`Destroy` 除く） | `Err(HostError::Failed)` — 既存規則不変・自動再起動なし |
| `Destroy` — endpoint attach 中 | 合法（lifecycle 規則内）。L0 共有セル/plugin オブジェクト・L1 leg は endpoint の退役 drop（collector スレッド）で解放 — RT drop なし |
| arm channel `Full` | `pending_arm` 退避（latest-wins）・**`&mut` non-RT entry point** で再送（`&self` query は defined non-drain — 上記 tier 規則） — endpoint は旧 ring で定義済み pass-through を継続 |
| endpoint `Drop` — L1 leg 返却 `try_push` が `Full` | `into_inner()` した leg は drop スレッド上でその場 drop（drop は必ず non-RT — plan 退役経路）。再 attach は次の `return_rx` drain で復元される leg が前提 |

**ノード側規則（adapter 契約）**: `process_audio` の `Err` は全て「`out` 不変更 = 上流内容の bypass」を意味する。`Ok` で書き込まれるのは plugin 出力・L1 pass-through・終端無音のいずれか — 「書き込まなかった」ことを panic や部分的書込みで表さない。

### 遅延報告（i の確定）

- `PluginHost::latency_frames(handle) -> Option<u32>` — L0 は `Some(0)`、L1 は `Activate` 済みなら `Some(lookahead_blocks × activated max_frames)`（spec 修正記録の frames 式）、未 `Activate`/無効 handle は `None`。
- `PluginEndpoint::latency_frames() -> u32` — prepare 時点の値を保持（re-activation で変わり得ることは doc に明記）。
- 秒換算は `latency_frames / sample_rate`（既定 2 × 512 @ 48 kHz ≈ 21.3 ms）— frames 整数で露出し f64 秒にしない（baseline の時刻原則に整合）。`OutOfProcessSpec.lookahead_blocks` は既存どおり登録時の選択面。

### 退役シーム（e の確定 — `plan.rs` 最小修正）

現行の穴: `PlanSwap::swap` の RwLock 内 `Arc` は writer スレッド（non-RT）で drop するが、`RtPlan.cached` の旧 `Arc` は RT 側 `active()` で差替時に drop — **最後の `Arc` なら `GraphPlan`（全ノード + endpoint + plugin リソース）の Drop が RT スレッドで走る**。snapshot-swap ADR は「退役 Plan の Arc は非 RT 側が保持して自然 drop」を期待しており、型で強制する機構がなかった。

- `RtPlan` に `retirer: Retirer<Arc<GraphPlan>>` を持たせ、`active()` で `cached` を差し替える際 **plan が実際に変わったときのみ**（`Arc::ptr_eq` で比較 — 毎ブロック retire しない）旧 `Arc` を `retirer.retire(old)` する。
- `impl Drop for RtPlan` で `cached.take()` を同じく retire — `RtPlan` 自身が RT 上で drop されても plan のデストラクタは走らない（`retire` は wait-free で RT-suitable）。
- `PlanSwap` は `DropCollector<Arc<GraphPlan>>` を `Arc<Mutex<…>>` で保持（`#[derive(Clone)]` と公開 API 形を維持）。**drain は `PlanSwap`/`Session` の全 non-RT entry point で試みる** — `swap` は write のたびに `collector.drain()` を呼び、別途 `drain_retired() -> usize`（低頻度・swap なし運用での明示 drain 点）と `retired_overflows() -> usize`（leak-on-full の飽和を観測する counter — 非 0 や増加は capacity/drain cadence の undersize を示す運用 signal。notification 化は v1 ではせず **counter 確認を AC で要求**する）を公開する。`PlanSwap::new` は既定 capacity（小 — swap 頻度比例）、`with_retire_capacity` で調整可能に。
- **endpoint drop → leg 返却の cadence 契約**: endpoint の退役 drop は必ず collector スレッド等の non-RT スレッドで起き（plan 退役 offload が AC8 を機構で担保）、L0 は `attached` の epoch CAS で即時 flag 返却、L1 は leg を `return_tx` へ push する。`L1Instance` の **`&mut` non-RT entry point**（`execute` verb 系・`try_recv_notification`・`prepare_endpoint`・`destroy`/teardown — `PluginHost` 面の `try_recv_notification(&mut)`（`composite.rs` L433）を含み、RT-suitable な host `process` は wait-free 部分のみ）が `return_rx` と arm-`Retirer` 側 `DropCollector` を drain するため、plan swap が一度も走らない session でも `&mut` entry point 呼出し毎に leg が還り再 attach が不定に塞がれない — 「退役 drain は swap だけに依存しない」が契約。`&self` query 点（`lifecycle_state`・`latency_frames`・`L1Instance::try_recv_notification`（`l1/mod.rs` L1233・L1264 — `&self`））は defined non-drain observation point — `&mut` プリミティブ（`spsc.rs` L91・L183・`retire.rs` L157・L251）に触れられず、`self.rt` 復元も `&mut` が要るため drain しても置き場がない。interior-mutability（`Mutex<Inner>` 内への channel 移設）は host `process` の RT-suitable/no-locks 契約と両立しないため不採（上記 tier 規則）。
- 効果: 退役 `GraphPlan` の最終参照がどこで剥がれても（RT の cached 差替・writer の RwLock 差替・Session teardown）**node/endpoint/`Mutex<SendCore>`/`RtLeg` scratch/`Arc<Staging>`/SPSC endpoint の dealloc は全て collector スレッドか collector 自身の drop（常に non-RT）で行われる** — AC8 を機構で担保する。node removal は新 `GraphPlan` + `swap` と同型（v1 グラフは順序リストのまま・in-place 除去 API は持たない）。
- teardown 順序: `Session` の field 順で `stream`（callback + `RtPlan`）→ `swap`（collector）の順に drop — retirer より collector が後に死ぬ安全順。channel に残った退役 `Arc` の destructor は最終 endpoint drop スレッド（non-RT）で走る（retire module の teardown 規則どおり）。

### video NONRT 境界（k の確定）

- `VideoPluginInstance`/`VideoProcessConfig`（plugin-core `video` モジュール）は **全操作 NONRT** — `prepare_endpoint`/`PluginEndpoint` の対象は audio `process` ライフサイクルを持つインスタンスのみであり、video plugin を RT パスへ接続する経路は**持たない**。
- OFX ホストアダプタ（`ruviform-plugin-ofx`）は NONRT L0 adapter であり、本 Issue で engine 接続経路は作らない。video engine 側への接続形は第 2 ターゲット統合の論点として deferred（trigger: NONRT video port の分離要求 — L1 design の deferred 行と整合）。
- **L1 video wire は確立しない** — wire プロトコル・shm スロット・runner は audio `process` 専用のまま。

### 依存追加の判断（暫定規約の理由記録）

| 依存 | 採否 | 理由 |
| --- | --- | --- |
| `ruviform-audio-core`（plugin-host の正式 dep 昇格） | 採用 | Domain foundations 層で baseline A.2 許可範囲。`process_audio` の `AudioBufferViewMut` 型を名指すために必要。plugin-core 経由の推移依存を直接化するのみで依存木は不変 |
| `ruviform-audio`（plugin-host の dev-dep） | 採用（テスト専用） | graph E2E の置き場所。prod 依存は非循環・`ruviform-audio` の Cargo.toml/公開 API に plugin 型・依存の露出ゼロ（AC3 の厳格解釈） |
| `ruviform-rt` `channel`/`retire_channel` | 採用（再利用） | arm/退役経路は新規機構を作らず RT 検証済みプリミティブを流用 |
| `ruviform-rt` `slot`（snapshot） | 不採用（arm 経路・plan 退役とも） | 初回 `read` の per-thread debt-node alloc が RT 初回適用ブロックに発生し warm-up 例外が要る + `PlanSwap` の `Clone`/公開形の維持コスト。SPSC+Retirer で同等の不変条件を例外なしで満たせる |
| `static_assertions`（plugin-host + audio の dev-dep） | 採用（テスト専用） | Send 証拠の負側（`!Send`）を compile 時に表明する `assert_not_impl_any!`・正側の `assert_impl_all!`（generic `F: FnMut` に対し `expect_send` 型関数では証明不可）のため — dev-dep のみで prod 依存木・公開 API は不変 |
| 新規外部依存（prod） | なし | `static_assertions` は dev-dep のみ — `[dependencies]`・依存木への追加はなし |

### 検証戦略（mock/fixture — device-independent）

| 層 | fixture | 検証 |
| --- | --- | --- |
| `ruviform-plugin-host` 単体/`rt_` | Send-capable mock plugin（`create_sendable` 実装する test factory — gain・DropFlag probe）・`RtLeg`（child なしで unit 検証可能な既存構造） | `rt_endpoint_l0_process_no_alloc`（shared core `try_lock` 経路が `assert_no_alloc` 窓内 0 alloc）・`rt_endpoint_l1_process_no_alloc`・`rt_endpoint_arm_apply_no_alloc`・`rt_endpoint_busy_bypass`（busy/poisoned/非 Processing の定義済み出力）・`expect_send::<PluginEndpoint>()` |
| `ruviform-audio` 単体/`rt_` | DropFlag モックノード + `PlanSwap` | `rt_plan_active_no_alloc`（`active()` try 経路 0 alloc）・`rt_plan_swap_retire_no_alloc`（差替で旧 `Arc` が retire され RT drop しない — flag が drain まで不変）・`rt_fn_node_pull_no_alloc` |
| `ruviform-plugin-host/tests/` E2E（dev-dep `ruviform-audio`） | test `Device`/`OutputStream` double（callback を手動駆動する device-independent mock）+ Send-capable gain plugin | **L0 graph E2E**: register→lifecycle→`prepare_endpoint`→closure push→`Session::open`→callback 駆動で per-block `process` + gain 正当性（AC1/AC9）・**構成面**: `latency_frames` == 0・`EndpointUnavailable`/`DuplicatePluginId` rejection（AC3/AC4） |
| 同上 | `src/bin/l1-test-worker`（`CARGO_BIN_EXE_*`）を `register_out_of_process` で spawn | **L1 graph E2E**: spawn→lifecycle→`prepare_endpoint`→graph→process→destroy（AC1/AC9）・`kill_child` → `Failed` + `InstanceFailed` once 通知 + 以後無音/pass-through（AC5）・`latency_frames == lookahead × activated max_frames`（AC6）・遅延未着 pass-through |
| 退役観測 | DropFlag probe を plugin/scratch に内蔵 | node removal/plan-swap で probe の Drop flag が RT コールバック経路では立たず `drain_retired` で立つ（AC8）・`retired_overflows()` が drain 後も 0 であること（overflow の observable signal を AC で検査）・L1 leg 返却 channel で再 attach 可能（swap なし運用でも verb drain で復元） |
| `Send` 証拠 | compile-time bound テスト（`#[cfg(test)]` inline `expect_send` + `static_assertions` dev-dep） | `PluginEndpoint`・具象 `L0Endpoint`/`L1Endpoint` 型（enum variant 名は assert 引数にしない）・ノード closure 型・`GraphPlan`/`RtPlan`/コールバック capture の `Send`（AC2）・`assert_not_impl_any!` で `!Send` 負側固定・DAG 監査で `ruviform-audio` に plugin 依存なし（AC3） |

spawn 引数・`CARGO_BIN_EXE_*` の利用は crate 内に閉じる（cross-package bin 解決に依存しない — 既存 `l1_proxy_e2e` と同じ方式）。video plugin の E2E は対象外（NONRT 境界の明文化まで）。

## Risks and Trade-offs

- **L0 `try_lock` contention による一時 bypass**: non-RT verb（state 往復等）が shared cell を保持している間、RT ブロックは bypass（`out` 不変更）になる。遅い verb の実行中は実質ミュート扱い — 許容と判断（defined・ハングなし・代替案の extraction/verb-relay は API 面が重く同値の失敗面を持つ）。運用上は重い verb を音声クリティカル区間外に呼ぶ利用側規律を doc に書く。
- **`pending_arm` 滞留**: arm channel が `Full` のまま non-RT entry point が来ない場合、endpoint は旧 generation ring で pass-through を継続する（定義済み劣化 — 無限に古い ring を使い続けるのではなく generation 不一致で submit が drop されるため出力は pass-through 化する）。容量 2 + 再 Activate は稀な運用 + 再送が `execute`/`try_recv_notification`/`prepare_endpoint`/host `process`/teardown の**全 `&mut` entry point** で試みられるため滞留窓は最小化される（`&self` query 点は defined non-drain — 上記 tier 規則）。overflow は `DropCollector::overflows` 型の観測点ではなく `pending_arm.is_some()` で残存確認可能。
- **`process_audio` の入力複写コスト**: `out → in_scratch` の bounded memcpy が全ブロックで走る。transform 意味を一律に保つための固定コストで、ソース型 plugin では無駄になり得る — v1 は一律複写で単純化し、bus/用途別の最適化（in-place 直接処理・コピー省略）は実プラグイン統合時の論点とする。
- **L0 スロット enum による verb dispatch の二分**: `Plain`/`Shared`（上記 `Instance` 内 slot enum）で各 verb が分岐する機械的な変更 — 振る舞いの差はないためリスクは低いが、`plugin_mut` が `Shared` で `None` を返す点だけ公開挙動の差分（L1 と同じ defined 挙動に揃う）。
- **plan 退役 capacity**: retire channel は bounded で `Full` 時は leak-on-full（`mem::forget` — RT drop しないための設計済み fallback）。swap 頻度比例なので容量は小で足りるが、`retired_overflows()` の観測点を drain 習慣と共に doc 化する。
- **代替案（未採用）と理由**: host-in-node wrapper（型上不可能 — ADR rejected）・full async service（L0 直接呼出しを喪失 — rejected）・全域 `Send` bound（OFX 不適合 — rejected）・endpoint extraction + verb relay（non-RT verb 到達を失い detach/reattach 語彙が要る — 共有セルの方が整合）・snapshot `slot` による arm/plan 移行（初回 read の warm-up 例外 + `PlanSwap` API 形の churn — 退役 hole のみを治す最小修正を優先。`PlanSwap` を snapshot へ移す全面的な置換は将来の選択肢として記録）。

## Deferred Design Concerns

- video plugin の RT/アウトプロセス経路・L1 video wire | 本 Issue は audio `process` 経路のみ・video port は全操作 NONRT | NONRT video port の分離要求（L1 design deferred 行と整合） | video plugin は L1/RT 経路を使えない
- OFX NONRT ノードの engine/video 側接続形 | 本 Issue は境界明文化まで | 第 2 ターゲット video engine 統合 | video plugin が engine から到達不能のまま
- 実規格アダプタ（CLAP/VST3/OFX）経由の実プラグイン動作 | runner 形態・実バイナリ依存の別 Issue 級 | 実プラグインでの統合需要（Issue #114） | 検証が mock/test-plugin 範囲に留まる
- イベント・transport のグラフ配線 | v1 グラフは `EventList::empty`/`None` transport | music-time/transport 源の engine 統合 | sample-accurate event が plugin へ届かない
- 複数 bus・plugin chain のルーティング/自動接続 | v1 グラフは順序リスト・`process_audio` は 1 bus 写像 | 実プラグイン構成の多様化 | 手動配線・単一 bus のみ
- `PlanSwap`/`RtPlan` の snapshot `slot` への全面移行 | 本 Issue は退役 hole の最小修正 | arc-swap debt warm-up を許容できる運用条件の確定・大規模再構成需要 | 現行機構のまま（退役は補正済み）
- endpoint `process` の `RtProcessor` 準拠（`Io` 関連型の借用形適合） | host 語彙との一貫性を優先 | 統一 RT 契約面の要求 | port 抽象の二系統化が残る

## Verification Notes

- [ ] spec AC1（実グラフ消費）: `PluginHost` 経由のインスタンスが `Graph`/`GraphPlan`/`Session`（mock device 駆動）内部から per-block `process` され gain 正当性が検証される — L0/L1 双方
- [ ] spec AC2（所有分離）: `PluginHost` は non-RT 残留、`expect_send` で endpoint/ノード closure/`GraphPlan`/`RtPlan`/コールバック capture の `Send` を静的証明、`!Send` な制御値が RT 側に存在しない
- [ ] spec AC3（構成面）: default L0 / opt-in `OutOfProcessSpec` の選択面が動作し `DuplicatePluginId`/`lookahead_blocks=0`/非 Send 対応の `EndpointUnavailable` が defined rejection — `ruviform-audio` の公開 API・Cargo.toml に plugin-host 露出なし（DAG 監査）
- [ ] spec AC4（明示的 prepare + bounded）: lifecycle 未完了の `prepare_endpoint` が `Err(Lifecycle)`、capacity 不整合が `Err(Prepare(UnsupportedConfig))`、activated/bound 超過ブロックが `Err(Prepare(UnsupportedConfig))`（panic/UB/alloc なし — `Err(Ipc)` は L1 内部の wire 表現不能に留める）
- [ ] spec AC5（error/fallback）: `kill_child` → `Failed` + `InstanceFailed` once + 以後 pass-through/無音・自動再起動なし・`Failed` 以後コマンドは defined 拒否・遅延未着/slot busy も pass-through
- [ ] spec AC6（latency 報告 — 修正版式）: `latency_frames`（host・endpoint 両面）が `lookahead_blocks × activated max_frames`（frames 整数）と一致
- [ ] spec AC7（RT alloc）: `rt_` 命名テスト（L0/L1 endpoint `process`・arm apply・plan `active` 経路）が `assert_no_alloc` 窓内 0 alloc で `mise run test-rt` グリーン
- [ ] spec AC8（退役）: node removal/plan-swap で DropFlag probe が RT 経路で立たず `drain_retired`/teardown で立つ — endpoint・staging・plugin オブジェクトを含む。`retired_overflows()`/collector `overflows()` が 0 であることを検査（leak-on-full 飽和の observable signal）
- [ ] spec AC9（E2E）: L0/L1 graph E2E が mock device で成立（L1 は `l1-test-worker` spawn→lifecycle→process→destroy）
- [ ] spec AC10（video 境界）: video NONRT 境界・延期 trigger・L1 video wire 非確立が本書および crate doc に明文化される
- [ ] spec AC11（quality gate）: `mise run fmt-check`/`check`/`lint`/`test`/`test-rt` パス・doc 監査で本チェーン起因 findings 0
- [ ] 設計レビュー: `Send` 契約が型で保持されること（`unsafe impl Send` なし）・`#[non_exhaustive]` 方針（`EndpointUnavailable` variant は新 enum でなく既存 `HostError` への追加・local-only wire 方針 — `encode`/`encode_host_error` の `Result<(), WireError>` 化 + `Display`/roundtrip リスト更新 + truncate-on-Err 規則の negative test）・engine が plugin 型を名指さないこと・退役 offload が `ruviform-rt` 既存機構の再利用に留まること
- [ ] レビュー反映の機構固定: L0 `SharedSlot`（epoch CAS attach・`SendCore.activated`・lock 内 commit+mirror+hook の順序・endpoint `Failed` 収束の第 2 書き手（`fetch_max`）・verb 3 段 lock 形態（pre-check/`match`/lock 内再読み）・`process_host` の `catch_unwind`・endpoint の lock 内 mirror 再読み・`&self` 照会の観測 union/`&mut` commit 分割）・L1 arm 経路（per-attach mint + seed arm（同 generation のみ正規化 no-op — leg は装着 staging を保持して移動・detached-Activate で `Inner::staging` が新世代でも、還流 leg は host 側 non-RT drain が `rt.arm` で re-arm するため再 attach seed は常に同 generation）・generation 単調 apply・`&mut` entry point tier での再送・全 drain arm の `Retirer` 行き・`arm_apply` 簿記分割・`return_rx` drain → `self.rt` 復元は wait-free 部分でも動くが clear/re-arm は `&mut` 点のみ・restore は mirror ≠ `Destroyed` gate）・`process_audio`（borrow 分割・per-block view・activated gate（in-bus-first 情報源・L1 は `frames ≤ armed frame_capacity` + `channels == bound.channels`・`min_frames` は child 契約委譲）・bound clip 順序）が本書どおり実装されること
- [x] レビュー記録（review-protocol）: cycles 1-2 — verdict **ACCEPT-WITH-NOTES**（設計は planning 向けに approved。non-blocking notes の pin 反映: N1 = `Shared`/`SharedSlot` の `lifecycle_state` は `mirror` Acquire 読み・`core.lock()` 不許可・`&self` の `Failed` union は `mirror` の read-only union、N2 = `L1Instance::process` の drain は `rt.is_none()` early-return より前、N4 = `try_recv_notification`/`RtLeg::arm`/wire roundtrip variant リストの cite 修正 — いずれも本書へ反映済み）
