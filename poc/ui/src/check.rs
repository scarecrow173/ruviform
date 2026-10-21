//! capability 検出（`--check` と `poc-metric.v1` の `env` 共通の情報源）。
//!
//! GUI を一切作らずに動くことが契約（test-spec「`--check` は GUI を作らず
//! exit 0」）。WebView2 レジストリ `pv` 値・wgpu adapter 列挙・OS/toolchain を
//! 検出し、実験側はこれをゲーティング（`status:"skipped"` 判定）に使う。

use serde_json::{Value, json};

/// 実験ゲーティングと `env` 記録に使う環境スナップショット。
/// `detect()` が各実験起動時に 1 回だけ呼ばれる（計測契約）。
pub struct Environment {
    /// `std::env::consts::OS`（"windows" 等）。
    pub os: &'static str,
    /// `std::env::consts::ARCH`（"x86_64" 等）。
    pub arch: &'static str,
    /// ビルド時 rustc バージョン（build.rs が `UI_POC_RUSTC` に焼き込む）。
    pub rustc: &'static str,
    /// WebView2 Runtime の `pv` 値。未検出なら None。
    pub webview2_version: Option<String>,
    /// wgpu adapter の先頭1件（`name (backend)`）。列挙不可・0件なら None。
    pub wgpu_adapter: Option<String>,
    /// `UI_POC_HEADLESS=1` が立っているか（強制 skip 経路）。
    pub headless_env: bool,
    /// 表示環境が存在すると判断できるか（実験がウィンドウを開ける前提）。
    pub display_available: bool,
}

impl Environment {
    /// `poc-metric.v1` の `env` フィールド相当の JSON を生成する。
    pub fn to_metric_env(&self) -> Value {
        json!({
            "os": self.os,
            "arch": self.arch,
            "webview2_version": self.webview2_version,
            "wgpu_adapter": self.wgpu_adapter,
            "rustc": self.rustc,
        })
    }
}

/// 環境検出を 1 回だけ実行する。
pub fn detect() -> Environment {
    Environment {
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        rustc: env!("UI_POC_RUSTC"),
        webview2_version: detect_webview2(),
        wgpu_adapter: detect_wgpu_adapter(),
        headless_env: std::env::var_os("UI_POC_HEADLESS").is_some_and(|v| v == "1"),
        display_available: display_available(),
    }
}

/// `--check` の出力 JSON を組み立てる（GUI 非生成・exit 0 の唯一の診断入口）。
pub fn check_json(env: &Environment) -> Value {
    json!({
        "schema": "poc-check.v1",
        "os": env.os,
        "arch": env.arch,
        "rustc": env.rustc,
        "webview2_version": env.webview2_version,
        "wgpu_adapter": env.wgpu_adapter,
        "headless_env": env.headless_env,
        "display_available": env.display_available,
    })
}

/// WebView2 Runtime バージョン検出。
/// Windows: EdgeUpdate Clients `{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}` の `pv` 値を
/// HKLM（64bit 視点 → 32bit 視点）→ HKCU の順に読む（公式推奨の検出手順）。
/// 非 Windows / 未検出は None。
#[cfg(windows)]
fn detect_webview2() -> Option<String> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

    const CLIENT: &str = "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
    let subkeys = [
        format!("SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{CLIENT}"),
        format!("SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{CLIENT}"),
    ];
    for (root, sub) in [
        (HKEY_LOCAL_MACHINE, subkeys[0].as_str()),
        (HKEY_LOCAL_MACHINE, subkeys[1].as_str()),
        (HKEY_CURRENT_USER, subkeys[1].as_str()),
    ] {
        if let Ok(key) = RegKey::predef(root).open_subkey(sub)
            && let Ok(pv) = key.get_value::<String, _>("pv")
            && !pv.is_empty()
            && pv != "0.0.0.0"
        {
            return Some(pv);
        }
    }
    None
}

#[cfg(not(windows))]
fn detect_webview2() -> Option<String> {
    None
}

/// wgpu adapter 検出。wgpu 30 の `enumerate_adapters` は Future を返すため
/// `pollster::block_on` で同期化する（GUI 非生成・イベントループ不要）。
/// 先頭 adapter を `name (backend)` 形式で返す。
fn detect_wgpu_adapter() -> Option<String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()));
    adapters.first().map(|a| {
        let info = a.get_info();
        format!("{} ({:?})", info.name, info.backend)
    })
}

/// 表示環境の存在判定。Windows は対話デスクトップセッションを前提
/// （`UI_POC_HEADLESS` で明示的に無効化可能）。Linux は DISPLAY/WAYLAND_DISPLAY、
/// macOS はウィンドウサーバ常駐を前提に true。
fn display_available() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some()
    }
    #[cfg(not(target_os = "linux"))]
    {
        true
    }
}
