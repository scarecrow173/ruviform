//! `LuauAnalyzer` — vendored Luau.Analysis (Luau 0.740) を狭小 C ABI
//! shim 経由で呼び、`ruviform_script` の中立契約 C5 `ScriptAnalyzer`
//! を実装する実 Luau analyzer backend（Issue #692 s2 / AC2 analyzer 側）。
//!
//! Analyzer の成否は実行 Grant を意味しない（契約 C5 — Runtime とは
//! 独立 Port。本 crate は `ScriptBackend`/`ScriptSession` を実装しない）。
//!
//! `ScriptAnalyzer::analyze` は `&self` のため、shim の可変状態
//! （Frontend + loaded type definitions）は `Mutex` で内部可変化する。
//! `BridgeHandle` の `NonNull` により `Send`/`Sync` は自動的に非実装 —
//! shim の owner-thread affinity（create を呼んだ thread のみ使用可）
//! を型で正直に表現し、`Box<dyn ScriptAnalyzer>`（Send/Sync 非要求）
//! として `AnalyzerRegistry` へ登録できる。

use std::collections::BTreeSet;
use std::sync::{Mutex, MutexGuard};
use std::thread::ThreadId;

use ruviform_script::{
    AnalyzeRequest, AnalyzeResult, AnalyzerCapabilities, AnalyzerDescriptor, AnalyzerError,
    AnalyzerId, CONTRACT_VERSION, Completion, CompletionKind, Diagnostic, DiagnosticLevel,
    DiagnosticSeverity, SOURCE_SCHEMA_VERSION, ScriptAnalyzer, ScriptBackendVersion,
    ScriptLanguageId, SourcePosition, SourceRange, TYPE_DEFINITION_SCHEMA_VERSION, TypeDefinition,
};

use crate::ffi::{BridgeHandle, CreateOptions, bridge_versions};

/// `LuauAnalyzer` のデフォルト `analyzer_id`。
pub const DEFAULT_ANALYZER_ID: &str = "luau-analysis";

/// この analyzer が実装する言語 ID（`"luau"`）。
#[must_use]
pub fn luau_language() -> ScriptLanguageId {
    ScriptLanguageId::new("luau")
}

/// `LuauAnalyzer` 生成オプション。
///
/// 既定値は製品向けの保守的な選択: strict mode 有効・completion は
/// opt-in（能力広告と一致）・ソースは Module として解析・
/// module time limit 5 秒。
#[derive(Debug, Clone)]
pub struct LuauAnalyzerOptions {
    /// strict モードで型検査するか（既定: `true`）。
    /// `false`（nonstrict）では型注釈違反が報告されない場合がある。
    pub strict: bool,
    /// Completion 能力を有効化するか（既定: `false`）。
    /// `false` では `capabilities.completion=false` を広告し、
    /// Completion 要求は `Unsupported{analyze.completion}` で
    /// fail-closed する（能力ゲート — FX-N-13 系）。
    pub enable_completion: bool,
    /// 受理する最大ソースバイト数（0 = 無制限、既定）。
    /// shim ABI は u32 のため `u32::MAX` 超過は connect 時に
    /// `InvalidRequest` で拒否する。
    pub max_source_bytes: u64,
    /// モジュール単位の型検査時間制限（秒、既定: 5.0、0 = 無制限）。
    /// 制限超過時は部分診断を成功結果として返さず
    /// [`AnalyzerError::Internal`] で fail-closed する。
    /// 負値・NaN は connect 時に [`AnalyzerError::InvalidRequest`]。
    pub module_time_limit_sec: f64,
    /// ソースを Module として扱うか（既定: `true`）。
    /// `false` で Script として解析する。
    pub source_type_module: bool,
}

impl Default for LuauAnalyzerOptions {
    fn default() -> Self {
        Self {
            strict: true,
            enable_completion: false,
            max_source_bytes: 0,
            module_time_limit_sec: 5.0,
            source_type_module: true,
        }
    }
}

/// shim が所有する可変状態。`Mutex` で包み `analyze(&self)` を実現する。
struct AnalyzerState {
    handle: BridgeHandle,
    /// ロード済み型定義の `package_name` 集合（request 同梱分の dedup 用。
    /// shim に unload 経路がないため analyzer 寿命中保持される）。
    loaded_definitions: BTreeSet<String>,
    /// shim owner-thread affinity: create を呼んだ thread の ID。
    owner_thread: ThreadId,
}

/// Luau.Analysis を FFI shim 経由で利用する実 Luau analyzer。
/// Runtime Backend なしで単独利用可能（契約 C5 の独立性）。
pub struct LuauAnalyzer {
    descriptor: AnalyzerDescriptor,
    state: Mutex<AnalyzerState>,
}

impl LuauAnalyzer {
    /// shim を初期化し version をネゴシエートする
    /// （`analyzer_id` = [`DEFAULT_ANALYZER_ID`]）。
    ///
    /// shim が報告する contract version が `ruviform_script::CONTRACT_VERSION`
    /// と一致しない場合、または `ABI`/`struct_size` ネゴシエーションが失敗した
    /// 場合は `AnalyzerError` で fail-closed（handle は生成されない）。
    ///
    /// # Errors
    ///
    /// [`AnalyzerError::VersionMismatch`]（version 不一致）、
    /// [`AnalyzerError::InvalidRequest`]（`max_source_bytes` が shim の
    /// u32 範囲を超過）、その他 shim/内部障害は対応する variant。
    pub fn connect(options: &LuauAnalyzerOptions) -> Result<Self, AnalyzerError> {
        Self::connect_inner(options, AnalyzerId::new(DEFAULT_ANALYZER_ID))
    }

    /// [`LuauAnalyzer::connect`] の named 版。同一言語の複数 instance を
    /// `AnalyzerRegistry` で区別するための `analyzer_id` を指定できる
    /// （registry の曖昧性を hint で解決する用途）。
    ///
    /// # Errors
    ///
    /// [`LuauAnalyzer::connect`] と同じ。
    pub fn connect_with_analyzer_id(
        options: &LuauAnalyzerOptions,
        analyzer_id: AnalyzerId,
    ) -> Result<Self, AnalyzerError> {
        Self::connect_inner(options, analyzer_id)
    }

    fn connect_inner(
        options: &LuauAnalyzerOptions,
        analyzer_id: AnalyzerId,
    ) -> Result<Self, AnalyzerError> {
        // shim ABI は u32 — 公開 options の u64 を fail-closed で検証変換。
        let max_source_bytes =
            u32::try_from(options.max_source_bytes).map_err(|_| AnalyzerError::InvalidRequest {
                reason: format!(
                    "max_source_bytes {} exceeds shim u32 range",
                    options.max_source_bytes
                ),
            })?;

        // 負値/NaN は shim では「無制限」として黙って扱われる —
        // 意図しない無制限化を防ぐため公開 options 境界で拒否する
        // （0 = 無制限は仕様どおり許容）。
        if options.module_time_limit_sec.is_nan() || options.module_time_limit_sec < 0.0 {
            return Err(AnalyzerError::InvalidRequest {
                reason: "module_time_limit_sec must be >= 0 (0 = unlimited)".to_string(),
            });
        }

        let versions = bridge_versions();
        // shim が公開する契約 version が本 crate の実装する契約 version と
        // 一致するかを接続時に検査する（公開定数は contract.rs と同期）。
        if versions.contract_version != CONTRACT_VERSION {
            return Err(AnalyzerError::VersionMismatch {
                field: "contract_version".to_string(),
                expected: CONTRACT_VERSION.to_string(),
                actual: versions.contract_version.to_string(),
            });
        }
        // impl_version/luau_version の空報告は shim の version 情報不全 —
        // descriptor に空 provenance を載せないよう fail-closed。
        if versions.impl_version.is_empty() || versions.luau_version.is_empty() {
            return Err(AnalyzerError::VersionMismatch {
                field: "impl_version".to_string(),
                expected: "non-empty shim version string".to_string(),
                actual: versions.impl_version.clone(),
            });
        }

        let handle = BridgeHandle::create(&CreateOptions {
            mode_strict: options.strict,
            enable_completion: options.enable_completion,
            max_source_bytes,
            module_time_limit_sec: options.module_time_limit_sec,
            source_type_module: options.source_type_module,
        })?;

        let capabilities = AnalyzerCapabilities {
            diagnostics: DiagnosticLevel::SyntaxAndType,
            completion: options.enable_completion,
            max_source_bytes: (options.max_source_bytes != 0).then_some(options.max_source_bytes),
            supported_languages: vec![luau_language()],
        };
        let descriptor = AnalyzerDescriptor {
            analyzer_id,
            language: luau_language(),
            version: ScriptBackendVersion::new(
                format!("{}+luau{}", versions.impl_version, versions.luau_version),
                CONTRACT_VERSION,
            ),
            capabilities,
        };
        Ok(Self {
            descriptor,
            state: Mutex::new(AnalyzerState {
                handle,
                loaded_definitions: BTreeSet::new(),
                owner_thread: std::thread::current().id(),
            }),
        })
    }

    /// 型定義を Analyzer のグローバル環境へ追加する
    /// （Completion 候補変化・型解決の永続化用）。
    /// 追加した定義は analyzer 寿命中保持され、以後の `analyze` 全てに
    /// 効く（shim に unload 経路がないため）。
    ///
    /// # Errors
    ///
    /// schema version 不一致は [`AnalyzerError::VersionMismatch`]、
    /// shim 失敗は対応する variant。
    pub fn add_type_definition(&self, def: &TypeDefinition) -> Result<(), AnalyzerError> {
        validate_type_definition(def)?;
        let mut state = self.lock_state()?;
        if state.owner_thread != std::thread::current().id() {
            return Err(AnalyzerError::InvalidRequest {
                reason: "analyzer used from a different thread than connect()".to_string(),
            });
        }
        state
            .handle
            .load_definition(&def.package_name, &def.source.text)?;
        state.loaded_definitions.insert(def.package_name.clone());
        Ok(())
    }

    fn lock_state(&self) -> Result<MutexGuard<'_, AnalyzerState>, AnalyzerError> {
        self.state.lock().map_err(|_| AnalyzerError::Internal {
            message: "analyzer state mutex poisoned".to_string(),
        })
    }
}

impl ScriptAnalyzer for LuauAnalyzer {
    fn descriptor(&self) -> &AnalyzerDescriptor {
        &self.descriptor
    }

    fn analyze(&self, request: &AnalyzeRequest) -> Result<AnalyzeResult, AnalyzerError> {
        // ---- 事前検証（shim 到達前に fail-closed） ----

        // 言語一致: request.language がこの analyzer の対象言語か。
        // （source.language は provenance であり、consumer が意図的に
        // 言語を再解釈する契約上の余地を残すためここでは検査しない。）
        if request.language != self.descriptor.language {
            return Err(AnalyzerError::InvalidRequest {
                reason: format!(
                    "language {} not supported by {}",
                    request.language, self.descriptor.language
                ),
            });
        }

        if request.source.schema_version != SOURCE_SCHEMA_VERSION {
            return Err(AnalyzerError::VersionMismatch {
                field: "source.schema_version".to_string(),
                expected: SOURCE_SCHEMA_VERSION.to_string(),
                actual: request.source.schema_version.to_string(),
            });
        }

        // wanted ⊆ advertised capabilities（契約 F-13 系 fail-closed。
        // AnalyzerRegistry::analyze も同じ検査を行うが、直接呼出し経路でも
        // 同じ不変条件を守る）。
        let supported = self.descriptor.capabilities.capability_ids();
        for wanted in request.wanted.capability_ids() {
            if !supported.contains(&wanted) {
                return Err(AnalyzerError::Unsupported { capability: wanted });
            }
        }

        // Completion は cursor 位置を必要とする — 要求されたのに
        // cursor がない request は不正として拒否する。
        if request.wanted.completion && request.cursor.is_none() {
            return Err(AnalyzerError::InvalidRequest {
                reason: "completion requested without cursor position".to_string(),
            });
        }

        // cursor.offset は UTF-8 バイトオフセット（SourcePosition 契約）。
        // shim は byte offset のみを使うため line/col の追加検証はしない。
        if let Some(cursor) = request.cursor {
            // u64→usize は 32-bit target で truncate し得るため try_from で
            // fail-closed に変換する（範囲外は InvalidRequest）。
            let Ok(offset) = usize::try_from(cursor.offset) else {
                return Err(AnalyzerError::InvalidRequest {
                    reason: "cursor offset out of range or not on char boundary".to_string(),
                });
            };
            if offset > request.source.text.len() || !request.source.text.is_char_boundary(offset) {
                return Err(AnalyzerError::InvalidRequest {
                    reason: "cursor offset out of range or not on char boundary".to_string(),
                });
            }
        }

        // request 同梱の型定義も schema 検証（shim 到達前）。
        if let Some(defs) = &request.type_definitions {
            for def in defs {
                validate_type_definition(def)?;
            }
        }

        // 広告済み max_source_bytes を shim 到達前に早期 reject
        // （shim 側の SOURCE_TOO_LARGE 検査は defense-in-depth として残る）。
        if let Some(max) = self.descriptor.capabilities.max_source_bytes
            && request.source.text.len() as u64 > max
        {
            return Err(AnalyzerError::InvalidRequest {
                reason: "source exceeds max_source_bytes".to_string(),
            });
        }

        // ---- shim 呼出し（Mutex で内部可変状態を保護） ----

        let mut state = self.lock_state()?;
        if state.owner_thread != std::thread::current().id() {
            return Err(AnalyzerError::InvalidRequest {
                reason: "analyzer used from a different thread than connect()".to_string(),
            });
        }

        // 要求同梱の型定義を未ロードのものだけ shim へロードする
        // （package_name dedup — shim に unload がないため初回のみ適用。
        // 再定義が必要な場合は add_type_definition を使う）。
        if let Some(defs) = &request.type_definitions {
            for def in defs {
                if state.loaded_definitions.contains(&def.package_name) {
                    continue;
                }
                state
                    .handle
                    .load_definition(&def.package_name, &def.source.text)?;
                state.loaded_definitions.insert(def.package_name.clone());
            }
        }

        // cursor は completion 要求時のみ shim へ渡す。shim は
        // cursor_offset >= 0 を completion 要求と解釈するため、
        // wanted.completion=false の request に cursor が付いていても
        // completion path（status 5 / autocomplete 実行）を起動させない。
        let cursor_offset = if request.wanted.completion {
            request
                .cursor
                .map_or(-1, |c| i64::try_from(c.offset).unwrap_or(i64::MAX))
        } else {
            -1
        };
        let json = state
            .handle
            .analyze(request.source.text.as_bytes(), cursor_offset)?;
        drop(state);

        parse_analyze_json(&json, request, request.wanted.completion)
    }
}

/// `TypeDefinition` の schema version を fail-closed 検証する。
fn validate_type_definition(def: &TypeDefinition) -> Result<(), AnalyzerError> {
    if def.schema_version != TYPE_DEFINITION_SCHEMA_VERSION {
        return Err(AnalyzerError::VersionMismatch {
            field: "type_definitions.schema_version".to_string(),
            expected: TYPE_DEFINITION_SCHEMA_VERSION.to_string(),
            actual: def.schema_version.to_string(),
        });
    }
    if def.source.schema_version != SOURCE_SCHEMA_VERSION {
        return Err(AnalyzerError::VersionMismatch {
            field: "type_definitions.source.schema_version".to_string(),
            expected: SOURCE_SCHEMA_VERSION.to_string(),
            actual: def.source.schema_version.to_string(),
        });
    }
    Ok(())
}

// ---- shim JSON → 中立契約型 ----
//
// shim の JSON schema（native/rf_luau_bridge.cpp）:
//   diagnostics[]: {severity:"error",kind:"syntax"|"type",code:"parse"|<num>,
//                   message, module, range:{start:{line,column},end:{...}}}
//   completions[]: {label,kind,deprecated,detail,insert_text,
//                   replace_range:{start:{line,column},end:{...}}}
//   completion_context: <int>（中立契約に対応 field がないため破棄）
// shim の position は {line, column}（0 始まり、column は行内バイト
// オフセット — Luau Position 規約）。中立 `SourcePosition` は
// {offset, line, col} のため、offset は Rust 側で line/col から
// byte offset を再計算する（`position_offset` — shim の
// `offsetToPosition` の逆変換で改行意味論を一致させる）。

/// JSON object の `key` を u32 として読む。欠損/非数値/範囲外は 0。
fn json_u32(obj: &serde_json::Value, key: &str) -> u32 {
    obj.get(key)
        .and_then(serde_json::Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
        .unwrap_or(0)
}

/// shim `offsetToPosition` の逆変換: (line, col) → UTF-8 byte offset。
/// 改行意味論は Luau Lexer / shim と一致させる — 行区切りは `\n` のみ
/// （Lexer の `isNewline` は `'\n'` のみで `\r` は空白扱い。`\r\n` は
/// `\n` で 1 回だけ計数され、単独 `\r` は改行にならない）。
/// `line` がソースの行数を超える場合は `len` に clamp、
/// `col` が行末（次の `\n` または EOF）を超える場合も行末に clamp
/// する — `len` まで許すと後続行の offset を指す内部不整合な
/// (line, col, offset) 三つ組になり得るため。
fn position_offset(source: &str, line: u32, col: u32) -> u64 {
    let bytes = source.as_bytes();
    let mut cur_line = 0u32;
    let mut line_start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() && cur_line < line {
        if bytes[i] == b'\n' {
            cur_line += 1;
            line_start = i + 1;
        }
        i += 1;
    }
    if cur_line < line {
        return bytes.len() as u64;
    }
    let line_end = bytes[line_start..]
        .iter()
        .position(|&b| b == b'\n')
        .map_or(bytes.len(), |p| line_start + p);
    // col は行内バイト数。usize 加算は saturating で overflow を
    // fail-safe に処理する（32-bit target / 異常に大きな col）。
    // u32→usize は 32bit 以上の target で常に lossless。
    let offset = line_start.saturating_add(col as usize).min(line_end);
    u64::try_from(offset).unwrap_or(u64::MAX)
}

/// JSON の position object `{line, column}` → `SourcePosition`。
/// object でない値は `None`（present-but-garbage を (0,0,0) に
/// 偽装しない — range 全体を欠落扱いにする）。
fn position_from_json(source: &str, v: &serde_json::Value) -> Option<SourcePosition> {
    if !v.is_object() {
        return None;
    }
    let line = json_u32(v, "line");
    let col = json_u32(v, "column");
    Some(SourcePosition {
        offset: position_offset(source, line, col),
        line,
        col,
    })
}

fn range_from_json(source: &str, v: Option<&serde_json::Value>) -> Option<SourceRange> {
    let range = v?;
    if !range.is_object() {
        return None;
    }
    Some(SourceRange {
        start: position_from_json(source, range.get("start")?)?,
        end: position_from_json(source, range.get("end")?)?,
    })
}

fn diagnostic_from_json(source: &str, d: &serde_json::Value) -> Option<Diagnostic> {
    // 非 object エントリは shim の契約違反 — None を返し、呼出し側
    // （parse_analyze_json）が Internal に写像して fail-visible にする。
    if !d.is_object() {
        return None;
    }
    Some(Diagnostic {
        severity: match d.get("severity").and_then(serde_json::Value::as_str) {
            Some("warning") => DiagnosticSeverity::Warning,
            Some("information") => DiagnosticSeverity::Info,
            Some("hint") => DiagnosticSeverity::Hint,
            // "error" および未知 severity は Error に寄せる（fail-visible）。
            _ => DiagnosticSeverity::Error,
        },
        message: d
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string(),
        range: range_from_json(source, d.get("range")),
        code: d.get("code").and_then(|c| match c {
            serde_json::Value::String(s) if !s.is_empty() => Some(s.clone()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            _ => None,
        }),
    })
}

fn completion_from_json(source: &str, c: &serde_json::Value) -> Option<Completion> {
    if !c.is_object() {
        return None;
    }
    // label は必須 — 欠落 candidate は契約上表示不能なため skip。
    let label = c.get("label").and_then(serde_json::Value::as_str)?;
    Some(Completion {
        kind: match c.get("kind").and_then(serde_json::Value::as_str) {
            Some("property") => CompletionKind::Field,
            Some("binding") => CompletionKind::Text,
            Some("keyword") => CompletionKind::Keyword,
            // string/type/module/generated_function/require_path/
            // hot_comment/unknown/その他 → Other（中立契約の 5 variant
            // に対応しない shim kind はここへ集約 — spec 写像表どおり）。
            _ => CompletionKind::Other,
        },
        label: label.to_string(),
        insert_text: c
            .get("insert_text")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        replace_range: range_from_json(source, c.get("replace_range")),
        detail: c
            .get("detail")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
    })
}

fn parse_analyze_json(
    json: &str,
    request: &AnalyzeRequest,
    want_completion: bool,
) -> Result<AnalyzeResult, AnalyzerError> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| AnalyzerError::Internal {
            message: format!("shim returned malformed JSON: {e}"),
        })?;

    // トップレベル schema を fail-visible に検証する。parse 可能だが
    // object でない / "diagnostics" が array でない JSON を silent な
    // 「diagnostic 0 件」成功にしない（shim 回帰を検出可能にする）。
    let diagnostics_value = value
        .get("diagnostics")
        .filter(|v| v.is_array())
        .ok_or_else(|| AnalyzerError::Internal {
            message: "shim JSON missing diagnostics array".to_string(),
        })?;
    if want_completion
        && !value
            .get("completions")
            .is_some_and(serde_json::Value::is_array)
    {
        return Err(AnalyzerError::Internal {
            message: "shim JSON missing completions array".to_string(),
        });
    }

    let source_text = request.source.text.as_str();

    // diagnostics の各エントリは object 必須 — 非 object を skip すると
    // shim の契約違反が silent な診断欠落になるため Internal で
    // fail-visible にする（配列自体の型は上で検証済み）。
    let diagnostics = diagnostics_value
        .as_array()
        .expect("diagnostics validated as array")
        .iter()
        .map(|d| {
            diagnostic_from_json(source_text, d).ok_or_else(|| AnalyzerError::Internal {
                message: "shim JSON diagnostics entry is not an object".to_string(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    // completion 未要求時は shim が "completions":null を返すため
    // 空 Vec。要求時は上で array を検証済み。
    let completions = if want_completion {
        value
            .get("completions")
            .and_then(serde_json::Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| completion_from_json(source_text, c))
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    Ok(AnalyzeResult {
        source_revision: request.source.revision,
        diagnostics,
        completions,
    })
}

#[cfg(test)]
mod tests {
    //! `position_offset` の改行意味論と JSON→中立型写像の unit test。
    //! FFI を介さない純粋関数の検証（実 shim 結合は tests/ と
    //! `ffi_boundary_tests.rs` が担う）。

    use ruviform_script::ScriptSource;

    use super::*;

    #[test]
    fn position_offset_matches_shim_newline_semantics() {
        // "a\r\nb\rc\nd" — Luau Lexer / shim offsetToPosition と同一規則:
        // 行区切りは \n のみ（\r は Lexer では空白扱い。単独 \r は改行に
        // ならず、\r\n は \n で 1 回だけ計数）。
        // byte:      0='a' 1='\r' 2='\n' 3='b' 4='\r' 5='c' 6='\n' 7='d'
        // line starts: L0=0, L1=3, L2=7（L3 は存在しない）
        let src = "a\r\nb\rc\nd";
        assert_eq!(position_offset(src, 0, 0), 0);
        assert_eq!(position_offset(src, 1, 0), 3);
        assert_eq!(position_offset(src, 2, 0), 7);
        assert_eq!(position_offset(src, 1, 1), 4);
        assert_eq!(position_offset(src, 2, 1), 8);
        // 存在しない line → len clamp
        assert_eq!(position_offset(src, 3, 0), src.len() as u64);
        assert_eq!(position_offset(src, 99, 0), src.len() as u64);
        // col 超過 → 行末 clamp（L0="a\r" は byte 0..2 — len ではなく
        // 行末 2 を指す。後続行へはみ出さないことを確認）。
        assert_eq!(position_offset(src, 0, 999), 2);
    }

    #[test]
    fn position_offset_ignores_lone_cr_like_luau_lexer() {
        // 単独 \r は Luau Lexer では空白（isNewline は '\n' のみ）—
        // shim offsetToPosition / 本関数も同規則で改行に数えない。
        // これが崩れると lone-\r を含む source の diagnostic range が
        // 誤った byte offset を指す（Lexer 生成 Position との不整合）。
        let src = "a\rb\nc";
        // L0="a\rb"(byte 0..3), L1="c"(byte 4)
        assert_eq!(position_offset(src, 0, 0), 0);
        assert_eq!(position_offset(src, 1, 0), 4);
        assert_eq!(position_offset(src, 2, 0), src.len() as u64);
    }

    #[test]
    fn position_offset_lone_lf_and_crlf() {
        let src = "x\ny\r\nz";
        // L0="x"(0), L1="y"(2), L2="z"(5)
        assert_eq!(position_offset(src, 2, 0), 5);
        assert_eq!(position_offset(src, 1, 1), 3);
    }

    #[test]
    fn diagnostic_maps_unknown_severity_to_error() {
        let d = serde_json::json!({
            "severity": "bogus",
            "message": "m",
            "code": "parse",
            "range": {"start": {"line": 0, "column": 0}, "end": {"line": 0, "column": 1}}
        });
        let src = "ab";
        let diag = diagnostic_from_json(src, &d).expect("diagnostic");
        assert_eq!(diag.severity, DiagnosticSeverity::Error);
        assert_eq!(diag.code.as_deref(), Some("parse"));
        assert_eq!(diag.range.expect("range").start.offset, 0);
        assert_eq!(diag.range.expect("range").end.offset, 1);
    }

    #[test]
    fn completion_kind_mapping_and_required_label() {
        let c = serde_json::json!({
            "label": "foo",
            "kind": "property",
            "detail": "number",
            "insert_text": "foo()",
            "replace_range": {"start": {"line": 0, "column": 0}, "end": {"line": 0, "column": 3}}
        });
        let comp = completion_from_json("foo", &c).expect("completion");
        assert_eq!(comp.kind, CompletionKind::Field);
        assert_eq!(comp.detail.as_deref(), Some("number"));
        assert_eq!(comp.insert_text.as_deref(), Some("foo()"));
        assert!(comp.replace_range.is_some());

        // label 欠落 → skip
        let bad = serde_json::json!({"kind": "keyword"});
        assert!(completion_from_json("", &bad).is_none());

        // 未知 kind → Other
        let unk = serde_json::json!({"label": "x", "kind": "hot_comment"});
        assert_eq!(
            completion_from_json("", &unk).expect("completion").kind,
            CompletionKind::Other
        );
    }

    #[test]
    fn malformed_json_is_internal_not_panic() {
        let mut req = minimal_request();
        req.source = ScriptSource::new("x", luau_language(), None, req.source.revision);
        let err = parse_analyze_json("{not json", &req, false).expect_err("must fail");
        assert!(matches!(err, AnalyzerError::Internal { .. }));
    }

    #[test]
    fn non_object_or_missing_diagnostics_json_is_internal() {
        // parse 可能だが schema を満たさない JSON を silent な
        // 「diagnostic 0 件」成功にしない（fail-visible）。
        let req = minimal_request();
        for bad in [
            "null",
            "[]",
            "\"x\"",
            "42",
            "{}",
            "{\"diagnostics\": null}",
            "{\"diagnostics\": \"oops\"}",
            "{\"diagnostics\": [42, {}]}",
        ] {
            let err =
                parse_analyze_json(bad, &req, false).expect_err("schema-invalid JSON must fail");
            assert!(
                matches!(err, AnalyzerError::Internal { .. }),
                "input: {bad}"
            );
        }

        // 正常形は受理される（completions:null は未要求時のみ許容）。
        let ok = parse_analyze_json("{\"diagnostics\": [], \"completions\": null}", &req, false)
            .expect("valid JSON");
        assert_eq!(ok.diagnostics, Vec::new());
        assert_eq!(ok.completions, Vec::new());

        // completion 要求時に completions が array でない → Internal。
        let err = parse_analyze_json("{\"diagnostics\": [], \"completions\": null}", &req, true)
            .expect_err("missing completions array must fail");
        assert!(matches!(err, AnalyzerError::Internal { .. }));
    }

    fn minimal_request() -> AnalyzeRequest {
        AnalyzeRequest {
            source: ScriptSource::new(
                "",
                luau_language(),
                None,
                ruviform_script::SourceRevision::new(0),
            ),
            language: luau_language(),
            type_definitions: None,
            wanted: ruviform_script::WantedCapabilities {
                diagnostics: DiagnosticLevel::SyntaxAndType,
                completion: false,
            },
            cursor: None,
        }
    }
}
