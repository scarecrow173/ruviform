//! `ruviform-script-luau` — Issue #692 s2 / RF-SCRIPT-03。
//! 実 Luau analyzer backend: `ruviform-script` の中立契約 C5
//! `ScriptAnalyzer` を vendored Luau.Analysis (Luau 0.740) +
//! 狭小 C ABI shim (`native/rf_luau_bridge.{h,cpp}`) で実装する。
//!
//! 公開 API は [`LuauAnalyzer`]/[`LuauAnalyzerOptions`]/
//! [`luau_language`] と中立契約型の再利用のみ — Luau/vendor/C++ 型・
//! 生ポインタ・mlua 型は公開面に出さない。`unsafe` は `ffi` モジュール
//! （extern "C" 宣言・RAII wrapper）に局所許可で隔離する。
//!
//! 依存選定 (AC4): `docs/adr/issue-692-luau-dependency-selection.md` —
//! analyzer = native Luau.Analysis bridge。`mlua` vendored Luau は
//! shim がリンクする Luau VM 静的ライブラリの供給元として必須依存
//! （Rust API としては未使用）であり、runtime `ScriptBackend` 実装は
//! 別 slice で依存選定を再評価する。
//!
//! 非目標（本 slice）: 実 runtime backend（`ScriptBackend`/
//! `ScriptSession`）、Synvelis thin adapter（AC3 — synvelis#271〜274
//! consumer gate）、3-OS 実機検証（Windows `x86_64` のみ実測、
//! macOS/Linux は `NotRun`）。

mod analyzer;
mod contract;
mod ffi;
#[cfg(test)]
mod ffi_boundary_tests;

pub use analyzer::{DEFAULT_ANALYZER_ID, LuauAnalyzer, LuauAnalyzerOptions, luau_language};
