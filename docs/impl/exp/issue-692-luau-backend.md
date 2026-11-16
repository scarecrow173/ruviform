---
id: "EXP-692LuauBackend0001"
type: "impl"
status: "completed"
title: "Issue #692 s2 Luau analyzer backend — 実装過程の観察と判断"
created: "2026-11-17"
updated: "2026-11-17"
owners:
  - "devin-worker"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/692"
  changes:
    added:
      - type: "directory"
        path: "docs/impl/"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/issue-692-luau-backend.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-692-luau-backend.md"
    - "docs/impl/exp/rf-script-02-luau-bridge-poc.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #692 s2 — experiment notes

実装過程の観察と判断の記録。正本 IR:
`docs/impl/ir/issue-692-luau-backend.md`、証拠:
`evidence/rf692/luau-backend-verification.md`。

## 採用した判断

- **`mlua` を dev-dependency ではなく必須依存にした**: 当初 design は
  「mlua = coexistence 検証の dev-dep」を想定していたが、shim が
  リンクする `luau*.lib` は mlua→mlua-sys→luau0-src(0.22.0+luau740)
  の build script が供給する静的ライブラリであり、dev-dep では
  本番依存グラフに入らず下流 consumer の最終リンクで解決不能になる。
  Rust API としては未使用のまま `[dependencies]` に配置し、
  spec/design/ADR を実態に合わせて修正した。
- **`analyze(&self)` の内部可変性は `Mutex<AnalyzerState>`**:
  RF-SCRIPT-00 の `ScriptAnalyzer` は `&self`（PoC の `&mut self`
  ミラーとは異なる）。shim Frontend は可変状態を持つため Mutex で
  直列化し、`loaded_definitions`（package_name dedup）と
  `owner_thread`（ThreadId 記録）を同じロック内で管理する。
  `NonNull<...>` フィールドにより `Send`/`Sync` は自動的に非実装
  となり、owner-thread affinity を型で表現する。
- **cursor offset の `usize` 変換は `try_from`**: `u64→usize` の
  `as` cast は 32-bit target で truncate し得るため、clippy
  `cast_possible_truncation` 対応も兼ねて fail-closed の `try_from`
  にした（範囲外は `InvalidRequest`）。
- **status 6/7 の写像**: `INJECTED`(7) は wildcard arm で
  `Internal{status code 保持}` に集約。`INTERNAL`(6) は spec 写像表の
  専用 message（"shim internal error"）を出すため明示 arm を持つ
  （review cycle 1 で message 不一致を検出・修正）。
- **`position_offset` は shim `offsetToPosition` の逆変換として
  byte 走査で実装**: 行区切りは Luau Lexer 規約どおり `\n` のみ
  （`\r` は空白扱い — 単独 `\r` は改行にならず `\r\n` は `\n` で
  1 回計数）。当初は shim 側と揃えて単独 `\r` も計数していたが、
  review cycle 1 で Lexer との乖離を指摘され両側を `\n` のみに
  統一。`col` は行末で clamp、`line_start + col` は saturating add。

## 棄却した案

- **typecheck timeout を部分診断つき成功で返す**: Luau
  `CheckResult.timeoutHits` が非空のとき shim は status 6
  （`Internal`）を返す。中立契約に partial 信号がなく、部分結果を
  完全な診断として返すのは fail-open のため棄却（review cycle 2
  P2 で検出・修正）。
- **shim status code を公開 enum 化**: 中立契約の `AnalyzerError`
  に写像して内部に閉じる設計（spec の境界表どおり）。公開面に
  status code を出すと vendor/ABI 詳細が漏れる。
- **型定義の request 同梱を都度 Frontend 再生成で実現**: shim に
  unload 経路がないため、`add_type_definition` の永続化 +
  `loaded_definitions` dedup で冪等にした（PoC と同一 semantics）。
- **`max_source_bytes` を shim のみに委譲**: Rust 側でも descriptor
  広告値で早期 reject し、無駄な FFI 往復を避ける
  （shim 側検査は defense-in-depth として残置）。

## 観察メモ

- `cargo check`（lib のみ）は test 専用 import を unused と誤検する
  ため、test module 側に `use ruviform_script::ScriptSource` を
  明示した（`use super::*` 経由の推移的 import に依存しない）。
- `AnalyzerRegistry::resolve` の Ok 型 `&dyn ScriptAnalyzer` は
  `Debug` 非実装のため、テストでは `.map(|_| ())` を挟んで
  `expect_err` を使う（s1 slice の `expect_resolve_err` helper と
  同じ制約）。
- mbx（rustc cache wrapper）が build-script stdout の非 UTF-8
  バイトで cache store を skip する warning を出すが、ビルド・
  テスト結果には影響しない（既知の環境警告）。
