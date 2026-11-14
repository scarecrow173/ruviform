// RF-SCRIPT-02 / Issue #560 — ffi-poc build script.
//
// vendored Luau.Analysis (Luau 0.740) と FFI shim を C++17 で静的ライブラリ化
// する。VM/Compiler/Ast 等の非 Analysis 翻訳単位は mlua-sys 経由の
// luau0-src (同一 Luau 0.740) が供給するため、ここでは Analysis の
// src/*.cpp と shim のみをコンパイルする。
//
// コンパイルフラグは luau0-src の Build 実装 (mlua-sys 経由) と揃える:
//   std=c++17, LUAI_MAXCSTACK=1000000, LUA_VECTOR_SIZE=3,
//   LUA_API=extern "C", debug 時 LUAU_ENABLE_ASSERT。
// これにより VM 側オブジェクトと ABI/ODR が一致する。

use std::env;
use std::path::PathBuf;

/// vcvars 未初期化の環境（素の shell / CI agent）でも cl.exe が
/// CRT/SDK ヘッダを解決できるよう、find-msvc-tools が検出した
/// 環境変数（INCLUDE/LIB/LIBPATH/PATH）を build script プロセスへ適用する。
/// cc が spawn する子プロセスへ継承される。既に INCLUDE があれば何もしない。
#[cfg(all(windows, target_env = "msvc"))]
#[expect(unsafe_code)]
// SAFETY: build script の main 直後・他スレッド生成前に呼ばれるため
// 環境変数変更は単一スレッドで安全。cc が spawn する子プロセスへ継承される。
fn setup_msvc_env() {
    if env::var_os("INCLUDE").is_some() {
        return;
    }
    let target = env::var("TARGET").unwrap_or_default();
    if let Some(tool) = find_msvc_tools::find_tool(&target, "cl.exe") {
        // SAFETY: 上記 SAFETY コメントの通り、単一スレッドでの環境変数設定。
        unsafe {
            for (key, value) in tool.env() {
                env::set_var(key, value);
            }
        }
    }
}

#[cfg(not(all(windows, target_env = "msvc")))]
fn setup_msvc_env() {}

fn main() {
    setup_msvc_env();

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let vendor = manifest_dir.join("vendor").join("luau");
    let analysis_src = vendor.join("Analysis").join("src");
    let native = manifest_dir.join("native");

    println!("cargo:rerun-if-changed={}", analysis_src.display());
    println!("cargo:rerun-if-changed={}", native.display());
    println!(
        "cargo:rerun-if-changed={}",
        vendor.join("REVISION").display()
    );

    // luau0-src Build と同一の define 群。
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .warnings(false)
        .define("LUAI_MAXCSTACK", "1000000")
        .define("LUA_VECTOR_SIZE", "3")
        .define("LUA_API", "extern \"C\"");

    // shim のコメントは UTF-8（日本語）。MSVC は既定で system codepage
    // （CP932 等）として読むため、UTF-8 ソースには /utf-8 が必須。
    // Luau vendored ソースは ASCII のため影響しない。
    if env::var("TARGET").is_ok_and(|t| t.contains("msvc")) {
        build.flag("/utf-8");
    }

    if env::var("PROFILE").unwrap_or_default() == "debug" {
        build.define("LUAU_ENABLE_ASSERT", None);
    }

    // include 解決順: Analysis 独自 → vendored public headers → shim。
    build.include(vendor.join("Analysis").join("include"));
    for dir in [
        "Ast", "Bytecode", "CodeGen", "Common", "Compiler", "Config", "Inliner", "Require", "VM",
    ] {
        let inc = vendor.join(dir).join("include");
        if inc.is_dir() {
            build.include(inc);
        }
    }
    build.include(&native);

    // Analysis/src/*.cpp を決定的順序で追加（luau0-src と同じ方針）。
    let mut sources: Vec<PathBuf> = std::fs::read_dir(&analysis_src)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "cpp"))
        .collect();
    sources.sort();
    for src in &sources {
        build.file(src);
    }
    build.file(native.join("rf_luau_bridge.cpp"));

    build.compile("rf_luau_bridge");

    // Analysis オブジェクトが参照する Luau シンボル（Ast/Common/Compiler/
    // VM/Config/Require 等）は mlua-sys 経由の luau0-src 静的ライブラリが
    // 供給する。rustc は native lib を依存グラフ順に並べるため、
    // rf_luau_bridge.lib が luau*.lib より後に置かれると bridge の未解決
    // 参照を解決できない（MSVC link.exe はコマンドラインを左から走査する）。
    // link-search は mlua-sys が既に供給しているため、同名 lib を本 crate
    // からも emit して bridge 直後の位置に再掲する。各 .lib 出現ごとに
    // 未解決シンボルが検索されるため重複は無害。
    // 注: lib 名は mlua-sys 0.13.0 / luau0-src 0.22.0+luau740 の生成物に
    // 対応する PoC 固定値。mlua-sys 更新時は要再照合。
    for lib in [
        "luauinliner",
        "luauvm",
        "luaubytecode",
        "luaucompiler",
        "luauast",
        "luaucommon",
        "luauconfig",
        "luaucustom",
        "luaurequire",
    ] {
        println!("cargo:rustc-link-lib=static={lib}");
    }
}
