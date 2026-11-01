---
id: "DISC-034QhIlaOBN3t7jVxA4sW7"
type: "discovery"
status: "active"
title: "Issue #185 Generic Probe: Node/Port observation capability, selective monitoring, RT-safe bounded PCM capture — existing-surface inventory and design option survey"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/synvelis/issues/18"
    - "https://github.com/scarecrow173/synvelis/issues/21"
    - "https://github.com/scarecrow173/synvelis/issues/22"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-184-typed-graph-ir-ports-buses-connections-validation-revisions-m1.md"
    - "docs/designs/issue-195-m2-typed-graph-compiler-buffer-pdc-rt-runtime-ownership-activation.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/media-observation-non-destructive-pcm.md"
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
    - "docs/specs/issue-184-typed-multi-bus-port-connection-graph-ir-contract-migration-m1.md"
    - "docs/specs/issue-195-typed-graph-audio-compiler-buffer-pdc-stateful-ownership-activation-m2.md"
    - "docs/discovery/issue-195-m2-typed-graph-compiler-runtime-audit.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/synvelis-driven-target-architecture-and-migration.md"
    - "docs/adr/synvelis-driven-generic-execution-boundaries.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/104"
    - "https://github.com/scarecrow173/ruviform/issues/184"
    - "https://github.com/scarecrow173/ruviform/issues/187"
    - "https://github.com/scarecrow173/ruviform/issues/195"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/187"
    - "https://github.com/scarecrow173/ruviform/issues/188"
    - "https://github.com/scarecrow173/ruviform/issues/196"
  deferred-by: []
---

# Issue #185 Generic Probe: Node/Port observation capability, selective monitoring, RT-safe bounded PCM capture — existing-surface inventory and design option survey

## Summary

Issue #185 asks Ruviform to own a generic Node/Port observation contract: declared probe capabilities, Off/Monitor/Capture execution states, lightweight metrics plus bounded PCM capture, and an RT→non-RT handoff that never allocates, locks, or performs I/O on the callback path. The inventory shows the M2 typed runtime (`ruviform-audio::typed`) already supplies every primitive the design needs — stable `NodeId`/`PortId`, generation-tagged plans, bounded slot arena, SPSC command/event channels, snapshot publish, retirer drains — and `PortCapability::ProbePoint` is already reserved in the IR for this issue. Best-current hypothesis (high confidence): resolve observation points as plan-bound slot taps, ship probe commands over the existing generation-tagged SPSC pattern, and place the PoC in `ruviform-audio::typed::probe` with contract vocabulary split between the IR crate (reference/capability vocabulary) and the engine (runtime machinery).

## Exploration Goal

どの既存資産が #185 の要求 1〜9（capability 列挙・実行状態・計測/PCM 取得・識別/時間メタデータ・RT 安全 Capture・多点比較・動的 ON/OFF・任意観測点・配送契約）を既に支え、どこに新規設計が必要かを棚卸しし、crate 配置・RT ハンドオフ方式・未対応/利用不可の分類モデルの選択肢を spec/ADR へ渡せる粒度で比較する。PoC スコープ（モック Graph で 2 観測点以上・pre/post Peak/RMS・上限制御 PCM 搬出・loss/溢れ/途中停止/revision 更新・ON/OFF 負荷差計測）もここで確定させる。

## Key Issues and Assumptions

- 観測点の物理的実体は arena `Slot` であり、`PortId`→slot 解決は plan 世代ごとに変わり得る — 「存在しない物理バッファを返さない」要件は binding 解決結果の分類（未接続 input・非 Audio domain・boundary のみの port）が必須。
- 既存 `observability_poc` は `#[doc(hidden)]` 実験専用で公開契約ではないが、`bridge`（drop-on-full SPSC + overflow counter）と `ObsRecord` の形状が実測済みの設計入力になる。
- `RtRuntime::process` は `RefCell<Slot>` arena + `&mut self` 単一 owner — tap は materialize 後 / processor 前と processor 後 / stamp 更新後の 2 点が安全な借用境界。processor 実行中の `NodeIo` borrow と衝突しない。
- `ScheduledNode.outputs: Vec<usize>` は port 宣言順の slot index のみで `PortId` を保持しない — output port の PortId 逆引きは `NodeSpec` の port 宣言順との対応付け（非 RT 側で実施）が必要。
- plan に per-node accumulated latency は現状 `total_latency` のみ（`own_latency`/`accum` は compile 内部値）— 経路遅延メタデータには additive な plan 拡張が要る。
- Probe ON/OFF は plan 世代とは独立だが、観測点の binding は世代に従う — command channel は generation タグ付きで stale 遮断の既存パターンを再利用するのが自然。
- Capture の PCM 運搬は可変長データのため `Copy` 制約の `observability_poc::bridge` 型レコードでは運べない — pre-allocated チャンクプール（非 RT→RT 空きチャンネル / RT→非 RT データチャンネル）か共有リングが必要。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| **A. crate 配置: `ruviform-audio::typed::probe`（IR 語彙は `ruviform-graph` に最小追加）** | 新規 crate・members/Cargo.lock 変更不要。Node/Port→slot 解決は engine 内の plan binding と同居。`PortCapability::ProbePoint` の既存フックと整合。層方向（engine→graph/rt/analysis）は下向きのみ | 「汎用契約」が audio engine 名付きの公開面になる。将来 MIDI/Event/Control ドメイン拡張時に再配置が要る可能性 | yes |
| **B. crate 配置: 新規 `ruviform-probe`（graph と audio の中間層）** | 観測契約の独立所有で domain 拡張に強い | 29 crate 構成への新規追加は members/Cargo.lock/crates README/層ドキュメント整合コスト大。実行 hook は結局 engine に必要で二層分割になる | no（PoC で過剰） |
| **C. crate 配置: 契約型を `ruviform-observability`、実行を audio** | 「shared observability primitives」の責務に字面が合う | `NodeId`/`PortId` を使うには `ruviform-graph` への依存が要り、foundation→structural の上向き依存で層ルール違反 | no |
| **D. RT→非 RT PCM 運搬: pre-allocated chunk プール + 双方向 SPSC** | `ruviform_rt::channel` をそのまま再利用。bounded・wait-free・alloc 0・溢れ方針（free 空=loss、data 満杯=retire+loss）が明示的。チャンク返還で無制限 capture でも予算内 | プール管理がやや複雑（free/data 2 チャンネル + retire 経路） | yes |
| **E. RT→非 RT PCM 運搬: 共有リング（連番+torn-read 検出）** | チャンネル 1 本で済む | 大容量 PCM の torn read を契約で許すか検出メタが複雑化。`unsafe` なしで共有可変バッファを型安全に書くのが難しい | no |
| **F. 能力分類: `media-core::Fact<T>` 再利用** | #151 で確立した「値か欠落理由か」の型パターン。`Copy` で RT 記述子にも乗る | 要求語彙（Configured/Observed/Inferred/Unavailable）は `Fact` の Known/NotDeclared/Unsupported/Malformed と一致しない。実行時観測点の分類は独自 enum が自然 | maybe（語彙は独自 enum・パターンは踏襲） |
| **G. メトリクス: `audio-analysis::LevelMeter` 再利用** | 棚卸し対象の活用・Peak/RMS 定義が既契約と一致 | `process` は `PlanarBufferView` 経由で slot→view 構築が要る（`&[&[f32]]` の組立は RT で可変長になり得る） | yes（ch 単位の `measure` 関数を抽出して共用） |
| **H. Probe 状態制御: plan 内組込み（PreparedPlan の一部）** | 世代と一体で atomic に publish | Probe ON/OFF のたびに再 compile が要り「動的 ON/OFF を非 RT で安全に」は満たすが publish コストと世代 churn が大きい | no |
| **I. Probe 状態制御: 独立 generation タグ付き SPSC command** | 既存 `RuntimeCommand`/`NodeEvent` の staleness パターンと同一。plan 再 compile なしで動作 | rebind（世代ごとの slot 再解決）を別コマンドとして運ぶ必要 | yes |

## Tentative Conclusions and Hypotheses

- **confirmed**: 観測点の標準集合は `{node input pre, node output post, graph boundary in, graph boundary out}` で、全て `Slot` 上の非破壊 read として実現できる。`ScheduledNode.inputs[i]`（`InputBinding.port` 保持済み）と `outputs[i]`（宣言順）+ `plan.inputs/outputs`（boundary）で非 RT 側が PortId→slot を解決できる。未接続 input（edges 空・scratch 無し）は物理バッファを持たず `Unavailable` 相当、非 Audio domain port は `Unsupported` 相当に分類する必要がある。
- **confirmed**: OFF 時コストは mode 判定のスキップのみで設計可能（probe 一覧は固定上限の Vec・全 Off 時はノードごとに比較のみ）。ON/OFF で音声出力不変は tap が read-only であることで構造保証される — 検証は「同入力・同 plan で probe 有無の出力 byte 一致」テストで行う。
- **confirmed**: `RtState.seq` は callback 序数で sample position ではない — `frame_cursor: u64`（process ごとに += frames）を追加すれば block 先頭の絶対 sample position を全 tap で共有でき、同一処理区間の pre/post・多点整列の時間軸になる。
- **hypothesis（高信頼）**: Probe command は `RuntimeCommand` と同じ generation タグ + stale 遮断を適用し、rebind（generation 更新で slot 解決が変わる時の再送）は非 RT broker が publish 成功後に発行する。RT は resolved slot index のみを保持して plan を逆引きしない。
- **hypothesis（高信頼）**: Capture は `Off/Monitor/Capture{budget}` の mode enum と `CaptureConfig{max_frames, pool_chunks}` で期間・メモリ予算を明示制御し、budget 尽きると `Stopped{BudgetExhausted}` を記録して Monitor へ復帰する。Stop 要求・溢れ（free 枯渇・data 満杯）は loss カウンタ + 状態イベントとして観測可能にする。
- **hypothesis（中信頼）**: `ruviform-audio-analysis` に `ChannelLevel::measure(&[f32])`（または同等の ch 単位関数）を抽出し `LevelMeter::process` もそれを呼ぶ形に揃えると、DRY を保ったまま `PlanarBufferView` 組立なしで slot チャンネルを直接計測できる。`ruviform-audio` → `ruviform-audio-analysis` の新規依存は engine→domain foundation の下向きで許容。

## Open Questions

- Probe handle（`ProbeId`）の採番主体: 非 RT broker が per-runtime 単調採番で十分か（PoC: yes。永続安定識別子・cross-session は範囲外として明記）。
- Monitor メトリクスの配送周期: 毎 block 送ると channel 容量を食う — PoC は「最新値 + 系列」のどちらを採るか（現行方針: block ごとの `MetricFrame` を SPSC で送り、満杯時は drop+loss 計数。pull 側は drain で最新系列を得る）。
- 経路遅延メタデータの粒度: per-tap `accumulated latency`（input 側は producer accum、output 側は node accum）を compiler が `ScheduledNode` に保持する additive 変更で十分か — design で確定。
- `audio-lens` consumer 影響分類の検証先: ruviform-audio/engine 面を消費しない根拠を consumer compat contract（#219）に照らして明文化する。

## Promotion Candidates

- [x] spec-doc needed: capability 分類・実行状態・Capture 予算・メタデータ・RT 安全性・配送契約・consumer 影響の要件確定（`docs/specs/issue-185-generic-probe-rt-capture.md`）。
- [x] adr-doc needed: crate 配置（A/B/C）・PCM 運搬（D/E）・状態制御経路（H/I）・能力分類語彙（F/独自）の決定記録（`docs/adr/issue-185-generic-probe-placement-rt-handoff.md` 予定）。

## Sources and Relations

- 一次要件: [Issue #185](https://github.com/scarecrow173/ruviform/issues/185)（要求 1〜9・範囲外・最小 PoC 完了条件）、[Issue #193](https://github.com/scarecrow173/ruviform/issues/193)（M7 実装境界・G7 RT Safety Gate）。
- 前提契約: [Issue #184](https://github.com/scarecrow173/ruviform/issues/184) Typed Graph IR（`PortCapability::ProbePoint`・`StableHandleMap`・`GraphRevision`）、[Issue #195](https://github.com/scarecrow173/ruviform/issues/195) M2 typed PreparedPlan/RtRuntime（`ScheduledNode` binding・generation・retire 経路）。
- 関連設計: `docs/designs/issue-184-...md`（ProbePoint は「#185 が逆引きする情報の保持」・mapping の観測利用は未検証と明記）、`docs/designs/issue-195-...md`（「Probe tap・offline driver」を #185/#191 へ defer）、`docs/adr/media-observation-non-destructive-pcm.md`（`Fact<T>` 分類パターン）、`docs/designs/foundation-observability-minimal-api.md`（counter/rt_bridge sketch）。
- Consumer 側の期待語彙: Synvelis #18/#21/#22（Configured/Observed/Inferred/Unavailable・Graph Generation/Frame/Latency/Loss Evidence・Recording Writer との責務分離）。
- 責務境界: Recording（Synvelis #12・Ruviform #187）の無欠損契約へ Probe の drop/overwrite 方針を転用しない — 本チェーンで spec 範囲外として明記。

## Evidence

### 2026-11-02 — 現行コード棚卸し（実ファイル走査結果）

**`crates/ruviform-audio-analysis`（696 行）**: `LevelMeter`（`levels: Vec<ChannelLevel>`、ch ごと `peak=max|x|`/`rms=sqrt(Σx²/n)`、`process(&PlanarBufferView)` は RT-suitable・`assert_no_alloc` 検証済み）、`SpectrumAnalyzer`、`WaveformPeaks`（min/max 間引き・非 RT 取得）。RT→非 RT 転送機構は持たず「consumers move snapshots over the existing ruviform-rt conventions」と明記。依存: audio-core/dsp/media-core。

**`crates/ruviform-observability`（157 行）**: `counter()` → `Counter`/`CounterReader` pair（`Arc<AtomicU64>`、`increment`/`read` は wait-free/no-alloc、構築のみ非 RT）。`u64` 単調・wrap modulo 2^64・snapshot 非トランザクションと明記。依存ゼロ。

**`crates/ruviform-rt`**: `spsc::channel<T:Send>`（rtrb wrapper・`try_push`/`try_pop`/drop-on-full は caller 方針）、`snapshot::slot`（ArcSwap last-good publish/read + guard-retire）、`retire_channel`（Retirer→DropCollector・leak-on-full+計数）。`observability_poc`（`#[doc(hidden)]`・公開契約外）: `bridge`（drop-on-full SPSC + shared overflow counter + aggregate cell、`Copy` record のみ）、`ObsRecord{kind,timestamp_nanos,value,reserved[2]}`、timestamp helper。bench `rt_observability_poc` あり（#104 のコスト計測）。

**`crates/ruviform-graph`（M1 IR）**: `NodeId`/`PortId`/`ConnectionId`/`GraphRevision`（u64・builder 採番・1 開始）、`RuntimeNodeIndex`/`RuntimePortIndex`（StableHandleMap 位置）。`PortCapability::ProbePoint` 宣言済み（「#185 boundary」コメント付き）。`SignalDomain::{Audio,TimedEvent,Control}` は `PortFormat::domain()` が正本。全 API 非 RT・依存は audio-core のみ。

**`crates/ruviform-audio::typed`（M2）**: `Compiler`（非 RT・`ValidatedGraph`→`CompiledPlan{plan,installs,removes,alloc_after}`）、`PreparedPlan{generation,max_frames,slots,delays,schedule,inputs,outputs,total_latency}`、`ScheduledNode{node,pslot,inputs:Vec<InputBinding{port,edges,scratch}>,outputs:Vec<usize>}`、`RtRuntime`（`reader:SnapshotReader`、`arena:Vec<RefCell<Slot>>`、`delays`、`RtState{commands,events,processors,procs_retirer,active_generation,seq}`）、`GraphRuntime`（publish/send_event/collect/status）。`NodeIo` は arena borrow + per-access view。`SharedStatus`/`RuntimeFault` で fault 可観測。旧 `Graph`/`GraphPlan`/`PlanSwap`/`Session`（pull 型単一 bus）は別系統で golden 維持。

**テスト基盤**: `crates/ruviform-audio/tests/common/mod.rs` に `chain()`/`merge()`/`mapped()` ビルダ・`MockDsp`（stateful・latency 注入・log）・`MockFactory`・`feed`/`drain` helper・`config(max_frames)`。`rt_` テストは `assert_no_alloc` の `AllocDisabler`（debug_assertions 時 global_allocator 差替）で計測。
