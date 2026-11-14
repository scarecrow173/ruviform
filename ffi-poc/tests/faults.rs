//! Fault injection fixture — shim の環境変数シーム
//! （`RF_FFI_POC_FAIL_INIT` / `RF_FFI_POC_FAIL_ANALYZE` / `RF_FFI_POC_FAIL_LOADDEF`）
//! が明示的なエラーを返すことを検証する。
//!
//! 環境変数はプロセス全体に影響するため、本ファイルは単一 #[test] に集約し、
//! テストスレッド並列実行での競合を避ける（cargo test の他バイナリとは
//! プロセスが分かれる）。

use ruviform_ffi_poc::{
    AnalyzeRequest, AnalyzerError, LuauAnalyzer, LuauAnalyzerOptions, ScriptAnalyzer,
    ScriptLanguageId, ScriptSource, SourceRevision, TypeDefinition, WantedCapabilities,
};

fn request() -> AnalyzeRequest {
    AnalyzeRequest {
        source: ScriptSource {
            text: "return 1".to_string(),
            language: ScriptLanguageId::luau(),
            revision: SourceRevision(1),
        },
        language: ScriptLanguageId::luau(),
        type_definitions: Vec::new(),
        wanted: WantedCapabilities::default(),
        cursor: None,
    }
}

#[test]
#[expect(unsafe_code)]
// SAFETY: std::env::set_var/remove_var は edition 2024 で unsafe。
// 本テストバイナリ内で唯一の #[test] であり、環境変数へのアクセスは
// このテストスレッドのみが行うため競合しない。
fn fault_injection_matrix_returns_explicit_errors() {
    // 1. init 失敗注入 → connect が Bridge エラーで fail-closed
    unsafe { std::env::set_var("RF_FFI_POC_FAIL_INIT", "1") };
    let err = LuauAnalyzer::connect(&LuauAnalyzerOptions::default())
        .err()
        .expect("injected init failure must fail");
    assert!(
        matches!(err, AnalyzerError::Bridge { status: 7, .. }),
        "got {err:?}"
    );
    unsafe { std::env::remove_var("RF_FFI_POC_FAIL_INIT") };

    // 2. analyze 失敗注入 → analyze が Bridge エラー
    let mut analyzer = LuauAnalyzer::connect(&LuauAnalyzerOptions::default()).expect("connect");
    unsafe { std::env::set_var("RF_FFI_POC_FAIL_ANALYZE", "1") };
    let err = analyzer
        .analyze(&request())
        .expect_err("injected analyze failure");
    assert!(
        matches!(err, AnalyzerError::Bridge { status: 7, .. }),
        "got {err:?}"
    );
    unsafe { std::env::remove_var("RF_FFI_POC_FAIL_ANALYZE") };

    // 3. load_definition 失敗注入 → add_type_definition が Bridge エラー
    unsafe { std::env::set_var("RF_FFI_POC_FAIL_LOADDEF", "1") };
    let err = analyzer
        .add_type_definition(TypeDefinition {
            package_name: "@test/fail".to_string(),
            source: "declare function f(): number".to_string(),
        })
        .expect_err("injected loaddef failure");
    assert!(
        matches!(err, AnalyzerError::Bridge { status: 7, .. }),
        "got {err:?}"
    );
    unsafe { std::env::remove_var("RF_FFI_POC_FAIL_LOADDEF") };

    // 4. 注入解除後は正常系へ復帰する（analyzer は破損していない）
    let result = analyzer
        .analyze(&request())
        .expect("analyze after injection cleared");
    assert_eq!(result.source_revision, SourceRevision(1));
}
