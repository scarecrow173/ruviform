# evidence/rf961 — virtual-driver-install-probe workflow（test-signed driver install 実測）設計と検証記録

Issue #961（[Issue #952](https://github.com/scarecrow173/ruviform/issues/952) Slice B 前提調査・OQ6 経路）の実装記録。`evidence/rf952/probe-env-*.txt` が実測した「GitHub-hosted Windows runner は両 leg で Secure Boot OFF・bcdedit `testsigning Yes`」という出荷状態を前提に、**test-signed driver の install 可否を最小コストで stage 分解して実測する** `workflow_dispatch` 専用 workflow（`.github/workflows/virtual-driver-install-probe.yml`）・最小 driver package（`drivers/testsigned-null/`）・補助スクリプト（`tools/testsigned-driver/`）を新設した。本ファイルは workflow の設計意図・ローカル検証結果・runner 実走の状態を記録する。runner 実走は PR merge 後に Coordinator が実施（`workflow_dispatch` は default branch に workflow が存在しないと実行できない制約 — #955 と同型の分担、§2）。**attempt 1（run 45471371020）実施済み — §3.5**。

## 1. workflow 設計

### 1.1 leg 構成と trigger

- `on: workflow_dispatch` のみ。`push`/`pull_request`/`pull_request_target`/`workflow_run`/`schedule`/`workflow_call` trigger を持たず、required gate・`ci-summary` の `needs`・ruleset とは一切接続しない独立 manual lane（`ci-yml-contract.test.mjs` の全 workflow 走査を通過済み — §3）。
- matrix `[windows-2025, windows-11-arm]`・`fail-fast: false`。
  - `windows-2025`（x64 Server 2025）: 優先 leg。WMS（midisrv + Midi2.\* transport）が存在しない SKU のため、仮想 endpoint は独自 driver 経路のみ（rf952 §5.2 実測）。test-signed install の可否が最も効く leg。
  - `windows-11-arm`（Win11 Enterprise 25H2, arm64）: 試行 leg。signtool の初ヒット kit 版が `10.0.22621.0\arm`（windows-2025 は `10.0.26100.0\arm64`）と leg 間で異なり、ARM64 kernel build の可否自体が測定対象。driver は `ARM64` platform で build する。
- `permissions: contents: read`、`timeout-minutes: 60`、concurrency は `${{ github.workflow }}-${{ github.ref }}` で直列化（cancel-in-progress: false）。
- Rust toolchain / mise は導入しない（`crates/**` 非接触のため cargo 不要。driver build は MSBuild + WDK のみ）。checkout は `drivers/`・`tools/` の取得に必要なため残す。

### 1.2 absence-is-data 契約（fail-closed）

- 各 stage の観測結果を leg ごとの `install-probe-<os>.json` へ `{stage,status,detail,exit_code,recorded_utc}` レコードとして追記し、stage 別の生ログを `stage-<name>-<os>.txt` に tee する。stage の失敗・測定対象の不在は **判定材料であり job 失敗ではない**。job が赤くなるのは probe 機構自体の破損（script エラー・checkout 失敗・artifact upload 失敗）のみ。
- status 語彙（`tools/testsigned-driver/probe-stage.ps1` に定義 — POLICIES §3・Issue #627 contract と同じ fail-closed 規則）:
  - `ok`（観測・実行成功）/ `absent`（測定対象が不在）/ `blocked`（前 stage の結果で実施不能）/ `not_run`（未実施）/ `error`（実行したが失敗 — exit code・stderr を detail に）
- 失敗時も evidence を必ず採取する — `cleanup` stage と `actions/upload-artifact` を `if: always()` で実行し、途中 stage が停止してもそこまでの JSON+ログ+build 済み package（`probe-pkg/`）が leg ごと artifact に残る。
- 前回 attempt の workspace 残存物は先頭の `Reset probe workspace` step で除去する（untracked の stage log・package が attempt を跨いで混ざらないよう stage 記録は attempt ごとに新規に積む）。

### 1.3 stage ↔ 受入条件（AC）対応表

| stage | AC | 内容 | 実装 |
| --- | --- | --- | --- |
| `env-build-surface` | AC3 本体 | OS build・Secure Boot/testsigning 再確認・Kits Include `km\wdm.h`/`wdf.h`/`ntddk.h`・WDK libs・`WindowsKernelModeDriver` platform toolset・`Inf2Cat`/`makecat`/`infverif`/`devcon`/`certutil`・`msbuild`/`vswhere`/`winget`・cert cmdlet・signtool（leg 別 kit 版の全 hit） | `stage-env-build-surface-<os>.txt` に全列挙、要約を JSON detail に |
| `driver-build` | AC3 + AC1 前半 | vswhere→msbuild 解決・`WindowsKernelModeDriver10.0` toolset 確認 → `msbuild /p:Platform=<x64|ARM64>` で `drivers/testsigned-null/` を build。tool 不在時は `winget install Microsoft.WindowsWDK` の job 内導入可否を評価（成否・所要時間を記録。EWDK ISO 経路は容量・帯域で非現実的と記録） | sys が生成されなければ後続の署名・install 系 stage は `blocked` |
| `catalog-create` | AC1 | `probe-pkg/` に sys+INF を集約 → `Inf2Cat /os:<10_0_X64|10_0_ARM64>` → 不在なら `makecat`（job 内生成 .cdf）で `.cat` 生成 | `.cat` が driver package 署名の対象。両 tool 不在なら `blocked` |
| `cert-create` | AC1 | `New-SelfSignedCertificate -Type CodeSigningCert -CertStoreLocation Cert:\LocalMachine\My` → `.cer` export（thumbprint を state に記録） | build 系と独立のため build 失敗時でも実施 |
| `store-register` | AC1 | `.cer` を `Cert:\LocalMachine\Root` と `Cert:\LocalMachine\TrustedPublisher` の**両 store** に `Import-Certificate` で pre-trust（Scream/VB-CABLE の job 内 install 実績と同手順 — `evidence/rf952/web-sources-signing-runner-wms.md` §2.3） | cert-create が ok の場合のみ |
| `sign` | AC1 | `signtool sign /fd sha256 /sha1 <thumbprint> /sm /v` を `.sys`（embedded）と `.cat` の両方へ。signtool は `Windows Kits\10\bin` 配下を full path 解決し、選択 path を detail に記録（`/sm` — 証明書は `LocalMachine\My` のため machine store を検索対象に指定） | sign 対象が無ければ `blocked` |
| `sign-verify` | AC2 | `signtool verify /pa /v` で署名の妥当性を確認 | sign が ok の場合のみ |
| `pnputil-add-driver` | **AC1 本体** | `pnputil /add-driver <inf> /install` で driver store への staging + install 試行。`Published Name : oemN.inf` を state に記録 | 署名検証・store・引数・class 制約の失敗切り分けは exit code + 出力で記録 |
| `enum-verify` | AC2 | `pnputil /enum-drivers` で INF の store 登録を確認（Published Name を entry 単位で逆引き）。この時点で devnode が無いことを `Get-PnpDevice` で記録 | staged 確認が devnode 作成の前提 |
| `devnode-create` | **AC2 本体** | `tools/testsigned-driver/TestDevice.ps1 -Action create` で root-enumerated devnode 作成（setupapi `DIF_REGISTERDEVICE`）→ `newdev!UpdateDriverForPlugAndPlayDevices(INSTALLFLAG_FORCE)` で driver 結合 → `Get-PnpDevice` の Status・`DEVPKEY_Device_ProblemCode`/`DevNodeStatus`・`Win32_SystemDriver` の load 状態を観測 | driver の PnP 列挙 + problem code で失敗 stage を特定 |
| `cleanup` | AC2 | `TestDevice.ps1 -Action remove`（`DiUninstallDevice`・fallback `pnputil /remove-device`）→ `pnputil /delete-driver <oem>.inf /uninstall /force` → 両 store+My から証明書除去 → 最終 `enum-drivers` 状態記録 | ephemeral VM だが uninstall まで記録するのが AC2 の要件 |

### 1.4 採用した設計判断と理由

1. **最小 driver package = repo 内ソース + in-job build 評価**。`drivers/testsigned-null/` に **WDM null driver**（`DriverEntry` + `AddDevice` + PnP/Power dispatch のみ・`wdm.h` のみに依存）の `.c` + universal INF + `.vcxproj` をソースのみで保持する。
   - KMDF ではなく **WDM** を選んだ理由: install 可否の判定が目的であり、KMDF は INF に `[DDInstall.Wdf]` / `KmdfService` 宣言と co-installer 経路の評価が必要になる。WDM なら `wdm.h` + `ntoskrnl.lib` のみで、INF は coinstaller なしの最小構成に留まる。
   - hardware id は `ROOT\RvfTestSigned`（独自名）・class は in-box の `System`（`{4d36e97d-...}`）— root-enumerated software device として作成する構成は `devcon install <inf> ROOT\<HwId>` と同型。
   - INF の model セクションを `NTamd64`/`NTarm64` に装飾（arm64 には `NTarm64` セクションが必要）。DDInstall は universal INF 形式どおり無装飾 `.NT` を共有。
   - build 済み `.sys`/`.cat`/`.cer` は commit しない（`drivers/.gitignore` で生成物を除外）。runner で build 不能と確定した場合の「prebuilt 同梱」切替は Coordinator 判断事項。
2. **devnode 作成 = setupapi/newdev P/Invoke（devcon 非依存）**。`pnputil /add-driver` は driver store staging までで root-enumerated devnode を作らず、`devcon` は runner に absent が確定済み（`probe-env-*.txt`）。`tools/testsigned-driver/TestDevice.ps1` に `Add-Type` の P/Invoke ラッパを実装した: `SetupDiCreateDeviceInfoList` → `SetupDiCreateDeviceInfo`（`DICD_GENERATE_ID`）→ `SetupDiSetDeviceRegistryProperty`（`SPDRP_HARDWAREID`）→ `SetupDiCallClassInstaller`（`DIF_REGISTERDEVICE`）→ `newdev!UpdateDriverForPlugAndPlayDevices`（`INSTALLFLAG_FORCE`）。devcon の job 内導入（WDK sample 由来 build/再配布）は再配布条項の確認が要るため代替案に留め、一次経路は P/Invoke とした。remove 側は `DiUninstallDevice`（fallback: `pnputil /remove-device`）。
3. **署名チェーン（標準 test-sign 手順）**。`New-SelfSignedCertificate`（`LocalMachine\My`）→ `.cer` export → `signtool sign /fd sha256 /sha1 <thumb> /sm`（`/sm` で machine store を検索対象に）→ `Root` + `TrustedPublisher` 両 store へ pre-trust → `signtool verify /pa` → `pnputil /add-driver /install` → enum/devnode → cleanup（devnode remove・`pnputil /delete-driver /uninstall`・両 store 除去）。**`bcdedit`・`Restart-Computer` は workflow に含めない**（testsigning=Yes は出荷状態で確定済み・reboot は runner session を kill して job 失敗＝evidence 破損 — `web-sources` §2.3）。
4. **stage 記録形式**。`install-probe-<os>.json`（構造化レコード配列）+ `probe-state-<os>.json`（stage 間の thumbprint・published INF 名・instance id の受け渡し）+ `stage-*-<os>.txt`（生ログ）を leg ごと artifact 化。失敗 stage の特定（AC2）は `status=error` の stage と detail で構造化して表現する。

## 2. 実走結果の記録経路（merge 後・Coordinator 実施）

1. `gh workflow run virtual-driver-install-probe.yml --ref main` で `windows-2025`・`windows-11-arm` 両 leg を dispatch（**`workflow_dispatch` は default branch に workflow が存在しないと実行できない制約のため、PR 段階では実行不可能**）。
2. 各 leg の artifact（`virtual-driver-install-probe-<os>-attempt-<n>`）を `gh run download` で取得し、`install-probe-<os>.json`・`stage-*-<os>.txt` を `evidence/rf961/` 配下へ記録・コミットする（`probe-pkg/` の build 済み binary は再現性の証拠として artifact に残し、repo への記録は JSON+テキストを基本とする — 判断は Coordinator）。
3. 実測値を AC ごとに評価し Issue #961 に記録する:
   - AC1: `pnputil-add-driver`（staging）と `devnode-create`（devnode 結合）までの成立 stage。
   - AC2: `Get-PnpDevice` Status / `DEVPKEY_Device_ProblemCode` / `Win32_SystemDriver` State・cleanup 結果。失敗時は `status=error` の stage + detail で失敗位置（署名・store・install 引数・class 制約・kernel load）を特定。
   - AC3: `env-build-surface`（WDK 有無・signtool・cert ツール）と `driver-build` の winget 導入可否評価 — job 内導入が必要なものを列挙。
   - AC4: 本ファイルの記録を `docs/designs/issue-952-virtual-endpoint-driver.md` の方式確定（attestation 前提 vs test-signing 経路）の更新材料とする — 更新自体は本 slice 範囲外（Coordinator/ユーザー判断）。

## 3. ローカル検証記録（PR 段階・この worktree で実施）

| 検証 | コマンド | 結果 |
| --- | --- | --- |
| workflow 構文検査 | `mise exec -- actionlint .github/workflows/virtual-driver-install-probe.yml` | **Pass**（exit 0・指摘なし） |
| workflow contract 全走査 | `mise run test-ci-daily` | **Pass**（113 tests — banned trigger 走査を含む） |
| tracker workflow contract | `mise run test-ci-tracker` | **Pass**（108 tests） |
| 衛生チェック | `mise exec -- prek run --all-files` | **Pass**（trailing-whitespace・end-of-file・check-yaml・check-toml・mixed-line-ending・BOM・private-key・actionlint 全て） |
| フォーマット | `mise run fmt-check` | **Pass**（exit 0） |
| Cargo `check`/`lint`/`test`/`build`・`*-full` | — | **NotApplicable**（`crates/` 変更なし — POLICIES §5 の設定変更区分） |
| runner 実走（両 leg の `workflow_dispatch`） | — | **NotRun**（§2 の制約 — merge 後に Coordinator が実施） |

補助証拠（canonical = mise ではない・`local-syntax-check.txt` / `local-inf-check.txt` に生出力）:

- pwsh 構文確認: `tools/testsigned-driver/*.ps1` を `[Parser]::ParseFile` で parse（2/2 OK）、workflow の `run:` ブロック 12 個を `${{ }}` 置換後 `ParseInput` で parse（12/12 OK）。
- `TestDevice.ps1` 埋め込み C# の `Add-Type` コンパイル確認（`[RvfTestDevice]` 型解決 OK — コンパイルのみ・P/Invoke の実行はしていない）。
- `probe-stage.ps1` の helper 実動確認（`Out-Probe` 追記・`Add-StageRecord`/`Get-StageStatus`・`Set/Get-ProbeState`・`Find-Tool` — temp dir 上で全 OK。証明書 store・devnode・driver install は未実行）。
- INF 構造テキスト検査（`local-inf-check.txt`）: `Signature`/`Class`/`CatalogFile`/`PnpLockdown`/`NTamd64`/`NTarm64` 装飾・hwid `ROOT\RvfTestSigned`・`DDInstall.NT`/`Services`/`%Strings%` 解決・workflow 側 `-HardwareId` との一致・vcxproj の `Inf`/`ClCompile` item（21/21 OK）。InfVerif は worker 環境に不在のため構文確認はここまで（Issue 宣言どおり「ローカル検証は INF/署名手順の構文確認まで」— 署名・証明書 store 書き込み・`pnputil` install・devnode 作成・driver build は実行していない。本機は WDK 非搭載のため実効不可能でもある）。

## 3.5 runner 実走記録（merge 後・Coordinator 実施）

### attempt 1 — run 45471371020（2026-11-24・merge SHA `49a2f258`・PR #965 直後の初回実走）

両 leg の artifact を §2 経路で `evidence/rf961/` に記録（`install-probe-<os>-attempt1.json`・`probe-state-<os>-attempt1.json`・`stage-*-<os>-attempt1.txt`）。job conclusion は両 leg failure だが、stage 記録（absence-is-data）は正しく採取された。

| leg | env-build-surface | driver-build | 実測値 |
| --- | --- | --- | --- |
| `windows-2025` | ok（toolset=3・kmHeaders=0・kmLibs=0・inf2catKits=0・winget=present） | error（msbuild exit=1 — `wdm.h` not found） | VS 18 Enterprise の `WindowsKernelModeDriver10.0` toolset target は在るが WDK headers/libs は未導入（SDK のみ）。winget は存在するが、初版 workflow は msbuild/toolset 欠如時にのみ導入試行する設計だったため未試行 |
| `windows-11-arm` | ok（toolset=3・kmHeaders=2・kmLibs=2・inf2catKits=1・winget=absent） | error（msbuild exit=1 — `Microsoft.DriverKit.Build.Tasks.18.0.dll` 欠如 MSB4062） | arm image は `Include\10.0.26100.0\km\` headers+libs+Inf2Cat を同梱するが、VS 18/MSBuild 18 に対応する DriverKit build task アセンブリが無く WDK targets が解決不能 |

判明した workflow 欠陥 2 件（本 PR で修正 — run の stage JSON で実証済み）:

1. **step exit-code 伝播**: pwsh step が最後のネイティブコマンドの `$LASTEXITCODE` を step 終了コードとして伝播するため、stage 記録を `error` として正しく残しても job が failure になる（absence-is-data 契約違反）。全 stage step 末尾に `exit 0` を追加し、probe 機構自体の破損のみが job 失敗になる形に修正。
2. **後続 stage skip**: stage step に `if: always()` が無く、1 stage の失敗で後続 stage が skip された（cert-create は「build 失敗時でも実施」設計だったが skip）。全 stage step に `if: always()` を追加。

併せて driver-build の WDK 導入可否評価を強化: winget 試行条件を「msbuild/toolset 欠如」から「msbuild/toolset/km headers のいずれか欠如」へ拡張（windows-2025 の `kmHeaders=0` で試行されるようになる）・`Microsoft.DriverKit.Build.Tasks.*.dll` の有無を診断記録に追加（windows-11-arm の失敗要因を stage 記録として構造化）。

AC 評価（attempt 1 時点・部分）:

- AC1/AC2: **未達** — build が両 leg で不成立のため install 系 stage は未実施（blocked/not_run）。
- AC3: **成立** — build surface・失敗要因・job 内導入に必要なもの（windows-2025: WDK 本体。windows-11-arm: VS18 対応 DriverKit build task。winget 導入経路は windows-2025 のみ存在）を stage 記録として取得。
- AC4: 材料となる記録を本節に追記。

### attempt 2-6 — 反復修正（各 run は修正 PR merge 直後に dispatch）

attempt 1 で検出した workflow 欠陥を修正しながら実走を重ねた。各 attempt の修正内容と判明事項:

| attempt | run | 修正 PR | 結果と判明事項 |
| --- | --- | --- | --- |
| 2 | 45472406140 | #967 | step `exit 0`/`if: always()` 修正で job green・全 stage 記録取得。cert-create/store-register は両 leg で ok（自己署名→Root+TrustedPublisher 登録は成立）。driver-build 継続失敗: windows-2025 の winget install が `Microsoft.WindowsWDK` の id 誤り（versioned id が必要）で `No package found` |
| 3 | 45472903820 | #968 | winget id を `Microsoft.WindowsWDK.10.0.26100` に修正 → windows-2025 で WDK install 成功（174s・kmHeaders 0→1・**winget による job 内 WDK 導入経路が実測で成立**）。新たな失敗: VS2026/MSBuild 18 は `Microsoft.DriverKit.Build.Tasks.18.0.dll` を要求するが image 同梱は 17.0 のみ → 既存 dll の rename workaround を試行したが copy 元 glob が `...PackageVerifier.17.0.dll` を先に拾い MSB4062 継続 |
| 4 | 45473752181 | #969 | copy 元を `Tasks.<n>.0.dll` 本体に限定 → `ValidateNTTargetVersion` 通過。次の MSB4062 は targets 698 行の `DPVerifierTask`（`...PackageVerifier.18.0.dll` も要求） |
| 5 | 45474564601 | #970 | copy を `Microsoft.DriverKit.Build.Tasks*.<n>.0.dll` → `.<maj>.0.dll` の family 全体に一般化 → **両 leg で driver build 成功**（x64/ARM64 `RvfTestSignedNull.sys` 生成）。次の失敗: sign stage で signtool の arch 選択ミス（x64 ホストが arm64 signtool を先頭 hit で選択）→ `ResourceUnavailable` |
| 6 | 45475270436 | #971 | signtool を `\x64\`/`\arm64\` Kits bin dir で arch-aware 選択 + try/catch → **driver-build/sign/sign-verify が両 leg で ok**。次の失敗: pnputil-add-driver「third-party INF does not contain digital signature information」— `.sys` 署名のみでは不十分で `.cat` catalog が必須。Inf2Cat は存在するが exit=-1・無出力 |

### attempt 7 — run 45476350137（2026-11-24・PR #973 merge 後）

両 leg の artifact を `evidence/rf961/` に記録（`install-probe-<os>-attempt7.json`・`stage-*-<os>-attempt7.txt`・`probe-state-<os>-attempt7.json`）。

| leg | 到達 | 実測値 |
| --- | --- | --- |
| `windows-2025` | catalog-create で step 停止 | driver-build ok（winget WDK）・cert/store/sign/sign-verify 全 ok。Inf2Cat は動作したが **INF の `DriverVer=11/25/2026` が未来日付で signability 拒否**（`22.9.7: DriverVer set to a date in the future`）→ makecat fallback も arch 非対応 binary（ResourceUnavailable）で script 停止・stage 記録未生成。後続 stage は `if: always()` で継続し pnputil は `.cat` 不在で error |
| `windows-11-arm` | **install チェーンほぼ完走** | catalog-create ok（Inf2Cat `/os:10_0_ARM64` で `.cat` 生成）→ cert/store/sign（.sys+.cat 両方）/sign-verify 全 ok → **pnputil-add-driver ok（`published=oem7.inf` — test-signed package が driver store に受理された）** → enum-verify ok（inf_in_store=True）→ devnode-create error（`SetupDiCreateDeviceInfo failed`）→ cleanup ok |

本 PR での修正（attempt 8 向け）:

1. **INF `DriverVer` を `01/01/2025` に修正** — postdated DriverVer は Inf2Cat の signability 検査で拒否される（attempt 7 の windows-2025 で実測）。
2. **`SP_DEVINFO_DATA` レイアウト修正** — `DevInst` を `IntPtr` から `uint` へ。実際の `SP_DEVINFO_DATA.DevInst` は DWORD で、IntPtr 宣言は x64/ARM64 で構造体を 40B（正しくは 32B）にしていた → `SetupDiCreateDeviceInfo` の失敗要因として最有力。併せて `Check` の例外に Win32 error code・native message を含めるよう改善（次回失敗時に原因切り分け可能）。
3. **makecat/Inf2Cat の native 起動を try/catch + arch-aware 選択** — makecat は signtool と同じく host arch と異なる binary を `$hits[0]` で拾うと ResourceUnavailable で step が死ぬ（attempt 7 の windows-2025 で実測・stage 記録すら残らなかった）。

AC 評価（attempt 7 時点・部分）:

- AC1（test-signed driver の install 可否）: **windows-11-arm で実測成立** — test-signed package が driver store に staging・install 受理された（`pnputil /add-driver /install` → `oem7.inf`）。windows-2025 は `.cat` 未生成で未到達（attempt 8 で再測定）。
- AC2（失敗 stage 特定）: windows-2025 = catalog-create（DriverVer postdated）、windows-11-arm = devnode-create（SetupDiCreateDeviceInfo）と stage 単位で特定済み。
- AC3（job 内導入に必要なもの）: windows-2025 = winget WDK install で build 成立を実測。windows-11-arm = DriverKit task dll rename workaround で build 成立を実測。
- AC4: 本節の記録。

### attempt 8 — run 45478442189（2026-11-24・PR #974 merge 後）

両 leg の artifact を `evidence/rf961/` に記録（`install-probe-<os>-attempt8.json`・`stage-*-<os>-attempt8.txt`・`probe-state-<os>-attempt8.json`）。

| leg | devnode-create | 実測値 |
| --- | --- | --- |
| `windows-2025` | **ok — 全経路完走** | catalog-create ok（Inf2Cat `/os:10_0_X64` で `.cat` 生成 — DriverVer 修正が的中）→ pnputil `oem12.inf` → devnode `ROOT\SYSTEM\0001` 作成・`status=OK; problem_code=0` → **`Win32_SystemDriver State=Running` — test-signed kernel driver が GitHub-hosted x64 runner でロード・実行中**。自己署名 → install → devnode → kernel load の全経路が成立 |
| `windows-11-arm` | error — load 時のみ署名拒否 | install 経路は全通過（catalog/sign/pnputil `oem7.inf`/devnode 作成・`.sys` が `system32\drivers` に配置）だが、devnode `status=Error; problem_code=52`（CM_PROB_UNSIGNED_DRIVER）・`Win32_SystemDriver State=Stopped`。**ARM64 kernel が load 時の署名検証で拒否 — testsigning=Yes が ARM64 kernel driver load に効いていない可能性**。拒否根拠（CI policy・status code）の切り分けが残件 |

本 PR での修正（attempt 9 向け）: devnode-create で `problem_code != 0` の場合に Code Integrity Operational event log・`setupapi.dev.log`（RvfTestSigned 関連行）・`bcdedit {current}`・`Confirm-SecureBootUEFI` を診断採取し、ARM64 kernel の拒否根拠を特定する。

AC 評価（attempt 8 時点・部分）:

- AC1: **windows-2025 で完全成立** — test-signed driver が install・devnode 作成・kernel load・Running まで実測完走。windows-11-arm は install/devnode まで成立・kernel load が problem 52 で拒否（要因は attempt 9 の診断で特定）。
- AC2: 全 stage が measured — windows-11-arm の残失敗は kernel driver load（problem 52）と stage・観測値で特定済み。
- AC3: windows-2025 = winget WDK install・windows-11-arm = task dll rename workaround ともに実測で経路確立。
- AC4: 本節の記録。

### attempt 9 — run 45479639538（2026-11-24・PR #975 merge 後・診断採取版）

両 leg の artifact を `evidence/rf961/` に記録（`*-attempt9.*`）。windows-2025 は attempt-8 と同じく全経路 ok（devnode `problem_code=0`・driver Running）。windows-11-arm は install/devnode まで同じく成立・problem 52 で拒否 — 追加した診断で**拒否根拠を特定**:

- CI Operational event `id=3004`: `RvfTestSignedNull.sys` の image integrity 検証失敗（hash がシステムで見つからない = 署名が kernel 信頼チェーンに載らない）
- `bcdedit {current}`: `testsigning=Yes`・`hypervisorlaunchtype=Auto` — **VBS（Virtualization Based Security）稼働中**
- CI policy 稼働一覧: `Microsoft Windows Driver Policy`・`Microsoft Windows Virtualization Based Security Policy`・`Endpoint Security Policy` が active（event 3099）
- `Confirm-SecureBootUEFI=False` — Secure Boot 自体は OFF
- `id=3085 Code Integrity will disable WHQL driver enforcement` — 緩和モードだが test-signed kernel load は拒否される

**結論**: ARM64 の `windows-11-arm` image では VBS policy 稼働下の Code Integrity が test-signing 緩和を honor せず、test-signed kernel driver の load が不可。解除には `hypervisorlaunchtype=Off`+reboot が要り ephemeral job 内では変更不可 — **ARM64 での test-signed driver load は GitHub-hosted runner では不成立**と確定。同一 package が x64（windows-2025・VBS 非稼働）では Running まで完走するため leg 間の差異は CI policy に帰因する。

最終 AC 評価（attempt 9 完了時点）:

- AC1（install 可否・stage 付き実測）: **成立** — windows-2025 = install→devnode→kernel load→Running 全通過（`oem12.inf`）。windows-11-arm = install 受理（`oem7.inf`）・devnode 作成まで成立・kernel load が CI policy で拒否（problem 52・root cause 特定済み）。
- AC2（PnP 列挙・uninstall 記録/失敗 stage 特定）: **成立** — windows-2025 は `ROOT\SYSTEM\0001` 列挙・cleanup stage で uninstall/devnode 削除まで記録。windows-11-arm の失敗 stage は kernel driver load（CI event 3004・VBS policy）まで特定。
- AC3（build 環境実測・job 内導入列挙）: **成立** — windows-2025: WDK 未導入→winget `Microsoft.WindowsWDK.10.0.26100`（実測 174s）・signtool x64 Kits・winget present。windows-11-arm: WDK headers/libs/Inf2Cat 同梱・DriverKit task dll は 17.0→18.0 rename workaround・winget absent・devcon 両 leg 非搭載（P/Invoke 代替実測済み）。
- AC4（evidence 記録・方式確定材料）: **成立** — 本ファイルの実走記録 + `docs/designs/issue-952-virtual-endpoint-driver.md` の方式確定を実測で更新済み（attestation 前提 → x64 は自己署名 test-signing で成立・ARM64 のみ attestation 要）。

## 4. 状態

- workflow 新設: `.github/workflows/virtual-driver-install-probe.yml`（PR #965 で merge、attempt 1 で検出した欠陥を修正済み — §3.5）。
- driver package 新設: `drivers/testsigned-null/`（ソースのみ — build 済みバイナリは commit しない）。
- 補助スクリプト新設: `tools/testsigned-driver/`（`probe-stage.ps1`・`TestDevice.ps1`・`README.md`）。
- 既存 workflow・required gate・ruleset・`mise.toml`・`crates/**` への変更: **なし**。
- runner 実走: **attempt 1-9 実施済み・計測完結**（§3.5）。windows-2025 = test-signed driver の install→devnode→kernel load→Running 全経路実測成立。windows-11-arm = install/devnode まで成立・kernel load は VBS CI policy で拒否（実測確定・job 内不可）。AC1-4 全て充足 — `docs/designs/issue-952-virtual-endpoint-driver.md` の方式確定も実測で更新済み。
