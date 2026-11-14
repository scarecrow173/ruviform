# 版整合照合記録 — Luau.Analysis ↔ mlua vendored Luau VM（Issue #560）

## 結論

PoC では **Luau 0.740 に両側を揃えた**。Analysis（vendored）と
mlua-sys 経由の VM（luau0-src）が同一 upstream revision を指すことを
Cargo.lock・yank 状態・vendored manifest の 3 系統で照合した。

## 照合表

| 系統 | 値 | 根拠 |
| --- | --- | --- |
| Analysis vendored | Luau `0.740` / commit `c0e346edd89066b44dca174c9f54ce84c746a540` | `ffi-poc/vendor/luau/REVISION`、upstream tag `0.740` |
| shim 報告 version | `"0.740"`（`rf_luau_bridge.cpp` `kLuauVersion`） | `rf_luau_bridge_luau_version()` の返り値をテスト `bridge_versions_report_luau_revision` で実測 |
| VM（mlua） | `luau0-src 0.22.0+luau740`（= Luau 0.740） | `Cargo.lock`、`mlua 0.12.2 → mlua-sys 0.13.0 → luau0-src` |
| ABI version | `RF_LUAU_BRIDGE_ABI_VERSION = 1` | `rf_luau_bridge.h` / create 時ネゴシエーション |
| contract version | `RF_LUAU_BRIDGE_CONTRACT_VERSION = 1` | 同上。不一致は create 時に status 8 で fail-closed |

## 0.744 → 0.740 への切替経緯（重要な判断記録）

1. 当初は mlua が Luau 0.744 を vendored すると想定し Analysis 0.744 で
   vendor を開始した。
2. `Cargo.lock` 解決の結果、`mlua-sys 0.13.0` の build-dependency は
   `luau0-src 0.22.0+luau740` だった。
3. crates.io で `luau0-src 0.22.1+luau744` は **yanked**（公開後に
   取り下げ）。mlua 0.12.2 の semver 範囲では 0.740 側しか解決できない。
4. 版整合を優先し、Analysis 側を upstream tag `0.740`
   （commit `c0e346edd89066b44dca174c9f54ce84c746a540`）で再 vendor した。
   shim が参照する Analysis API（`Frontend::check`、`autocomplete`、
   `FileResolver`、`ConfigResolver`、`registerBuiltinGlobals`、
   `freeze`/`unfreeze`、`TypeError`、`AutocompleteEntry`）は 0.740 で
   全て存在することを vendored header で確認済み。
5. 結果として VM/Analysis は同一 Luau revision 0.740 で一致。
   **将来 mlua/luau0-src が新版（0.745+）へ更新された場合は、Analysis
   vendored revision の再照合が必須**（build.rs の lib 名再 emit も
   mlua-sys 生成物に依存する PoC 固定値であり、更新時に要再確認）。

## 版ネゴシエーション fail-closed fixture

| fixture | 内容 | 結果 |
| --- | --- | --- |
| ABI version 不一致 | `abi_version=0xFFFF` を注入して `rf_luau_analyzer_create` を直接呼出 | status 8 `RF_LUAU_ERR_VERSION`、handle 非生成（`ffi::create_negotiation_tests::abi_version_mismatch_fails_closed` Pass） |
| struct_size 不一致 | 縮小 `repr(C)` 構造体を渡す | 非 0 status、handle 非生成（`struct_size_mismatch_rejected` Pass） |
| contract version | `LuauAnalyzer::connect` が `rf_luau_bridge_contract_version()` を照合 | 正常系は 1=1 で一致。shim 側 contract version getter 実測値を `version_info_is_exposed_for_negotiation_evidence` で記録 |

## impl_version / contract_version 分離

契約（RF-SCRIPT-00）の `ScriptBackendVersion` に倣い、PoC では
`ScriptBackendVersion { impl_version: "luau-0.740", contract_version: 1 }`
を descriptor として公開する（`ffi-poc/src/contract.rs` /
`analyzer.rs`）。impl_version（Luau revision）と contract_version
（bridge 契約版）を分離して保持し、Analyzer 未登録時の
`AnalyzerError::Unavailable`（FX-N-24 相当）と fail-closed 解決を
`AnalyzerRegistry` で実装した。
