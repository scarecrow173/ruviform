---
id: "TSPEC-034NXpUzrEtLN4lNJSzI77"
type: "test-spec"
status: "approved"
title: "L1 video E2E tests（Issue #118 — video() façade・frame 輸送・param・cross-kind・Dedicated pin）"
created: "2026-10-28"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/118"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_video_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/tests/l1_video_runner_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/bin/l1-test-video-worker.rs"
      - type: "file"
        path: "crates/ruviform-plugin-export/src/bin/ruviform-plugin-l1-video-runner.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/video.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/src/composite.rs"
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
  related:
    - "docs/test-specs/issue-118-wire-v3-codec-tests.md"
    - "docs/test-specs/issue-118-video-failed-parity-tests.md"
    - "docs/test-specs/plugin-l1-proxy-contract-tests.md"
    - "docs/test-specs/plugin-l1-runner-e2e-tests.md"
    - "docs/test-specs/issue-116-supervisor-e2e-tests.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-118-video-l1.md"
    - "docs/adr/issue-118-video-l1-host-surface.md"
    - "docs/adr/issue-118-video-l1-create-path-segment.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-118-video-l1.md"
    - "docs/designs/issue-118-video-l1.md"
    - "docs/adr/issue-118-video-l1-host-surface.md"
    - "docs/adr/issue-118-video-l1-create-path-segment.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# L1 video E2E tests（Issue #118 — video() façade・frame 輸送・param・cross-kind・Dedicated pin）

## Purpose

video L1 の公開面 E2E oracle — `PluginHost::video()` 借用 façade 経由で video route 登録→spawn→`Instantiate{plugin_id, config}`→lifecycle→`Render`→param→`Destroy` が外から正しく観測されること、frame payload が `vshm` segment 経由で往復すること、route kind 分離と Dedicated 構造 pin が成立することを pin する。fixture は `l1-test-video-worker`（hand-rolled fault-injection fixture）と real `ruviform-plugin-l1-video-runner`（`run_video` 経由の実 runner）の両 lane — `l1_proxy_e2e` が worker と real runner の両 lane を持つ先例どおり。codec 形式は `issue-118-wire-v3-codec-tests`、異常系の収束意味論は `issue-118-video-failed-parity-tests` が所有する。

## Feature

`VideoPluginHost<'a>`（`PluginHost::video(&mut self)` が返す借用 view）が `VideoHostCommand`/`VideoHostReply` 語彙で video route の register/execute を提供し、`VideoOutOfProcessSpec`（Dedicated 固定・`isolation` 字段なし）で video 子を spawn する。`tests/l1_video_e2e.rs` は **公開 API のみ**（`#[path]` harness なし — `l1_supervisor_e2e` 先例）で `pub(crate)` 機構に触れず、`PluginHost` の単一 slot table が audio/video 両 kind の handle を一意に mint することを compile レベルでも証明する。

## Rules

- **正常系ライフサイクル**: `video().register_out_of_process(spec)` → `VideoHostCommand::Scan`（video route のみ列挙）→ `Instantiate{plugin_id, config}` → `Initialize` → `Activate` → `StartProcessing` → `Render` → `StopProcessing` → `Deactivate` → `Destroy` が L0 契約どおり順に成功する。`Instantiate` は `Creating` phase の `CreateVideoInstance` request を受理する必要がある（`Creating` ゲート拡張の E2E 観測）
- **frame 輸送**: `Render{handle, input, at}` で input `VideoFrame`（`Rgba8` pin — mock plugin が受理する format）が `vshm` in 領域経由で子に届き、`Rendered` reply 後に出力 `VideoFrame` が有効。出力 pixel が deterministic transform（fixture の反転/fill）と一致・`pts`/`duration`/`ColorInfo` が input→output に pass-through（runner が input metadata で output を seed する契約）
- **lifecycle gate**: 違法遷移は `Err(HostError::Lifecycle(_))`（例: `StartProcessing` を `Initialized` 状態で・`Render` を `Processing` 以外で — facade が mirror `Processing` を前置検査）・`Failed` instance への `Destroy` 以外の全動詞は `Err(HostError::Failed)`
- **`Instantiate` bound 拒否**: `frame_bytes(config) > WIRE_VIDEO_MAX_FRAME_BYTES`・`segment_len > WIRE_VIDEO_MAX_SEGMENT_BYTES`・dims > `WIRE_VIDEO_MAX_DIM`・frame_rate 零成分 → `Err(HostError::Prepare(PrepareError::UnsupportedConfig))`（wire 非送信の early defined 拒否 — `check_uniform_bounds` 同語彙）
- **`Render` input 照合**: `input.format()/width()/height()` が cached config と不一致 → `Err(HostError::InvalidValue)`（wire 非送信 — caller 供給値の domain 違反）
- **param チャネル**: `GetParams` → `ParamIds`・`GetParamInfo{id}` → `ParamInfo(Some)`/`ParamInfo(None)`（未知 id — error でなく `None`）・`GetParamValue{id}` → `ParamValue`・`SetParamValue{id, value}` → `Ok`。未知 `ParamId` の `GetParamValue`/`SetParamValue` → `Err(HostError::InvalidValue)` 透過（runner 側 `id ∈ parameters()` 先行検査）
- **encode obligation (a)**: `SetParamValue{id, value: Double(NaN|±Inf)}` → facade が `Err(HostError::InvalidValue)` を **wire 非送信で**返す（非有限 `Double` の encode は禁止 — 送信されたら子側 decode が `Malformed` で畳み子死する経路を塞ぐ事前検証）
- **route kind 分離**: audio `HostCommand::Scan` は video route を列挙しない・video `VideoHostCommand::Scan` は audio route を列挙しない（`descriptors` kind-tag filter）。audio `Instantiate` が video route の `plugin_id` → `Err(UnknownPluginId)`・video `Instantiate` が audio route → `Err(UnknownPluginId)`。同一 `plugin_id` の双方向登録 → `Err(DuplicatePluginId)`（1 id = 1 route = 1 kind）
- **cross-kind 動詞**: audio `HostCommand`（lifecycle/state/GUI/process 系）が video handle を指す → `Err(HostError::Unsupported)`・video `VideoHostCommand` が audio handle を指す → `Err(HostError::Unsupported)`（wire 非送信 — slot 分岐で defined 拒否）
- **direct `PluginHost` メソッドの video 答え**: `process` → `Err(Unsupported)`・`latency_frames` → `None`・`plugin_mut` → `None`・`prepare_endpoint`/`attach_endpoint` → `Err(EndpointUnavailable)`・`lifecycle_state` → `Some(mirror)`・`kill_child` → 動く（`Failed` + 通知）・`destroy` → 動く
- **Dedicated pin**: `VideoOutOfProcessSpec` に `isolation`/`lookahead_blocks` 字段が**存在しない**（compile-time pin — `isolation` を書こうとするコードはコンパイルできない。「非対応の形で pin」による AC5/R-V5 の充足）。video route は `Instantiate` 毎に専用子を spawn し pool を持たない
- **`Err` reply の正常系意味**: `Err(PluginFailed)`（`--fail-render`/`--fail-initialize`）で caller は `Err` を受けるが mirror は遷移しない・子は生きる（detailed parity は failed-parity spec — 本 spec は「E2E で `Err` が defined に返る」面のみ）
- **`descriptor(handle)`**（**v1 未提供** — plan Open Question は pinned 回答で確定）: handle→descriptor lookup API は video view に生えない — descriptor 情報は `Scan`/`register` 時点で呼び出し側が保持する形。`video()` façade の doc に「v1 未提供」を明記する（T3）

## Examples

- happy path: `VideoOutOfProcessSpec{descriptor, program: worker, args: [], command_timeout: 10s, render_timeout: 60s}` → `video().register_out_of_process(spec)` → `Scan` が worker descriptor を 1 件返す（audio route が混在しても video のみ）→ `Instantiate{id, config: 64×64 Rgba8 30fps}` → `Rendered` frame が fixture transform と byte 一致・pts/duration/color が input と一致 → `Destroy` `Ok` で `lifecycle_state` が `None`/Destroyed 化
- real-runner lane: 同じ command series を `program = ruviform-plugin-l1-video-runner`（`run_video` + mock invert/panic factory）で駆動 — `erase_video`/`run_video`/`vshm` attach の実コード経路が公開 API の下で通る
- bound 拒否: `Instantiate{id, config: 8193×64 Rgba8}`（dims 超過）・`config: 8192×8192 RgbaF32`（frame_bytes = 8192×8192×4ch×4B = 1,073,741,824 B = 1 GiB > 256 MiB cap）→ `Err(Prepare(UnsupportedConfig))` — wire に `CreateVideoInstance` が出ない（子プロセス非 spawn で観測）
- cross-kind 行列: `PluginHost` に audio route（in-process factory or out-of-process worker）+ video route を同居登録 → audio `Scan` は audio のみ・video `Scan` は video のみ・audio `Instantiate` → `UnknownPluginId`・`PluginHost::process(video_handle)` → `Err(Unsupported)`・`video().execute(VideoHostCommand::Activate{audio_handle})` → `Err(Unsupported)`・`prepare_endpoint(video_handle)` → `Err(EndpointUnavailable)`
- param 系列: `GetParams` → `[0,1]`・`GetParamInfo{0}` → `ParamInfo(Some{choices 持ち})`・`GetParamInfo{999}` → `ParamInfo(None)`・`GetParamValue{0}` → `ParamValue`・`SetParamValue{999, _}` → `Err(InvalidValue)`・`SetParamValue{0, Double(NaN)}` → `Err(InvalidValue)`（wire 非送信 — 子が生存し後続動詞が通ることで非送信を立証）
- lifecycle 拒否: `Activate`（`Initialize` 前）→ `Err(Lifecycle)`・`Render`（`StartProcessing` 前）→ `Err(Lifecycle)`・kill 後の `Activate` → `Err(Failed)`・`Destroy` → `Ok`
- notification: kill/違反後に `try_recv_notification` が `InstanceFailed{handle, exit}` を返す — video slot の通知が drain される（skip されない — `try_recv_notification` video arm の観測）

## Guarantees

- `video()` façade 経由で video plugin が L0 と同一の lifecycle 意味論で L1 子プロセス内に spawn・render・param・destroy できる — scan→create→destroy の全 command series が公開 API で完結する
- frame payload は `vshm` segment で往復し、出力は `Rendered` reply 到達時にのみ有効（commit-on-success — `Err` reply 時に出力を読まない）・input metadata（pts/duration/color）は output に保存される
- route kind は host 側のみの属性として分離される — `Scan`/`Instantiate`/cross-kind 動詞/登録衝突が全て defined 語彙で答える（`Hello.capabilities`/wire kind 記述は使わない決定の裏付け — kind 混同は `spec.descriptor` field-match で検出される）
- Dedicated 固定が構造的に pin される — `VideoOutOfProcessSpec` の型に `isolation`/`lookahead_blocks` が存在せず Bundled video は表現不能（runtime 拒否経路自体が存在しない）
- caller 供給値の domain 違反は wire 非送信の local `Err`（`InvalidValue`/`Prepare(UnsupportedConfig)`）— encode 不可能な値・上限超過が子へ届かない

## Non-goals

- wire codec 形式・tag・presence strictness・`vshm` byte math — `issue-118-wire-v3-codec-tests` が対象
- `Failed` 収束の全分岐（子スコープ/帰属可能/inert/timeout/mid-verb）— `issue-118-video-failed-parity-tests` が正本（本 spec は正常系 + 「通知が drain される」観測のみ）
- runner registry の内部 dispatch 契約（`CreateVideoInstance` 検証 chain・stash replay・`id ∈ parameters()` 先行検査・`BlockIn` drop）— `issue-118-video-failed-parity-tests` + plugin-export 側 `l1_video_runner_e2e` MiniHost suite が所有。本 spec は `PluginHost::video()` 公開面のみを対象とする
- 4K/8K 実寸 segment の性能・容量検証 — E2E は小 frame で意味論を pin。byte math は codec spec、上限は host early-check で担保
- OFX 実バイナリ・real-adapter lane — v1 に存在しない（mock video fixture のみ — spec Out of Scope）
- RT 契約・endpoint・supervisor — video には適用されない

## Risk

- `Creating` ゲート未拡張（`l1/mod.rs` L1914-1915 が `CreateVideoInstance` を受理しない）→ create が恒に `Err(Ipc)` で video instantiate が一切動かない — happy-path E2E が唯一の観測点
- `descriptors` kind-tag 欠落 → video descriptor が audio `Scan` に漏れ route kind 分離が破れる
- cross-kind `Unsupported` arm 欠落 → audio 動詞が video slot の `L1Instance` ではなく `VideoL1Instance` を指して dispatch が panic または誤経路
- `try_recv_notification` video arm 欠落 → `InstanceFailed` が永遠に失われる（E2E の notification 観測が唯一の検出点）
- repack/unpack の row copy bug（stride≠dims・plane offset）→ pixel が化けるが codec round-trip は green — frame 内容一致 E2E のみが検出
- metadata seed 欠落 → `pts`/`duration`/`color` が output から落ち、NLE の timestamp 整合が壊れる
- façade pre-validate 欠落（obligation (a)）→ 非有限 `Double` が encode され decode で `Malformed` → 子死（host 契約違反を caller の domain 違反に見せない）
