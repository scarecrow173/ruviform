//! `HostApiSurface` neutrality validation tests (test-spec R6/E6).

use ruviform_script::prelude::*;

fn binding(name: &str) -> HostBinding {
    HostBinding {
        name: name.to_string(),
        signature: HostSignature {
            args: vec![ValueKind::String],
            returns: ValueKind::Any,
        },
        effect: HostEffect::Query,
        schema_version: HOST_SURFACE_SCHEMA_VERSION,
        timeout_hint: None,
    }
}

#[test]
fn validate_neutral_accepts_neutral_names() {
    let surface = HostApiSurface::new(vec![
        binding("action.invoke"),
        binding("project.query"),
        binding("timeline.read"),
    ])
    .expect("neutral names accepted");
    assert_eq!(surface.bindings.len(), 3);
    assert_eq!(surface.schema_version, HOST_SURFACE_SCHEMA_VERSION);
}

#[test]
fn validate_neutral_rejects_os_fs_net_shell_names() {
    for name in [
        "os.exec",
        "fs.read",
        "fs.write",
        "net.http",
        "network.fetch",
        "shell.run",
        "process.spawn",
        "env.get",
        "sys.info",
        "danger.raw",
        "OS.EXEC", // case-insensitive
    ] {
        let surface = HostApiSurface {
            bindings: vec![binding(name)],
            schema_version: HOST_SURFACE_SCHEMA_VERSION,
        };
        let err = surface
            .validate_neutral()
            .expect_err("forbidden prefix must fail");
        assert!(
            matches!(err, HostSurfaceError::ForbiddenBindingName { .. }),
            "{name} should be ForbiddenBindingName, got {err:?}"
        );
    }
}

#[test]
fn validate_neutral_rejects_empty_and_duplicate_names() {
    let surface = HostApiSurface {
        bindings: vec![binding("")],
        schema_version: HOST_SURFACE_SCHEMA_VERSION,
    };
    assert!(matches!(
        surface.validate_neutral(),
        Err(HostSurfaceError::EmptyBindingName)
    ));

    let surface = HostApiSurface {
        bindings: vec![binding("dup"), binding("dup")],
        schema_version: HOST_SURFACE_SCHEMA_VERSION,
    };
    assert!(matches!(
        surface.validate_neutral(),
        Err(HostSurfaceError::DuplicateBindingName { .. })
    ));
}

#[test]
fn host_api_surface_new_validates_at_construction() {
    assert!(HostApiSurface::new(vec![binding("os.exec")]).is_err());
    assert!(HostApiSurface::new(vec![binding("action.invoke")]).is_ok());
    assert!(HostApiSurface::empty().validate_neutral().is_ok());
}

#[test]
fn host_effect_is_advisory_not_authorization() {
    // All three effect classes are accepted by validation — effect is
    // metadata, not a grant.
    for effect in [HostEffect::Pure, HostEffect::Query, HostEffect::Effect] {
        let mut b = binding("action.invoke");
        b.effect = effect;
        let surface = HostApiSurface::new(vec![b]).expect("any effect accepted");
        assert_eq!(surface.bindings[0].effect, effect);
    }
}
