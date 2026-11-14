//! FX-P-08 / 契約 fixture の headless 統合テスト。
//! 公開 API（契約ミラー型 + `LuauAnalyzer` + `AnalyzerRegistry`）のみを使う。
//! FFI 直叩き・生バイト・スレッド跨ぎは `src/ffi_boundary_tests.rs` が担う。

use ruviform_ffi_poc::{
    AnalyzeRequest, AnalyzerError, AnalyzerRegistry, Cursor, DiagnosticKind, DiagnosticLevel,
    LuauAnalyzer, LuauAnalyzerOptions, ScriptAnalyzer, ScriptLanguageId, ScriptSource,
    SourceRevision, TypeDefinition, WantedCapabilities, verify_result_revision,
};

fn request(source: &str, revision: u64) -> AnalyzeRequest {
    AnalyzeRequest {
        source: ScriptSource {
            text: source.to_string(),
            language: ScriptLanguageId::luau(),
            revision: SourceRevision(revision),
        },
        language: ScriptLanguageId::luau(),
        type_definitions: Vec::new(),
        wanted: WantedCapabilities {
            diagnostics: Some(DiagnosticLevel::SyntaxAndType),
            completion: false,
        },
        cursor: None,
    }
}

fn connect(opts: &LuauAnalyzerOptions) -> LuauAnalyzer {
    LuauAnalyzer::connect(opts).expect("analyzer connect")
}

#[test]
fn fx_p08_valid_source_produces_result_with_revision() {
    let mut analyzer = connect(&LuauAnalyzerOptions::default());
    let req = request("local x: number = 1\nreturn x\n", 1);
    let result = analyzer.analyze(&req).expect("analyze");
    assert_eq!(result.source_revision, SourceRevision(1));
    assert!(result.completions.is_none());
    // 正常系では Error 級 Diagnostic が出ないこと（warning は許容）。
    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| d.kind != DiagnosticKind::Syntax
                || d.severity != ruviform_ffi_poc::DiagnosticSeverity::Error),
        "unexpected syntax error diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn fx_p08_syntax_error_is_structured() {
    let mut analyzer = connect(&LuauAnalyzerOptions::default());
    let req = request("local x = \nreturn x\n", 2);
    let result = analyzer.analyze(&req).expect("analyze");
    assert_eq!(result.source_revision, SourceRevision(2));
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.kind == DiagnosticKind::Syntax),
        "expected syntax diagnostic, got {:?}",
        result.diagnostics
    );
}

#[test]
fn fx_p08_type_error_is_structured() {
    // strict mode で型注釈違反を確実にエラー化する。
    let opts = LuauAnalyzerOptions {
        strict: true,
        ..LuauAnalyzerOptions::default()
    };
    let mut analyzer = connect(&opts);
    let req = request("local x: number = \"not a number\"\n", 3);
    let result = analyzer.analyze(&req).expect("analyze");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.kind == DiagnosticKind::Type),
        "expected type diagnostic, got {:?}",
        result.diagnostics
    );
}

#[test]
fn fx_p08_completion_changes_with_type_definition() {
    let opts = LuauAnalyzerOptions::default();
    let mut analyzer = connect(&opts);

    // 型定義なし: `rfPoc` プレフィックスの候補は存在しない。
    let mut req = request("local v = rfPoc", 10);
    req.cursor = Some(Cursor {
        byte_offset: "local v = rfPoc".len(),
    });
    req.wanted.completion = true;
    let before = analyzer.analyze(&req).expect("analyze before");
    let before_labels: Vec<&str> = before
        .completions
        .as_ref()
        .expect("completions")
        .iter()
        .map(|c| c.label.as_str())
        .collect();
    assert!(!before_labels.iter().any(|l| l.contains("rfPocGreeter")));

    // グローバル関数を持つ型定義を追加すると候補が現れる。
    analyzer
        .add_type_definition(TypeDefinition {
            package_name: "@test/defs".to_string(),
            source: "declare function rfPocGreeter(name: string): string".to_string(),
        })
        .expect("load definition");

    req.source.revision = SourceRevision(11);
    let after = analyzer.analyze(&req).expect("analyze after");
    let after_labels: Vec<&str> = after
        .completions
        .as_ref()
        .expect("completions")
        .iter()
        .map(|c| c.label.as_str())
        .collect();
    assert!(
        after_labels.iter().any(|l| l.contains("rfPocGreeter")),
        "expected rfPocGreeter in completions, got {after_labels:?}"
    );
    // revision が結果へ伝播していること。
    assert_eq!(after.source_revision, SourceRevision(11));
    assert!(
        after
            .completions
            .as_ref()
            .unwrap()
            .iter()
            .all(|c| c.source_revision == SourceRevision(11))
    );
}

#[test]
fn fx_n13_completion_unsupported_is_explicit() {
    // enable_completion=false の Analyzer は Completion 能力を広告しない。
    let opts = LuauAnalyzerOptions {
        enable_completion: false,
        ..LuauAnalyzerOptions::default()
    };
    let mut analyzer = connect(&opts);
    assert!(!analyzer.descriptor().capabilities.completion);

    let mut req = request("local x = 1", 20);
    req.cursor = Some(Cursor { byte_offset: 10 });
    req.wanted.completion = true;
    let err = analyzer
        .analyze(&req)
        .expect_err("completion must be unsupported");
    match err {
        AnalyzerError::Unsupported { capability } => assert_eq!(capability, "completion"),
        other => panic!("expected Unsupported{{completion}}, got {other:?}"),
    }
}

#[test]
fn fx_n14_stale_result_rejected_by_consumer() {
    let mut analyzer = connect(&LuauAnalyzerOptions::default());
    let req = request("return 1", 30);
    let result = analyzer.analyze(&req).expect("analyze");
    assert!(verify_result_revision(&result, SourceRevision(30)).is_ok());
    assert_eq!(
        verify_result_revision(&result, SourceRevision(31)),
        Err(AnalyzerError::StaleResult)
    );
}

#[test]
fn fx_n24_analyzer_unavailable_when_unregistered() {
    let mut registry = AnalyzerRegistry::new();
    let req = request("return 1", 40);
    let err = registry
        .analyze(&req)
        .expect_err("empty registry must fail closed");
    assert_eq!(err, AnalyzerError::Unavailable);
}

#[test]
fn registry_resolves_and_reports_descriptor() {
    let mut registry = AnalyzerRegistry::new();
    let analyzer = connect(&LuauAnalyzerOptions::default());
    let desc = analyzer.descriptor().clone();
    registry.register(Box::new(analyzer)).expect("register");
    assert_eq!(registry.list().count(), 1);
    let resolved = registry
        .resolve(&ScriptLanguageId::luau())
        .expect("resolve");
    assert_eq!(resolved.descriptor().analyzer_id, desc.analyzer_id);
    // impl_version / contract_version が分離していること（契約語彙）。
    assert_ne!(resolved.descriptor().version.impl_version, "");
    assert!(resolved.descriptor().version.impl_version.contains("luau"));
}

#[test]
fn invalid_cursor_and_oversize_source_are_explicit_errors() {
    let mut analyzer = connect(&LuauAnalyzerOptions {
        max_source_bytes: 8,
        ..LuauAnalyzerOptions::default()
    });

    // カーソルがソース長を超える → InvalidRequest
    let mut req = request("local x = 1", 50);
    req.cursor = Some(Cursor { byte_offset: 999 });
    req.wanted.completion = true;
    let err = analyzer.analyze(&req).expect_err("cursor range");
    assert!(
        matches!(err, AnalyzerError::InvalidRequest { .. }),
        "got {err:?}"
    );

    // max_source_bytes 超過 → InvalidRequest（Rust 層で fail-closed）
    let req = request("local x = 123456789", 51);
    let err = analyzer.analyze(&req).expect_err("source too large");
    assert!(
        matches!(err, AnalyzerError::InvalidRequest { .. }),
        "got {err:?}"
    );
}

#[test]
fn analyzer_independent_of_runtime_backend() {
    // Runtime Backend を一切登録せず Analyzer 単独で解析できること
    // （契約 C5: Analyzer/Runtime は独立 Port。解析成功は実行許可ではない）。
    let mut analyzer = connect(&LuauAnalyzerOptions::default());
    let req = request("local s = 'ok'\nreturn s\n", 60);
    let result = analyzer.analyze(&req).expect("analyze without runtime");
    assert_eq!(result.source_revision, SourceRevision(60));
    // 結果型に実行許可/Grant を示すフィールドが存在しないことを型で保証する
    // （AnalyzeResult は diagnostics/completions/source_revision のみ）。
}

#[test]
fn completion_kind_and_positions_are_structured() {
    let mut analyzer = connect(&LuauAnalyzerOptions::default());
    let src = "local alpha = 1\nal";
    let mut req = request(src, 70);
    req.cursor = Some(Cursor {
        byte_offset: src.len(),
    });
    req.wanted.completion = true;
    let result = analyzer.analyze(&req).expect("analyze");
    let completions = result.completions.as_ref().expect("completions");
    assert!(!completions.is_empty(), "expected at least one completion");
    // `al` プレフィックスで `alpha` が候補に含まれるはず。
    assert!(
        completions.iter().any(|c| c.label == "alpha"),
        "expected 'alpha' completion, got {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    // 全候補が kind を持ち revision を保持する。
    assert!(
        completions
            .iter()
            .all(|c| c.source_revision == SourceRevision(70))
    );
    assert!(completions.iter().all(|c| !c.label.is_empty()));
}

#[test]
fn luau_vm_and_analysis_coexist_in_same_binary() {
    // mlua (vendored Luau VM = luau0-src Luau 0.740) と Analysis shim が
    // 同一バイナリで動作することを実測する。VM 呼出しは Analyzer の
    // 成否とは独立（Grant ではない）。
    let lua = mlua::Lua::new();
    let value: i64 = lua.load("return 20 + 22").eval().expect("eval");
    assert_eq!(value, 42);

    let mut analyzer = connect(&LuauAnalyzerOptions::default());
    let result = analyzer
        .analyze(&request("return 'analysis-ok'", 80))
        .expect("analyze");
    assert_eq!(result.source_revision, SourceRevision(80));
}

#[test]
fn version_info_is_exposed_for_negotiation_evidence() {
    let analyzer = connect(&LuauAnalyzerOptions::default());
    let v = &analyzer.descriptor().version;
    // impl_version は "<shim impl>+luau<version>" 形式で Luau revision を含む。
    assert!(
        v.impl_version.contains("luau0.740"),
        "got {}",
        v.impl_version
    );
    assert_eq!(v.contract_version, 1);
}
