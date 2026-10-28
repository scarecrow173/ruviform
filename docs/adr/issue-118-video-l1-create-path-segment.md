---
id: "ADR-034NVCYbbmwSxZnvHYNkAx"
type: "adr"
status: "proposed"
title: "video/NONRT プラグイン L1 の create-path・segment sizing（CreateVideoInstance に VideoProcessConfig を載せる形）（Issue #118）"
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
    - "docs/adr/issue-118-video-l1-host-surface.md"
    - "docs/designs/issue-117-bundle-child-process.md"
    - "docs/designs/plugin-l1-out-of-process.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-118-video-l1.md"
    - "docs/discovery/issue-118-video-l1.md"
    - "docs/designs/issue-117-bundle-child-process.md"
    - "docs/designs/plugin-l1-out-of-process.md"
    - "docs/specs/issue-119-l1-wire-version-policy.md"
  derived-by:
    - "docs/designs/issue-118-video-l1.md"
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/76"
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

# video/NONRT プラグイン L1 の create-path・segment sizing（CreateVideoInstance に VideoProcessConfig を載せる形）（Issue #118）

## Context and Problem Statement

Issue #118 の video L1 経路は per-instance 専用 shm segment を使う（人間確定済み — spec R-F1・discovery Q1）。ところが audio と構造が違う鶏卵問題がある:

- audio の `shm::Segment` は **固定レイアウト**（`SLOT_BYTES = 558,208` — `wire.rs` shm mod）でサイズが plugin/config に依らないため、host は `CreateInstance` 時点で必ず同じ大きさの segment を作れる
- video の segment は **`VideoProcessConfig{format,width,height}` から決まる**（4K `RgbaF32` ≈ 126.6 MiB vs 1080p `Yuv420p` ≈ 3.0 MiB — 桁が違う）。ところがこの config は plugin trait 上 **`initialize(&config)` で届く**（`video.rs` L117）— `CreateInstance` 相当の段階では host はまだ config を知らない

つまり「**segment をいつ・どのサイズで作るか**」と「**config をいつ・どの動詞で子に渡すか**」を一貫した形で決める必要がある。加えて、frame の in/out 領域の配置（stride をどう扱うか）も segment sizing と一体の論点である — `VideoFrame` は per-plane stride を持ち `stride ≥ row_bytes` の pad 付きがあり得る（`frame.rs` L232-251）が、slot 占有量は stride に依存する。

## Decision Drivers

* `CreateInstance` で audio segment を attach する既存流れ（`l1/mod.rs` L1746-1875）の invariant — **host が segment を create/own/unlink し、子は `shm_key` で attach するだけ** — を video にも維持する
* 「segment 無しの minted instance」という中間状態を作らない（registry entry が存在するのに data-plane が未確立の窓は失敗分岐を増やす）
* `VideoProcessConfig` は initialize 時に固定され processing 中は不変（`video.rs` L22-73 の契約）— config を途中変更する語彙を v1 で作らない
* decode/attach は `WIRE_MAX_VIDEO_*` で bounded（R-B4）— hostile config で巨大 segment を要求される攻撃面を早期に遮断する
* wire は単一最新バージョン — `CreateVideoInstance` の payload 形は v3 で固定される（#119 方針）

## Considered Options

* **Option A — `CreateVideoInstance{plugin_id, shm_key, config}`**（採用）: `VideoProcessConfig` を create 動詞に載せ、host が config から segment サイズを算出して create → `shm_key` と config を同送。runner は attach + stash config + create() → `VideoInitialize`（payload 無し）で stash を `initialize(&config)` に再生
* **Option B — 遅延 attach（`initialize` 後の第 2 attach 動詞）**: `CreateVideoInstance{plugin_id}` → `VideoInitialize{config}` → host が reply 後に segment を作り `VideoAttachSegment{shm_key}` → 子 attach → `VideoActivate`。segment の正確サイズは得られるが「initialized だが data-plane 未確立」の中間状態と新 attach 動詞が要り、attach 失敗時に「live entry を持つが segment が無い」収束分岐が増える
* **Option C — 固定最大 segment（`WIRE_VIDEO_MAX_FRAME_BYTES × 2 + header` 常時確保）**: config 到着前に作れる最简形。だが 1 instance あたり ≈ 512 MiB 上限を常時確保する = 上限がそのまま common-case 要求になる（`Rgba8` 4K 実需 63 MiB の 8 倍）。「最大宣言」を config でなく予約に使うと bounded の意味が薄れる
* **Option D — create で上限宣言・initialize で実 config**: `CreateVideoInstance{plugin_id, shm_key, max_frame_bytes}` で上限だけ宣言して segment を上限サイズで作り、`VideoInitialize{config}` で実 config を渡し「実 config ≤ 宣言」を子が検証。config を initialize 時に決められる柔軟さはあるが、宣言上限と実値の差分だけ segment が無駄になり、「宣言 < 実値」不整合という新しい失敗クラスが生まれる — config を instantiate 時に決める公開面（Option A の `VideoHostCommand::Instantiate{plugin_id, config}`）と整合しない

## Decision Outcome

Chosen option: "**Option A — `CreateVideoInstance{plugin_id, shm_key, config: VideoProcessConfig}`**、segment は config から一意に算出された canonical packing（in/out 各 1 領域・depth=1）"。

確定する契約:

### 1. create-path — `CreateVideoInstance` に `VideoProcessConfig` を載せる

- `CreateVideoInstance { plugin_id: String, shm_key: String, config }`（tag 12）— audio `CreateInstance` の `{plugin_id, shm_key, in_depth, out_depth}` に相当するが、depth 字段は持たない（depth=1 固定 — 値を wire に載せる概念がない）。config の wire 形は `u8 format + u32 width + u32 height + FrameRate{num,den}`（13 B）
- host 側 `create_video_instance` は `frame_bytes(config)` → `vshm::Segment::create` → `shm_key` + config を同送 — **host が config からサイズを決めて作る**（audio と同じ所有関係: host create/own/unlink・子 attach のみ）
- 早期検証（host 側・wire 非送信）: `frame_bytes ≤ WIRE_VIDEO_MAX_FRAME_BYTES`・`segment_len ≤ WIRE_VIDEO_MAX_SEGMENT_BYTES` → 違反は **`Err(HostError::Prepare(PrepareError::UnsupportedConfig))`**（`check_uniform_bounds` と同語彙）。config dims が `WIRE_VIDEO_MAX_DIM` 超過・`format` が未知・`frame_rate` 零成分も同じ defined 拒否
- runner 側 `CreateVideoInstance` 処理は audio と同一手順（id 解決 → `WIRE_MAX_INSTANCES` → 重複 handle → `vshm` attach → `create()` → insert）。attach は header 宣言値（`vshm` header の `in_bytes`/`out_bytes`/`wire_version`）と config からの再計算を照合 — 不一致は `Err(Ipc)`（host 契約違反）
- `VideoInitialize` は **payload 無し** — runner が `CreateVideoInstance` で stash した config を `initialize(&config)` hook に再生する。これで trait 契約（`initialize` が config を受ける）は保ちつつ wire 上の重複をなくす — host→子への config 伝達は create 1 回で済む
- `VideoHostCommand::Instantiate{plugin_id, config}` も同じ形（caller が instantiate 時に format/dims を決める — OFX/NLE の使用形と一致）。config を途中で変える語彙は v1 では存在しない → format 再交渉は destroy+再 instantiate

### 2. segment layout — canonical packing・depth=1

- `wire::vshm`（`shm` の兄弟 module）に header を定義: 28 B・全 LE u32 — `magic("RVLV")`、`wire_version(=3)`、`in_off(=28)`、`in_bytes`、`out_off(=28+in_bytes)`、`out_bytes`、`reserved(0)`。audio `shm` header と同じ 7 字段形だが `in_depth`/`out_depth`/`slot_bytes` の代わりに byte 容量字段（video は slot 概念がなく領域容量のみ）
- in/out 領域は **`stride = row_bytes` の tightly packed canonical 配置** — plane は `offset(0)=0` から `offset(p+1) = offset(p) + len(p)`（`len = stride × plane_height`）で連続。`VideoFrameDesc` は stride 字段を explicit に運ぶが v1 実値は常に canonical — pad 付き caller frame は host が copy-in で repack、子側 adapter が必要なら自身で再 pad
- depth=1（in 領域 1・out 領域 1）— 同期 render では次 frame は reply 後にしか書けず ring が要らない。`frame_bytes`/`segment_len` の byte math は design `docs/designs/issue-118-video-l1.md` の表どおり
- 非 canonical desc（`stride != row_bytes`・`offset(p) != Σlen`・`offset+len > region_bytes`）は **`Err(HostError::Ipc)`**（host 契約違反 — duplicate handle と同クラス）。dims/format の config 不一致は **`Err(HostError::Prepare(PrepareError::UnsupportedConfig))`**（audio `Activate` bound 拒否の写像）

### Consequences

* Good, because `CreateInstance`→`shm_key` 同送の既存流れをそのまま写せ、「segment 無しの minted instance」中間状態を作らない — `CreateVideoInstance` の失敗（attach 不一致・config 拒否・duplicate handle）は全て `Err` reply で entry 無しのまま閉じる
* Good, because config を instantiate 時に決める形は OFX/NLE の実使用に一致し、`VideoInitialize` の payload 重複（config を 2 箇所で運ぶ）もなくせた — `VideoProcessConfig` は create 1 回で子に届き、runner stash が `initialize(&config)` に再生されるだけ
* Good, because canonical packing で slot 占有量が config から一意に決まり、per-frame stride 変動・desc の「offset/len が region 内か」という複合検証面が単純化される — `offset(p) == Σlen` の自己整合性だけで layout 全体が決まる
* Bad, because `Instantiate` が `config` を取る形は audio の `Instantiate`+`Activate{config}` 分離と位相が違う — caller は instantiate 時に format/dims を決める必要があり、format 変更は destroy+再 instantiate になる（v1 の正直な制約として doc 明記）
* Bad, because `VideoFrameDesc` は stride を運ぶのに v1 で使う値は canonical のみ — 「desc が stride を持つのに repack する」見かけ上の冗長がある（ただし wire 互換を保ったまま将来 pad-preserving 化できる余地として正当化）
* Bad, because 固定最大 segment（Option C）よりも segment create が config 依存になり、`frame_bytes` 計算が host と runner の両方に要る（`vshm` の consts/計算が wire-side 正本なので一箇所に集約される — 重複するのは呼出し点だけ）

### Confirmation

- `docs/designs/issue-118-video-l1.md` 決定サマリ 5・8・Architecture「video shm segment レイアウト」「runner 側」が本 ADR を参照しており、byte math・canonical packing・`CreateVideoInstance` 手順は design の表どおり
- 検証: `vshm` layout 単体テスト（header・`frame_bytes`/`segment_len` byte math・canonical packing）・`CreateVideoInstance` の各拒否分岐（config bound・attach header 不一致・duplicate・`InstanceLimit`）・`VideoInitialize` が stash config を hook に再生する runner 側テスト・E2E scan→create→initialize→…→destroy

## Implementation Plan

* Affected paths: `crates/ruviform-plugin-core/src/wire.rs`（`Command::CreateVideoInstance`・`VideoFrameDesc`/`VideoProcessConfig`/`PixelFormat`/`ColorInfo` codec・`WIRE_MAX_VIDEO_*`・`wire::vshm` mod）・`crates/ruviform-plugin-host/src/l1/vshm.rs`（新規 — `Segment::create(slot, config)`・`unique_key` 共用・`bytes`/`bytes_mut` 2 箇所の scoped unsafe）・`crates/ruviform-plugin-host/src/l1/mod.rs`（`create_video_instance`・`InstKind::Video{config}`）・`crates/ruviform-plugin-host/src/video.rs`（`VideoHostCommand::Instantiate`）・`crates/ruviform-plugin-export/src/vshm.rs`（新規 — attach/validate）・`crates/ruviform-plugin-export/src/lib.rs`（`run_video`・`VideoEntry{config}`）
* Patterns to follow: `create_instance`（`l1/mod.rs` L1746-1875 — mint → register → DriverCmd → request → Live flip/rollback）・`l1/shm.rs` の `Segment`/`unique_key`/`write_header`/`View`・`export/lib.rs` `create_instance`（id 解決 → 上限 → 重複 → attach → create → insert）・audio `Activate` bound 拒否（`check_uniform_bounds` → `Prepare(UnsupportedConfig)`）
* Patterns to avoid: 遅延 attach（initialize 後の第 2 attach）・固定最大 segment・depth 字段・caller stride の pad 保持（v1 は repack — desc の stride 字段は残すが実値は canonical）
* Dependencies or configuration: `shared_memory` crate の既存使用と同じ（`l1/shm.rs`・`export/shm.rs` の scoped unsafe パターン — 新規 `vshm` 両側に同じ deviation を適用）

## Verification

* [ ] `vshm` layout 単体テスト: header 全字段・`frame_bytes`/`segment_len` の byte math・canonical packing 配置（`offset(p)=Σlen`・`stride=row_bytes`）・attach 拒否（magic/version/in_bytes/out_bytes 不一致）
* [ ] `CreateVideoInstance` codec round-trip + 拒否（config dims/`WIRE_MAX_VIDEO_*` 超過・未知 format tag・frame_rate 零・truncated・未知 tag）
* [ ] runner 側 `CreateVideoInstance` 手順（id 解決 → 上限 → 重複 → attach → create → insert）と `VideoInitialize` の stash config 再生
* [ ] host 側 `create_video_instance` の early 拒否（`frame_bytes`/`segment_len` 超過 → `Prepare(UnsupportedConfig)`・wire 非送信）と attach 後の `Live` flip/rollback
* [ ] `mise run lint`・`mise run test` 全 workspace で pass・`doc_status.js audit --json` blocking 0

## Pros and Cons of the Options

### Option A — `CreateVideoInstance{plugin_id, shm_key, config}`

* Good, because `CreateInstance`→`shm_key` 同送の既存 invariant をそのまま写せ、`VideoInitialize` が payload 無しで済み（config は create 1 回で届く）、「segment 無しの minted instance」中間状態がない、`VideoHostCommand::Instantiate{plugin_id, config}` と対称的な公開面になる
* Bad, because `Instantiate` が `config` を取る形は audio の `Instantiate`+`Activate` 分離と違う（caller が instantiate 時に format を決める — format 再交渉は v1 で不可）・`CreateVideoInstance` が audio `CreateInstance` と payload 形が違う並列動詞になる

### Option B — 遅延 attach（initialize 後の第 2 attach）

* Good, because config が `initialize` で届く trait 契約に忠実（stash 中継が要らない）・segment サイズは実 config から正確に決まる
* Bad, because「initialized だが data-plane 未確立」の中間状態と `VideoAttachSegment` 動詞が要り、attach 失敗時に「live entry を持つが segment が無い」収束分岐が増える — 新しい verb・phase・failure 窓を足す対価が create 1 回で済む Option A より大きい

### Option C — 固定最大 segment

* Good, because config 到着前に必ず作れる最简形（segment sizing が config 非依存）
* Bad, because ≈ 512 MiB 上限を 1 instance あたり常時確保する = 上限が common-case 要求になり、`Rgba8` 4K（実需 63 MiB）のような typical case に 8 倍の予約を強いる — bounded bound の意味が薄れる

### Option D — create で上限宣言・initialize で実 config

* Good, because config を initialize 時に決められる柔軟さがあり、segment は宣言上限で早めに作れる
* Bad, because 宣言上限と実値の差分だけ segment が無駄になり、「宣言 < 実値」不整合という新しい失敗クラスが生まれる。`VideoHostCommand::Instantiate{plugin_id, config}` で instantiate 時に決める Option A の公開面と整合しない（config を 2 箇所に分ける設計になる）

## More Information

- design `docs/designs/issue-118-video-l1.md` 決定サマリ 5・8・Architecture「wire v3 プロトコル」「video shm segment レイアウト」「host 内部」「runner 側」・Risks の create-path trade 説明
- spec `docs/specs/issue-118-video-l1.md` R-B2・R-B4・AC1・AC3（frame 輸送・bound・E2E 要求）
- discovery `docs/discovery/issue-118-video-l1.md` 軸 1・軸 5（pool depth・呼出し側挙動・plane 幾何）・Q3・Q5
- `crates/ruviform-plugin-host/src/l1/mod.rs` `create_instance`（L1746-1875）・`crates/ruviform-plugin-host/src/l1/shm.rs`（`Segment`/`unique_key`/`View`）・`crates/ruviform-plugin-export/src/lib.rs` `create_instance`・`crates/ruviform-plugin-export/src/shm.rs`・`crates/ruviform-plugin-core/src/wire.rs` `shm` mod（audio header の正本・`vshm` の兄弟形）
- `crates/ruviform-video-core/src/frame.rs`（`from_shared_planes`・plane dims 導出・`required_len` 不変式）・`crates/ruviform-video-core/src/pixel.rs`（`bytes_per_sample`/`chroma_subsampling`/`plane_count` — `frame_bytes` 計算の正本）
