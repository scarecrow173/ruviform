---
id: "TASK-034NXuzqcpqV0zLNlPSHUR"
type: "task"
status: "todo"
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

host 公開面（`PluginHost::video()`）のみを打つ E2E — `l1-test-video-worker`（export 層 `run_video` を通した wire-faithful 子）を `VideoOutOfProcessSpec.program` に登録し、`env!("CARGO_BIN_EXE_…")` で位置解決する #114 `l1-test-worker` パターン。実 runner lane（export crate の `ruviform-plugin-l1-video-runner`）は同一 wire 規約を打つ 2 正面の一方。fixture 側の hook 制御は `RUVIFORM_TEST_*` env 経由（audio worker と同パターン — `args` は reserved）。

1. `crates/ruviform-plugin-host/Cargo.toml` — `[[bin]] name = "l1-test-video-worker"` を `l1-test-worker` の隣に追加（`required-features` なし — 既定 lane で build される）
2. `crates/ruviform-plugin-host/src/bin/l1-test-video-worker.rs`（新規）: `fn main() -> ExitCode { ruviform_plugin_export::run_video(vec![...]) }` — mock `VideoPluginFactory` 列 + `RUVIFORM_TEST_*` env 制御 hook fault matrix。host crate の `src/bin/` に置き、video 型は `ruviform-video-core` 経由で到達（plugin-host の dep edge — T3 で追加済み）
3. 同 file — fault matrix（`RUVIFORM_TEST_*` env → fixture が対応する hook 分岐 — audio `l1-test-worker` の env contract に倣う）: `render` が `Err(PluginFailed)` / panic / 非 canonical `Rendered.output` / config 不一致 dims を emit するケース（後者 2 つは `Rendered` emit 側の fixture が意図的に semantic violation を作る — runner dispatch の検証とは別経路で host 検証を刺す）・`initialize`/`deactivate` が `Err` を返すケース（commit-then-advisory・init 失敗 Created 維持の pin）・`parameter_value()` が非有限 `Double` を報告・`parameters()` が `WIRE_MAX_ELEMENTS` 超 `ids`/over-limit `PlainParamInfo` を報告（obligation (b)(c) — `Err(PluginFailed)` で子生存）・crash/hang（子の早期 exit・応答なし — timeout/`converge_child`/`InstanceFailed` の発火源）・`VideoGetParamInfo` が `parameter()` の `Option` 契約（id で `None` を返す plugin）を通るケース
4. `crates/ruviform-plugin-host/tests/l1_video_e2e.rs`（新規 — `l1_proxy_e2e.rs` と同じ public-API-only 原則 — `PluginHost`/`VideoPluginHost`/`VideoOutOfProcessSpec`/`VideoHostCommand`/`HostError`/`VideoProcessConfig`/`VideoFrame`/`MediaTime`/`PlainParamValue`/`PluginHandle`/`InstanceFailed` のみを使い、`l1::` internals/`wire::`/`InstShared` は一切参照しない）: `env!("CARGO_BIN_EXE_l1-test-video-worker")` を `program` に登録した `VideoPluginHost` で打つ happy-path + fault-path suite
5. 同 file — happy path: `video.register_out_of_process` → `Scan`（video-kind のみ列挙）→ `Instantiate{id, config}` → lifecycle（`Initialize`→`Activate`→`StartProcessing`→`Render`→`StopProcessing`→`Deactivate`→`Destroy`）→ `lifecycle_state` 遷移の pin・`Render` の出力 frame 内容（反転/fill の deterministic pixel oracle — in 領域の byte が反転されて out 領域に出る）・pts/duration/color pass-through（input `VideoFrame` の metadata が `Rendered` 復元 frame にそのまま出る）・params（`GetParams` id 列・`GetParamInfo` の `Option`・`GetParamValue`/`SetParamValue` round-trip・`VideoGetParamInfo` の `ParamInfo(None)` — plugin が id で `None` を返す契約）
6. 同 file — cross-kind surface: 同一 `PluginHost` に audio route（`register_out_of_process` で `l1-test-worker` — audio worker との並存）と video route を登録 → audio `Scan` が video id を返さない・video `Scan` が audio id を返さない（kind filter）・audio `Instantiate{video_id}`/`Instantiate{video handle}` → `Err(UnknownPluginId)`・`register_out_of_process` で同 id を両 kind に登録 → `Err(DuplicatePluginId)` 双方向・audio slot handle に video verb / video slot handle に audio verb → `Err(Unsupported)`（wire 非送信 — `execute` dispatch の slot 分岐）・video handle に direct `PluginHost::process`/`latency_frames`/`plugin_mut`/`prepare_endpoint` → `Err(Unsupported)`/`None`/`Err(EndpointUnavailable)`・`kill_child`/`try_recv_notification` は両 kind で動く
7. 同 file — **Failed parity**: `Render` 中の子死亡 → waiter `Err(Ipc)` + `InstanceFailed{handle}`（`try_recv_notification` で exit status 検査 — audio #116/`l1_child_instance` の同パターン）・create 中の子死亡 → `Err(Ipc)` + handle 未発行（mint しない — `InstanceFailed` 通知も来ない）・`Rendered.output` semantic violation fixture → `Err(Ipc)` + per-instance `Failed`（sibling は生きる — Dedicated なら 1 子=1 instance で観測可能）・`Err(PluginFailed)` の `Render`/`initialize` → mirror 不変（`Err` reply で子は生きる — `lifecycle_state` が `Failed` でないことを pin）・inert `BlockIn`（wire に書いても返信・子死なし — MiniHost 側で打つ runner lane、または fixture 経由）・post-`Shutdown` frame の non-event（child exit 後の `InstanceFailed` が来ない）・non-finite `Double` の `SetParamValue` → `Err(InvalidValue)`（wire 非送信 — obligation (a)）・over-limit `parameters()`/`parameter()` fixture → `Err(PluginFailed)` で子生存（obligation (c)）
8. 同 file — bound 拒否（wire 非送信・子未 spawn — `VideoOutOfProcessSpec` 登録後 `Instantiate` が config bound 超過で `Err(Prepare(UnsupportedConfig))` を返す。frame_bytes/`MAX_DIM`/未知 format・非零 frame_rate — `Instantiate` は early-reject で子を起こさないので `kill_child`/`try_recv_notification` に何も出ないことを pin）
9. 同 file — **real runner 2 正面**: 同一 wire 規約を export crate の `ruviform-plugin-l1-video-runner`（`env!("CARGO_BIN_EXE_ruviform-plugin-l1-video-runner")` — export crate の `dev-dependencies` に host crate が必要か、またはこの test target を `required-features` なしで export crate に置くかは実装判断。plan は `plugin-host/tests/l1_video_e2e.rs` に統一）に対して happy-path + Failed-parity の代表ケースを打つ — `VideoOutOfProcessSpec.program` を差し替えるだけで同一 wire 規約を打つ 2 正面として成立
10. `render_timeout` — `Render` のみ `render_timeout`（既定 60s）を、他動詞は `command_timeout` を使う。hang fixture で `Render` の timeout が `render_timeout` で発火することを pin（timeout 値を `VideoOutOfProcessSpec` で短く設定可能 — fixture が `Render` で sleep/hang し `Err(Ipc)` + `converge_instance` になる）。`command_timeout`/`render_timeout` の分離を `VideoOutOfProcessSpec` で明示

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-issue-118-video-l1.md](../plans/implement-issue-118-video-l1.md)（本 task は T4 — T2/T3 両完了が前提）
- [../designs/issue-118-video-l1.md](../designs/issue-118-video-l1.md) L297-316（`Slot` match site の host 面検証）・L318-326（`create_video_instance`・`Render` 写像）・L337-352（`Failed` 規則・`Err` reply mirror 不変・`InstanceFailed` exactly-once・inert `BlockIn`・post-`Shutdown`）・L354-372（timeout 分離・bound 拒否・`InvalidValue`）
- [../specs/issue-118-video-l1.md](../specs/issue-118-video-l1.md) AC3・AC4・AC6・AC7・AC9・R-F1〜F7・R-V1〜V7（公開面から観測可能な全範囲）
- [issue-118-video-failed-parity-tests.md](../test-specs/issue-118-video-failed-parity-tests.md)（Failed/通知/inert 規則の oracle）
- [issue-118-video-e2e-tests.md](../test-specs/issue-118-video-e2e-tests.md)（公開面・frame 内容・cross-kind の oracle）

## Dependencies

- 先行タスク: `l1-video-runner-and-erased-surface`（`run_video`・reference bin・export crate `vshm`/`VideoRunner`）+ `l1-video-host-surface`（`video()` façade・`VideoOutOfProcessSpec`・`VideoL1Instance`・`vshm::Segment`）の両方
- `l1-test-video-worker` は `ruviform-plugin-export` の `run_video` を呼ぶため export crate への依存は T2 で確定済み。video 型は `ruviform-video-core`（plugin-host dep edge — T3 で追加済み）経由
- audio `l1-test-worker` は cross-kind lane で併用（`CARGO_BIN_EXE_l1-test-worker` は既存 bin）

## Affected Files

- `crates/ruviform-plugin-host/src/bin/l1-test-video-worker.rs`（新規）— `run_video` + `RUVIFORM_TEST_*` env fault matrix
- `crates/ruviform-plugin-host/tests/l1_video_e2e.rs`（新規）— 公開面 only E2E（happy + fault + cross-kind + bound + real runner）
- `crates/ruviform-plugin-host/Cargo.toml` — `[[bin]] l1-test-video-worker` 追加

## Done When

- [ ] `l1-test-video-worker` が `run_video` で起動し、`RUVIFORM_TEST_*` env で `Render`/`initialize`/`deactivate`/params fault・crash/hang・`Rendered` semantic violation・over-limit params を制御できる
- [ ] `tests/l1_video_e2e.rs` が `PluginHost::video()` のみで happy-path（lifecycle・render 内容・pts/duration/color pass-through・params round-trip・`ParamInfo(None)`）を pass する
- [ ] cross-kind suite（kind filter Scan・`DuplicatePluginId`・`Unsupported`/`EndpointUnavailable`・direct `PluginHost` メソッドの video 答え）が pass する
- [ ] Failed parity（子死亡/`InstanceFailed`/create 中死亡/`Err` reply mirror 不変/semantic violation → `Err(Ipc)`/inert `BlockIn`/post-`Shutdown`/obligation (a)(c)）が pass する
- [ ] bound 拒否（`Instantiate` → `Err(Prepare(UnsupportedConfig))`・子未 spawn）と `render_timeout` 分離が pass する
- [ ] 実 runner lane（`ruviform-plugin-l1-video-runner`）が同一 wire 規約で happy + 代表 fault を pass する

## Verification

- [ ] `cargo test -p ruviform-plugin-host --test l1_video_e2e` — 公開面 E2E 全 pass
- [ ] `cargo test -p ruviform-plugin-export --test l1_video_runner_e2e` — export runner lane 全 pass
- [ ] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` が全 pass（audio suite 無退行を含む）
