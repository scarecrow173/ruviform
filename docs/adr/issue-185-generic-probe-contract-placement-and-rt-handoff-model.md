---
id: "ADR-034QhNYvqRm2YZ7nXzOiMb"
type: "adr"
status: "accepted"
title: "issue-185 generic probe contract placement and RT handoff model"
created: "2026-11-02"
updated: "2026-11-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/185"
    - "https://github.com/scarecrow173/ruviform/issues/193"
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
    - "realtime-contract-and-verification-policy.md"
    - "m2-typed-graph-audio-runtime-placement-stateful-rt-ownership-and-activation-reti.md"
    - "typed-multi-bus-graph-ir-placement-and-port-connection-validation-model-issue-18.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "media-observation-non-destructive-pcm.md"
    - "public-enum-non-exhaustive-policy.md"
    - "synvelis-driven-generic-execution-boundaries.md"
    - "docs/designs/foundation-observability-minimal-api.md"
    - "docs/designs/shared-foundations-and-modularization-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-185-generic-probe-observation-inventory.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/104"
    - "https://github.com/scarecrow173/ruviform/issues/187"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# issue-185 generic probe contract placement and RT handoff model

## Context and Problem Statement

Issue #185 は Ruviform に「一般的な Node/Port 境界での観測能力宣言・選択的観測実行・RT→非 RT 転送契約」を要求する。#195 の M2 typed runtime（`PreparedPlan`/`RtRuntime`/generation/slot ownership）が確定し、観測点の物理的実体は arena `Slot` に確定した。残る設計判断は (a) 契約・実行機構の crate 配置、(b) 観測点をどう tap するか、(c) 可変長 PCM をどう RT→非 RT へ運ぶか、(d) probe の動的制御をどう publish するか、(e) capability 分類の語彙、(f) Peak/RMS 計測の再利用形の 6 点である。

## Decision Drivers

* RT 契約（alloc 0・非ブロッキング・OS I/O なし・destructor なし）を `process` 区間で維持すること（RT 契約 ADR）
* 「存在しない物理バッファを返さない」— capability 報告は `PreparedPlan` の binding 解決結果で分類すること（要求 1）
* OFF 時コスト≈0、ON/OFF で音声出力不変（要求 2・7）
* Capture は期間・メモリ予算を明示制御し、溢れ方針・loss 記録を定義すること（要求 5）
* 同一 generation+sample position で多点整列でき、PDC 遅延を metadata で説明可能にすること（要求 4・6）
* `ruviform-rt` 既存 primitive の再利用（新規同期機構を作らない）
* クレート層方向（上位→下位のみ・循環禁止）と既存 crate 数の維持
* Synvelis 語彙（Configured/Observed/Inferred/Unavailable）と矛盾しない分類を Ruviform 側で定義すること

## Considered Options

* **契約・実行の配置**: (a) `ruviform-audio::typed::probe` に集約、(b) IR 語彙を `ruviform-graph`・実行を `ruviform-audio` に分割、(c) 新規 `ruviform-probe` crate
* **観測点の tap 方式**: (a) `RtRuntime::process` の schedule loop に直接 tap 点を挿入、(b) `GraphProcessor` ラッパで包む、(c) graph 内に probe 専用 node を挿入する
* **PCM/メトリクス搬送**: (a) pre-allocated chunk プール + `free`/`chunks_out`/`telemetry` 3 チャンネル分離（在飛 chunk ≤ pool の構造的不満杯保証 + POD record drop-newest）＋ collector 側 `recycle` 返却経路、(b) 共有リング + 連番 torn-read 検出、(c) `observability_poc::bridge` 型の固定 `Copy` レコードを PCM 単位で刻む
* **動的制御**: (a) generation-scoped command（binding 付き）は activation 時 drain・lifetime command は毎 callback drain の `ProbeCommand` SPSC、(b) `PreparedPlan` 内に probe 集合を組み込み publish、(c) `NodeEvent` への multiplex
* **capability 語彙**: (a) 専用 `#[non_exhaustive]` enum（`Configured`/`Inferred`/`Unavailable`/`Unsupported`/`Unknown`）、(b) `media-core::Fact<T>` 再利用
* **Peak/RMS 計測**: (a) `ruviform-audio-analysis` に channel 単位 `measure` 関数を抽出して共用、(b) probe モジュール内に private 複製、(c) `LevelMeter` + `PlanarBufferView` を tap 毎に組立てる

## Decision Outcome

* **配置**: (a) `ruviform-audio::typed::probe` に契約・実行・配送を集約する。観測点の PortId→slot 解決は `PreparedPlan` binding と同居するのが自然で、`ruviform-graph` には既存 `PortCapability::ProbePoint` 宣言のみで追加変更を要しない。`ruviform-audio` → `ruviform-graph`/`ruviform-rt`/`ruviform-audio-analysis`/`ruviform-observability` の依存方向はすべて下向きで層ルールに適合する。将来 domain 拡張（TimedEvent/Control payload）で契約が大きくなった場合、crate 分割は additive に移行可能な module 境界を保つ。
* **tap 方式**: (a) `RtRuntime::process` 内の schedule loop に 2 点（node input = materialize 後・processor 前、node output = processor 後・stamp 更新直後）+ boundary（graph input = schedule 前、graph output = schedule 後）の固定 tap 点を置く。`GraphProcessor`/`NodeIo` の API に変更を加えず、probe は `&mut RtState` 内で slot の非破壊 read のみを行う。OFF 時は mode 判定のみでタップ処理を実行しない。
* **PCM/メトリクス搬送**: (a) chunk プール + 3 チャンネル分離。非 RT broker が `CaptureChunk{header, Vec<f32>}` を `max_frames × max_channels` で確保して `free` チャンネルへ流す（`chunk_frames >= plan.max_frames`、copy は `[..frames]` のみで `frames_valid` が有効 prefix を記録）。RT→非 RT は **`chunks_out`（`Box<Chunk>` のみ・capacity = pool サイズ）** と **`telemetry`（`Metric`/`Status` POD record・明示 capacity・drop-newest + loss 計数）** の 2 チャンネルに分離する。在飛 chunk 総数（free + RT working slot + chunks_out + collector）は pool に制限されるため `chunks_out` の push は原理的に満杯にならない — `Retirer` の leak-on-full（overflow で `mem::forget`・上限なし）を PCM 搬送に使わない。防御的 `try_push` 失敗時は working slot へ戻し次 block で再利用 + fault 計数。telemetry 満杯は drop-newest + per-probe loss 計数（monitor 流量で capture を枯渇させない）。drain 済み chunk は `ProbeCollector::recycle` で free へ返却し、broker が drain ごとに pool を目標数まで補充する（caller が recycle せず drop しても pool 縮小→broker 補充で回復、leak しない）。Remove/teardown 時の working chunk は `chunks_out` に orphan として流し collector で破棄する。
* **動的制御**: (a) `ProbeCommand` SPSC を 2 種に分けて適用する — **generation-scoped command**（`Install`/`Rebind`: slot index 等の plan 由来 binding を運ぶ）は `RuntimeCommand` と同じ規則で `activate_generation` 内で `plan.generation` 一致時のみ適用（stale は計数・不適用 — publish 直後に送った command は queue で待ち、対象 generation の activation で原子的に適用される）、**lifetime command**（`SetMode`/`Remove`: probe_id 宛てで plan 内容を含まない）は staleness 判定なしで毎 callback `activate_generation` の後に drain する。broker は publish 成功後 `GraphRuntime::published_plan`（新規アクセッサ・publish 時に retain する `Arc<PreparedPlan>`）から新 plan を取得し、全 live probe の binding を再解決して `Rebind(plan.generation)` を送る — Rebind は binding+mode の完全状態を運ぶため欠落した `SetMode` は次の Rebind で自己修復する。防御として `RtProbe` の binding は `bound_generation` を持ち、tap 時に `plan.generation` と不一致なら `Unbound` status を記録して観測をスキップする（slot index 再利用による**誤 slot の無言観測を型で防ぐ**）。plan 再 compile は不要。
* **binding 規則の集約（fix at the right boundary）**: probe resolver が output port→slot・effective input slot の規則を複製しないよう、`ScheduledNode::outputs` を `Vec<(PortId, usize)>` に変更し、`InputBinding::effective_slot`（`scratch ?? 単一 edge slot`）を compile 時に確定する。NodeIo/materialize/probe が同じ規則を読む。
* **tap の鮮度監査**: tap は `NodeIo` と同じ `slot.stamp() == seq` 規則を `try_borrow`（panic-free）で適用し、不一致ではデータ record を発行せず `Unobserved` を記録する。経路遅延は Rebind 時に tap 点別 `effective_latency`（input direct edge → `accum[producer]`、scratch merge → `accum[prod]+delay_frames`、output → `accum[node]`、boundary output → `total_latency`）として確定し record に載せる。
* **capability 語彙**: (a) 専用 `#[non_exhaustive]` enum。`Fact<T>` の「値か欠落理由か」パターンは踏襲するが、要求語彙（Configured/Observed/Inferred/Unavailable）は `Fact` の分類（Known/NotDeclared/Unsupported/Malformed）とは意味が異なり、実行時 binding 解決という文脈独自の分類として独立した enum にする。
* **Peak/RMS**: (a) `ruviform-audio-analysis` に `ChannelLevel::measure(&[f32])`（仮称）を抽出し、`LevelMeter::process` もそれを呼ぶ形に揃える。`PlanarBufferView` 組立てが不要になり、DRY を保って slot channel を直接計測できる。
* **時間メタデータ**: `RtState` に絶対 frame cursor（`frame_cursor: u64`）を追加し、record の `sample_position` を block 先頭の cursor 値とする（`seq` は callback ordinal で可変 `frames` から位置を再構成できないため、独立した絶対位置源が必要）。`ScheduledNode` に `latency_accum`/`own_latency` を additive 追加し、tap record に経路遅延を載せる。

### Consequences

* Good, because: 新規 crate・workspace members・`Cargo.lock` の構造変更不要。既存 SPSC/retire/snapshot primitive と staleness パターンをそのまま再利用でき、RT 契約の根拠が既契約に沿う。OFF 時コストは probe 一覧の mode 比較のみで、音声経路に分岐が入らない。
* Bad, because: 「汎用観測契約」が `ruviform-audio` の公開面に定着し、TimedEvent/Control の payload 観測や engine 外 consumer が増えた場合に再配置コストが残る。`RtRuntime::process` の schedule loop に tap 点が増え、単一 owner のコード経路がやや複雑になる。

### Confirmation

* PoC テストが spec の AC-1〜AC-10（AC-5b 含む）を満たし、`rt_` 契約テストで probe ON/OFF 有無の alloc 0 を検証する。ON/OFF 負荷差の計測結果は impl record に記録する。
* capability 列挙・slot 解決・loss 計数が決定的で、generation 更新後の rebind が**新 plan の正しい slot を観測**することを integration test が検証する。
* `cargo tree -p ruviform-audio` で新規依存が `ruviform-audio-analysis`・`ruviform-observability` のみで層方向を維持することを確認する。

## Implementation Plan

* Affected paths: `crates/ruviform-audio/src/typed/`（`probe.rs` 新規・`runtime.rs`/`compile.rs`/`mod.rs` additive 拡張）、`crates/ruviform-audio-analysis/src/level.rs`（channel 単位関数の抽出）、`crates/ruviform-audio/Cargo.toml`（新規依存）、`crates/ruviform-audio/tests/`（probe PoC テスト）
* Patterns to follow: `RuntimeCommand`/`NodeEvent` の generation タグ + stale 計数、`Retirer`/`DropCollector` の RT→非 RT 退役、`SharedStatus`/`RuntimeFault` の fault 可観測性、`assert_no_alloc` の `rt_` テスト、公開 enum `#[non_exhaustive]`・`**RT contract:**` doc ラベル
* Patterns to avoid: RT 側での `Vec` 確保・`RefCell` borrow の跨ぎ・destructor 実行、`Option`/`0` 値での欠落理由の潰し、`PreparedPlan` への probe 組込み（plan churn）、`observability_poc` 型への直接依存（`#[doc(hidden)]` 実験面は参照のみ）
* Dependencies or configuration: `ruviform-audio-analysis`・`ruviform-observability` を `ruviform-audio` の `[dependencies]` に追加（層方向は下向きのみ）。新規 feature・optional 依存は追加しない

## Verification

* [ ] `mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`verify-ci` exit 0
* [ ] `rt_probe_*` テストで `assert_no_alloc` 窓内の alloc 0（Monitor・Capture・OFF 各 mode）
* [ ] probe 有無で boundary 出力が bit 一致（信号不変性）
* [ ] chunk の `generation`/`sample_position`/`loss`/経路遅延メタデータが drain で取得できる
* [ ] free 枯渇・data 満杯・途中停止・generation 更新・遅延 drain で RT 非ブロック + loss/fault が非 RT 観測できる
* [ ] `cargo tree -p ruviform-audio --edges normal` に `ruviform-graph`/`ruviform-rt`/`ruviform-audio-core`/`ruviform-media-core`/`ruviform-audio-analysis`/`ruviform-observability` のみで循環 0
* [ ] probe モジュールが `#![forbid(unsafe_code)]` を宣言（doc コメント内の語を誤検出しないため grep ではなくコンパイラ属性で担保）

## Pros and Cons of the Options

### 契約・実行の配置

* **(a) `ruviform-audio::typed::probe` 集約（採用）**: 実装が plan binding と同居し、PoC が最小 diff で済む。`PortCapability::ProbePoint` は IR 宣言として既存のまま使える。悪い点: 「汎用」が audio engine の公開面に置かれ、将来 domain 拡張時に再配置コスト。
* **(b) IR 語彙を `ruviform-graph` 分割**: capability enum を IR crate に置くと「Port/Node→観測点参照契約」の所有者が明確。悪い点: 実行時 binding 解決は結局 engine で必要で、二クレート間で型を往復する層設計が増える（PoC で過剰）。
* **(c) 新規 `ruviform-probe` crate**: 独立所有で将来拡張に強い。悪い点: members/Cargo.lock/crates README/層ドキュメントの整合コストが最大で、slot 解決には結局 `ruviform-audio` の plan 型への依存が必要（上向き参照か engine に hook が要る）。

### 観測点の tap 方式

* **(a) schedule loop 直接 tap（採用）**: `process` の制御点が materialize/processor/stamp の確定位置にあり、借用境界が明確。`GraphProcessor` API 無変更。OFF 時コストは mode 比較のみ。悪い点: `RtRuntime::process` が probe 認知になり、経路がやや肥大。
* **(b) `GraphProcessor` ラッパ**: probe を processor の decorator として包むと runtime 無改造。悪い点: install/remove/activate 時の wrapper 管理が増え、「node input pre」はラッパ内 `NodeIo` 経由で取得可能だが boundary・materialize 後の scratch slot を正確に tap するには runtime 側の情報が必要になる。probe を外す際の型階層も複雑化。
* **(c) graph 内 probe node**: 観測をグラフ構造として表現。悪い点: 観測点が schedule に影響し得る構造（信号不変性の根拠が弱まる）、graph 変更のたびに probe node の再配線が要り、OFF 時コストも schedule 上に残る。

### PCM 搬送

* **(a) chunk プール + 双方向 SPSC（採用）**: bounded・wait-free・alloc 0・溢れ方針明示。`channel`/`retire_channel` 再利用で新規機構なし。悪い点: free/data 2 チャンネル + retire 経路のプール管理。
* **(b) 共有リング + torn-read 検出**: チャンネル 1 本。悪い点: 大量 PCM の torn read を契約で扱う必要・共有可変バッファを `unsafe` なしで型安全にするのが難しい。
* **(c) 固定 `Copy` レコード刻み**: `observability_poc::bridge` と同型。悪い点: PCM ブロックを record 化するには channel 容量×record size で予算が読みづらく、`Copy` 制約が `Vec` を排除するため inline 配列の上限を決める必要がある（実質チャンクプールと同じ問題を劣悪な形で解く）。

### 動的制御

* **(a) generation タグ付き `ProbeCommand` SPSC（採用）**: 既存 staleness パターンと同一。publish なしで mode 変更が即時反映。rebind は publish 後に broker が発行するだけ。
* **(b) `PreparedPlan` 内 probe 組込み**: atomic に plan と一体で publish。悪い点: ON/OFF ごとに再 compile・plan churn・registry 差分が発生し「動的 ON/OFF を非 RT で安全に」の意図（軽い切替）に反する。
* **(c) `NodeEvent` multiplex**: 既存イベント経路で済む。悪い点: `NodeEvent` は node 宛てで、probe は port/slot 宛て + deck 管理が必要なペイロードで型が合わない。

### capability 語彙

* **(a) 専用 enum（採用）**: 要求語彙と実行時分類を一致させ、`#[non_exhaustive]` で将来 variant 追加を許す。悪い点: `Fact` パターンと並行するもう一つの分類型が増える。
* **(b) `Fact<T>` 再利用**: 既存パターンの統一。悪い点: `Fact` の語彙は「宣言の有無・バックエンド非対応・不正値」で、実行時の「宣言済み/観測可能/推論/利用不可/不明」という probe 文脈にフィットしない（Observed/Inferred を表せない）。

### Peak/RMS 計測

* **(a) `ChannelLevel::measure` 抽出共用（採用）**: `LevelMeter` の定義をそのまま再利用しつつ `PlanarBufferView` 組立てを不要にする。悪い点: `ruviform-audio` に `ruviform-audio-analysis` への新規依存が発生する（下向きで許容）。
* **(b) private 複製**: 依存追加なし。悪い点: Peak/RMS の定義が二箇所に分散し、契約のずれが将来発生する可能性。
* **(c) `LevelMeter` + view 組立て**: 既存 API そのまま。悪い点: `PlanarBufferView::new(&[&[f32]])` の slice 配列を tap 毎に組立てると、RT で可変長の stack 配列準備か固定上限が要り、slot 直読みの方が自然。

## More Information

* 一次要件: [Issue #185](https://github.com/scarecrow173/ruviform/issues/185)、M7 実装境界・G7 RT Safety Gate は [Issue #193](https://github.com/scarecrow173/ruviform/issues/193)。
* 前提契約: [M2 runtime ADR](m2-typed-graph-audio-runtime-placement-stateful-rt-ownership-and-activation-reti.md)（generation/slot/retire ownership）、[M1 IR ADR](typed-multi-bus-graph-ir-placement-and-port-connection-validation-model-issue-18.md)（`PortCapability::ProbePoint`）。
* 関連: [RT 契約 ADR](realtime-contract-and-verification-policy.md)（API 単位の RT 保証・検証 lane）、[media-observation ADR](media-observation-non-destructive-pcm.md)（`Fact<T>` 分類パターン）、[#104 RT 観測 PoC](https://github.com/scarecrow173/ruviform/issues/104)（コスト実測）。
* 実装 spec: [issue-185-generic-probe-rt-capture.md](../specs/issue-185-generic-probe-rt-capture.md)。
