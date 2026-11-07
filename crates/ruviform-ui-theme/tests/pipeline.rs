//! Contract-fixture tests for the DTCG pipeline (Issue #385).
//!
//! Fixtures live in `tests/fixtures/`. The resolver must be deterministic:
//! same inputs → byte-identical `to_normalized_json` and identical
//! (fail-fast, first-error) classification.

use ruviform_ui_theme::{
    DocumentLayer, DurationUnit, LengthUnit, ResolveError, ResolvedValue, Rounding, ScaleContext,
    StrokeStyle, ThemeResolver, TokenDocument, TokenError,
};

fn fixture(name: &str) -> TokenDocument {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let json = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
    TokenDocument::from_json(&json)
        .unwrap_or_else(|e| panic!("fixture {name} failed to parse: {e}"))
}

fn fixture_err(name: &str) -> TokenError {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let json = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
    match TokenDocument::from_json(&json) {
        Ok(_) => panic!("fixture {name} unexpectedly parsed"),
        Err(e) => e,
    }
}

fn stack() -> Vec<TokenDocument> {
    vec![
        fixture("primitive.json"),
        fixture("semantic-modeless.json"),
        fixture("component.json"),
    ]
}

#[test]
fn valid_pipeline_resolves_typed_values() {
    let theme = ThemeResolver::resolve(&stack(), "dark").unwrap();

    assert_eq!(theme.schema_version(), "rvf-dtcg-profile.v1");
    assert_eq!(theme.mode(), "dark");
    assert_eq!(theme.sources().len(), 3);
    assert_eq!(theme.sources()[0].layer, DocumentLayer::Primitive);
    assert_eq!(theme.sources()[2].layer, DocumentLayer::Component);

    // dark mode value wins over base.
    let primary = theme.color("color.primary").unwrap();
    assert_eq!(primary.components[0], f64::from(0x66) / 255.0);
    assert_eq!(primary.alpha, 1.0);

    // alias resolves to the mode-selected target value.
    let bg = theme.color("color.bg").unwrap();
    assert_eq!(bg, primary);
    let button = theme.color("button.bg").unwrap();
    assert_eq!(button, primary);

    // object-form color with alpha.
    let muted = theme.color("color.muted").unwrap();
    assert_eq!(muted.components, [0.5, 0.5, 0.55]);
    assert_eq!(muted.alpha, 0.8);

    let md = theme.length("space.md").unwrap();
    assert_eq!(md.value, 12.0);
    assert_eq!(md.unit, LengthUnit::Px);
    assert_eq!(md.rounding, Rounding::None);

    let page = theme.length("space.page").unwrap();
    assert_eq!(page.unit, LengthUnit::Rem);
    assert_eq!(page.rounding, Rounding::Nearest);

    assert_eq!(
        theme.font_family("font.family").unwrap(),
        ["Inter", "system-ui"]
    );
    assert_eq!(theme.font_weight("font.weight"), Some(600));
    assert_eq!(theme.number("scale.factor"), Some(1.25));

    let fast = theme.duration("motion.fast").unwrap();
    assert_eq!(fast.unit, DurationUnit::Milliseconds);
    assert_eq!(fast.value, 120.0);

    assert_eq!(
        theme.get("motion.ease"),
        Some(&ResolvedValue::CubicBezier([0.2, 0.0, 0.0, 1.0]))
    );
    assert_eq!(
        theme.get("stroke.border"),
        Some(&ResolvedValue::StrokeStyle(StrokeStyle::Dashed))
    );
}

#[test]
fn deterministic_output_is_stable() {
    let a = ThemeResolver::resolve(&stack(), "dark").unwrap();
    let b = ThemeResolver::resolve(&stack(), "dark").unwrap();
    assert_eq!(a.to_normalized_json(), b.to_normalized_json());
    assert!(
        a.to_normalized_json()
            .contains("\"schema\": \"rvf-dtcg-profile.v1\"")
    );
}

#[test]
fn light_mode_uses_base_values() {
    let theme = ThemeResolver::resolve(&stack(), "light").unwrap();
    let primary = theme.color("color.primary").unwrap();
    assert_eq!(primary.components[0], f64::from(0x1e) / 255.0);
}

#[test]
fn consumer_override_scoped_to_same_key_and_mode() {
    let mut docs = stack();
    docs.push(fixture("product-override.json"));
    // out-of-order input must still resolve in layer order.
    docs.swap(0, 3);
    let dark = ThemeResolver::resolve(&docs, "dark").unwrap();
    let light = ThemeResolver::resolve(&docs, "light").unwrap();

    let dark_primary = dark.color("color.primary").unwrap();
    assert_eq!(dark_primary.components[0], f64::from(0xff) / 255.0);
    assert_eq!(dark_primary.components[1], f64::from(0xaa) / 255.0);

    let light_primary = light.color("color.primary").unwrap();
    assert_eq!(light_primary.components[1], f64::from(0x88) / 255.0);

    // Alias followers observe the override too.
    assert_eq!(dark.color("color.bg").unwrap(), dark_primary);
    assert_eq!(dark.color("button.bg").unwrap(), dark_primary);

    // Keys the product doc does not touch stay at common-layer values.
    assert_eq!(dark.length("space.md").unwrap().value, 12.0);
    assert!(dark.get("color.muted").is_some());
}

#[test]
fn missing_mode_is_error() {
    let err = ThemeResolver::resolve(&stack(), "contrast").unwrap_err();
    match err {
        ResolveError::MissingMode { document, mode } => {
            assert_eq!(document, "rvf-primitive");
            assert_eq!(mode, "contrast");
        }
        other => panic!("expected MissingMode, got {other:?}"),
    }
}

#[test]
fn modeless_document_resolves_under_any_requested_mode() {
    let json = r##"{
      "$extensions": { "dev.ruviform.dtcg-profile": {
        "profile": "rvf-dtcg-profile.v1", "layer": "primitive", "id": "t" } },
      "c": { "$type": "color", "x": { "$value": "#112233" } }
    }"##;
    let doc = TokenDocument::from_json(json).unwrap();
    let theme = ThemeResolver::resolve(&[doc], "anything").unwrap();
    assert!(theme.color("c.x").is_some());
}

#[test]
fn modeless_document_reaches_alias_resolution_not_mode_gate() {
    let docs = vec![fixture("semantic-modeless.json")];
    // dangling alias -> classified UnknownReference at alias stage,
    // proving the mode gate did not fire on the modeless doc
    assert!(matches!(
        ThemeResolver::resolve(&docs, "anything").unwrap_err(),
        ResolveError::UnknownReference { .. }
    ));
}

#[test]
fn non_ascii_hex_color_is_invalid_value_not_panic() {
    for bad in ["#日", "#é1", "#1é"] {
        let json = format!(
            r#"{{
              "$extensions": {{ "dev.ruviform.dtcg-profile": {{
                "profile": "rvf-dtcg-profile.v1", "layer": "primitive", "id": "t" }} }},
              "c": {{ "$type": "color", "x": {{ "$value": {bad:?} }} }}
            }}"#
        );
        assert!(
            matches!(
                TokenDocument::from_json(&json),
                Err(TokenError::InvalidValue { .. })
            ),
            "{bad} must be InvalidValue, not a panic"
        );
    }
}

#[test]
fn duplicate_token_path_is_parse_error() {
    let json = r##"{
      "$extensions": { "dev.ruviform.dtcg-profile": {
        "profile": "rvf-dtcg-profile.v1", "layer": "primitive", "id": "t" } },
      "a": { "$type": "color",
        "b": { "c": { "$value": "#112233" } },
        "b.c": { "$value": "#445566" } }
    }"##;
    assert!(matches!(
        TokenDocument::from_json(json),
        Err(TokenError::InvalidValue { .. })
    ));
}

#[test]
fn non_string_rounding_extension_is_parse_error() {
    let json = r#"{
      "$extensions": { "dev.ruviform.dtcg-profile": {
        "profile": "rvf-dtcg-profile.v1", "layer": "primitive", "id": "t" } },
      "d": { "$type": "dimension", "x": {
        "$value": { "value": 1, "unit": "px" },
        "$extensions": { "dev.ruviform.rounding": 123 } } }
    }"#;
    assert!(matches!(
        TokenDocument::from_json(json),
        Err(TokenError::InvalidValue { .. })
    ));
}

#[test]
fn empty_documents_is_error() {
    assert!(matches!(
        ThemeResolver::resolve(&[], "dark"),
        Err(ResolveError::EmptyDocuments)
    ));
}

#[test]
fn alias_cycle_is_error() {
    let docs = vec![fixture("err-cycle.json")];
    match ThemeResolver::resolve(&docs, "m").unwrap_err() {
        ResolveError::AliasCycle { cycle } => {
            assert_eq!(cycle, ["a.x", "a.y", "a.x"]);
        }
        other => panic!("expected AliasCycle, got {other:?}"),
    }
}

#[test]
fn unknown_reference_is_error() {
    let docs = vec![fixture("err-unknown-ref.json")];
    match ThemeResolver::resolve(&docs, "m").unwrap_err() {
        ResolveError::UnknownReference { path, reference } => {
            assert_eq!(path, "a.x");
            assert_eq!(reference, "a.missing");
        }
        other => panic!("expected UnknownReference, got {other:?}"),
    }
}

#[test]
fn override_type_mismatch_is_error() {
    let mut docs = stack();
    docs.push(fixture("err-type-mismatch.json"));
    match ThemeResolver::resolve(&docs, "dark").unwrap_err() {
        ResolveError::TypeMismatch { path, base, .. } => {
            assert_eq!(path, "color.primary");
            assert_eq!(base, "color");
        }
        other => panic!("expected TypeMismatch, got {other:?}"),
    }
}

#[test]
fn alias_type_mismatch_is_error() {
    let json = r##"{
      "$extensions": { "dev.ruviform.dtcg-profile": {
        "profile": "rvf-dtcg-profile.v1", "layer": "primitive", "id": "t" } },
      "a": { "$type": "dimension", "x": { "$value": "{b.y}" } },
      "b": { "$type": "color", "y": { "$value": "#fff" } }
    }"##;
    let doc = TokenDocument::from_json(json).unwrap();
    assert!(matches!(
        ThemeResolver::resolve(&[doc], "m"),
        Err(ResolveError::TypeMismatch { .. })
    ));
}

#[test]
fn invalid_value_is_parse_error() {
    assert!(matches!(
        fixture_err("err-invalid-value.json"),
        TokenError::InvalidValue { .. }
    ));
}

#[test]
fn unknown_type_is_parse_error() {
    assert!(matches!(
        fixture_err("err-unknown-type.json"),
        TokenError::UnknownTokenType { .. }
    ));
}

#[test]
fn reserved_key_is_parse_error() {
    assert!(matches!(
        fixture_err("err-reserved-key.json"),
        TokenError::ReservedKey { .. }
    ));
}

#[test]
fn missing_profile_is_parse_error() {
    assert!(matches!(
        fixture_err("err-missing-profile.json"),
        TokenError::MissingProfile
    ));
}

#[test]
fn partial_alias_is_parse_error() {
    assert!(matches!(
        fixture_err("err-partial-alias.json"),
        TokenError::InvalidValue { .. }
    ));
}

#[test]
fn duplicate_source_is_error() {
    let base = fixture("primitive.json");
    let dup = base.clone();
    assert!(matches!(
        ThemeResolver::resolve(&[base, dup], "dark"),
        Err(ResolveError::DuplicateSource { .. })
    ));
}

#[test]
fn with_scale_keeps_tokens_and_changes_context() {
    let theme = ThemeResolver::resolve(&stack(), "dark").unwrap();
    let scaled = theme.with_scale(ScaleContext {
        scale_factor: 2.0,
        rem_base_px: 16.0,
    });
    assert_eq!(scaled.scale().scale_factor, 2.0);
    assert_eq!(scaled.color("color.primary"), theme.color("color.primary"));
    assert_ne!(scaled.to_normalized_json(), theme.to_normalized_json());
}
