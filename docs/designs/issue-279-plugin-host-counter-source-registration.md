---
id: "DESIGN-034RTpdouoLLSINNczwa3c"
type: "design"
status: "approved"
title: "Issue #279 OBS-04 Plugin Host 4 CounterのSource登録: Registrar注入・Descriptor・Lease寿命"
created: "2026-11-04"
updated: "2026-11-04"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/279"
    - "https://github.com/scarecrow173/ruviform/issues/275"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-279-plugin-host-counter-source-registration.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-277-observability-core-source-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-121-l1-diagnostics.md"
    - "docs/designs/issue-117-bundle-child-process.md"
    - "docs/adr/realtime-contract-and-verification-policy.md"
    - "docs/adr/public-enum-non-exhaustive-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-279-plugin-host-counter-source-registration.md"
    - "docs/designs/issue-277-observability-core-source-contract.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-279-plugin-host-counter-source-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/121"
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/278"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/289"
    - "https://github.com/scarecrow173/ruviform/issues/278"
  deferred-by: []
---

# Issue #279 OBS-04 Plugin Host 4 CounterのSource登録: Registrar注入・Descriptor・Lease寿命

## Context

`ruviform-plugin-host` は Issue #121 以来、L1 データプレーンの 4 診断 Counter を
`ruviform_observability::counter()` ペアとして保持する。所有構成は
`AudioCounters`（`l1/mod.rs`・`InstKind::Audio` arm）が 4 cell を所有し、
writer は `RtLegCounters`（3 本・`RtLeg` へ clone）+ `results_dropped`
（`InstShared::push_result` が `inner` lock 内で書く）、reader は
`AudioCounterReaders`（clone 共有 cell）として query 面へ出ている。

OBS-02（Issue #277）で `ObservationRegistrar`/`RegistrationLease`/
`SourceLifecycle` の契約が着地したが、実 Consumer は未接続のまま。
本設計は Plugin Host（Domain 側）が契約を消費する**接続面だけ**を固定する。
OBS-07 Collector（#289）・OBS-03 tracing_export（#278）は未実装であり、
本設計はそれらの実装を前提としない。

## Scope

### In Scope

- `src/l1/sources.rs`（新規・`pub(crate) mod sources`）: Domain 宣言の
  4 `MetricDescriptor`、登録結果 bundle `AudioSourceLeases`、
  per-slot 状態 `AudioSourceSlot`。
- `l1/mod.rs`: `create_instance` に `registrar: Option<&dyn ObservationRegistrar>`
  を追加し、`InstanceParts`/`L1Instance` に `source_leases` を配線、
  `destroy` で retire。
- `composite.rs`: `PluginHost.observation_registrar` フィールド +
  `set_observation_registrar` setter、`instantiate` 両 arm と
  `instantiate_bundled`/`create_on_fresh` への経路。
- 上記の決定的試験（`l1/sources.rs` mod tests + `create_instance` harness 試験）。

### Out of Scope

- OBS-07 Collector 実装（Registry/Poll/Baseline/Rate/History/Snapshot）、
  OBS-03 tracing 変換、OBS-05/09/11。`ObservationRegistrar` ポートの
  Domain 側消費のみ。
- L0（in-process）経路・video 経路の Source 登録（`InstKind::Video` は
  counter を持たない構造的不変条件・`VideoInstanceParts` は別型）。
- `OutOfProcessSpec`/`SpawnSpec`/`RestartRecipe` へのフィールド追加。
- `audio_counters`/`AudioCounterSnapshot`/`diagnostics_query` 等の公開 API 変更。

## Components and Boundaries

### Registrar の注入面 — `diag` と同じ経路規約

`PluginHost` に `observation_registrar: Option<Arc<dyn ObservationRegistrar>>`
を保持し、`set_observation_registrar(&mut self, Option<Arc<dyn ObservationRegistrar>>)`
で接続/切断する。`PluginHost::new()` は `None`（未接続 = 未登録の既存互換）。

registrar は **引数経路**で mint に届ける（`diag: &DiagRegistry` と同じ規約。
spec 経路に埋めると `set_*` が routes 全体を走査する必要があり、host 全体の
「現行の接続」が route ごとに残存し得る不整合を生む）:

```text
PluginHost::instantiate
├── Dedicated: L1Instance::spawn(spec, &self.diag, registrar)
│     └── L1Child::spawn_scanned(&spec.view(), diag)
│     └── child.create_instance(spec, depth, diag, registrar)
└── Bundled: instantiate_bundled(spec, pool, &self.diag, registrar)
      ├── (pool hit) child.create_instance(spec, depth, diag, registrar)
      └── create_on_fresh(spec, pool, depth, diag, registrar)
            └── L1Child::spawn_scanned(&spec.view(), diag)
            └── child.create_instance(spec, depth, diag, registrar)
```

`Supervisor::attempt` は公開 `HostCommand::Instantiate` を撃つだけなので
host 保持の registrar で自動的にカバーされる（restart = 新 mint = 新登録）。
`Option<&dyn ObservationRegistrar>` は借用で十分（登録は同期・所有権不要）。

### 登録タイミング — `went_live` flip 直後、`inner` lock の外

`create_instance` は mint → driver install → wire `CreateInstance` →
`Ok` reply → `Creating → Live` flip（inner lock 内）→ `InstanceParts` 返却の
順。登録は **Live flip 成功時のみ**、同じ lock で取り出した reader clone を
lock 解放後に `register` する:

- 失敗パス（wire Err・timeout・mid-mint converge の fold）では一度も登録
  しない → rollback で退避すべき Source は存在しない。「live になった
  instance のみが Source を持つ」= facade 寿命と Source 寿命の対応が最も素直。
- lock 内で `register` を呼ばない（collector 実装が内部 lock を持つ可能性
  があり、長い呼出で `inner` を占有しない）。reader clone は lock 内で取得。
- `InstKind::Video` arm / `create_video_instance` は counter を持たない構造的
  不変条件のまま — registrar は audio 経路にしか届かない。

### `AudioSourceLeases` — 登録結果と退避の owner

```rust
pub(crate) struct AudioSourceLeases {
    slots: [Option<AudioSourceSlot>; 4],
    retired: Vec<RetiredSource>,
}

enum AudioSourceSlot {
    Registered(RegistrationLease),     // Active 登録
    Refused(RegistrationError),        // typed degradation（plugin は継続）
    Unattached,                        // registrar 未接続 — 意図的未登録
}
```

- `register(readers: AudioCounterReaders, registrar: &dyn ObservationRegistrar)
  -> Self`: 4 counter 順に `MetricSource::new(descriptor, Instance, Counter(reader))`
  → `registrar.register`。slot ごとに `Registered`/`Refused` を記録 — **1 本の
  失敗は他 3 本を巻き込まない**（per-source typed degradation）。
- `unattached() -> Self`: registrar 未接続時。
- `retire(&mut self)`: 全 `Registered` lease を `lease.retire()`（`Active →
  Retiring`）し、`RetiredSource` ticket を `retired` に退避する — Issue の
  「Lease 退避」。ticket Drop は no-op だが、保持すると Collector drain 進行を
  観測できる（試験・将来の OBS-11 query のため）。
- `registered_count()`/`identities()`/`retired_lifecycles()` は試験観測用。

### `L1Instance::destroy` — writer quiescence → retire の順序

`destroy` は既に quiescence 段階を持つ:

```text
drain_endpoint_channels()  — facade rt leg の復帰（writer quiescence の宣言点）
if registered:
    request(DestroyInstance)
    remove_instance(inner, i)  — residual doorbell flush が results_dropped
                               を最後に書き得る点。InstShared drop で writer 消滅
    send_driver(RemoveInstance)
★ self.source_leases.retire()  — 全 owner 側 writer が消えた後
rt_state.store(Destroyed)
last-referrer teardown
```

`remove_instance` の内部で residual doorbell flush が `results_dropped` を
最後に increment し得るため、retire は `remove_instance` 完了後（owner 側
writer の最終 write 後）に置く。残る writer は detached endpoint の `RtLeg`
clone のみ — destroy 後に残存 endpoint が write した分は Collector 側の
「最終回収不能は Partial」が担う（本設計で隠さない・偽装しない）。

`Drop for L1Instance` → `destroy()` — 未退避 drop は存在しないが、Lease 自体の
無退避 Drop は `abandon()` で可視化される（契約の網）。

### `InstanceParts`/`L1Instance` — lease は facade 寿命と一致

`RegistrationLease` は `Send` のみ（`Sync` 非保証）— `InstShared`/`Shared`
（driver/reader thread 共有）に置かず、facade `L1Instance`（`!Send`）に
`InstanceParts` 経由で渡す。**登録は create した thread で完結**、退避も
facade thread で行うので `Send` のみで足りる。

### Domain 宣言の 4 MetricDescriptor

`l1/sources.rs` が descriptors() で供給。`CounterDescriptor`（semantics 非保持）
ではなく `MetricDescriptor` を直接使い、`SourceSemantics` を宣言する:

| key | unit | reporting | semantics（要点） |
|---|---|---|---|
| `l1.dropped_submits` | Count | OnIncrease{1, 1s, High} | Submit 不成立（busy slot / full channel）・block 最大+1・XRUN 非混同。正常時 0 の fault signal |
| `l1.passthrough_submits` | Count | SnapshotOnly | 実代替出力 block 数・warmup 込み。warmup 増分は正常で無条件 WARN 禁止（meaning に明記） |
| `l1.results_failed_observed` | Count | OnIncrease{1, 1s, High} | RT が `RESULT_FAILED` 付き result を初回 pop で観測した数。全 Failure 発生数ではない |
| `l1.results_dropped` | Count | OnIncrease{1, 1s, High} | Reader 側 enqueue 失敗の破棄数。全結果欠落数ではない |

共通: `kind=Counter`・`aggregation=Cumulative`・`scope=Instance`・
`confidentiality=Internal`・`origin="ruviform-plugin-host::l1"`・
`temperature=Warm`・`expected_range=None`（累積 Counter の値域は無限）。
`warmup=None` — warmup は `lookahead_blocks` 個の block であり mint 時点で
sample rate 未定のため `Duration` 化できない（Deferred）。post-warmup の
passthrough 増加は `results_failed_observed`/`results_dropped` 側の
OnIncrease が拾う（Issue の「別条件で判定」）。min_interval=1s は Domain
宣言ヒントであり保証ではない。

### Source epoch = Counter cell epoch

- **登録境界（新 `SourceIdentity`）**: mint（`create_instance` の Live flip）
  — `AudioCounters::new` が新 cell を作る唯一の点。re-mint（`PluginHost::mint`
  の generation+1・`Supervisor::attempt` の再 Instantiate）→ 新
  `AudioCounters` → 新登録。
- **退避境界**: destroy / failed-create rollback（rollback では未登録 —
  Live flip 未到達）。`converge_child`/`converge_instance` の Failed は cell
  を殺さない → Source 継続（既存 `audio_counters_outlive_legs_remint_at_zero`
  の契約と一致）。
- **非境界**: leg swap（`arm`/`arm_apply`）・endpoint checkout/return は cell
  を共有し続ける — 再登録しない。同一 cell の `CounterReader` clone 再登録は
  `SourceReused` 競合かつ「同一 cell の 2 identity」で誤 Series 合成の構造的
  原因になるため。

## Data and Control Flow

```text
mint (audio, any route):
    spec/depth → L1Child::create_instance(spec, depth, diag, registrar)
        wire CreateInstance Ok → inner lock: entry Creating→Live,
            leg_counters=counters.leg.clone(), readers=counters.readers().clone
        registrar==Some → AudioSourceLeases::register(readers, reg):
            4 × MetricSource{descriptor, scope:Instance, Counter(reader)}
            → reg.register → Registered(lease) | Refused(err)
        registrar==None → AudioSourceLeases::unattached()
        → InstanceParts{…, source_leases} → L1Instance::from_parts

destroy / Drop:
    drain_endpoint_channels → remove_instance（最後の owner write）
    → source_leases.retire()（全 Registered → Retiring、ticket 退避）
    → Collector（着地後）が FinalDrain → Removed を駆動
```

## Non-Functional Constraints

- RT 不変: `process`/leg writers は一切変更しない。`counter()`/`Counter`/
  `CounterReader` の既存契約（Relaxed・no-alloc・単一 Counter 単調増加・
  複数 Atomic 非同時 Snapshot 非保証）を維持。
- 登録・retire・Lease Drop は全て非 RT（`create_instance`/`destroy`/`Drop`
  は全て既存の非 RT path — alloc/lock を使う既存経路に合流）。
- 新規外部依存なし（`ruviform-observability` の `source` module は default
  build・std のみ）。`logging` feature 非依存。
- 全公開 enum: `AudioSourceSlot` は `pub(crate)`（クレート内部限定）のため
  `#[non_exhaustive]` 対象外（ADR の「クレート外到達可能な全公開 enum」）。
  `RegistrationError` 自体は OBS-02 側で `non_exhaustive` 済み。

## Risks and Trade-offs

- **同名 key・異 Instance**: `scope=Instance` + `&'static` key は OBS-02 の
  cardinality 制限内。同名 key の区別は `SourceId`/`SourceGeneration` に委ねる
  （Collector が key+identity で Series を合成する前提）。`KeyConflict` を返す
  Collector 実装には typed degrad として `Refused` slot が残り、誤 Series 合成
  fallback は作らない。
- **登録失敗の可視性**: `Refused(RegistrationError)` を lease bundle に保持し
  `PluginHost` 側の既存 Query は変更しない。外部への新 query API は OBS-11
  （integration/購読）の責務として追加しない — 失敗は Collector が知っている
  （型付きで返している）。
- **warmup の `Duration` 非表現**: `SourceSemantics.warmup` は `Option<Duration>`
  のみで block 数を書けない。mint 時点で sample rate 未定のため `None` + meaning
  に明記する選択 — Activate 後に semantics を差し替える設計は descriptor の
  immutable 性を損なうので採らない。
- **`send_driver(RemoveInstance)` 後の退避**: detach された endpoint leg が
  destroy 後も write し得る — OBS-07 の「最終回収不能は Partial」責務で、
  host 側では retire を quiescence 宣言点（remove_instance 後）に置く正直な
  範囲に留める。

## Deferred Design Concerns

- `AudioSourceLeases` の `retired` ticket から公開 Removed 観測 query |
  OBS-11 integration 責務 | #293 | 現段階で public surface を増やすと
  Collector 未存在の仕様を先行公開することになる。
- warmup `Duration` の Activate 後 semantics 供給 | spec Deferred と同じ。
- post-warmup passthrough 異常増加の OnIncrease 細分化 | OBS-09 Diagnostics
  の warmup 条件解釈の設計確定後。
- `PluginHost` で `RegistrarStopped` 後の再接続ポリシー（再 `set_*`）|
  OBS-11 runner の lifecycle 支援責務 | #293。

## Verification Notes

- [ ] `mise run fmt`/`check`/`lint`/`test`/`test-rt` が全 pass。
- [ ] 既存 counter/lifecycle/endpoint/e2e 試験が無改変で pass（RT/ABI 回帰なし）。
- [ ] `l1/sources.rs` mod tests: register 4 slot Active・per-slot Refused・
  unattached・retire 順序・ticket 退避。
- [ ] `create_instance` harness 試験: registrar 接続時に mint で 4 Source が
  登録される・registrar None で未登録・registrar error で create が成功継続・
  failed-create rollback で登録なし。
- [ ] destroy で writer quiescence → retire の順序が観測できる（state が
  Retiring/Removed に進む）。
- [ ] 同名 key・異 Instance（harness で 2 mint）で SourceId が区別される
  （誤 Series 合成なし）。
