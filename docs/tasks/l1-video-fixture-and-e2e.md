---
id: "TASK-034NXuzqcpqV0zLNlPSHUR"
type: "task"
status: "done"
title: "L1 video fixture + 公開面 E2E（l1-test-video-worker + tests/l1_video_e2e.rs — Failed parity・frame 内容・cross-kind）"
created: "2026-10-28"
updated: "2026-10-29"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/118"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-host/src/bin/l1-test-video-worker.rs"
      - type: "file"
        path: "crates/ruviform-plugin-host/tests/l1_video_e2e.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-host/Cargo.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-issue-118-video-l1.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-issue-118-video-l1.md"
    - "docs/tasks/l1-video-runner-and-erased-surface.md"
    - "docs/tasks/l1-video-host-surface.md"
  blocks:
    - "docs/tasks/l1-video-docs-sweep.md"
    - "docs/tasks/l1-video-l1-final-verification.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/l1-child-instance-split.md"
    - "docs/tasks/l1-bundle-e2e-and-granularity-verification.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-issue-118-video-l1.md"
    - "docs/designs/issue-118-video-l1.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-118-video-failed-parity-tests.md"
    - "docs/test-specs/issue-118-video-e2e-tests.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/76"
    - "https://github.com/scarecrow173/ruviform/issues/114"
    - "https://github.com/scarecrow173/ruviform/issues/117"
  defers: []
  deferred-by: []
---

# L1 video fixture + 公開面 E2E（l1-test-video-worker + tests/l1_video_e2e.rs — Failed parity・frame 内容・cross-kind）

## Work

host 公開面（`PluginHost::video()`）のみを打つ E2E — `l1-test-video-worker`（audio `l1-test-worker` と同型の **hand-rolled wire child** — `run_video` は通さない）を `VideoOutOfProcessSpec.program` に登録し、同一 package の `env!("CARGO_BIN_EXE_l1-test-video-worker")` で位置解決する #114 `l1-test-worker` パターン。実 runner lane（export crate の `ruviform-plugin-l1-video-runner`）は同一 wire 規約を打つ 2 正面の一方。fixture 制御は **`spec.args` 経由の argv flag**（audio worker と同パターン — env contract は使わない。`l1-test-worker` 実装にも `RUVIFORM_TEST_*` 等の env 経路は存在しない）。

なぜ `run_video` ではなく hand-rolled か（構造制約 — audio と同じ理由で wire 層に直接居座る子が要る）:

- `run_video` は `run()` と同じ serve skeleton を共有し、任意 argv を usage reject（`crates/ruviform-plugin-export/src/lib.rs:141-144`）する — fault flag を子へ渡す経路がない
- 非 canonical `Rendered.output` desc は runner 由来の出力 desc 経路では**構造的に到達不能**（runner が `Rendered` を自前で canonical に組み立てるため plugin hook から注入できない）— AC4(b) の帰属可能 per-instance `Failed` を pin するには wire reply を直接書く子が要る
- crash/hang/stop-reading 等の wire-level fault は plugin-hook 層より下（frame loop 層）で発火させる必要があり、`run_video` の hook 面には flag 受け口がない
- `ruviform-plugin-host` → `ruviform-plugin-export` の dep edge は design の pinned 4-edge 表にない — host crate の bin から export 層を呼べない（audio `l1-test-worker` が export を通らず wire を直接話すのと同じ境界）

1. `crates/ruviform-plugin-host/Cargo.toml` — `[[bin]] name = "l1-test-video-worker"` を `l1-test-worker` の隣に追加（`required-features` なし — 既定 lane で build される）
2. `crates/ruviform-plugin-host/src/bin/l1-test-video-worker.rs`（新規 — テンプレは `src/bin/l1-test-worker.rs`）: `ruviform_plugin_core::wire` codec で frame を直接 read/write する wire-faithful 子 — 起動で `Hello` → `Scan`→`Scanned`（video-kind registry 報告）→ 単一スレッドの `Command`→`Reply` loop → `Shutdown`/`EOF` で exit 0。video registry は小さな `HashMap<u32, …>`（envelope `instance` キー — audio worker の registry パターン同型）で、`CreateVideoInstance{shm_key,format,width,height,frame_rate}` の shm_key から `shared_memory` attach し `wire::vshm` header を検証して in/out 領域を指す。inbound `BlockIn` は wire 規約どおり常に inert drop（返信しない）。video 型（`VideoProcessConfig`/`VideoFrame` desc 分解・pixel 変換 oracle）は `ruviform-video-core` 経由（plugin-host dep edge — T3 で追加済み）。**`ruviform-plugin-export` への依存は持たない**
3. 同 file — fault matrix（`spec.args` → argv flag — audio `l1-test-worker` の argv family に揃える）:
   - wire-level（audio worker 同 family・同一語彙）: `--crash-after-msgs=N`・`--crash-exit-code=C`・`--hang-after-msgs=N`・`--stop-reading-after-msgs=N`・`--leave-heir-after-msgs=N`・`--sleep-ms=M`・`--hello-version=V`
   - video hook 失敗（`Err(PluginFailed)` reply — hook の commit 規則どおり。`stop-processing`/`deactivate` は commit-then-advisory — audio `--fail-stop-processing` 同型）: `--fail-initialize`・`--fail-start-processing`・`--fail-stop-processing`・`--fail-deactivate`・`--fail-render`
   - reply 捏造: `--noncanonical-output` — `Rendered{output}` に非 canonical stride/offset（`stride ≠ row_bytes`・`offset(p) ≠ Σlen`）または config 不一致 dims/format を載せる — AC4(b) 帰属可能 per-instance `Failed` の fixture 経路
   - param 応答: `--fail-param-value`（`VideoGetParamValue`/`VideoSetParamValue` → `Err(PluginFailed)` — obligation (b) の host 観測形）・`--overlimit-params`（`VideoGetParams`/`VideoGetParamInfo` → `Err(PluginFailed)` — obligation (c) の adapter-normalized 答えを fixture 側で emulate）
   - flag 不要（小 registry の default 応答でカバー）: `parameter(id)` で `None` を返す `VideoGetParamInfo` → `ParamInfo(None)`・未知 id の `Err(InvalidValue)` pass-through・inert `BlockIn`（常に drop）
   - adapter 正規化自体（non-finite `Double` → `Err(PluginFailed)`・over-limit → `Err(PluginFailed)` の `erase_video` 境界 pin）は real-runner lane（item 9 — T2 bin の `RUVIFORM_PANIC_*`/over-limit fixture）側で刺す — hand-rolled worker は wire 上に adapter 境界を持たない
4. `crates/ruviform-plugin-host/tests/l1_video_e2e.rs`（新規 — `l1_proxy_e2e.rs` と同じ public-API-only 原則 — `PluginHost`/`VideoPluginHost`/`VideoOutOfProcessSpec`/`VideoHostCommand`/`HostError`/`VideoProcessConfig`/`VideoFrame`/`MediaTime`/`PlainParamValue`/`PluginHandle`/`InstanceFailed` のみを使い、`l1::` internals/`wire::`/`InstShared` は一切参照しない）: `env!("CARGO_BIN_EXE_l1-test-video-worker")` を `program` に登録した `VideoPluginHost` で打つ happy-path + fault-path suite
5. 同 file — happy path: `video.register_out_of_process` → `Scan`（video-kind のみ列挙）→ `Instantiate{id, config}` → lifecycle（`Initialize`→`Activate`→`StartProcessing`→`Render`→`StopProcessing`→`Deactivate`→`Destroy`）→ `lifecycle_state` 遷移の pin・`Render` の出力 frame 内容（反転/fill の deterministic pixel oracle — in 領域の byte が反転されて out 領域に出る）・pts/duration/color pass-through（input `VideoFrame` の metadata が `Rendered` 復元 frame にそのまま出る）・params（`GetParams` id 列・`GetParamInfo` の `Option`・`GetParamValue`/`SetParamValue` round-trip・`VideoGetParamInfo` の `ParamInfo(None)` — plugin が id で `None` を返す契約）
6. 同 file — cross-kind surface: 同一 `PluginHost` に audio route（`register_out_of_process` で `l1-test-worker` — audio worker との並存）と video route を登録 → audio `Scan` が video id を返さない・video `Scan` が audio id を返さない（kind filter）・audio `Instantiate{video_id}`/`Instantiate{video handle}` → `Err(UnknownPluginId)`・`register_out_of_process` で同 id を両 kind に登録 → `Err(DuplicatePluginId)` 双方向・audio slot handle に video verb / video slot handle に audio verb → `Err(Unsupported)`（wire 非送信 — `execute` dispatch の slot 分岐）・video handle に direct `PluginHost::process`/`latency_frames`/`plugin_mut`/`prepare_endpoint` → `Err(Unsupported)`/`None`/`Err(EndpointUnavailable)`・`kill_child`/`try_recv_notification` は両 kind で動く
7. 同 file — **Failed parity**: `Render` 中の子死亡 → waiter `Err(Ipc)` + `InstanceFailed{handle}`（`try_recv_notification` で exit status 検査 — audio #116/`l1_child_instance` の同パターン）・create 中の子死亡 → `Err(Ipc)` + handle 未発行（mint しない — `InstanceFailed` 通知も来ない）・`Rendered.output` semantic violation fixture → `Err(Ipc)` + per-instance `Failed`（sibling は生きる — Dedicated なら 1 子=1 instance で観測可能）・`Err(PluginFailed)` の `Render`/`initialize` → mirror 不変（`Err` reply で子は生きる — `lifecycle_state` が `Failed` でないことを pin）・inert `BlockIn`（runner lane で pin — T2 `l1_video_runner_e2e` の MiniHost が raw frame を書き drop/no-reply/子生存を検査。`PluginHost` 公開面からは video 子へ `BlockIn` を発行する経路がないため本 file では打たない）・post-`Shutdown` frame の non-event（child exit 後の `InstanceFailed` が来ない）・non-finite `Double` の `SetParamValue` → `Err(InvalidValue)`（wire 非送信 — obligation (a)）・over-limit `parameters()`/`parameter()` fixture → `Err(PluginFailed)` で子生存（obligation (c)）
8. 同 file — bound 拒否（wire 非送信・子未 spawn — `VideoOutOfProcessSpec` 登録後 `Instantiate` が config bound 超過で `Err(Prepare(UnsupportedConfig))` を返す。frame_bytes/`MAX_DIM`/未知 format・非零 frame_rate — `Instantiate` は early-reject で子を起こさないので `kill_child`/`try_recv_notification` に何も出ないことを pin）
9. 同 file — **real runner 2 正面**: 同一 wire 規約を export crate の `ruviform-plugin-l1-video-runner` に対して happy-path + Failed-parity の代表ケースを打つ — `VideoOutOfProcessSpec.program` を差し替えるだけで成立。bin の位置解決は `env!("CARGO_BIN_EXE_…")` では**不可**（`CARGO_BIN_EXE_*` は同一 package の bin にしか効かず、export crate の bin は host crate の test から参照できない）— `tests/l1_proxy_e2e.rs:1988-2014` の `runner_program()` パターンをそのまま pin する: `RUVIFORM_L1_VIDEO_RUNNER` env override を先に読み → なければ `std::env::current_exe()` の sibling dir → `deps/` parent の順に `ruviform-plugin-l1-video-runner` を探し、見つからなければ searched candidates 付きで panic（audio lane の `RUVIFORM_L1_RUNNER` 慣習に揃える）
10. `render_timeout` — `Render` のみ `render_timeout`（既定 60s）を、他動詞は `command_timeout` を使う。hang fixture で `Render` の timeout が `render_timeout` で発火することを pin（timeout 値を `VideoOutOfProcessSpec` で短く設定可能 — fixture が `Render` で sleep/hang し `Err(Ipc)` + `converge_instance` になる）。`command_timeout`/`render_timeout` の分離を `VideoOutOfProcessSpec` で明示

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-118-video-l1.md](../plans/implement-issue-118-video-l1.md)（本 task は T4 — T2/T3 両完了が前提）
- [../designs/issue-118-video-l1.md](../designs/issue-118-video-l1.md) L297-316（`Slot` match site の host 面検証）・L318-326（`create_video_instance`・`Render` 写像）・L337-352（`Failed` 規則・`Err` reply mirror 不変・`InstanceFailed` exactly-once・inert `BlockIn`・post-`Shutdown`）・L354-372（timeout 分離・bound 拒否・`InvalidValue`）
- [../specs/issue-118-video-l1.md](../specs/issue-118-video-l1.md) AC3・AC4・AC6・AC7・AC9・R-F1〜F6・R-V1〜V6（公開面から観測可能な全範囲）
- [issue-118-video-failed-parity-tests.md](../test-specs/issue-118-video-failed-parity-tests.md)（Failed/通知/inert 規則の oracle）
- [issue-118-video-e2e-tests.md](../test-specs/issue-118-video-e2e-tests.md)（公開面・frame 内容・cross-kind の oracle）

## Dependencies

- 先行タスク: `l1-video-runner-and-erased-surface`（`run_video`・reference bin `ruviform-plugin-l1-video-runner` — real-runner lane の program・export crate `vshm`/`VideoRunner`）+ `l1-video-host-surface`（`video()` façade・`VideoOutOfProcessSpec`・`VideoL1Instance`・`vshm::Segment`）の両方
- `l1-test-video-worker` の crate 依存は `ruviform-plugin-core`（`wire` codec — plugin-host の既存 dep）+ `shared_memory`（既存 dep — `l1::shm` と同じ `as_slice` scoped-unsafe 許可パターン）+ `ruviform-video-core`（video 型 — T3 で追加済み）のみ。**`ruviform-plugin-export` への dep edge は作らない**（design の pinned 4-edge 表にない — fixture は wire 層に直接居座る）。real-runner lane が spawn する `ruviform-plugin-l1-video-runner` は cargo dep ではなく実行ファイルとして参照する（`runner_program()` 解決 — item 9）
- audio `l1-test-worker` は cross-kind lane で併用（`CARGO_BIN_EXE_l1-test-worker` は既存 bin）

## Affected Files

- `crates/ruviform-plugin-host/src/bin/l1-test-video-worker.rs`（新規）— hand-rolled wire child（`l1-test-worker` template）+ `spec.args` argv fault matrix
- `crates/ruviform-plugin-host/tests/l1_video_e2e.rs`（新規）— 公開面 only E2E（happy + fault + cross-kind + bound + real runner）
- `crates/ruviform-plugin-host/Cargo.toml` — `[[bin]] l1-test-video-worker` 追加

## Done When

- [x] `l1-test-video-worker` が hand-rolled wire loop（`Hello`→`Scan`/`Scanned`→`Command`→`Reply`→`Shutdown`）で起動し、`spec.args` 経由の argv flag で `--fail-render`/`--fail-initialize`/`--fail-deactivate`/`--fail-start-processing`/`--fail-stop-processing`/`--fail-param-value`/`--overlimit-params`/`--noncanonical-output`/`--crash-after-msgs`/`--hang-after-msgs` 等を制御できる
- [x] `tests/l1_video_e2e.rs` が `PluginHost::video()` のみで happy-path（lifecycle・render 内容・pts/duration/color pass-through・params round-trip・`ParamInfo(None)`）を pass する
- [x] cross-kind suite（kind filter Scan・`DuplicatePluginId`・`Unsupported`/`EndpointUnavailable`・direct `PluginHost` メソッドの video 答え）が pass する
- [x] Failed parity（子死亡/`InstanceFailed`/create 中死亡/`Err` reply mirror 不変/semantic violation → `Err(Ipc)`/inert `BlockIn`/post-`Shutdown`/obligation (a)(c)）が pass する
- [x] bound 拒否（`Instantiate` → `Err(Prepare(UnsupportedConfig))`・子未 spawn）と `render_timeout` 分離が pass する
- [x] 実 runner lane（`ruviform-plugin-l1-video-runner`）が同一 wire 規約で happy + 代表 fault を pass する

## Verification

- [x] `cargo test -p ruviform-plugin-host --test l1_video_e2e` — 公開面 E2E 全 pass
- [x] `cargo test -p ruviform-plugin-export --test l1_video_runner_e2e` — export runner lane 全 pass
- [x] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` が全 pass（audio suite 無退行を含む）
