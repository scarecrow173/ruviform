//! FFI Shim への extern "C" バインディング。unsafe はこのモジュールに隔離し、
//! 各 unsafe fn は局所許可（`#[expect(unsafe_code)]`）+ SAFETY コメントを持つ。
//! 公開 API から C++/Luau 型・ポインタは一切漏出しない。
//!
//! ffi-poc (RF-SCRIPT-02) `src/ffi.rs` の製品化移植。差分は
//! `AnalyzerError` が `ruviform_script` の中立契約型である点と、
//! status→error 写像が契約 variant に揃う点のみ。ABI/JSON schema は不変。

use std::ffi::{CStr, CString, c_char, c_void};
use std::ptr::NonNull;

use ruviform_script::{AnalyzerError, CapabilityId};

use crate::contract::{
    ABI_VERSION, RF_LUAU_ERR_COMPLETION_UNSUPPORTED, RF_LUAU_ERR_CURSOR_RANGE,
    RF_LUAU_ERR_INTERNAL, RF_LUAU_ERR_INVALID_UTF8, RF_LUAU_ERR_SOURCE_TOO_LARGE,
    RF_LUAU_ERR_THREAD_MISMATCH, RF_LUAU_ERR_VERSION, RF_LUAU_OK, SHIM_CONTRACT_VERSION,
};

#[repr(C)]
struct RfLuauCreateOptions {
    struct_size: u32,
    abi_version: u32,
    contract_version: u32,
    mode_strict: i32,
    enable_completion: i32,
    max_source_bytes: u32,
    module_time_limit_sec: f64,
    source_type_module: i32,
}

#[expect(unsafe_code)]
// SAFETY: extern 宣言は shim (native/rf_luau_bridge.cpp) の定義と 1:1 対応。
// unsafe はこのモジュールに隔離され、呼出し側は全て SAFETY コメント付き。
unsafe extern "C" {
    fn rf_luau_bridge_impl_version() -> *const c_char;
    fn rf_luau_bridge_contract_version() -> u32;
    fn rf_luau_bridge_luau_version() -> *const c_char;
    fn rf_luau_analyzer_create(
        options: *const RfLuauCreateOptions,
        out: *mut *mut c_void,
        out_error: *mut *mut c_char,
    ) -> i32;
    fn rf_luau_analyzer_destroy(analyzer: *mut c_void);
    fn rf_luau_analyzer_load_definition(
        analyzer: *mut c_void,
        package_name: *const c_char,
        definition_source: *const c_char,
        out_error: *mut *mut c_char,
    ) -> i32;
    fn rf_luau_analyzer_analyze(
        analyzer: *mut c_void,
        source: *const c_char,
        source_len: usize,
        cursor_offset: i64,
        json_out: *mut *mut c_char,
        out_error: *mut *mut c_char,
    ) -> i32;
    fn rf_luau_free(ptr: *mut c_void);
}

/// shim が返した UTF-8 文字列。Drop で `rf_luau_free` を呼ぶ。
struct ShimString(NonNull<c_char>);

impl ShimString {
    /// SAFETY: ptr は shim が `new char[]` で確保した NUL 終端文字列。
    /// null の場合は None。
    #[expect(unsafe_code)]
    // SAFETY: 呼び出し側は shim が返した非 null の `new char[]` ポインタを渡す。
    unsafe fn from_raw(ptr: *mut c_char) -> Option<Self> {
        NonNull::new(ptr).map(Self)
    }

    #[expect(unsafe_code)]
    fn as_str(&self) -> &str {
        // SAFETY: ShimString は常に NUL 終端された有効な UTF-8 を保持する
        // （shim の copyString が保証）。
        unsafe { CStr::from_ptr(self.0.as_ptr()) }
            .to_str()
            .unwrap_or("")
    }
}

impl Drop for ShimString {
    #[expect(unsafe_code)]
    fn drop(&mut self) {
        // SAFETY: self.0 は shim が確保したポインタであり、rf_luau_free が
        // 対応する delete[] を行う。一度だけ解放される。
        unsafe { rf_luau_free(self.0.as_ptr().cast::<c_void>()) }
    }
}

/// shim のバージョン情報（create 前に取得可能）。
// フィールド名は契約 `ScriptBackendVersion` の impl_version/contract_version
// 語彙と shim 報告の luau_version をそのまま保持するため postfix 共通は意図的。
#[allow(clippy::struct_field_names)]
pub(crate) struct BridgeVersions {
    pub impl_version: String,
    pub contract_version: u32,
    pub luau_version: String,
}

/// SAFETY: 3 関数とも引数なしで静的文字列/数値を返す。
#[expect(unsafe_code)]
// SAFETY: shim のバージョン getter は副作用がなく静的データを返す。
pub(crate) fn bridge_versions() -> BridgeVersions {
    unsafe {
        BridgeVersions {
            impl_version: CStr::from_ptr(rf_luau_bridge_impl_version())
                .to_string_lossy()
                .into_owned(),
            contract_version: rf_luau_bridge_contract_version(),
            luau_version: CStr::from_ptr(rf_luau_bridge_luau_version())
                .to_string_lossy()
                .into_owned(),
        }
    }
}

/// Analyzer 生成オプション（shim ABI に対応する内部表現。
/// 公開 `LuauAnalyzerOptions` の `max_source_bytes: u64` は connect 時に
/// u32 へ検証変換される）。
#[derive(Debug, Clone)]
pub(crate) struct CreateOptions {
    pub mode_strict: bool,
    pub enable_completion: bool,
    pub max_source_bytes: u32,
    pub module_time_limit_sec: f64,
    pub source_type_module: bool,
}

/// FFI ハンドル。Drop で shim の destroy を呼ぶ。
/// `NonNull` 由来で `Send`/`Sync` は自動的に非実装 — shim の
/// owner-thread affinity を型で正直に表現する（`unsafe impl` 不使用）。
pub(crate) struct BridgeHandle {
    pub(crate) ptr: NonNull<c_void>,
}

impl BridgeHandle {
    /// ABI/契約 version をネゴシエートして shim Analyzer を生成する。
    /// 不一致は `AnalyzerError::VersionMismatch` で fail-closed。
    #[expect(unsafe_code)]
    // SAFETY: options は本関数内で構築した有効な構造体、out/out_error は
    // 有効なポインタ。返却ハンドルは destroy まで shim 側で有効。
    pub(crate) fn create(opts: &CreateOptions) -> Result<Self, AnalyzerError> {
        let options = RfLuauCreateOptions {
            struct_size: u32::try_from(size_of::<RfLuauCreateOptions>()).unwrap_or(u32::MAX),
            abi_version: ABI_VERSION,
            contract_version: SHIM_CONTRACT_VERSION,
            mode_strict: i32::from(opts.mode_strict),
            enable_completion: i32::from(opts.enable_completion),
            max_source_bytes: opts.max_source_bytes,
            module_time_limit_sec: opts.module_time_limit_sec,
            source_type_module: i32::from(opts.source_type_module),
        };
        let mut handle: *mut c_void = std::ptr::null_mut();
        let mut err: *mut c_char = std::ptr::null_mut();
        // SAFETY: 上記の通り。返却ポインタは null または有効。
        let status =
            unsafe { rf_luau_analyzer_create(&raw const options, &raw mut handle, &raw mut err) };
        let err_msg = unsafe { ShimString::from_raw(err) };
        match status {
            RF_LUAU_OK => {
                // SAFETY: RF_LUAU_OK 時に handle は非 null の有効ポインタ。
                // NonNull::new は unsafe ではない。
                let ptr = NonNull::new(handle).ok_or_else(|| AnalyzerError::Internal {
                    message: "shim returned null handle".to_string(),
                })?;
                Ok(Self { ptr })
            }
            RF_LUAU_ERR_VERSION => {
                let err_text = err_msg.as_ref().map_or("", ShimString::as_str);
                // shim のエラーメッセージから実測値を拾えないため、
                // 期待値と定数を記録する。
                Err(AnalyzerError::VersionMismatch {
                    field: "abi/contract".to_string(),
                    expected: format!("abi={ABI_VERSION} contract={SHIM_CONTRACT_VERSION}"),
                    actual: err_text.to_string(),
                })
            }
            other => Err(bridge_error(other, err_msg.as_ref())),
        }
    }

    /// 型定義を shim のグローバル環境へ登録する。
    #[expect(unsafe_code)]
    // SAFETY: self.ptr は有効な Analyzer ハンドル。文字列は NUL 終端
    // CString で、呼出し中のみ有効。
    pub(crate) fn load_definition(
        &self,
        package_name: &str,
        definition_source: &str,
    ) -> Result<(), AnalyzerError> {
        let pkg = CString::new(package_name).map_err(|_| AnalyzerError::InvalidRequest {
            reason: "package_name contains NUL".to_string(),
        })?;
        let def = CString::new(definition_source).map_err(|_| AnalyzerError::InvalidRequest {
            reason: "definition_source contains NUL".to_string(),
        })?;
        let mut err: *mut c_char = std::ptr::null_mut();
        // SAFETY: ptr は有効、pkg/def は NUL 終端で生存期間は呼出し中。
        let status = unsafe {
            rf_luau_analyzer_load_definition(
                self.ptr.as_ptr(),
                pkg.as_ptr(),
                def.as_ptr(),
                &raw mut err,
            )
        };
        let err_msg = unsafe { ShimString::from_raw(err) };
        if status == RF_LUAU_OK {
            Ok(())
        } else {
            Err(bridge_error(status, err_msg.as_ref()))
        }
    }

    /// ソースを解析し `JSON` 文字列を返す。`cursor_offset` < 0 で Completion なし。
    #[expect(unsafe_code)]
    // SAFETY: self.ptr は有効。source は len バイトの有効な読み取り領域
    // （NUL 終端不要）。返却 json_out は rf_luau_free 対象として ShimString が
    // 所有する。
    pub(crate) fn analyze(
        &self,
        source: &[u8],
        cursor_offset: i64,
    ) -> Result<String, AnalyzerError> {
        let mut json: *mut c_char = std::ptr::null_mut();
        let mut err: *mut c_char = std::ptr::null_mut();
        // SAFETY: source.as_ptr() は source_len バイト有効。len=0 の場合も
        // shim は source_len のみ参照し data() を読まない（isValidUtf8 が
        // len=0 で即 true、std::string(data,0) も安全）。
        let status = unsafe {
            rf_luau_analyzer_analyze(
                self.ptr.as_ptr(),
                source.as_ptr().cast::<c_char>(),
                source.len(),
                cursor_offset,
                &raw mut json,
                &raw mut err,
            )
        };
        let json_str = unsafe { ShimString::from_raw(json) };
        let err_msg = unsafe { ShimString::from_raw(err) };
        match status {
            RF_LUAU_OK => Ok(json_str
                .as_ref()
                .map_or_else(String::new, |s| s.as_str().to_string())),
            other => Err(bridge_error(other, err_msg.as_ref())),
        }
    }
}

impl Drop for BridgeHandle {
    #[expect(unsafe_code)]
    fn drop(&mut self) {
        // SAFETY: ptr は create で得た有効ハンドル。destroy は一度だけ呼ぶ。
        unsafe { rf_luau_analyzer_destroy(self.ptr.as_ptr()) }
    }
}

/// shim ステータスコードを中立契約 `AnalyzerError` へ写像する。
/// C++/VM の型や生ポインタはここで文字列/コードのみに変換される。
///
/// 写像規則（docs/specs/issue-692-luau-backend.md の表どおり）:
/// - 1 `INVALID_UTF8` / 2 `CURSOR_RANGE` / 3 `SOURCE_TOO_LARGE` /
///   4 `THREAD_MISMATCH` → `InvalidRequest`（要求・使用文脈の不正。
///   `THREAD_MISMATCH` は safe code では到達不能だが shim 契約の
///   defense-in-depth として維持）
/// - 5 `COMPLETION_UNSUPPORTED` → `Unsupported{analyze.completion}`
/// - 8 `VERSION` → `VersionMismatch`（create 経路でも個別処理済み）
/// - その他（6 `INTERNAL` / 7 `INJECTED` / 未知コード）→ `Internal`
///   （status code を message に保持して診断可能にする）
fn bridge_error(status: i32, message: Option<&ShimString>) -> AnalyzerError {
    let msg = message.map_or_else(String::new, |s| s.as_str().to_string());
    match status {
        RF_LUAU_ERR_INVALID_UTF8
        | RF_LUAU_ERR_CURSOR_RANGE
        | RF_LUAU_ERR_SOURCE_TOO_LARGE
        | RF_LUAU_ERR_THREAD_MISMATCH => AnalyzerError::InvalidRequest { reason: msg },
        RF_LUAU_ERR_COMPLETION_UNSUPPORTED => AnalyzerError::Unsupported {
            capability: CapabilityId::new("analyze.completion"),
        },
        RF_LUAU_ERR_VERSION => AnalyzerError::VersionMismatch {
            field: "bridge".to_string(),
            expected: format!("abi={ABI_VERSION} contract={SHIM_CONTRACT_VERSION}"),
            actual: msg,
        },
        // status 6 は spec 写像表どおり専用 message（"shim internal error"）。
        RF_LUAU_ERR_INTERNAL => AnalyzerError::Internal {
            message: format!("shim internal error (status {status}): {msg}"),
        },
        // status 7（fault injection seam — test-only）を含むその他の
        // コードは Internal。status code を message に保持する。
        _ => AnalyzerError::Internal {
            message: format!("shim error (status {status}): {msg}"),
        },
    }
}

#[cfg(test)]
mod create_negotiation_tests {
    //! create 時の `version`/`struct_size` ネゴシエーション fault fixture。
    //! extern 宣言を再定義せず、ffi.rs の実シグネチャを使う。

    use super::*;

    fn base_options() -> RfLuauCreateOptions {
        RfLuauCreateOptions {
            struct_size: u32::try_from(size_of::<RfLuauCreateOptions>()).unwrap_or(u32::MAX),
            abi_version: ABI_VERSION,
            contract_version: SHIM_CONTRACT_VERSION,
            mode_strict: 0,
            enable_completion: 1,
            max_source_bytes: 0,
            module_time_limit_sec: 0.0,
            source_type_module: 0,
        }
    }

    #[test]
    #[expect(unsafe_code)]
    // SAFETY: options/out/out_error は有効なローカルポインタ。shim は
    // version 不一致を検出しハンドルを返さない。err は rf_luau_free で解放。
    fn abi_version_mismatch_fails_closed() {
        let mut options = base_options();
        options.abi_version = 0xFFFF; // 不一致を注入
        let mut handle: *mut c_void = std::ptr::null_mut();
        let mut err: *mut c_char = std::ptr::null_mut();
        // SAFETY: 上記の通り。
        let status =
            unsafe { rf_luau_analyzer_create(&raw const options, &raw mut handle, &raw mut err) };
        assert_eq!(status, 8, "expected RF_LUAU_ERR_VERSION");
        assert!(handle.is_null());
        if !err.is_null() {
            // SAFETY: err は shim が返した new char[] ポインタ。
            unsafe { rf_luau_free(err.cast::<c_void>()) };
        }
    }

    #[test]
    #[expect(unsafe_code)]
    // SAFETY: shim は struct_size のみ先に検査し、不足なら他フィールドを
    // 読まずに INTERNAL を返す。縮小構造体の先頭 u32 がそのまま
    // struct_size として解釈される。
    fn struct_size_mismatch_rejected() {
        // align(8) で RfLuauCreateOptions(f64 含有)と同じ alignment に揃え、
        // ポインタキャストを alignment 的に正当化する。struct_size=4 の
        // 先頭フィールドだけが shim に読まれる。
        #[repr(C, align(8))]
        struct TinyOptions {
            struct_size: u32,
        }
        let options = TinyOptions { struct_size: 4 };
        let mut handle: *mut c_void = std::ptr::null_mut();
        let mut err: *mut c_char = std::ptr::null_mut();
        // SAFETY: 上記の通り。shim は sizeof(RfLuauCreateOptions) 未満を拒否。
        let status = unsafe {
            rf_luau_analyzer_create(
                (&raw const options).cast::<RfLuauCreateOptions>(),
                &raw mut handle,
                &raw mut err,
            )
        };
        assert_ne!(status, 0);
        assert!(handle.is_null());
        if !err.is_null() {
            // SAFETY: err は shim が返した new char[] ポインタ。
            unsafe { rf_luau_free(err.cast::<c_void>()) };
        }
    }
}
