# Platform Matrix — Issue #560 Luau.Analysis Native Bridge PoC

POLICIES.md の Pass/Fail/NotRun/Unsupported 規範に従い、実測した OS のみ
Pass とし、未実機は NotRun とする。**PoC Pass を 3OS 正式配布 Pass に
読み替えない。**

| OS | Build | Link | Run（test） | 配布サイズ | 性能/Memory | 判定 |
| --- | --- | --- | --- | --- | --- | --- |
| Windows x86_64（MSVC, Build Tools 2022） | Pass | Pass | Pass（22/22 tests） | debug のみ計測（release 未計測） | 未計測 | **PoC Pass（headless 範囲）** |
| macOS（Apple Silicon / Intel） | NotRun | NotRun | NotRun | NotRun | NotRun | toolchain・実機なし |
| Linux（x86_64） | NotRun | NotRun | NotRun | NotRun | NotRun | toolchain なし（`check-linux` は io-file/io-archive 限定で本 crate 対象外） |

## 補足

- Windows 実測環境: CP932 ロケールの日本語 Windows、MSVC cl.exe、
  Rust は mise 管理 toolchain。`/utf-8` と MSVC env 自動設定が
  必須条件として判明（`build-link-run.md`）。
- macOS/Linux での Build/Link/Run・配布物計測・性能/Memory 差分は
  **本スライス対象外の Deferred**。統合 Gate（#558/#559/#294）で
  別途実機検証が必要。
- 「対応未宣言」: 本 PoC は Windows でのみ動作を実測しており、
  macOS/Linux の「対応」を宣言しない。
