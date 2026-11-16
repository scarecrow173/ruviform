//! Fault injection fixture — shim の test-only 環境変数シーム
//! （`RF_LUAU_FAIL_INIT` / `RF_LUAU_FAIL_ANALYZE` / `RF_LUAU_FAIL_LOADDEF`、
//! status 7 `RF_LUAU_ERR_INJECTED`）が中立契約 `AnalyzerError::Internal`
//! に写像されることを検証する。
//!
//! 環境変数はプロセス全体に影響するため、本ファイルは単一 #[test] に集約し、
//! テストスレッド並列実行での競合を避ける（cargo test の他バイナリとは
//! プロセスが分かれる）。ffi-poc の `RF_FFI_POC_FAIL_*` を製品命名へ改名。

use ruviform_script::{
    AnalyzeRequest, AnalyzerError, ScriptAnalyzer, ScriptLanguageId, ScriptSource, SourceRevision,
    TypeDefinition, WantedCapabilities,
};
use ruviform_script_luau::{LuauAnalyzer, LuauAnalyzerOptions, luau_language};

fn request() -> AnalyzeRequest {
    AnalyzeRequest {
        source: ScriptSource::new("return 1", luau_language(), None, SourceRevision::new(1)),
        language: luau_language(),
        type_definitions: None,
        wanted: WantedCapabilities {
            diagnostics: ruviform_script::DiagnosticLevel::SyntaxAndType,
            completion: false,
        },
        cursor: None,
    }
}

fn typedef() -> TypeDefinition {
    TypeDefinition::new(
        "rvFailDefs",
        ScriptSource::new(
            "declare function rvFailFn(): number",
            ScriptLanguageId::new("luau"),
            None,
            SourceRevision::new(1),
        ),
    )
}

fn assert_injected_internal(err: &AnalyzerError) {
    match err {
        AnalyzerError::Internal { message } => {
            assert!(message.contains("status 7"), "got {message}");
        }
        other => panic!("expected Internal{{status 7}}, got {other:?}"),
    }
}

#[test]
#[expect(unsafe_code)]
// SAFETY: std::env::set_var/remove_var は edition 2024 で unsafe。
// 本テストバイナリ内で唯一の #[test] であり、環境変数へのアクセスは
// このテストスレッドのみが行うため競合しない。
fn fault_injection_matrix_returns_typed_errors() {
    // 1. init 失敗注入 → connect が Internal{status 7} で fail-closed
    unsafe { std::env::set_var("RF_LUAU_FAIL_INIT", "1") };
    let err = LuauAnalyzer::connect(&LuauAnalyzerOptions::default())
        .err()
        .expect("injected init failure must fail");
    assert_injected_internal(&err);
    unsafe { std::env::remove_var("RF_LUAU_FAIL_INIT") };

    // 2. analyze 失敗注入 → analyze が Internal{status 7}
    let analyzer = LuauAnalyzer::connect(&LuauAnalyzerOptions::default()).expect("connect");
    unsafe { std::env::set_var("RF_LUAU_FAIL_ANALYZE", "1") };
    let err = analyzer
        .analyze(&request())
        .expect_err("injected analyze failure");
    assert_injected_internal(&err);
    unsafe { std::env::remove_var("RF_LUAU_FAIL_ANALYZE") };

    // 3. load_definition 失敗注入 → add_type_definition が Internal{status 7}
    unsafe { std::env::set_var("RF_LUAU_FAIL_LOADDEF", "1") };
    let err = analyzer
        .add_type_definition(&typedef())
        .expect_err("injected loaddef failure");
    assert_injected_internal(&err);
    unsafe { std::env::remove_var("RF_LUAU_FAIL_LOADDEF") };

    // 4. 注入解除後は正常系へ復帰する（analyzer は破損していない）
    let result = analyzer
        .analyze(&request())
        .expect("analyze after injection cleared");
    assert_eq!(result.source_revision, SourceRevision::new(1));
}
