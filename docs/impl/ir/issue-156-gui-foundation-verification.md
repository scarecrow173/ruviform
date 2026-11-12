---
id: "IMPL-034PAk9mR2sNqT8vWxYz5dEhJb"
type: "impl"
status: "completed"
title: "Issue #156 GUI foundation implementation + real-machine verification"
created: "2026-10-31"
updated: "2026-10-31"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-156-gui-foundation-verification.md"
      - type: "file"
        path: "crates/ruviform-ui/"
      - type: "file"
        path: "crates/ruviform-ui-webview/"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/README.md"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/discovery/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/exp/data/issue-156-gate-fmt-check.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-156-gate-check.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-156-gate-lint.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-156-gate-test.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-156-gate-check-ui-webview.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-156-gate-lint-ui-webview.log"
      - type: "file"
        path: "docs/impl/exp/data/issue-156-gate-test-ui-webview.log"
  implements:
    - "docs/designs/issue-156-gui-host-foundation.md"
  implemented-by: []
  depends-on:
    - "docs/discovery/issue-156-gui-host-foundation-inventory.md"
    - "docs/adr/ui-architecture-webview-hybrid.md"
    - "docs/adr/ui-tech-selection-poc.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-156-gui-host-foundation.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/issue-156-gui-host-foundation.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #156 GUI foundation — 実装と実機検証記録

## Summary

`poc/ui` の成果を `ruviform-ui`(WebView 非依存コア)+
`ruviform-ui-webview`(feature `webview` の `wry`+`tao` adapter)の
2 クレートとして本番化し、headless テスト・CLI 実動作・
Windows 11 + WebView2 実機 GUI 検証まで実施した。
検証コマンド 7 本は全て exit 0、実機では intent/進捗/cancel/
panic 封じ込め/focus/resize/close の各経路を実動作で確認した。

## Context

- source: `poc/ui` PoC（Issue #156 — GUI host foundation）の成果を本番クレートとして実装し、実機検証まで行う記録
- implements: [../../designs/issue-156-gui-host-foundation.md](../../designs/issue-156-gui-host-foundation.md)
- depends-on: [../../discovery/issue-156-gui-host-foundation-inventory.md](../../discovery/issue-156-gui-host-foundation-inventory.md)、[../../adr/ui-architecture-webview-hybrid.md](../../adr/ui-architecture-webview-hybrid.md)、[../../adr/ui-tech-selection-poc.md](../../adr/ui-tech-selection-poc.md)
- 関連記録: `docs/impl/ir/poc-*.md`（PoC 各 AC の実装記録）と `docs/impl/exp/ac*.jsonl`（PoC 実験ログ）— 性能特性の根拠はこれらの計測値

## Implementation

### Changes

- `crates/ruviform-ui/` 新設 — WebView 非依存の GUI コアクレート。`DemoService`（intent/進捗/cancel/panic 封じ込め/focus/resize/close の各経路）と headless `demo_cli` example を含む
- `crates/ruviform-ui-webview/` 新設 — feature `webview` の `wry`+`tao` adapter。`demo_app` example・`rvf://app`→`http://rvf.app` scheme 書き換え・`BRIDGE_JS` 注入を実装
- `Cargo.toml` / `Cargo.lock` — ワークスペースメンバー登録・依存解決
- `mise.toml` — `check-ui-webview` / `lint-ui-webview` / `test-ui-webview` タスク追加
- `.github/workflows/ci.yml` — `ui-verify` lane 追加（3 OS で check/lint/test）
- `crates/README.md`・`AGENTS.md`・`docs/designs/README.md`・`docs/discovery/README.md`・`docs/impl/ir/README.md` — クレート索引・帳簿更新
- `docs/impl/exp/data/issue-156-gate-*.log` 7 本 — gate 実行ログ生成

### Decisions During Implementation

- **2 クレート分離**: `ruviform-ui`（WebView 非依存コア）と `ruviform-ui-webview`（`wry`+`tao` adapter、`webview` feature は非 default）に分離し、workspace 既定経路は headless のまま維持
- **バイナリ pull の閾値分離**: PoC 計測（binary pull は 1 MiB で約 94–115 MB/s・16 MiB で約 88–146 MB/s、64 KiB の小 pull は JSON RTT より遅い）に基づき、小値は JSON・バルクのみ `rvf://data` pull（1 読出上限 16 MiB）とする設計を採用
- **`RenderTier` 分離**: Canvas2D が約 32k 列で約 8.75 fps まで低下する計測を受け、DOM/Canvas2D/WebGL2/NativeSurface を描画 tier として分離

### 依存分離の確認

- `ruviform-ui` は `ruviform-exec`/`ruviform-rt` のみに依存し
  `wry`/`tao`/window 型を持たない(feature なし `cargo check` でも
  GUI 依存は解決されない)。
- `ruviform-ui-webview` は `ruviform-ui` のみに依存し、
  `webview` feature が非 default のため workspace 既定経路は
  headless のまま(`mise run check` で実証)。
- foundations→adapter・adapter→adapter 依存なし。RT クレートへの
  依存追加なし(UI は非 RT 専用)。

## Related Experiments

### Adopted

なし — 本記録は `poc/ui` 成果の本番化と検証であり、新規の実験ログは作成していない。性能特性の根拠となった PoC 計測は各 `docs/impl/ir/poc-*.md` が `docs/impl/exp/ac*.jsonl` を adopted として記録済み。検証 gate log 7 本のみ `docs/impl/exp/data/` に保存。

### Rejected

なし。

## Validation

実施環境: Windows 11 + WebView2（実機）。以下は全て実実行。

### ゲート結果(全て実実行・exit 0)

| コマンド | 結果 | log |
| --- | --- | --- |
| `mise run fmt-check` | pass | `../exp/data/issue-156-gate-fmt-check.log` |
| `mise run check` | pass(headless・wry/tao 非含有) | `../exp/data/issue-156-gate-check.log` |
| `mise run lint` | pass(`-D warnings`) | `../exp/data/issue-156-gate-lint.log` |
| `mise run test` | pass(ruviform-ui 15 件含む全ワークスペース) | `../exp/data/issue-156-gate-test.log` |
| `mise run check-ui-webview` | pass | `../exp/data/issue-156-gate-check-ui-webview.log` |
| `mise run lint-ui-webview` | pass(`-D warnings`) | `../exp/data/issue-156-gate-lint-ui-webview.log` |
| `mise run test-ui-webview` | pass(scheme 6 件) | `../exp/data/issue-156-gate-test-ui-webview.log` |

### 実機 GUI 検証(Windows 11 + WebView2・`demo_app` example)

`cargo run -p ruviform-ui-webview --features webview --example demo_app`
を実起動し `orca computer` 経由で操作。結果:

| 項目 | 結果 | 観測 |
| --- | --- | --- |
| 起動・ページロード | OK | `http://rvf.app/index.html` に WebView2 が `rvf://app` を書き換えてロード(PoC 実測どおり)。`BRIDGE_JS` 注入済み |
| intent → 即時結果 | OK | `sys.capabilities` が `{"schema":1,...,"os":"windows","arch":"x86_64","data":{"max_chunk":16777216}}` を返却 |
| job 進捗→完了→pull | OK | `demo.synthesize`(30 s)が `job started: 1` → `finished` で `demo/peaks`(3000 points/24 KB)を publish |
| cancel | OK | `demo.sleep` 起動 → `cancel requested` → `job 2 cancelled` |
| panic 封じ込め | OK | `demo.panic` が `job 3 failed: job worker panicked` を返しホスト存続(ワーカースレッド境界で捕捉) |
| focus/blur | OK | 各フォーカス遷移が `window focus`/`window blur` ログとして UI に到達 |
| resize | OK | `SetWindowPos` で 900x600 へ変更 → `viewport resize dpr=1 884x561`(WebView bounds 再適用) |
| close/shutdown | OK | `WM_CLOSE` → `CloseRequested` → イベントループ正常終了・プロセス消滅 |

`demo_cli`(headless)でも同一 `DemoService` が synthesize→pull→
cancel を完走(`job 1 finished`/`job 1 cancelled as requested`)し、
GUI/CLI 同一計算経路の成立を確認した。

### 性能特性(PoC 計測値の再掲・本番設計への反映)

`poc/ui` の計測(詳細は `docs/impl/ir/poc-*.md` と discovery 文書):

- binary pull: 1 MiB で約 94–115 MB/s、16 MiB で約 88–146 MB/s、
  cold path 約 330 ms。64 KiB の小 pull は約 2.9 MB/s で JSON RTT
  (64 B ≈0.5–1 ms、64 KiB ≈4.4 ms)より遅い → 設計では小値は JSON、
  バルクのみ `rvf://data` pull(1 読出上限 16 MiB)とする閾値分離を採用
- Canvas2D は約 32k 列で約 8.75 fps まで低下、WebGL2 は 57.9–60 fps
  を維持 → `RenderTier` で DOM/Canvas2D/WebGL2/NativeSurface を分離
- embedded assets 込み release 単一バイナリ約 8.1 MB、portable zip
  約 3.0 MB

## Risks

実装時に `未解決・既知制限` として記録された既知制限・未検証項目（Issue 候補として切り分け済み）:

1. **macOS / Linux 実機未検証** — `ui-verify` CI が 3 OS で
   check/lint/test を回すが窓付き動作は未確認。AudioLens 移行前に
   各 OS で `demo_app` 実機検証が必要(独立 Issue 候補)。
2. **高 DPI(DPR>1)未実測** — 検証機は dpr=1。`ScaleFactorChanged` で
   bounds 再適用する経路は resize と同一だが、DPI 変更実測は未取得。
3. **HTTPS virtual-host / WebGPU セキュアコンテキスト未検証** —
   WebView2 の `rvf://`→`http://` 書き換えで secure context が失われ
   WebGPU/WebGL 制約があり得る。wry PR #1762
   (`ICoreWebView2CompositionController`)による合成解決は未実装。
4. **`wgpu` ネイティブ面は契約のみ** — `NativeSurfaceHost`/
   `SurfaceRegion` の型はあるが実装 adapter は未存在
   (WebView2 composition 制約・測定要請が前提)。
5. **push 経路の合体** — `evaluate_script` 連打は WebView2 が合体する
   ため「1 push=1 更新」前提は不可。設計は pull/sequence latest-wins。
6. **小 pull は JSON より遅い** — 64 KiB 未満は JSON 側に残す運用が前提。
7. **AudioLens 規模の長時間負荷** — ジョブ有界メールボックス(進捗
   drop・終端必達)と pull 上限で設計済みだが、実アプリ規模での
   長時間ストレスは AudioLens 移行時に別途計測する。

## Follow-ups

上記 Risks のうち Issue 候補として切り分けた項目への対応:

- macOS / Linux 各 OS での `demo_app` 実機検証（AudioLens 移行前・独立 Issue 候補）
- 高 DPI(DPR>1)環境での `ScaleFactorChanged` 経路の実測
- HTTPS virtual-host / WebGPU secure context の検証と、wry PR #1762（`ICoreWebView2CompositionController`）による合成解決の実装検討
- `NativeSurfaceHost`/`SurfaceRegion` の実装 adapter
- AudioLens 規模の長時間負荷計測（実アプリ規模でのストレス確認）
