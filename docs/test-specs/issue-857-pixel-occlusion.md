---
id: "TSPEC-rf857PixelOcclusion"
type: "test-spec"
status: "draft"
title: "Issue #857 pixel-occlusion — real-dcomp-pixel の lane 自身 top-level window 遮蔽（class B）検証仕様"
created: "2026-11-21"
updated: "2026-11-21"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/857"
    - "crates/ruviform-plugin-dock-test/src/real_dock.rs"
  changes:
    added:
      - type: "file"
        path: "docs/discovery/issue-857-pixel-occlusion.md"
      - type: "file"
        path: "docs/specs/issue-857-pixel-occlusion.md"
      - type: "file"
        path: "docs/test-specs/issue-857-pixel-occlusion.md"
      - type: "dir"
        path: "evidence/rf857/"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_dock.rs"
      - type: "file"
        path: "docs/test-specs/issue-532-dcomp-pixel.md"
      - type: "file"
        path: "evidence/rf631/plugin-dock-real-scenario.jsonl"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-857-pixel-occlusion.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-857-pixel-occlusion.md"
    - "docs/discovery/issue-857-pixel-occlusion.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-857-pixel-occlusion.md"
    - "docs/discovery/issue-857-pixel-occlusion.md"
    - "docs/test-specs/issue-532-dcomp-pixel.md"
    - "docs/specs/issue-532-dcomp-residual.md"
    - "docs/discovery/issue-532-dcomp-residual.md"
    - "evidence/rf532/plugin-dock-real-pinned.jsonl"
    - "evidence/rf631/plugin-dock-real-scenario.jsonl"
    - "evidence/rf857/"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-857-pixel-occlusion.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-857-pixel-occlusion.md"
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/532"
    - "https://github.com/scarecrow173/ruviform/issues/631"
  defers: []
  deferred-by: []
metadata:
  issue: 857
---

# Issue #857 pixel-occlusion — 検証仕様

`real-dcomp-pixel` が lane 自身の spawn する別 top-level window に
被覆されて fail する問題（class B）の処遇 — sampling 時の bounded
de-occlusion（`raise` → `set_topmost` pair + `osmut::restore`）と
fail-closed verdict gate — の検証設計。設計確定は
`docs/specs/issue-857-pixel-occlusion.md`、class 確定根拠は
`docs/discovery/issue-857-pixel-occlusion.md` を参照。

## 対象 case

- `real-dcomp-pixel` — lane `rf631.plugin-dock-real`（interactive
  Windows / GPUI 実 window）。`win32-bitblt-screen` backend の screen
  capture が `EDITOR_COLOR` green `>= 0.5` を被測定 child rect で観測
  することを oracle とする。
- 設計変更は `observe_child_pixels` の採取経路と
  `editor_pixel_body` の verdict 分岐（`PixelObservation::Occluded`
  追加）に閉じる。他 case の verdict・exit code 意味論は不変。

## 受入条件（Issue #857 AC → 測定項目）

### AC1 — 脱遮蔽後の interactive 実走で `real-dcomp-pixel` = `Pass`

| 項目 | 判定 |
| --- | --- |
| case verdict（`evidence/rf631/plugin-dock-real-scenario.jsonl` 内 `real-dcomp-pixel` 行） | `pass` |
| `screen_pixels` step（backend `win32-bitblt-screen`） | `green_fraction >= 0.5` |
| `occlusion_hit_test` の role | 全 probe point が被測定 hierarchy 内（`editor_child`/`container`/`root`/`none`）— `other` なし |
| `occlusion_exstyle`（root） | `noredirectionbitmap == false`（DComp on の実証） |
| `deocclusion_apply_*` / `deocclusion_restore` steps | mutation applied / `topmost_restored: true` が記録されていること |

**lane exit code は class C（`real-dcomp-bandescape`/
`real-dcomp-childinv`）の既知 Fail で 1 のまま** — AC1 の判定は
exit code ではなく上記 JSONL 項目で行う。

### AC2 — 遮蔽残存時の fail-closed verdict

| 状況 | 期待 verdict |
| --- | --- |
| `occlusion_hit_test` のいずれかの probe point が role `other` を返す（評価済み capture を持つ child） | `not_run` — detail が external top-level occlusion を明示し、外部 window の画素を Pass/Fail どちらにも読み替えない |
| 全 hit が hierarchy 内で green < 0.5 | `fail`（遮蔽起因でない実ミス — compositor/raster の問題として正直に記録） |

検証方法: `PixelObservation::Occluded` → `not_run(...)` の写像は
コード契約（`editor_pixel_body` verdict 分岐 + `pixel_extras` の
`occluded` extras）＋本仕様/`docs/specs/issue-857-pixel-occlusion.md`
の記録で担保する。脱遮蔽が成功した run では本経路は不発 —
遮蔽が残った環境（外部 topmost window が採取中も被覆し続ける
host）では同 lane の実走が `NotRun` + `screen_pixels.occluded`
extras を残すことを fail-closed 実証として記録する。

### AC3 — 実走証拠の記録

`evidence/rf857/` に run 記録を残す（`evidence/rf857/run-record.md`
または JSONL 抜粋）:

- 対象 SHA・run 条件（interactive Windows・DComp on・`mise run
  test-ui-plugin-dock-real`）
- `occlusion_hit_test` が被測定 child 系 role を返すこと
- `screen_pixels` の backend・green_fraction
- `occlusion_exstyle` root `noredirectionbitmap=false`
- `window_identities`（4 window mount・被測定 root の識別）
- `deocclusion_*` steps（baseline/apply/restore の mutation outcome）

canonical lane 出力 `evidence/rf631/plugin-dock-real-scenario.jsonl`
は lane が再生成するものをそのまま commit に含める。

## verdict 意味論の確定対応表（仕様どおり）

| 状況 | `PixelObservation` | case verdict |
| --- | --- | --- |
| lifecycle NG | — | `Fail`（lifecycle detail） |
| 評価済み child の hit に role `other` 残存 | `Occluded` | `NotRun`（fail-closed） |
| 全 hit が hierarchy 内・green >= 0.5 | `GreenSeen` | `Pass` |
| 全 hit が hierarchy 内・green < 0.5 | `NoGreen` | `Fail` |
| capture 不能・非表示・children 無し | `NotEvaluable` | `NotRun` |
| open 拒否 / 非 Windows | — | `Unsupported` |

## 非回帰（本 slice で変えないもの）

- `test-ui-plugin-dock`（headless scenario lane）・
  `test-ui-plugin-dock-console`（`--list`/`--run` catalog 投影）の
  verdict 集合と exit code は main と同一。
- `test-ui-plugin-dock-real-headless`（`--blocked-probe`）は
  environment-blocked → `NotRun` mapping の自己検証として exit 0
  想定 — 本 slice の `Occluded` → `NotRun` 意味論と整合。
- class C 2 case の既知 Fail 継続（lane exit 1）は想定どおり —
  本 slice では verdict を変更しない。

## 検証コマンド（正規入口 = mise タスク）

| コマンド | 役割 |
| --- | --- |
| `mise run fmt` / `mise run fmt-check` | 整形適用 / 非破壊チェック |
| `mise run check ruviform-plugin-dock-test` | 対象 crate compile |
| `mise run lint ruviform-plugin-dock-test` | `-D warnings` |
| `mise run test ruviform-plugin-dock-test` | unit test 非回帰 |
| `mise run test-ui-plugin-dock` | headless scenario lane 非回帰 |
| `mise run test-ui-plugin-dock-console` | console lane 非回帰 |
| `mise run test-ui-plugin-dock-real-headless` | `--blocked-probe` fail-closed mapping 自己検証（exit 0） |
| `mise run check-ui-driver-boundary` | driver boundary guard 非回帰 |
| `mise run test-ui-plugin-dock-real` | **中核 — interactive Windows 実走（AC1/AC3 gate）** |
| `mise exec -- prek run --all-files` | pre-commit hook 全ファイル実行 |

ローカル検証は変更範囲の非回帰証明であり、統合受入は merge 後の
main SHA に対する必須 CI 結果（Coordinator が確認）で確定する。
