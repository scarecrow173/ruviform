---
id: "SPEC-034XAXSMzbLxf6doTfUBYY"
type: "spec"
status: "approved"
title: "Issue #532 GPUI DComp public-surface 監査スライス"
created: "2026-11-15"
updated: "2026-11-15"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/532"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-532-gpui-dcomp-surface.md"
      - type: "file"
        path: "docs/designs/issue-532-gpui-dcomp-surface.md"
      - type: "file"
        path: "docs/plans/issue-532-gpui-dcomp-surface.md"
      - type: "file"
        path: "docs/tasks/issue-532-gpui-dcomp-surface.md"
      - type: "file"
        path: "docs/test-specs/issue-532-gpui-dcomp-surface.md"
      - type: "file"
        path: "docs/impl/ir/issue-532-gpui-dcomp-surface.md"
      - type: "file"
        path: "docs/impl/exp/issue-532-gpui-dcomp-surface.md"
      - type: "file"
        path: "docs/discovery/issue-532-gpui-dcomp-surface.md"
      - type: "file"
        path: "evidence/rf532/gpui-public-surface-audit.md"
    modified:
      - type: "file"
        path: "docs/discovery/issue-532-dcomp-native-dock.md"
      - type: "file"
        path: "docs/designs/issue-532-dcomp-native-dock.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/discovery/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements: []
  implemented-by:
    - "docs/plans/issue-532-gpui-dcomp-surface.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/issue-532-gpui-dcomp-surface.md"
    - "docs/discovery/issue-532-dcomp-native-dock.md"
    - "docs/designs/issue-532-gpui-dcomp-surface.md"
    - "docs/test-specs/issue-532-gpui-dcomp-surface.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-532-gpui-dcomp-surface.md"
  derived-by:
    - "docs/designs/issue-532-gpui-dcomp-surface.md"
    - "docs/plans/issue-532-gpui-dcomp-surface.md"
  verifies: []
  verified-by:
    - "docs/test-specs/issue-532-gpui-dcomp-surface.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/388"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/synvelis/issues/176"
  defers: []
  deferred-by: []
---

# Issue #532 GPUI DComp public-surface 監査スライス

## What Should Be Built

Issue #532 の方式 A/B/C 調査手順の技術入力となる、vendored gpui 0.2.2
Windows platform 層の public-surface 監査記録。正本の監査記録
`docs/discovery/issue-532-gpui-dcomp-surface.md` と証拠ノート
`evidence/rf532/gpui-public-surface-audit.md` に、(a) DComp
オブジェクトグラフと `topmost` 引数の全出現箇所、(b)
`GPUI_DISABLE_DIRECT_COMPOSITION` の意味論と非 DComp 経路、(c)
public vs `pub(crate)` 到達可能性、(d) Option A のパッチ面積、
(e) Option B の private-internal 要件、(f) gpui バージョン情勢、
(g) exit-127 のコードパス仮説を記録する。先行 discovery
`docs/discovery/issue-532-dcomp-native-dock.md` の Open Questions を
更新し、回答可能なものを解消・実 OS 依存のものは open のまま残す。
コード変更・patch/vendor 適用・実画素実験は対象外。

## Why Now

先行の headless 監査 slice（merge 済み）は DComp 制約の実機事実と
L1 証拠の CI 対応付けを固定したが、方式 A/B/C の採否に必要な
upstream public-surface の事実（patch 面積・公開到達性・env var
経路・private-internal 要件・exit-127 仮説）が未調査のまま残って
いた。L2/L3 利用要件発生時の実機調査を「何を・どの順で」開始できる
ようにするため、docs-only の source 監査で技術入力を先に固定する。

## Users and Value

- Coordinator / Issue #532・#262 の Owner: 方式 A/B/C の採否判断に
  必要な upstream 事実（patch 面積・公開面・env var 経路）を
  docs から参照できる。
- Ruviform の後続実装者: Option A（patch）・Option B（private
  interposition）の具体的な作業面積と前提を把握できる。
- #388/#389/SV #176 の Owner: L2/L3 再開時の最初の zero-code 経路
  （env var）とその blocker（exit-127）の切り分け手順を参照できる。

## Scope

### In Scope

- vendored `gpui-0.2.2` source の読み取り専用監査（7 項目）と
  `evidence/rf532/` への証拠ノート。
- `docs/discovery/issue-532-gpui-dcomp-surface.md`（正本監査記録）・
  `docs/designs/issue-532-gpui-dcomp-surface.md`（方式 A/B/C への
  技術入力設計）の作成。
- 先行 `docs/discovery/issue-532-dcomp-native-dock.md` の Open
  Questions 更新と `docs/designs/issue-532-dcomp-native-dock.md` の
  upstream 調査リスク記述の更新。
- 文書チェーン（spec/design/plan/task/test-spec/impl-ir/impl-exp）と
  触った dir の index 再生成。

### Out of Scope

- 実画面・実 Plugin・実 GUI の L2/L3 試験、実 OS での exit-127 再現。
- `crates/` 配下のコード変更、`Cargo.toml`/`Cargo.lock` の
  `[patch.crates-io]` 追加、gpui の fork/vendor 取り込み。
- 方式 A/B/C の採否判定・実装、Issue #532 の Close、GitHub 依存の
  登録・削除。
- cargo registry source の編集（読み取り専用参照）。

## Requirements

- R1: DComp オブジェクトグラフ（device/target/visual/swapchain の
  生成・保持・所有）を辿り、全ての `CreateTargetForHwnd` サイトと
  `topmost` 引数を列挙する。
- R2: `GPUI_DISABLE_DIRECT_COMPOSITION` の読み取りサイト・受理値・
  非 DComp 経路の実動作を記録する。
- R3: downstream crate から公開 API 経由で触れるもの/触れないものを
  境界ごとに記録する。
- R4: Option A（`[patch]`/fork で `topmost` literal 変更）のパッチ
  面積（literal の個数・場所）を見積もる。パッチ適用自体はしない。
- R5: Option B（ruviform private compositor/child attachment）に
  必要な DComp/visual-tree コードを、gpui object 再利用不能を前提に
  特定する。
- R6: gpui 0.2.2 が最新 publishable 版であること、zed repo 現行版が
  publish 不能であることを確認し、根拠として
  `zed-industries/zed#55271` を inline code 形式で引用する
  （auto-reference を誘発しない書式）。
- R7: exit-127 について非 DComp 経路のコードパスから原因仮説を記録
  する。仮説のみとし、実 OS 検証は PENDING として残す。
- R8: 先行 discovery の Open Questions を更新し、回答可能なものを
  解消・実 OS 依存のものは open のまま残す。`status: resolved` を
  戻さない。

## Acceptance Criteria

- [ ] `docs/discovery/issue-532-gpui-dcomp-surface.md` が正本として
  作成され、F1（DComp グラフ）・F2（env var 意味論）・F3（公開面）・
  F4（patch 面積）・F5（exit-127 仮説）・F6（version 情勢）・F7
  （Option B 空間）・方式 A/B/C への技術入力を含む。
- [ ] `evidence/rf532/gpui-public-surface-audit.md` がコードパス・
  行番号・判定根拠を含む。
- [ ] `docs/designs/issue-532-gpui-dcomp-surface.md` が監査結果の
  方式 A/B/C への写像を設計として記録する。
- [ ] `docs/discovery/issue-532-dcomp-native-dock.md` の Open
  Questions が更新され、回答可能分が解消・実 OS 依存分が open 維持。
  status は resolved のまま。
- [ ] 文書チェーン（spec/design/plan/task/test-spec/impl-ir/impl-exp）
  が front matter relations で相互に解決可能。
- [ ] 触った `docs/<type>/` dir の index が `doc-maintenance` で
  再生成される。
- [ ] `mise exec -- prek run --files <変更ファイル>` が exit 0。
- [ ] exit-127・`topmost=false` 効果・実 plugin 画素が「未検証/
  PENDING」として明記され、Pass/Unsupported への偽装がない。

## Deferred Design Concerns

- 方式 A/B/C（および A' env var 経路）の技術的採否と実装設計 |
  実機 L2/L3 調査が前提 | Windows L2/L3 利用要件発生時 |
  採否未決のまま実装に入ると契約違反のリスク。
- exit-127 の実 OS 再現と原因確定 | interactive Windows 11 desktop
  session 依存 | L2/L3 再開時の Step 1 | env var 経路（A'）が
  実質的に使用不能なまま残る可能性。
- `topmost=false`/`SetWindowRgn`/第二 DComp target の実画素 semantics |
  実機 fixture が前提 | L2/L3 再開時 | 方式の採否材料が欠ける。

## Sources and Relations

- `docs/discovery/issue-532-gpui-dcomp-surface.md` — 正本監査記録
  （derives-from）。
- `docs/designs/issue-532-gpui-dcomp-surface.md` — 方式 A/B/C への
  技術入力設計。
- `evidence/rf532/gpui-public-surface-audit.md` — 監査証拠ノート。
- `docs/discovery/issue-532-dcomp-native-dock.md` — 先行監査（本 slice
  が Open Questions を更新）。
- vendored `gpui-0.2.2` source（読み取り専用）・crates.io version
  history・`` `zed-industries/zed#55271` `` — 外部根拠。
