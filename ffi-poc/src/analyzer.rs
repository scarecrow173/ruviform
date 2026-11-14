//! `LuauAnalyzer` — FFI Shim 越しに Luau.Analysis を呼ぶ `ScriptAnalyzer`
//! 実装と、契約語彙の `AnalyzerRegistry`（Fail-Closed 解決）。
//!
//! Analyzer の成否は実行 Grant を意味しない（契約 C5）。本モジュールは
//! Runtime Backend を一切参照しない。

use std::collections::BTreeMap;

use crate::contract::{
    AnalyzeRequest, AnalyzeResult, AnalyzerCapabilities, AnalyzerDescriptor, AnalyzerError,
    Completion, CompletionKind, Diagnostic, DiagnosticKind, DiagnosticLevel, DiagnosticSeverity,
    ScriptBackendVersion, ScriptLanguageId, SourcePosition, SourceRange, SourceRevision,
    TypeDefinition, WantedCapabilities, verify_result_revision,
};
use crate::ffi::{BridgeHandle, CreateOptions, bridge_versions};

/// `ScriptAnalyzer` Port（契約 C5 のミラー）。Vendor 型をシグネチャに出さない。
pub trait ScriptAnalyzer {
    /// 静的記述を返す。
    fn descriptor(&self) -> &AnalyzerDescriptor;
    /// 解析を実行する。`&mut self` — FFI ハンドルは可変状態を持つ。
    fn analyze(&mut self, request: &AnalyzeRequest) -> Result<AnalyzeResult, AnalyzerError>;
}

/// `LuauAnalyzer` 生成オプション。
#[derive(Debug, Clone)]
pub struct LuauAnalyzerOptions {
    /// strict モードで型検査するか（既定: false = nonstrict）。
    pub strict: bool,
    /// Completion 能力を有効化するか（既定: true）。
    /// false にすると capabilities.completion=false を広告し、
    /// Completion 要求は `Unsupported{completion}` で Fail Closed する
    /// （FX-N-13 fixture 用の能力ゲート）。
    pub enable_completion: bool,
    /// 受理する最大ソースバイト数（0 = 無制限）。
    pub max_source_bytes: u32,
    /// モジュール単位の型検査時間制限（秒、0 = 無制限）。
    /// timeout fault fixture 用。
    pub module_time_limit_sec: f64,
    /// ソースを Module として扱うか（既定: false = Script）。
    pub source_type_module: bool,
}

impl Default for LuauAnalyzerOptions {
    fn default() -> Self {
        Self {
            strict: false,
            enable_completion: true,
            max_source_bytes: 0,
            module_time_limit_sec: 0.0,
            source_type_module: false,
        }
    }
}

/// Luau.Analysis を FFI Shim 経由で利用する Analyzer。
/// Runtime Backend なしで単独利用可能（契約 C5 の独立性）。
pub struct LuauAnalyzer {
    descriptor: AnalyzerDescriptor,
    handle: BridgeHandle,
    loaded_definitions: Vec<TypeDefinition>,
}

impl LuauAnalyzer {
    /// shim を初期化し version をネゴシエートする。
    /// ABI/契約 version 不一致は `AnalyzerError::VersionMismatch` で
    /// fail-closed（handle は生成されない）。
    pub fn connect(options: &LuauAnalyzerOptions) -> Result<Self, AnalyzerError> {
        let versions = bridge_versions();
        // shim が公開する契約 version が PoC ミラー定数と一致するかを
        // 接続時に検査する（公開定数は ffi.rs と同期）。
        if versions.contract_version != crate::ffi::CONTRACT_VERSION {
            return Err(AnalyzerError::VersionMismatch {
                field: "contract_version".to_string(),
                expected: crate::ffi::CONTRACT_VERSION.to_string(),
                actual: versions.contract_version.to_string(),
            });
        }

        let handle = BridgeHandle::create(&CreateOptions {
            mode_strict: options.strict,
            enable_completion: options.enable_completion,
            max_source_bytes: options.max_source_bytes,
            module_time_limit_sec: options.module_time_limit_sec,
            source_type_module: options.source_type_module,
        })?;

        let capabilities = AnalyzerCapabilities {
            diagnostics: DiagnosticLevel::SyntaxAndType,
            completion: options.enable_completion,
            max_source_bytes: u64::from(options.max_source_bytes),
            supported_languages: vec![ScriptLanguageId::luau()],
        };
        let descriptor = AnalyzerDescriptor {
            analyzer_id: "luau-analysis-ffi-poc".to_string(),
            language: ScriptLanguageId::luau(),
            version: ScriptBackendVersion {
                impl_version: format!("{}+luau{}", versions.impl_version, versions.luau_version),
                contract_version: crate::ffi::CONTRACT_VERSION,
            },
            capabilities,
        };
        Ok(Self {
            descriptor,
            handle,
            loaded_definitions: Vec::new(),
        })
    }

    /// 型定義を Analyzer のグローバル環境へ追加する
    /// （Completion 候補変化 fixture: FX-P-08 関連）。
    /// 追加済み定義は `analyze` ごとに再適用される。
    pub fn add_type_definition(&mut self, def: TypeDefinition) -> Result<(), AnalyzerError> {
        self.handle
            .load_definition(&def.package_name, &def.source)?;
        self.loaded_definitions.push(def);
        Ok(())
    }
}

impl ScriptAnalyzer for LuauAnalyzer {
    fn descriptor(&self) -> &AnalyzerDescriptor {
        &self.descriptor
    }

    fn analyze(&mut self, request: &AnalyzeRequest) -> Result<AnalyzeResult, AnalyzerError> {
        // capability 検査（Fail Closed — 契約 F-13/FX-N-13）
        let caps = &self.descriptor.capabilities;
        if request.language != self.descriptor.language {
            return Err(AnalyzerError::InvalidRequest {
                reason: format!(
                    "language {} not supported by {}",
                    request.language, self.descriptor.language
                ),
            });
        }
        if let Some(wanted) = request.wanted.diagnostics {
            let supported = match caps.diagnostics {
                DiagnosticLevel::Unsupported => 0,
                DiagnosticLevel::Syntax => 1,
                DiagnosticLevel::SyntaxAndType => 2,
            };
            let needed = match wanted {
                DiagnosticLevel::Unsupported => 0,
                DiagnosticLevel::Syntax => 1,
                DiagnosticLevel::SyntaxAndType => 2,
            };
            if needed > supported {
                return Err(AnalyzerError::Unsupported {
                    capability: "diagnostics".to_string(),
                });
            }
        }
        if request.wanted.completion && !caps.completion {
            return Err(AnalyzerError::Unsupported {
                capability: "completion".to_string(),
            });
        }
        if caps.max_source_bytes != 0 && request.source.text.len() as u64 > caps.max_source_bytes {
            return Err(AnalyzerError::InvalidRequest {
                reason: "source exceeds max_source_bytes".to_string(),
            });
        }
        if !request.source.text.is_empty()
            && std::str::from_utf8(request.source.text.as_bytes()).is_err()
        {
            return Err(AnalyzerError::InvalidRequest {
                reason: "source is not valid UTF-8".to_string(),
            });
        }
        if let Some(cursor) = request.cursor
            && (cursor.byte_offset > request.source.text.len()
                || !request.source.text.is_char_boundary(cursor.byte_offset))
        {
            return Err(AnalyzerError::InvalidRequest {
                reason: "cursor offset out of range or not on char boundary".to_string(),
            });
        }

        // 要求同梱の型定義を未適用のものだけ shim へロードする
        // （package_name 重複は PoC では初回のみ適用 — 再定義は
        // add_type_definition 側の責務）。
        for def in &request.type_definitions {
            if self
                .loaded_definitions
                .iter()
                .any(|d| d.package_name == def.package_name)
            {
                continue;
            }
            self.handle
                .load_definition(&def.package_name, &def.source)?;
            self.loaded_definitions.push(def.clone());
        }

        let cursor_offset = request
            .cursor
            .map_or(-1, |c| i64::try_from(c.byte_offset).unwrap_or(i64::MAX));
        let json = self
            .handle
            .analyze(request.source.text.as_bytes(), cursor_offset)?;
        parse_analyze_json(&json, request.source.revision, request.cursor.is_some())
    }
}

/// `AnalyzerRegistry` — 契約 C5 の Fail-Closed 解決をミラー。
/// Backend Registry とは独立。未知/曖昧/Version/Capability 不一致を拒否する。
#[derive(Default)]
pub struct AnalyzerRegistry {
    analyzers: Vec<Box<dyn ScriptAnalyzer>>,
}

impl AnalyzerRegistry {
    /// 新しい空 Registry。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Analyzer を登録する。同一 `analyzer_id` の重複は拒否。
    pub fn register(&mut self, analyzer: Box<dyn ScriptAnalyzer>) -> Result<(), AnalyzerError> {
        let id = &analyzer.descriptor().analyzer_id;
        if self
            .analyzers
            .iter()
            .any(|a| &a.descriptor().analyzer_id == id)
        {
            return Err(AnalyzerError::Internal {
                message: format!("duplicate analyzer id: {id}"),
            });
        }
        self.analyzers.push(analyzer);
        Ok(())
    }

    /// 登録済み Analyzer を列挙する。
    pub fn list(&self) -> impl Iterator<Item = &AnalyzerDescriptor> {
        self.analyzers.iter().map(|a| a.descriptor())
    }

    /// 言語と言語一致で Analyzer を Fail-Closed 解決する。
    /// 未登録は `Unavailable`（F-18/FX-N-24）。複数候補は `backend` 相当の
    /// 明示指定がないため `Internal`（`PoC` では単一登録を前提）。
    pub fn resolve(
        &self,
        language: &ScriptLanguageId,
    ) -> Result<&dyn ScriptAnalyzer, AnalyzerError> {
        let mut matches = self
            .analyzers
            .iter()
            .filter(|a| a.descriptor().language == *language);
        match (matches.next(), matches.next()) {
            (None, _) => Err(AnalyzerError::Unavailable),
            (Some(a), None) => Ok(a.as_ref()),
            (Some(_), Some(_)) => Err(AnalyzerError::Internal {
                message: "ambiguous analyzer candidates".to_string(),
            }),
        }
    }

    /// 登録済み Analyzer 経由で解析する。未登録なら `Unavailable`。
    pub fn analyze(&mut self, request: &AnalyzeRequest) -> Result<AnalyzeResult, AnalyzerError> {
        // resolve は &self、analyze は &mut self のため位置を先に解決する。
        let index = self
            .analyzers
            .iter()
            .position(|a| a.descriptor().language == request.language)
            .ok_or(AnalyzerError::Unavailable)?;
        if self
            .analyzers
            .iter()
            .skip(index + 1)
            .any(|a| a.descriptor().language == request.language)
        {
            return Err(AnalyzerError::Internal {
                message: "ambiguous analyzer candidates".to_string(),
            });
        }
        let result = self.analyzers[index].analyze(request)?;
        // 契約 F-14: 結果の source_revision が要求と一致しなければ
        // Consumer 側で stale として拒否する（ここでは PoC の
        // Consumer 役として検査する）。
        verify_result_revision(&result, request.source.revision)?;
        Ok(result)
    }
}

// ---- shim JSON → 契約型 ----

/// JSON object の `key` を u32 として読む。欠損/非数値/範囲外は 0。
fn json_u32(obj: &serde_json::Value, key: &str) -> u32 {
    obj.get(key)
        .and_then(serde_json::Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
        .unwrap_or(0)
}

fn position_from_json(v: &serde_json::Value) -> SourcePosition {
    SourcePosition {
        line: json_u32(v, "line"),
        column: json_u32(v, "column"),
    }
}

fn range_from_json(v: Option<&serde_json::Value>) -> Option<SourceRange> {
    let range = v?;
    Some(SourceRange {
        start: position_from_json(range.get("start")?),
        end: position_from_json(range.get("end")?),
    })
}

fn diagnostic_from_json(d: &serde_json::Value, revision: SourceRevision) -> Option<Diagnostic> {
    Some(Diagnostic {
        severity: match d.get("severity").and_then(serde_json::Value::as_str) {
            Some("warning") => DiagnosticSeverity::Warning,
            Some("information") => DiagnosticSeverity::Information,
            _ => DiagnosticSeverity::Error,
        },
        kind: match d.get("kind").and_then(serde_json::Value::as_str) {
            Some("syntax") => DiagnosticKind::Syntax,
            Some("type") => DiagnosticKind::Type,
            _ => DiagnosticKind::Other,
        },
        code: d
            .get("code")
            .map(|c| match c {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                _ => String::new(),
            })
            .unwrap_or_default(),
        message: d
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string(),
        range: range_from_json(d.get("range"))?,
        source_revision: revision,
    })
}

fn completion_from_json(c: &serde_json::Value, revision: SourceRevision) -> Option<Completion> {
    Some(Completion {
        label: c
            .get("label")
            .and_then(serde_json::Value::as_str)?
            .to_string(),
        kind: match c.get("kind").and_then(serde_json::Value::as_str) {
            Some("property") => CompletionKind::Property,
            Some("binding") => CompletionKind::Binding,
            Some("keyword") => CompletionKind::Keyword,
            Some("string") => CompletionKind::String,
            Some("type") => CompletionKind::Type,
            Some("module") => CompletionKind::Module,
            Some("generated_function") => CompletionKind::GeneratedFunction,
            Some("require_path") => CompletionKind::RequirePath,
            Some("hot_comment") => CompletionKind::HotComment,
            _ => CompletionKind::Other,
        },
        detail: c
            .get("detail")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        insert_text: c
            .get("insert_text")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        replace_range: range_from_json(c.get("replace_range"))?,
        source_revision: revision,
    })
}

fn parse_analyze_json(
    json: &str,
    revision: SourceRevision,
    want_completions: bool,
) -> Result<AnalyzeResult, AnalyzerError> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| AnalyzerError::Internal {
            message: format!("shim JSON parse failed: {e}"),
        })?;

    let diagnostics = value
        .get("diagnostics")
        .and_then(serde_json::Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|d| diagnostic_from_json(d, revision))
                .collect()
        })
        .unwrap_or_default();

    let completions = if want_completions {
        value
            .get("completions")
            .and_then(serde_json::Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| completion_from_json(c, revision))
                    .collect()
            })
    } else {
        None
    };

    Ok(AnalyzeResult {
        source_revision: revision,
        diagnostics,
        completions,
    })
}

/// テスト・fixture 用: revision から決定的に導出する補助。
#[doc(hidden)]
#[must_use]
pub fn revision_of(n: u64) -> SourceRevision {
    SourceRevision(n)
}

/// `WantedCapabilities` の既定値ヘルパ（diagnostics=SyntaxAndType, completion=true）。
#[must_use]
pub fn default_wanted() -> WantedCapabilities {
    WantedCapabilities {
        diagnostics: Some(DiagnosticLevel::SyntaxAndType),
        completion: false,
    }
}

/// BTreeMap の再輸出（fixture で型定義テーブルを組み立てる用途）。
#[doc(hidden)]
pub type FixtureMap = BTreeMap<String, String>;
