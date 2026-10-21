---
id: "TASK-034JfHavtGmdqmRGyDAbch"
type: "task"
status: "done"
title: "PoC ネイティブサーフェス合成（wgpu 隣接構成・PR #1762 評価）"
created: "2026-10-20"
updated: "2026-10-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/28"
  changes:
    added:
      - type: "file"
        path: "poc/ui/src/exp/surface.rs"
      - type: "doc"
        path: "docs/impl/ir/poc-native-surface.md"
      - type: "doc"
        path: "docs/impl/exp/ac3-surface-adjacent-parent.jsonl"
      - type: "doc"
        path: "docs/impl/exp/ac3-surface-adjacent-child.jsonl"
    modified:
      - type: "file"
        path: "poc/ui/Cargo.toml"
      - type: "file"
        path: "poc/ui/Cargo.lock"
      - type: "file"
        path: "poc/ui/RESULTS.md"
      - type: "file"
        path: "poc/ui/src/args.rs"
      - type: "file"
        path: "poc/ui/src/exp/mod.rs"
      - type: "doc"
        path: "docs/tasks/poc-native-surface.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/issue-28-ui-poc.md"
  implemented-by:
    - "../impl/ir/poc-native-surface.md"
  depends-on:
    - "../plans/issue-28-ui-poc.md"
    - "poc-ui-harness-host.md"
  blocks:
    - "poc-adr-finalize.md"
  supersedes: []
  superseded-by: []
  related:
    - "../specs/issue-28-ui-poc.md"
    - "../designs/issue-28-ui-poc.md"
    - "../adr/ui-tech-selection-poc.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/issue-28-ui-poc.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/issue-28-ui-poc.md"
  references:
    - "https://github.com/tauri-apps/wry/pull/1762"
  defers: []
  deferred-by: []
---

# PoC ネイティブサーフェス合成（wgpu 隣接構成・PR #1762 評価）

## Work

plan `PLAN-034JfDEU8mhCDj4ldYFZUG` の T4。design の AC3 実験設計を正本として、`wgpu` ネイティブサーフェスと WebView の同居を Windows 実機で評価し、DOM 内合成の制限事実を記録する。

- [x] `src/exp/surface.rs` に `--experiment surface --mode adjacent-parent` を実装: tao 親 HWND 自身を wgpu `Surface` 化し、WebView を `build_as_child` で左領域に配置、右の親クライアント領域に wgpu 描画（追加 HWND・unsafe 不要の主経路）
- [x] `--mode adjacent-child` を実装: sibling 子 HWND（windows-rs `WS_CHILD` + raw-window-handle）を作成して wgpu に割り当て、sibling z-order と WebView 領域への一時重なりを試験。**`unsafe` は windows-rs 呼出箇所の SAFETY コメント付き局所許可のみ**（暫定規約）。sibling HWND を wry API（`build_as_child` 第 2 子ビューや `WebViewExtWindows` reparent）で構成できるかを先に確認し、最小構成を選択する（design Open Questions）
- [x] 検査マトリクス各項目を両モードで pass/fail + 観察として記録: イベントヒットテスト・フォーカス遷移・z-order（sibling HWND・WebView vs ネイティブ面）・リサイズ追従（`set_bounds` + `SetWindowPos`/`surface.configure`）・wgpu フレーム継続
- [x] DOM overlay がネイティブ面に被るケースの回避策として `SetWindowRgn` クリップを試行し、実用性を観察記録（design Open Questions）
- [x] 「Windows の windowed WebView2 では子 HWND が同矩形ネイティブ面の上に座り、真の下層 DOM 内合成には `ICoreWebView2CompositionController` + DirectComposition が必要で wry 未対応（未 merge PR [#1762](https://github.com/tauri-apps/wry/pull/1762)）」を `poc/ui/RESULTS.md` に記録。macOS（layer-backed NSView/CAMetalLayer sibling 合成が最有力・自前実装）・Linux（真の下層合成は実質不可寄り → 隣接/Canvas 収束）は「文書評価」行として記録
- [ ] （任意・時間余裕時のみ）wry fork に PR #1762 を `[patch.crates-io]` で適用する探索的スパイクを試し、結果は「探索的付録」として本編と分離して記録
- [x] wgpu MSRV 1.87 が宣言 `rust-version = "1.85"` と衝突する事実を `RESULTS.md` に記録（pin toolchain 1.99.0 で検証・引き上げ判断は MSRV 見直しタスク連携）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/issue-28-ui-poc.md](../plans/issue-28-ui-poc.md) — `PLAN-034JfDEU8mhCDj4ldYFZUG`（approved）。T4 の正本
- [../designs/issue-28-ui-poc.md](../designs/issue-28-ui-poc.md) — AC3 surface 実験（2 モード・検査マトリクス・fork スパイク方針）の正本
- [../specs/issue-28-ui-poc.md](../specs/issue-28-ui-poc.md) — AC3 の正本
- [../adr/ui-tech-selection-poc.md](../adr/ui-tech-selection-poc.md) — 「合成方式」採否条項（隣接別サーフェス第一仮説）の対象
- [../test-specs/issue-28-ui-poc.md](../test-specs/issue-28-ui-poc.md) — 検査マトリクス・文書評価ラベル・探索的付録の分離規則

## Dependencies

- 前提: `poc-ui-harness-host.md`（host・`check.rs` の wgpu アダプタ検出・metrics が前提）
- 後続をブロック: `poc-adr-finalize.md`（AC3 エビデンス供給）
- wgpu（MSRV 1.87）は `poc/ui` 独立 WS 内でのみ依存（pin toolchain 1.99.0 でビルド。宣言値 1.85 との衝突記録は本タスクで `RESULTS.md` に記載）
- `unsafe`: windows-rs 呼出箇所のみ SAFETY コメント付き局所許可（暫定規約どおり）。`adjacent-parent` 主経路は unsafe 不要の設計

## Affected Files

- `poc/ui/src/exp/surface.rs`・`poc/ui/src/exp/mod.rs`（レジストリ追加）— 新規/更新（wgpu・windows-rs はこのモジュールに封じる — 将来 `ruviform-gpu` adapter のリハーサル）
- `poc/ui/Cargo.toml` — wgpu・windows 依存追加（`poc/ui` 内のみ）
- `poc/ui/RESULTS.md` — AC3 行 + PR #1762 制限事実 + MSRV 衝突記録 + macOS/Linux 文書評価行
- `docs/impl/exp/` — 実施ログを追記（fork スパイク実施時は「探索的付録」として分離）

## Done When

- [x] `--experiment surface --mode adjacent-parent` で wgpu 描画と WebView 子ビューが同居し、検査マトリクス全項目が pass/fail + 観察で記録される
- [x] `--mode adjacent-child` で sibling 子 HWND 構成の z-order・重なり挙動が記録され、`unsafe` が SAFETY コメント付き局所に限定されている
- [x] PR #1762 依存の制限事実と macOS/Linux 文書評価行が `RESULTS.md` に存在する
- [x] wgpu MSRV 衝突の記録が `RESULTS.md` に存在する

## Verification

- [x] `mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- --experiment surface --mode adjacent-parent` — `poc-metric.v1` 最終行 + 検査マトリクス結果・`status` 規約どおり
- [x] `mise exec -- cargo run --manifest-path poc/ui/Cargo.toml -- --experiment surface --mode adjacent-child` — sibling 構成の検査結果が記録される
- [x] `grep -rn "unsafe" poc/ui/src/` — `unsafe` が `exp/surface.rs` の windows-rs 呼出箇所 + SAFETY コメント付き局所に限定される
- [x] `git status --porcelain -- Cargo.toml Cargo.lock crates` — 出力が空（wgpu 依存がルートに混入しない）
- [x] Manual: `poc/ui/RESULTS.md` に AC3 行・PR #1762 制限事実・文書評価ラベル付き macOS/Linux 行が存在する（補助確認）
