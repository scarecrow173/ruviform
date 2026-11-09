---
id: "DESIGN-034udQraUSrCxCnhZza67E"
type: "design"
status: "draft"
title: "Issue #127 Linux 実配布方式 設計 — GPUI 標準 Driver の runtime 依存・Capability Matrix・deb/AppImage/Flatpak 適合条件"
created: "2026-11-09"
updated: "2026-11-09"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/127"
    - "docs/specs/issue-127-linux-dist-design.md"
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
  implements:
    - "docs/specs/issue-127-linux-dist-design.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-127-linux-dist-design.md"
    - "docs/specs/issue-384-driver-neutral-gui-architecture.md"
    - "docs/designs/issue-384-driver-neutral-gui-contract.md"
    - "docs/designs/issue-386-driver-neutral-gui-host.md"
    - "docs/designs/issue-388-native-surface-slot.md"
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
    - "docs/designs/issue-115-l1-gui-cross-process-parenting.md"
    - "docs/designs/issue-243-gpui-realwin-windows.md"
    - "docs/designs/issue-465-legacy-webview-p0p1.md"
    - "docs/discovery/issue-388-389-native-dock-current-state.md"
    - "docs/reports/issue-431-real-verify.md"
    - "docs/adr/ui-tech-selection-poc.md"
    - "docs/adr/issue-465-legacy-webview-retirement-status.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-127-linux-dist-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-127-linux-dist-design.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/419"
    - "https://github.com/scarecrow173/ruviform/issues/262"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/465"
    - "https://github.com/scarecrow173/ruviform/issues/57"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/synvelis/issues/28"
    - "docs/impl/exp/linux-container-device-and-arch-verification.jsonl"
    - "poc/ui/RESULTS.md"
    - "ci/verify-linux/Dockerfile"
  defers: []
  deferred-by: []
---

# Issue #127 Linux 実配布方式 設計 — GPUI 標準 Driver の runtime 依存・Capability Matrix・deb/AppImage/Flatpak 適合条件

## Context and Positioning

本設計は Issue #127「Linux 実配布方式の設計」の正本であり、Ruviform の GPUI 標準 Driver を Linux 実環境へ配布する際の前提条件・能力・制約を文書化する。docs のみの設計タスクであり、コード・テスト・CI・依存の変更は行わない。

前提となる確定済み境界:

- **GPUI 標準 Driver は暫定的な現行設計ベースライン**であり、無条件の最終採用ではない。採否・対象 OS/Backend/形式の最終確定は Issue #262（GPUI Native 採用 gate）・#388/#389（NativeSurfaceSlot / PluginEditorSession・Native Dock 証拠）・#419（GPUI/OS/backend・Native Plugin Dock 証拠）の独立証拠に委ねる。根拠 ADR `docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md` は `proposed`。
- **Driver-neutral UI Contract**（`docs/designs/issue-257-driver-neutral-ui-contract.md`）が上位の共通契約であり、本設計はその Linux 配布面の詳細化である。
- **旧 WebView 経路**（`wry 0.57` / `tao 0.37` / WebKitGTK 4.1 / GTK3）は歴史的経路であり、物理的撤去は Issue #465 が所有する。撤去完了までは依存が残るが、GPUI 配布の必須条件ではない（§6）。
- **Consumer 非回帰**は Issue #294 の独立証拠 lane が所有し、本設計の「対応済み」をもって代替しない。

## Evidence Vocabulary and Honesty Rules

本設計および関連証拠で使う判定語彙を固定する。

| 語彙 | 意味 |
| --- | --- |
| `Pass` | 対象 OS/Backend/形式/revision で実測され、契約を満たした記録がある |
| `Fail` | 対象環境で実測され、契約を満たさなかった記録がある |
| `Unsupported` | 規格・API・プラットフォーム機構上その経路が存在しない、または現行実装が明示的に非対応と宣言している |
| `NotRun` | 実行可能だが未実施。実機・Backend・パッケージ・Dock の各 gate がこれに該当する現状 |
| `Blocked` | 前提証拠・環境・依存 Issue が未充足で実行不能 |

ルール:

- `NotRun` / `Unsupported` / `Blocked` を `Pass` に転用しない。fake 層・headless・他 OS・他 Driver・旧 WebView 経路の成功を GPUI Linux 実機の成功として扱わない。
- 実測を行う際は対象 Linux ディストリビューション/バージョン・Backend（X11 / Wayland / XWayland）・パッケージ形式・検証 revision を証拠に記録する義務がある。
- 「文書・lockfile 由来」は依存の**存在宣言**であり、実行時動作の保証ではない。本設計では `Declared`（宣言）として実測（`Pass`/`Fail`）と区別する。

## Scope

### In Scope

- 対象ディストリビューション/バージョンと X11/Wayland Backend の現状整理（未確定の明示と評価軸）。
- GPUI 0.2.2 および `ruviform-ui-gpui` の Linux 実行時依存 Inventory（宣言レベル）。
- GUI capability・runtime 依存・パッケージ形式適合条件の Matrix。
- deb / AppImage / Flatpak の比較と採用/保留/対象外の判断根拠。
- 旧 WebView/WebKitGTK の分離、Owner 境界、確認手順、証拠リンク。

### Out of Scope

- 対象 distro の最終決定、Linux 実機検証の実施、パッケージ形式の最終採用、Consumer 製品の配布決定、WebView 撤去の実装、GPUI 最終採否、非 Linux packaging、無関係な既存 audit findings の修復（spec 側 Out of Scope と同一）。

## 1. Target OS / Backend — 現状整理

### 1.1 対象ディストリビューション/バージョン

**現時点では未確定（`NotRun`）**。Issue #127 は「対象ディストリビューション/バージョンを記録する」ことを求めるが、実測なしに特定 distro を支持対象として宣言することは本設計の正直性ルールに反する。以下の評価軸を決定時の入力として固定する。

| 評価軸 | 確認すべき内容 | 現状 |
| --- | --- | --- |
| glibc / ベースライン | `rust:1-trixie` 系コンテナや各 distro の glibc が Rust 1.99.0 toolchain・GPUI 依存（`x11rb`・`wayland-*`・`ash` 等）の最低要件を満たすか | 未実測。`docs/adr/platform-and-toolchain-targets.md` の stable toolchain 方針（MSRV 1.89・pin 1.99.0）に整合する下限確認が必要 |
| Vulkan ICD / GPU ドライバ | `blade-graphics`（ash/gpu-alloc/naga）が要求する Vulkan loader + ICD が対象 distro の標準ドライバで提供されるか。GPU 非依存 foundations の例外として GUI Driver は GPU 前提 | 未実測。Mesa（RADV/ANV）・各ベンダドライバの存在確認が必要 |
| EGL / GL | `khronos-egl`・`glow` 経路の libEGL/libGL 存在 | 未実測 |
| fontconfig / フォント | `fontconfig-parser`・`fontdb`・`cosmic-text` 系のフォント列挙が distro 標準の fontconfig + 最低 1 フォントで成立するか | 未実測。`ci/verify-linux` は `fonts-liberation` を同梱済み（旧 GTK 経路向け） |
| IME | X11 で `zed-xim`（XIM）経由の入力メソッドが機能するか。Wayland の IME（text-input protocol）は GPUI 側対応状況に依存 | 未実測・`NotRun` |
| xdg-desktop-portal | `ashpd`/`zbus`/`oo7` が使う portal D-Bus API と対象 distro の portal 実装（xdg-desktop-portal-gtk/kde）の対応 | 未実測 |
| Native Plugin Dock | §4 の XEmbed/XWayland 機構が対象 distro の X11/Wayland 環境で利用可能か | 未実測。#388 matrix 参照 |

旧 WebView 経路の文書評価（`poc/ui/RESULTS.md`）に Ubuntu 22.04+/Debian 12+/Fedora 38+ の例があるが、**これは wry/WebKitGTK 4.1 の評価であり GPUI の distro 下限を意味しない**。GPUI の distro 下限は上記軸の実測で別途決定する。

### 1.2 対象 Backend 範囲

GPUI 0.2.2 は default features で `wayland`・`x11` を有効化する（`docs/designs/issue-386-driver-neutral-gui-host.md` 参照）。Backend の整理:

| Backend | GPUI 宣言 | 現状判定 | 備考 |
| --- | --- | --- | --- |
| X11 ネイティブ | `x11` feature 有効 | `NotRun` | `x11-dl`（dlopen）+ `x11rb`/`xcb`/`xkbcommon`/`zed-xim` 経路。Xvfb での起動すら未実施 |
| Wayland ネイティブ | `wayland` feature 有効 | `NotRun` | `wayland-client`/`wayland-backend`/`calloop`/`xkbcommon` 経路。 compositor・portal 依存 |
| XWayland（X11 app on Wayland session） | X11 経路の一形態 | `NotRun` | Native Plugin Dock の L0 XEmbed が XWayland 経由で機能するかは #388 matrix で `NotRun` |

Backend の最終サポート宣言は実機 matrix（§8）の結果を待つ。現時点で「Wayland のみ」「X11 のみ」等の絞り込みは行わない。

## 2. Runtime Dependency Inventory（宣言レベル）

`Cargo.lock` と `docs/designs/issue-386-driver-neutral-gui-host.md` に基づく GPUI 0.2.2 の Linux 依存を、パッケージングへの影響とともに整理する。**全て `Declared`（宣言）であり、実行時動作は未実測**。

### 2.1 グラフィックス / GPU

| クレート | 役割 | 実行時システム依存（推定・宣言） | パッケージング含意 |
| --- | --- | --- | --- |
| `blade-graphics 0.7.1` | GPUI の描画基盤（Vulkan 主経路） | `libvulkan.so.1`（loader）+ GPU ICD（Mesa RADV/ANV・ベンダドライバ） | ICD は GPU/ドライバ由来のため同梱不可。Vulkan loader の有無確認が必須。未導入環境では起動不可の可能性（`NotRun`） |
| `ash` / `gpu-alloc` / `gpu-alloc-ash` / `naga` | Vulkan API・確保・シェーダ変換 | 同上（`ash` は `libvulkan` を dlopen） | loader のみ必要。ICD はシステム依存 |
| `khronos-egl` / `glow` | EGL/GL サーフェス・フォールバック経路 | `libEGL.so.1` / `libGL.so.1` | Wayland/X11 surface 確立に必要な可能性。blade-graphics が Vulkan 主・GL 従の構成かは未検証 |
| `ash-window` / `raw-window-handle` | ウィンドウハンドル受け渡し | なし（型のみ） | — |

### 2.2 ウィンドウ / イベントループ / Wayland・X11

| クレート | 役割 | 実行時システム依存（推定・宣言） | パッケージング含意 |
| --- | --- | --- | --- |
| `wayland-client` / `wayland-backend` / `wayland-cursor` / `wayland-scanner` / `wayland-protocols` / `wayland-protocols-plasma` | Wayland client・プロトコル | `libwayland-client.so`・`libwayland-cursor.so`・`libxkbcommon.so`（wayland-backend の dlopen feature 依存を要確認） | distro 標準パッケージで充足可能性が高いが未実測 |
| `calloop` / `calloop-wayland-source` | Wayland イベントループ | `libwayland-client` 経由 | 同上 |
| `x11rb` / `x11rb-protocol` / `xcb` / `as-raw-xcb-connection` / `x11-dl` | X11 プロトコル・接続 | `libX11.so.6`・`libxcb.so.1`・`libXcursor`/`libXi`/`libXrandr` 等（`x11-dl` は dlopen） | `x11-dl` は実行時 dlopen のためリンク時依存にならず、実行時に各 `.so` が必要。バージョン寛容性は高いが未実測 |
| `xkbcommon` | キーマップ・レイアウト | `libxkbcommon.so.0` | Wayland/X11 双方で必要。distro 標準で充足可能性高い |
| `zed-xim` | X11 IME（X Input Method）client | `libX11` + 実行時の IME サーバ（ibus/fcitx 等） | IME 実動作はサーバ側依存。`NotRun` |

### 2.3 テキスト / フォント / IME

| クレート | 役割 | 実行時システム依存（推定・宣言） | パッケージング含意 |
| --- | --- | --- | --- |
| `cosmic-text` / `rustybuzz` / `swash` / `etagere` | テキスト shaping・レイアウト・グリフ atlas | なし（pure Rust）+ fontconfig 経由のフォント列挙 | — |
| `fontdb` / `fontconfig-parser` / `yeslogic-fontconfig-sys` | フォント DB・fontconfig 解析 | `libfontconfig.so.1` + フォントファイル（`fonts-*`） | fontconfig と最低 1 フォントが必須。フォント非同梱ならシステムフォント依存 |
| `font-kit`（GPUI default feature） | プラットフォームフォント API | Linux では fontconfig 経路 | feature 構成依存 |
| `resvg` / `usvg` | SVG 描画 | pure Rust 主体（フォントは fontconfig 経由） | — |

### 2.4 Portal / D-Bus / IME 補助

| クレート | 役割 | 実行時システム依存（推定・宣言） | パッケージング含意 |
| --- | --- | --- | --- |
| `ashpd 0.11.1` | xdg-desktop-portal 呼出（file chooser 等） | `xdg-desktop-portal` + backend（gtk/kde）+ D-Bus session | portal が無い/未起動の環境では該当機能が `Fail`/`Unsupported` になる可能性。sandbox 形式では portal が必須経路 |
| `zbus` / `zbus_macros` / `zbus_names` / `dbus` | D-Bus client（pure Rust） | D-Bus session bus（`dbus-daemon`/`dbus-broker`） | Wayland session では通常存在。コンテナでは `dbus-x11` 等の同梱が必要（`ci/verify-linux` が `dbus dbus-x11` を導入済み） |
| `oo7` | Secret Service portal（資格情報） | `xdg-desktop-portal` の Secret Service 実装 | 使用有無は GPUI の機能依存。未使用なら不要の可能性（`NotRun`） |

### 2.5 まとめ — パッケージングへの含意

- **同梱候補**: GPUI バイナリ本体 + Rust 依存は静的リンクで同梱される。`libvulkan` loader・`libfontconfig`・フォント・`libxkbcommon`・`libX11`/`libxcb`・`libwayland-*`・`libEGL`/`libGL`・D-Bus/portal は**実行時システム依存**として残る可能性が高い。
- **ICD/GPU ドライバは同梱不可**: Vulkan ICD は GPU ベンダ・カーネルモジュールに密結合しており、deb/AppImage/Flatpak のいずれでも同梱は現実的でない。対象 OS の GPU ドライバ提供が前提条件になる。
- **フォントはポリシー次第**: 同梱すれば再現性が上がるがサイズ増。非同梱ならシステムフォント依存で描画差が出る可能性。
- **`x11-dl`/`ash` の dlopen**: リンク時依存を持たないため deb の `Depends` に現れにくいが、実行時には各 `.so` が必要。`ldd` だけでは不足を検出できない点に注意（検証手順 §8 に反映）。

## 3. GUI Capability Matrix

対象 OS（Linux・distro 未確定）× Backend（X11 / Wayland / XWayland）× capability の判定表。**現時点で Linux 実機の `Pass` は 1 件もない**。`Declared` は設計・lockfile 上の存在宣言、`Unsupported` は機構上の非対応、`NotRun` は未実施。

凡例: `D`=Declared（宣言のみ・未実測）、`U`=Unsupported、`N`=NotRun、`B`=Blocked、`P`/`F`=Pass/Fail（実測・現在該当なし）。

| Capability | X11 | Wayland | XWayland | 根拠・備考 |
| --- | --- | --- | --- | --- |
| ウィンドウ生成・ライフサイクル | N | N | N | GPUI `Application`/`Window` 経路は宣言済み（`D`）だが Linux 実機起動は未実施。#243 は Windows 実測のみ |
| 複数ウィンドウ | N | N | N | 同上。Linux での multi-window 実測なし |
| 入力（keyboard/mouse） | N | N | N | `x11rb`/`wayland-client` 経路は宣言済み。実入力イベント未観測 |
| フォーカス | N | N | N | 同上 |
| IME | N | U? | N | X11 は `zed-xim`（XIM）経路が宣言済みだが実動作 `NotRun`。Wayland の IME は text-input protocol 対応が GPUI 側で未確認のため `U?`（要検証・現時点では `Unsupported` 寄りだが断定しない） |
| フォント描画 | N | N | N | `cosmic-text`/`fontdb`/`fontconfig` 経路は宣言済み。フォント列挙・レンダリング品質は未実測 |
| DPI / スケーリング | N | N | N | GPUI の scale factor 処理は宣言済み。HiDPI 実表示未観測 |
| NativeSurfaceSlot（L0: 自プロセス内 surface） | N | N | N | `native_surface_slot` capability flag は存在。Linux での実 surface 確保は未実施 |
| Native Plugin Dock L0（VST3/CLAP 埋込） | N | U | N | #388 matrix: X11 は `XEmbed`/`X11EmbedWindowID`・CLAP `x11` XID 経路が存在（`NotRun`）。Wayland ネイティブは CLAP/VST3 規格に埋込機構がなく `Unsupported` |
| Native Plugin Dock L1（クロスプロセス GUI 親子） | U | U | U | #388 matrix: X11 は GtkSocket/Plug（GTK3 deprecated・GTK4 撤去）で現行 `Unsupported`。XWayland/Wayland も同様に `Unsupported`（現行経路なし）。要再評価 |
| 実 Plugin GUI（#389 PluginEditorSession） | N | N | N | 実 plugin GUI の完了済み実装証拠なし。`NotRun` |
| a11y（アクセシビリティ） | D | D | D | accesskit 系は Cargo.lock で未確認。ADR capability matrix では declared。実対応は未検証 |
| GPU 非依存 fallback | N | N | N | GPU 無し/失敗時の software fallback 経路は未確認。`blade-graphics` が Vulkan 必須か GL fallback を持つか未検証 |

補足:

- `U?` は「現時点で `Unsupported` と断定はしないが `Pass` でもない」保留表記。最終判定は実機 matrix で行う。
- Native Plugin Dock の詳細行列（OS×Level×plugin 規格）は `docs/designs/issue-388-native-surface-slot.md` の D6 matrix が正本であり、本表はその Linux 行を配布設計の観点で再掲したもの。矛盾時は #388 側を正とする。
- Linux での `Pass` が 0 件であることは設計上の欠陥ではなく、実機検証が後続 Issue に委ねられている現状の正直な記録である。

## 4. Package Format Comparison — deb / AppImage / Flatpak

3 形式の適合条件を、Issue #127 が要求する 7 観点で比較する。**全形式とも現時点では `保留`（採用保留）** であり、未実測の形式を配布可能と断定しない。

### 4.1 比較表

| 観点 | deb | AppImage | Flatpak |
| --- | --- | --- | --- |
| **同梱依存 vs システム依存** | システム依存中心。`Depends` で `libvulkan1`・`libfontconfig1`・`libxkbcommon0`・`libx11-6`・`libxcb1`・`libwayland-client0`・`libegl1`/`libgl1`・`xdg-desktop-portal` 等を宣言する設計が自然。GPU ICD は依存に含められない（ドライバ依存）。フォントは `fonts-*` 依存 or 同梱の選択 | 実行時依存を同梱しやすい（AppDir に `.so` を同梱可能）。ただし Vulkan ICD・カーネルモジュールは同梱不可。`x11-dl`/`ash` の dlopen 依存は同梱で充足可能 | runtime（org.freedesktop.Platform 等）に依存を集約。同梱は `--filesystem`/extension で限定的。portal 前提の設計が必要 |
| **sandbox / portal** | sandbox なし。システム権限で動作。portal は任意利用（file chooser 等で ashpd が使う場合は portal 必須） | sandbox なし（AppImage 自体は単なる実行ファイル）。bubblewrap 等で自前 sandbox 化は可能だが標準では非 sandbox | **sandbox 必須**。xdg-desktop-portal 経由の file chooser・Secret Service 等が必須経路。直接の X11/Wayland socket・D-Bus アクセスは portal/permission で制限される |
| **更新 / インストール** | `apt`/`dpkg` のパッケージ管理に統合。署名は distro の apt 署名または独自 repo。更新はパッケージ更新に従う | 単一実行ファイル。更新はアプリ側または AppImageUpdate 等の外部機構。署名は任意（GPG/署名付き AppImage）。インストール概念が薄い | `flatpak install/update` に統合。署名は Flathub または独自 repo の GPG。sandbox 内更新は runtime と分離 |
| **OS / Driver 制約** | 対象 distro の apt repo・アーキテクチャ（x86_64/aarch64）・glibc に依存。deb は distro 毎にビルドまたは汎用ビルドの glibc 下限管理が必要 | glibc 下限をビルド環境で制御可能（古い distro でビルドすれば広く動く）。アーキテクチャ別 AppImage が必要 | runtime（Freedesktop/KDE）が対象 distro の基盤を規定。distro ネイティブ版と runtime 版の二重サポートが発生し得る |
| **Native Plugin Dock 適合** | sandbox なしのため XEmbed/XWayland の直接利用に制約が少ない。L0 XEmbed（X11）は原理上可能（`NotRun`）。L1 は現行 `Unsupported` | 同上（非 sandbox）。X11/XWayland での XEmbed 利用に sandbox 障害がない | **sandbox が XEmbed/直接ウィンドウ埋込を阻害する可能性が高い**。X11 socket アクセス・XWayland 利用は portal/permission で制限され、Native Plugin Dock L0 が機能しない可能性。要実機検証だが構造的リスク大 |
| **失敗時 fallback** | 依存不足時は `apt` が依存解決で未然に防ぐか、起動時に共有ライブラリ不足で `Fail`。Vulkan ICD 不在時は GPU 初期化失敗 | 同梱依存で起動は安定しやすいが、ICD/カーネル依存は残る。`--appimage-extract` でのデバッグ容易 | sandbox 内で依存不足は runtime が吸収するが、portal/backend 不在で機能 `Fail`/`Unsupported` になる。fallback は portal 非依存経路の確保が必要 |

### 4.2 判断根拠（採用/保留/対象外）

| 形式 | 現時点の判断 | 根拠 |
| --- | --- | --- |
| **deb** | **保留** | システム依存モデルは GPUI の実行時依存（§2.5）と整合し、Native Plugin Dock の XEmbed 経路を阻害しない点で最も制約が少ない。ただし対象 distro 未確定・実機未検証のため採用は保留。distro 毎のパッケージングコストと apt 署名/repo 体制の整備が前提 |
| **AppImage** | **保留** | 同梱依存で配布の手軽さはあるが、Vulkan ICD・フォント・portal の扱い・署名/更新機構が未設計。Native Plugin Dock への sandbox 障害はないが、実機検証なしに採用は保留 |
| **Flatpak** | **保留（Native Plugin Dock 適合に構造的疑義）** | sandbox/portal モデルはセキュリティ上望ましいが、XEmbed/直接ウィンドウ埋込を要求する Native Plugin Dock L0 との適合が最も疑わしい。portal 経由で実現できるかは未検証。Native Dock が必須要件の Consumer では `対象外` になり得るが、Consumer が Dock 非依存なら `保留` のまま検討余地あり |

**重要**: 上記は全て `保留` であり、**未実測の形式を配布可能と断定しない**。最終採用は (a) 対象 OS/Backend 決定、(b) 実機 capability matrix の `Pass` 証拠、(c) 各 Consumer の製品要件（Dock 必須か・署名・リリース workflow）の三点が揃ってから行う。

### 4.3 形式決定に必要な今後の証拠

- 各形式での GPUI 起動・入力・フォント・DPI/IME の実機 `Pass`/`Fail`。
- Flatpak sandbox 下での Native Plugin Dock L0（XEmbed or portal 経由）の可否。
- deb の `Depends` 最小集合の実測（`lddtree`/`ldd` + dlopen 依存の手動確認）。
- AppImage の同梱 `.so` セットと glibc 下限の実測。
- 署名・更新・インストール UX の Consumer 要件との照合。

## 5. Ownership Boundary — Ruviform / AudioLens / Synvelis

Issue #127 の AC4 に対応し、責務を明確に分離する。

| 領域 | Owner | 内容 | 本設計での扱い |
| --- | --- | --- | --- |
| 再利用可能な Capability Matrix | **Ruviform** | OS/Backend/capability の判定表・判定語彙・実測/未実測の区別（§3） | 本設計で正本化 |
| runtime/ABI/packaging 要件 | **Ruviform** | GPUI の実行時依存・同梱/システム依存の分類・パッケージ形式の適合条件（§2・§4） | 本設計で条件を記録。最終採用は保留 |
| 共通 GUI / NativeSurface 制約 | **Ruviform** | Driver-neutral UI Contract・NativeSurfaceSlot の共通契約 | `issue-257`/`issue-388` に委譲。本設計は Linux 配布面のみ |
| 製品固有の配布形式 | **AudioLens / Synvelis** | 各製品が deb/AppImage/Flatpak/その他から選択。Ruviform は形式を強制しない | Consumer repo 側 Issue で決定 |
| 署名・同梱方針 | **AudioLens / Synvelis** | コード署名・同梱アセット・ライセンス表記等 | Consumer 責務。Ruviform の license 未決定（`licensing-and-distribution-policy` ADR）は配布物の license 表記に影響するため Consumer は留意 |
| 対応 OS 範囲の製品宣言 | **AudioLens / Synvelis** | 製品としてサポートする distro/バージョンの宣言 | Consumer 責務。Ruviform は評価軸のみ提供 |
| リリース workflow | **AudioLens / Synvelis** | ビルド・署名・公開・更新の workflow | Consumer 責務 |
| 製品レベル受入 | **AudioLens / Synvelis** | 製品としての受入基準 | Consumer 責務。Ruviform の capability matrix を入力として使う |
| 実機・Native Dock 受入 | **#419 / #262 / #388 / #389** | GPUI/OS/backend・Native Plugin Dock の独立証拠 | 本設計はリンクのみ。判定の実施は各 Issue |
| Consumer 非回帰 | **#294** | AudioLens/Synvelis の候補 tuple に対する非回帰 lane | 本設計はリンクのみ |
| legacy WebView 撤去 | **#465** | `wry`/`tao`/WebKitGTK 依存の物理的撤去 | 本設計は分離記述のみ |

**暗黙の必須依存を生じさせない条件**: 本設計は「deb を推奨」「Flatpak を必須」のような製品強制をしない。パッケージ形式の採用/保留/対象外は全て `保留` であり、各 Consumer が自製品の要件（Dock 必須・署名・対象ユーザ層）に基づいて独立に決定できる。Ruviform が提供するのは適合条件と判断根拠の**入力**であり、結論ではない。

## 6. Legacy WebView / WebKitGTK 4.1 の分離

Issue #127 の AC3 に対応する。

### 6.1 現行必須条件からの除外

旧 WebView/WebKitGTK 4.1 固有の runtime/Browser 検証（`docs/adr/ui-tech-selection-poc.md`・`poc/ui/RESULTS.md`・`docs/impl/exp/linux-container-device-and-arch-verification.jsonl` seq 10 の #57 Linux GTK 経路検証）は、**GPUI 配布の現行必須条件ではない**。これらの知見は:

- `wry 0.57` + `tao 0.37` + `libwebkit2gtk-4.1-0` + GTK3 という**旧 Driver** の依存・能力・制約を記録したものである。
- GPUI（`gpui 0.2.2`）は WebKitGTK/GTK に依存しない。GPUI の Linux 依存は §2 の通り全く別系統（Vulkan/blade-graphics・wayland/x11・cosmic-text・fontconfig・portal）である。
- したがって「WebKitGTK 4.1 が動いた」「GTK3 が必要だった」という旧知見を GPUI の distro 下限・依存・capability の根拠にしてはならない。

### 6.2 残存レガシー依存の明示

旧 WebView 経路のコード・依存が**削除済みではない**。`Cargo.lock` には `glib`/`gtk`/`javascriptcore-rs`/`soup3`/`tao`/`webkit2gtk`/`wry` 等が残存しており、レガシー経路が機能している間はこれらの依存も残る。撤去は Issue #465 の責務であり、本設計は「削除済みとは主張しない」境界のみを記録する。

### 6.3 分離の記述ルール

- 本設計の §2・§3・§4 は **GPUI Driver のみ**を対象とし、WebKitGTK/GTK/wry/tao の依存・capability を一切含めない。
- 旧知見を参照する場合は「歴史的参照」と明示し、GPUI の `Pass`/`Fail`/`NotRun` 判定に混入させない。
- `ci/verify-linux/Dockerfile` は旧 GTK 経路（`libwebkit2gtk-4.1-dev`/`libgtk-3-dev`）を含むが、GPUI 検証には別途 GPUI 用依存（`libvulkan`/`libfontconfig`/`libxkbcommon`/`libwayland`/`libx11` dev パッケージ等）の追加が必要になる可能性がある。これは後続実装 Issue の検討事項であり、本 docs タスクでは変更しない。

## 7. Executable Verification Procedures

Issue #127 AC5 の「実行可能な確認手順」を定義する。現時点では全て `NotRun` だが、手順自体は確定する。

### 7.1 確認手順の階層

| 階層 | 環境 | 目的 | 現状 |
| --- | --- | --- | --- |
| L0: コンテナ（Xvfb/Wayland headless） | `ci/verify-linux` 系コンテナ（`rust:1-trixie` + Xvfb）を GPUI 用依存で拡張 | GPUI バイナリの**起動・ウィンドウ生成・基本描画**の smoke。実機 GPU/入力/IME/Dock は対象外 | 手順は既存コンテナを拡張して実現可能。未実施 `NotRun` |
| L1: 実機 Linux（X11 session） | 対象 distro の X11 環境 | X11 Backend での全 capability（入力・フォーカス・フォント・DPI/IME・NativeSurfaceSlot・Dock L0 XEmbed） | `NotRun`。#419/#262/#388/#389 の実機 lane で実施 |
| L2: 実機 Linux（Wayland session） | 対象 distro の Wayland compositor 環境 | Wayland Backend での全 capability + XWayland 経由 Dock | `NotRun`。同上 |
| L3: パッケージ形式別 | 各形式でビルドした成果物をクリーン環境へ install | deb/AppImage/Flatpak それぞれの起動・依存解決・Dock 適合・fallback | `NotRun`。形式採用後に実施 |

### 7.2 L0 コンテナ smoke の具体手順（将来実装用）

1. `ci/verify-linux/Dockerfile` を GPUI 用に拡張: `libvulkan1`・`mesa-vulkan-drivers`（llvmpipe/lavapipe）・`libfontconfig1`・`fonts-liberation`・`libxkbcommon0`・`libx11-6`・`libxcb1`・`libwayland-client0`・`libegl1`・`libgl1`・`xdg-desktop-portal`・`dbus-x11` を追加（正確なパッケージ名は対象 distro で確認）。
2. `cargo build -p ruviform-ui-gpui --features gpui --example real_os_smoke`（既存 example を Linux で実行）。
3. `xvfb-run` 下で起動し、ウィンドウ生成・数フレーム描画・終了を観測。
4. 結果を `Pass`/`Fail`/`Unsupported`/`NotRun` で記録し、対象 revision・distro・Backend を証拠に残す。
5. **重要**: L0 の `Pass` は「コンテナ内 Xvfb での起動成功」であり、実機 GPU・実入力・IME・Native Dock の `Pass` ではない。証拠には環境を明記する。

### 7.3 実機 matrix の記録要件

実機検証を行う際は以下を証拠に含める:

- 対象ディストリビューション/バージョン（例: Ubuntu 24.04、Debian 13、Fedora 41 等・実測時に確定）。
- Backend（X11 / Wayland / XWayland）。
- パッケージ形式（deb / AppImage / Flatpak / 直接バイナリ）。
- 検証 revision（Ruviform commit SHA）。
- 各 capability の `Pass`/`Fail`/`Unsupported`/`NotRun`/`Blocked` 判定。
- Native Plugin Dock は L0/L1 × plugin 規格（VST3/CLAP）別に記録。

### 7.4 証拠リンク

- 実機・Native Plugin Dock の受入: **Issue #419**（GPUI/OS/backend・Native Plugin Dock 証拠）・**Issue #262**（GPUI Native 採用 gate）・**Issue #388**（NativeSurfaceSlot）・**Issue #389**（PluginEditorSession）。
- Consumer 非回帰: **Issue #294**（cross-consumer candidate gate の証拠 lane）。
- 本設計の capability matrix が `Pass` になっても、#419/#262 の独立証拠が揃うまで GPUI 最終採用とはしない。

## 8. Risks and Deferred Concerns

| リスク / 懸念 | 内容 | 対応 |
| --- | --- | --- |
| GPUI Linux 実機未検証 | `Pass` が 0 件。起動すら未確認 | L0 コンテナ smoke → L1/L2 実機 matrix の順で検証。#419/#262 に委譲 |
| Vulkan ICD 非同梱 | GPU ドライバ依存のため全形式で ICD 同梱不可。GPU 無し環境での fallback 未確認 | §2.5・§3 GPU fallback 行。実機で ICD 有無別に検証 |
| Flatpak × Native Dock | sandbox が XEmbed/直接埋込を阻害する構造的疑義 | §4.1・§4.2。実機で portal 経由の可否を検証。不可なら Dock 必須 Consumer では `対象外` |
| `x11-dl`/`ash` の dlopen 見落とし | `ldd` で検出できない実行時依存 | §7.2 の依存確認に `LD_DEBUG`/`strace` 的手法を含める（実装時） |
| Wayland IME 未確認 | text-input protocol 対応が GPUI で未確認 | §3 IME 行 `U?`。実機 Wayland session で検証 |
| 旧 WebView 知見の混入 | WebKitGTK/GTK の成功を GPUI に転用する誤り | §6 の分離ルール。test-spec で検証 |
| license 未決定 | `Cargo.toml` の `license = "MIT"` は仮置き。配布物の license 表記が未確定 | `licensing-and-distribution-policy` ADR。Consumer の署名・同梱方針に影響 |
| gpui の `rust-version` 未宣言 | manifest に `rust-version` なし。edition 2024 → rustc ≥1.85。repo pin は 1.99.0 | MSRV 1.89 方針（`workspace-msrv` ADR）との整合は問題なし。GPUI 更新時に再確認 |

## 9. Design Summary — Issue #127 AC 写像

| Issue #127 AC | 本設計の対応節 | 状態 |
| --- | --- | --- |
| AC1: distro/Backend/capability/runtime 依存を実測/未実測区別 Matrix で記録 | §1（distro/Backend）・§2（runtime 依存）・§3（capability matrix） | 記録完了。実測は `NotRun` |
| AC2: deb/AppImage/Flatpak の適合条件比較と採用/保留/対象外の根拠。未実測を配布可能と断定しない | §4（比較表・判断根拠・必要証拠） | 記録完了。全形式 `保留` |
| AC3: 旧 WebView/WebKitGTK を現行必須条件から除外・#465 と分離・削除済みと主張しない | §6 | 記録完了 |
| AC4: Ruviform/Consumer の Owner 区別・暗黙の必須依存を生じさせない | §5 | 記録完了 |
| AC5: 採否・対象 OS・未対応範囲・確認手順・判定語彙・#419/#262/#294 リンク | §3（判定語彙）・§4（採否=保留）・§7（手順・リンク） | 記録完了 |

## Sources and Relations

- Issue #127 本文（正本）。関連: #57・#28・#419・#262・#294・#465・#388・#389。
- 上流: `docs/specs/issue-127-linux-dist-design.md`、`docs/specs/issue-384-driver-neutral-gui-architecture.md`、`docs/designs/issue-384-driver-neutral-gui-contract.md`、`docs/designs/issue-257-driver-neutral-ui-contract.md`、`docs/designs/issue-386-driver-neutral-gui-host.md`（GPUI 0.2.2・Linux 依存の一次整理）、`docs/designs/issue-388-native-surface-slot.md`（D6 matrix 正本）、`docs/designs/issue-115-l1-gui-cross-process-parenting.md`、`docs/designs/issue-243-gpui-realwin-windows.md`。
- ADR: `docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md`（proposed）、`docs/adr/platform-and-toolchain-targets.md`、`docs/adr/licensing-and-distribution-policy.md`、`docs/adr/ui-tech-selection-poc.md`（superseded・歴史的参照）、`docs/adr/issue-465-legacy-webview-retirement-status.md`。
- 歴史的参照: `poc/ui/RESULTS.md`、`docs/impl/exp/linux-container-device-and-arch-verification.jsonl`、`ci/verify-linux/Dockerfile`、`docs/discovery/issue-388-389-native-dock-current-state.md`、`docs/reports/issue-431-real-verify.md`。
- Consumer: AudioLens #127、Synvelis #28。
