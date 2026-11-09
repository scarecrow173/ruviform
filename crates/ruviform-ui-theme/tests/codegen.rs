//! Issue #418 theme-codegen の検証:
//! - 同一入力の繰り返し生成が byte-identical（決定性）
//! - commit 済み `src/generated.rs` が最新（`--check` の鮮度検査）
//! - 生成 `Theme::resolved_theme()` が実行時 `ThemeResolver::resolve` と一致
//! - 不正 manifest / resolve 失敗で非 0 終了し部分出力を残さない

use std::path::{Path, PathBuf};
use std::process::Command;

use ruviform_ui_theme::generated::{audiolens_dark, rvf_primitive, synvelis_dark};
use ruviform_ui_theme::{LengthUnit, ResolvedTheme, ThemeResolver, TokenDocument};

const CRATE_ROOT: &str = env!("CARGO_MANIFEST_DIR");
const MANIFEST: &str = "codegen/themes.json";
const GENERATED: &str = "src/generated.rs";

fn codegen_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_theme-codegen"))
}

fn manifest_path() -> PathBuf {
    Path::new(CRATE_ROOT).join(MANIFEST)
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ruviform-ui-theme-codegen-{}-{tag}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn run_codegen(args: &[&str]) -> std::process::Output {
    codegen_bin()
        .args(args)
        .current_dir(CRATE_ROOT)
        .output()
        .expect("spawn theme-codegen")
}

#[test]
fn generation_is_byte_identical_across_runs() {
    let dir = temp_dir("determinism");
    let first = dir.join("first.rs");
    let second = dir.join("second.rs");
    let manifest = manifest_path();
    let manifest_arg = manifest.to_str().expect("manifest path is UTF-8");

    for out in [&first, &second] {
        let out_arg = out.to_str().expect("out path is UTF-8");
        let output = run_codegen(&["--manifest", manifest_arg, "--out", out_arg]);
        assert!(
            output.status.success(),
            "theme-codegen failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert_eq!(
        std::fs::read(&first).expect("read first output"),
        std::fs::read(&second).expect("read second output"),
        "同一入力の繰り返し生成が byte-identical でない"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn committed_generated_rs_is_fresh() {
    let output = run_codegen(&["--manifest", MANIFEST, "--out", GENERATED, "--check"]);
    assert!(
        output.status.success(),
        "src/generated.rs が古いか不一致: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn generated_themes_match_runtime_resolve() {
    // manifest の document を実行時に resolve し、生成 Theme の
    // ResolvedTheme snapshot と normalized JSON で比較する。
    let manifest_text = std::fs::read_to_string(manifest_path()).expect("read codegen manifest");
    let manifest: serde_json::Value =
        serde_json::from_str(&manifest_text).expect("manifest is valid JSON");
    let themes = manifest["themes"].as_array().expect("themes array");
    assert_eq!(
        themes.len(),
        3,
        "test は現行 manifest の 3 theme を対象とする"
    );

    for theme_value in themes {
        let name = theme_value["name"].as_str().expect("theme name");
        let mode = theme_value["mode"].as_str().expect("theme mode");
        let docs = theme_value["documents"]
            .as_array()
            .expect("documents array");
        let mut documents = Vec::with_capacity(docs.len());
        for doc in docs {
            let rel = doc.as_str().expect("document path");
            // manifest は codegen/ 配下、document path は manifest dir 相対。
            let path = Path::new(CRATE_ROOT).join("codegen").join(rel);
            let text =
                std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
            documents.push(TokenDocument::from_json(&text).expect("parse document"));
        }
        let runtime = ThemeResolver::resolve(&documents, mode).expect("runtime resolve");

        let generated: ResolvedTheme = match name {
            "audiolens-dark" => audiolens_dark::Theme::new().into_resolved_theme(),
            "synvelis-dark" => synvelis_dark::Theme::new().into_resolved_theme(),
            "rvf-primitive" => rvf_primitive::Theme::new().into_resolved_theme(),
            other => panic!("test does not know generated module for theme `{other}`"),
        };
        assert_eq!(
            generated.to_normalized_json(),
            runtime.to_normalized_json(),
            "theme `{name}`: generated ResolvedTheme != runtime resolve"
        );
    }
}

#[test]
fn generated_accessors_return_typed_values() {
    // audiolens-dark: color + dimension token を持つ product fixture。
    let theme = audiolens_dark::Theme::new();
    assert_eq!(audiolens_dark::Theme::MODE, "dark");
    // accessor は ResolvedTheme の typed getter と一致する。
    let bg = theme.surface_bg();
    assert_eq!(theme.resolved_theme().color("surface.bg"), Some(bg));
    let space_sm = theme.space_sm();
    assert_eq!(space_sm.value, 4.0);
    assert_eq!(space_sm.unit, LengthUnit::Px);
    assert_eq!(theme.resolved_theme().length("space.sm"), Some(space_sm));

    // rvf-primitive: 全 8 ResolvedValue variant を含む fixture。
    // accessor の戻り値型が variant ごとに正しいことを確認する。
    let primitive = rvf_primitive::Theme::new();
    let _font_family: &[String] = primitive.font_family();
    let _weight: u16 = primitive.font_weight();
    let _number: f64 = primitive.scale_factor();
    let _duration = primitive.motion_fast();
    let _bezier = primitive.motion_ease();
    let _stroke = primitive.stroke_border();
    let _ = rvf_primitive::Theme::default();
}

#[test]
fn invalid_manifest_fails_without_partial_output() {
    let dir = temp_dir("invalid");
    let bad_manifest = dir.join("bad.json");
    std::fs::write(&bad_manifest, "{ not json").expect("write bad manifest");
    let out = dir.join("out.rs");
    let manifest_arg = bad_manifest.to_str().expect("manifest path is UTF-8");
    let out_arg = out.to_str().expect("out path is UTF-8");

    let output = run_codegen(&["--manifest", manifest_arg, "--out", out_arg]);
    assert!(!output.status.success(), "invalid manifest must fail");
    assert!(!out.exists(), "失敗時に部分出力ファイルを残してはいけない");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn unresolvable_document_fails() {
    let dir = temp_dir("unresolvable");
    let manifest = dir.join("manifest.json");
    // 存在しない document を指す manifest。
    std::fs::write(
        &manifest,
        r#"{"themes":[{"name":"ghost","mode":"dark","documents":["missing.json"]}]}"#,
    )
    .expect("write manifest");
    let out = dir.join("out.rs");
    let manifest_arg = manifest.to_str().expect("manifest path is UTF-8");
    let out_arg = out.to_str().expect("out path is UTF-8");

    let output = run_codegen(&["--manifest", manifest_arg, "--out", out_arg]);
    assert!(!output.status.success(), "missing document must fail");
    assert!(!out.exists());
    let _ = std::fs::remove_dir_all(&dir);
}
