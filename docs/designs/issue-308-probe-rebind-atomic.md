---
id: "DESIGN-034Ui3n3nIZNFJKLyIv0Uf"
type: "design"
status: "approved"
title: "Issue #308 — Probe再Bindingのstaged all-or-nothing設計"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/308"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-308-probe-rebind-atomic.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-185-generic-probe-typed-runtime-observation-rt-capture.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/realtime-contract-and-verification-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-308-probe-rebind-atomic.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/309"
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/309"
  deferred-by: []
---

# Issue #308 — Probe再Bindingのstaged all-or-nothing設計

## Context

`typed::probe`（Issue #185）は non-RT の `ProbeBroker` が `ProbeCommand` を bounded SPSC channel 経由で RT の `ProbeDeck` へ送り、`RtRuntime::process` 内の `drain_commands` が generation guard 付きで適用する。`on_publish` は現行 `ClearAll{generation}` + 個別 `Bind{generation, binding}×N` を逐次 `try_push` し、途中の `CommandChannelFull` で部分適用が残る。Generation guard は「旧 slot を新 PCM として誤観測しない」ことを既に保証しているが、「適用の全件性」と「失敗の型付き報告」は保証していない。

制約: RT 側は no alloc / no block / no destructor。`ProbeDeck` の `bindings` は `Vec::with_capacity(max_probes)` で確保済みの arena であり、`clear`・in-place 置換・capacity 内 `push` のみが許される。

## Scope

### In Scope

- `crates/ruviform-audio/src/typed/probe.rs` のみ（`runtime.rs`/`mod.rs`/`spsc.rs` は変更不要と判断 — 下記「代替案と却下理由」参照）。
- `crates/ruviform-audio/tests/typed_probe.rs` の拡張。

### Out of Scope

- `GraphRuntime::publish` 自体の command 列（`RuntimeCommand::Install/Remove`）— publish 失敗時は plan が activate されず generation guard が stale drop するため既に安全（last-good rollback）。本 Issue は publish **成功後**の probe 側だけを扱う。
- Clock Epoch・Latency 比較（#309）、RT 観測基盤（#280）、Collector 集約（#289/#107 Consumer 側）。

## Components and Boundaries

### Staged rebind（世代付き staged slots）

`ProbeDeck` に第2の binding arena `staged: Vec<ProbeBinding>`（`pair()` で `max_probes` capacity 確保、allocation なし）と `staging: Option<u64>`（開いている attempt の generation epoch）を追加する。`bindings`（live）と `staged`（非 live）は完全に分離され、**staged は Commit まで観測経路に一切関与しない**。

`ProbeCommand` の generation-scoped 系を次の3変種へ置き換える（`ClearAll` は廃止）:

| Command | RT 適用条件 | 効果 |
| --- | --- | --- |
| `RebindBegin { generation }` | `g == plan.generation` | `staged.clear()` + `staging = Some(g)`。stale は drop + `record_stale_commands` |
| `RebindStage { generation, binding }` | `g == plan.generation && staging == Some(g)` | `staged` 内を probe id で upsert（replace または capacity 内 push）。溢れたら `staging = None` で attempt を無効化 + `lost_records` 計数 |
| `RebindCommit { generation }` | `g == plan.generation && staging == Some(g)` | `staging = None` → **atomic swap**（後述）→ status 発行 |

`install()` が使う `Bind{generation, binding}` と `SetMode`/`StartCapture`/`StopCapture`/`Remove` は変更しない。これらは live `bindings` にのみ作用し、staging window とは独立。

### Commit の atomic swap

`RebindCommit` 適用時、`core::mem::swap(&mut self.bindings, &mut self.staged)` で live/staged を丸ごと交換する。両 Vec は `max_probes` capacity を持つため再確保は起きず、RT 契約（no alloc）を守る。swap 後 `self.staged`（旧 live 集合）に対して status を発行し、最後に `self.staged.clear()`（capacity 保持）で staging を閉じる。

status 発行（全て `push_status` 経由、telemetry channel overflow は `lost_records` 計数の既存動作）:

- staged 内の各 binding → `StatusKind::Installed`（従来の `Bind` と同じ「RT Installed」証拠）。
- 旧 live にあって staged に無い probe → 新設 `StatusKind::RebindDropped`（point が新 plan で解決不能になった明示通知。broker は probe を installed のまま保持し、次回 publish で再解決を試みる）。
- 旧 `mode == Capture` → 新 `mode != Capture` → `StatusKind::CaptureStopped { reason: StopReason::GenerationBoundary }`（新設 variant）。
- 新 `mode == Capture`（旧が Capture/非 Capture いずずれでも）→ `StatusKind::CaptureStarted { budget_frames }`（新 segment の明示開始。`RecordMeta.generation` が旧 segment と区別する）。

Commit は `drain_commands` 内・当該 callback の `run_site` より前に完結するため、観測側からは「旧集合が全部残る」か「新集合が全部適用される」の二値にしか見えない — これが all-or-nothing の実体。

### Broker 側: `on_publish` → `RebindReceipt`

`ProbeBroker::on_publish` は以下を行う:

1. `runtime.published_plan()` から `generation` を取得。
2. `self.probes` 全件を `resolve_point` で再解決し、解決できたものを `ProbeBinding`（`bound_generation = generation`）として staged Vec に構築。解決不能は `receipt.unbound` へ。
3. Capture Policy を適用（後述）して staged binding の `mode`/`capture_remaining` を決定。
4. `RebindBegin` → `RebindStage×N` → `RebindCommit` を順に `try_push`。**いずれかが失敗したら `Err(ProbeError::CommandChannelFull)` を返す** — その時点で queue に残るのは Commit を持たない staged prefix のみで、live bindings は一切変更されない（= PendingRetry 意味論。doc に明記）。
5. 成功時 `Ok(RebindReceipt)` を返す。

broker は `&mut self` で逐次呼ばれるため、Begin〜Commit の間に他の broker コマンドが queue に割り込むことはない（staging window は broker 呼出し単位で atomic）。

### `RebindReceipt`（GUI-readable Receipt）

```rust
pub struct RebindReceipt {
    pub generation: u64,
    /// staged に載った probe（commit で Installed になる見込み）。
    pub bound: Vec<ProbeId>,
    /// point が解決不能で staged に載らなかった installed probe
    /// （commit で RebindDropped になる見込み）。
    pub unbound: Vec<ProbeId>,
    /// Capture が境界で切断される見込みの probe。
    pub capture_stopped: Vec<ProbeId>,
    /// Capture が新 budget で再開される見込みの probe。
    pub capture_restarted: Vec<ProbeId>,
}
```

Receipt が覆う段階と、残りの段階の観測経路:

| 段階 | 観測経路 |
| --- | --- |
| User Requested | `on_publish` 呼出し自体（Consumer 側） |
| Enqueued | `Ok(RebindReceipt)`（全件 enqueue 済み）または `Err(CommandChannelFull)` = PendingRetry（queue に Commit 無しの inert prefix のみ） |
| RT Installed | telemetry `StatusRecord`（commit 時の per-probe `Installed`/`RebindDropped`/`CaptureStopped`/`CaptureStarted`、`generation` で照合） |
| Observed | `MetricRecord`/`CaptureChunk` の `meta.generation == receipt.generation` |
| Plan superseded / late generation | commit 前に更に publish が進むと `g != plan.generation` で stale drop → `SharedStatus::stale_commands` + `RuntimeFault::StaleDiscarded`、当該 generation の `Installed` が出ないことで判別 |
| shutdown/cancel | RT が drain しない限り commit は Pending のまま — Receipt/Status が来ないこと自体が未適用の正直な表現（Installed を受け取るまで成功と見なさない契約） |

### Capture 継続 Policy

```rust
pub enum CapturePolicy {
    /// 境界で Capture を切断: staged mode = Monitor、残 budget は
    /// 破棄（新世代へ繰越さない）。commit で
    /// CaptureStopped{GenerationBoundary} を発行。既定。
    StopAtBoundary,
    /// 新世代で満額 budget の Capture を明示再開: staged mode =
    /// Capture、capture_remaining = budget_frames（残 frame ではなく
    /// 満額 = 新 segment）。commit で CaptureStarted を発行。
    Restart,
}
```

- `ProbeEntry` に `capture_policy` を追加（install 時 `StopAtBoundary`）。`ProbeBroker::set_capture_policy(probe, policy)` で probe 毎に変更（RT command を伴わない — on_publish の resolve 時に消費されるため）。
- `Stop` の RT 適用 Ack 契約は既存機構で成立: `stop_capture` → `StopCapture` command → RT 適用 → `CaptureStopped{Requested}` status が telemetry に出る。Consumer はこの status を collector で受けてから publish すれば「RT 上で停止済み」を確認できる。publish が StopCapture の drain に先行しても FIFO 順で StopCapture → Begin → … → Commit と適用され、整合する。
- 旧 Generation 残 frame の繰越し禁止: 現行 `capture_remaining = entry.budget_frames` の無条件再設定を廃止。`StopAtBoundary` は残 budget を破棄、`Restart` は**満額** budget で新 segment を開始する（残 frame の継続ではない）。

### doc/impl 不一致の修正

現行 `on_publish` doc の「their entry flips to `Off`」は実装に存在しない。新 doc では「unresolved probe は staged に載らず、commit で `RebindDropped` status が発行される。broker の `probes` には残り、次回 publish で再解決される」と実装どおりに記述する。

## Data and Control Flow

```
non-RT                                        RT (process 内 drain_commands)
───────                                       ──────────────────────────────
on_publish(runtime, spec)
  ├─ resolve_point × probes ──► staged Vec 構築（非 live）
  ├─ capture policy 適用 ────► mode/capture_remaining 決定
  ├─ try_push RebindBegin{g} ───────────────► staging = Some(g), staged.clear()
  ├─ try_push RebindStage{g,b} × N ─────────► staged upsert
  └─ try_push RebindCommit{g} ──────────────► swap(bindings, staged)
                                               ├─ Installed × N
                                               ├─ RebindDropped（旧のみ）
                                               ├─ CaptureStopped{GenerationBoundary}
                                               └─ CaptureStarted（新 Capture）
                                            run_site …（新 bindings のみ観測）
```

失敗時: 途中 `try_push` 失敗 → `Err(CommandChannelFull)`。queue に残る staged prefix は Commit が無いため live に影響しない。retry は新しい Begin で staged を上書き開始するため安全。

## Non-Functional Constraints

- RT 契約維持: `RebindStage`/`RebindCommit` の適用は `Vec::clear`/upsert/`mem::swap`/`clear` のみ — capacity 内で allocation/deallocation なし。`staged` は `pair()` で確保。
- `bindings`/`staged` の二重化で binding メモリは最大 `2 × max_probes × sizeof(ProbeBinding)` に増える（既定 32 probes で数 KB — bounded）。
- command channel の必要容量は `2 + N`（Begin + N Stage + Commit）。従来 `1 + N` より 1 slot 増。失敗は全て PendingRetry で安全側。
- telemetry channel overflow 時の status 欠落は既存 `lost_records` counter で観測（部分的な Installed 欠落は「全件成功に見せない」要件と整合 — counter が欠落を明示）。

## Risks and Trade-offs

- **staged の stale 残留**: Commit を持たない attempt の staged 内容は live に影響せず、次の Begin で clear される。`staging` epoch check で attempt 外の Stage/Commit を拒否。
- **同 generation retry**: publish は generation を進めるため通常は一意。万一同 generation で retry しても Begin が staged を clear してやり直すため冪等。
- **`RebindDropped` の意味**: commit で「旧 live にあって staged に無い」= point 未解決（または remove 競合の順序差 — queue FIFO で最終状態は正しい）。probe 自体は broker に残り、次回 publish で復活し得る点を doc に明記。
- **capture policy の broker/RT 乖離**: `set_capture_policy` は RT command を送らない。policy は on_publish の staged mode 決定にのみ効くため乖離は発生しない。publish を伴わない policy 変更は何も起こさない（doc 明記）。
- **`install(mode = Capture)` の既存 quirk**: install 直後の `capture_remaining = 0` は本 Issue の対象外（既存挙動を維持。`capture()` 経由が正規経路）。

## Alternatives Considered

- **単一 batch command（`Rebind{generation, bindings: Vec<ProbeBinding>}`）**: queue slot は 1 つで済むが、`Vec` の heap 確保が non-RT 側で行われ RT 側での drop が destructor になる。返却路（recycle channel）の追加が必要で複雑化する。staged slots は全 command が POD のまま RT 契約を素直に守れるため却下。
- **`Producer::try_push_batch` / spsc.rs 拡張**: rtrb の `push_slice` は部分 push を許し、かつ free slot 数を事前照会する API が無いため all-or-nothing を保証できない。さらに `ProbeCommand` が `Copy` でないと slice push できず、variant 追加ほどの意味がない。却下。
- **ClearAll+Bind のまま容量事前確認**: free slot 数を知る術が無く、確認と push の間に consumer が pop する競合もある。staged 方式は失敗しても無害なのでこの問題自体が消える。却下。

## Deferred Design Concerns

- Clock Epoch / port identity を含む Segment の品質分類（Gap/Unverified の完全な語彙）| #309 PROBE-TIME・#280 OBS-05 が Owner | 各 Issue 着手時 | epoch を跨ぐ区間を verified と誤認する恐れ（本設計は generation/port identity の区別まで）
- GUI 側での Receipt/Status の集約表示 | Synvelis SF-04/SF-06 | Consumer 実装時 | API は提供されるが表示は Consumer 依存

## Verification Notes

- [ ] `typed_probe.rs` 拡張テストで queue 飽和点（Begin 前/直後/Stage 途中/Commit 前）・連続切替・Capture 境界・Golden 一致を検証
- [ ] `mise run check` / `lint` / `fmt-check` / `test` / `test-rt` 全緑
