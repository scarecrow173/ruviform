# tools/testsigned-driver/ — test-signed driver install probe の補助スクリプト

Issue #961 専用。`.github/workflows/virtual-driver-install-probe.yml` の
runner job から呼ばれ、test-signed driver package（`drivers/testsigned-null/`）
の署名・store 登録・devnode 作成・install・列挙・cleanup を補助する。
`tools/` 直下への配置を避けてこの subdir に閉じる（rf#962 との共有面の分離）。

| ファイル | 役割 |
| --- | --- |
| `probe-stage.ps1` | workflow の各 step が dot-source する stage 記録ヘルパ。`Out-Probe`（生ログ tee）・`Add-StageRecord`/`Get-StageStatus`（stage↔status の JSON 記録 — `ok`/`absent`/`blocked`/`not_run`/`error`）・`Set-ProbeState`/`Get-ProbeState`（証明書 thumbprint・published INF 名・devnode instance id の stage 間受け渡し）・`Find-Tool`（PATH → `Windows Kits\10\bin` 探索）。 |
| `TestDevice.ps1` | root-enumerated devnode の作成・driver 結合・削除。runner に `devcon` が absent（`evidence/rf952/probe-env-*.txt` 実測）のため、`devcon install` 相当を setupapi/newdev の P/Invoke（`SetupDiCreateDeviceInfo` → `DIF_REGISTERDEVICE` → `newdev!UpdateDriverForPlugAndPlayDevices`）で実装する。`-Action remove` は `DiUninstallDevice` 経路。 |

## 実行前提

これらの script が署名・証明書 store への書き込み・driver install を行うのは
**GitHub-hosted runner の job 内だけ**。workflow は `workflow_dispatch`
専用で、testsigning=Yes・Secure Boot OFF は出荷状態として確定済みのため
`bcdedit` 設定や再起動は行わない（job 内 reboot は runner session を
kill する — `evidence/rf952/web-sources-signing-runner-wms.md` §2.3）。
worker ホストでの実行は禁止（ローカル検証は構文確認まで）。
