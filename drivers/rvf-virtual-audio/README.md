# RvfVirtualAudio

Issue #977 用の仮想オーディオ endpoint driver（MS-PL 派生）。Microsoft SysVAD サンプルを ruviform 向けに最小化した KMDF port-class audio driver。

- render endpoint: `Ruviform Cable Input`（アプリ側の再生先）
- capture endpoint: `Ruviform Cable Output`（アプリ側の録音元）
- hardware id: `ROOT\RvfVirtualAudio`（root-enumerated）
- 対象 arch: x64 のみ

出所・変更点の正本は `PROVENANCE.md`。ライセンスは upstream MS-PL（`LICENSE-MS-PL.txt`）。

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
6. `endpoint_probe`（`crates/ruviform-audio-device`）で `Ruviform Cable Input` / `Ruviform Cable Output` の pair 候補を確認

手動ワークフロー `virtual-audio-driver-probe.yml` が上記を stage 化して実施する（`workflow_dispatch` のみ、windows-2025/x64）。
