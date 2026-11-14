//! FFI 境界 fault fixture — 生バイト・version 不一致・native thread 不一致など、
//! 安全な公開 API（`String`/`&str`）では表現できない契約ケースを
//! `ffi` モジュール経由で直接検証する。unsafe はこのファイルに局所許可で
//! 隔離し、SAFETY コメントを付す。

use crate::contract::AnalyzerError;
use crate::ffi::{BridgeHandle, CreateOptions, bridge_versions};

fn default_opts() -> CreateOptions {
    CreateOptions {
        mode_strict: false,
        enable_completion: true,
        max_source_bytes: 0,
        module_time_limit_sec: 0.0,
        source_type_module: false,
    }
}

#[test]
fn bridge_versions_report_luau_revision() {
    let v = bridge_versions();
    assert_eq!(v.contract_version, crate::ffi::CONTRACT_VERSION);
    assert_eq!(v.luau_version, "0.740");
    assert!(v.impl_version.contains("ffi-poc"));
}

#[test]
fn invalid_utf8_bytes_rejected_at_shim_boundary() {
    let handle = BridgeHandle::create(&default_opts()).expect("create");
    // 0xFF は UTF-8 として不正。Rust の String API では作れない入力を
    // FFI 層へ直接渡し、shim が INVALID_UTF8 で fail-closed することを確認。
    let bad: &[u8] = b"local x = \xff\xfe";
    let err = handle.analyze(bad, -1).expect_err("invalid utf8 must fail");
    assert!(
        matches!(err, AnalyzerError::InvalidRequest { .. }),
        "got {err:?}"
    );
}

#[test]
fn cursor_beyond_source_rejected_by_shim() {
    let handle = BridgeHandle::create(&default_opts()).expect("create");
    let err = handle
        .analyze(b"return 1", 999)
        .expect_err("cursor range must fail");
    assert!(
        matches!(err, AnalyzerError::InvalidRequest { .. }),
        "got {err:?}"
    );
}

#[test]
fn source_size_limit_enforced_by_shim() {
    let opts = CreateOptions {
        max_source_bytes: 4,
        ..default_opts()
    };
    let handle = BridgeHandle::create(&opts).expect("create");
    let err = handle
        .analyze(b"return 12345", -1)
        .expect_err("oversize source must fail");
    assert!(
        matches!(err, AnalyzerError::InvalidRequest { .. }),
        "got {err:?}"
    );
}

#[test]
fn completion_disabled_returns_unsupported_status() {
    let opts = CreateOptions {
        enable_completion: false,
        ..default_opts()
    };
    let handle = BridgeHandle::create(&opts).expect("create");
    let err = handle
        .analyze(b"return 1", 3)
        .expect_err("completion must be unsupported");
    match err {
        AnalyzerError::Unsupported { capability } => assert_eq!(capability, "completion"),
        other => panic!("expected Unsupported{{completion}}, got {other:?}"),
    }
}

// version_mismatch / struct_size_mismatch の create 系 fault fixture は
// ffi.rs 内の `create_negotiation_tests` に置く（extern 宣言の再定義を
// 避けるため、実シグネチャを持つモジュール内で検査する）。

#[test]
#[expect(unsafe_code)]
// SAFETY: 生ポインタは create 済みハンドルのアドレス値のみを運び、
// 別スレッドでは status 問い合わせだけを行う。destroy は所有スレッドで
// BridgeHandle::drop が実行するため二重解放は起きない。
fn thread_mismatch_rejected_by_shim() {
    let handle = BridgeHandle::create(&default_opts()).expect("create");
    // ハンドルを所有スレッドから別スレッドへ「貸し出し」、shim の
    // ownerThread 検査が THREAD_MISMATCH を返すことを確認する。
    // BridgeHandle は Send ではないため、生ポインタ経由で FFI を呼ぶ。
    let ptr = handle.ptr.as_ptr() as usize;
    let src = b"return 1";
    let result = std::thread::spawn(move || unsafe {
        unsafe extern "C" {
            fn rf_luau_analyzer_analyze(
                analyzer: *mut std::ffi::c_void,
                source: *const std::ffi::c_char,
                source_len: usize,
                cursor_offset: i64,
                json_out: *mut *mut std::ffi::c_char,
                out_error: *mut *mut std::ffi::c_char,
            ) -> i32;
            fn rf_luau_free(ptr: *mut std::ffi::c_void);
        }
        let mut json: *mut std::ffi::c_char = std::ptr::null_mut();
        let mut err: *mut std::ffi::c_char = std::ptr::null_mut();
        let status = rf_luau_analyzer_analyze(
            ptr as *mut std::ffi::c_void,
            src.as_ptr().cast::<std::ffi::c_char>(),
            src.len(),
            -1,
            &raw mut json,
            &raw mut err,
        );
        if !err.is_null() {
            rf_luau_free(err.cast::<std::ffi::c_void>());
        }
        status
    })
    .join()
    .expect("thread join");
    assert_eq!(result, 4, "expected RF_LUAU_ERR_THREAD_MISMATCH");
    // handle はここで drop され destroy が呼ばれる（所有スレッドで実行）。
}

// 注: shim 文字列の所有権/解放は ShimString(ffi.rs) の Drop が
// rf_luau_free を一度だけ呼ぶ構造で担保され、tests/bridge.rs の
// 正常系がその経路を通る。個別の free 検査は ASan/valgrind 相当の
// 実行系検証が必要であり、本 PoC では所有権設計のコードレビューと
// 正常系 Drop 経路の通過をもって記録する（evidence/rf560 参照）。
