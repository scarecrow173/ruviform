# evidence/rf952 — virtual-endpoint-probe workflow（Slice A runner 実測）設計と検証記録

Issue #955（[Issue #952](https://github.com/scarecrow173/ruviform/issues/952) Slice A）の実装記録。`docs/designs/issue-952-virtual-endpoint-driver.md` §Slice A の scope (a)–(e) を GitHub-hosted runner 上で実測するための `workflow_dispatch` 専用 workflow を `.github/workflows/virtual-endpoint-probe.yml` に新設した。本ファイルは workflow の設計意図・ローカル検証結果・runner 実走の状態を記録する。**runner 実走は merge 後に実施済み**（run [45453855000](https://github.com/scarecrow173/ruviform/actions/runs/45453855000)・両 leg success — §5）。

## 1. workflow 設計

### 1.1 leg 構成と trigger

- `on: workflow_dispatch` のみ。`push`/`pull_request`/`pull_request_target`/`workflow_run`/`schedule`/`workflow_call` trigger を持たず、required gate・`ci-summary` の `needs`・ruleset とは一切接続しない独立 manual lane（`ci-yml-contract.test.mjs` の全 workflow 走査を通過済み — §3）。
- matrix `[windows-2025, windows-11-arm]`・`fail-fast: false`。
  - `windows-2025`（x64 Server 2025）: 対照 leg。`midisrv`/WMS は Server SKU 上に存在しないはずであり、**不在を実測として記録する**。
  - `windows-11-arm`（Win11 Enterprise 25H2, arm64）: WMS in-box 条件を満たす唯一の hosted image。probe example のネイティブビルド自体が `aarch64-pc-windows-msvc` の compile 確認を兼ねる（`rustc -vV` の host triple を evidence に記録し、実際に測ったアーキテクチャを明示する）。
- `permissions: contents: read`、`timeout-minutes: 60`、concurrency は `${{ github.workflow }}-${{ github.ref }}` で直列化（cancel-in-progress: false — 測定途中の証拠欠落を防ぐ）。
- Rust toolchain は repo 慣例どおり `jdx/mise-action@v5.2.0` + `MISE_ENV: ci`（`mise.ci.toml` の rust 1.99.0 pin、`mise install --locked rust`）。cargo registry cache も `ci.yml` と同じ `${{ runner.os }}-cargo-<lock hash>` キー（registry 内容は arch 非依存のため両 leg で共有される）。

### 1.2 「不在は失敗ではない」契約

測定対象の不在（`midisrv`・BLOOP transport・`midi` console・PowerShell module・audio endpoint 等）は**判定材料であり job 失敗ではない**。各 step は結果をテキストファイルへ記録して完走し、job が赤くなるのは probe 機構自体の破損（script エラー・checkout 失敗・artifact upload 失敗）のみ。cargo の build/実行も exit code をファイルに記録して継続する — `windows-11-arm` での compile 失敗そのものが Open Question 7 の実測値である。

### 1.3 probe 項目と design doc §Slice A scope の対応

| scope | 内容 | workflow 内の実装 |
| --- | --- | --- |
| (a) 環境 query | OS build・`Get-Service AudioSrv, AudioEndpointBuilder, midisrv`・WMS transport 状況・`midi` console の有無・PowerShell module の有無 | `probe-env-<os>.txt`：OS build（CIM + `ver`）、3 サービスの状態（不在時は `absent` 明示）、`midisrv` の PathName/ProcessId + ロード済み module 名の `midi|loop|transport|bloop` フィルタ、`System32\*midi*` ファイル一覧（`Midi2.*Transport.dll` が transport 同梱の直接観測になる）、`wdmaud2.drv`、AppX パッケージ、`midi --version`/`--help`（存在時）、`New-MidiLoopback`/`New-MidiBasicLoopback` 他の `Get-Command` |
| (b) 作成試行と WinMM 反映 | 到達可能な経路で transient loopback 作成 → WinMM への反映確認 | `loopback-attempt-<os>.txt`：route 1 `midi loopback create -a/-b`（transient・`--save-to-config` なし・観測のみ）、route 2 `New-MidiBasicLoopback -Name "RvfProbeBLOOP"`（BLOOP = verifier 意味論と整合する単一 endpoint）、route 3 `New-MidiLoopback -BaseName "RvfProbeLOOP"`（A/B pair・観測のみ）、route 4 `Windows.Devices.Midi2` 型解決の reachability。各 route の前後で `midi enumerate endpoints`（console 存在時）を記録し、`midi_endpoint_probe-after-loopback-<os>.txt` で WinMM への同名 in/out port 出現を観測 |
| (c) 既存 probe の実走 | `endpoint_probe`・`midi_endpoint_probe` | `endpoint_probe-<os>.txt` / `midi_endpoint_probe-<os>.txt`（`mise exec -- cargo run -p ... --example ...` + `exit=` 記録） |
| (d) Secure Boot | `Confirm-SecureBootUEFI` | `probe-env-<os>.txt`：`Confirm-SecureBootUEFI`（throw 時は error 記録）+ `HKLM\...\SecureBoot\State\UEFISecureBootEnabled` レジストリ値 + `bcdedit /enum`（testsigning flag の傍証） |
| (e) audio 側 query | `AudioSrv` 状態と MMDevice endpoint 列挙 | services 節 + `endpoint_probe-<os>.txt` の列挙出力 |
| 補足 | `devcon`/`pnputil`/`signtool`/`infverif` の有無（Slice B/D 入力） | `probe-env-<os>.txt`：PATH 上の有無 + `Windows Kits\10\bin` 配下の探索結果 |

### 1.4 provisioning 経路の制約遵守

- **service config JSON 直書きは実施しない**（Microsoft が明示的に非サポート — `web-sources-signing-runner-wms.md` §3.1）。読み取りを含む config file 操作も行わず、作成試行は `midi` console・PowerShell module・`Windows.Devices.Midi2` API のサポート経路のみ。
- LOOP A/B pair は verifier 意味論の危険（`loopback.rs` の name-match candidate が Fail 化）があるため**観測のみ**とし、round-trip 検証は行わない（接続は Slice C の範囲）。BLOOP 作成が主目的 — `New-MidiBasicLoopback` が到達可能なら単一 endpoint で試行する。
- transient endpoint は service restart / job 終了で消えるため cleanup step は持たない（ephemeral VM の性質をそのまま利用）。

### 1.5 evidence の出力形態

各 leg は 5 ファイルを job 内で生成し、`virtual-endpoint-probe-<os>-attempt-<n>` artifact として upload する（`if: always()`・retention 30 日・upload-artifact v4 の attempt 一意命名は repo 慣例）。ファイル命名は既存ローカル実測（`endpoint_probe-windows-local.txt` 等）に倣い `<probe>-<os>.txt` 形とし、merge 後に `evidence/rf952/` へ記録する際の対応を一意にする。

## 2. 実走結果の記録経路（merge 後・Coordinator 実施）

1. `gh workflow run virtual-endpoint-probe.yml --ref main` で `windows-2025`・`windows-11-arm` 両 leg を dispatch（**`workflow_dispatch` は default branch に workflow が存在しないと実行できない制約があるため、PR 段階では実行不可能**）。
2. 各 leg の artifact（`virtual-endpoint-probe-<os>-attempt-<n>`）を `gh run download` で取得し、`evidence/rf952/` 配下へ `<probe>-windows-2025.txt` / `<probe>-windows-11-arm.txt` として記録・コミットする。
3. 実測値（`midisrv` の有無/状態・BLOOP transport 同梱可否・loopback 作成可否・WinMM 反映・Secure Boot・aarch64 compile 可否）を Issue #955 へ記録し、`docs/discovery/issue-952-virtual-endpoint-driver-survey.md` の Open Questions 1–7 と `docs/designs/...` Deferred Design Concerns の MIDI 方式確定（WMS vs 独自 driver）を判定する。

## 3. ローカル検証記録（PR 段階・この worktree で実施）

| 検証 | コマンド | 結果 |
| --- | --- | --- |
| workflow 構文検査 | `mise exec -- actionlint .github/workflows/virtual-endpoint-probe.yml` | **Pass**（exit 0・指摘なし） |
| workflow contract 全走査 | `mise run test-ci-daily` | **Pass**（113 tests — `ci-yml-contract.test.mjs` の banned trigger 走査を含む） |
| tracker workflow contract | `mise run test-ci-tracker` | **Pass**（108 tests — `workflow_run` 非宣言の全ファイル走査を含む） |
| 衛生チェック | `mise exec -- prek run --all-files` | **Pass**（trailing-whitespace・end-of-file・check-yaml・check-toml・mixed-line-ending・BOM・private-key・actionlint 全て） |
| フォーマット | `mise run fmt-check` | **Pass**（exit 0） |
| Cargo `check`/`lint`/`test`/`build`・`*-full` | — | **NotApplicable**（`crates/` 変更なし — POLICIES §5 の設定変更区分） |
| runner 実走（両 leg の `workflow_dispatch`） | — | **NotRun**（§2 の制約 — merge 後に Coordinator が実施） |

補助証拠（canonical = mise ではない）: probe step の pwsh ロジックを本 worktree の Windows ホスト（`10.0.26300.9974` x86_64・`midisrv` Running・tools 系 absent）で抜粋実行し、全クエリ・全「absent/skipped」分岐・WinRT 型解決の error 記録が設計どおり動くことを確認した。この過程で `Get-AppxPackage -AllUsers` の非昇格エラーを「0 件」と誤記する欠陥を検出し、クエリ失敗と 0 件を分離する記録へ修正した。

## 4. 状態

- workflow 新設: `.github/workflows/virtual-endpoint-probe.yml`（本 PR）。
- 既存 workflow・required gate・ruleset への変更: **なし**。
- runner 実走: **実施済み** — run 45453855000（`workflow_dispatch`・main `20dc41f7`・両 leg success）。artifact 全文は本ディレクトリの `<probe>-windows-2025.txt` / `<probe>-windows-11-arm.txt` 群に記録（§5）。

## 5. runner 実走結果（run 45453855000・2026-11-24 UTC）

`gh workflow run virtual-endpoint-probe.yml --ref main` で両 leg を dispatch。job 結果は両 leg `success`（probe 機構の破損なし）。artifact 10 ファイルを `gh run download` で取得し本ディレクトリへ記録した。

### 5.1 windows-11-arm（Win11 Enterprise 25H2 build 26200・aarch64）

- `midisrv`: **存在** — `C:\Windows\system32\midisrv.exe`・StartType=Manual・job 中の probe 観測では `Running`（`probe-env`・`midi_endpoint_probe`）。
- WMS transport 同梱: **あり** — `System32` に `Midi2.LoopbackMidiTransport.dll`（BLOOP 相当）・`Midi2.VirtualMidiTransport.dll`・`Midi2.KSTransport.dll`・`Midi2.MidiSrvTransport.dll` 他 Midi2.\* 一式。`wdmaud2.drv`（WinMM 互換層）も存在。
- 作成経路: **全て absent** — `midi` console なし、PowerShell module（`New-MidiBasicLoopback`/`New-MidiLoopback`/`Get-MidiEndpointList`）なし、`[Windows.Devices.Midi2.MidiLoopbackManager]`/`MidiService` の WinRT 型解決は .NET PowerShell から not resolvable。loopback 作成は全 route skip → `provisioning: NOT_PROVISIONED`（WinMM 反映も未観測）。**API 経路は到達可能性未確認** — transport DLL は在るが管理 surface が in-box に無い。Slice C は compiled API client（`windows` crate → `Windows.Devices.Midi2`）または console/module の job 内 install が要る。
- WinMM/WinRT 実測: input 0 / output 1（GS Wavetable のみ・`DRV_QUERYDEVICEINTERFACE` はこの host の driver で crash → 入力側 query のみ）。
- WASAPI: endpoint 0 件（render/capture 共に存在しない・consent store microphone=Allow）。
- Secure Boot / test-signing: `Confirm-SecureBootUEFI`=False・`UEFISecureBootEnabled`=0・`bcdedit` testsigning=**Yes**。
- toolchain: `aarch64-pc-windows-msvc` で `ruviform-audio-device`（endpoint_probe 32s）・`ruviform-midi-device`（midi_endpoint_probe 3s）共にネイティブ compile+実行成功。
- driver tools: `pnputil` あり・`signtool` あり（Kits `10.0.22621.0\arm` 配下・PATH 外）・`devcon`/`infverif` なし。

### 5.2 windows-2025（Server 2025 build 26100・x64）

- `midisrv`/WMS: **完全に不存在** — サービス未登録、`Midi2.*` DLL なし（`midimap.dll`・`Windows.Devices.Midi.dll` のみ）、`wdmaud2.drv` なし、AppX 0 件。MIDI virtual endpoint の WMS 経路は Server SKU では成立しないことが実測で確定。
- WinMM: input 0 / output 1（GS Wavetable のみ）。WASAPI: endpoint 0 件。
- Secure Boot / test-signing: `Confirm-SecureBootUEFI`=False・`UEFISecureBootEnabled`=0・`bcdedit` testsigning=**Yes**。
- driver tools: `pnputil` あり・`signtool` あり（Kits `10.0.26100.0\arm64` 配下・PATH 外）・`devcon`/`infverif` なし。

### 5.3 Open Questions への判定（`docs/discovery/issue-952-virtual-endpoint-driver-survey.md` §OQ）

| OQ | 判定 | 実測根拠 |
| --- | --- | --- |
| 1. `windows-11-arm` で `midisrv` 有効か・console/module/BLOOP 同梱か | **midisrv 存在・Running（Manual）。BLOOP transport DLL 群 in-box。console/PS module は非同梱** | `probe-env-windows-11-arm.txt`・`midi_endpoint_probe-windows-11-arm.txt` |
| 2. BLOOP が GA image で in-box か・install 自動化 | **transport 自体は in-box**（LoopbackMidiTransport.dll）。作成経路の install 自動化可否は**未実測**（probe は presence のみ確認 — console/module install または API client 経路の要否が残る） | `probe-env-windows-11-arm.txt` |
| 3. WMS loopback が WinMM にどう見えるか | **未実測 — open**（作成経路 absent で未作成） | `loopback-attempt-*.txt` |
| 4. LOOP pair の verifier verdict | **未実測 — open**（同上） | 同上 |
| 5. attestation-signed driver が Server 2025 で install 可か | **未実測 — open**。ただし OQ6 の実測により test-signed driver install の可能性が開いた（attestation 前に検証できる経路になり得る） | `probe-env-windows-2025.txt` |
| 6. Secure Boot 有効状態 | **False / testsigning=Yes（両 leg）** — 「testsigning 経路は閉塞」という survey 時点の想定を**実測で否定**。test-signed driver の install 試行が GitHub-hosted で可能な環境である | `probe-env-*.txt`（bcdedit `testsigning Yes`・`Confirm-SecureBootUEFI False`） |
| 7. `aarch64-pc-windows-msvc` で compile 可か | **可** — `windows-11-arm` で両 probe example をネイティブ compile+実行成功（rust 1.99.0 pin 動作） | `probe-env`・`endpoint_probe`・`midi_endpoint_probe` の windows-11-arm 各 txt |
| 8. EV 証明書調達可否 | **repo 外工程 — open**。ただし OQ6 の実測で test-signing 経路が候補に浮上し、調達優先度は再評価対象 | — |

### 5.4 MIDI 方式確定への材料

- WMS（midisrv + Midi2.* transport）は **`windows-11-arm` で in-box だが管理 surface（console・PS module・.NET からの WinRT 到達）が無い**。loopback provisioning は compiled `Windows.Devices.Midi2` API client か console/module の job 内 install に依存する — Slice C で到達性を実測する必要がある（API reachability は PowerShell 型解決失敗のみ観測で、compiled 経路は未検証）。
- `windows-2025` では WMS 自体が存在しないため、Server 系 runner での MIDI 仮想 endpoint は独自 driver（OQ6 で test-signing install が候補に浮上）に限られる。
- audio 側は両 leg で WASAPI endpoint 0 件・仮想 endpoint 未 provisioning — 独自 driver（test-signed install 試行経路）または user-mode 経路の検討が継続。
