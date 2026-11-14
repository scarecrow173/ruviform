//! Analyzer Registry/Port（契約設計 C5 の PoC ミラー）。
//!
//! Runtime registry とは独立した registry。language/backend を明示して
//! 解決し、未登録・曖昧・version 不一致は fail-closed。
//! `AnalyzeResult` は `ScriptValue`/診断のみを返し、vendor 型を出さない。

use std::collections::BTreeSet;

use crate::contract::{
    AnalyzerId, POC_CONTRACT_VERSION, RegistryError, ResolveError, ResolveErrorReason,
    ScriptLanguageId,
};
use crate::value::{DiagnosticSeverity, ScriptDiagnostic, ScriptError, ScriptSource};

/// Analyzer が広告する能力（`"analyze.parse"`, `"analyze.diagnostics"` 等）。
#[derive(Clone, Debug)]
pub struct AnalyzerCapabilities {
    pub capability_ids: BTreeSet<String>,
    pub contract_version: u32,
}

/// `AnalyzerDescriptor`（契約設計 C5）。
#[derive(Clone, Debug)]
pub struct AnalyzerDescriptor {
    pub id: AnalyzerId,
    pub language: ScriptLanguageId,
    pub impl_version: String,
    pub capabilities: AnalyzerCapabilities,
}

/// `AnalyzeResult`（契約設計 C5）。diagnostics は source_revision を
/// consumer 側で保持するためここには含めない。
#[derive(Clone, Debug)]
pub struct AnalyzeResult {
    pub diagnostics: Vec<ScriptDiagnostic>,
    pub capabilities_used: BTreeSet<String>,
}

/// Analyzer 実装 trait。vendor 型を出さない。
pub trait ScriptAnalyzer {
    fn descriptor(&self) -> &AnalyzerDescriptor;
    /// source を解析し診断を返す。評価は行わない。
    fn analyze(&self, source: &ScriptSource) -> Result<AnalyzeResult, ScriptError>;
}

/// `resolve` の入力要件。
#[derive(Clone, Debug, Default)]
pub struct AnalyzerRequirement {
    pub language: Option<ScriptLanguageId>,
    pub analyzer: Option<AnalyzerId>,
    pub required_capabilities: BTreeSet<String>,
    pub min_contract_version: Option<u32>,
}

/// プロセス内 Analyzer 登録・実行時解決（契約設計 C5）。
pub struct AnalyzerRegistry {
    analyzers: Vec<Box<dyn ScriptAnalyzer>>,
}

impl Default for AnalyzerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalyzerRegistry {
    pub fn new() -> Self {
        Self {
            analyzers: Vec::new(),
        }
    }

    pub fn register(&mut self, analyzer: Box<dyn ScriptAnalyzer>) -> Result<(), RegistryError> {
        let id = analyzer.descriptor().id.clone();
        // RegistryError::DuplicateBackend は backend 用だが、analyzer も
        // 同じ「重複 ID 拒否」規則を使う（ID 型は別だが PoC では共用）。
        if self.analyzers.iter().any(|a| a.descriptor().id == id) {
            return Err(RegistryError::DuplicateBackend(
                crate::contract::ScriptBackendId::new(id.as_str()),
            ));
        }
        self.analyzers.push(analyzer);
        Ok(())
    }

    pub fn resolve(&self, req: &AnalyzerRequirement) -> Result<&dyn ScriptAnalyzer, ResolveError> {
        if let Some(lang) = &req.language
            && !self
                .analyzers
                .iter()
                .any(|a| a.descriptor().language == *lang)
        {
            return Err(ResolveError::NoMatchingBackend {
                reason: ResolveErrorReason::UnknownLanguage,
                detail: format!("unknown language: {lang}"),
            });
        }
        let mut candidates: Vec<&dyn ScriptAnalyzer> = self
            .analyzers
            .iter()
            .map(|a| a.as_ref())
            .filter(|a| {
                req.language
                    .as_ref()
                    .is_none_or(|l| a.descriptor().language == *l)
                    && req
                        .analyzer
                        .as_ref()
                        .is_none_or(|id| a.descriptor().id == *id)
            })
            .filter(|a| {
                let d = a.descriptor();
                d.capabilities.contract_version == POC_CONTRACT_VERSION
                    && req
                        .min_contract_version
                        .is_none_or(|m| d.capabilities.contract_version >= m)
            })
            .filter(|a| {
                let caps = &a.descriptor().capabilities.capability_ids;
                req.required_capabilities.iter().all(|c| caps.contains(c))
            })
            .collect();

        match candidates.len() {
            0 => Err(ResolveError::NoMatchingBackend {
                reason: ResolveErrorReason::MissingCapability,
                detail: "no analyzer satisfies requirements".to_owned(),
            }),
            1 => Ok(candidates.remove(0)),
            _ => Err(ResolveError::AmbiguousBackend {
                candidates: candidates
                    .iter()
                    .map(|a| crate::contract::ScriptBackendId::new(a.descriptor().id.as_str()))
                    .collect(),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// 実装: Luau / Rhai の syntax-only analyzer（評価しない）
// ---------------------------------------------------------------------------

fn diag(
    severity: DiagnosticSeverity,
    code: &str,
    message: String,
    line: Option<u32>,
) -> ScriptDiagnostic {
    ScriptDiagnostic {
        severity,
        code: code.to_owned(),
        message,
        range: line.map(|l| crate::value::SourceRange {
            start: u64::from(l),
            end: u64::from(l),
        }),
    }
}

fn caps(ids: &[&str]) -> AnalyzerCapabilities {
    AnalyzerCapabilities {
        capability_ids: ids.iter().map(|s| (*s).to_owned()).collect(),
        contract_version: POC_CONTRACT_VERSION,
    }
}

/// Luau syntax analyzer。`Chunk::into_function` で compile のみ行い
/// 実行しない（mlua/lua_load 経路）。
pub struct LuauAnalyzer {
    descriptor: AnalyzerDescriptor,
}

impl LuauAnalyzer {
    pub fn new() -> Self {
        Self {
            descriptor: AnalyzerDescriptor {
                id: AnalyzerId::new("luau-mlua-analyzer"),
                language: ScriptLanguageId::new("luau"),
                impl_version: env!("CARGO_PKG_VERSION").to_owned(),
                capabilities: caps(&["analyze.parse", "analyze.diagnostics"]),
            },
        }
    }
}

impl Default for LuauAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptAnalyzer for LuauAnalyzer {
    fn descriptor(&self) -> &AnalyzerDescriptor {
        &self.descriptor
    }

    fn analyze(&self, source: &ScriptSource) -> Result<AnalyzeResult, ScriptError> {
        crate::contract::ensure_non_rt()?;
        let lua = mlua::Lua::new();
        let chunk = lua.load(source.text.as_str()).set_name("<poc-analyze>");
        match chunk.into_function() {
            Ok(_) => Ok(AnalyzeResult {
                diagnostics: vec![diag(
                    DiagnosticSeverity::Info,
                    "parse.ok",
                    "luau source compiled".to_owned(),
                    None,
                )],
                capabilities_used: ["analyze.parse".to_owned()].into_iter().collect(),
            }),
            Err(e) => {
                let line = e.to_string().lines().next().and_then(|l| {
                    // mlua error 文字列から行番号を拾う（best-effort）。
                    l.split(':')
                        .nth(1)
                        .and_then(|n| n.trim().parse::<u32>().ok())
                });
                Ok(AnalyzeResult {
                    diagnostics: vec![diag(
                        DiagnosticSeverity::Error,
                        "parse.error",
                        e.to_string(),
                        line,
                    )],
                    capabilities_used: ["analyze.parse".to_owned()].into_iter().collect(),
                })
            }
        }
    }
}

/// Rhai syntax analyzer。`Engine::compile` で AST 化のみ（eval しない）。
pub struct RhaiAnalyzer {
    descriptor: AnalyzerDescriptor,
}

impl RhaiAnalyzer {
    pub fn new() -> Self {
        Self {
            descriptor: AnalyzerDescriptor {
                id: AnalyzerId::new("rhai-analyzer"),
                language: ScriptLanguageId::new("rhai"),
                impl_version: env!("CARGO_PKG_VERSION").to_owned(),
                capabilities: caps(&["analyze.parse", "analyze.diagnostics"]),
            },
        }
    }
}

impl Default for RhaiAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptAnalyzer for RhaiAnalyzer {
    fn descriptor(&self) -> &AnalyzerDescriptor {
        &self.descriptor
    }

    fn analyze(&self, source: &ScriptSource) -> Result<AnalyzeResult, ScriptError> {
        crate::contract::ensure_non_rt()?;
        let engine = rhai::Engine::new_raw();
        match engine.compile(&source.text) {
            Ok(_) => Ok(AnalyzeResult {
                diagnostics: vec![diag(
                    DiagnosticSeverity::Info,
                    "parse.ok",
                    "rhai source compiled".to_owned(),
                    None,
                )],
                capabilities_used: ["analyze.parse".to_owned()].into_iter().collect(),
            }),
            Err(e) => {
                // ParseError は Position を持つ。
                let pos = e.position();
                Ok(AnalyzeResult {
                    diagnostics: vec![diag(
                        DiagnosticSeverity::Error,
                        "parse.error",
                        e.to_string(),
                        pos.line().map(|l| l as u32),
                    )],
                    capabilities_used: ["analyze.parse".to_owned()].into_iter().collect(),
                })
            }
        }
    }
}
