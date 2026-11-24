# drivers/ — 独自 kernel driver ソースの置き場

Issue #961（`docs/designs/issue-952-virtual-endpoint-driver.md` §Slice B の前提調査）で新設。
`crates/**` は Rust workspace の領域のため、WDK build 対象の C/INF ソースは
このディレクトリに置く。配置は設計 doc の Deferred Design Concerns
「独自 driver の置き場所」に対する仮置き判断であり、恒久構成は Slice B 本体で確定する。

## testsigned-null/

GitHub-hosted runner（testsigning=Yes 出荷状態 — `evidence/rf952/` 実測）で
test-signed driver の install 可否を判定するための**最小 WDM null driver**。

- 機能を持たない — `ROOT\RvfTestSigned` hardware id の root-enumerated
  software device に function driver として結合し、PnP 列挙までを確認する。
  audio/MIDI endpoint 実装（Slice B 本体）は含まない。
- ソースのみを保持する。build 済みの `.sys`/`.cat`/`.cer`・`bin/`/`obj/`
  生成物は commit しない（このディレクトリの `.gitignore` を参照）。
- build は runner 上の MSBuild + WDK `WindowsKernelModeDriver10.0`
  platform toolset を前提とし、有無・job 内導入可否は
  `.github/workflows/virtual-driver-install-probe.yml` が実測する
  （worker ホストでの build は前提にしない）。

## rvf-virtual-audio/

Issue #977（Slice B stage 1）の仮想オーディオ endpoint driver。Microsoft
SysVAD サンプル（MS-PL）を render/capture 1 対に最小化した KMDF
port-class audio driver。render `Ruviform Cable Input` / capture
`Ruviform Cable Output` の MMDevice endpoint を生成する。

- 出所・vendoring 対応表・変更点は `rvf-virtual-audio/PROVENANCE.md`、
  build/install 手順の概要は `rvf-virtual-audio/README.md` を参照。
- ソースのみを保持する（`.sys`/`.cat`/`.cer`・`bin/`/`obj/` は commit
  しない — このディレクトリの `.gitignore` を参照）。
- build→test-sign→install→endpoint 列挙は手動 workflow
  `.github/workflows/virtual-audio-driver-probe.yml`（`workflow_dispatch`
  のみ、windows-2025/x64）が stage 化して実施する。
