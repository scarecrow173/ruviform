---
id: "ADR-034NVCXBHjsHlfixAwUSHE"
type: "adr"
status: "accepted"
title: "video/NONRT プラグイン L1 の別立て host 表面・plugin_id 名前空間・Dedicated 固定（Issue #118）"
created: "2026-10-29"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/118"
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
    - "docs/specs/issue-119-l1-wire-version-policy.md"
    - "docs/adr/issue-117-bundle-child-failed-granularity.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-118-video-l1.md"
    - "docs/adr/issue-118-video-l1-create-path-segment.md"
    - "docs/designs/issue-117-bundle-child-process.md"
    - "docs/adr/plugin-isolation-strategy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-118-video-l1.md"
    - "docs/discovery/issue-118-video-l1.md"
    - "docs/designs/issue-117-bundle-child-process.md"
    - "docs/adr/plugin-isolation-strategy.md"
  derived-by:
    - "docs/designs/issue-118-video-l1.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/117"
    - "https://github.com/scarecrow173/ruviform/issues/119"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# video/NONRT プラグイン L1 の別立て host 表面・plugin_id 名前空間・Dedicated 固定（Issue #118）

## Context and Problem Statement

Issue #118 は `VideoPluginInstance`（`crates/ruviform-plugin-core/src/video.rs` — 全操作 NONRT の別 trait）を L1 子プロセスで動かす経路を確立する。公開 API の 3 論点がここに集約する:

- **host 表面の形** — audio `HostCommand`/`HostReply`（`crates/ruviform-plugin-host/src/command.rs`）は `Instantiate{plugin_id}` + lifecycle/state/GUI 動詞の audio 形であり、`VideoProcessConfig`・`render{input,output,at}`・plain param 動詞は語彙に存在しない。人間確定済みの前提は「audio `HostCommand`/`Instantiate` には video 動詞を足さない」— 別立て video 表面の**正確な形**を確定する必要がある
- **`plugin_id` 名空間** — `PluginHost.routes` は `plugin_id → Route` の単一 map（`composite.rs` L331）で L0/L1 audio route を引く。video route を同名空間に置くか別名空間に置くかで、登録衝突・`Scan`/`Instantiate` の見え方・handle の一意性の契約が変わる
- **isolation 選択面** — 人間確定済み: video-capable instance は `OutOfProcessIsolation::Dedicated` のみ。`OutOfProcessSpec.isolation` は既定 `Bundled` なので、video spec がこの字段をそのまま持つと「既定で禁止形になる」危ない構造になる — 拒否の**仕方**を確定する必要がある

加えて `PluginHandle{slot,generation}` は `PluginHost` の単一 slot/generation table で mint される opaque 値（`boundary.rs` L25-59）であり、独立した第 2 の host 型を持つと 2 系統の handle が値として alias し得る構造上の問題がある。

## Decision Drivers

* audio `HostCommand`/`HostReply`/`PluginHost::execute`・`OutOfProcessSpec`/`OutOfProcessIsolation` の意味を**無変更**に保つ（spec R-B6 — audio 経路無退行）
* `PluginHandle` は全 surface で一意に slot を指す — alias 可能な第 2 mint を作らない
* `Failed`+`InstanceFailed` 通知・`lifecycle_state`・`kill_child` の機構を audio/video で二重化しない（#117 確定契約をそのまま使う）
* `Bundled` + video は構造的に表現不能にする（runtime 拒否ではなく「書けない」ことを選ぶ — 最強の defined 拒否）
* `publish = false` の 0.1.0 未公開 API — additive な拡張は許容されるが、静かな既定値退化（`isolation` が既定 `Bundled` のまま video で禁止になる）は避ける

## Considered Options

* **Option A — `PluginHost::video() -> VideoPluginHost<'_>` 借用 façade**（採用）: `PluginHost` が `video(&mut self)` で `&mut` 借用の video 表面 view を返し、`VideoHostCommand`/`VideoHostReply`/`VideoOutOfProcessSpec` を別 enum/struct として `execute`/登録を提供。内部の slot table・routes map・通知機構は共有
* **Option B — 独立した `VideoPluginHost` 型**（`PluginHost` を内包する separate host）: 完全分離だが handle space が 2 系統に分かれ、`{slot,generation}` alias による誤用経路を構造的に排除できない。共有機構（mint・notify・`Failed`）の二重化も要る
* **Option C — `HostCommand`/`HostReply` への video variant 追加**: `Instantiate{plugin_id, kind}`・`Render{...}` を audio enum に additive 追加 — 人間確定済みの「audio 表面を拡張しない」を直接踏み破る → 却下
* **Option D — 別 `plugin_id` 名空間**（audio/video で独立した登録表）: 同一 id を audio route と video route に重複登録できてしまい、id が plugin の同一性を表す既存不変式（`DuplicatePluginId` で守られる 1 id = 1 route）を弱める → 却下
* **Option E — `VideoOutOfProcessSpec.isolation` 字段を残し `Bundled` を実行時 `Err` で拒否**: `OutOfProcessSpec` 再利用の対称性はあるが、既定値が禁止形になる「書けば書けるが必ず失敗する」公開面を作る。字段自体を持たせる理由がない → 却下

## Decision Outcome

Chosen option: "**Option A — `PluginHost::video() -> VideoPluginHost<'_>` 借用 façade**、単一 `plugin_id` 名空間に kind タグ付き route、Dedicated 固定の `VideoOutOfProcessSpec`（`isolation` 字段なし）"。

確定する公開契約:

### 1. video 表面の形 — `PluginHost::video(&mut self) -> VideoPluginHost<'_>`

- `ruviform-plugin-host` に新 module `video`（仮称 `src/video.rs`）を追加し、`VideoHostCommand`/`VideoHostReply`（両 `#[non_exhaustive]`）・`VideoOutOfProcessSpec`・`VideoPluginHost<'a>` を定義する。`VideoPluginHost<'a>` は `&'a mut PluginHost` を保持する薄い view で、`register_out_of_process(spec)`・`execute(VideoHostCommand)`・`lifecycle_state`/`kill_child`/`try_recv_notification`（`PluginHost` 本体へ delegate）を持つ。`descriptor(handle)` は**新規 surface** — `PluginHost` に同名メソッドは存在しない（delegate 先はなく、`descriptors` 登録表から route 非依存に引く薄い lookup として video view 側に実装する）
- `VideoHostCommand` の動詞集合: `Scan`・`Instantiate{plugin_id, config: VideoProcessConfig}`・`Destroy{handle}`・`Initialize`・`Activate`・`StartProcessing`・`Render{handle, input: VideoFrame, at: MediaTime}`・`StopProcessing`・`Deactivate`・`GetParams`・`GetParamInfo{handle,id}`・`GetParamValue{handle,id}`・`SetParamValue{handle,id,value}`（詳細は design `docs/designs/issue-118-video-l1.md`）
- audio `HostCommand`/`HostReply`/`PluginHost::execute`/`OutOfProcessSpec`/`OutOfProcessIsolation` は**無変更**。`PluginHost::video()` の追加のみ

### 2. `plugin_id` 名空間 — 単一 map・kind タグ付き route

- `PluginHost.routes` は `plugin_id → Route` の単一 map のまま。`Route` に `OutOfProcessVideo{spec: VideoSpawnSpec}` variant（`pool` 字段なし — Dedicated のみで pool は要らない）を追加する。`VideoSpawnSpec` は **新規 minimal 型**で audio `SpawnSpec` を再利用しない — `lookahead_blocks`/`instance_depth()` は audio doorbell ring の概念であり video では dead field になる（字段構成は `{descriptor, program, args, command_timeout, render_timeout}`）
- 登録衝突: audio `register_factory`/`register_out_of_process` と video `register_out_of_process` が同一 `plugin_id` を claim した場合は双方向とも **`Err(HostError::DuplicatePluginId)`**（map key 衝突の既存規則 — 1 id = 1 route = 1 kind）
- `Scan`: audio `HostCommand::Scan` は `InProcess`/`OutOfProcess` route のみ列挙し video route を隠す。video `VideoHostCommand::Scan` は video route のみ列挙（各表面は自分の kind の記述だけを返す — cross-kind id を instantiate 面に見せない）。実装上、`Scan` reply は現行 `self.descriptors.clone()` の全量返却（`composite.rs` L456）なので、**`descriptors` の各 entry が登録面の kind を tag として持つ**（audio 登録 → audio-kind・video 登録 → video-kind）必要がある — 無タグのまま video 登録を push すると video descriptor が audio `Scan` に漏れる。`Scan` reply は requester の kind の entry だけに filter する（shadowed 登録は自分の登録面の kind で残る — 現行の shadowed-listing 意味論を kind 内に維持）
- `Instantiate`: audio `Instantiate{plugin_id}` が video route を引く場合 → **`Err(HostError::UnknownPluginId)`**（その surface からは instantiate 不能な id = 未登録と同語彙）。逆方向も同型
- cross-kind 動詞: audio `HostCommand` が video slot を指す → **`Err(HostError::Unsupported)`**（「その instance は要求された操作を実装しない」既存語彙 — dispatch の slot 分岐で wire 非送信に拒否）。`VideoHostCommand` が audio slot を指す場合も同型。`prepare_endpoint` が video slot を指す → **`Err(HostError::EndpointUnavailable)`**（wire を通らない local-only 語彙が既存）
- `Slot` variant 追加に伴う全 match site の答えは design の「`Slot` match site 全網羅」表が正本 — 要点: `try_recv_notification`（`composite.rs` L873-877）は video `notify_rx` を drain する**必須 arm**（現行 `let Some(Slot::OutOfProcess(_))` は video slot を静かに skip し、`InstanceFailed` 通知が失われる）。direct `PluginHost` メソッドが video handle を受けた場合: `process` → `Err(Unsupported)`・`latency_frames` → `None`・`plugin_mut` → `None`・`lifecycle_state` → `Some(mirror)`・`kill_child`/`destroy` → 既存機構どおり動作

### 3. isolation — `VideoOutOfProcessSpec` に `isolation` 字段を持たせない（Dedicated 固定・構造的 pin）

- `VideoOutOfProcessSpec { descriptor, program, args, command_timeout, render_timeout }` — `lookahead_blocks`（audio RT leg の概念）も `isolation` も**持たない**。`Bundled` + video は公開面上**表現不能**であり、runtime 拒否 path は存在しない — これが spec R-F4 確定値（video = Dedicated のみ）の最強の pin 形
- `render_timeout`（既定 **60 s**）は `VideoRender` reply wait にのみ適用。`command_timeout`（既定 **10 s**）は handshake/lifecycle/param 動詞に適用（audio `command_timeout` と同規則 — timeout→`Err(Ipc)`+子 `Failed` 収束）
- 将来 Bundled video を許す変更が来る場合は `isolation` 字段の additive 追加 + `Bundled` 許容の HoL 文書化を**別 ADR**で行う（本 ADR はその解禁を予約しない — deferred 論題として design の deferred 表に記録）

### Consequences

* Good, because `PluginHandle` が全 surface で一意に slot を指す不変式を構造的に保つ（handle alias 経路が存在しない）— `Failed`/`InstanceFailed`/`lifecycle_state`/`kill_child`/`prepare_endpoint` 拒否は全て既存機構がそのまま効く
* Good, because audio `HostCommand`/`execute`/`OutOfProcessSpec`/`OutOfProcessIsolation` は完全に無変更 — 公開 API の additive 拡張のみ（`video()` accessor・`Route` variant・`Slot` variant・`VideoHost*` 型）
* Good, because `Bundled` + video を「書けない」形にしたことで、`isolation` 既定値の静かな退化（`OutOfProcessSpec` を video で再利用した場合に `Bundled` 既定が禁止形になる落とし穴）を構造的に排除した
* Bad, because `PluginHost` が「audio route」と「video route」の 2 kind を routes map に持つようになり、audio `Scan`/`Instantiate` が kind filter を必要とする — 単一 map を共有する代償（filter は小さいが、概念として「表面が見える route の subset」が増える）
* Bad, because `video()` の `&mut` 借用は audio `execute` 呼出しと compile-time で直列化する — 同一 `PluginHost` 上で audio/video 両面を交互に使う処理は `drop(view)` 跨ぎが要る（`PluginHost` が単一 control スレッド前提なので実害は限定的）
* Bad, because `VideoOutOfProcessSpec` が `OutOfProcessSpec` と別型になり spec の形が 2 種ある（`args`/`descriptor`/`program` は同名でも field 集合が違う — caller は kind ごとに spec 型を使い分ける）

### Confirmation

- `docs/designs/issue-118-video-l1.md` 決定サマリ 2-4 が本 ADR を参照しており、実装は design の Architecture 節「host 公開面」「host 内部」に従う
- 公開面の検証: audio `Scan` が video route を返さない・video `Scan` が audio route を返さない・cross-kind `Instantiate` → `Err(UnknownPluginId)`・双方向登録衝突 → `Err(DuplicatePluginId)`・cross-kind 動詞 → `Err(Unsupported)`・`prepare_endpoint` on video handle → `Err(EndpointUnavailable)`・`VideoOutOfProcessSpec` に `isolation` 字段がないことを型で確認（compile-time pin — `isolation` を書こうとするコードはコンパイルできない）
- `VideoHostCommand::Instantiate` が `VideoProcessConfig` を受けて `Err(HostError::Ipc)`/`Err(HostError::Prepare(PrepareError::UnsupportedConfig))`/`Err(HostError::InstanceLimit)` を返す分岐が design の contract どおり

## Implementation Plan

* Affected paths: `crates/ruviform-plugin-host/src/video.rs`（新規 — `VideoHostCommand`/`VideoHostReply`/`VideoOutOfProcessSpec`/`VideoPluginHost<'a>`・`descriptor` 新規 surface）・`crates/ruviform-plugin-host/src/composite.rs`（`Route::OutOfProcessVideo` variant・`video()` accessor・`descriptors` entry の kind-tag + Scan/Instantiate の kind filter・cross-kind dispatch 拒否・`try_recv_notification` の video `notify_rx` drain arm・`Slot` 全 match site の video 答え）・`crates/ruviform-plugin-host/src/lib.rs`（re-export）・`crates/ruviform-plugin-host/src/l1/`（`VideoL1Instance`・`vshm` module・`InstKind` 分離 — 詳細は design）
* Patterns to follow: `command.rs` の `HostCommand`/`HostReply` 形（`#[non_exhaustive]` + handle payload）・`composite.rs` の `Route`/slot/dispatch 構造・`OutOfProcessSpec::new` の既定値 field 形（`command_timeout`/`render_timeout`）・issue-117 design の additive `Route`/`HostError` 拡張
* Patterns to avoid: `HostCommand`/`HostReply` に video variant を足さない（人間確定済みの禁止形）・`VideoPluginHost` が独自の slot table/mint を持つ形（handle alias の温床 — 借用 view のみ）・`VideoOutOfProcessSpec` に `isolation`/`lookahead_blocks` 字段を足さない（構造的 pin）
* Dependencies or configuration: `ruviform_plugin_core::{VideoProcessConfig, PlainParamInfoOwned, PlainParamValue, ParamId}`・`ruviform_video_core::VideoFrame`・`ruviform_media_core::MediaTime` の `ruviform-plugin-host` への参照（既存の下向き依存方向と同じ — plugin-host は既に plugin-core に依存）。`ruviform-media-core` は現状 dev-dep のみ（`Cargo.toml` L39）→ `[dependencies]` への昇格が要る（`ruviform-audio-core` の #113 昇格と同じ経路・transitive 増加なし）。`ruviform-plugin-export` 側は `ruviform-video-core` + `ruviform-media-core` が新規/昇格で要る（`VideoEntry` が `VideoFrame`/`MediaTime` を構築 — create-path ADR 側の記録）

## Verification

* [ ] `cargo test -p ruviform-plugin-host` で kind filter（audio `Scan`/`Instantiate` が video route を見ない・video `Scan` が audio route を見ない — `descriptors` kind-tag による filter）・`DuplicatePluginId` 双方向衝突・cross-kind `Unsupported`・`EndpointUnavailable` をカバーする unit/E2E が追加され pass
* [ ] video handle に対する direct `PluginHost` メソッドの答え: `process` → `Err(Unsupported)`・`latency_frames`/`plugin_mut` → `None`・`prepare_endpoint` → `Err(EndpointUnavailable)`・`lifecycle_state` → `Some(mirror)`・`try_recv_notification` が video slot の `InstanceFailed` を fan-out する（skip しない）
* [ ] `VideoOutOfProcessSpec` の struct 定義に `isolation`/`lookahead_blocks` 字段がない（ソースレビューで確認 — compile-time pin なのでランタイムテストは要らない）
* [ ] `mise run lint`・`mise run test` が全 workspace で pass（audio 無退行）
* [ ] `doc_status.js audit --json` が blocking 0

## Pros and Cons of the Options

### Option A — `PluginHost::video() -> VideoPluginHost<'_>` 借用 façade

* Good, because `PluginHandle` が単一 slot table で mint され handle alias が構造上起きない・`Failed`/notify/`lifecycle_state`/`kill_child`/`prepare_endpoint` 機構をそのまま共有できる・audio 公開面は完全に無変更
* Bad, because `video()` の `&mut` 借用が audio `execute` 呼出しと compile-time で直列化する（audio/video 両面を同一 `PluginHost` で交互に使うコードは view の drop が要る）・`VideoHost*` 型が追加される分 public API surface が増える

### Option B — 独立した `VideoPluginHost` 型

* Good, because `PluginHost` と完全に分離でき `execute`/登録面の kind 分離が型レベルで自明になる
* Bad, because `PluginHandle{slot,generation}` が 2 系統の独立 table で mint され、**同じ値が両 host で有効な handle になり得る**（alias 安全弁がない）— 「この handle はどの host のものか」は caller の責務になり、`Failed` 通知・slot table の二重化も要る

### Option C — `HostCommand`/`HostReply` に video variant を足す

* Good, because surface が 1 つのままで `execute` の入口が一箇所になる
* Bad, because audio `HostCommand`/`HostReply` を変更し人間確定済みの「audio 表面を拡張しない」を直接踏み破る — `Instantiate` の payload を `config` 有りに変える必要もあり additive では済まない

### Option D — 別 `plugin_id` 名空間

* Good, because audio/video で id の衝突を型レベルで考えなくてよい
* Bad, because 「1 plugin_id = 1 plugin」という既存不変式を弱め（同一 id を audio route と video route に重複登録可能になる）、`Scan` で kind 混同の記述が返る経路を作る — 構造的な id 一意性を失わせる対価が大きい

### Option E — `VideoOutOfProcessSpec.isolation` 字段を残して runtime `Err` で拒否

* Good, because `OutOfProcessSpec` との形の対称性があり、将来 Bundled video を許す変更が字段追加なしで済む
* Bad, because 「書けるが必ず `Err` になる」公開面を作る（既定 `Bundled` で必ず失敗する API）— 構造的に表現不能にする方が強い拒否であり、runtime 拒否を用意する理由がない

## More Information

- design `docs/designs/issue-118-video-l1.md` 決定サマリ 2-4・Architecture「host 公開面」「host 内部」・Risks の借用 façade/単一 namespace/Dedicated の trade 説明
- spec `docs/specs/issue-118-video-l1.md` R-F4・R-F5・R-V5・AC5（Dedicated 限定の検証要求）
- discovery `docs/discovery/issue-118-video-l1.md` 軸 6（Dedicated 推奨）・Q2・Q9
- `crates/ruviform-plugin-host/src/composite.rs`（`Route`・`routes` map・`prepare_endpoint` の `Slot` 分岐）・`crates/ruviform-plugin-host/src/command.rs`（audio `HostCommand`/`HostReply`）・`crates/ruviform-plugin-host/src/boundary.rs` 相当の `boundary.rs`（`PluginHandle`/`HostError`）
- `docs/adr/issue-117-bundle-child-failed-granularity.md`（`Failed` 粒度・`isolation` 選択面の先行 ADR — 本 ADR は video では `isolation` を選択面にしない点で意図的に分岐）
