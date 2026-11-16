//! 製品契約の統合テスト — `ruviform_script` の中立型と公開 API
//! （`LuauAnalyzer`/`LuauAnalyzerOptions`/`ScriptAnalyzer`）のみを使う。
//! FFI 直叩き・生バイト・スレッド跨ぎは `src/ffi_boundary_tests.rs`、
//! fault injection は `tests/faults.rs`、registry 解決は
//! `tests/registry.rs` が担う。

use std::fmt::Write as _;

use ruviform_script::{
    AnalyzeRequest, AnalyzerError, AnalyzerRegistry, CONTRACT_VERSION, DiagnosticLevel,
    DiagnosticSeverity, ScriptAnalyzer, ScriptLanguageId, ScriptSource, SourcePosition,
    SourceRevision, TypeDefinition, WantedCapabilities, verify_result_revision,
};
use ruviform_script_luau::{LuauAnalyzer, LuauAnalyzerOptions, luau_language};

fn luau() -> ScriptLanguageId {
    luau_language()
}

fn source(text: &str, rev: u64) -> ScriptSource {
    ScriptSource::new(text, luau(), None, SourceRevision::new(rev))
}

fn request(text: &str, rev: u64) -> AnalyzeRequest {
    AnalyzeRequest {
        source: source(text, rev),
        language: luau(),
        type_definitions: None,
        wanted: WantedCapabilities {
            diagnostics: DiagnosticLevel::SyntaxAndType,
            completion: false,
        },
        cursor: None,
    }
}

fn connect(opts: &LuauAnalyzerOptions) -> LuauAnalyzer {
    LuauAnalyzer::connect(opts).expect("analyzer connect")
}

#[test]
fn descriptor_advertises_neutral_contract() {
    let analyzer = connect(&LuauAnalyzerOptions::default());
    let d = analyzer.descriptor();
    assert_eq!(d.analyzer_id.as_str(), "luau-analysis");
    assert_eq!(d.language, luau());
    assert_eq!(d.version.contract_version, CONTRACT_VERSION);
    assert_eq!(d.capabilities.diagnostics, DiagnosticLevel::SyntaxAndType);
    // default options: enable_completion=false → capability も false。
    assert!(!d.capabilities.completion);
    assert_eq!(d.capabilities.max_source_bytes, None);
    assert_eq!(d.capabilities.supported_languages, vec![luau()]);
    // impl_version は "<shim impl>+luau<version>" 形式で Luau revision を含む。
    assert!(
        d.version.impl_version.contains("luau0.740"),
        "got {}",
        d.version.impl_version
    );
}

#[test]
fn valid_source_produces_result_with_revision() {
    let analyzer = connect(&LuauAnalyzerOptions::default());
    let req = request("local x: number = 1\nreturn x\n", 1);
    let result = analyzer.analyze(&req).expect("analyze");
    assert_eq!(result.source_revision, SourceRevision::new(1));
    assert!(verify_result_revision(&req, &result).is_ok());
    // completion 未要求 → 空 Vec。
    assert_eq!(result.completions, Vec::new());
    // 正常系では Error 級 Diagnostic が出ないこと。
    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| d.severity != DiagnosticSeverity::Error),
        "unexpected error diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn syntax_error_is_structured() {
    let analyzer = connect(&LuauAnalyzerOptions::default());
    let req = request("local x = \nreturn x\n", 2);
    let result = analyzer.analyze(&req).expect("analyze");
    assert_eq!(result.source_revision, SourceRevision::new(2));
    let parse = result
        .diagnostics
        .iter()
        .find(|d| d.code.as_deref() == Some("parse"))
        .expect("parse diagnostic");
    assert_eq!(parse.severity, DiagnosticSeverity::Error);
    assert_ne!(parse.message, "");
    // range は offset/line/col を持つ SourceRange に再構成される。
    let range = parse.range.expect("diagnostic range");
    assert!(range.start.offset <= range.end.offset);
    assert_ne!(result.diagnostics, Vec::new());
}

#[test]
fn diagnostic_offsets_follow_luau_newline_semantics() {
    // shim が emit する (line, column) を Rust 側 position_offset が
    // byte offset へ戻す end-to-end の round-trip。行区切りは
    // Luau Lexer 規約の \n のみ — \r\n は \n で 1 回、単独 \r は
    // 改行にならない（Lexer.cpp isNewline）。
    let analyzer = connect(&LuauAnalyzerOptions::default());

    // "--x\r\n!" — '!' は byte 5、lexer は line=1,col=0 と報告する
    // はず（\r\n の \n のみが行を進める）。
    let src_crlf = "--x\r\n!";
    let result = analyzer
        .analyze(&request(src_crlf, 60))
        .expect("analyze crlf");
    let d = result
        .diagnostics
        .iter()
        .find(|d| d.severity == DiagnosticSeverity::Error)
        .expect("error diagnostic");
    let start = d.range.expect("range").start;
    assert_eq!(start.offset, 5, "crlf diagnostic offset");
    assert_eq!(start.line, 1);
    assert_eq!(start.col, 0);

    // "--[[\r]]\n!" — ブロックコメント内の単独 \r は改行にならず、
    // '!' は byte 8、line=1,col=0。\r を改行と誤計数すると offset 5
    // （コメント終端 ']' の位置）を指してしまう — 本 test が
    // その回帰を検出する。
    let src_lone_cr = "--[[\r]]\n!";
    let result = analyzer
        .analyze(&request(src_lone_cr, 61))
        .expect("analyze lone-cr");
    let d = result
        .diagnostics
        .iter()
        .find(|d| d.severity == DiagnosticSeverity::Error)
        .expect("error diagnostic");
    let start = d.range.expect("range").start;
    assert_eq!(start.offset, 8, "lone-cr diagnostic offset");
    assert_eq!(start.line, 1);
    assert_eq!(start.col, 0);
}

#[test]
fn module_time_limit_timeout_fails_closed() {
    // module_time_limit_sec を実質 0 にすると型検査が TimeLimitError
    // で打ち切られ、Luau は timeoutHits に module を記録する。
    // 部分診断を成功結果として返さず Internal で fail-closed する
    // ことを確認する（review cycle 2 P2 — silent partial diagnostics
    // の防止）。
    let analyzer = connect(&LuauAnalyzerOptions {
        module_time_limit_sec: f64::MIN_POSITIVE,
        ..LuauAnalyzerOptions::default()
    });
    // 型検査が重い source で timeout を確実に踏む。
    let mut src = String::new();
    for i in 0..200 {
        writeln!(src, "local v{i}: number = {i}").expect("write");
    }
    let err = analyzer
        .analyze(&request(&src, 62))
        .expect_err("timeout must fail closed");
    assert!(matches!(err, AnalyzerError::Internal { .. }), "got {err:?}");
}

#[test]
fn invalid_module_time_limit_rejected_at_connect() {
    for bad in [f64::NAN, -1.0, f64::NEG_INFINITY] {
        let err = LuauAnalyzer::connect(&LuauAnalyzerOptions {
            module_time_limit_sec: bad,
            ..LuauAnalyzerOptions::default()
        })
        .map(|_| ())
        .expect_err("invalid module_time_limit_sec must fail");
        assert!(
            matches!(err, AnalyzerError::InvalidRequest { .. }),
            "value {bad}: got {err:?}"
        );
    }
    // 0（無制限）と正値は受理される。
    for ok in [0.0, 5.0] {
        LuauAnalyzer::connect(&LuauAnalyzerOptions {
            module_time_limit_sec: ok,
            ..LuauAnalyzerOptions::default()
        })
        .expect("valid module_time_limit_sec");
    }
}

#[test]
fn type_error_is_structured_in_strict_mode() {
    // default strict=true: 型注釈違反が Error diagnostic になる。
    let analyzer = connect(&LuauAnalyzerOptions::default());
    let req = request("local x: number = \"not a number\"\n", 3);
    let result = analyzer.analyze(&req).expect("analyze");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Error),
        "expected type error diagnostic, got {:?}",
        result.diagnostics
    );

    // strict=false (nonstrict) では同じ source が型エラーを出さない
    // （strict option の実効性 — options が shim へ届いている証拠）。
    let analyzer_ns = connect(&LuauAnalyzerOptions {
        strict: false,
        ..LuauAnalyzerOptions::default()
    });
    let result_ns = analyzer_ns.analyze(&req).expect("analyze nonstrict");
    assert!(
        result_ns
            .diagnostics
            .iter()
            .all(|d| d.severity != DiagnosticSeverity::Error),
        "unexpected error diagnostics in nonstrict: {:?}",
        result_ns.diagnostics
    );
}

#[test]
fn completion_returns_neutral_items() {
    let analyzer = connect(&LuauAnalyzerOptions {
        enable_completion: true,
        ..LuauAnalyzerOptions::default()
    });
    let src = "local alpha = 1\nal";
    let mut req = request(src, 70);
    req.cursor = Some(SourcePosition {
        offset: src.len() as u64,
        line: 1,
        col: 2,
    });
    req.wanted.completion = true;
    let result = analyzer.analyze(&req).expect("analyze");
    assert_ne!(result.completions, Vec::new(), "expected completions");
    // `al` プレフィックスで `alpha` が候補に含まれるはず。
    assert!(
        result.completions.iter().any(|c| c.label == "alpha"),
        "expected 'alpha' completion, got {:?}",
        result
            .completions
            .iter()
            .map(|c| &c.label)
            .collect::<Vec<_>>()
    );
    assert!(result.completions.iter().all(|c| !c.label.is_empty()));
    // replace_range は shim が計算する identifier prefix 範囲。
    assert!(result.completions.iter().any(|c| c.replace_range.is_some()));
}

#[test]
fn completion_unsupported_is_typed_error() {
    // enable_completion=false の Analyzer は Completion 能力を広告しない。
    let analyzer = connect(&LuauAnalyzerOptions::default());
    assert!(!analyzer.descriptor().capabilities.completion);

    let mut req = request("local x = 1", 20);
    req.cursor = Some(SourcePosition {
        offset: 10,
        line: 0,
        col: 10,
    });
    req.wanted.completion = true;
    let err = analyzer
        .analyze(&req)
        .expect_err("completion must be unsupported");
    assert!(
        matches!(&err, AnalyzerError::Unsupported { capability } if capability.as_str() == "analyze.completion"),
        "got {err:?}"
    );
}

#[test]
fn completion_requires_cursor() {
    let analyzer = connect(&LuauAnalyzerOptions {
        enable_completion: true,
        ..LuauAnalyzerOptions::default()
    });
    let mut req = request("local x = 1", 21);
    req.wanted.completion = true;
    // cursor = None — completion には位置が必須。
    let err = analyzer
        .analyze(&req)
        .expect_err("completion without cursor must fail");
    assert!(
        matches!(err, AnalyzerError::InvalidRequest { .. }),
        "got {err:?}"
    );
}

#[test]
fn cursor_without_completion_request_stays_diagnostics_only() {
    // regression: cursor の存在だけで native completion を起動しては
    // いけない。wanted.completion=false なら cursor_offset は shim に
    // 渡さず（-1）、Unsupported でもなく diagnostics のみの成功結果に
    // なること。
    let analyzer = connect(&LuauAnalyzerOptions::default());
    assert!(!analyzer.descriptor().capabilities.completion);

    let mut req = request("local x = 1", 21);
    req.cursor = Some(SourcePosition {
        offset: 1,
        line: 0,
        col: 1,
    });
    req.wanted.completion = false;

    let result = analyzer
        .analyze(&req)
        .expect("diagnostics-only request with cursor must succeed");
    assert_eq!(result.source_revision, req.source.revision);
    assert_eq!(result.completions, Vec::new());
}

#[test]
fn type_definitions_enable_type_resolution() {
    let analyzer = connect(&LuauAnalyzerOptions::default());
    let def = TypeDefinition::new(
        "rvTestDefs",
        source("declare function rvTestGreeter(name: string): string", 5),
    );

    // 型定義なし: 未知グローバル `rvTestGreeter` で Error diagnostic。
    let mut req = request("local s = rvTestGreeter(\"hi\")", 30);
    let before = analyzer.analyze(&req).expect("analyze before");
    assert!(
        before
            .diagnostics
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Error),
        "expected unknown-global error, got {:?}",
        before.diagnostics
    );

    // request 同梱の型定義で型解決 → Error なし。
    req.type_definitions = Some(vec![def]);
    req.source = source("local s = rvTestGreeter(\"hi\")", 31);
    let after = analyzer.analyze(&req).expect("analyze after");
    assert!(
        after
            .diagnostics
            .iter()
            .all(|d| d.severity != DiagnosticSeverity::Error),
        "unexpected error diagnostics: {:?}",
        after.diagnostics
    );
    assert_eq!(after.source_revision, SourceRevision::new(31));
}

#[test]
fn add_type_definition_persists_and_dedups() {
    let analyzer = connect(&LuauAnalyzerOptions::default());
    let def = TypeDefinition::new(
        "rvPersistDefs",
        source("declare function rvPersistFn(): number", 6),
    );
    analyzer.add_type_definition(&def).expect("add typedef");

    // 同名 package を重複同梱しても dedup で再ロードしない（冪等）。
    let mut req = request("local a = rvPersistFn()", 100);
    req.type_definitions = Some(vec![def.clone(), def]);
    let result = analyzer.analyze(&req).expect("analyze with dup defs");
    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| d.severity != DiagnosticSeverity::Error),
        "unexpected error diagnostics: {:?}",
        result.diagnostics
    );

    // type_definitions=None の後続 request でも定義は保持される
    // （shim に unload がないため analyzer 寿命中永続）。
    let req2 = request("local b = rvPersistFn()", 101);
    let result2 = analyzer.analyze(&req2).expect("analyze persisted");
    assert!(
        result2
            .diagnostics
            .iter()
            .all(|d| d.severity != DiagnosticSeverity::Error),
        "unexpected error diagnostics: {:?}",
        result2.diagnostics
    );
}

#[test]
fn add_type_definition_redefinition_is_last_write_wins() {
    // 同名 package の再定義は Luau が拒否しない — global binding は
    // 宣言名で上書きされるため last-write-wins。request 同梱の dedup
    // （loaded 済みは skip）とは別経路: add_type_definition は明示的な
    // 再定義 API として毎回 shim へロードする。
    let analyzer = connect(&LuauAnalyzerOptions::default());
    analyzer
        .add_type_definition(&TypeDefinition::new(
            "rvDupDefs",
            source("declare function rvDupFn(): number", 8),
        ))
        .expect("add typedef v1");
    analyzer
        .add_type_definition(&TypeDefinition::new(
            "rvDupDefs",
            source("declare function rvDupFn(): string", 8),
        ))
        .expect("redefine typedef v2");

    // v2（string）が勝つ: string 注釈は OK、number 注釈は型エラー。
    let ok = analyzer
        .analyze(&request("local s: string = rvDupFn()", 200))
        .expect("analyze string use");
    assert!(
        ok.diagnostics
            .iter()
            .all(|d| d.severity != DiagnosticSeverity::Error),
        "unexpected error diagnostics: {:?}",
        ok.diagnostics
    );

    let err = analyzer
        .analyze(&request("local n: number = rvDupFn()", 201))
        .expect("analyze number use");
    assert!(
        err.diagnostics
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Error),
        "expected type error after redefinition, got {:?}",
        err.diagnostics
    );
}

#[test]
fn completion_reflects_loaded_type_definition() {
    let analyzer = connect(&LuauAnalyzerOptions {
        enable_completion: true,
        ..LuauAnalyzerOptions::default()
    });
    let def = TypeDefinition::new(
        "rvCompletionDefs",
        source("declare function rvTestGreeter(name: string): string", 7),
    );

    let make_req = |rev: u64| {
        let src = "local v = rvTestGreet";
        let mut req = request(src, rev);
        req.cursor = Some(SourcePosition {
            offset: src.len() as u64,
            line: 0,
            col: u32::try_from(src.len()).expect("len fits u32"),
        });
        req.wanted.completion = true;
        req
    };

    // 型定義なし: `rvTestGreeter` は候補に現れない。
    let before = analyzer.analyze(&make_req(110)).expect("analyze before");
    assert!(
        !before
            .completions
            .iter()
            .any(|c| c.label.contains("rvTestGreeter")),
        "unexpected rvTestGreeter before typedef: {:?}",
        before
            .completions
            .iter()
            .map(|c| &c.label)
            .collect::<Vec<_>>()
    );

    analyzer.add_type_definition(&def).expect("add typedef");

    // 型定義後: completion 候補に現れる（FX-P-08 系の製品版）。
    let after = analyzer.analyze(&make_req(111)).expect("analyze after");
    assert!(
        after
            .completions
            .iter()
            .any(|c| c.label.contains("rvTestGreeter")),
        "expected rvTestGreeter in completions, got {:?}",
        after
            .completions
            .iter()
            .map(|c| &c.label)
            .collect::<Vec<_>>()
    );
    assert_eq!(after.source_revision, SourceRevision::new(111));
}

#[test]
fn schema_version_mismatch_fails_closed() {
    let analyzer = connect(&LuauAnalyzerOptions::default());

    // source.schema_version 不一致
    let mut req = request("return 1", 40);
    req.source.schema_version = 99;
    let err = analyzer.analyze(&req).expect_err("source schema mismatch");
    assert!(
        matches!(&err, AnalyzerError::VersionMismatch { field, .. } if field == "source.schema_version"),
        "got {err:?}"
    );

    // TypeDefinition.schema_version 不一致
    let mut bad_def = TypeDefinition::new("rvBad", source("declare function f(): number", 1));
    bad_def.schema_version = 99;
    let mut req = request("return 1", 41);
    req.type_definitions = Some(vec![bad_def]);
    let err = analyzer.analyze(&req).expect_err("typedef schema mismatch");
    assert!(
        matches!(&err, AnalyzerError::VersionMismatch { field, .. } if field == "type_definitions.schema_version"),
        "got {err:?}"
    );

    // TypeDefinition.source.schema_version 不一致
    let mut bad_src = TypeDefinition::new("rvBadSrc", source("declare function g(): number", 1));
    bad_src.source.schema_version = 99;
    let mut req = request("return 1", 42);
    req.type_definitions = Some(vec![bad_src]);
    let err = analyzer
        .analyze(&req)
        .expect_err("typedef source schema mismatch");
    assert!(
        matches!(&err, AnalyzerError::VersionMismatch { field, .. } if field == "type_definitions.source.schema_version"),
        "got {err:?}"
    );
}

#[test]
fn invalid_cursor_and_oversize_source_are_explicit_errors() {
    let analyzer = connect(&LuauAnalyzerOptions {
        enable_completion: true,
        max_source_bytes: 8,
        ..LuauAnalyzerOptions::default()
    });

    // カーソルがソース長を超える → InvalidRequest
    let mut req = request("local x = 1", 50);
    req.cursor = Some(SourcePosition {
        offset: 999,
        line: 0,
        col: 0,
    });
    req.wanted.completion = true;
    let err = analyzer.analyze(&req).expect_err("cursor range");
    assert!(
        matches!(err, AnalyzerError::InvalidRequest { .. }),
        "got {err:?}"
    );

    // UTF-8 char-boundary でない offset（"あ" は 3 byte — offset 7 は途中）
    let src = "local あ";
    let mut req = request(src, 51);
    req.cursor = Some(SourcePosition {
        offset: 7,
        line: 0,
        col: 7,
    });
    req.wanted.completion = true;
    let err = analyzer.analyze(&req).expect_err("non-boundary cursor");
    assert!(
        matches!(err, AnalyzerError::InvalidRequest { .. }),
        "got {err:?}"
    );

    // max_source_bytes 超過 → InvalidRequest
    let req = request("local x = 123456789", 52);
    let err = analyzer.analyze(&req).expect_err("source too large");
    assert!(
        matches!(err, AnalyzerError::InvalidRequest { .. }),
        "got {err:?}"
    );
}

#[test]
fn unsupported_language_is_invalid_request() {
    let analyzer = connect(&LuauAnalyzerOptions::default());
    let mut req = request("return 1", 60);
    req.language = ScriptLanguageId::new("rhai");
    let err = analyzer.analyze(&req).expect_err("language mismatch");
    assert!(
        matches!(err, AnalyzerError::InvalidRequest { .. }),
        "got {err:?}"
    );
}

#[test]
fn deterministic_results_for_identical_requests() {
    let analyzer = connect(&LuauAnalyzerOptions::default());
    let req = request("local x: number = 1\nreturn x\n", 61);
    let r1 = analyzer.analyze(&req).expect("analyze 1");
    let r2 = analyzer.analyze(&req).expect("analyze 2");
    assert_eq!(r1, r2);
}

#[test]
fn stale_result_rejected_by_consumer_check() {
    let analyzer = connect(&LuauAnalyzerOptions::default());
    let req = request("local x = 1", 70);
    let result = analyzer.analyze(&req).expect("analyze");
    assert!(verify_result_revision(&req, &result).is_ok());
    assert_eq!(
        verify_result_revision(&req, &{
            let mut stale = result.clone();
            stale.source_revision = SourceRevision::new(71);
            stale
        }),
        Err(AnalyzerError::StaleResult)
    );
}

#[test]
fn luau_vm_and_analysis_coexist_in_same_binary() {
    // mlua (vendored Luau VM = luau0-src Luau 0.740) と Analysis shim が
    // 同一バイナリで動作することを実測する（AC4 共存実証の製品版）。
    // VM 呼出しは Analyzer の成否とは独立（実行 Grant ではない）。
    let lua = mlua::Lua::new();
    let value: i64 = lua.load("return 20 + 22").eval().expect("eval");
    assert_eq!(value, 42);

    let analyzer = connect(&LuauAnalyzerOptions::default());
    let result = analyzer
        .analyze(&request("local s = 'ok'\nreturn s\n", 80))
        .expect("analyze");
    assert_eq!(result.source_revision, SourceRevision::new(80));
}

#[test]
fn registry_end_to_end_with_real_analyzer() {
    // 製品 registry への実 analyzer 登録→解決→解析の疎通
    // （fail-closed 行列の詳細は tests/registry.rs）。
    let mut registry = AnalyzerRegistry::new();
    let analyzer = connect(&LuauAnalyzerOptions::default());
    registry.register(Box::new(analyzer)).expect("register");
    let requirement = ruviform_script::AnalyzerRequirement {
        language: luau(),
        analyzer: None,
        required_capabilities: std::collections::BTreeSet::new(),
        min_contract_version: CONTRACT_VERSION,
    };
    let result = registry
        .analyze(
            &requirement,
            &request("local x: number = 1\nreturn x\n", 90),
        )
        .expect("registry analyze");
    assert_eq!(result.source_revision, SourceRevision::new(90));
}
