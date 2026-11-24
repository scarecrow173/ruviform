# evidence/rf977 — virtual-audio-driver-probe workflow（SysVAD 派生仮想 audio endpoint driver）設計と検証記録

Issue #977（[Issue #952](https://github.com/scarecrow173/ruviform/issues/952) Slice B stage 1）の実装記録。`drivers/rvf-virtual-audio/`（Microsoft SysVAD・MS-PL を render/capture 1 pair に最小化した KMDF port-class audio driver）と、その build→test-sign→install→devnode→endpoint 列挙→`endpoint_probe` pair 候補検出を stage 分解して実測する `workflow_dispatch` 専用 workflow（`.github/workflows/virtual-audio-driver-probe.yml`）・補助スクリプト（`tools/virtual-audio-driver/`）を新設した。本ファイルは設計意図・ローカル検証結果・runner 実走の状態を記録する。**runner 実走は PR merge 後に Coordinator が実施**（`workflow_dispatch` は default branch に workflow が存在しないと実行できない制約 — #955/#961 と同型の分担、§2）。

## 1. workflow / driver 設計

### 1.1 leg 構成と trigger

- `on: workflow_dispatch` のみ。required gate・`ci-summary` の `needs`・ruleset とは一切接続しない独立 manual lane（`ci-yml-contract.test.mjs` の全 workflow 走査を通過済み — §3）。
- matrix `[windows-2025]`・`fail-fast: false`。**x64 のみ** — ARM64（`windows-11-arm`）は VBS Code Integrity 稼働下で test-signing の kernel load が成立しない（rf961 attempt-9 実測 — problem 52 / CI event 3004）。
- `permissions: contents: read`、`timeout-minutes: 90`（WDK job 内導入 ~3min + driver build + cargo build of endpoint_probe を見込む）、concurrency は `${{ github.workflow }}-${{ github.ref }}` で直列化。
- `jdx/mise-action@v5.2.0`（install:false）+ `mise trust`/`mise install --locked rust` + `actions/cache@v6.1.0` cargo registry で `endpoint_probe` 実行用の Rust toolchain を復元（`virtual-endpoint-probe.yml` と同型。`MISE_ENABLE_TOOLS: rust`/`MISE_ENV: ci`）。

### 1.2 absence-is-data 契約（fail-closed — rf961 と同一）

- 各 stage の観測を `vaudio-probe-windows-2025.json` へ `{stage,status,detail,exit_code}` レコードとして追記し、生ログを `stage-<name>-windows-2025.txt` に tee。status 語彙は `tools/testsigned-driver/probe-stage.ps1` の ok/absent/blocked/not_run/error。
- stage の失敗・測定対象の不在は判定材料であり job 失敗ではない。`cleanup` と artifact upload は `if: always()`。
- `bcdedit` flag 設定・OS 再起動は一切行わない（testsigning=Yes+SecureBoot OFF は出荷状態として確定済み）。

### 1.3 stage 一覧

| stage | 内容 | 依存 |
| --- | --- | --- |
| `env-build-surface` | OS build・SecureBoot/testsigning・Kits 版・km headers/libs・`portcls/stdunk/libcntpr` lib・toolset・cert cmdlet・winget 列挙 | — |
| `driver-build` | vswhere→msbuild + `WindowsKernelModeDriver10.0` で `RvfVirtualAudio.vcxproj` Release\|x64 build。不在時は winget `Microsoft.WindowsWDK.10.0.26100` job 内導入 + DriverKit task dll workaround（rf961 踏襲） | env |
| `catalog-create` | `vaudio-pkg/` に sys+INF 集約 → Inf2Cat → makecat fallback で `RvfVirtualAudio.cat` | build |
| `cert-create` / `store-register` | self-signed CodeSigning cert → `LocalMachine\Root`+`TrustedPublisher` pre-trust | — |
| `sign` / `sign-verify` | `signtool sign /fd sha256 /sha1 /sm` → `verify /pa`（.sys+.cat 両方） | cat+cert |
| `pnputil-add-driver` | `pnputil /add-driver vaudio-pkg\RvfVirtualAudio.inf /install` | sign-verify |
| `enum-verify` | `enum-drivers` で INF の store 在籍 + devnode 0 件確認 | add-driver |
| `devnode-create` | `TestDevice.ps1 -Action create -HardwareId 'ROOT\RvfVirtualAudio' -ClassName MEDIA -ClassGuid {4d36e96c-…} -InfPath …` → Status/problem_code/`Win32_SystemDriver`/`Win32_SoundDevice` 観測 + problem_code 診断（CI event・setupapi.dev.log） | enum-verify |
| `endpoint-probe` | `Invoke-EndpointProbe.ps1` が `mise exec -- cargo run -p ruviform-audio-device --example endpoint_probe` を実行し `pair_count=`/`endpoints_enumerated=`/`pairs=` を記録 | devnode 非依存（観測） |
| `cleanup` | devnode remove → `pnputil /delete-driver /uninstall` → 証明書除去 → 最終 enum | — |

### 1.4 driver 側の設計判断

- SysVAD `TabletAudioSample` 構成から speaker+mic in の 1 pair のみ登録（`minipairs.h` 書き換え — `g_RenderEndpoints={SpeakerMiniports}`、`g_CaptureEndpoints={MicInMiniports}`）。
- endpoint 名は topology bridge pin の custom pin-name GUID→`MediaCategories` 登録で固定: render=`Ruviform Cable Input`（`{131373ee-…}`）、capture=`Ruviform Cable Output`（`{46eab9d5-…}`）。`endpoint_probe` の `VIRTUAL_NAME_HINTS` と `DIRECTION_TOKENS` stem（"ruviform cable"）に整合。
- KMDF（`DriverType=KMDF` + `KmdfService`/`KmdfLibraryVersion=1.33`— testsigned-null の WDM とは別系統。SysVAD が `wdf.h`/`wdfminiport.h` を共通 header で参照するため KMDF 維持が最小変更）。
- vendoring 対応表・変更点の正本は `drivers/rvf-virtual-audio/PROVENANCE.md`（upstream pin `d49bb7d7`・37 ファイルは意味差分 0 — `diff -bB --strip-trailing-cr` で検証。行末空白・EOL は repo hygiene hook で正規化・3 ファイル意図変更）。
- 詳細は `docs/designs/issue-977-virtual-audio-driver.md`。

## 2. 実走結果の記録経路（merge 後・Coordinator 実施）

1. `gh workflow run virtual-audio-driver-probe.yml --ref main` で dispatch（PR 段階では `workflow_dispatch` が動作しない制約 — #955/#961 と同型）。
2. artifact（`virtual-audio-driver-probe-windows-2025-attempt-<n>`）を `gh run download` で取得し、`vaudio-probe-windows-2025.json`・`stage-*-windows-2025.txt`・`endpoint-probe-windows-2025.txt` を `evidence/rf977/` へ記録・コミット。
3. 判定は stage 成立範囲の解釈として本ファイルへ追記（受入: build→sign→install→devnode→endpoint 列挙→pair 候補検出の各 stage の status）。

## 3. ローカル検証記録（PR 段階・この worktree で実施）

| 検証 | コマンド | 結果 |
| --- | --- | --- |
| workflow 構文検査 | `mise exec -- actionlint .github/workflows/virtual-audio-driver-probe.yml` | **Pass**（exit 0・指摘なし） |
| workflow contract 全走査 | `mise run test-ci-daily` | **Pass**（113 tests — banned trigger 走査を含む） |
| tracker workflow contract | `mise run test-ci-tracker` | **Pass**（108 tests） |
| 衛生チェック | `mise exec -- prek run --all-files` | **Pass**（全 hook） |
| フォーマット | `mise run fmt-check` | **Pass**（exit 0） |
| Cargo `check`/`lint`/`test`/`build`・`*-full` | — | **NotApplicable**（`crates/` 変更なし） |
| runner 実走（`workflow_dispatch`） | — | **NotRun**（§2 の制約 — merge 後に Coordinator が実施） |
| driver kernel build / 署名 / install / endpoint 列挙 | — | **NotRun**（worker ホストに WDK/VS 未搭載 — runner 上で判定） |

補助証拠（canonical = mise ではない・生出力は同ディレクトリの `local-*.txt`）:

- `local-syntax-check.txt`: `Invoke-EndpointProbe.ps1`・再利用 helper 2 本の `ParseFile` OK、workflow `run:` ブロック 14/14 parse OK、`RvfVirtualAudio.vcxproj` XML well-formed（item 数は text 側で ClCompile=16/Inf=1/ResourceCompile=1 を確認）。
- `local-inf-check.txt`: INF 構造 22 項目 OK（Version/Class=MEDIA/hwid/`KmdfService`+`KmdfLibraryVersion=1.33`/interfaces/HW AddReg/MediaCategories 両 GUID→名の token chain）+ `%Strings%` 未解決参照 0 + `minipairs.h` endpoint 名 4/4 一致 + pin-name GUID が toptable/INF 両側で一致。
- vendoring 検証: `drivers/rvf-virtual-audio/` の 37 ファイルが upstream pin `d49bb7d72f3651952b2b6e4ad28d14fe2247eb69` と `diff -bB --strip-trailing-cr` で意味差分 0（行末空白・EOL のみ repo hygiene hook で正規化）、意図変更は `minipairs.h`/`micintoptable.h`/`EndpointsCommon/speakertoptable.h` の 3 ファイルのみ（PROVENANCE.md の記述どおり）。
- 実行していないもの: `InfVerif`（worker 環境にツール不在）、kernel build（WDK 不在）、署名・store 書き込み・`pnputil` install・devnode 作成・`endpoint_probe` 実走 — 全て runner lane で判定。

## 4. runner 実走記録（Coordinator 実施）

### attempt 1 — run 45490145973（merge SHA `633bbc87`）

| stage | status | detail |
| --- | --- | --- |
| env-build-surface | ok | msbuild=present; toolset=3; kmHeaders=0→winget 導入; signtoolKits=3; winget=present |
| driver-build | **error** | `msbuild exit=1` |
| catalog-create / sign / sign-verify / pnputil | blocked | 前段 error のため未試行（契約どおり fail-closed） |
| cert-create / store-register | ok | thumbprint=EE0D58B9…・Root+TrustedPublisher |
| enum-verify / devnode / endpoint-probe / cleanup | error/ok | driver 不在のまま記録・cleanup ok |
| job | success | absence-is-data で全 step exit 0（rf961 と同契約） |

**失敗分析**: `MiniportAudioEngineNode.cpp` — upstream は `EndpointsCommon` を `SYSVAD_BTH_BYPASS;SYSVAD_USB_SIDEBAND` 定義の StaticLibrary として build するため本ファイルの sideband 呼出（`IsSidebandDevice()`/`m_pSidebandDevice`・8 箇所）が未ガードのまま compile する。本ツリーは macro 未定義の単一 vcxproj build のため宣言側（`minwavert.h`・guard 済み）と食い違い `C3861`/`C2065`。

**attempt-2 修正**: `MiniportAudioEngineNode.cpp` の `if (...) {...} else` 8 箇所へ他 upstream ファイルと同型の `#if defined(SYSVAD_BTH_BYPASS) || defined(SYSVAD_USB_SIDEBAND)` guard を適用（sideband 無効時は非 sideband path のみ compile — upstream 互換）。PROVENANCE.md を `＝`→`≠` に訂正。

### attempt 2 — run 45492389771（merge SHA `1d365aea`・attempt2-*.txt 同梱）

| stage | status | detail |
| --- | --- | --- |
| env-build-surface | ok | msbuild=present（**32bit** `MSBuild\Current\Bin` を選択）; toolset=3; kmHeaders=0→winget 導入 |
| driver-build | **error** | `msbuild exit=1` — sideband C3861/C2065 は解消・16 ファイル全て compile |
| 以降 | blocked | 契約どおり fail-closed |

**失敗分析（2 件の独立欠陥）**:

1. `LNK2019 operator new(size_t, POOL_TYPE, ULONG)` — upstream `EndpointsCommon.vcxproj` は `_NEW_DELETE_OPERATORS_` を定義し `NewDelete.cpp` が 3 引数 placement new を実装する。本 vcxproj で未定義のため宣言（PortCls header 側）だけ残り link 失敗。`_USE_WAVERT_` も合わせて ClCompile define へ追加（upstream と同じく sideband 系のみ除外）。
2. `INF verification exception: Unable to load DLL 'x86\InfVerif.dll'` — WDK は `InfVerif.dll` を `bin\amd64`/`arm64` のみ同梱。vswhere の `MSBuild\**\Bin\MSBuild.exe` find が 32bit `MSBuild\Current\Bin` を先に返すため x86 名で load を試みて失敗。**attempt-3 修正**: msbuild 選択を `Bin\amd64\` 優先に変更（env-build-surface / driver-build の 2 箇所）。
