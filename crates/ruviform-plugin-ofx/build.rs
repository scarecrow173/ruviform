use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src/ffi");
    println!("cargo:rerun-if-changed=vendor/openfx/include");
    println!("cargo:rerun-if-changed=tests/fixture");

    let mut c = cc::Build::new();
    c.include("vendor/openfx/include")
        .include("src/ffi")
        .warnings(true)
        .warnings_into_errors(true)
        .file("src/ffi/ofx_bridge.c")
        .file("src/ffi/ofx_registry.c")
        .file("src/ffi/ofx_params.c");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        c.flag("-pthread");
    }
    c.compile("ofx_bridge");

    build_fixture();
}

/// Compile the C test fixture into a real shared library inside `OUT_DIR` so
/// no build output lands in the package directory. The DLL path is exported
/// to the test binary via `OFX_FIXTURE_DLL`.
fn build_fixture() {
    let srcs = [
        Path::new("tests/fixture/ofx_test_fixture.c"),
        Path::new("tests/fixture/ofx_fixture_probes.c"),
    ];
    for s in srcs {
        assert!(s.exists(), "fixture source missing: {}", s.display());
    }

    let compiler = cc::Build::new().get_compiler();
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR");
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").expect("target os");

    let (dll, implib, pdb) = match target_os.as_str() {
        "windows" => (
            "ofx_test_fixture.dll",
            Some("ofx_test_fixture.lib"),
            Some("ofx_test_fixture.pdb"),
        ),
        "macos" => ("libofx_test_fixture.dylib", None, None),
        _ => ("libofx_test_fixture.so", None, None),
    };
    let dll_out = format!("{out_dir}/{dll}");

    let mut cmd: Command = compiler.to_command();
    if compiler.is_like_msvc() {
        cmd.arg("/nologo")
            .arg("/LD")
            .arg(format!("/Fo{out_dir}/"))
            .arg("/Ivendor/openfx/include")
            .arg("/Itests/fixture");
        for s in &srcs {
            cmd.arg(*s);
        }
        cmd.arg(format!("/Fe{dll_out}")).arg("/link");
        if let Some(imp) = implib {
            cmd.arg(format!("/IMPLIB:{out_dir}/{imp}"));
        }
        if let Some(p) = pdb {
            cmd.arg(format!("/PDB:{out_dir}/{p}"));
        }
        cmd.arg("/INCREMENTAL:NO");
    } else {
        if target_os == "macos" {
            cmd.arg("-dynamiclib");
        } else {
            cmd.arg("-shared");
        }
        cmd.arg("-fPIC")
            .arg("-pthread")
            .arg("-Ivendor/openfx/include")
            .arg("-Itests/fixture");
        for s in &srcs {
            cmd.arg(*s);
        }
        cmd.arg("-o").arg(&dll_out);
    }
    let output = cmd.output().expect("spawn fixture compiler");
    assert!(
        output.status.success(),
        "fixture compile failed\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    for s in &srcs {
        println!("cargo:rerun-if-changed={}", s.display());
    }
    println!("cargo:rerun-if-changed=tests/fixture/fixture_control.h");
    println!("cargo:rerun-if-changed=tests/fixture/fixture_internal.h");
    println!("cargo:rustc-env=OFX_FIXTURE_DLL={dll_out}");
}
