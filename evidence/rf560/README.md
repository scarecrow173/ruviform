# evidence/rf560 — Issue #560 Luau.Analysis Native Bridge PoC 証拠

Issue #560 `[test/script][RF-SCRIPT-02]` の headless PoC スライスの証拠
成果物。`ffi-poc/` 隔離 crate による Luau.Analysis ネイティブブリッジ・
版整合・fault fixture の実測記録をまとめる。

**本スライスは技術 PoC のみ。real-OS 3OS 配布実行・製品 Binary 正式
統合・Consumer 受入は統合 Gate（#558/#559/#294・synvelis 側）の責務で
あり、本証拠の Pass を 3OS 正式配布 Pass に読み替えない。**

## ファイル

| ファイル | 内容 |
| --- | --- |
| `gap-report.md` | PoC-proven / contract-covered / NotRun の分類と #558/#559 への引き継ぎ事項 |
| `license-ledger.md` | Luau.Analysis vendored source の Revision・License・toolchain・サイズ台帳 |
| `revision-alignment.md` | mlua vendored Luau VM と Analysis の版整合照合記録 |
| `build-link-run.md` | Windows での Build/Link/Run 記録（コマンド・toolchain・結果・既知の問題） |
| `platform-matrix.md` | OS/Build/Link/Run の実測マトリクス（未実機は NotRun） |

## 対象 SHA

| repo | SHA | 備考 |
| --- | --- | --- |
| scarecrow173/ruviform | `07db76a4826874b29c68a12475c8cd7217c14d93` | 本 worktree の merge 基点 HEAD（作業中に `origin/main` 取り込みで更新される場合あり） |

## 収集方法

- Build/Link/Run: `mise run check|lint|test ruviform-ffi-poc` + 補助的な
  `cargo test -p ruviform-ffi-poc`（canonical = mise、cargo 直接実行は
  補助証拠）。
- 版整合: `Cargo.lock` の `luau0-src`/`mlua-sys`/`mlua` 解決版、
  crates.io の yank 状態、vendored `REVISION` manifest の照合。
- License: vendored `ffi-poc/vendor/luau/LICENSE.txt` /
  `lua_LICENSE.txt` の内容確認と upstream tag 照合。
- 未実施 OS の Build/Link/Run・配布サイズ/性能/memory 実測は本環境に
  macOS/Linux toolchain がないため NotRun（`platform-matrix.md` 参照）。
