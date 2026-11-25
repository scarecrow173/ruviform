# RvfVirtualAudio

Issue #977 用の仮想オーディオ endpoint driver + Issue #978 用の仮想 MIDI loopback miniport（MS-PL / MS-LPL 派生）。Microsoft SysVAD サンプルを ruviform 向けに最小化した KMDF port-class audio driver に、WDK 8.1 DMusUART サンプル由来の DMus MIDI miniport を同梱する。

- render endpoint: `Ruviform Cable Input`（アプリ側の再生先）
- capture endpoint: `Ruviform Cable Output`（アプリ側の録音元）
- audio loopback: render stream の DMA buffer を capture stream が kernel 内で読む render→capture 経路（Issue #977 stage 2 — rf977 attempt-21 で round-trip Pass）
- MIDI port（in/out 同名 pair）: `Ruviform MIDI Loopback`（WinMM midiIn/midiOut で同名列挙 — `tests/loopback.rs` の bidirectional name match に供する software render→capture loopback）
- hardware id: `ROOT\RvfVirtualAudio`（root-enumerated）
- 対象 arch: x64 のみ

出所・変更点の正本は `PROVENANCE.md`。ライセンスは audio 部が upstream MS-PL（`LICENSE-MS-PL.txt`）、MIDI miniport 部（`RvfMidi*.h/.cpp`）が upstream MS-LPL v1.1（`LICENSE-MS-LPL.txt`）。

## build

WDK + VS (WindowsKernelModeDriver10.0 ツールセット) が必要。CI workflow (`virtual-audio-driver-probe.yml`) では winget で WDK 10.0.26100 を job 内 install している。

```
msbuild RvfVirtualAudio.vcxproj /p:Configuration=Release /p:Platform=x64
```

生成物（`.sys` / `.cat` / `bin` / `obj` 等）はコミット対象外（`drivers/.gitignore` 参照）。

## install 手順（test-signing runner 想定）

1. `Inf2Cat` or `makecat` で `RvfVirtualAudio.cat` を生成
2. 自己署名証明書を作成し `LocalMachine\Root` / `TrustedPublisher` へ登録
3. `signtool sign` で `.cat`（必要なら `.sys` も）に署名
4. `pnputil /add-driver RvfVirtualAudio.inf /install` で driver store へ登録
5. `tools\testsigned-driver\TestDevice.ps1`（または同等の setupapi 経路）で `ROOT\RvfVirtualAudio` の root devnode を作成
6. `endpoint_probe`（`crates/ruviform-audio-device`）で `Ruviform Cable Input` / `Ruviform Cable Output` の pair 候補を、`midi_endpoint_probe`（`crates/ruviform-midi-device`）で WinMM port 列挙と loopback 候補を確認
7. `mise run test-audio-virtual-endpoint`（`cargo test -p ruviform-audio-device --features real-device --test virtual_endpoint -- --ignored`）で render→capture の audio round-trip を確認
8. `mise run test-midi-virtual-endpoint`（`cargo test -p ruviform-midi-device --test loopback -- --ignored`）で `Ruviform MIDI Loopback` の in/out pair 検出と round-trip を確認

手動ワークフロー `virtual-audio-driver-probe.yml` が上記を stage 化して実施する（`workflow_dispatch` のみ、windows-2025/x64）。手順詳細は `docs/specs/issue-629-provisioning-runbook.md` 追記節を参照。
