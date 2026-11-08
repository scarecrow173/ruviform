---
id: "DESIGN-034UdN3RWXD8Kk4elhwAtR"
type: "design"
status: "approved"
title: "Issue #243 GPUI 実機 GUI 検証 Windows 実窓ライフサイクル証拠 — 設計"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/243"
  changes:
    added:
      - "crates/ruviform-ui-gpui/examples/window_lifecycle_probe.rs"
      - "docs/specs/issue-243-gpui-realwin-windows.md"
      - "docs/plans/implement-issue-243-gpui-realwin-windows.md"
      - "docs/tasks/implement-issue-243-gpui-realwin-windows.md"
      - "docs/test-specs/issue-243-gpui-realwin-windows.md"
      - "docs/impl/ir/issue-243-gpui-realwin-windows.md"
    modified:
      - "crates/ruviform-ui-gpui/Cargo.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/issue-243-gpui-realwin-windows.md"
  implemented-by: []
  depends-on:
    - "../designs/issue-386-driver-neutral-gui-host.md"
    - "../designs/issue-431-webview-gpui-real-verify.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../reports/issue-431-real-verify.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/issue-243-gpui-realwin-windows.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../designs/issue-386-driver-neutral-gui-host.md"
  defers: []
  deferred-by: []
---

# Issue #243 GPUI 実機 GUI 検証 Windows 実窓ライフサイクル証拠 — 設計

## Context

`ruviform-ui-gpui` の `GpuiDriver` は `WindowDriver` port（`create_window`/`resize`/
`focus`/`request_close`/`respond_close`/`destroy_window`/`post_event`/`windows`/
`monitors`/`exit` + `capabilities`）を実装し、`WindowLifecycle` イベント
（`Created`/`Ready`/`Resized`/`Moved`/`Focused`/`ScaleFactorChanged`/
`CloseRequested`/`Destroyed`/`Action`）を `DriverEvent::Windowed` で配送する。
headless では `ruviform-ui-test` の共有 fixture（`run_lifecycle`）と
`gpui-test` の `TestAppContext` 経路で契約検証済み（Issue #386/#431）。

本設計は Windows 実機での観測方法だけを定める。Driver・契約・fixture は変更しない。

## Scope

### In Scope

- 観測用 example `window_lifecycle_probe`（`crates/ruviform-ui-gpui/examples/`）1 本。
  `required-features = ["gpui"]` の `[[example]]` エントリを crate の `Cargo.toml` に追加
  （workspace `check --all-targets` が default feature で example を build しないために必須）。
- 既存 example の再実行による現行 main での再確認:
  - `real_window_lifecycle`（2 窓・Dock 2-phase/abort/detach/reattach・
    OS `WM_CLOSE` による Defer/Approve）— `evidence/issue-431-real-verify/wm-close.ps1`
    を再利用（読み取り・実行のみ。変更しない）
  - `minimal_window`（単一窓の `Created`/`Ready`/`Focused`/`Resized`/`Moved`/
    `CloseRequested`→approve→`Destroyed`）
- 実測ログの収集と `docs/impl/ir/` への証拠記録。

### Out of Scope

- 製品コード（`src/`）・公開 API・共有 fixture・他 example の変更
- `evidence/` 配下への新規ファイル追加（dispatch の編集許可範囲外。
  証跡は impl record 文書内のログ引用で代替する）
- `mise.toml` への新規 task 追加（編集禁止範囲。example の実行は
  `mise exec -- cargo run --example ...` で行い補助証拠と明記する）
- docs 生成 index（`docs/*/README.md`）の再生成（編集禁止範囲。
  `--no-index` で scaffold し、再生成は Coordinator/後続へ委ねる）

## Components and Boundaries

- `window_lifecycle_probe.rs`: 観測専用。`GpuiDriver` を `WindowDriver` port 経由でのみ
  操作し、全 `DriverEvent` を時系列ログとして stderr に出す。判定は末尾で
  PASS/FAIL を exit code に反映する。view/Dock の実描画は対象外
  （`real_window_lifecycle` が DockWorkspace モデル経路を既に担う）。
- `wm-close.ps1`（既存・再利用）: `PostMessageW(WM_CLOSE)` で実 OS close 経路を駆動。
  物理クリックではないことを証拠に明記する。

## Data and Control Flow

probe の既定モード（全て実窓・実 OS）:

1. `capabilities()` を出力（multi_window/focus/scale_factor/resize/ime/
   native_surface_slot の Supported/Unsupported を実値で記録）。
2. `create_window` で 2 窓（`main`・`veto`）を予約 → `run` 開始。
   `Created`/`Ready`/幾何イベントを全て記録。
3. 両窓 `Ready` 後、Tick ベースの step 機械で順に:
   - `focus(veto, true)` → `Focused` 遷移を観測（`main` の `Focused(false)` も記録）
   - `focus(main, true)` → 逆方向を観測
   - `focus(veto, false)` → `UiError::Unsupported` を期待して記録
   - `focus(unknown_id, true)` → `UiError::Host` を期待して記録
   - `resize(main, 960x600)` → `Resized` イベント到達を観測
   - `monitors()` → 実値（台数・scale factor）を出力
4. close フェーズ（外部 `wm-close.ps1 -Steps "veto,veto,veto,main"` が駆動）:
   - `veto` への 1 回目 `WM_CLOSE` → `CloseRequested` → `respond_close(Veto)`
     → window 生存を確認（Destroyed が来ないことを trace で検証）
   - 2 回目 → `Defer` → 生存確認
   - 3 回目 → `Approve` → `Destroyed`（Destroyed が 1 回だけであることを検証）
   - `veto` の `Destroyed` 後に `create_window` で `reborn` を生成
     → 新 `WindowId` が旧 id と異なること（非再利用）を検証
   - `main` への `WM_CLOSE` → `CloseRequested` に**応答しない**
     → 無応答既定 Approve で `Destroyed`（既定動作の実測）
   - `main` の `Destroyed` 後に `request_close(reborn)`（programmatic 経路）
     → `CloseRequested` → `respond_close(Veto)` → 生存確認
   - 続けて `destroy_window(reborn)` → `CloseRequested` を経ず `Destroyed`
     （bypass 経路の実測。契約は「`CloseRequested` を bypass する」と明記しており、
     実測で `CloseRequested` が出た場合は契約違反の発見として FAIL 記録する）
   - `reborn` の `Destroyed` で `exit()` → PASS 判定
5. `--degenerate-create` モード（別プロセス・別実行）: `logical_size` 0x0 の
   `WindowSpec` で `create_window` し、`Created`/`Ready`/`Destroyed` の
   有無を記録して分類出力（window 生成失敗の縮退経路の観測）。
   結果は実測どおり記録し、panic した場合も exit code とログを証拠とする。

各イベントには経路タグを付けない代わりに、文書側で「どの操作がどのイベントを
誘発したか」を時系列ログから対応付ける。OS メッセージ経路（`WM_CLOSE`）と
gpui 内部 API 経路（`request_close`/`focus`/`resize`/`destroy_window`）は
操作主体が異なるため、ログの前後関係とステップ番号で区別する。

## Non-Functional Constraints

- `unsafe` なし。例は `examples/` の manual verification で CI 対象外
  （`required-features = ["gpui"]`）。
- example は `WindowDriver` port と `DockWorkspace` の公開 API のみを使い、
  gpui 型を直接触らない（adapter 境界の維持）。
- 実行は実 OS session が必要。timeout（180s — `wm-close.ps1` の step 毎
  window 検索 deadline 30s × 4 step + 起動待ちを見込む）で無限 hang を防ぐ。

## Risks and Trade-offs

- `focus(veto, false)` は gpui 0.2.2 で `Unsupported` が契約上の正解。
  例はこれを FAIL ではなく観測値として記録する（契約どおりの応答を確認する）。
- `WM_CLOSE` の `Veto`/`Defer` は `on_should_close` → `respond_close` の
  同期 flush 内で処理されるため、外部スクリプトの step 間隔
  （`StepWaitMs`）内に確実に完結する。逆に step を詰めすぎると
  前の close が未処理のまま次が来るため、既定値のまま使う。
- degenerate spec（0x0）が gpui 内部で panic する可能性がある。
  別モード・別プロセスで実行し、主系列の証拠を失わないようにする。
- `monitors()` の scale factor は「生きている window で観測した値」の推定
  （driver 実装の既知の近似）。実測値として記録し、精度限界を明記する。

## Deferred Design Concerns

- `ScaleFactorChanged` の実遷移・モニター切断: DPI の異なる複数モニタ環境が必要。
  owner: 後続 #243 dispatch（NotRun として記録）。
- IME・物理入力・NativeSurface attach 失敗: 実ユーザー入力または #388/#389 の範囲。
- consumer 非回帰: audio-lens#166・synvelis#113/#114。

## Verification Notes

- [ ] `cargo fmt --all -- --check`（`mise run fmt-check`）pass
- [ ] `cargo check --workspace --all-targets`（`mise run check`）pass
- [ ] `cargo clippy -p ruviform-ui-gpui --all-targets --features gpui -- -D warnings`
  （`mise run lint-ui-gpui`）pass
- [ ] `cargo test -p ruviform-ui-gpui --features gpui-test`（`mise run test-ui-gpui`）pass
- [ ] probe 既定モードが `[probe] PASS` + exit 0（実機）
- [ ] probe `--degenerate-create` の観測結果を分類して記録（Pass/Fail は実測どおり）
- [ ] `real_window_lifecycle` を `wm-close.ps1 -Steps "inspector,inspector,main"` で
  再実行し PASS + exit 0
- [ ] `minimal_window` を `wm-close.ps1 -Steps "minimal"`（タイトル部分一致
  `ruviform — minimal gpui window`）で再実行し exit 0
