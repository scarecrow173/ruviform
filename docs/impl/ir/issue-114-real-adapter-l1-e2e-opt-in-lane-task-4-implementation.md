---
id: "IMPL-034MTufALVIX739hFwiVkS"
type: "impl"
status: "completed"
title: "Issue #114 real adapter L1 E2E + opt-in lane — Task 4 implementation"
created: "2026-10-27"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-clap/tests/l1_real_adapter_e2e.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/tests/l1_real_adapter_e2e.rs"
      - type: "file"
        path: "docs/impl/ir/issue-114-real-adapter-l1-e2e-opt-in-lane-task-4-implementation.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-clap/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-clap/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/README.md"
      - type: "file"
        path: "crates/ruviform-plugin-ofx/README.md"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "docs/tasks/real-adapter-e2e-and-lane.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/test-specs/l1-real-plugin-lane-tests.md"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/real-adapter-e2e-and-lane.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #114 real adapter L1 E2E + opt-in lane — Task 4 implementation

## Summary

Issue #114 Task 4 完了 — plan actions 12–14（実プラグイン L1 E2E + opt-in
lane 配線）。`ruviform-plugin-clap` / `ruviform-plugin-vst3` の両 crate に
`tests/l1_real_adapter_e2e.rs` を新設し（455/493 行・各 3 test —
`#![cfg(feature = "host")]` + 全 test `#[ignore]` ゲート）、T1–T3 が
積み上げた bridge + runner bin + fixture cdylib を
`PluginHost`/`OutOfProcessSpec` 経由で実子プロセスとして駆動する L1
E2E を pin した。`mise.toml` に `test-l1-real-plugin`（3 段 lane）・
`lint-l1-real-plugin`、`ci.yml` に `l1-real-verify` job（3-OS matrix・
workflow_dispatch 内 additive）を配線し、OFX の L1 構造的境界（AC5）を
crate README に明文化、AGENTS.md / crates README / test-spec の doc 同期まで
実施した。

- `tests/l1_real_adapter_e2e.rs`（両 crate・新規）: 3 本ずつ —
  `real_adapter_l1_full_series`（`Scan`→`Instantiate`→`Initialize`→
  `Activate`→`StartProcessing`→32 block `process`（`ParamValue` event
  込み）→`SaveState`/`RestoreState`/`OpenGui`/`CloseGui` 全て
  `Err(Unsupported)`→`StopProcessing`→`Deactivate`→`Destroy`→clean
  exit）、`real_adapter_l1_kill_reports_failed_once`（`kill_child`→
  `Failed` 収束→`InstanceFailed` exactly once→`Err(Failed)` 系 defined
  拒否・pass-through 意味不変・自動再起動なし）、
  `real_adapter_l1_missing_runner_is_ipc_rejection`（不在 runner → spawn
  `Err(HostError::Ipc)` pre-Hello・`Failed` 未到達・notification なし）。
  fixture cdylib / runner bin の欠落は `assert!` fail（silent skip なし）。
- `mise.toml`: `[tasks.test-l1-real-plugin]` 3 段（`cargo build
  --features host` → `cargo test --lib --bins`（bridge 契約層 + runner
  bin の `#[cfg(test)]` fixture 解決 unit test）→ `cargo test --test
  l1_real_adapter_e2e -- --ignored`）+ `[tasks.lint-l1-real-plugin]`
  （clippy `--all-targets --features host -D warnings`）。
- `ci.yml`: `l1-real-verify` job — `verify`/`icc-verify` 同型 steps・
  3-OS matrix（`fail-fast: false`）・`mise run test-l1-real-plugin` +
  `mise run lint-l1-real-plugin`・別 workflow ファイル・新規 trigger
  なし。macOS leg が raw `.dylib` CLAP 解決と VST3 `bundleEntry` 経路の
  初実行である旨を matrix comment に記録。
- `Cargo.toml`（両 crate）: `[dev-dependencies] ruviform-plugin-host`
  （E2E が `PluginHost`/`OutOfProcessSpec` を消費 — dev-dep 方向で
  adapter→host の依存 DAG 非環を維持）。`Cargo.lock` に dev-dep edge
  2 件の差分。
- doc 同期: `AGENTS.md` task 表 2 行 + ci job 記述更新・`crates/README.md`
  adapter 行・両 crate README に `feature "host"` 節（bridge/runner/
  lane/`Err(Unsupported)`/VST3 `uid()` platform-order id hex 注意）・
  `ruviform-plugin-ofx/README.md` に L1 境界節（AC5・Issue #118 追跡）・
  `docs/test-specs/l1-real-plugin-lane-tests.md` の lane 3 段定義を
  `--lib --bins` に同期。

敵対的レビュー 1 ラウンド — **確定修正 6 件**を全て反映（「Review
findings and resolutions」参照）。特に `cast_unsigned` 指摘は reviewer の
「identity no-op」判定自体が誤りで、fix 側が `TUID` の `c_char`（`i8`）
語義を確認してコメント訂正で正しく収束させた。全 gate green 実績 +
`mise run test-l1-real-plugin` 全 lane 実測 green（`--lib --bins`
63+2/47+2・`--ignored` E2E 3+3 実子プロセス）。

## Context

approved plan `docs/plans/implement-issue-114-real-adapter-l1-e2e.md` の
Task 4（`docs/tasks/real-adapter-e2e-and-lane.md`・actions 12–14）。
upstream: approved design
`docs/designs/issue-114-real-adapter-l1-e2e.md`（決定サマリ g/h/i —
「E2E」「lane」「OFX 境界」「既知値 const pin」「ParamValue 係数」）・
spec `docs/specs/issue-114-real-adapter-l1-e2e.md`（AC2–AC7 — E2E・
異常系・OFX 境界・CI lane・契約不変）・test-spec
`docs/test-specs/real-adapter-l1-e2e-tests.md`（E2E 契約・既知値・係数・
kill/欠落系）と `docs/test-specs/l1-real-plugin-lane-tests.md`（lane
機構・`#[ignore]` ゲート・fixture 欠落 fail・CI job 配線 — 本 task が
`--lib --bins` へ同期した文書）。
`crates/ruviform-plugin-host/tests/l1_proxy_e2e.rs`（`runner_program()`/
`mixed_routes_full_series`/`wait_host_failed`/`wait_host_notification`/
`host_block`/`out_list` reborrow・bit-exact `sample_eq` の先例）を構造的
に踏襲。`depends-on`: Task 3
`docs/tasks/real-adapter-runner-bins.md`（runner bin + fixture 解決 —
E2E の `CARGO_BIN_EXE_*`・sibling fixture path がこれに依存）と
T1/T2 bridge（`factories()`/`descriptors()`/`fixture_lib_name` を runner
経由で消費）。Task 5（final verification + closeout）の直接前提。

## Implementation

### Changes

- `crates/ruviform-plugin-clap/tests/l1_real_adapter_e2e.rs`（新規・455
  行）: `#![cfg(feature = "host")]`（feature off で空 — `env!(
  "CARGO_BIN_EXE_*")` も未定義になり default `cargo test` 経路と完全
  分離）+ 全 test に `#[ignore = "<理由>"]`。`EXPECTED: PluginDescriptor`
  を const pin（`org.ruviform.clap-gain-poc`/`Ruviform Gain PoC`/`Ruviform`/
  `0.1.0`/`&["audio-effect","stereo"]` — 子が実 `clap_plugin_factory`
  scan から合成する値と独立に親側が pin し、`Instantiate` handshake の
  `Descriptor::matches` で一致を要求）。`runner_program()` は
  `env!("CARGO_BIN_EXE_ruviform-plugin-clap-l1-runner")` + `is_file()`
  assert・`fixture_path()` は runner sibling + `fixture_lib_name()`、
  `assert_fixture_present()` で欠落 fail（skip 経路なし）。主 E2E は
  `OutOfProcessSpec::new(EXPECTED, runner)`（`spec.args` 空・既定
  `lookahead_blocks=2`/`command_timeout=10s`）→ `PluginHost` で全
  lifecycle + 32 block `process`（`ParamValue::new(0.25)` → CLAP plain
  `0.5` gain・delivered block = gain × lookahead 前 input の membership
  判定 + `delivered >= 8` floor + 先頭 2 call の pre-lookahead silence
  assert）+ state/GUI 全動詞 `Err(Unsupported)` + clean teardown（exit
  0・notification なし）。異常系 2 本は kill（`Failed` 収束・
  `InstanceFailed` once・exit status 同梱・`Err(Failed)` 系 defined
  拒否・pass-through 意味不変・`Destroy` で escape）と binary 不在
  （`Err(Ipc)` pre-Hello・route は登録維持・notification なし）。
- `crates/ruviform-plugin-vst3/tests/l1_real_adapter_e2e.rs`（新規・493
  行）: 同型。差分は descriptor `id` の derive — `GainComponent::CID` が
  `pub(crate)` で `tests/` から不可のため、`vst3::uid(0x5A1B2C3D,
  0x4E5F6A7B, 0x8C9D0E1F, 0x2A3B4C5D)`（vendored deterministic `const
  fn`・prod dep の test target から参照可）の raw 16 byte を test 側で
  独自に lowercase hex 化（`String::leak` で `'static` 化・call bounded）。
  `#[cfg]`-conditional literal（Win `3d2c1b5a…`/非 Win `5a1b2c3d…`）は
  regression pin のみ。`TUID` 要素は `c_char`（本 platform では `i8`）
  のため hex index 前に `byte.cast_unsigned()` で符号ビットを正しく
  reinterpret（review 指摘対応 — 後述）。gain 係数は plain `0.25`（VST3
  fixture は normalized ≡ plain）。
- `mise.toml`（+21 行）: `[tasks.test-l1-real-plugin]` 3 段 run 配列 +
  `[tasks.lint-l1-real-plugin]` — lane 契約（fixture+runner 生成 pin・
  unit test 層分離・`--ignored` E2E）を description と header comment に
  明記。
- `.github/workflows/ci.yml`（+49 行）: `l1-real-verify` job を既存
  workflow_dispatch workflow 内に additive 追加 — `verify`/`icc-verify`
  同型 steps（checkout → mise-action → `mise install --locked` → cargo
  cache → run）・3-OS matrix `fail-fast: false`・macOS 初実行旨の
  comment・ALSA step なし（adapter dep 木に cpal 非出現）。
- `crates/ruviform-plugin-clap/Cargo.toml`・`crates/ruviform-plugin-vst3/
  Cargo.toml`: `[dev-dependencies] ruviform-plugin-host = { path = "../
  ruviform-plugin-host" }` + 意図 comment（dev-dep 方向で DAG 非環）。
  `Cargo.lock`: 両 adapter package に `ruviform-plugin-host` dep edge 追加。
- `crates/ruviform-plugin-ofx/README.md`: 「L1（out-of-process）境界 —
  Issue #114 AC5」節を新設 — `VideoPluginInstance` 全操作 NONRT・video
  wire 非確立のため L1 子プロセス経路（`run(factories)` + audio shm
  block）に構造上載らない・Issue #118 追跡・延期 trigger = NONRT video
  port 分離要求。
- doc 同期: `AGENTS.md`（検証入口 task 表に 2 行 + ci job 記述を
  `l1-real-verify` 3-OS matrix 含めて更新）・`crates/README.md`（両
  adapter 行）・両 adapter crate README に `feature "host"` 節（bridge
  概要・runner・`RUVIFORM_L1_REAL_PLUGIN_DIR`・lane 入口・
  `Err(Unsupported)` state/GUI・VST3 `uid()` platform-order id hex
  注意）・`docs/test-specs/l1-real-plugin-lane-tests.md`（lane 3 段定義・
  Rules・Examples・Guarantees・Risk を `--lib` → `--lib --bins` に同期
  + macOS 初実行 Risk 行追加）。

### Review findings and resolutions（敵対的レビュー 1 ラウンド — 確定修正 6 件・全て反映済み）

- **`--lib` → `--lib --bins`（lane カバレッジ欠落）**: 初版 lane の第 2
  段が `--lib` のみで、T3 の runner bin 自身の `#[cfg(test)]` fixture
  解決 unit test（各 bin 2 件）が lane/CI のどの step にも載らない構造的
  欠落を指摘。`mise.toml` run 配列を `--lib --bins` に修正し、lane 契約
  を参照する全文書（task doc・`l1-real-plugin-lane-tests.md` test-spec・
  AGENTS.md・両 crate README・ci.yml comment）を同期 — lane 契約の
  authoritative な定義は 1 箇所に留めず「同じ規約を各文書が同じ言葉で
  述べる」既存スタイルを維持した。
- **delivered floor（`delivered >= 8`）+ pre-lookahead silence assert**:
  初版の `process` ループは非 silent block の membership のみ検査し、
  pipeline が一度も deliver しなくても pass し得た（delivered == 0 で
  ループ完走）。先頭 `lookahead_blocks`（2）call は seq−lookahead
  underflow で定義上必ず silence である assert（`l1_proxy_e2e`/
  `plugin_graph_e2e` の pin と同形）+ 30 eligible call 中 `delivered >=
  8` の floor を追加 — membership 意味論（stale-slot replay は defined
  underrun として delivered に計上）が先行例の exact `call−2`
  attribution（`matched >= 16`）より緩いこと、実 spawn adapter は in-repo
  stub worker より pace が遅れ得ることを comment に記録した上で、floor で
  sustained delivery を証明する。
- **macOS first-exercise note**: `l1-real-verify` の macOS leg が raw
  `.dylib` の CLAP 解決（非 bundle）と VST3 `bundleEntry` 経路の初実行
  であることを matrix comment と test-spec Risk に明記 — macOS job が
  赤い場合は platform finding として扱い、Linux/Windows の回帰と区別
  する判断基準を文書化。
- **`Scan` assert の意味を示す comment（tautology 誤読の防止）**: 親側
  `Scan` は composite の登録 descriptor 面を replay するだけで子の scan
  を証明しない — `assert_eq!(descriptors, vec![EXPECTED])` は登録の
  pin であり、child-synthesized descriptor との field-equality 証明は
  `Instantiate` handshake（`Descriptor::matches` vs 子の `Scanned` wire
  reply）に乗る旨を両 test に comment 明記（「Scan が tautology では」
  という誤読と、その逆に「Scan assert が child scan を証明する」という
  過剰読みの両方を排除）。
- **`cast_unsigned` は no-op ではない（reviewer 判定の訂正）**: reviewer
  は `byte.cast_unsigned()`（`expected_id` の hex 化ループ）を「`u8` 上の
  identity no-op」と主張したが、`vst3` の `TUID` 要素型は `c_char` —
  本 platform では `i8` であり、`cast_unsigned` は `i8 → u8` の実
  reinterpret（負値 byte が index に化けるのを防ぐ必須変換）。指摘自体が
  誤りと判明したため cast は維持し、要素型が `i8` である理由を comment
  に記録して収束 — 指摘の全採用ではなく根拠確認で正しく解決した例。
- **GUI 動詞 comment の正確化**: 初版 comment が `OpenGui`/`CloseGui` を
  state 動詞と同じ「wire 往復 `Err(Unsupported)`」に分類していたが、
  実際は `L1Instance::execute` が host-side で short-circuit し wire に
  出ない（L1 GUI deferred — defined rejection で round-trip ですら
  ない）。両 test の comment を「`SaveState`/`RestoreState` = wire 往復
  defined reply・`OpenGui`/`CloseGui` = host-side short-circuit」の区別に
  訂正。

### Decisions During Implementation

- **delivered-block は membership 意味論（exact `call−2` attribution は
  要求しない）** — stale-slot replay も defined underrun 出力であり、実
  spawned adapter の pacing は in-repo stub より遅れ得る。defined 値のみ
  が出現すること（全出力 sample が `EXPECTED_GAIN ×` 過去 input のいずれ
  か + channel sign）を検査し、floor で pipeline の持続的 delivery を
  別途 pin する 2 層にした（先行例の厳格 attribution を実 adapter の
  defined 語義へ緩和 — spec の「delivered block == gain × lookahead 前
  input・underrun/pass-through 意味不変」と整合）。
- **`spec.args` 空・既定 `lookahead_blocks=2`/`command_timeout=10s`** —
  `run` argv 規約不変（T3 と同じ決定 — plugin path/loader 引数を argv
  に追加しない）。latency 断言 `latency_frames == 1024`（2×512）で
  pipeline depth を pin。
- **fixture/runner の欠落は `assert!` fail のみ** — env gate・早期
  `return`・`eprintln!` skip は一切なし（test-spec Rules の silent
  skip 禁止を構造で担保）。`RUVIFORM_L1_REAL_PLUGIN_DIR` は runner 側
  shell-only override で test 側の skip 条件にしない。
- **VST3 `id` は `vst3::uid(...)` bytes から実行時 derive** — `tuid_hex`
  （prod）と独立な経路で hex 化し、写像を「再述」ではなく「検証」に
  する（`uid()` byte order の platform 依存 — Windows は先頭 GUID
  field が反転 — を literal regression pin で両形 pin）。
- **kill test は notification exactly once + exit status 同梱** —
  `InstanceFailed` 到達後に queue drain 確認（二重 emit なし）・reaped
  status は code（Windows）か signal（unix）のいずれか必携・以後の
  `Initialize`/`SaveState` は `Err(Failed)`・`Scan` は route 生存で
  `Ok`・`Destroy` が escape — defined crash path の全域を pin。
- **binary 不在 test は spawn `Err(Ipc)` で完結** — `Failed` 未到達・
  notification なし・route 登録は維持（`Scan` `Ok`）— pre-Hello
  rejection が hang せず defined に収束することの pin（runner の
  missing-fixture `exit(1)` と同じ収束形）。
- **`ruviform-plugin-host` は dev-dep** — E2E test だけが `PluginHost`/
  `OutOfProcessSpec` を消費し、adapter→host は dev-dep 方向で依存 DAG
  の非環を維持（`cargo tree` で audit — `host` off で新規 dep が
  non-optional 木に出ないことも確認）。
- **4 層分離** — `#[ignore]` + `#![cfg(feature = "host")]` +
  `required-features` + optional dep で default 経路（`cargo test
  --workspace`/`mise run test`/`check`/`lint`/`test-rt`/`verify-ci`）に
  一切差分を出さない（test-spec Guarantees・task doc Done When どおり）。

## Related Experiments

### Adopted

なし — approved design/plan の確定実装のみ（実験ログなし）。

### Rejected

なし — 代替実装の評価は design の確定事項（`#[ignore]` opt-in lane・
`spec.args` 不使用・compile-time 固定 fixture・dev-dep 方向）をそのまま
実装した。

## Validation

- default gate（実績・全 green）:
  - `mise run fmt-check` — exit 0
  - `mise run check` — exit 0（`l1_real_adapter_e2e.rs` は feature off で
    空・runner bin は `required-features` で非生成 — default 構成不変）
  - `mise run lint` — exit 0（clippy `-D warnings` clean）
  - `mise run test` — exit 0（`cargo test --workspace` — `#[ignore]`
    E2E 非実行・default 経路無差分）
  - `mise run test-rt` — exit 0（`rt_` filter — 本 task は test 配線
    のみで RT-suitable API 非追加・既存 RT test 無退行）
- opt-in lane（実績・全 green — `mise run test-l1-real-plugin`）:
  - 段 1 `cargo build -p ruviform-plugin-clap -p ruviform-plugin-vst3
    --features host` — exit 0（fixture cdylib + runner bin 生成）
  - 段 2 `cargo test ... --features host --lib --bins` — exit 0（CLAP
    lib 63 + bin 2・VST3 lib 47 + bin 2 — in-process bridge 契約層 +
    runner bin fixture 解決 unit test）
  - 段 3 `cargo test ... --features host --test l1_real_adapter_e2e --
    --ignored` — exit 0（CLAP 3 + VST3 3 — `real_adapter_l1_full_series`
    ×2・`real_adapter_l1_kill_reports_failed_once` ×2・
    `real_adapter_l1_missing_runner_is_ipc_rejection` ×2 全 pass・実
    fixture cdylib を dlopen する実子プロセスで全 lifecycle・32 block
    shm process・state/GUI `Err(Unsupported)`・kill/欠落異常系を往復）
  - `mise run lint-l1-real-plugin` — exit 0（`--all-targets --features
    host -D warnings` — bridge・runner bin・E2E test target 0 warning）
  - `cargo test -p ruviform-plugin-clap -p ruviform-plugin-vst3
    --features host`（`--ignored` なし）— exit 0（unit test のみ・
    `#[ignore]` E2E 除外を確認）
- dep 木監査: `cargo tree -p ruviform-plugin-clap -p ruviform-plugin-vst3`
  — adapter→export/host(dev) 方向のみ・逆向き依存ゼロ・`host` off で新規
  dep が non-optional 木に出現しない。
- 対象 test-spec: `docs/test-specs/real-adapter-l1-e2e-tests.md`・
  `docs/test-specs/l1-real-plugin-lane-tests.md`（`verified-by` edge —
  E2E 契約・係数 pin・kill/欠落系・lane 機構・`#[ignore]` ゲート・
  fixture 欠落 fail・CI job 配線を担保 — 後者は本 task が `--lib
  --bins` 同期を行った側）。

## Non-Goals

- **full gate の再配線・CI 自動化** — `l1-real-verify` は workflow_dispatch
  専用の既存 ci.yml 内 additive job（別 workflow ファイル・push/PR/schedule
  trigger は作らない — AGENTS.md の manual-verification 方針どおり）。
- **macOS `.vst3` bundle / raw `.dylib` 解決の追加対応** — bridge は既存の
  解約規約をそのまま使い、macOS leg の初実行結果は `l1-real-verify` の
  3-OS matrix で収集する（赤い場合は platform finding として別途対応 —
  ci.yml comment・test-spec Risk に記録済み）。
- **OFX の L1/video wire 確立** — `VideoPluginInstance` は構造上 L1 audio
  shm 面に載らない（AC5 の境界明文化のみ・Issue #118 で追跡）。
- **state/GUI 動詞の実装** — `Err(Unsupported)` の defined 応答 pin のみ
  （bridge は規格 state/GUI 動詞を呼ばない・偽の round-trip 主張なし —
  L1 GUI は deferred）。
- **`delivered` floor の厳格化** — `delivered >= 8` は実 adapter pacing
  を見込んだ実測 floor（in-repo stub の `matched >= 16`/exact
  attribution より緩い）。より強い pin は platform pacing 実績を見て
  T5/final verification 以降で検討する余地あり。

## Risks

- **opt-in lane は実行されなければ腐る** — `#[ignore]` 分離の性質上、
  `l1-real-verify` の手動 dispatch と runbook/README の `mise run
  test-l1-real-plugin` 明記で緩和（spec AC6・design 決定 h と整合 —
  test-spec Risk に記録済み）。
- **`delivered >= 8` は heuristic floor** — 実 spawned adapter の pacing
  が極端に遅い環境（高負荷 CI host）では defined 動作でも floor 未達の
  flaky になり得る。membership 検査自体は deterministic（defined 値のみ
  検査）で、floor 未達は pipeline の実質 non-delivery を意味するため
  検出としては正しいが、余裕を見て 8/30 に設定した（先代 16/30 の半分 —
  block ごと 5ms sleep + 10s command_timeout の headroom 込み）。
- **macOS leg は初実行** — raw `.dylib` CLAP 解決と VST3 `bundleEntry`
  経路は Windows/Linux で実測済みだが macOS 未実行（ci.yml comment・
  test-spec Risk の platform finding 注記どおり）。
- **`String::leak`（VST3 `expected_id`）** — call ごとに 1 度の bounded
  leak（test scope・`expected_descriptor()` 呼出回数分のみ — fixture
  bridge 側の `'static` 実体化と同じ戦略）。

## Follow-ups

- Task 5 `docs/tasks/real-adapter-final-verification.md` — full gate +
  opt-in lane 実測・AC 証跡・status 同期の final verification（本 task
  の lane/E2E を証跡として消費する直接の下游）。
- `l1-real-verify` の初回手動 dispatch — 3-OS matrix で macOS leg の
  raw `.dylib`/`bundleEntry` 経路を初実測（platform finding があれば
  別 task 化）。
- Issue #118 — OFX/video の L1 経路・video wire 確立（AC5 境界明文化の
  追跡先）。
