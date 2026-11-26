# rf1009 xrun driver injection implementation evidence

## 対象と実施条件

- 対象 slice: driver injection implementation
- 基底 SHA: `0fb97d1ba89e1fee154cdbbb72ceb2db15fdcd8c`（PR #1026 merge 後の main）。本 evidence は、この SHA 上の本 slice working tree diff に対する実装検証である
- OS: Microsoft Windows 11 Pro 10.0.26300
- 実行日時: 2026-11-26 JST
- 実施者: Orca Worker `orca_session_id:fc8185a6-14f5-41a6-8467-fe0761f5e50d`
- 対象 endpoint/pin: Ruviform Cable Output を構成する WaveMicIn filter の capture streaming pin（`eMicInDevice` + `KSPIN_WAVEIN_HOST`、PinType `SystemCapturePin`）。設計文書の「loopback capture pin」は virtual-cable capture の意味に限定し、speaker filter の `RenderLoopbackPin` や host capture pin 全般へ拡張しない

この文書は driver implementation slice の code review evidence とローカル lane 記録である。formal proof や provisioned runtime evidence ではない。

## 実装内容の確認

- driver service `Parameters` key の `RvfXrunScenario` / `RvfXrunStartPacket` / `RvfXrunPacketCount` を、capture stream 作成時の passive な `CMiniportWaveRTStream::ReadRegistrySettings` で読む。scenario 値は stream object のメンバーに保持し、stream 中に書き換える KS property、device-interface control、IOCTL は追加していない
- 対象 pin は `eMicInDevice`、かつ `KSPIN_WAVEIN_HOST`、かつ `IsSystemCapturePin` の 3 条件で判定する。render pin、speaker filter の OS loopback pin、MIDI path、将来追加されうる他 capture pin には適用しない
- scenario `1`（capture-overflow-drop）では、notification completion index が `[RvfXrunStartPacket, RvfXrunStartPacket + RvfXrunPacketCount)` の completion で `KeSetEvent` だけを抑止する。`UpdatePosition`、loopback capture fill、position/packet counter、既存 ETW event、notification list は変更しない
- scenario `2`（capture-content-gap）では、notification・position・packet counter を変えず、指定 packet 区間に対応する capture DMA byte span だけを零 fill する。copy 済み領域を後からなぞって消す pass は追加していない

## ローカル検証

| コマンド | exit code | verdict | 記録 |
| --- | ---: | --- | --- |
| `mise run fmt-check` | 0 | Pass | `cargo fmt --all -- --check` が出力なく完了 |
| `mise run build-vaudio-driver` | 1 | NotRun | MSBuild `/c/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/MSBuild/Current/Bin/MSBuild.exe` は解決されたが、`WindowsKernelModeDriver10.0` platform toolset が無く MSB8020 で停止。driver build 環境が未装備なため製品欠陥の Fail ではなく環境前提未充足の NotRun と記録する。WDK/MSBuild は install していない |
| `mise exec -- prek run --all-files` | 0 | Pass | trailing whitespace、end of files、YAML/TOML、mixed line ending、UTF-8 BOM、private key、actionlint の各 hook が pass |

`build-vaudio-driver` の NotRun は build の代替 Pass ではない。driver binary の compile/link 検証、driver install/sign/provisioning、provisioned workflow 実走は後続 slice または WDK 装備環境が必要である。

## static RT invariant review

設計 D7 の 7 項目を実装後のコードに対して静的に確認した。確認は code review であり、formal proof ではない。

| D7 項目 | 確認結果 | コード上の根拠 | 残留リスク |
| --- | --- | --- | --- |
| 1. scenario parse/検証は stream 作成時だけ。RT path で registry key・文字列・可変長データを読まない | Pass（静的） | `ReadRegistrySettings` は passive/page code で `RvfXrun*` を direct/typecheck table から読む（`minwavertstream.cpp` 130-157）。既定値と fail-closed validation は stream `Init` 内で一度だけ実行される（同 198-218, 317-325）。RT の `TimerNotifyRT` / `WriteBytes` に registry API 呼出はない | registry 個別値の欠落と型違いを区別しないが、どちらも disabled へ正規化するため注入面の fail-closed 契約は保たれる |
| 2. RT の注入判定は integer comparison と整数 counter/state 更新だけ。floating point・allocation・logging文字列構築・追加の時間換算除算なし | Pass（静的） | notification 判定は completion index の比較と increment のみ（`minwavertstream.cpp` 2005-2020）。content gap 判定は zero start/end byte の比較と `chunk` の整数加算だけ（同 1703-1741）。zero byte boundary は passive な buffer allocation で計算する（同 661-668） | 既存 position/copy path には除算と整数 bit 演算が既にある。本 slice はそれを増やしていないが、既存 path 全体の性能証明は対象外 |
| 3. 新しい lock を作らず、stream position lock と adapter loopback lock の順序を逆転しない | Pass（静的） | notification/completion state は既存 `m_PositionSpinLock` 配下の `TimerNotifyRT` で更新する。content gap も `WriteBytes` 内で既存の `RvfLoopbackCaptureWrite` を呼ぶだけで、lock を追加していない。既存の stream position lock → adapter loopback lock 順序を変えていない | lock ordering は静的レビュー依存であり、WDK 装備環境での compile/run による追加確認は未実施 |
| 4. notification 抑止は `KeSetEvent` を行わない選択だけ。list 操作や event 登録状態を変えない | Pass（静的） | `eMINIPORT_BUFFER_COMPLETE` ETW event は抑止区間でも既存どおり書き、`KeSetEvent` だけを `!suppressNotification` 条件にした（`minwavertstream.cpp` 2077-2086）。notification list の走査・登録・解除コードは変更していない | OS/client がどの discontinuity flag を返すかは provisioned runtime evidence でしか確定しない |
| 5. content gap の零 fill は DPC/position update が本来書く bounded DMA span を超えない。追加 scan/copy pass を作らない | Pass（静的） | `WriteBytes` は既存の `ByteDisplacement` と DMA wrap の `runWrite` 内だけを処理し、injection boundary をまたぐ場合だけ chunk を分ける。各 chunk は 1 回だけ copy または zero fill し、合計 byte 数を増やさない（`minwavertstream.cpp` 1686-1746） | packet boundary を固定 linear byte span として解釈するため、実機の packet/position 相関との完全一致は provisioned sample oracle で確認する必要がある |
| 6. DISPATCH_LEVEL で passive-level API、file/registry I/O、pool allocation、floating point state を使わない | Pass（静的） | 追加した RT path の API は既存 `RvfLoopbackCaptureWrite` と `RtlZeroMemory`、整数 state 更新だけである。registry 読込は passive initialization、zero boundary 計算は passive buffer allocationにある。新規 allocation、file I/O、logging 文字列構築、浮動小数点状態は追加していない | binary にコンパイラが浮動小数点命令を生成しないことまでは build 未実行のため確認できない。既存 `RvfLoopback` の integer-only 設計は変更していない |
| 7. scenario 状態は immutable snapshot と monotonic counter/state。stream 中の書き換え経路を持たない | Pass（静的） | snapshot/state は stream object メンバーに閉じる（`minwavertstream.h` 147-154）。validation 後に scenario を書き換える公開経路はなく、completion index/write byte position は RUN 中に単調増加し、既存 capture position reset と同じ STOP 境界で reset される（`minwavertstream.cpp` 1300-1310） | static registry は同時に開かれた複数の対象 capture stream を個別制御できない。設計どおり provisioned lane は 1 stream 限定であり、複数同時制御は後続の private KS property 再評価である |

## fail-closed と境界の静的確認

### scenario validation

`RvfXrunScenario` が `1` または `2` のときだけ、`RvfXrunStartPacket <= 4096` かつ `1 <= RvfXrunPacketCount <= 16` を要求する。未知 scenario、範囲外、値を読めない場合、非対象 stream のいずれでも scenario/start/count を disabled 相当へ正規化し、stream 作成は失敗させない。registry への書き戻しはない（`minwavertstream.cpp` 198-218）。

### notification suppression 境界

completion index は 0 起点で、使用してから incrementするため、first completion は index 0 である。抑止区間は `[start, start + count)` で、start+count 以降の completion では `KeSetEvent` を再開する。counter は `bufferCompleted` かつ `KSSTATE_RUN` のときだけ進め、STOP で capture packet counter と一緒に reset する。pause 中は timer が止まり completion が発生しないため、pause 時間を index として数えない（`minwavertstream.cpp` 2003-2020, 2083-2086）。

### content zero-fill 境界

zero byte span は passive allocation 時に `start * packetBytes` から `(start + count) * packetBytes` までとして計算する。RT path は stream 先頭からの write byte position と比較し、boundary をまたぐ position update だけを最大でも boundary 前・区間内・boundary 後の chunk に分ける。区間外の PCM を zero fill しない構造であり、position・packet counter・notification には触れない（`minwavertstream.cpp` 661-668, 1676-1746）。

## 実施しなかったもの

- provisioned Windows runner 実走、run id・artifact・case result 採取: NotRun。後続の provisioned gate/evidence slice
- driver install/sign/provisioning、実 endpoint への scenario registry 投入と cleanup: NotRun。後続の provisioning/tool implementation slice
- positive/negative case の観測 counter と `rf1009-xrun-case/v1` case JSON artifact: NotRun。後続の harness implementation slice
- capability matrix/runbook の実測更新: NotRun。provisioned evidence 未取得のため本 slice では更新しない
- 実 GUI 検証: NotApplicable。本 slice は kernel driver implementation と headless な静的/ローカル lane 検証であり、GUI surface を変更していない
- output render underrun investigation/implementation: NotRun。本 slice の対象外であり、後続 slice で Unsupported のまま閉じるか観測経路を実装するかを決める
