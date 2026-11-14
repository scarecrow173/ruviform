---
id: "EXP-rf-script-02-luau-bridge-poc"
title: "Issue #560 Luau.Analysis Native Bridge PoC — 検証計画と実測経緯"
created: "2026-11-14"
updated: "2026-11-14"
---

# Issue #560 Luau.Analysis Native Bridge PoC — 検証と実測経緯(2026-11-14)

- 正本 IR: `docs/impl/ir/rf-script-02-luau-bridge-poc.md`
- 契約正本: RF-SCRIPT-00（`docs/specs/issue-558-script-contract.md`、
  `docs/designs/issue-558-script-contract-design.md`）
- 証拠: `evidence/rf560/`（gap-report / license-ledger /
  revision-alignment / build-link-run / platform-matrix）

## 1. 検証計画（headless PoC スライス）

| # | 検証項目 | 方法 |
| --- | --- | --- |
| V1 | Analysis を FFI shim 経由で呼出、構造化 diagnostics/completion を返す | `tests/bridge.rs` 正常/構文/型/completion fixture |
| V2 | mlua VM と Analysis の同一バイナリ・同一 revision 共存 | `luau_vm_and_analysis_coexist_in_same_binary` + 版照合 |
| V3 | version/struct_size/cursor/UTF-8/size/thread の fail-closed | `ffi_boundary_tests.rs` + `ffi.rs` negotiation tests |
| V4 | fault injection（init/analyze/loaddef） | `tests/faults.rs` 環境変数シーム |
| V5 | 契約語彙（Unavailable/Unsupported/StaleResult/revision 分離） | `tests/bridge.rs` FX-N-13/14/24・F-18 相当 |
| V6 | canonical 検証 | `mise run check|lint|test ruviform-ffi-poc` + `fmt-check` |

対象外（統合 Gate へ Deferred）: real-OS 3OS 配布 Build/Link/Run・
配布サイズ/性能/Memory 実測、製品 Binary 統合、Consumer 受入。

## 2. 実測結果サマリ

| 検証 | 結果 |
| --- | --- |
| V1 | **Pass** — 13 integration tests 全 Pass（構文/型 error の構造化 Diagnostic、型定義追加による completion 候補変化を実測） |
| V2 | **Pass** — Luau 0.740 に両側統一（`revision-alignment.md`）。mlua での VM 実行と Analysis が同一 test binary で動作 |
| V3 | **Pass** — ABI/contract/struct_size 不一致は create 時 status 8/非0、cursor/UTF-8/size/thread は明示 status code |
| V4 | **Pass** — `RF_FFI_POC_FAIL_{INIT,ANALYZE,LOADDEF}` で status 7、解除後に正常復帰 |
| V5 | **Pass** — `Unavailable`/`Unsupported{completion}`/`StaleResult`/revision 保持を契約ミラー型で実測 |
| V6 | **Pass** — `cargo check`/`clippy -D warnings`/`cargo test`（22/22）/`cargo fmt --check` 全て clean（Windows, MSVC） |

## 3. 実験で判明した重要事項（試行錯誤の記録）

### 3.1 Luau revision の不一致と解消（0.744 → 0.740）

- 当初 mlua が Luau 0.744 を vendored すると想定して Analysis 0.744 で
  vendor 開始 → `Cargo.lock` は `luau0-src 0.22.0+luau740` を解決。
- `luau0-src 0.22.1+luau744` は crates.io で **yanked** と判明し、
  mlua 0.12.2 の VM は 0.740 固定と確定。
- Analysis を upstream tag `0.740`
  （`c0e346edd89066b44dca174c9f54ce84c746a540`）で再 vendor。
  shim が使う API は 0.740 で全て存在を header で確認済み。
- **教訓**: 版整合は「想定」ではなく `Cargo.lock` 解決結果と
  registry の yank 状態を正本として照合する必要がある。

### 3.2 MSVC 環境と `/utf-8`

- `cl.exe` が `stddef.h` 不在で失敗 → vcvars 未ロード環境と判明。
  build.rs が `find-msvc-tools` で環境を補う方式を採用
  （edition 2024 の `env::set_var` unsafe 規約に SAFETY コメント付き適合）。
- shim の日本語コメント（UTF-8）が CP932 既定で C1020 を起こす
  → MSVC target に `/utf-8` を付与して解消。

### 3.3 静的リンク順序（118 未解決シンボル）

- `rf_luau_bridge.lib` が mlua-sys の `luau*.lib`（9 本）より後に
  配置され、MSVC の左→右走査で未解決に。
- build.rs が同名 lib を bridge 直後に再 emit して解消。
  lib 名は mlua-sys 0.13.0/luau0-src 0.22.0+luau740 の生成物に対応する
  PoC 固定値（更新時要再照合 — `build-link-run.md`）。

### 3.4 Windows の env 変数二系統問題（fault fixture の発見）

- Rust `env::set_var`/`remove_var` は Win32 PEB env block を更新するが、
  C++ `std::getenv` が読む CRT `_environ` は起動時 snapshot のまま。
- fault injection が in-process で観測不能になることを fixture が
  検出 → shim の `envFlag` を Windows では `GetEnvironmentVariableA`
  で読むよう修正（非 Windows は `std::getenv`）。
- **教訓**: Rust↔C++ を跨ぐ環境変数 seam では、読む側の API が
  更新側と同じ env store を参照するかを検証する必要がある。

### 3.5 clippy pedantic 対応

- `unsafe_code` は `#[expect]`+SAFETY コメントの局所許可で隔離。
- `struct_field_names`（`impl_version`/`contract_version`）は契約
  語彙のミラーのため scoped `#[allow]`+理由コメント。
- JSON 解析は `parse_analyze_json` が 115 行超過 →
  `json_u32`/`position_from_json`/`range_from_json`/
  `diagnostic_from_json`/`completion_from_json` に分割し、
  cast/redundant_closure 系 lint も同時に解消。

## 4. 未実施・残件

- macOS/Linux の Build/Link/Run・配布計測: NotRun（toolchain/実機なし）。
- release ビルドのサイズ計測: 未実施（debug `.lib` 約 377 MB は
  debug info 込みで配布見積に使わない）。
- ASan/valgrind 相当のメモリ検査: 未実施（所有権はレビュー+Drop 経路）。
- `module_time_limit_sec` の決定的 timeout fixture: 未整備。
- build-script 非 UTF-8 stdout（mbx cache bypass、cl.exe の CP932
  出力が原因と推定）: 未対処 — 別 Issue 検討対象として記録。
