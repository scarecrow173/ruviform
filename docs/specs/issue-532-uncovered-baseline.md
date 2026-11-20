---
id: "SPEC-034ZaaUncoveredBaseline001"
type: "spec"
status: "approved"
title: "Issue #532 uncovered-baseline bisect slice（settle 付き topmost 再観測と遮蔽なし baseline で compositor-internal occlusion を確定）"
created: "2026-11-20"
updated: "2026-11-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-532-uncovered-baseline.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_dock.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/real_catalog.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/ospres.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/src/bin/plugin-dock-real-scenario.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/tests/console.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/tests/plugin_dock.rs"
      - type: "file"
        path: "crates/ruviform-plugin-dock-test/README.md"
      - type: "file"
        path: "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-532-presentation-probe.md"
    - "docs/specs/issue-532-foreground-probe.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-532-dcomp-occlusion.md"
    - "docs/specs/issue-532-dcomp-pixel.md"
    - "evidence/rf532/presentation-probe.md"
    - "evidence/rf532/foreground-probe-bisect.md"
  refines:
    - "docs/specs/issue-532-dcomp-occlusion.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-532-presentation-probe.md"
  derived-by:
    - "docs/designs/issue-532-uncovered-baseline.md"
    - "docs/test-specs/issue-532-uncovered-baseline.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-532-uncovered-baseline.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
  defers: []
  deferred-by: []
metadata:
  issue: 532
---

# Issue #532 uncovered-baseline bisect slice（compositor-internal occlusion の確定）

## 背景と目的

`issue-532-presentation-probe` slice の判別は「screen green=0 は外部
HWND z-order 遮蔽で完全に説明可能。uncovered 状態での非提示は
settle 未検証のため未確定」まで絞られた。`presentation-probe.md`
§残課題の 4 項が残ったままである:

- (a) topmost 直後 capture は即時に取られたため、flat dark 画面が
  pre-paint frame の捕捉か真の非提示かが未判別。
- (b) 遮蔽のない環境を制御して取得した baseline が存在しない。
- (c) `EnumWindows` inventory は cap 256 で打ち切られ、自 window の
  z_index を直接観測できていない。
- (d) `PW_RENDERFULLCONTENT` による切り分けが未検証。

本 slice はこの 4 項を消化する実験 slice であり、修正でも方式
採否の決裁でもない。遮蔽なし環境での baseline 再観測と settle
付き topmost 再観測により、「残る no-green が pre-paint/unsettled
frame 由来か、compositor-internal/presentation 層の遮蔽か」を
確定させる。

## スコープ

### In Scope

- `ruviform-plugin-dock-test` に新規 real-lane catalog case
  `real-dcomp-uncovered` を追加（`REAL_CATALOG` 23 行・
  `real_entries` 23 要素・`real-dcomp-present` 直後の位置）。
- `ospres::Inventory` に `own_windows` field を追加。自 process の
  top-level window を cap 非依存で全件列挙し、真の z_index を記録
  する（残課題 c）。
- 新 case の probe 流れ: own-window inventory → baseline 観測 →
  env-prep mutation（`osmut` の raise → `SetForegroundWindow` →
  topmost — `real-dcomp-foreground` と同じ staged mutation/verified
  restore を `uc_*` step prefix で再利用）→ uncovered 判定（全
  hit-test 点が自 window 系 role を返すか）→ settle 付き再観測
  （`std::thread::sleep` による累積 200/500/1200ms の 3 段 —
  caller thread で sleep するため GPUI app thread の pump は継続
  する）→ post-settle capture variants（`PrintWindow` flag 変種 +
  window DC — 残課題 d）→ verified restore → restore 後観測。
- 各 settle 段で hit-test・screen green・child `has_update`（未処理
  update region — paint 滞留の判別子）を記録する。
- `mise.toml` `test-ui-plugin-dock-real` 直上の stale コメント修正
  （comment のみ — task 定義本体・lane 追加・description 変更は
  禁止）。
- baseline（env 未設定・canonical `mise run test-ui-plugin-dock-real`）
  と env-on（`GPUI_DISABLE_DIRECT_COMPOSITION=1`・直接 `cargo run`・
  別 `--out`）の両 interactive session 実走を記録する。
- 観測から compositor-internal occlusion の有無を確定し、方式
  採否（A patch/B-region/B-dcomp2/C）への帰結を doc chain と
  `evidence/rf532/` 証拠文書に記録する。判明した事実と推測を
  区別し、未確定なら未確定のまま残す。

### Out of Scope

- Issue `#532` の解決・方式採否の決裁（本 slice の occlusion 確定
  結果を入力とする後続 slice）。
- compositor 修正本体（`ruviform-ui-gpui`・`[patch]`/vendor）。
- heavyweight capture（Magnification/Windows.Graphics.Capture）導入。
- verdict 意味論の変更 — probe case の verdict は「実行・必須
  mutation 成功・restore 検証・lifecycle」のみ。green/hit-test/
  uncovered 判定は verdict 非入力の bisect データ。
- `real-dcomp-pixel` の既知 `Fail` の緩和・`NotRun` 化。
- Focus/Resize/ReDock/入力の実走、merge-SHA CI 証拠、
  macOS/Linux L1 対応付け、consumer 通知、Issue auto-close。
- `docs/**/README.md` index（Coordinator が統合時に整備）、
  `AGENTS.md`、consumer 側コード、`Cargo.lock`/ルート `Cargo.toml`
  （依存差分が出た場合のみ `Cargo.lock` 再生成可）。

## 要件

### 機能要件

1. `REAL_CATALOG` に `real-dcomp-uncovered` 行を追加し、
   `real_entries`/`blocked_reports`/`blocked_probe_violations`/
   console 期待値の件数を 22→23 に整合させる。
2. 新 case は既存 dcomp case と同じ attach lifecycle
   （`prepare_editor_host` → `attempt_real_attach` →
   `conclude_editor_session`）を通り、attach 中・lease 生存中に
   probe を実行する。
3. env-prep mutation と restore は `osmut` のみを経由し、
   `osoccl`/`ospix`/`ospres` の query-only 契約を侵さない。
   Win32 型を公開境界に出さない（HWND は `u64`）。
4. settle は bounded（累積 ~1.2s）で、各段の `settle_ms`・
   `settle_kind`・hit-test・screen green・child `has_update` を
   step row に記録する。等待を暗黙に行わない。
5. uncovered 判定は「child rect の全 hit-test 点が
   `editor_child`/`container`/`root` いずれかの role を返す」で
   計算し、`uc_uncovered` row に記録する。遮蔽を排除できない
   場合は `uncovered:false` を実測として記録し、以後の観測は
   続行する。
6. 復元は `osmut::restore` で topmost flag を baseline 値に戻し、
   `topmost_restored` を検証する。復元検証失敗は verdict 入力。
7. `SetForegroundWindow` の FALSE（foreground lock 拒否）は
   正直に記録し、成功に偽装しない。
8. `real-dcomp-pixel`・`real-dcomp-foreground`・
   `real-dcomp-present` の step 名・verdict・観測順序を変えない
   （step prefix を `ProbeLabels` で注入し、fg 側は `fg_*` のまま）。

### 非機能要件

- `unsafe` は FFI 境界の局所許可のみ。本 slice で新規 unsafe は
  追加しない見込み（`std::thread::sleep` + 既存 query/mutation API
  のみ）。
- 新規 dependency・feature 追加なし。
- mutation は sub-second の bounded 操作で、error path でも復元を
  試行する。
- RT contract: non-RT only。

## 受入基準

- [ ] `real-dcomp-uncovered` が catalog/entry/console で整合し、
  件数 assertion が 23 に更新されている。
- [ ] `ospres::Inventory.own_windows` が自 process の top-level
  window を cap 非依存で列挙する。
- [ ] baseline + env-on の interactive lane で case が実走し、
  inventory・各段階の mutation/hit-test/screen green・settle 各段・
  `has_update`・capture variants・restore 結果が JSONL に記録
  される。
- [ ] `real-dcomp-pixel` の verdict が `Fail`（既知の occlusion）
  を維持する。
- [ ] `mise run fmt-check`、crate check/lint/test（default +
  `real-dock`）、console/contract/boundary/blocked-probe lane、
  `prek run --all-files` が green。
- [ ] compositor-internal occlusion の有無と方式採否への帰結が
  doc chain + `evidence/rf532/` に記録され、未確定部分は正直に
  `unresolved` と明記される。
- [ ] Issue `#532` は open のまま、PR は `Refs #532`（auto-close
  しない）。
