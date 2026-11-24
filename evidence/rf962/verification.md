# ruviform#962 検証記録 — compiled `Windows.Devices.Midi2` API reachability probe（Slice C stage 1）

実施日: 2026-11-25 / worktree: `rf962-midi-wms` / push branch:
`feat/962-midi-wms-api-probe`。base SHA `102a972a`（branch 起点・
作業開始時点の origin/main）。

host OS: Windows 11 x64（`DESKTOP-NH00KUI` — WMS 非搭載 host。
`ActivatableClassId` 未登録の一般 image での absence-is-data 実測）。

## 成果物

- `crates/ruviform-midi-device/examples/midi_wms_api_probe.rs` —
  `Windows.Devices.Midi2` への staged WinRT reachability probe
  （`cargo run -p ruviform-midi-device --example midi_wms_api_probe`）
- `crates/ruviform-midi-device/Cargo.toml` —
  `[target.'cfg(windows)'.dev-dependencies]` に `windows 0.62`
  （`Foundation`/`Win32_System_Registry`/`Win32_System_WinRT`）と
  `windows-core 0.62` を追加（`windows` crate に Midi2 metadata が
  無いため `Win32_System_WinRT` の raw 関数のみ使用）
- `Cargo.lock` — 既存 `windows 0.62.2` / `windows-core 0.62.2` への
  dep edge 追加のみ（新規 package 解決なし）
- `.github/workflows/virtual-endpoint-probe.yml` — Slice C stage 1 の
  3 step（compiled probe・WinMM 再観測・loopback verifier）と
  3 artifact を追加
- `evidence/rf962/midi_wms_api_probe-windows-local.txt` — local host
  実走の raw 出力
- `evidence/rf962/actions.jsonl` — 検証アクションと設計判断の記録
- `evidence/rf962/*-windows-11-arm.txt` / `*-windows-2025.txt` —
  runner 実走 artifact（run 45475291648、branch ref dispatch —
  attempt-1、計 8 ファイル × 2 leg）

## probe 設計

- 到達経路: `RoInitialize`（MTA）→ `RoGetActivationFactory` /
  `RoActivateInstance`（runtime class ごと）→ 各 statics getter →
  `MidiDiagnostics.PingService` → `MidiSession.Create` +
  `IClosable.Close` → `MidiBasicLoopbackManager.CreateTransientLoopback`。
  全て microsoft/MIDI の IDL（`src/in-box/Client/WinRT/core/*.idl`）上
  `[noexcept]` 同期呼び出し — `IAsyncOperation` は不使用。
- IID と vtable slot 順は `src/in-box/Inc/midi_sdk_idl_defs.h` +
  各 IDL の宣言順から採取。`#[windows::core::interface]` macro は
  `IInspectable` parent を張れない（`IInspectable_Vtbl::new` の
  signature が異なる）ため、`macro_rules! winrt_interface!` で
  `IInspectable_Vtbl` 継承の `#[repr(C)]` vtable を手宣言する
  （windows-core 内部の `define_interface!` と同型。SAFETY 条件は
  ファイル内 comment 参照）。
- 宣言した vtable method は全て最低 1 回呼び出す（binding を実測し、
  仮定で済ませない）。property getter の read-back で slot 順の
  正しさを実測検証する設計。
- staged JSON-line 出力（`{"stage":"..","result":"..","k":"v"}`、
  result = `pass`/`fail`/`skipped`/`info`/`notrun`）。人間向けの
  progress 行（`activation: requesting ...`）は `println!` のまま
  混在させ、機械可読な観測は JSON line に限定。
- **absence-is-data**: どの stage の失敗も計測値であり probe は
  exit 0 で完走する。到達不可の後段 stage は `skipped` + `reason`。
  exit ≠ 0 は probe 機構自体の破損のみ。
- **BLOOP のみ作成試行**: `MidiBasicLoopbackManager`（basic loopback
  = 双方向同名の単一 endpoint）の `CreateTransientLoopback` のみ。
  LOOP A/B pair は `tests/loopback.rs` の name-match candidate が
  false `Fail` 化し得るため `IMidiLoopbackManagerStatics` は
  `IsTransportAvailable`/`TransportId` の観測までで作成しない。
- legacy class name 2 件（`Windows.Devices.Midi2.MidiService` /
  `MidiLoopbackManager` — `midi` console 文書が使う表記）は
  `REGDB_E_CLASSNOTREG` 期待の negative control として `info` 記録し、
  「class 未登録」判定が計測系の故障でないことを担保する。
- WMS service config JSON には一切触れない（Microsoft 非サポート面 —
  rf952 workflow contract 継承）。`virtual-os-endpoint` の claim は
  しない（midisrv は service であり OS endpoint ではない）。
- `verdict` stage は `api_reachable` / `furthest_stage` /
  `service_pinged` / `session_opened` / `bloop_created` を集約する。

## workflow 拡張（`.github/workflows/virtual-endpoint-probe.yml`）

既存 step 群の後段に追加（`<probe>-<os>.txt` 命名と per-leg artifact
は既存 contract に倣う）:

1. `Run midi_wms_api_probe` — compiled probe の実走。
   `windows-11-arm` leg では cargo build 自体が
   aarch64-pc-windows-msvc の native compile 確認を兼ねる。
2. `Re-run midi_endpoint_probe after compiled API probe` — 作成された
   transient BLOOP が process 終了後も WinMM から観測できるかの
   再測定（残存/消滅のどちらも計測値）。
3. `Run loopback round-trip verifier` — `mise run
   test-midi-virtual-endpoint`（BLOOP 残存時は実 round-trip、
   不在時は fail-closed `NotRun`）。

3 step とも exit code を `.txt` に記録した後で
`$global:LASTEXITCODE = 0` に neutralize する。pwsh は script 末尾の
`$LASTEXITCODE` を step 成否に伝播するため、計測値（probe の exit
code・verifier の `Fail`）が job を赤くしないよう「記録して継続」の
contract を新 step では明示的に実装した（job 失敗条件は機構破損のみ
という workflow 冒頭 contract の実現）。

## 実測記録（local gate — host OS）

| コマンド | 結果 |
| --- | --- |
| `mise run fmt-check` | exit 0（`cargo fmt --all -- --check` clean） |
| `mise run check ruviform-midi-device` | exit 0（`cargo check --locked --all-targets -p`、warning 0） |
| `mise run lint ruviform-midi-device` | exit 0（`cargo clippy --locked --all-targets -p -- -D warnings`、pedantic clean — `unsafe_code` は module-level `#![expect]` + per-site SAFETY comment で局所許可） |
| `mise run test ruviform-midi-device` | exit 0（全 test pass） |
| `mise exec -- actionlint .github/workflows/virtual-endpoint-probe.yml` | exit 0 |
| `mise run test-ci-daily` | exit 0（113 tests — `ci-yml-contract.test.mjs` の全 workflow trigger contract 走査を含む。`workflow_dispatch` 専用 lane 維持を確認） |
| `mise run test-ci-tracker` | exit 0（108 tests） |
| `mise exec -- prek run --all-files` | exit 0（全 hook Passed — actionlint changed-workflows・check-yaml・check-toml 含む） |
| `mise exec -- cargo run -p ruviform-midi-device --example midi_wms_api_probe` | exit 0 — raw 出力は `midi_wms_api_probe-windows-local.txt` |
| `mise run test-midi-virtual-endpoint` | exit 0 — `loopback_round_trip` は `verdict: NotRun — NOT_PROVISIONED`（fail-closed 挙動を実測）、`real_device_silent_output_smoke` は `verdict: Pass` |

### local probe 実測値（WMS 非搭載 host）

- `presence`: `Windows.Devices.Midi2.dll` /
  `Microsoft.Windows.Devices.Midi2.dll` 共に absent、winmd 0 件、
  `ActivatableClassId` 0 件 — client component payload はこの image
  に存在しない。rf952 で観測した WinSxS
  `microsoft-windows-devices-midi2` package が service-side file のみ
  （transports・`MidiSrv.exe`・`wdmaud2.drv`、client DLL なし）という
  構造と整合。
- `ro_initialize`: `pass`（MTA）— WinRT 初期化は API 非依存で到達。
- `activation`: 4 runtime class 全て `REGDB_E_CLASSNOTREG`
  （`0x80040154`）で `fail` — IID 別に記録。legacy class name 2 件は
  期待どおり同 HRESULT の `info`（negative control として機能）。
- `service_ping` / `session` / `bloop_create`:
  `skipped`（`*_statics_unavailable`）— 前段失敗で後段が skip される
  staged 設計どおり。
- `verdict`: `notrun` — `furthest_stage: activation`。非搭載 host での
  `pass` 偽装は無し（absence-is-data の実証）。

## runner 実走記録（run 45475291648 — branch ref dispatch）

既存 workflow への変更のため `gh workflow run
virtual-endpoint-probe.yml --ref feat/962-midi-wms-api-probe` で本
branch の workflow 定義を dispatch（#955/#961 の「merge 後実走」は
新規 workflow が default branch に存在しない制約に由来 — 本 PR は
既存ファイルの変更なので merge 前実走が可能）。attempt-1、両 leg
`success`。

### windows-11-arm leg（`midi_wms_api_probe-windows-11-arm.txt`）

- **aarch64-pc-windows-msvc native compile: Pass** — `Compiling
  ruviform-midi-device` / `Finished` / probe 実行まで完走（`env-arch:
  aarch64`）。
- `presence`: `Windows.Devices.Midi2.dll` absent・winmd 0 件・
  `ActivatableClassId` **0 件** — rf952 が WinSxS で確認した in-box
  `microsoft-windows-devices-midi2` package が service-side file のみ
  （transports・`MidiSrv.exe`）で client component を含まないことが、
  WinRT registration 面からも実測で確定。
- `ro_initialize`: `pass`（MTA）。
- `activation`: 4 runtime class **全て `REGDB_E_CLASSNOTREG`
  （`0x80040154`）** — `MidiDiagnostics`/`MidiSession`/
  `MidiLoopbackManager`/`MidiBasicLoopbackManager` の statics は
  いずれも activation factory 未登録。legacy class name 2 件も同
  HRESULT（negative control どおり）。
- `service_ping`/`session`/`bloop_create`: `skipped`
  （`*_statics_unavailable`）。
- `verdict`: `notrun` — `furthest_stage: activation`、
  `bloop_created: false`。

### windows-2025 leg（`midi_wms_api_probe-windows-2025.txt`）

- `presence`〜`activation` まで arm leg と同一 profile（client
  component absent・全 class `0x80040154`）→ `verdict: notrun`。
  Server SKU に WMS 自体が無いという前提と一致する対照計測。

### 後続 step（両 leg）

- `midi_endpoint_probe-after-wms-api-*`: `NOT_PROVISIONED` — API 未到達
  のため BLOOP は作成されず、WinMM 上に同名 in/out pair は出現しない
  （API 経路が閉じていることの WinMM 側裏付け）。
- `loopback-verifier-*`: `cargo test --test loopback -- --ignored` を
  実走。`loopback_round_trip` は両 leg で `verdict: NotRun —
  NOT_PROVISIONED`（fail-closed 維持）。`real_device_silent_output_smoke`
  は両 leg で **`verdict: Fail — 'GS Wavetable' output port failed to
  open: Backend("winmm midiOutOpen failed")`** — hosted runner 上の GS
  Wavetable は列挙されるが open 不能であることが実測された（本 slice
  の測定対象外だが fail-closed verifier が実測を記録した点は contract
  どおり）。`exit=101` を記録後に step 継続 — 両 job `success` で
  `$LASTEXITCODE` neutralize が意図どおり機能。

### 計測の結論（Slice C stage 1 の主問に対する実測値）

`Windows.Devices.Midi2` client API は **両 leg で activation factory
未登録 — compiled-API 経路の provisioning（BLOOP 作成を含む）は
hosted runner 上で到達不能**。rf952 が確認した in-box WMS は
service-side（`midisrv` + transport DLL）のみで、client component
（WinRT 型登録 + `midi` console + PS module）は image 非搭載。サポート
経路で BLOOP を provisioning するには client component の別途導入
（Microsoft の App SDK/console 配布物）か driver 経路（Slice B）が
必要という位置づけが実測で確定した。

## NotRun / 対象外

| 項目 | 状態 | 理由 |
| --- | --- | --- |
| runner 実走（両 leg） | **Pass — 実走済み** | run 45475291648（branch ref dispatch、attempt-1、両 job success）。artifact は本 dir の `*-windows-11-arm.txt` / `*-windows-2025.txt` |
| aarch64-pc-windows-msvc native compile | **Pass** | arm leg で example が compile/run 完走（`midi_wms_api_probe-windows-11-arm.txt` の cargo 出力） |
| `service_ping` / `session` / BLOOP 作成の到達 | **計測済み = 到達不能** | activation で全 class `REGDB_E_CLASSNOTREG` — 到達不能そのものが本 stage の計測結果。client component 導入後の再計測は slice 後続の判断事項 |
| transient BLOOP の残存観測・verifier round-trip | **計測済み = 不可** | BLOOP 未作成のため re-probe `NOT_PROVISIONED`・verifier `NotRun` — いずれも fail-closed の honest verdict |
| LOOP A/B pair 作成 | 対象外（statics 観測のみ） | `tests/loopback.rs` name-match verifier の false `Fail` 回避 — BLOOP 限定の設計 |
| WMS service config JSON | 対象外 | Microsoft 非サポート面 — 読み書きとも行わない |
| `virtual-os-endpoint` claim | 対象外 | provisioning 経路の到達可否の計測が slice の範囲 — midisrv は service であり OS endpoint としては扱わない |
