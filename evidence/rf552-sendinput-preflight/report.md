# RF552 — SendInput preflight 実測証拠レポート（Windows 実 OS・3 lane）

Issue #552「real-OS input lane SendInput preflight」の実測証拠。
`os_input::preflight` が lane 本体の前に注入の実効を独立 OS API の
read-back で検証し、3 lane すべてで preflight 通過後に本体が
`pass` したことを verdict JSON で確認した。

## 判定

| 対象 | 判定 | 根拠 |
| --- | --- | --- |
| keyboard lane preflight（`real_os_smoke`、VK_F24 → `GetAsyncKeyState`） | **Pass** | `verdict-keyboard-windows.json`: verdict=pass、preflight.effective=true（"VK_F24 down confirmed by read-back, released"）、exit code 0 |
| pointer lane preflight（`real_os_pointer_smoke`、絶対 move → `GetCursorPos`） | **Pass** | `verdict-pointer-windows.json`: verdict=pass、preflight.effective=true（cursor (2701,346) → (2717,362) を read-back で確認）、exit code 0 |
| multiwindow lane preflight（`real_os_multiwindow_smoke`、pointer+keyboard） | **Pass** | `verdict-multiwindow-windows.json`: verdict=pass、両 check effective、exit code 0 |
| preflight 不通 → `environment_blocked` / exit 3 の実環境発生 | **NotRun** | 本環境では SendInput は正常動作しており自然発生せず。blocked 分岐は mock Probe ユニットテスト（`os_input::tests` 8 件）で検証済み。Task Spec の指示どおり障害の強制注入は行わない |
| macOS / Linux | **Unsupported** | `os_input` は Windows のみ実装（既存方針どおり `UiError::Unsupported` → verdict `unsupported` / exit 2） |

## 実行環境

- OS: Windows（対話 desktop session、Orca IDE 稼働中の実機）
- toolchain: repo pin `rust = "1.99.0"`（mise 管理）
- 実行方法: `cargo run -p ruviform-ui-gpui --features gpui --example <lane> -- --out evidence/rf552-sendinput-preflight/<verdict>.json`（補助証拠・canonical lane entry は `mise run example-ui-gpui-real-os-*-smoke` だが `--out` が rf313 証拠 dir 固定のため、本 Issue の証拠 dir へ直接出力した）

## verdict スキーマ（v1 → v2）

- `schema` を `rf313-real-os-smoke/v2`、
  `rf313-real-os-pointer-smoke/v2`、
  `rf243-real-os-multiwindow-smoke/v2` に更新。
- 全 verdict に `"preflight"` オブジェクトを追加
  （`{"effective": bool, "checks": [{"family", "effective", "detail"}]}`）。
  unsupported 時は `"preflight": null`。
- `verdict` 文字列に `environment_blocked` を追加（既存
  `pass`/`fail`/`unsupported` と衝突しない新値）。lane 本体の
  `pass`/`fail` 意味は不変。
- exit code: `0` pass / `1` fail / `2` unsupported /
  `3` environment_blocked（preflight 不通・lane 本体未実行）。

## preflight の実効確認内容

- **pointer**: カーソル原点を `GetCursorPos` で読み、virtual
  screen 内側へ絶対移動 16px（`MOUSEEVENTF_ABSOLUTE|VIRTUALDESK`）
  → `GetCursorPos` を 25ms 間隔で poll し ±8px 以内を確認 →
  成否に関わらず原点へ復帰。実測 detail:
  `cursor (2701, 346) -> (2717, 362) confirmed by read-back
  Some((2716, 361))`（正規化量子化の 1px ずれは tolerance 内）。
- **keyboard**: `VK_F24`（0x87、未割当・修飾なし）を key-down
  注入 → `GetAsyncKeyState` high bit を poll → key-up を必ず送信。
  実測 detail: `VK_F24 down confirmed by read-back, released`。
- read-back は注入呼び出し（`SendInput`）の戻り値とは独立した
  OS API であり、「受理されたが実効ゼロ」の障害を検出できる。

## 関連証拠

- 障害の先行観測:
  `evidence/issue-243-multiwindow-verify/environment-blocked-sendinput.md`
  （SendInput が sent=1/2 を返しつつカーソル移動・キー注入ともに
  実効ゼロ。mouse_event/keybd_event/SetCursorPos は有効だった）。
- 本 preflight はその証拠の推奨（`SendInput → GetCursorPos`
  preflight）を keyboard/pointer 双方に拡張したもの。
