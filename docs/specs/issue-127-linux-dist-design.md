---
id: "SPEC-034udQra6CS6JQJJN4e0hG"
type: "spec"
status: "proposed"
title: "Issue #127 Linux 実配布方式の設計 — GPUI 標準 Driver の runtime 依存・Capability Matrix・パッケージ形式適合条件"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/127"
    - "https://github.com/scarecrow173/ruviform/issues/384"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-127-linux-dist-design.md"
      - type: "doc"
        path: "docs/designs/issue-127-linux-dist-design.md"
      - type: "doc"
        path: "docs/test-specs/issue-127-linux-dist-design.md"
      - type: "doc"
        path: "docs/plans/issue-127-linux-dist-design.md"
      - type: "doc"
        path: "docs/tasks/issue-127-linux-dist-design.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/issue-384-driver-neutral-gui-architecture.md"
    - "docs/designs/issue-384-driver-neutral-gui-contract.md"
    - "docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md"
    - "docs/adr/platform-and-toolchain-targets.md"
    - "docs/adr/licensing-and-distribution-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-388-native-surface-slot.md"
    - "docs/specs/issue-389-plugin-editor-session.md"
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/specs/issue-465-legacy-webview-p0p1.md"
    - "docs/designs/issue-257-driver-neutral-ui-contract.md"
    - "docs/designs/issue-386-driver-neutral-gui-host.md"
    - "docs/designs/issue-388-native-surface-slot.md"
    - "docs/designs/issue-465-legacy-webview-p0p1.md"
    - "docs/adr/ui-tech-selection-poc.md"
    - "docs/adr/issue-465-legacy-webview-retirement-status.md"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/ruviform/issues/127"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/57"
    - "https://github.com/scarecrow173/ruviform/issues/28"
    - "https://github.com/scarecrow173/ruviform/issues/419"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/465"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/synvelis/issues/28"
  defers: []
  deferred-by: []
---

# Issue #127 Linux 実配布方式の設計 — GPUI 標準 Driver の runtime 依存・Capability Matrix・パッケージ形式適合条件

## What Should Be Built

Ruviform の GPUI 標準 Driver を Linux 実環境へ配布するための設計記録を、docs チェーン（spec / design / test-spec / plan / task）として正本化する。具体的には次を文書化する。

1. **対象環境の整理**: 対象ディストリビューション/バージョン（現時点では未確定であることを明示）と、X11 / Wayland / XWayland の対象 Backend 範囲。
2. **Runtime 依存 Inventory**: GPUI（`gpui 0.2.2`、Zed 系、`Cargo.lock` 由来）および `ruviform-ui-gpui` が Linux で要求する実行時依存を、「確認済み（実測）/ 文書・lockfile 由来（宣言）/ 未実測 / 非対応 / 未実行」に区別して記録する。
3. **GUI Capability Matrix**: ウィンドウ生成・ライフサイクル、入力、フォーカス、IME、フォント描画、DPI/スケーリング、NativeSurfaceSlot、実 Plugin GUI（Native Plugin Dock）について、Backend・OS 別に `Pass` / `Fail` / `Unsupported` / `NotRun` / `Blocked` の判定語彙で記録する。実機未検証の項目は `NotRun` または `Unsupported` とし、他経路の成功を転用しない。
4. **パッケージ形式比較**: `deb` / `AppImage` / `Flatpak` の各形式について、同梱依存 vs システム依存、sandbox/portal、更新・インストール、OS/Driver 制約、Native Plugin Dock 適合、失敗時 fallback の条件を比較し、採用 / 保留 / 対象外の判断根拠を記録する。未実測の形式を配布可能と断定しない。
5. **旧 WebView/WebKitGTK の分離**: 旧経路（`wry 0.57` / `tao 0.37` / WebKitGTK 4.1 / GTK3）の runtime・Browser 検証知見を GPUI 配布の現行必須条件から除外し、レガシー経路が残る間の依存は Issue #465 の撤去計画と分離して明示する。削除済みとは主張しない。
6. **Owner 境界**: Ruviform 共通基盤（runtime/ABI/packaging 要件・再利用可能な Capability Matrix・共通 GUI/NativeSurface 制約）と、AudioLens・Synvelis 各製品（製品固有の配布形式・署名・同梱方針・対応 OS 範囲・リリース workflow・製品レベル受入）の責務を区別し、Ruviform の判断が両 Consumer に暗黙の新規必須依存を生じさせないことを保証する。
7. **確認手順と証拠連携**: 実行可能な確認手順（コンテナ lane・実機 matrix）と判定語彙を文書化し、実機・Native Plugin Dock 受入は #419/#262、Consumer 非回帰は #294 の独立証拠へリンクする。

## Why Now

- Issue #127 の 2026-11-09 改訂で、Linux GUI/OS/Native Plugin Host の runtime 能力と packaging 前提を文書化する責務が Ruviform 側に明示された。GPUI 標準 Driver（Issue #384 系 ADR で提案・条件付き採用方向）の配布可否は、実測 capability とパッケージ形式適合の両方が揃うまで確定できない。
- deb / AppImage / Flatpak の比較は対象 OS 選定前に開始可能だが、最終採用・サポート宣言には対象 OS/Backend/OS 機能の決定と実機証拠が必要。先に適合条件と判断根拠を文書化しておくことで、Consumer（AudioLens / Synvelis）が独立に配布方式を決定する際の共通基盤側入力が揃う。
- 旧 WebView/WebKitGTK 経路の知見（Issue #57・#28・ADR `ui-tech-selection-poc`・`poc/ui/RESULTS.md`）は歴史的参照であり、GPUI 配布の必須条件と混同すると検証語彙が壊れる。#465 の撤去計画と分離して現行条件を固定する必要がある。

## Users and Value

- **Ruviform 保守者 / 設計レビュアー**: Linux 配布の前提条件・未対応範囲・確認手順が一箇所の正本に集約され、Issue #127 の受入が文書単位で追跡できる。
- **AudioLens / Synvelis の実装者・リリース担当**: 共通基盤が提供する Capability Matrix・runtime/ABI/packaging 条件を入力として、自製品の配布形式・署名・同梱方針・対応 OS を独立に決定できる。Ruviform 側の判断が暗黙の必須依存を生まない。
- **検証・受入担当（#262 / #419 / #294 lane）**: どの capability が実測済みでどれが `NotRun`/`Unsupported` かを判定語彙付きで参照でき、実機受入の対象範囲を誤認しない。

## Scope

### In Scope

- Issue #127 本文の受入 5 行を全て設計文書へ転記・具現化する docs チェーン 5 文書（spec / design / test-spec / plan / task）と、それに伴う `docs/*/README.md` 管理索引の再生成。
- 対象ディストリビューション/バージョンの**現状整理**（未確定であることの明示と、決定に必要な評価軸の列挙）。実際の distro 選定は本 Issue の範囲外。
- X11 / Wayland / XWayland Backend 範囲の整理と、GPUI 0.2.2 の Linux 実行時依存（`Cargo.lock` / `docs/designs/issue-386-driver-neutral-gui-host.md` 由来）の Inventory 化。
- GUI capability・runtime 依存・パッケージ形式適合条件の Matrix 化（実測/未実測/宣言/非対応/未実行の区別）。
- deb / AppImage / Flatpak の比較と採用/保留/対象外の判断根拠（未実測形式を配布可能と断定しない）。
- 旧 WebView/WebKitGTK 4.1 固有事項の分離記述と #465 との責務境界。
- Ruviform / AudioLens / Synvelis の Owner 境界と、暗黙の新規必須依存を生じさせない条件。
- 実行可能な確認手順（既存 `ci/verify-linux` コンテナ lane の位置づけ、実機 matrix、判定語彙）と #419/#262/#294 への証拠リンク。

### Out of Scope

- `crates/`・`Cargo.toml`/`Cargo.lock`・`mise.toml`・`.github/` を含む一切のソースコード・テスト・CI・依存変更（docs 設計専用タスク）。
- 対象ディストリビューション/バージョンの**最終決定**、および Linux 実機での GPUI 起動・入力・DPI/IME・Native Plugin Dock の実測（全て `NotRun` として記録し、実施は後続 Issue/実機 lane の責務）。
- パッケージ形式（deb / AppImage / Flatpak）の**最終採用宣言**および各 Consumer 製品の配布方式・署名・同梱方針の決定（Consumer repo 側の責務。AudioLens #127 / Synvelis #28 系）。
- 旧 WebView/WebKitGTK 依存の物理的撤去（Issue #465 の責務。本 spec は「削除済みとは主張しない」境界記述のみ）。
- GPUI 標準 Driver 自体の最終採否（Issue #262・#388・#389・#419 の独立証拠が前提。#384 ADR は proposed であり無条件の最終採用ではない）。
- Windows / macOS の配布設計、および Linux 以外のプラットフォーム固有 packaging。
- 既存 docs 監査で検出済みの無関係な既知 findings（例: `docs/discovery/al-media-support-matrix.md` の ID/status/source 形式、issue-313/issue-243/issue-410 の broken relation）の修復。

## Requirements

- **AC1 対応**: 対象ディストリビューション/バージョン（未確定を含む）、X11/Wayland 等の対象 Backend、GUI/フォント/IME/DPI/NativeSurface/実 Plugin GUI の必要 Capability・実行時依存を、実測/未実測を区別した Matrix として design 文書に記録する。
- **AC2 対応**: deb / AppImage / Flatpak それぞれについて、同梱・システム依存、sandbox/portal、更新/インストール、OS/Driver 制約、Plugin Native Dock・失敗時 fallback の適合条件を比較し、採用/保留/対象外の判断根拠を記録する。未実測の形式を配布可能と断定しない。
- **AC3 対応**: 旧 WebView/WebKitGTK 4.1 固有の runtime/Browser 検証を GPUI 配布の現行必須条件から除外し、既存 Legacy 経路が残る間の依存は #465 の撤去計画と分離して明示し、削除済みとは主張しない。
- **AC4 対応**: 共通基盤側の runtime/ABI/packaging 要件と AudioLens・Synvelis の製品固有の配布方式・署名・同梱方針の Owner を明確に区別し、Ruviform の判断によって両 Consumer に暗黙の新規必須依存を生じさせない。
- **AC5 対応**: 採否・対象 OS・未対応範囲・実行可能な確認手順・`Pass`/`Fail`/`Unsupported`/`NotRun`（必要に応じ `Blocked`）を文書化し、実機・Native Plugin Dock の受入は #419/#262、Consumer 非回帰は #294 の独立証拠へリンクする。
- **正直性要件**: 実施していない Linux 実機・Backend・パッケージ・Native Plugin Dock 検証を実施済みと記録しない。旧 WebView 経路の成功・fake/headless の成功を GPUI の実機成功へ転用しない。
- **docs-only 要件**: 変更は `docs/` 配下の文書と管理索引再生成に限定し、`crates/`・`Cargo.*`・`mise.toml`・`.github/` に触れない。

## Acceptance Criteria

Issue #127 本文の受入をそのまま正本とし、各行を design 文書の対応節へ写像する。

- [ ] (AC1) design 文書に、対象ディストリビューション/バージョン・X11/Wayland Backend・GUI/フォント/IME/DPI/NativeSurface/実 Plugin GUI の Capability・実行時依存を、実測/未実測区別付き Matrix として記録した節がある。
- [ ] (AC2) design 文書に、deb / AppImage / Flatpak の同梱・システム依存、sandbox/portal、更新/インストール、OS/Driver 制約、Plugin Native Dock・失敗時 fallback の適合条件比較と採用/保留/対象外の判断根拠があり、未実測形式を配布可能と断定していない。
- [ ] (AC3) design 文書が旧 WebView/WebKitGTK 4.1 固有の runtime/Browser 検証を GPUI 配布の現行必須条件から除外し、残存レガシー依存を #465 撤去計画と分離して明示し、削除済みと主張していない。
- [ ] (AC4) design 文書が Ruviform 共通基盤と AudioLens・Synvelis の Owner 境界を明確に区別し、暗黙の新規必須依存を生じさせない条件を記述している。
- [ ] (AC5) design 文書が採否・対象 OS・未対応範囲・実行可能な確認手順・判定語彙（`Pass`/`Fail`/`Unsupported`/`NotRun`/`Blocked`）を記録し、実機・Native Plugin Dock 受入を #419/#262、Consumer 非回帰を #294 の独立証拠へリンクしている。
- [ ] (構造) spec / design / test-spec / plan / task の 5 文書が front matter relations で相互リンクし、`docs/{specs,designs,test-specs,plans,tasks}/README.md` 管理索引が再生成済みで、新規文書の audit error が 0。
- [ ] (構造) `mise run fmt-check` が pass し、diff が `docs/` のみである。

## Deferred Design Concerns

| 後送り項目 | 理由 | 追跡先 |
| --- | --- | --- |
| 対象ディストリビューション/バージョンの最終決定 | Issue #127 は条件・比較の記録が責務。実測なしに distro を断定しない | Issue #127 本体 / #262 |
| Linux 実機での GPUI 起動・入力・DPI/IME・Native Plugin Dock 検証 | 実機環境と Backend が未確定。全て `NotRun` | Issue #419 / #262 / #388 / #389 |
| deb / AppImage / Flatpak の最終採用 | 未実測形式を配布可能と断定しない。実機+Consumer 決定が前提 | Issue #127 本体 / AudioLens #127 / Synvelis #28 |
| 旧 WebView/WebKitGTK 依存の物理的撤去 | レガシー経路が残る間は依存も残る。撤去は独立 Issue | Issue #465 |
| `ci/verify-linux` への GPUI lane 追加 | docs 設計タスクの範囲外。確認手順として位置づけのみ記録 | 後続実装 Issue |

## Sources and Relations

- Issue #127 本文（2026-11-09 改訂・Linux 実配布方式 — パッケージ/コンテナ/依存・GPUI ネイティブ面・consumer 配布影響）を正本として確認。
- 上流 ADR: `docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md`（GPUI 標準 Driver の提案・条件付き採用方向、status proposed）、`docs/adr/platform-and-toolchain-targets.md`（desktop 3 OS・x86_64+aarch64・stable toolchain）、`docs/adr/licensing-and-distribution-policy.md`（license 未決定・publish=false 原則）。
- 関連 spec/design: `docs/specs/issue-384-driver-neutral-gui-architecture.md`、`docs/designs/issue-257-driver-neutral-ui-contract.md`（Driver-neutral 全体契約）、`docs/designs/issue-386-driver-neutral-gui-host.md`（GPUI 0.2.2・Linux 依存の一次整理）、`docs/specs|designs/issue-388-native-surface-slot.md`（NativeSurfaceSlot / L0・L1 Dock matrix）、`docs/specs|designs/issue-389-plugin-editor-session.md`（PluginEditorSession / Native Dock）、`docs/specs/issue-294-cross-consumer-candidate-gate.md`（Consumer 非回帰証拠）、`docs/specs|designs|adr/issue-465-*`（legacy WebView 撤去境界）、`docs/designs/issue-115-l1-gui-cross-process-parenting.md`、`docs/designs/issue-243-gpui-realwin-windows.md`。
- 歴史的参照（現行必須条件ではない）: `docs/adr/ui-tech-selection-poc.md`（superseded・wry 0.57/tao 0.37/WebKitGTK 4.1 採用の旧判断）、`poc/ui/RESULTS.md`（旧 WebView capability/packaging matrix）、`docs/impl/exp/linux-container-device-and-arch-verification.jsonl`（#57 旧 GTK 経路のコンテナ検証記録）、`ci/verify-linux/Dockerfile`（既存 Linux 検証コンテナ）。
- 関連 Issue: #57（旧 Linux 配布選定の前身 tracker）、#28（UI PoC）、#419（GPUI/OS/backend・Native Plugin Dock 証拠）、#262（GPUI Native 採用 gate）、#388/#389（Native Dock / PluginEditorSession）、#465（legacy WebView 撤去）、#294（cross-consumer 非回帰）。
