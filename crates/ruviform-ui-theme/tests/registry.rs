//! Theme Registry contract (Issue #385): named product themes share the
//! same resolver and differ only by their document stacks.

use ruviform_ui_theme::{ResolveError, ThemeRegistry, ThemeResolver, TokenDocument};

fn fixture(name: &str) -> TokenDocument {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let json = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
    TokenDocument::from_json(&json).unwrap_or_else(|e| panic!("fixture {name}: {e}"))
}

fn base_stack() -> Vec<TokenDocument> {
    vec![
        fixture("primitive.json"),
        fixture("semantic-modeless.json"),
        fixture("component.json"),
    ]
}

#[test]
fn registry_matches_direct_resolver_output() {
    let mut registry = ThemeRegistry::new();
    registry.register("base", base_stack());
    let via_registry = registry.resolve("base", "dark").unwrap();
    let direct = ThemeResolver::resolve(&base_stack(), "dark").unwrap();
    assert_eq!(
        via_registry.to_normalized_json(),
        direct.to_normalized_json()
    );
}

#[test]
fn product_theme_override_is_isolated_to_its_own_stack() {
    let mut consumer = base_stack();
    consumer.push(fixture("product-override.json"));

    let mut registry = ThemeRegistry::new();
    registry.register("base", base_stack());
    registry.register("consumer", consumer);

    let base = registry.resolve("base", "dark").unwrap();
    let override_ = registry.resolve("consumer", "dark").unwrap();
    assert_ne!(
        base.color("color.primary"),
        override_.color("color.primary")
    );
    assert_eq!(base.color("color.muted"), override_.color("color.muted"));
}

#[test]
fn unknown_theme_is_error() {
    let registry = ThemeRegistry::new();
    match registry.resolve("missing", "dark") {
        Err(ResolveError::UnknownTheme { name }) => assert_eq!(name, "missing"),
        other => panic!("expected UnknownTheme, got {other:?}"),
    }
}

#[test]
fn registered_stack_errors_surface_unchanged() {
    let mut registry = ThemeRegistry::new();
    registry.register("light-only", vec![fixture("product-light-only.json")]);
    assert!(matches!(
        registry.resolve("light-only", "dark"),
        Err(ResolveError::MissingMode { .. })
    ));
}
