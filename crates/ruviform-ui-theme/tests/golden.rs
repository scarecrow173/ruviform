//! Golden normalized-output and error-classification fixtures (Issue #385).
//!
//! Success cases compare `ResolvedTheme::to_normalized_json()` byte-for-byte
//! against `tests/golden/*.json`. Invalid cases assert the classified error.

use ruviform_ui_theme::{ResolveError, ScaleContext, ThemeResolver, TokenDocument, TokenError};

fn fixture(name: &str) -> TokenDocument {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let json = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
    TokenDocument::from_json(&json)
        .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"))
}

fn stack() -> Vec<TokenDocument> {
    vec![
        fixture("primitive.json"),
        fixture("semantic-modeless.json"),
        fixture("component.json"),
    ]
}

fn assert_golden(actual: &str, name: &str) {
    let path = format!("{}/tests/golden/{name}.json", env!("CARGO_MANIFEST_DIR"));
    let expected =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
    assert_eq!(format!("{actual}\n"), expected, "golden mismatch: {name}");
}

#[test]
fn valid_dark_matches_golden() {
    let theme = ThemeResolver::resolve(&stack(), "dark").unwrap();
    assert_golden(&theme.to_normalized_json(), "valid-dark");
}

#[test]
fn consumer_override_dark_matches_golden() {
    let mut docs = stack();
    docs.push(fixture("product-override.json"));
    let theme = ThemeResolver::resolve(&docs, "dark").unwrap();
    assert_golden(&theme.to_normalized_json(), "consumer-override-dark");
}

#[test]
fn os_font_missing_keeps_full_fallback_chain_golden() {
    let mut docs = stack();
    docs.push(fixture("product-os-font.json"));
    let theme = ThemeResolver::resolve(&docs, "dark").unwrap();
    assert_eq!(
        theme.font_family("font.brand"),
        Some(
            [
                "Ruviform Brand Not Installed".to_owned(),
                "Inter".to_owned(),
                "sans-serif".to_owned()
            ]
            .as_slice()
        ),
    );
    assert_golden(&theme.to_normalized_json(), "os-font-fallback-dark");
}

#[test]
fn dpi_scale_2x_changes_only_scale_context_golden() {
    let base = ThemeResolver::resolve(&stack(), "dark").unwrap();
    let scaled = base.with_scale(ScaleContext {
        scale_factor: 2.0,
        rem_base_px: 16.0,
    });
    assert_eq!(scaled.scale().scale_factor, 2.0);
    assert_eq!(
        scaled.tokens().collect::<Vec<_>>(),
        base.tokens().collect::<Vec<_>>(),
    );
    assert_golden(&scaled.to_normalized_json(), "dpi-scale-2x-dark");
}

#[test]
fn invalid_value_is_document_error() {
    let path = format!(
        "{}/tests/fixtures/err-invalid-value.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let json = std::fs::read_to_string(path).unwrap();
    match TokenDocument::from_json(&json) {
        Err(TokenError::InvalidValue { .. }) => {}
        other => panic!("expected InvalidValue, got {other:?}"),
    }
}

#[test]
fn ref_cycle_is_classified() {
    let docs = vec![fixture("err-cycle.json")];
    assert!(matches!(
        ThemeResolver::resolve(&docs, "dark"),
        Err(ResolveError::AliasCycle { .. })
    ));
}

#[test]
fn type_mismatch_is_classified() {
    let mut docs = stack();
    docs.push(fixture("err-type-mismatch.json"));
    assert!(matches!(
        ThemeResolver::resolve(&docs, "dark"),
        Err(ResolveError::TypeMismatch { .. })
    ));
}

#[test]
fn missing_mode_is_classified() {
    let mut docs = stack();
    docs.push(fixture("product-light-only.json"));
    match ThemeResolver::resolve(&docs, "dark") {
        Err(ResolveError::MissingMode { document, mode }) => {
            assert_eq!(document, "consumer-light-only");
            assert_eq!(mode, "dark");
        }
        other => panic!("expected MissingMode, got {other:?}"),
    }
}

#[test]
fn output_is_independent_of_input_order_and_repetition() {
    let mut forward = stack();
    forward.push(fixture("product-override.json"));
    let mut reversed = forward.clone();
    reversed.reverse();
    let a = ThemeResolver::resolve(&forward, "dark")
        .unwrap()
        .to_normalized_json();
    let b = ThemeResolver::resolve(&reversed, "dark")
        .unwrap()
        .to_normalized_json();
    let c = ThemeResolver::resolve(&forward, "dark")
        .unwrap()
        .to_normalized_json();
    assert_eq!(a, b);
    assert_eq!(a, c);
}
