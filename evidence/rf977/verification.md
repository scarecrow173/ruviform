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

### attempt 3 — run 45493612214（merge SHA `6b0a0caf`・attempt3-* 同梱）

| stage | status | detail |
| --- | --- | --- |
| env-build-surface | ok | msbuild=present（依然 32bit `Current\Bin`） |
| driver-build | **error** | `msbuild exit=1` — compile/link 成功・残 2 欠陥（下記） |
| catalog-create 以降 | blocked | sys 未生成で fail-closed（endpoint-probe は absence 記録 ok） |

**失敗分析**:

1. `x86\InfVerif.dll` 再発 — amd64 優先フィルタが機能しなかった。vswhere の `-find 'MSBuild\**\Bin\MSBuild.exe'` は `Bin\amd64\MSBuild.exe` にマッチしない（パターンが `Bin\MSBuild.exe` 終端を要求）ため amd64 候補が一度も列挙されず常に 32bit fallback。**attempt-4 修正**: 32bit hit の `Bin\MSBuild.exe` → `Bin\amd64\MSBuild.exe` 派生 + Test-Path 確認。
2. `SIGNTASK: No file digest algorithm specified` — vcxproj に署名設定がなく WDK が build 内署名を試行（`/fd` 無し signtool 呼出で失敗）。署名は本 workflow の sign stage（job 生成証明書）が担う設計のため **`<SignMode>Off</SignMode>`** を両 config に追加。

**進捗**: `_USE_WAVERT_`/`_NEW_DELETE_OPERATORS_` 追加で compile+link は完走（LNK2019 解消）。

### attempt 4 — run 45495043095（merge SHA `9a269ab7`・attempt4-* 同梱）

| stage | status | detail |
| --- | --- | --- |
| env-build-surface | ok | msbuild=present |
| driver-build | **error** | `msbuild exit=1` — amd64 MSBuild 選択は生效（`Bin\amd64\MSBuild.exe` で InfVerif が実行され実検証へ到達） |
| 以降 | blocked | fail-closed |

**失敗分析**: `INF error 1199` — DIRID 13 (CopyFiles) は OS 10.0.16299 導入構文のため、その DDInstall を参照する Models section に TargetOSVersion decoration が必要。`SignMode Off` で build 内署名エラーは解消。**attempt-5 修正**: `[RvfVA.NTamd64]` → `[RvfVA.NTamd64.10.0...16299]`（本 driver は windows-2025 runner 専用で下位 OS 対象外）。

### attempt 5 — run 45496106387（merge SHA `8fc74098`・attempt5-* 同梱）

| stage | status | detail |
| --- | --- | --- |
| driver-build | **error** | `msbuild exit=1` — ただし **`RvfVirtualAudio.sys` 生成に成功**（compile/link/InfVerif/StampInf 全通過） |
| 以降 | blocked | fail-closed |

**失敗分析**: build 内 Inf2Cat（`MSB6006 exit=-2`）が signability test で失敗 — INF の `CopyFiles` 参照 `RvfVirtualAudio.sys` が package staging dir（`x64\Release\RvfVirtualAudio\`）に無い。flatten project では INF=プロジェクト dir・sys=x64\Release\ に分離するため必然的に解決不可。catalog 生成は workflow catalog-create stage が `vaudio-pkg\` staging で担う設計（rf961 と同型）のため `<RunInf2Cat>false</RunInf2Cat>` を追加。**残課題**: env の `inf2catKits=0` — Inf2Cat 不在なら makecat fallback（x64 arch-aware・実装済み）が動くかは attempt-6 で実測。

### attempt 6 — run 45497250172（merge SHA `fe712750`・artifact は attempt-5 と同条件のため stage 記録のみ）

| stage | status | detail |
| --- | --- | --- |
| driver-build | **error** | `msbuild exit=1` — sys は生成されるが build 内 Inf2Cat が依然実行 |
| 以降 | blocked | fail-closed |

**失敗分析**: vcxproj `<RunInf2Cat>false</RunInf2Cat>`（PropertyGroup）が無効 — WDK targets の既定値で上書きされた。**attempt-7 修正**: `/p:RunInf2Cat=false` を global property として msbuild コマンドラインで渡す（vcxproj の記述は撤去・comment で経緯を記録）。

### attempt 7 — run 45498616766（merge SHA `5c6ab5b3`・attempt7-* 同梱）

| stage | status | detail |
| --- | --- | --- |
| driver-build | **error** | `msbuild exit=1` — `/p:RunInf2Cat=false`（global property）でも build 内 Inf2Cat が実行 |
| 以降 | blocked | fail-closed |

**失敗分析**: `RunInf2Cat` はこの target のゲートではない（WindowsDriver.common.targets:1438 の Inf2Cat は別条件）。根本原因は **`FilesToPackage` 欠落** — upstream `TabletAudioSample.vcxproj` は `<FilesToPackage Include="$(TargetPath)"/>` で driver 出力を package dir（`x64\Release\RvfVirtualAudio\`）へ copy させている。本 vcxproj で欠落したため sys が package dir に無く、INF CopyFiles を signability test が解決できなかった。**attempt-8 修正**: `FilesToPackage` を追加し、in-build Inf2Cat を正常動作させる（`/p:RunInf2Cat=false` 撤去 — catalog-create stage 側でも `vaudio-pkg\` staging で実施する設計は維持）。

### attempt 8 — run 45499994447（merge SHA `5ff55207`・attempt8-* 同梱）— **install chain 完走**

| stage | status | detail |
| --- | --- | --- |
| driver-build | **ok** | `x64\Release\RvfVirtualAudio.sys` 生成（`FilesToPackage` で Inf2Cat signability 通過） |
| catalog-create | ok | `rvfvirtualaudio.cat` 生成 |
| cert/store | ok | 自己署名 → Root+TrustedPublisher |
| sign/sign-verify | ok | sys+cat 署名・検証 |
| pnputil-add-driver | **ok** | `published=oem12.inf` — test-signed audio driver が driver store に受理 |
| enum-verify | ok | `inf_in_store=True` |
| devnode-create | **ok** | `ROOT\MEDIA\0000; status=OK; problem_code=0; service=RvfVirtualAudio`・`Win32_SystemDriver State=Running` |
| endpoint-probe | ok | render `Speakers (Ruviform Virtual Audio Device)` + capture `Ruviform Cable Output` 両方 active・format/period 取得済。pair=0 |

**残件**: render endpoint 名が custom GUID ではなく "Speakers" — **Windows が KSNODETYPE_SPEAKER 系 Category の endpoint friendly name を仕様としてハードコード**（pin-name カスタム不可・OSR/MS Q&A 実例・upstream も Name=NULL）。capture 側は同一機構で custom name が表示される非対称。**attempt-9 修正**: LINEOUT_DEST pin の `Category` を `KSNODETYPE_SPEAKER`→`KSNODETYPE_LINE_CONNECTOR` に変更（VB-Cable/AudioMirror 系と同じ仮想 cable 構成 — line-connector では MediaCategories pin-name が有効）。

### attempt 9 — run 45501429195（merge SHA `ed00ccf5`・attempt9-* 同梱）— **全 AC 相当達成**

| stage | status | detail |
| --- | --- | --- |
| env-build-surface | ok | `msbuild=present; winget=present`（キャッシュで WDK surface 再利用） |
| driver-build | ok | `x64\Release\RvfVirtualAudio.sys` 生成 |
| catalog-create | ok | `rvfvirtualaudio.cat` 生成 |
| cert/store | ok | 自己署名 → Root+TrustedPublisher |
| sign/sign-verify | ok | sys+cat 署名・検証 |
| pnputil-add-driver | ok | `published=oem12.inf` |
| enum-verify | ok | `inf_in_store=True` |
| devnode-create | ok | `ROOT\MEDIA\0000; status=OK; problem_code=0; service=RvfVirtualAudio` |
| endpoint-probe | **ok** | **`pair_count=1` — `Ruviform Cable Input (Ruviform Virtual Audio Device)` ↔ `Ruviform Cable Output (Ruviform Virtual Audio Device)`**・`endpoints_enumerated=2; virtual_candidates=2` |
| cleanup | ok | |

**結果**: LINE_CONNECTOR 化により render endpoint に MediaCategories 登録の custom name（`Ruviform Cable Input` = `RVF_CABLE_INPUT_NAME` GUID 経由）が表示され、render↔capture の仮想 cable pair が `VIRTUAL_NAME_HINTS`（cable input/output + ruviform cable stem）で検出された。build→sign→install→devnode→**endpoint pair 検出**の全経路が GitHub-hosted `windows-2025` で成立 — rf#977 の受入条件に相当する実測が完結。

### attempt 10 — run 45503086872（merge SHA `cefb7642`・attempt10-* 同梱）— stream-roundtrip 初測定

| stage | status | detail |
| --- | --- | --- |
| 〜 endpoint-probe | ok | attempt-9 と同一経路・`pair_count=1` 再現 |
| stream-roundtrip | **error** | `test_exit=101; verdict: Fail` — `cargo test --features real-device --test virtual_endpoint` 実走 |
| cleanup | ok | |

**測定内容**: `mise run test-audio-virtual-endpoint` で `tests/virtual_endpoint.rs` の `render_capture_round_trip` を job 内実行（compile 含め ~30s — cargo cache で deps 解決済み）。

**失敗分析**: pair candidate 検出・render open・format 交渉（in=48k Mono F32 / out=48k Stereo F32）まで全て成立し、capture `IAudioClient::Initialize` のみ **`E_ACCESSDENIED` → `permission_denied`**。capture bridge pin の `Category=KSNODETYPE_MICROPHONE` が Windows mic privacy consent の対象となり、headless runner（Server 2025・interactive consent 不可）で capture open が拒否された。**attempt-11 修正**: `micintoptable.h` の capture bridge pin `Category` を `KSNODETYPE_MICROPHONE`→`KSNODETYPE_LINE_CONNECTOR` へ（render 側と対称。line-in は mic privacy gate 対象外かつ仮想 cable の形態として正しい — VB-CABLE 同型）。

### attempt 11 — run 45504161772（merge SHA `d6e830b6`・attempt11-* 同梱）

| stage | status | detail |
| --- | --- | --- |
| 〜 endpoint-probe | ok | attempt-9/10 と同一経路・`pair_count=1` 再現 |
| stream-roundtrip | **error** | `test_exit=101; verdict: Fail` — `input open failed: permission_denied` **再現** |
| cleanup | ok | |

**失敗分析**: `KSNODETYPE_LINE_CONNECTOR` 化でも capture open は `E_ACCESSDENIED` — **mic device capability の consent gate は endpoint Category 非依存**（全 capture endpoint が対象）であることが確定。gate の解除は driver 側では不可能で、runner 環境の provisioning（`DeviceAccess\Global\{E5323777-...}\Value=Allow`・HKLM+HKCU 両層）が必要。**attempt-12 修正**: stream-roundtrip stage 内で consent を Allow に provision し before/after を記録（ephemeral runner 限定・cert store 登録と同じ環境調整の一部）。driver 側の LINE_CONNECTOR は semantic 正当性のみを根拠として維持（PROVENANCE 訂正済み）。

### attempt 12 — run 45505257940（merge SHA `cc1e3ec2`・attempt12-* 同梱）

| stage | status | detail |
| --- | --- | --- |
| 〜 endpoint-probe | ok | `pair_count=1` 再現 |
| stream-roundtrip | **error** | `verdict: Fail` — consent=`HKLM:=Allow,HKCU:=Allow` 生效も `permission_denied` 再現 |
| cleanup | ok | |

**失敗分析**: `DeviceAccess\Global` の Allow 化だけでは不十分 — consent 評価は別系統（`CapabilityAccessManager\ConsentStore`・`AppPrivacy` policy）か audiosrv が起動時に cache する。**attempt-13 修正**: 残り全 consent 層（CAM ConsentStore HKLM+HKCU・AppPrivacy policy）+ `AudioEndpointBuilder`/`Audiosrv` restart で再評価を強制。

### attempt 13 — run 45506203119（merge SHA `c5dc4c05`・attempt13-* 同梱）— **capture open 成立・streaming 実走**

| stage | status | detail |
| --- | --- | --- |
| 〜 endpoint-probe | ok | `pair_count=1` 再現 |
| stream-roundtrip | **error** | `verdict: Fail` — ただし **両 stream open・callback 実走に到達**（consent 5 層 Allow + svc restart で `permission_denied` 解消） |
| cleanup | ok | |

**実走観測**: `callbacks: in=328 out=332 | frames: in=157440 out=158976`・`same_clock_domain=true`・`non_finite=0`・`peak_bits=0x3efffc00`（≈0.5 振幅）。consent provisioning（DeviceAccess+CAM ConsentStore HKLM/HKCU・AppPrivacy policy・audio svc restart）で capture open は完全解決。**残 Fail 3 件**は oracle 層:

1. `output timestamp regressions: 2` — render stream 先頭の position 報告異常（軽微）
2. `marker oracle: only 0 markers` — 刺激の marker burst が capture 側に観測されない
3. `tone oracle: 440 Hz share 0.000` — capture データに刺激の 440Hz が含まれない

**根本原因確定**: capture データの `observed_rms=0.3535`（≈0.5/√2）・440Hz share 0 — SysVAD の capture stream は **`ToneGenerator` で合成サイン波を生成する**（`minwavertstream.cpp` `WriteBytes`→`GenerateSine`）。**upstream SysVAD は render→capture の loopback を持たない** — speaker/mic は独立 endpoint で仮想 cable の経路が存在しない。pair 名による検出（AC2）は成立したが、真の round-trip（AC3）には **driver 内 kernel loopback の実装が必要** — render stream の DMA buffer を capture stream が読む経路（Stereo→Mono downmix・render 未稼働時 silence）を adapter 共有経由で新設する。これは小修正ではなく実装 slice — Worker へ委譲。

### attempt 14 — run 45511337020（branch `feat/977-vaudio-loopback`・SHA `765f5866`・attempt14-* 同梱）— loopback 実装・driver-build 失敗

| stage | status | detail |
| --- | --- | --- |
| env-build-surface | ok | `msbuild=present; winget=present` |
| driver-build | **error** | `msbuild exit=1` |
| catalog〜devnode | blocked | sys 未生成の連鎖 |
| endpoint-probe | ok（vacuous） | `devnode=blocked; pair_count=0` |
| stream-roundtrip | ok（**vacuous pass — 判定材料外**） | `test_exit=0; verdict=no-verdict-line` — endpoint 不在で test が early return（`finished in 0.00s`）。**round-trip の評価としては NotRun 相当** |
| cleanup | ok | |

**実装内容**: `CAdapterCommon` に `RVF_LOOPBACK_RENDER_SOURCE` registry を新設し、render stream が DMA buffer/format/run 状態/play cursor を publish、capture `WriteBytes` を `ToneGenerator.GenerateSine`→`RvfLoopbackCaptureWrite` へ置換（新規 `RvfLoopback.cpp/.h` — DISPATCH_LEVEL 対応の整数演算のみ変換: PCM16/24/32・f32・downmix・最近傍 rate map）。併せて `SetState(STOP)` で DMA 時刻基準・carry-forward を reset、`SetState(RUN)` の stamp を position lock 下へ移動（timestamp regressions 対応）。

**失敗分析**: worker ホストは WDK 非搭載で compile 不可のため、静的確認で捕捉できなかった警告を runner の `/WX` が拾った — `RvfLoopback.cpp:323` で `IsEqualGUIDAligned`（int 返却）を `BOOLEAN` へ代入し `C4242` → `C2220` error。修正: 三項で `? TRUE : FALSE` 化。compile error は本箇所のみ（他翻訳単位は通過）。

**次修正**: 上記 C4242 修正のみ — loopback 機構自体は未実走のため attempt-15 で driver-build→round-trip 全経路を再測定。

### attempt 15 — run 45512530945（branch `feat/977-vaudio-loopback`・SHA `34a30b92`・attempt15-* 同梱）— **loopback 実データ疎通**

| stage | status | detail |
| --- | --- | --- |
| 〜 endpoint-probe | ok | `pair_count=1` 再現 |
| stream-roundtrip | **error** | `test_exit=101`・`verdict: Fail` — ただし loopback が **実 PCM を運搬** |
| cleanup | ok | |

**実走観測**: `observed_rms=0.1449`（expected `0.1446`・ratio 1.002 — 刺激振幅そのもの）・`peak_bits=0x3f474a00`（≈0.78）・`non_finite=0`・`in=48k Mono F32 / out=48k Stereo F32`・`same_clock_domain=true`・`callbacks in=328 out=333`・`out frames/callback min=96 max=480`（可変長）。marker は検出されるようになったが間隔 `0.1104s`（100ms±10% の上限 110ms 超過）・tone share `0.085`（<0.25）。`out regressions=2` は attempt-13 と同数で loopback 非依存。

**失敗分析**: `RvfLoopbackCopyToCapture` が **fill 毎に `PlayBytePos` へ re-anchor** していたため、render/capture 両 stream の独立 position クロックの jitter が fill 境界の dup/skip として現れた。結果として (a) marker burst の実効間隔が伸び、(b) 不連続が帯域外ノイズとして 440Hz share を押し下げた。**次修正**: capture 側に連続読みカーソル（`ReadBytePos`/`ReadPosValid`）を持たせ、drift が半 buffer を超えた時のみ再 anchor。regressions は render stream の position reset（STOP）に発生源を特定 — render stream の `m_ullLinearPosition`/`m_ullPresentationPosition` を STOP を跨いで monotonic 維持へ。

### attempt 16 — run 45513922182（branch `feat/977-vaudio-loopback`・SHA `45b13a49`・attempt16-* 同梱）— **marker+tone oracle 通過**

| stage | status | detail |
| --- | --- | --- |
| 〜 endpoint-probe | ok | `pair_count=1` 再現 |
| stream-roundtrip | **error** | `test_exit=101`・`verdict: Fail` — 残 `output timestamp regressions: 2` のみ |
| cleanup | ok | |

**実走観測**: `oracle marker` 成立（mean interval が ±10% 内へ）・`oracle tone: 440Hz share of power=0.954`（≥0.25 — 連続カーソルで不連続 smear が消失）・`observed_rms` 正常・`non_finite=0`。失敗は `output timestamp regressions: 2` のみに集約。

**失敗分析**: `m_ullLinearPosition`/`m_ullPresentationPosition` の monotonic 化は **regressions に無影響**（依然ちょうど 2）→ cpal `output_timestamp()` が `timestamp.callback` に使う `qpc_position`（`IAudioClock::GetPosition` 直値）の extrapolation base は linear/presentation ではない。残る STOP reset 対象は `KSAUDIO_POSITION` の **`m_ullPlayPosition`/`m_ullWritePosition`（mod cursor）** — audiodg が PlayOffset unwrap で device position を追跡し、position epoch restart が `qpc_position` の backward extrapolation として regressions を生む仮説。**次修正**: render stream では全 position clock（Play/Write/Linear/Presentation）を STOP を跨いで monotonic 維持（packet counter・OS write position は protocol 契約なので reset 維持）。

### attempt 17 — run 45515209274（branch `feat/977-vaudio-loopback`・SHA `0e131c65`・attempt17-* 同梱）— mod cursor monotonic 化

| stage | status | detail |
| --- | --- | --- |
| 〜 endpoint-probe | ok | `pair_count=1` 再現 |
| stream-roundtrip | **error** | `verdict: Fail` — `output timestamp regressions: 2` のみ（attempt-16 と同一） |
| cleanup | ok | |

**実走観測**: `observed_rms=0.1444`（ratio 0.998）・`tone share=0.957`・marker 正常・`non_finite=0`・`errors out=0` — mod cursor の非 reset 化で新規 glitch/error は発生せず。regressions は依然 **ちょうど 2**。

**失敗分析**: PlayOffset/WriteOffset も monotonic にしても不変 → `qpc_position` の extrapolation base は mod cursor でもない。driver が報告する全 position field（4 種）は monotonic 化済みで不変のため、残る driver-reported 値は `GetPacketCount`（`m_llPacketCounter` — OS が packets×packet_size で device position を導出しうる）。**次修正**: render stream の packet counter も monotonic 維持（capture は GetReadPacket の epoch 依存計算があるため reset 維持）。

### attempt 18 — run 45516401796（branch `feat/977-vaudio-loopback`・SHA `44fd9020`・attempt18-* 同梱）— packet counter monotonic 化・発生源確定

| stage | status | detail |
| --- | --- | --- |
| 〜 endpoint-probe | ok | `pair_count=1` 再現 |
| stream-roundtrip | **error** | `verdict: Fail` — `output timestamp regressions: 2` のみ |
| cleanup | ok | |

**実走観測**: tone share・marker・rms・errors 全て attempt-16/17 と同値水準 — packet counter 非 reset 化も regressions に無影響（依然ちょうど 2）。

**発生源確定**: render stream の **driver が報告しうる全 position/counter field（PlayOffset・WriteOffset・LinearPosition・PresentationPosition・GetPacketCount）を STOP 跨ぎ monotonic 化しても件数不変** → `qpc_position` の backward extrapolation の base は driver-reported 値ではなく **audiodg（user-mode）自身の position epoch bookkeeping**（OS 自身が供給する packet index の restart — driver から制御不能）。非対称性の裏付け: input stream（同 lifecycle・同 `IAudioClock` API）は `GetReadPacket` が返す `PerformanceCounterValue`（**driver-supplied timestamp**）を使うため regressions=0 — output には該当 API が無く、audiodg が自身の extrapolation に頼る経路でだけ regression が出る。driver 側で打てる手は全て打ったため Coordinator に判断照会（verifier 側 tolerance 許容の要否含む）。
