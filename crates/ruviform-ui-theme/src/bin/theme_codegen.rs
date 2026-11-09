//! `theme-codegen` — DTCG manifest 駆動の静的 Rust 型生成（Issue #418）。
//!
//! manifest JSON が宣言する各 theme について、`ThemeResolver::resolve` を
//! 生成時に 1 回実行し、結果の `ResolvedTheme` を Rust リテラルとして
//! `src/generated.rs` へ emit する。resolve 規則は再実装せず、lib の
//! 公開 API（`TokenDocument::from_json` + `ThemeResolver::resolve`）のみを
//! 使う。出力は rustfmt で正規化され、同一入力に対し byte-identical である
//! ことが `tests/codegen.rs` で検証される。
//!
//! 使い方:
//!
//! ```text
//! theme-codegen --manifest <path> --out <path> [--check]
//! ```
//!
//! - `--check`: 書き込まず生成テキストと既存 `--out` を byte 比較する
//!   （不一致・不存在は exit 1）。
//! - 失敗時は stderr にメッセージを出し exit 1。引数不正は usage を出し
//!   exit 2。rustfmt が PATH に無い場合も exit 1（mise 管理 toolchain
//!   経由での実行を前提とする）。

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use ruviform_ui_theme::{
    ColorSpace, DocumentLayer, DurationUnit, LengthUnit, ResolvedTheme, ResolvedValue, Rounding,
    StrokeStyle, ThemeResolver, TokenDocument,
};

const USAGE: &str = "usage: theme-codegen --manifest <path> --out <path> [--check]";

/// codegen が生成する `Theme` の private field・inherent method と衝突しない
/// ための予約識別子。token path がこれらへ sanitize された場合はエラー。
const RESERVED_IDENTS: [&str; 6] = [
    "resolved",
    "new",
    "default",
    "clone",
    "resolved_theme",
    "into_resolved_theme",
];

const RUST_KEYWORDS: [&str; 52] = [
    "as", "async", "await", "become", "box", "break", "const", "continue", "crate", "do", "dyn",
    "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl", "in", "let",
    "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "try", "type", "typeof",
    "unsafe", "unsized", "use", "virtual", "where", "while", "yield", "abstract",
];

struct Args {
    manifest: PathBuf,
    out: PathBuf,
    check: bool,
}

struct ThemeSpec {
    name: String,
    mode: String,
    documents: Vec<String>,
}

struct ResolvedModule {
    /// 出力 `pub mod` 名（sanitize 済み・一意）。
    module: String,
    /// manifest 宣言の theme 名・mode・document path（module doc 用）。
    name: String,
    mode: String,
    documents: Vec<String>,
    theme: ResolvedTheme,
}

struct TokenPlan<'a> {
    ident: String,
    path: String,
    value: &'a ResolvedValue,
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(args) => args,
        Err(message) => {
            eprintln!("theme-codegen: {message}");
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("theme-codegen: {message}");
            ExitCode::from(1)
        }
    }
}

fn parse_args() -> Result<Args, String> {
    let mut manifest = None;
    let mut out = None;
    let mut check = false;
    let mut iter = env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--manifest" => manifest = Some(PathBuf::from(next_value(&mut iter, "--manifest")?)),
            "--out" => out = Some(PathBuf::from(next_value(&mut iter, "--out")?)),
            "--check" => check = true,
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    let (Some(manifest), Some(out)) = (manifest, out) else {
        return Err("--manifest and --out are required".to_owned());
    };
    Ok(Args {
        manifest,
        out,
        check,
    })
}

fn next_value(iter: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    iter.next()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn run(args: &Args) -> Result<(), String> {
    let manifest_dir = args
        .manifest
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let specs = parse_manifest(&args.manifest)?;

    // module 名で sort・衝突検査を兼ねて収集する（emit 順は module 名昇順）。
    let mut modules: BTreeMap<String, ResolvedModule> = BTreeMap::new();
    for spec in &specs {
        let module = resolve_theme(spec, &manifest_dir)?;
        if modules.contains_key(&module.module) {
            return Err(format!(
                "theme `{}`: module name `{}` collides with another theme",
                module.name, module.module
            ));
        }
        modules.insert(module.module.clone(), module);
    }

    let module_refs: Vec<&ResolvedModule> = modules.values().collect();
    let generated = emit_source(&module_refs)?;
    let formatted = rustfmt(&generated)?;

    if args.check {
        let existing = fs::read(&args.out).map_err(|e| {
            if args.out.exists() {
                format!("cannot read {}: {e}", args.out.display())
            } else {
                format!(
                    "{} does not exist — run `mise run codegen-theme` to generate it",
                    args.out.display()
                )
            }
        })?;
        if existing == formatted.as_bytes() {
            println!("{}: up to date", args.out.display());
            return Ok(());
        }
        return Err(format!(
            "{}: generated output differs — run `mise run codegen-theme` to regenerate",
            args.out.display()
        ));
    }

    if let Some(parent) = args.out.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    }
    fs::write(&args.out, formatted.as_bytes())
        .map_err(|e| format!("cannot write {}: {e}", args.out.display()))?;
    println!("{}: wrote {} bytes", args.out.display(), formatted.len());
    Ok(())
}

fn parse_manifest(path: &Path) -> Result<Vec<ThemeSpec>, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("cannot read manifest {}: {e}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("manifest {} is not valid JSON: {e}", path.display()))?;
    let root = value
        .as_object()
        .ok_or_else(|| "manifest root must be a JSON object".to_owned())?;
    for key in root.keys() {
        if key != "themes" {
            return Err(format!("manifest has unknown key `{key}`"));
        }
    }
    let themes = root
        .get("themes")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "manifest `themes` must be an array".to_owned())?;
    if themes.is_empty() {
        return Err("manifest `themes` must be non-empty".to_owned());
    }
    let mut specs = Vec::with_capacity(themes.len());
    for (index, theme) in themes.iter().enumerate() {
        specs.push(parse_theme_spec(theme, index)?);
    }
    let mut seen = BTreeSet::new();
    for spec in &specs {
        if !seen.insert(spec.name.as_str()) {
            return Err(format!("duplicate theme name `{}`", spec.name));
        }
    }
    Ok(specs)
}

fn parse_theme_spec(value: &serde_json::Value, index: usize) -> Result<ThemeSpec, String> {
    let label = format!("manifest themes[{index}]");
    let obj = value
        .as_object()
        .ok_or_else(|| format!("{label} must be an object"))?;
    for key in obj.keys() {
        if !["name", "mode", "documents"].contains(&key.as_str()) {
            return Err(format!("{label} has unknown key `{key}`"));
        }
    }
    let name = required_string(obj, "name", &label)?;
    let mode = required_string(obj, "mode", &label)?;
    let documents_value = obj
        .get("documents")
        .ok_or_else(|| format!("{label} is missing `documents`"))?;
    let documents_list = documents_value
        .as_array()
        .ok_or_else(|| format!("{label} `documents` must be an array"))?;
    if documents_list.is_empty() {
        return Err(format!("{label} `documents` must be non-empty"));
    }
    let mut documents = Vec::with_capacity(documents_list.len());
    for (doc_index, doc) in documents_list.iter().enumerate() {
        let path = doc
            .as_str()
            .ok_or_else(|| format!("{label} documents[{doc_index}] must be a string"))?;
        if path.is_empty() {
            return Err(format!("{label} documents[{doc_index}] must be non-empty"));
        }
        documents.push(path.to_owned());
    }
    Ok(ThemeSpec {
        name,
        mode,
        documents,
    })
}

fn required_string(
    obj: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    label: &str,
) -> Result<String, String> {
    let value = obj
        .get(key)
        .ok_or_else(|| format!("{label} is missing `{key}`"))?;
    let text = value
        .as_str()
        .ok_or_else(|| format!("{label} `{key}` must be a string"))?;
    if text.is_empty() {
        return Err(format!("{label} `{key}` must be non-empty"));
    }
    Ok(text.to_owned())
}

fn resolve_theme(spec: &ThemeSpec, manifest_dir: &Path) -> Result<ResolvedModule, String> {
    let module = module_ident(&spec.name);
    let mut documents = Vec::with_capacity(spec.documents.len());
    for relative in &spec.documents {
        let path = manifest_dir.join(relative);
        let text = fs::read_to_string(&path).map_err(|e| {
            format!(
                "theme `{}`: cannot read document {}: {e}",
                spec.name,
                path.display()
            )
        })?;
        let document = TokenDocument::from_json(&text).map_err(|e| {
            format!(
                "theme `{}`: invalid document {}: {e}",
                spec.name,
                path.display()
            )
        })?;
        documents.push(document);
    }
    let theme = ThemeResolver::resolve(&documents, &spec.mode).map_err(|e| {
        format!(
            "theme `{}` (mode `{}`): resolve failed: {e}",
            spec.name, spec.mode
        )
    })?;
    Ok(ResolvedModule {
        module,
        name: spec.name.clone(),
        mode: spec.mode.clone(),
        documents: spec.documents.clone(),
        theme,
    })
}

fn emit_source(modules: &[&ResolvedModule]) -> Result<String, String> {
    let mut out = String::new();
    out.push_str(
        "//! Generated by `theme-codegen` — do not edit by hand.\n\
         //! Regenerate with `mise run codegen-theme`.\n\
         //! Verified byte-identical by `tests/codegen.rs` / `mise run codegen-theme-check`.\n\n",
    );
    for (index, module) in modules.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str(&emit_module(module)?);
    }
    Ok(out)
}

fn emit_module(module: &ResolvedModule) -> Result<String, String> {
    let theme = &module.theme;
    let tokens: Vec<(&str, &ResolvedValue)> = theme.tokens().collect();

    // token path → accessor/field 識別子の計画（衝突・予約語はここで検査）。
    let mut idents = BTreeMap::new();
    let mut plans = Vec::with_capacity(tokens.len());
    for &(path, value) in &tokens {
        let ident = field_ident(path);
        if RESERVED_IDENTS.contains(&ident.as_str()) {
            return Err(format!(
                "theme `{}`: token path `{path}` sanitizes to reserved identifier `{ident}`",
                module.name
            ));
        }
        if let Some(previous) = idents.insert(ident.clone(), path) {
            return Err(format!(
                "theme `{}`: token paths `{previous}` and `{path}` both sanitize to `{ident}`",
                module.name
            ));
        }
        plans.push(TokenPlan {
            ident,
            path: path.to_owned(),
            value,
        });
    }

    let mut out = String::new();
    // clippy::mixed_attributes_style を避けるため module doc は外側の
    // `///` のみで記述する（`pub mod` への `//!` 併用は禁止）。
    let _ = writeln!(
        out,
        "/// Generated theme `{}` (mode `{}`).\n\
         ///\n\
         /// `ThemeResolver::resolve` output materialized at generation time —\n\
         /// schema `{}`, mode `{}`, {} document(s): {}.\npub mod {} {{",
        module.name,
        module.mode,
        theme.schema_version(),
        theme.mode(),
        module.documents.len(),
        module
            .documents
            .iter()
            .map(|d| format!("`{d}`"))
            .collect::<Vec<_>>()
            .join(", "),
        module.module
    );
    let _ = writeln!(
        out,
        "    /// Resolved theme `{}` snapshot (mode `{}`).",
        module.name, module.mode
    );
    let _ = writeln!(out, "    pub struct Theme {{");
    for plan in &plans {
        let _ = writeln!(
            out,
            "        {}: {},",
            plan.ident,
            payload_type(plan.value)?
        );
    }
    let _ = writeln!(out, "        resolved: crate::ResolvedTheme,");
    let _ = writeln!(out, "    }}\n");

    let _ = writeln!(out, "    impl Theme {{");
    let _ = writeln!(out, "        /// Mode this theme was resolved for.");
    let _ = writeln!(
        out,
        "        pub const MODE: &str = {};",
        rust_string(&module.mode)
    );
    out.push('\n');
    emit_theme_new(&mut out, module, theme, &plans)?;

    let _ = writeln!(
        out,
        "        /// The underlying [`crate::ResolvedTheme`] snapshot.\n\
         \x20       #[must_use]\n        pub fn resolved_theme(&self) -> &crate::ResolvedTheme {{\n            &self.resolved\n        }}\n"
    );
    let _ = writeln!(
        out,
        "        /// Consume the wrapper and return the [`crate::ResolvedTheme`] snapshot.\n\
         \x20       #[must_use]\n        pub fn into_resolved_theme(self) -> crate::ResolvedTheme {{\n            self.resolved\n        }}\n"
    );
    emit_theme_accessors(&mut out, &plans)?;

    let _ = writeln!(out, "    }}\n");
    let _ = writeln!(out, "    impl Default for Theme {{");
    let _ = writeln!(out, "        fn default() -> Self {{");
    let _ = writeln!(out, "            Self::new()\n        }}\n    }}\n}}");
    Ok(out)
}

/// `Theme::new()` — typed field 初期化・token map・`ResolvedTheme` 構築。
fn emit_theme_new(
    out: &mut String,
    module: &ResolvedModule,
    theme: &ResolvedTheme,
    plans: &[TokenPlan<'_>],
) -> Result<(), String> {
    let _ = writeln!(
        out,
        "        /// Materialize the resolved theme from the generated literals.\n\
         \x20       ///\n\
         \x20       /// Token values are the `ThemeResolver::resolve` output captured at\n\
         \x20       /// generation time; use [`Self::resolved_theme`] for the underlying\n\
         \x20       /// [`crate::ResolvedTheme`] snapshot.\n\
         \x20       #[allow(clippy::too_many_lines)]\n\
         \x20       #[must_use]\n\
         \x20       pub fn new() -> Self {{"
    );
    for plan in plans {
        let _ = writeln!(
            out,
            "            let {} = {};",
            plan.ident,
            payload_expr(plan.value)?
        );
    }
    let _ = writeln!(
        out,
        "            let tokens: std::collections::BTreeMap<String, crate::ResolvedValue> = ["
    );
    for plan in plans {
        let _ = writeln!(
            out,
            "                ({}, {}),",
            owned_string(&plan.path),
            resolved_value_expr(plan)?
        );
    }
    let _ = writeln!(
        out,
        "            ]\n                .into_iter()\n                .collect();"
    );
    let _ = writeln!(out, "            let resolved = crate::ResolvedTheme::new(");
    let _ = writeln!(out, "                {},", owned_string(&module.mode));
    out.push_str("                vec![\n");
    for source in theme.sources() {
        let layer = document_layer(source.layer)?;
        let _ = writeln!(out, "                    crate::TokenSource {{");
        let _ = writeln!(out, "                        layer: {layer},");
        let _ = writeln!(
            out,
            "                        id: {},",
            owned_string(&source.id)
        );
        let _ = writeln!(
            out,
            "                        fingerprint: {},",
            group_digits(&source.fingerprint.to_string())
        );
        let _ = writeln!(out, "                    }},");
    }
    let _ = writeln!(
        out,
        "                ],\n                tokens,\n            );"
    );
    let _ = writeln!(out, "            Self {{");
    for plan in plans {
        let _ = writeln!(out, "                {},", plan.ident);
    }
    let _ = writeln!(
        out,
        "                resolved,\n            }}\n        }}\n"
    );
    Ok(())
}

/// token ごとの typed accessor（`self.<ident>` をそのまま返す）。
fn emit_theme_accessors(out: &mut String, plans: &[TokenPlan<'_>]) -> Result<(), String> {
    for plan in plans {
        let _ = writeln!(
            out,
            "        /// `{}` (`{}`).",
            plan.path,
            type_label(plan.value)?
        );
        let ret = accessor_return_type(plan.value)?;
        let body = if matches!(plan.value, ResolvedValue::FontFamily(_)) {
            format!("self.{}.as_slice()", plan.ident)
        } else {
            format!("self.{}", plan.ident)
        };
        let _ = writeln!(out, "        #[must_use]");
        let _ = writeln!(out, "        pub fn {}(&self) -> {ret} {{", plan.ident);
        let _ = writeln!(out, "            {body}\n        }}\n");
    }
    Ok(())
}

/// token path・theme 名 → Rust 識別子。ASCII 英数字を小文字化し、それ以外を
/// `_` に潰した `snake_case`。空は `token`、先頭数字は `n` 前置、keyword は
/// 末尾 `_` を付ける。常に成功する純粋関数（衝突検査は呼出側の責務）。
fn field_ident(path: &str) -> String {
    let mut out = String::new();
    let mut last_sep = false;
    for ch in path.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_sep = false;
        } else if !last_sep && !out.is_empty() {
            out.push('_');
            last_sep = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out.is_empty() {
        out.push_str("token");
    }
    if out.as_bytes()[0].is_ascii_digit() {
        out.insert(0, 'n');
    }
    if RUST_KEYWORDS.contains(&out.as_str()) {
        out.push('_');
    }
    out
}

/// theme 名 → 出力 `pub mod` 名。`field_ident` と同じ正規化規則。
fn module_ident(name: &str) -> String {
    field_ident(name)
}

/// Rust の文字列リテラル（`{:?}` の escape 済み・引用符付き）。
fn rust_string(text: &str) -> String {
    format!("{text:?}")
}

/// `String` 値を作る式（`String::from("...")`）。
fn owned_string(text: &str) -> String {
    format!("String::from({})", rust_string(text))
}

fn rust_f64(value: f64) -> String {
    group_digits(&format!("{value:?}"))
}

/// 数字ランが 5 桁以上のとき `_` を挿入する（`clippy::unreadable_literal` /
/// `clippy::inconsistent_digit_grouping` 対策。値は変わらない）。整数部・
/// 指数部は右から 3 桁区切り、小数部は clippy の canonical 形式に合わせて
/// 左から 3 桁区切り（端数は末尾）にする。
fn group_digits(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + text.len() / 3);
    let mut run = String::new();
    let mut fraction = false;
    for ch in text.chars() {
        match ch {
            '0'..='9' => run.push(ch),
            '.' => {
                flush_digit_run(&mut out, &mut run, fraction);
                fraction = true;
                out.push(ch);
            }
            _ => {
                flush_digit_run(&mut out, &mut run, fraction);
                fraction = false;
                out.push(ch);
            }
        }
    }
    flush_digit_run(&mut out, &mut run, fraction);
    out
}

fn flush_digit_run(out: &mut String, run: &mut String, fraction: bool) {
    if run.len() <= 4 {
        out.push_str(run);
    } else if fraction {
        // 小数部: 左から 3 桁区切り（端数は末尾）。
        for (i, ch) in run.chars().enumerate() {
            if i > 0 && i.is_multiple_of(3) {
                out.push('_');
            }
            out.push(ch);
        }
    } else {
        // 整数部・指数部: 右から 3 桁区切り。
        let chars: Vec<char> = run.chars().collect();
        let n = chars.len();
        for (i, ch) in chars.iter().enumerate() {
            if i > 0 && (n - i).is_multiple_of(3) {
                out.push('_');
            }
            out.push(*ch);
        }
    }
    run.clear();
}

/// `ResolvedValue` の payload 式（field の型そのもの）。
fn payload_expr(value: &ResolvedValue) -> Result<String, String> {
    Ok(match value {
        ResolvedValue::Color(c) => format!(
            "crate::ResolvedColor {{ space: {}, components: [{}, {}, {}], alpha: {} }}",
            color_space(c.space)?,
            rust_f64(c.components[0]),
            rust_f64(c.components[1]),
            rust_f64(c.components[2]),
            rust_f64(c.alpha)
        ),
        ResolvedValue::Length(l) => format!(
            "crate::ResolvedLength {{ value: {}, unit: {}, rounding: {} }}",
            rust_f64(l.value),
            length_unit(l.unit)?,
            rounding(l.rounding)?
        ),
        ResolvedValue::FontFamily(families) => {
            let items = families
                .iter()
                .map(|f| owned_string(f))
                .collect::<Vec<_>>()
                .join(", ");
            format!("vec![{items}]")
        }
        ResolvedValue::FontWeight(w) => format!("{w}"),
        ResolvedValue::Number(n) => rust_f64(*n),
        ResolvedValue::Duration(d) => format!(
            "crate::ResolvedDuration {{ value: {}, unit: {} }}",
            rust_f64(d.value),
            duration_unit(d.unit)?
        ),
        ResolvedValue::CubicBezier(b) => format!(
            "[{}, {}, {}, {}]",
            rust_f64(b[0]),
            rust_f64(b[1]),
            rust_f64(b[2]),
            rust_f64(b[3])
        ),
        ResolvedValue::StrokeStyle(s) => stroke_style(*s)?.to_owned(),
        _ => return Err("unsupported ResolvedValue variant".to_owned()),
    })
}

/// token map 挿入用の `crate::ResolvedValue::Variant(...)` 式。
fn resolved_value_expr(plan: &TokenPlan<'_>) -> Result<String, String> {
    let (variant, argument) = match plan.value {
        ResolvedValue::Color(_) => ("Color", plan.ident.clone()),
        ResolvedValue::Length(_) => ("Length", plan.ident.clone()),
        ResolvedValue::FontFamily(_) => ("FontFamily", format!("{}.clone()", plan.ident)),
        ResolvedValue::FontWeight(_) => ("FontWeight", plan.ident.clone()),
        ResolvedValue::Number(_) => ("Number", plan.ident.clone()),
        ResolvedValue::Duration(_) => ("Duration", plan.ident.clone()),
        ResolvedValue::CubicBezier(_) => ("CubicBezier", plan.ident.clone()),
        ResolvedValue::StrokeStyle(_) => ("StrokeStyle", plan.ident.clone()),
        _ => return Err("unsupported ResolvedValue variant".to_owned()),
    };
    Ok(format!("crate::ResolvedValue::{variant}({argument})"))
}

fn payload_type(value: &ResolvedValue) -> Result<&'static str, String> {
    Ok(match value {
        ResolvedValue::Color(_) => "crate::ResolvedColor",
        ResolvedValue::Length(_) => "crate::ResolvedLength",
        ResolvedValue::FontFamily(_) => "Vec<String>",
        ResolvedValue::FontWeight(_) => "u16",
        ResolvedValue::Number(_) => "f64",
        ResolvedValue::Duration(_) => "crate::ResolvedDuration",
        ResolvedValue::CubicBezier(_) => "[f64; 4]",
        ResolvedValue::StrokeStyle(_) => "crate::StrokeStyle",
        _ => return Err("unsupported ResolvedValue variant".to_owned()),
    })
}

fn accessor_return_type(value: &ResolvedValue) -> Result<String, String> {
    if matches!(value, ResolvedValue::FontFamily(_)) {
        return Ok("&[String]".to_owned());
    }
    payload_type(value).map(str::to_owned)
}

fn type_label(value: &ResolvedValue) -> Result<&'static str, String> {
    Ok(match value {
        ResolvedValue::Color(_) => "color",
        ResolvedValue::Length(_) => "dimension",
        ResolvedValue::FontFamily(_) => "fontFamily",
        ResolvedValue::FontWeight(_) => "fontWeight",
        ResolvedValue::Number(_) => "number",
        ResolvedValue::Duration(_) => "duration",
        ResolvedValue::CubicBezier(_) => "cubicBezier",
        ResolvedValue::StrokeStyle(_) => "strokeStyle",
        _ => return Err("unsupported ResolvedValue variant".to_owned()),
    })
}

fn color_space(space: ColorSpace) -> Result<&'static str, String> {
    Ok(match space {
        ColorSpace::Srgb => "crate::ColorSpace::Srgb",
        _ => return Err("unsupported ColorSpace variant".to_owned()),
    })
}

fn length_unit(unit: LengthUnit) -> Result<&'static str, String> {
    Ok(match unit {
        LengthUnit::Px => "crate::LengthUnit::Px",
        LengthUnit::Rem => "crate::LengthUnit::Rem",
        _ => return Err("unsupported LengthUnit variant".to_owned()),
    })
}

fn rounding(rounding: Rounding) -> Result<&'static str, String> {
    Ok(match rounding {
        Rounding::None => "crate::Rounding::None",
        Rounding::Nearest => "crate::Rounding::Nearest",
        Rounding::Floor => "crate::Rounding::Floor",
        Rounding::Ceil => "crate::Rounding::Ceil",
        _ => return Err("unsupported Rounding variant".to_owned()),
    })
}

fn duration_unit(unit: DurationUnit) -> Result<&'static str, String> {
    Ok(match unit {
        DurationUnit::Milliseconds => "crate::DurationUnit::Milliseconds",
        DurationUnit::Seconds => "crate::DurationUnit::Seconds",
        _ => return Err("unsupported DurationUnit variant".to_owned()),
    })
}

fn stroke_style(style: StrokeStyle) -> Result<&'static str, String> {
    Ok(match style {
        StrokeStyle::Solid => "crate::StrokeStyle::Solid",
        StrokeStyle::Dashed => "crate::StrokeStyle::Dashed",
        StrokeStyle::Dotted => "crate::StrokeStyle::Dotted",
        StrokeStyle::Double => "crate::StrokeStyle::Double",
        StrokeStyle::Groove => "crate::StrokeStyle::Groove",
        StrokeStyle::Ridge => "crate::StrokeStyle::Ridge",
        StrokeStyle::Outset => "crate::StrokeStyle::Outset",
        StrokeStyle::Inset => "crate::StrokeStyle::Inset",
        _ => return Err("unsupported StrokeStyle variant".to_owned()),
    })
}

fn document_layer(layer: DocumentLayer) -> Result<&'static str, String> {
    Ok(match layer {
        DocumentLayer::Primitive => "crate::DocumentLayer::Primitive",
        DocumentLayer::Semantic => "crate::DocumentLayer::Semantic",
        DocumentLayer::Component => "crate::DocumentLayer::Component",
        DocumentLayer::Product => "crate::DocumentLayer::Product",
        _ => return Err("unsupported DocumentLayer variant".to_owned()),
    })
}

/// 生成テキストを rustfmt で正規化する。rustfmt は mise pin toolchain の
/// component であり、未導入環境では明示的に失敗させる（決定性のため
/// フォールバックはしない）。
fn rustfmt(source: &str) -> Result<String, String> {
    let mut child = Command::new("rustfmt")
        .arg("--edition")
        .arg("2024")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot spawn `rustfmt` (run via mise toolchain): {e}"))?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| "rustfmt stdin unavailable".to_owned())?
        .write_all(source.as_bytes())
        .map_err(|e| format!("cannot write to rustfmt stdin: {e}"))?;
    let output = child
        .wait_with_output()
        .map_err(|e| format!("cannot wait for rustfmt: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "rustfmt failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|e| format!("rustfmt produced non-UTF-8 output: {e}"))
}
