---
id: "DESIGN-034QqV7xR1009ZkLmNu"
type: "design"
status: "draft"
title: "仮想 endpoint の決定的 xrun 注入と adapter 観測契約（Issue #1009）"
created: "2026-11-26"
updated: "2026-11-26"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/1009"
  changes:
    added:
      - type: "file"
        path: "docs/designs/issue-1009-virtual-xrun-injection.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/designs/issue-952-virtual-endpoint-driver.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "POLICIES.md"
    - "docs/designs/issue-565-wasapi-input-frame-position-design.md"
    - "docs/designs/issue-629-provisioning-runbook.md"
    - "docs/designs/issue-952-virtual-endpoint-driver.md"
  refines:
    - "docs/designs/issue-565-wasapi-input-frame-position-design.md"
  refined-by: []
  derives-from:
    - "docs/designs/issue-952-virtual-endpoint-driver.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "POLICIES.md"
  defers: []
  deferred-by: []
---

# 仮想 endpoint の決定的 xrun 注入と adapter 観測契約（Issue #1009）

## Context

Issue #1009 は、RvfVirtualAudio virtual endpoint に xrun/drop/gap を決定的に注入し、adapter が観測できる oracle を定義する。本書は設計 slice であり、driver・adapter・test harness・workflow の実装は行わない。確定するのは oracle の種類、方向別の観測契約、注入制御面、realtime path の制約、後続実装 slice の境界と provisioned evidence schema である。

設計入力は 2026-11-26 時点の `ab6dc99fe8935ac09082d05e2176b10e729ba263` で確認した。rf#1002 run `45575005948` では、active stream に対する bounded user-mode CPU starvation が callback 継続を観測しながら `xruns=0`、`discontinuities=0`、`invalidated=0`、`errors=0` に留まり、`xrun_probe` は `Unsupported` と記録された。この結果は、user-mode 側の負荷だけでは型付き xrun evidence を作れないことを示す。本書ではこの negative result を再検証せず、driver 側で WaveRT packet/notification semantics を制御する注入面を設計する。

## 確定事項

| ID | 決定 | 結論 |
| --- | --- | --- |
| D1 | 必須 Pass oracle | input capture overflow では typed `DeviceError::Xrun` を必須 Pass oracle にする。`ClockDiscontinuity`・timestamp/sample discontinuity・stall は補助 oracle であり、typed Xrun の代替にしない |
| D2 | oracle 合成 | capture overflow case の Pass は、`Xrun >= 1`、前方 `frame_position` discontinuity に伴う `ClockDiscontinuity >= 1`、注入後も callback が継続すること、unexpected error がないことを同時に要求する |
| D3 | 方向別契約 | `DeviceError::Xrun` の型は方向共通だが、観測・推論契約は方向別にする。input capture overflow を第一実装範囲とし、output render underrun は adapter 観測経路がないため Unsupported/後続 slice に分ける |
| D4 | 注入制御面 | 第一実装は static registry scenario を stream 作成時に snapshot する方式にする。private KS property set と device-interface control は不採用とし、動的制御が必要になった場合だけ後続 slice で再評価する |
| D5 | 注入方法 | capture overflow は notification completion のうち指定区間だけ client notification を抑止し、position/packet counter は通常どおり進める。capture content gap は negative control として PCM だけを無音化し、notification/position を変えない |
| D6 | adapter mapping | WASAPI native input は、連続した capture epoch での前方 `frame_position` gap を capture overrun/loss として `DeviceError::Xrun` に写像し、同じ境界を `ClockDiscontinuity` としても通知する。後方 jump・stop/start reset・flag だけの不連続は `ClockDiscontinuity` に留める |
| D7 | realtime 制約 | RT path での注入判定は integer-only・allocation-free・O(1) とする。registry I/O、動的メモリ確保、floating point、passive-level API、stream 生成後の scenario 再読込を RT path に入れない |
| D8 | 責務分担 | driver は fault の発生だけを担い、adapter は backend 観測事実の normalization だけを担い、test harness は oracle 判定と JSON 出力、workflow/provisioning tool は scenario の投入・除去と artifact 収集を担う |
| D9 | 実装範囲 | 本書は docs-only。driver/adapter/test/tool/workflow の変更、provisioned 実走、capability matrix/runbook の実測更新は後続 implementation/gate slice の exit condition とする |

## 現行入力

### adapter の観測面

`DeviceError` は `Copy` で allocation-free の public error enumであり、`Xrun` と `ClockDiscontinuity` を独立した variant として持つ（`crates/ruviform-audio-core/src/device.rs:1096-1159`）。cpal error normalization は `cpal::ErrorKind::Xrun` を error-callback path でだけ `DeviceError::Xrun` に写像し、control path の `Xrun` は `Backend("other")` に落とす（`crates/ruviform-audio-device/src/error.rs:46-68`）。

Windows/WASAPI input は cpal の portable input path ではなく native input pathを使う。cpal が packet device positionを保持しないためで、native path は `IAudioCaptureClient::GetBuffer` が返す packet 先頭の device positionを `InputCallbackInfo::frame_position` に入れる（`crates/ruviform-audio-device/src/device.rs:135-146`、`crates/ruviform-audio-device/src/wasapi_input.rs:39-48`）。`PositionTracker` は `device_position == previous + frames` の連続性を検査し、`AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY` または contiguity breakを `DeviceError::ClockDiscontinuity` として通知してから packet を配送する（`wasapi_input.rs:679-710`、`:899-919`）。

cpal 0.18.2 の WASAPI input は `AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY` を `ErrorKind::Xrun` に写像する経路を持つ（cpal `src/host/wasapi/stream.rs:820-826`）。しかし現行 adapter は WASAPI input でその cpal pathを使わない。output では cpal 0.18.2 が buffer取得・callback・`ReleaseBuffer` を行うだけで、underrun を検査して error callbackへ `ErrorKind::Xrun` を送る経路を持たない（cpal `src/host/wasapi/stream.rs:873-910`）。ruviform の output open も cpal output pathを使う（`crates/ruviform-audio-device/src/device.rs:224-285`）。この非対称が、方向別契約を必要とする直接的な根拠である。

### driver の注入面

RvfVirtualAudio は SysVAD 由来の WaveRT miniport driverである。`CMiniportWaveRTStream` は high-resolution timer の DPC で `UpdatePosition` を呼び、notification intervalごとに `m_llPacketCounter` を進め、登録済み notification eventへ `KeSetEvent` を送る（`drivers/rvf-virtual-audio/EndpointsCommon/minwavertstream.cpp:1823-1935`）。`GetReadPacket` は最新の completed packet番号を返し、OS が読んだ前回 packetとの差を `droppedPackets` として計算するが、現状は trace コメントだけで upper layerへ明示通知しない（同ファイル `:943-1040`）。

render-to-capture loopback は `CAdapterCommon` の registryに render DMA bufferを登録し、capture fillが `ReadBytePos` を連続的に進めて render bufferから読む。変換は integer-onlyで、DISPATCH_LEVELでも floating point stateを使わない（`drivers/rvf-virtual-audio/RvfLoopback.h:11-22`、`:81-106`、`drivers/rvf-virtual-audio/common.cpp:5159-5280`）。driver は `Parameters` registry keyから起動時/初期化時設定を読む既存経路を持つ（`drivers/rvf-virtual-audio/adapter.cpp:407-497`）。stream object も capture 初期化時に registry 設定を読むため、stream 作成を再読込境界にできる（`minwavertstream.cpp:130` の `ReadRegistrySettings` と `:361` の呼び出し）。

### workflow と evidence の境界

manual provisioned workflow `.github/workflows/virtual-audio-driver-probe.yml` は driver build/provision後に streaming/MIDI/e2e/fault stageを実行し、case 単位の結果を収集する。`tools/virtual-audio-driver/Invoke-FaultCases.ps1` は device-loss case で controller 側が stimulus を投入し、その成否と test process の observation を分けて記録している（同ファイル `:337-388`、`:521-544`）。xrun でも同じ分離が必要である。controller が「注入を設定した」という記録は、adapter が「xrunを観測した」という証拠の代替にしない。

## oracle taxonomy と判定規則

### oracle の意味

| oracle | 観測単位 | 保証できること | 保証できないこと | 判定上の地位 |
| --- | --- | --- | --- | --- |
| typed `DeviceError::Xrun` | input stream の error callback | capture 方向で frame loss/overrun に対応する型付き eventが consumer 契約へ到達したこと | underrun/overrun の原因が driver 注入だったこと、その backend の内部実装 | 必須 Pass oracle |
| `DeviceError::ClockDiscontinuity` | input stream の error callback | timestamp/frame positionを境界前後で joinしてはいけないこと | それだけでは xrun の種別や原因 | 必須の補助 oracle |
| frame position discontinuity | `InputCallbackInfo::frame_position` の連続値 | capture epoch内で前方 gapが生じたこと | sample 内容そのものの欠損範囲 | Xrun 判定の根拠。case JSONへ必須記録 |
| timestamp discontinuity | capture timestamp/callback timestamp の系列 | clock/position境界の時間的な影響 | xrun の有無。timer schedulingや shared clock補間だけでも変化し得る | 補助記録。単独では Pass/Failを決めない |
| sample discontinuity | stimulus marker/PCM系列 | 実際に配送されたPCM内容の欠損や無音区間 | packet protocol上の xrun。無音化は xrunではない | negative control と診断用。typed Xrun の代替にしない |
| stall | callback間隔・event待ち時間 | callbackが一時停止したこと | overrun/underrunの型や原因。rf#1002では starvationだけでは xrun evidenceにならなかった | 補助記録。単独では Passにしない |

### Pass/Fail/NotRun/Unsupported

`capture-overflow-drop-input` caseの Pass 条件は次のとおりである。

1. controller が scenario を stream 作成前に書き込み、適用対象 stream の作成後に callbacks が注入前に進んでいる。
2. 注入測定窓で input error callbackに `DeviceError::Xrun` が1回以上現れる。
3. 同じ注入区間をまたいで前方 `frame_position` gapが記録され、`DeviceError::ClockDiscontinuity` が1回以上現れる。
4. 注入後も input callbackが再開し、`StreamInvalidated` や分類不能な `Backend(_)` errorで stream が終了しない。
5. case JSON と raw logに scenario・run provenance・counterが欠けず残る。

Fail は、stimulus が適用され harness が健康であるのに必須 oracleを満たさない場合、または negative controlで typed Xrun/discontinuityを誤って出した場合である。NotRun は provisioning、endpoint、driver、registry write、stream開始など case実行の前提が成立しなかった場合である。Unsupported は、契約上その direction/backendが観測できないと宣言されている caseである。output render underrun は第一実装でこの区分に置く。exit code 0や workflow jobの成功だけを Passとみなさない。

### negative control

`capture-content-gap-input-negative` caseは、同じ packet数の区間で capture PCMだけを無音化し、notification・position・packet番号を通常どおり進める。この caseの Pass は、sample gapを検出しつつ `Xrun=0` かつ `ClockDiscontinuity=0` である。無音や sample不連続だけを xrunと誤認する adapter mappingを検出するために設ける。逆に、この caseだけでは capture overflow の正の証拠にならない。

## 方向別契約

input capture overflow では、backendが配送する capture sequenceの前方欠損を overrun/lossと解釈できる。frame positionが `previous + frames` を超えて前方へ進み、stream が同じ epochで継続する場合、途中の capture frameは consumerへ配送されなかった。この観測は、注入が packet notificationを抑止して作る missing packet windowと一致する。したがって input では typed `DeviceError::Xrun` を必須 oracleにできる。

output render underrun では事情が異なる。driver内部には render write positionが更新されない場合の simple underrun detectionと ETW glitch reportがある（`minwavertstream.cpp:1898-1911`）。しかし cpal 0.18.2 WASAPI outputには underrunを adapter error callbackへ届ける発火経路がなく、ruviform outputは cpal output pathを使っている。loopback capture上の無音や callback timingの乱れは output underrunの同義語ではない。driver内部 ETW traceも adapter observationではない。このため output render underrun を inputと同じ推論規則で扱う設計は採用しない。

結論として、`DeviceError::Xrun` の variantは方向共通のままにするが、観測契約は方向別に定義する。input では adapterが frame positionと packet flagから判定する。output では、backendか adapter output pathが underrunの観測事実を受け取る契約を実装するまで Unsupportedとする。対称性を欠く理由は実装の難易度ではなく、cpal/WASAPI output経路に観測可能な入力が存在しないことである。

## adapter mapping の差分

`docs/designs/issue-565-wasapi-input-frame-position-design.md` は、cpal が discontinuity flagを `Xrun` に写すのに対し、ruviform portは flagと position jumpを `ClockDiscontinuity` へ寄せる設計を採った（同文書 line 81-82）。本書はその区別を残したまま一部を refineする。flagだけの不連続、後方 jump、resetは従来どおり `ClockDiscontinuity` であり、typed Xrunと等価ではない。新たに `Xrun` へ写すのは、同じ capture epochで `frame_position` が前方へ抜ける場合だけである。

| 観測事実 | 現行 mapping（`ab6dc99f`） | 設計後 mapping | 理由 |
| --- | --- | --- | --- |
| cpal error-callback `ErrorKind::Xrun` | `DeviceError::Xrun` | 変更なし | 既存 normalizationを維持する |
| cpal control-path `ErrorKind::Xrun` | `Backend("other")` | 変更なし | control pathで発火しない前提を崩さない |
| WASAPI native inputの `AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY` のみ | `ClockDiscontinuity` | `ClockDiscontinuity` | flagだけでは forward frame lossを確認できない。cpal portable inputとは精度を分ける |
| WASAPI native inputで forward `frame_position` gap | `ClockDiscontinuity` | `Xrun` と `ClockDiscontinuity` をこの順で通知し、packetは実 positionで配送する | 同じ epochで capture frameが抜けたことは input overrun/lossを示す。epoch境界をjoinしてはいけない事実も残る |
| WASAPI native inputで backward jumpまたは stop/start reset | `ClockDiscontinuity` | `ClockDiscontinuity` のみ | 位置系の再 anchorであり、capture overrunとは限らない |
| WASAPI output underrun | 観測経路なし | 第一実装では Unsupported。観測経路実装後に `Xrun` 契約を後続 sliceで定義する | cpal 0.18.2 outputに発火経路がない |
| capture PCM無音/sample gap | errorなし | errorなし。negative controlでは誤検出として扱う | 内容の欠損と packet protocol上の overrunを混同しない |

この差分は `ClockDiscontinuity` を typed Xrunと等価に扱うものではない。`ClockDiscontinuity` は後方 jumpや resetでも発生しうる一般の境界通知であり、typed Xrun は forward capture lossに限定する。`Xrun` と `ClockDiscontinuity` の両方が発生するのは、lossが起きたうえで timestamp/positionの epoch境界もjoin不能になるためである。

実装上は `PositionTracker` が booleanだけでなく mismatchの方向を返す必要がある。error callbackの順序は `Xrun`、必要なら `ClockDiscontinuity`、その後に packet配送とする。callback上で allocationや文字列構築を行わない現行 `DeviceError` の制約は変えない。

## driver 注入制御面

### 比較と採否

| 方式 | 決定性 | 再読込境界 | stream単位制御 | 実装リスク | 採否 |
| --- | --- | --- | --- | --- | --- |
| static registry scenario | stream作成時に snapshotすれば deterministic | stream close/open | capture pin種別で限定し、stream instanceごとに snapshot。同時に複数の該当 capture streamを開く環境では個別指定できない | 低い。既存 registry読込経路に近い | 採用 |
| private KS property set | stream中に切り替えられる | property SET時点。atomic snapshotが必要 | pin/filter handle単位にできる | 中。property table、user-mode側の pin解決、検証、遷移同期が増える | 第一実装では不採用 |
| device-interface control | test processが任意時刻に制御できる | IOCTL受信時点 | device interface/session単位にできる | 高い。新しいIOCTL ABI、ACL、lifetime、並行性、user-mode clientが必要 | 不採用 |

第一実装に必要なのは、stream開始からの packet indexを使って一度だけ注入を起こすことであり、stream中の動的 arm/disarmではない。static registry scenarioでこの要件を満たせるため、KS propertyやdevice-interface controlを先に導入する理由はない。複数 endpointの同時制御、1 stream内での複数 scenario、consumer appが介入しない長時間 fault campaignが必要になった場合は、後続 sliceで private KS property setを先に再評価する。

### scenario contract

registry値は driver service `Parameters` keyに置き、対象 capture stream objectが作成時に snapshotする。値の存在しない既定状態は無効である。driverは registryへ書き戻さない。

| 値 | 型 | 意味 |
| --- | --- | --- |
| `RvfXrunScenario` | `REG_DWORD` | `0` disabled、`1` capture-overflow-drop、`2` capture-content-gap。それ以外は無効 |
| `RvfXrunStartPacket` | `REG_DWORD` | `KSSTATE_RUN` 後の notification completion indexを0起点で数え、注入開始位置とする。上限は `4096` |
| `RvfXrunPacketCount` | `REG_DWORD` | 注入対象の連続 packet数。`1..16` |

無効値、範囲外、未知の scenarioは partial injectionを行わず、その streamでは disabledに正規化する。controller側も同じ範囲を事前検証し、範囲外なら書き込まず caseを NotRunにする。registryを直接編集された場合に driverが stream作成を失敗させる必要はない。注入しないことが fail-closedであり、観測側が誤って Passになることを防ぐ。

第一実装の適用対象は Ruviform Cable Output側の loopback capture pinに限定する。host capture pinや render pinへ無差別に適用しない。ここでいう stream単位とは、stream objectが生成時に独立した immutable snapshotを持つ意味である。provisioned laneが同時に開く対象 capture streamを1本に限定するため、endpoint instanceの一意な識別までは第一実装の要件にしない。この制限は evidence schemaの `endpoint.target_pin` に明記する。

実装メモ: 現行 driver では、Ruviform Cable Output を構成する capture streaming pin は WaveMicIn filter の `KSPIN_WAVEIN_HOST`（`eMicInDevice`、PinType `SystemCapturePin`）である。この設計でいう loopback capture pin は、speaker filter の `RenderLoopbackPin` ではなく、この virtual-cable capture pin を指す。したがって driver injection implementation は `eMicInDevice` かつ `KSPIN_WAVEIN_HOST` の streamにだけ適用し、PinType `SystemCapturePin` 全般や将来の host capture pinへ拡張しない。

controller は streamを開く前に値を書き、case終了後に `finally` 相当で値を消す。再読込境界は driver service再起動ではなく stream close/openである。workflowは scenario write、stream open、observation、cleanupの順序を raw logへ残す。

## WaveRT packet/notification semantics 上の注入方法

### capture-overflow-drop

capture streamの DPCは注入区間でも `UpdatePosition`、loopback capture fill、position/packet counter更新を通常どおり行う。変えるのは notification eventへの `KeSetEvent` だけで、`RvfXrunStartPacket` から `RvfXrunPacketCount` 個の notification completionでは client notificationを抑止する。抑止後の次の completionで notificationを再開する。対象は WaveRT event-driven capture streamであり、notificationを使わない capture modeではこの scenarioを Unsupportedとして扱う。

この方法では linear/presentation positionと packet counterの相関を人為的に壊さない。OSは抑止中に read packetを進められず、再開時には最新 completed packetだけを受け取るため、中間 packetを失う。driverが `m_llPacketCounter` を余分に跳ね上げる必要はなく、timestamp correlationにも反しない。adapterから見える evidenceは、callback stall後の forward frame position gap、必要に応じた WASAPI discontinuity flag、typed Xrun、ClockDiscontinuityである。driver内部の ETW/traceは診断用であり、この adapter evidenceと混同しない。

### capture-content-gap

negative controlでは notification・position・packet counterを変えず、指定 packet区間だけ capture DMAへ書く PCMを無音化する。loopback copyを通さずに零 fillしてもよいが、copy回数や処理時間を増やしてはいけない。sample gapは作れるが、packet protocol上の欠損はないため、adapterは `Xrun` も `ClockDiscontinuity` も出してはいけない。

### render underrun

render underrunを driver内部で疑似的に起こす方法として、write position更新を staleとみなす、notification timingを遅らせる、loopback sourceを止める、といった候補がある。しかし第一実装でこれらを採用しない。driver内部で glitchを起こしても cpal output経路に観測 eventが届かず、loopback PCMや callback timingだけでは output xrunを型付きで主張できないためである。後続 sliceでは、cpal outputの replacement/native output、cpal upstream契約、WASAPI低水準観測、driverが render glitchを OS/adapterへ表現できるかを分けて調査する。

## realtime path の invariant

注入機構は次を invariantとして実装する。

1. scenarioの parseと検証は stream作成時に一度だけ行い、RT pathでは registry key、文字列、可変長データを読まない。
2. RT pathの注入判定は integer comparisonと整数 counter更新だけにする。floating point、除算を伴う時間換算の追加、allocation、logging文字列の構築を行わない。
3. DPC/spinlock内では、既存の position更新と同じ lock順序だけを使う。新しい lockを導入せず、stream position lockと adapter loopback lockの順序を逆転させない。
4. notification抑止は `KeSetEvent` を行わない選択だけであり、list操作や event登録状態を変更しない。
5. content gapの零 fillは、その DPCが本来書く bounded DMA spanを超えない。注入機構が追加の scanや copy passを作らない。
6. DISPATCH_LEVELで passive-level API、file I/O、registry I/O、pool allocation、floating point stateを使わない。
7. scenario状態は stream object内の immutable snapshotと monotonic counterで表し、stream中の書き換え経路を持たない。

これらは driver実装 sliceの code review項目であり、provisioned evidenceの Pass条件とは別に確認する。

## 責務分担

| layer | 担うこと | 担わないこと |
| --- | --- | --- |
| driver (`drivers/rvf-virtual-audio`) | scenario snapshot、notification抑止、content zero-fill、必要最小限の内部 trace | oracle判定、Pass/Fail決定、user-mode artifact形式 |
| adapter (`crates/ruviform-audio-device`) | WASAPI packet position/flagの検査、`DeviceError` normalization、counter観測に必要な public契約の維持 | scenarioの解釈、driver制御、evidence artifact生成 |
| test harness (`crates/ruviform-audio-device/tests` / `crates/ruviform-audio-test`) | stream開始、callback/error/frame position/timestamp/sample観測、oracle合成、case JSON出力 | driver registry値の投入、driver内部動作の仮定による shortcut判定 |
| provisioning/tool (`tools/virtual-audio-driver`) | registry値の validation/write/cleanup、driver provisioning、endpoint選択、raw log収集 | adapter oracleの再実装、exit 0を evidence扱いすること |
| workflow (`.github/workflows`) | provisioned runner上での stage順序、artifact upload、run id/SHAの provenance | 個別 caseの oracle合成 |
| docs/evidence | 判定語彙、schema、runbook/capability matrixの実測値 | 未実施を Passとして記録すること |

## 後続実装 slice のファイル境界

本 design sliceでは次のファイルを変更しない。後続 sliceの提案境界であり、Coordinator/Issue分割で調整してよい。

| slice | 変更候補 | exit condition |
| --- | --- | --- |
| driver injection implementation | `drivers/rvf-virtual-audio/adapter.cpp`、`EndpointsCommon/minwavertstream.cpp/.h`、必要なら `RvfLoopback.cpp/.h` | static registry scenarioの snapshot、drop/gap注入、RT invariantを実装し、driver buildと static検査を通す |
| adapter observation implementation | `crates/ruviform-audio-device/src/wasapi_input.rs`、`src/error.rs`、必要なら `src/device.rs` と unit test | forward gapを `Xrun`+`ClockDiscontinuity`へ写像し、reset/backward gapや content-only gapを誤認しない unit/integration testが通る |
| harness implementation | `crates/ruviform-audio-device/tests/virtual_endpoint.rs`、必要なら `crates/ruviform-audio-test` | positive/negative caseの観測 counterと JSON lineを出力し、headless/non-provisioned環境では NotRun/Unsupportedを返す |
| provisioning/tool implementation | `tools/virtual-audio-driver/Invoke-FaultCases.ps1` または新規 helper | scenario write/cleanup、case JSON集約、controller stimulusログを実装する |
| workflow implementation | `.github/workflows/virtual-audio-driver-probe.yml` | xrun stageで positive/negative caseを実行し、artifactとして case JSON/raw log/provisioning logを uploadする |
| provisioned gate/evidence | `evidence/rf1009/`、capability matrix/runbookの該当行 | provisioned Windows実走で run id/artifact/case resultを取得し、matrixを Pass/Fail/NotRun/Unsupportedで更新する |
| output investigation/implementation | adapter output契約、必要なら native output path、driver render glitch表現 | output render underrunを Unsupportedのまま閉じるか、観測経路付きで別 Issue/sliceへ移すかを決める |

## provisioned evidence schema/artifact 要件

case JSONの schema名は `rf1009-xrun-case/v1` とする。必須 fieldは次のとおりである。

```json
{
  "schema": "rf1009-xrun-case/v1",
  "run": {
    "id": "required",
    "attempt": "required",
    "repo": "required",
    "sha": "required",
    "os": "required",
    "recorded_utc": "required"
  },
  "case": {
    "name": "capture-overflow-drop-input",
    "direction": "input",
    "result": "Pass|Fail|NotRun|Unsupported",
    "detail": "required non-empty string"
  },
  "endpoint": {
    "render_name": "required when used",
    "capture_name": "required",
    "capture_instance_id": "required",
    "target_pin": "loopback-capture"
  },
  "injection": {
    "control_surface": "static-registry-stream-snapshot",
    "scenario": "capture-overflow-drop",
    "start_packet": 20,
    "packet_count": 3,
    "applied": true,
    "cleanup": "cleared|clear_failed|not_applicable",
    "reload_boundary": "stream-open"
  },
  "oracle": {
    "typed_xruns": 0,
    "clock_discontinuities": 0,
    "stream_invalidated": 0,
    "backend_errors": 0,
    "input_callbacks": 0,
    "output_callbacks": 0,
    "forward_frame_position_gaps": 0,
    "max_forward_gap_frames": 0,
    "sample_gap_detected": false,
    "max_callback_stall_ms": 0
  },
  "artifacts": {
    "raw_stdout": "required",
    "raw_stderr": "required",
    "controller_log": "required",
    "driver_trace": "optional",
    "pcm_capture": "optional"
  }
}
```

必須 counterは `typed_xruns`、`clock_discontinuities`、`input_callbacks`、`backend_errors`、forward position gapである。timestamp discontinuity、sample gap、stall、ETW/driver trace、PCM captureは診断用の任意または補助 fieldであり、typed Xrunの代替にしない。`output_callbacks` は input caseでも loopback stimulusの健康確認に使うため、render streamを使う caseでは必須とする。artifactは case JSON、test stdout/stderr、controller log、provisioning log、endpoint probe outputを最低限含める。run id、attempt、merge対象 SHA、artifact URL/名前を capability matrixへ転記できる形にする。

## 検証 lane の区分

本 docs-only sliceの正規検証は `mise run fmt-check`、`mise run test-ci-daily`、`mise run test-ci-tracker`、`mise exec -- prek run --all-files` である。これらは設計文書の Markdown/frontmatter/構成と CI helper契約が壊れていないことを確認する軽量検査であり、driver注入や adapter観測の証拠ではない。

implementation sliceでは、少なくとも対象 crateに対する `mise run check ruviform-audio-device`、`mise run lint ruviform-audio-device`、`mise run test ruviform-audio-device`、driver build lane、provisioned manual workflowが必要になる。capability matrixを更新する gate sliceでは、mainではなく provisioned artifactに紐づく run idと case JSONを証拠にする。provisioned実走を行わない限り、rf#1009の runtime acceptanceを Passと記録してはいけない。

## Issue 受入条件との対応

| 受入条件 | 本 sliceの結果 |
| --- | --- |
| oracle種類と現行 adapter mapping差分 | 本書で設計確定。typed Xrun必須、補助 oracleの地位、mapping差分を定義した |
| driver注入面と adapter観測契約の責務分担 | 本書で設計確定。static registry stream snapshot採用、責務表を定義した |
| 仮想endpointで決定的注入を観測した provisioned実走 evidence | 後続 implementation/gate slice。本 sliceでは採取していないため Passとしない |
| realtime pathが allocation-free/bounded/no-floating-point | 設計 invariantとして確定。実装証拠は後続 slice |
| capability matrix/runbookの xrun行を実測更新 | 後続 provisioned gate slice。本 sliceでは更新しない |
| `mise run` の対象 lane pass | 本 slice指定の軽量 laneは PRで実測記録。implementation laneは後続 slice |

## 残リスク

第一に、notification抑止が Windows OS側で常に `AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY` を立てるかは、現行 code readingだけでは確定しない。設計上の typed Xrun判定は flagに依存せず forward frame position gapを根拠にするため、この不確実性は Pass oracleを崩さない。ただし positionが欠けたり backward jumpとして現れたりする場合は adapter実装 sliceで Failとして扱う。

第二に、static registry scenarioは同時に開かれた複数の該当 capture streamを個別制御できない。provisioned laneでは対象 streamを1本に制限する。複数同時制御や長時間 campaignが必要になれば、private KS property setの再評価を後続 sliceの exit conditionとする。

第三に、output render underrunは現行 adapter経路で観測できない。loopback PCMの変化や driver ETWだけを根拠に typed output xrunを主張してはいけない。outputを対象に入れる場合は、adapterが受け取れる backend observability契約を先に設計する。
