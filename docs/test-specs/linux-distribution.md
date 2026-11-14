---
id: "TSPEC-034X9FNp2hKXlrw3sQ5AOt"
type: "test-spec"
status: "proposed"
title: "Linux 配布設計（CI 照合確定版）の検証"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/127"
    - "docs/designs/linux-distribution-design.md"
    - "evidence/rf127/linux-ci-crosscheck.md"
  changes:
    added:
      - type: "doc"
        path: "evidence/rf127/linux-ci-crosscheck.md"
      - type: "doc"
        path: "docs/specs/linux-distribution.md"
      - type: "doc"
        path: "docs/designs/linux-distribution-design.md"
      - type: "doc"
        path: "docs/test-specs/linux-distribution.md"
      - type: "doc"
        path: "docs/plans/linux-distribution-plan.md"
      - type: "doc"
        path: "docs/tasks/linux-distribution-task.md"
      - type: "doc"
        path: "docs/impl/ir/linux-distribution.md"
      - type: "doc"
        path: "docs/impl/exp/linux-distribution.md"
    modified:
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
      - type: "doc"
        path: "docs/impl/exp/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/linux-distribution.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-127-linux-dist-design.md"
  refines:
    - "docs/test-specs/issue-127-linux-dist-design.md"
  refined-by: []
  derives-from:
    - "docs/specs/linux-distribution.md"
  derived-by: []
  verifies:
    - "docs/specs/linux-distribution.md"
    - "docs/designs/linux-distribution-design.md"
  verified-by: []
  references:
    - "evidence/rf127/linux-ci-crosscheck.md"
    - "docs/designs/issue-127-linux-dist-design.md"
    - "docs/test-specs/issue-127-linux-dist-design.md"
    - "docs/plans/issue-127-linux-dist-design.md"
    - "docs/tasks/issue-127-linux-dist-design.md"
  defers: []
  deferred-by: []
---

# Linux 配布設計（CI 照合確定版）の検証

## Purpose

Issue #127 の残件スライス（既存 Ubuntu CI の照合と設計文書の確定）が、証拠と設計記述の間で「実測・宣言・未実施」の区別を崩さずに完了したことを保証する。本 spec が守る意図は「CI の成功を実機・配布の成功に見せかけない」ことであり、将来この文書群を更新する者が判定語彙を緩めた場合に検出するための規範を固定する。実機検証 Issue（#419/#262/#388/#389）や Consumer 製品 packaging が本 spec の対象を引き継ぐ際の入口としても機能する。

## Feature

Linux 配布設計の CI 照合確定版 — 既存 Ubuntu CI lane の対象 revision・成功 run・判定語彙が `evidence/rf127/linux-ci-crosscheck.md` と `docs/designs/linux-distribution-design.md` に矛盾なく記録され、docs チェーンと管理 index が整合していること。

## Rules

- **R1（SHA 固有性）**: 設計・証拠文書が引用する CI 成功は、必ず対象 revision（commit SHA）・run URL・job 名を伴う。SHA を欠く「CI が通った」記述、または対象 revision と異なる SHA の成功を現行 revision の `Pass` として引用する記述を許さない。
- **R2（NotRun の保持）**: CI に lane が定義されているが対象 revision で未実行の場合、`NotRun`（または `NotRun(Pending)`）と記録し `Pass` にしない。現行 main HEAD（収集時 `621490e8`）の CI 未実施はこの規則の適用例である。
- **R3（P(L0) の非昇格）**: `P(L0)`（Xvfb+openbox+lavapipe の offscreen/headless CI 成功）は実機 Linux `Pass`・GUI-L2・EXT-L3 の証拠に使わない。headless CI（`gpui-test`/`uitest`）の成功も実 display・実入力・実 session の `Pass` にしない。
- **R4（能力別判定の分離）**: Native Build/Link・Headless・Feature・依存・ABI の各観点で、実測 `Pass`・`Declared`・`Unsupported`・`NotRun`・`Deferred`・`Blocked` を混同しない。dlopen 実行時依存の網羅検査や専用 ABI lane のように「CI 定義なし」の項目は `NotRun` とし、CI を増設しない。
- **R5（形式の保留）**: deb/AppImage/Flatpak のいずれについても、作成・署名・install 検証の実績なしに採用・配布可能を主張しない。全形式 `保留` の記述を維持する。
- **R6（Owner 境界）**: 製品 packaging 形式・署名・同梱アセット・対応 OS 宣言・リリース workflow・製品受入を Ruviform の決定事項として記述しない。Ruviform は共通の入力（capability matrix・runtime/ABI 基準・適合条件）のみを提供する。
- **R7（Legacy 分離）**: 旧 WebView/WebKitGTK 経路（`wry`/`tao`/GTK3）の成功・依存を GPUI 配布の必須条件・証拠として引用しない。削除済みとも主張しない（撤去は Issue #465）。
- **R8（文書チェーン整合）**: spec→design→test-spec→plan→task の relations が実在ファイルを指し、双方向の型付き relation が矛盾しない。管理 index（`docs/*/README.md`・`docs/impl/*/README.md`）は doc-maintenance/impl-doc の生成物であり手編集しない。

## Examples

- **E1（R2 の適用）**: Given 収集時点の main HEAD `621490e8` に `ci.yml` run が存在しない、When 設計文書が OS-L1 の状態を記述する、Then `NotRun(Pending)` と記録し最新成功 run（43439434737 @ `365c74ac`）を祖先 SHA の証拠として分離して引用する。
- **E2（R3 の適用）**: Given `ui-gpui-offscreen-verify` が ubuntu-24.04 hosted で `real_window` 3 passed を記録した、When capability matrix の X11 ウィンドウ生成行を更新する、Then `P(L0)`（ubuntu-24.04 hosted）と記し実機 `Pass` にはしない。
- **E3（R5 の適用）**: Given 3 形式とも package 未作成・署名なし・install 未検証、When §4 の判断根拠を記述する、Then 全形式 `保留` を維持し「CI の L0 成功」を採用根拠にしない。
- **E4（R7 の適用）**: Given `ui-verify (ubuntu-latest)` が旧 WebView 経路を含む UI lane で成功した、When GPUI 配布の証拠を引用する、Then この lane の成功を GPUI 必須条件の証拠に使わない。

## Guarantees

- 設計・証拠文書の全ての `Pass` 判定は、対象 revision・実行 URL・job/テスト名を辿れる。
- `NotRun`/`Deferred`/`Unsupported`/`Blocked` の項目には理由と再評価条件が記録されている。
- docs チェーン 5 文書 + IR/exp が `audit_docs`（artifact profile）・`audit_impl_record`・`audit_experiment_log` の blocking finding なしで完了する。
- 管理 index が doc-maintenance/impl-doc の生成手順で再生成され、`doc_status` materialized profile で index 系 finding がない。

## Non-goals

- 実機 Linux GUI/Plugin/Device の検証（#419/#262/#388/#389 が所有）。
- deb/AppImage/Flatpak の実作成・署名・install 検証。
- CI lane の追加・変更・閾値の緩和。
- macOS/Windows leg の受入。
- 旧 WebView/WebKitGTK 依存の撤去（#465）。
- Consumer（AudioLens/Synvelis）製品 packaging の決定。

## Risk

- この spec の保証が落ちると、「CI が通った」ことが「Linux で配布できる」ことに誤読され、未検証の distro・形式・実機 capability を前提にした製品判断が生まれる。
- `P(L0)` と実機 `Pass` の混同は、Native Plugin Dock・IME・DPI 等の実機依存機能の Fail をリリース後まで検出できないリスクを生む。
- SHA を欠く CI 引用は、後続のコード変更で無効化された証拠を有効なまま参照し続ける事故を招く。
