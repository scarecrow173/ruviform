//! `ruviform-script-luau` — Issue #692 s2 / RF-SCRIPT-03 + Issue #716。
//! 実 Luau backend: `ruviform-script` の中立契約 C5 `ScriptAnalyzer`
//! を vendored Luau.Analysis (Luau 0.740) + 狭小 C ABI shim
//! (`native/rf_luau_bridge.{h,cpp}`) で、C2 `ScriptBackend`/
//! `ScriptSession`（eval/budget/artifact 実行経路）を mlua 0.12.2
//! Rust API（vendored Luau 0.740 — 同一 revision）で実装する。
//!
//! 公開 API は [`LuauAnalyzer`]/[`LuauAnalyzerOptions`]/
//! [`luau_language`]/[`LuauBackend`] と中立契約型の再利用のみ —
//! Luau/vendor/C++ 型・生ポインタ・mlua 型は公開面に出さない。
//! `unsafe` は `ffi` モジュール（extern "C" 宣言・RAII wrapper）に
//! 局所許可で隔離し、runtime module は safe Rust のみで構成する。
//!
//! 依存選定: `docs/adr/issue-692-luau-dependency-selection.md`
//! （analyzer = native Luau.Analysis bridge）+
//! `docs/adr/issue-716-runtime-backend.md`（runtime = mlua Rust API —
//! #692 AC4 再評価の確定。hybrid 構成）。`mlua` は shim がリンクする
//! Luau VM 静的ライブラリの供給元であり、runtime backend がその
//! Rust API を使用する。
//!
//! 非目標: Synvelis thin adapter（consumer gate — synvelis#271〜274）、
//! host binding 実行 seam（中立契約に handler 経路なし — 記述子の
//! 検証・受理のみ）、named entry points・evaluate 再実行・Resume VM
//! 復元、`max_instructions` budget（mlua に API なし —
//! `UnsupportedBudget` で fail-closed）、3-OS 実機検証（Windows
//! `x86_64` のみ実測、macOS/Linux は `NotRun`）。実 OS/実 VM 経路の
//! 受入は consumer gate（synvelis#516・synvelis#267 AUTX-08）へ委譲。

mod analyzer;
mod contract;
mod ffi;
#[cfg(test)]
mod ffi_boundary_tests;
mod runtime;

pub use analyzer::{DEFAULT_ANALYZER_ID, LuauAnalyzer, LuauAnalyzerOptions, luau_language};
pub use runtime::LuauBackend;
