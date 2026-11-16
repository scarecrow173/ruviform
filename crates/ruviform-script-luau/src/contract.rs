//! FFI shim のステータスコード定数。
//! `native/rf_luau_bridge.h` の `#define RF_LUAU_*` と 1:1 で対応する
//! （ABI/JSON schema は ffi-poc RF-SCRIPT-02 から不変で移植）。

/// shim ABI version。`native/rf_luau_bridge.h` の
/// `RF_LUAU_BRIDGE_ABI_VERSION` と同期。
pub(crate) const ABI_VERSION: u32 = 1;

/// shim が実装する Script/Analyzer 契約 version。
/// `native/rf_luau_bridge.h` の `RF_LUAU_BRIDGE_CONTRACT_VERSION` と
/// 同期し、`ruviform_script::CONTRACT_VERSION` と一致必須
/// （`LuauAnalyzer::connect` で fail-closed 検査）。
pub(crate) const SHIM_CONTRACT_VERSION: u32 = 1;

pub(crate) const RF_LUAU_OK: i32 = 0;
pub(crate) const RF_LUAU_ERR_INVALID_UTF8: i32 = 1;
pub(crate) const RF_LUAU_ERR_CURSOR_RANGE: i32 = 2;
pub(crate) const RF_LUAU_ERR_SOURCE_TOO_LARGE: i32 = 3;
pub(crate) const RF_LUAU_ERR_THREAD_MISMATCH: i32 = 4;
pub(crate) const RF_LUAU_ERR_COMPLETION_UNSUPPORTED: i32 = 5;
/// native 内部失敗（spec の status 6 — 専用 message "shim internal error"）。
pub(crate) const RF_LUAU_ERR_INTERNAL: i32 = 6;
// status 7 = INJECTED（test-only fault injection seam `RF_LUAU_FAIL_*`）
// — `AnalyzerError::Internal` の wildcard arm が処理するため個別 const なし。
pub(crate) const RF_LUAU_ERR_VERSION: i32 = 8;
