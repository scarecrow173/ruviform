# Gap Report — Issue #560 Luau.Analysis Native Bridge PoC

headless PoC スライスの実測結果を、契約被覆・PoC 実証・未実施に分類する。

## PoC-proven（本 worktree で実測 Pass）

| 項目 | 証拠 |
| --- | --- |
| Luau.Analysis (C++17) を FFI shim 経由で Rust から呼出 | `ffi-poc/native/rf_luau_bridge.{h,cpp}` + `src/ffi.rs`。正常/構文/型 Error・Completion 更新を構造化 JSON→contract 型で返す（22/22 tests Pass） |
| mlua Luau VM と Analysis の同一バイナリ・同一 revision リンク | `luau_vm_and_analysis_coexist_in_same_binary` Pass。両側 Luau 0.740（`revision-alignment.md`） |
| 版整合ネゴシエーション fail-closed | ABI/contract/struct_size 不一致 → status 8/非0 で拒否（unit tests Pass） |
| 不正 Cursor / Source Revision / UTF-8 / サイズ / Thread の明示 Error | 各 fixture Pass（status code 明示、panic/UB なし） |
| C++/VM/vendor 型の公開型への非漏出 | 公開 API は contract mirror 型 + `serde_json` のみ。C++ 型は shim 内に封じる |
| Analyzer 未登録 `Unavailable` / 未実装能力 `Unsupported` | FX-N-24/FX-N-13 相当 Pass |
| stale `source_revision` の Consumer 拒否 | FX-N-14 相当 Pass |
| Fault injection（init/analyze/loaddef 失敗） | status 7 明示エラー、解除後正常復帰 Pass |
| 解析成功 ≠ 実行 Grant | `ScriptAnalyzer` を runtime backend から独立した Port として実装し、型・テストで分離 |
| License/互換性台帳・版整合記録 | `license-ledger.md` / `revision-alignment.md` |

## Contract-covered（契約が意味論を規定、本 PoC はミラー実装で実測）

| 項目 | 契約根拠 |
| --- | --- |
| `ScriptAnalyzer`/`AnalyzerRegistry`/`AnalyzerCapabilities`/`AnalyzeRequest`/`AnalyzeResult`/`Diagnostic`/`Completion`/`AnalyzerError` の語彙 | `docs/designs/issue-558-script-contract-design.md` C5、`docs/specs/issue-558-script-contract.md` |
| `source_revision` 保持・stale は Consumer 拒否 | 同上 |
| `impl_version`/`contract_version` 分離・不一致 fail-closed | `ScriptBackendVersion` 相当 |
| diagnostics の `Unsupported|Syntax|SyntaxAndType` 段階 | `AnalyzerCapabilities` |

## NotRun / Deferred（本スライス対象外 — 正直な記録）

| 項目 | 理由 | 引き継ぎ先 |
| --- | --- | --- |
| macOS Build/Link/Run | 実機・toolchain なし | 統合 Gate で実機検証 |
| Linux Build/Link/Run | toolchain なし（`check-linux` は対象外 crate 限定） | 同上 |
| real-OS 3OS 配布物サイズ・性能・Memory 差分 | 未実施（debug `.lib` のみ計測） | #558/#559 統合 Gate |
| 製品 Binary への正式統合 | 本 PoC は隔離 crate、`publish=false` | #558/#559/#294 |
| Consumer（synvelis Script Editor 等）受入 | consumer repo は read-only 参照のみ | synvelis 側 Issue |
| release ビルドでのサイズ計測 | 未実施（debug 計測のみ） | 後続 Gate |
| ASan/valgrind 等のメモリ検査 | 未実施（所有権はコードレビュー+Drop 経路で担保） | 後続 Gate（任意） |
| `module_time_limit_sec` の決定的 fixture | shim は対応するが timeout 再現 fixture は未整備 | 必要なら後続 |
| build-script 非 UTF-8 stdout（mbx cache bypass） | cl.exe の CP932 出力が原因と推定、未対処 | 別 Issue 検討（本 PoC 範囲外） |

## #558/#559 への引き継ぎ（技術採否材料）

- **採用可否（技術面）**: vendored Luau.Analysis + 狭小 C ABI + JSON
  境界という構成で、外部 luau-lsp/system install なしに in-process
  解析が実現できることを Windows で実証した。mlua VM との同一
  revision 共存・静的リンク共存も実測 Pass。
- **前提条件**: mlua-sys/luau0-src の版が更新されるたびに Analysis
  vendored revision と build.rs の lib 名再 emit を再照合する必要が
  ある（`revision-alignment.md` の 0.744→0.740 経緯を参照）。
- **未実施 OS の別受入 Owner/条件**: macOS/Linux の Build/Link/Run・
  配布計測は統合 Gate の責務として Deferred。Windows のみの PoC Pass
  をもって 3OS 対応を宣言しない。
- **製品配布の別受入**: `ffi-poc/` は隔離 PoC であり本番公開 crate
  化は別判断。本番化する場合は unsafe 境界の監査・release サイズ
  計測・3OS 検証が前提。
