---
id: "DESIGN-034X9FMLEAxQoslIlJ3YBV"
type: "design"
status: "approved"
title: "Linux 配布設計（Issue #127・CI 照合確定版）"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/127"
    - "docs/specs/linux-distribution.md"
    - "docs/designs/issue-127-linux-dist-design.md"
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
  implements:
    - "docs/specs/linux-distribution.md"
  implemented-by:
    - "docs/plans/linux-distribution-plan.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-127-linux-dist-design.md"
    - "docs/specs/issue-127-linux-dist-design.md"
    - "docs/test-specs/issue-127-linux-dist-design.md"
    - "docs/plans/issue-127-linux-dist-design.md"
    - "docs/tasks/issue-127-linux-dist-design.md"
    - "docs/impl/ir/gt-05-ci-offscreen-lanes-slice.md"
    - "evidence/rf388-os-l1-ci/report.md"
  refines:
    - "docs/designs/issue-127-linux-dist-design.md"
  refined-by: []
  derives-from:
    - "docs/specs/linux-distribution.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/linux-distribution.md"
  references:
    - "evidence/rf127/linux-ci-crosscheck.md"
    - "docs/specs/issue-127-linux-dist-design.md"
    - "docs/test-specs/issue-127-linux-dist-design.md"
    - "docs/plans/issue-127-linux-dist-design.md"
    - "docs/tasks/issue-127-linux-dist-design.md"
  defers: []
  deferred-by: []
---

# Linux 配布設計（Issue #127・CI 照合確定版）

## Context and Positioning

本設計は Issue #127「Linux 共通配布設計」の**確定版**であり、先行版 `docs/designs/issue-127-linux-dist-design.md`（PR #529・#573 で統合）を `refines` 関係で更新したものである。docs のみの設計タスクであり、コード・テスト・CI・依存・パッケージ成果物の変更は行わない。先行版と本版が矛盾する場合は本版を正とする。

本 slice で新たに確定した事実:

- **ubuntu-24.04 hosted runner の GPUI lane に成功実績がある**。`ui-gpui-verify (ubuntu-latest)` と `ui-gpui-offscreen-verify` を含む全 20 job が run [43439434737](https://github.com/scarecrow173/ruviform/actions/runs/43439434737)（headSha `365c74ac4b8e696044fb9bae386dfd03f86ecf32`）で success。`real_window` は 3 passed / 0 failed（102.92s）。先行版が `NotRun` と記録した「hosted 初回実行」は解消された。
- **現行 main HEAD（収集時点 `621490e8`）の CI は未実施**（`ci.yml` は workflow_dispatch 専用・daily-ci scheduler が dispatch）。`365c74ac` は祖先 SHA の証拠であり、`365c74ac..621490e8` に `crates/`・`mise.toml`・`ci.yml` の差分があるため HEAD の `Pass` とはしない（`NotRun(Pending)`）。
- **Linux OS-L1 は必須、GUI-L2/EXT-L3 は利用要件発生まで Deferred**（`POLICIES.md`）。本設計の `P(L0)`・headless CI の成功は L1 の一部証拠であり、実機 GUI/Plugin の `Pass` ではない。

前提となる確定済み境界（先行版から継承）:

- **GPUI 標準 Driver は暫定的な現行設計ベースライン**であり、無条件の最終採用ではない。採否・対象 OS/Backend/形式の最終確定は Issue #262・#388/#389・#419 の独立証拠に委ねる。
- **Driver-neutral UI Contract**（`docs/designs/issue-257-driver-neutral-ui-contract.md`）が上位の共通契約であり、本設計はその Linux 配布面の詳細化である。
- **旧 WebView 経路**（`wry`/`tao`/WebKitGTK 4.1/GTK3）は歴史的経路であり、物理的撤去は Issue #465 が所有する。撤去完了までは依存が残るが、GPUI 配布の必須条件ではない（§6）。
- **Consumer 非回帰**は Issue #294 の独立証拠 lane（`consumer-verify.yml`）が所有し、本設計の「対応済み」をもって代替しない。最新成功は run [43347745016](https://github.com/scarecrow173/ruviform/actions/runs/43347745016) @ ruviform_rev `3692c6fa`。

## Evidence Vocabulary and Honesty Rules

先行版の語彙を継承し、CI run 証拠の扱いを追加する。

| 語彙 | 意味 |
| --- | --- |
| `Pass` | 対象 OS/Backend/形式/revision で実測され、契約を満たした記録がある |
| `Fail` | 対象環境で実測され、契約を満たさなかった記録がある |
| `Unsupported` | 規格・API・プラットフォーム機構上その経路が存在しない、または現行実装が明示的に非対応と宣言している |
| `NotRun` | 実行可能だが未実施。CI 定義済みで対象 revision の run がない場合もこれに該当する |
| `Blocked` | 前提証拠・環境・依存 Issue が未充足で実行不能 |
| `Deferred` | 利用要件が成立するまで保留（POLICIES の検証レベル Deferred） |
| `P(L0)` | Xvfb/openbox/lavapipe の offscreen 環境でのみ実測 Pass。実機 `Pass` と区別する |

ルール:

- `NotRun`/`Unsupported`/`Blocked`/`Deferred` を `Pass` に転用しない。fake 層・headless・他 OS・他 Driver・旧 WebView 経路の成功を GPUI Linux 実機の成功として扱わない。
- CI job の green は job/step 成功の証拠であり、個別 capability の `Pass` はテスト名・実行数が run log で確認できるものに限定する。
- CI 証拠は SHA 固有である。対象 revision と異なる SHA の run 成功を現行 revision の `Pass` として引用しない（`evidence/rf127/linux-ci-crosscheck.md` §2 の SHA 対応表を正本とする）。
- 「文書・lockfile 由来」は依存の**存在宣言**（`Declared`）であり、実行時動作の保証ではない。

## Scope

### In Scope

- 対象ディストリビューション/バージョンと X11/Wayland Backend の現状整理（distro 未確定の明示と評価軸）。
- GPUI 0.2.2 および `ruviform-ui-gpui` の Linux 実行時依存 Inventory（宣言 + CI 実測済み分）。
- GUI capability・runtime 依存・パッケージ形式適合条件の Matrix（CI 照合結果を反映）。
- deb / AppImage / Flatpak の比較と採用/保留/対象外の判断根拠。
- 既存 Ubuntu CI の対象 revision・成功 Run の照合結果の設計への反映。
- 旧 WebView/WebKitGTK の分離、Owner 境界、確認手順、証拠リンク。

### Out of Scope

- 対象 distro の最終決定、Linux 実機検証の実施、パッケージ形式の最終採用、Consumer 製品の配布決定、WebView 撤去の実装、GPUI 最終採否、非 Linux packaging、CI lane の増設・変更、実 deb/AppImage/Flatpak 作成・署名（spec 側 Out of Scope と同一）。

## 1. Target OS / Backend — 現状整理

### 1.1 対象ディストリビューション/バージョン

**現時点では未確定（`NotRun`）**。実測なしに特定 distro を支持対象として宣言することは正直性ルールに反する。CI が実測したのは **ubuntu-24.04 hosted runner（GitHub Actions `ubuntu-latest`）1 環境**であり、これは distro サポート宣言ではない。評価軸を決定時の入力として固定する。

| 評価軸 | 確認すべき内容 | 現状（CI 照合後） |
| --- | --- | --- |
| glibc / ベースライン | `rust:1-trixie` 系コンテナや各 distro の glibc が Rust toolchain・GPUI 依存の最低要件を満たすか | Debian コンテナ（GT-05 実測・Docker Debian + mesa 25.0.7）と **ubuntu-24.04 hosted runner（run 43439434737）で build・起動・offscreen 描画を実測**。`libc` は lockfile で 0.2.189 pin（0.2.190 の `libc::ENOATTR` 削除で gpui の `zed-async-tar` 依存が build 不能になる upstream 問題 — `docs/impl/ir/gt-05-ci-offscreen-lanes-slice.md`）。他 distro の glibc 下限は未実測 |
| Vulkan ICD / GPU ドライバ | `blade-graphics`（ash/gpu-alloc/naga）が要求する Vulkan loader + ICD が対象 distro の標準ドライバで提供されるか | ソフトウェア Vulkan は **Docker Debian（GT-05）と ubuntu-24.04 hosted（`ui-gpui-offscreen-verify`・`mesa-vulkan-drivers`=lavapipe）の両方で実測**。**mesa ≥24.2 が下限**（22.3.6 系は depth-24 `PutImage` `BadMatch` で恒久的黒画面）。ubuntu-24.04 stock mesa での offscreen lane 成功は hosted 実測済み。実機 GPU ICD（RADV/ANV・ベンダドライバ）は `NotRun` |
| EGL / GL | `khronos-egl`・`glow` 経路の libEGL/libGL 存在 | `libegl-dev`/`libgl1-mesa-dev` は ubuntu-24.04 lane の apt セットに含まれ build/link は実測済みだが、**EGL/GL 経路の実行時利用は未観測**（Vulkan 主経路で動作） |
| fontconfig / フォント | fontconfig + 最低 1 フォントでの列挙・描画 | GT-05（Docker Debian）と **ubuntu-24.04 hosted offscreen lane（`fonts-dejavu-core` + `fontconfig` 導入済み・`real_window` の lab story がラベル text 描画を含む）で実測**。他 distro のフォント環境は未実測 |
| IME | X11 で `zed-xim`（XIM）経由の入力メソッドが機能するか。Wayland の IME は GPUI 側対応状況に依存 | 未実測・`NotRun` |
| xdg-desktop-portal | `ashpd`/`zbus`/`oo7` が使う portal D-Bus API と対象 distro の portal 実装の対応 | 未実測（ubuntu-24.04 lane は portal を導入していない。GPUI の既定経路では portal 未使用の可能性があるが未検証） |
| Native Plugin Dock | XEmbed/XWayland 機構が対象 distro の X11/Wayland 環境で利用可能か | 未実測。#388 matrix 参照 |

旧 WebView 経路の文書評価（`poc/ui/RESULTS.md`）に Ubuntu 22.04+/Debian 12+/Fedora 38+ の例があるが、**これは wry/WebKitGTK 4.1 の評価であり GPUI の distro 下限を意味しない**。

### 1.2 対象 Backend 範囲

GPUI 0.2.2 は default features で `wayland`・`x11` を有効化する。Backend の整理:

| Backend | GPUI 宣言 | 現状判定 | 備考 |
| --- | --- | --- | --- |
| X11 ネイティブ | `x11` feature 有効 | `P(L0)`（実機 `NotRun`） | `x11-dl`（dlopen）+ `x11rb`/`xcb`/`xkbcommon`/`zed-xim` 経路。GT-05（Docker Debian + mesa 25.0.7・`real_window` 2 passed）に加え、**ubuntu-24.04 hosted CI で `real_window` 3 passed を実測**（run 43439434737・`ui-gpui-offscreen-verify`）。いずれも Xvfb+openbox+lavapipe の L0 であり実機 X11 session は `NotRun` |
| Wayland ネイティブ | `wayland` feature 有効 | `NotRun` | `wayland-client`/`wayland-backend`/`calloop`/`xkbcommon` 経路。compositor・portal 依存。ubuntu-24.04 lane は wayland 系 dev パッケージを導入するが Wayland session での実行は未実施 |
| XWayland（X11 app on Wayland session） | X11 経路の一形態 | `NotRun` | L0 XEmbed が XWayland 経由で機能するかは #388 matrix で `NotRun` |

Backend の最終サポート宣言は実機 matrix（§7）の結果を待つ。「Wayland のみ」「X11 のみ」等の絞り込みは行わない。

## 2. Runtime Dependency Inventory（宣言レベル＋CI 実測分）

`Cargo.lock`・`docs/designs/issue-386-driver-neutral-gui-host.md`・`.github/workflows/ci.yml` に基づく GPUI 0.2.2 の Linux 依存を整理する。ubuntu-24.04 hosted CI で **build/link/実行が成功した依存集合は実測済み**だが、これは「CI が導入した apt セット + cargo 依存で GPUI の headless/offscreen 経路が動く」ことの証拠であり、全実行時依存の網羅証明ではない。

### 2.1 グラフィックス / GPU

| クレート | 役割 | 実行時システム依存 | パッケージング含意・CI 実測 |
| --- | --- | --- | --- |
| `blade-graphics 0.7.1` | GPUI の描画基盤（Vulkan 主経路） | `libvulkan.so.1`（loader）+ GPU ICD | ICD は GPU/ドライバ由来のため同梱不可。**ubuntu-24.04 hosted で `libvulkan-dev`/`libvulkan1`/`mesa-vulkan-drivers` 導入下の lavapipe 描画を実測**（offscreen lane）。実機 GPU ICD は `NotRun` |
| `ash` / `gpu-alloc` / `gpu-alloc-ash` / `naga` | Vulkan API・確保・シェーダ変換 | 同上（`ash` は `libvulkan` を dlopen） | loader のみ必要。ICD はシステム依存。dlopen のため `ldd` に現れない点に注意 |
| `khronos-egl` / `glow` | EGL/GL サーフェス・フォールバック経路 | `libEGL.so.1` / `libGL.so.1` | `libegl-dev`/`libgl1-mesa-dev` は CI apt セットに含まれ link は実測。**実行時の EGL/GL 経路利用は未観測** |
| `ash-window` / `raw-window-handle` | ウィンドウハンドル受け渡し | なし（型のみ） | — |

### 2.2 ウィンドウ / イベントループ / Wayland・X11

| クレート | 役割 | 実行時システム依存 | パッケージング含意・CI 実測 |
| --- | --- | --- | --- |
| `wayland-client` / `wayland-backend` / `wayland-cursor` / `wayland-scanner` / `wayland-protocols` / `wayland-protocols-plasma` | Wayland client・プロトコル | `libwayland-client.so`・`libwayland-cursor.so`・`libxkbcommon.so` | `libwayland-dev`/`wayland-protocols` は CI apt セットに含まれ build は実測。Wayland session での実行は `NotRun` |
| `calloop` / `calloop-wayland-source` | Wayland イベントループ | `libwayland-client` 経由 | 同上 |
| `x11rb` / `x11rb-protocol` / `xcb` / `as-raw-xcb-connection` / `x11-dl` | X11 プロトコル・接続 | `libX11.so.6`・`libxcb.so.1`・`libXcursor`/`libXi`/`libXrandr` 等（`x11-dl` は dlopen） | `libx11-dev`/`libxcb1-dev`/`libxi-dev`/`libxtst-dev`/`libxkbcommon-dev`/`libxkbcommon-x11-dev` は CI apt セット。Xvfb+openbox 下の実 X11 接続・window 生成・pixel readback を **Docker Debian（GT-05）と ubuntu-24.04 hosted（run 43439434737）の両方で実測**。他 distro では未実測 |
| `xkbcommon` | キーマップ・レイアウト | `libxkbcommon.so.0` | Wayland/X11 双方で必要。両 L0 lane でロード実測 |
| `zed-xim` | X11 IME（X Input Method）client | `libX11` + 実行時の IME サーバ（ibus/fcitx 等） | IME 実動作はサーバ側依存。`NotRun` |

### 2.3 テキスト / フォント / IME

| クレート | 役割 | 実行時システム依存 | パッケージング含意・CI 実測 |
| --- | --- | --- | --- |
| `cosmic-text` / `rustybuzz` / `swash` / `etagere` | テキスト shaping・レイアウト・グリフ atlas | なし（pure Rust）+ fontconfig 経由のフォント列挙 | — |
| `fontdb` / `fontconfig-parser` / `yeslogic-fontconfig-sys` | フォント DB・fontconfig 解析 | `libfontconfig.so.1` + フォントファイル | `libfontconfig1-dev`/`fontconfig`/`fonts-dejavu-core` は offscreen lane の apt セット。**ubuntu-24.04 hosted でフォント列挙・ラベル描画を実測**（`real_window` の lab story） |
| `zed-font-kit`（GPUI `font-kit` feature の実クレート） | プラットフォームフォント API | Linux では fontconfig 経路 | feature 構成依存 |
| `resvg` / `usvg` | SVG 描画 | pure Rust 主体（フォントは fontconfig 経由） | — |

### 2.4 Portal / D-Bus / IME 補助

| クレート | 役割 | 実行時システム依存 | パッケージング含意・CI 実測 |
| --- | --- | --- | --- |
| `ashpd 0.11.1` | xdg-desktop-portal 呼出（file chooser 等） | `xdg-desktop-portal` + backend（gtk/kde）+ D-Bus session | portal が無い/未起動の環境では該当機能が `Fail`/`Unsupported` になる可能性。sandbox 形式では portal が必須経路。CI lane は portal を導入しておらず、GPUI 既定経路での portal 利用は未観測 |
| `zbus` / `zbus_macros` / `zbus_names` / `dbus` | D-Bus client（pure Rust） | D-Bus session bus | Wayland session では通常存在。CI lane は session bus なしで headless/offscreen が動作しており、GPUI 既定経路は D-Bus 非依存の可能性があるが未検証 |
| `oo7` | Secret Service portal（資格情報） | `xdg-desktop-portal` の Secret Service 実装 | 使用有無は GPUI の機能依存。`NotRun` |

### 2.5 まとめ — パッケージングへの含意（Ruviform 共通 runtime 最低要件）

Ruviform 共通基盤が Linux で GPUI Driver を動かすために**現時点で文書化できる最低要件**は以下である。これは「ubuntu-24.04 hosted CI で実測された依存集合 + 宣言済みだが未実測の依存」であり、distro サポート宣言ではない。

- **CI 実測済みの実行時依存集合（ubuntu-24.04 hosted・L0/headless/offscreen 経路）**: `libvulkan1` + `mesa-vulkan-drivers`（lavapipe）+ `libegl1`/`libgl1`（mesa）+ `libx11-6`/`libxcb1`/`libxi6`/`libxtst6` + `libxkbcommon0` + `libwayland-client0` + `libfontconfig1` + `fonts-*`（最低 1 フォント）+ D-Bus は GPUI 既定経路で未使用の可能性。対応する `-dev` パッケージは build/link 時に必要。
- **同梱候補**: GPUI バイナリ本体 + Rust 依存は静的リンクで同梱される。`libvulkan` loader・`libfontconfig`・フォント・`libxkbcommon`・`libX11`/`libxcb`・`libwayland-*`・`libEGL`/`libGL`・D-Bus/portal は**実行時システム依存**として残る可能性が高い。
- **ICD/GPU ドライバは同梱不可**: Vulkan ICD は GPU ベンダ・カーネルモジュールに密結合しており、deb/AppImage/Flatpak のいずれでも同梱は現実的でない。対象 OS の GPU ドライバ提供が前提条件になる。
- **フォントはポリシー次第**: 同梱すれば再現性が上がるがサイズ増。非同梱ならシステムフォント依存で描画差が出る可能性。
- **`x11-dl`/`ash` の dlopen**: リンク時依存を持たないため deb の `Depends` に現れにくいが、実行時には各 `.so` が必要。`ldd` だけでは不足を検出できない。**専用の dlopen 依存検査 lane は CI に存在しない（`NotRun`）**— 実機/配布 package 検証時に `LD_DEBUG`/`strace` 的手法で確認する項目として残す。
- **L0 実測フロア（更新）**: GT-05（Docker Debian・mesa 25.0.7）に加え **ubuntu-24.04 hosted CI（run 43439434737）で同一 lane（`mise run test-ui-offscreen-xvfb`）が成功**。これが現在の「CI で再現可能な最小 L0 依存集合」の実測値である。最小 runtime 集合の切り詰め・実機 distro 差は未実測。

## 3. GUI Capability Matrix

対象 OS（Linux・distro 未確定）× Backend（X11 / Wayland / XWayland）× capability の判定表。CI 照合により X11 の一部 capability が **ubuntu-24.04 hosted の L0 lane で実測済み**（`P(L0)` hosted）になった。`Declared` は設計・lockfile 上の存在宣言、`Unsupported` は機構上の非対応、`NotRun` は未実施。**実機環境（実 GPU・実 display・実 session・実入力）の `Pass` は 0 件を維持する**。

凡例: `D`=Declared、`U`=Unsupported、`N`=NotRun、`B`=Blocked、`P`/`F`=Pass/Fail（実機実測・現在該当なし）、`P(L0)`=L0 offscreen lane（Xvfb+openbox+lavapipe）でのみ実測 Pass。`P(L0)` には実測環境を併記する（Docker Debian=GT-05 / ubuntu-24.04 hosted=CI run 43439434737）。**`P(L0)` の実機判定は `N` として扱う**。

| Capability | X11 | Wayland | XWayland | 根拠・備考 |
| --- | --- | --- | --- | --- |
| ウィンドウ生成・ライフサイクル | P(L0): Docker Debian（GT-05・`real_window` 2 passed）+ **ubuntu-24.04 hosted CI（run 43439434737・`real_window` 3 passed）** | N | N | 実 GPUI window 生成・teardown を Xvfb+openbox+lavapipe で実測。実機 session は `NotRun` |
| 複数ウィンドウ | N | N | N | Linux での multi-window 実測なし（L0 lane は `_NET_WM_PID` 解決のため `--test-threads=1` 直列であり、同時複数 window は未検証） |
| 入力（keyboard/mouse） | N | N | N | `x11rb`/`wayland-client` 経路は宣言済み。L0 lane の story walk は semantic dispatch であり OS 入力イベントではない。実入力は `NotRun` |
| フォーカス | N | N | N | 同上。semantic dispatch は OS フォーカス遷移を証明しない |
| IME | N | N | N | X11 は `zed-xim`（XIM）経路が宣言済みだが実動作 `NotRun`。Wayland の IME は text-input protocol 対応が GPUI 側で未確認のため `NotRun`（機構不在が確認されれば `Unsupported` へ再分類） |
| フォント描画 | P(L0): Docker Debian + **ubuntu-24.04 hosted CI** | N | N | fontconfig+`fonts-dejavu-core` 経由の列挙・ラベル描画を両環境で実測。実機フォント環境・描画品質差は `NotRun` |
| DPI / スケーリング | N | N | N | GPUI の scale factor 処理は宣言済み。L0 lane は theme scale 1.0/2.0 cell を実施済みだが、OS レベルの scale factor・HiDPI 実表示は未観測 |
| NativeSurfaceSlot（L0: 自プロセス内 surface） | N | N | N | `native_surface_slot` capability flag は存在。Linux での実 surface 確保は未実施 |
| Native Plugin Dock L0（VST3/CLAP 埋込） | N | U | N | #388 matrix: X11 は `XEmbed`/`X11EmbedWindowID`・CLAP `x11` XID 経路が存在（`NotRun`）。Wayland ネイティブは CLAP/VST3 規格に埋込機構がなく `Unsupported` |
| Native Plugin Dock L1（クロスプロセス GUI 親子） | U | U | U | #388 matrix: X11 は GtkSocket/Plug（GTK3 deprecated・GTK4 撤去）で現行 `Unsupported`。XWayland/Wayland も同様 |
| 実 Plugin GUI（#389 PluginEditorSession） | N | N | N | 実 plugin GUI の完了済み実装証拠なし。`NotRun` |
| a11y（アクセシビリティ） | D | D | D | accesskit 系は Cargo.lock で未確認。実対応は未検証 |
| GPU 非依存 fallback | P(L0): Docker Debian + **ubuntu-24.04 hosted CI** | N | N | lavapipe（`mesa-vulkan-drivers`）で GPU ハードウェア非依存の Blade→Vulkan 描画を両環境で実測（mesa ≥24.2 必須）。実機 GPU 有無別・GL fallback 経路は `NotRun` |

補足:

- Wayland IME のように機構の有無が未確認の項目は `NotRun` とし、機構不在が確認された時点で `Unsupported` へ再分類する。
- Native Plugin Dock の詳細行列は `docs/designs/issue-388-native-surface-slot.md` の D6 matrix が正本であり、本表はその Linux 行を配布設計の観点で再掲したもの。矛盾時は #388 側を正とする。
- 実機での `Pass` が 0 件であることは設計上の欠陥ではなく、実機検証が後続 Issue に委ねられている現状の正直な記録である。

## 4. Package Format Comparison — deb / AppImage / Flatpak

3 形式の適合条件を Issue #127 が要求する 7 観点で比較する。**全形式とも現時点では `保留`（採用保留）** であり、CI の L0/headless 実測は形式採用の根拠にならない（形式の成立には実機・install・署名の証拠が必要）。

### 4.1 比較表

| 観点 | deb | AppImage | Flatpak |
| --- | --- | --- | --- |
| **同梱依存 vs システム依存** | システム依存中心。`Depends` で `libvulkan1`・`libfontconfig1`・`libxkbcommon0`・`libx11-6`・`libxcb1`・`libwayland-client0`・`libegl1`/`libgl1`・`xdg-desktop-portal` 等を宣言する設計が自然。GPU ICD は依存に含められない。フォントは `fonts-*` 依存 or 同梱の選択 | 実行時依存を同梱しやすい（AppDir に `.so` を同梱可能）。ただし Vulkan ICD・カーネルモジュールは同梱不可。`x11-dl`/`ash` の dlopen 依存は同梱で充足可能 | runtime（org.freedesktop.Platform 等）に依存を集約。同梱は `--filesystem`/extension で限定的。portal 前提の設計が必要 |
| **sandbox / portal** | sandbox なし。システム権限で動作。portal は任意利用 | sandbox なし（AppImage 自体は単なる実行ファイル）。bubblewrap 等で自前 sandbox 化は可能だが標準では非 sandbox | **sandbox 必須**。xdg-desktop-portal 経由の file chooser・Secret Service 等が必須経路。直接の X11/Wayland socket・D-Bus アクセスは portal/permission で制限される |
| **更新 / インストール** | `apt`/`dpkg` のパッケージ管理に統合。署名は distro の apt 署名または独自 repo。更新はパッケージ更新に従う | 単一実行ファイル。更新はアプリ側または AppImageUpdate 等の外部機構。署名は任意（GPG/署名付き AppImage）。インストール概念が薄い | `flatpak install/update` に統合。署名は Flathub または独自 repo の GPG。sandbox 内更新は runtime と分離 |
| **OS / Driver 制約** | 対象 distro の apt repo・アーキテクチャ（x86_64/aarch64）・glibc に依存。distro 毎にビルドまたは汎用ビルドの glibc 下限管理が必要 | glibc 下限をビルド環境で制御可能。アーキテクチャ別 AppImage が必要 | runtime（Freedesktop/KDE）が対象 distro の基盤を規定。distro ネイティブ版と runtime 版の二重サポートが発生し得る |
| **Native Plugin Dock 適合** | sandbox なしのため XEmbed/XWayland の直接利用に制約が少ない。L0 XEmbed（X11）は原理上可能（`NotRun`）。L1 は現行 `Unsupported` | 同上（非 sandbox）。X11/XWayland での XEmbed 利用に sandbox 障害がない | **sandbox が XEmbed/直接ウィンドウ埋込を阻害する可能性が高い**。X11 socket アクセス・XWayland 利用は portal/permission で制限され、Native Plugin Dock L0 が機能しない可能性。要実機検証だが構造的リスク大 |
| **失敗時 fallback** | 依存不足時は `apt` が依存解決で未然に防ぐか、起動時に共有ライブラリ不足で `Fail`。Vulkan ICD 不在時は GPU 初期化失敗 | 同梱依存で起動は安定しやすいが、ICD/カーネル依存は残る。`--appimage-extract` でのデバッグ容易 | sandbox 内で依存不足は runtime が吸収するが、portal/backend 不在で機能 `Fail`/`Unsupported` になる。fallback は portal 非依存経路の確保が必要 |

### 4.2 判断根拠（採用/保留/対象外）

| 形式 | 現時点の判断 | 根拠 |
| --- | --- | --- |
| **deb** | **保留** | システム依存モデルは GPUI の実行時依存（§2.5）と整合し、Native Plugin Dock の XEmbed 経路を阻害しない点で最も制約が少ない。ただし対象 distro 未確定・実機未検証・package 未作成のため採用は保留 |
| **AppImage** | **保留** | 同梱依存で配布の手軽さはあるが、Vulkan ICD・フォント・portal の扱い・署名/更新機構が未設計。実機検証なしに採用は保留 |
| **Flatpak** | **保留（Native Plugin Dock 適合に構造的疑義）** | sandbox/portal モデルはセキュリティ上望ましいが、XEmbed/直接ウィンドウ埋込を要求する Native Plugin Dock L0 との適合が最も疑わしい。Dock 必須 Consumer では `対象外` になり得るが、Dock 非依存なら `保留` のまま検討余地あり |

**重要**: 上記は全て `保留` であり、**未実測の形式を配布可能と断定しない**。最終採用は (a) 対象 OS/Backend 決定、(b) 実機 capability matrix の `Pass` 証拠、(c) 各 Consumer の製品要件（Dock 必須か・署名・リリース workflow）の三点が揃ってから行う。CI の OS-L1/L0 証拠はこの三点のいずれも満たさない。

### 4.3 形式決定に必要な今後の証拠

- 各形式での GPUI 起動・入力・フォント・DPI/IME の実機 `Pass`/`Fail`。
- Flatpak sandbox 下での Native Plugin Dock L0（XEmbed or portal 経由）の可否。
- deb の `Depends` 最小集合の実測（`lddtree`/`ldd` + dlopen 依存の手動確認）。
- AppImage の同梱 `.so` セットと glibc 下限の実測。
- 署名・更新・インストール UX の Consumer 要件との照合。

## 5. Ownership Boundary — Ruviform / AudioLens / Synvelis

Issue #127 の AC に対応し、責務を明確に分離する。**Ruviform は共通の入力（capability matrix・runtime/ABI 基準・packaging 適合条件）を提供し、製品 packaging の結論は各 Consumer が Own する**。

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
| Consumer 非回帰 | **#294** | AudioLens/Synvelis の候補 tuple に対する非回帰 lane（`consumer-verify.yml`） | 最新成功 run 43347745016 @ `3692c6fa` を証拠として記録。リンクのみ |
| legacy WebView 撤去 | **#465** | `wry`/`tao`/WebKitGTK 依存の物理的撤去 | 本設計は分離記述のみ |

**暗黙の必須依存を生じさせない条件**: 本設計は「deb を推奨」「Flatpak を必須」のような製品強制をしない。パッケージ形式の採用/保留/対象外は全て `保留` であり、各 Consumer が自製品の要件に基づいて独立に決定できる。

## 6. Legacy WebView / WebKitGTK 4.1 の分離

旧 WebView/WebKitGTK 4.1 固有の runtime/Browser 検証（`docs/adr/ui-tech-selection-poc.md`・`poc/ui/RESULTS.md`・`docs/impl/exp/linux-container-device-and-arch-verification.jsonl` seq 10 の #57 Linux GTK 経路検証）は、**GPUI 配布の現行必須条件ではない**。

- `wry` + `tao` + `libwebkit2gtk-4.1-0` + GTK3 という**旧 Driver** の依存・能力・制約を記録したものである。
- GPUI（`gpui 0.2.2`）は WebKitGTK/GTK に依存しない。GPUI の Linux 依存は §2 の通り全く別系統である。
- 旧知見を GPUI の distro 下限・依存・capability の根拠にしてはならない。
- `ui-verify (ubuntu-latest)` lane の成功は旧 WebView 経路を含む UI lane の成功であり、**GPUI 配布の必須条件の証拠としては扱わない**。
- 旧 WebView 経路のコード・依存は**削除済みではない**（`Cargo.lock` に `glib`/`gtk`/`javascriptcore-rs`/`soup3`/`tao`/`webkit2gtk`/`wry` 等が残存）。撤去は Issue #465 の責務。
- `ci/verify-linux/Dockerfile` は旧 GTK 経路を含むが、GPUI 検証には別途 GPUI 用依存の追加が必要になる可能性がある（後続実装 Issue の検討事項）。

## 7. Executable Verification Procedures

Issue #127 AC の「実行可能な確認手順」を定義する。L0 相当は GT-05（Docker Debian）と ubuntu-24.04 hosted CI（run 43439434737）の両方で実現済み（`P(L0)`）。L1 以降・実機 matrix は引き続き `NotRun`/`Deferred`。

### 7.1 確認手順の階層

| 階層 | 環境 | 目的 | 現状 |
| --- | --- | --- | --- |
| L0: コンテナ/CI（Xvfb/Wayland headless） | `mise run test-ui-offscreen-xvfb`（Xvfb+openbox+lavapipe）を Docker Debian / ubuntu-24.04 hosted で実行 | GPUI バイナリの**起動・ウィンドウ生成・基本描画**の smoke | **実現済み（`P(L0)`）**: GT-05（Docker Debian・`real_window` 2 passed）+ ubuntu-24.04 hosted CI（run 43439434737・`real_window` 3 passed）。CI job `ui-gpui-offscreen-verify` 成功。`ci/verify-linux` Dockerfile 拡張形の L0 は依然 `NotRun`（deferred） |
| L1: 実機 Linux（X11 session） | 対象 distro の X11 環境 | X11 Backend での全 capability（入力・フォーカス・フォント・DPI/IME・NativeSurfaceSlot・Dock L0 XEmbed） | `NotRun`。#419/#262/#388/#389 の実機 lane で実施 |
| L2: 実機 Linux（Wayland session） | 対象 distro の Wayland compositor 環境 | Wayland Backend での全 capability + XWayland 経由 Dock | `NotRun`（Deferred・利用要件発生まで） |
| L3: パッケージ形式別 | 各形式でビルドした成果物をクリーン環境へ install | deb/AppImage/Flatpak それぞれの起動・依存解決・Dock 適合・fallback | `NotRun`（Deferred・形式採用後に実施） |

### 7.2 既存 CI lane の対応（OS-L1 証拠）

Linux OS-L1（必須）の Native Build/Link・Headless・Feature・依存・ABI は既存 CI で検証される。対象 revision と成功 run の正本は `evidence/rf127/linux-ci-crosscheck.md`。概要:

| 受入観点 | lane（ubuntu-latest） | run 43439434737 @ `365c74ac` での結果 |
| --- | --- | --- |
| Native Build/Link・Lint・Test | `verify`（`lint-full` clippy `--all-targets` + `test-full` workspace test。当該 SHA に明示的 `build-full`/`check-full` step なし） | Pass（`test result: ok` 201 block・FAILED 0） |
| RT | `rt-verify`（rt_ contract 166 件） | Pass |
| Feature matrix | `icc-verify`・`compression-verify`・`io-structured-verify`・`l1-real-verify`（plugin host）・`msrv-verify` | Pass |
| GPUI Driver headless/contract | `ui-gpui-verify`（`gpui-test`/`uitest` headless 契約・golden・scenario） | Pass |
| GPUI offscreen（L0） | `ui-gpui-offscreen-verify`（Xvfb+openbox+lavapipe） | Pass（`real_window` 3 passed） |
| UI headless（旧 WebView 経路含む） | `ui-verify` | Pass（GPUI 必須条件ではない — §6） |
| bench smoke | `bench` | Pass |
| 集約 | `ci-summary`（fail-closed） | Pass（verdict=PASS） |

ABI は専用 lane を持たず domain ABI 契約 test が `test-full` に内包される。dlopen 実行時依存の網羅検査 lane は存在しない（`NotRun` — §2.5・Deferred 記録）。

### 7.3 L0 コンテナ smoke の具体手順（将来実装用）

GT-05 で `mise run test-ui-offscreen-xvfb` として実現済み（ubuntu-24.04 hosted CI でも成功）。`ci/verify-linux` Dockerfile を拡張する下記手順は依然 deferred の将来実装用。lane 要件（**openbox 等 WM が必須**・**mesa ≥24.2**・**`--test-threads=1`**）は下記コンテナ形にも適用する。

1. `ci/verify-linux/Dockerfile` を GPUI 用に拡張: `libvulkan1`・`mesa-vulkan-drivers`・`libfontconfig1`・`fonts-*`・`libxkbcommon0`・`libx11-6`・`libxcb1`・`libwayland-client0`・`libegl1`・`libgl1`・`xdg-desktop-portal`・`dbus-x11` を追加。
2. `cargo build -p ruviform-ui-gpui --features gpui` 系の smoke 対象を build。
3. `xvfb-run` + openbox 下で起動し、ウィンドウ生成・数フレーム描画・終了を観測。
4. 結果を `Pass`/`Fail`/`Unsupported`/`NotRun` で記録し、対象 revision・distro・Backend を証拠に残す。
5. **重要**: L0 の `Pass` は「headless 環境での起動成功」であり、実機 GPU・実入力・IME・Native Dock の `Pass` ではない。

### 7.4 実機 matrix の記録要件

実機検証を行う際は以下を証拠に含める:

- 対象ディストリビューション/バージョン（実測時に確定）。
- Backend（X11 / Wayland / XWayland）。
- パッケージ形式（deb / AppImage / Flatpak / 直接バイナリ）。
- 検証 revision（Ruviform commit SHA）。
- 各 capability の `Pass`/`Fail`/`Unsupported`/`NotRun`/`Blocked` 判定。
- Native Plugin Dock は L0/L1 × plugin 規格（VST3/CLAP）別に記録。

### 7.5 証拠リンク

- **CI 照合レジャー（本 slice の正本）**: `evidence/rf127/linux-ci-crosscheck.md`。
- 実機・Native Plugin Dock の受入: **Issue #419**・**Issue #262**・**Issue #388**・**Issue #389**。
- L0 offscreen lane の実測記録: `docs/impl/ir/gt-05-ci-offscreen-lanes-slice.md`・`docs/designs/gt-05-ci-offscreen-lanes-design.md`（Issue #314 RF-GT-05）。
- OS-L1 CI 照合の先行例: `evidence/rf388-os-l1-ci/report.md`（Issue #388）。
- Consumer 非回帰: **Issue #294**（`consumer-verify.yml`）。
- 本設計の capability matrix が `Pass` になっても、#419/#262 の独立証拠が揃うまで GPUI 最終採用とはしない。

## 8. Risks and Trade-offs

| リスク / 懸念 | 内容 | 対応 |
| --- | --- | --- |
| GPUI Linux 実機未検証 | 実機 `Pass` は 0 件。L0 offscreen lane（Docker Debian + ubuntu-24.04 hosted CI）でのみ起動・描画・pixel readback を実測 | L1/L2 実機 matrix（§7.1）で検証。#419/#262 に委譲。`P(L0)` は実機の代替ではない |
| 現行 main HEAD の CI 未実施 | 収集時点 HEAD `621490e8` に `ci.yml` run なし。最新成功は祖先 `365c74ac` | merge 後の対象 SHA の必須 CI（daily-ci dispatch）を Coordinator が確認。統合検証は merge 後 SHA の CI 証拠 |
| headless lane 環境要件 | bare Xvfb（WM 無し）は黒画面。mesa <24.2 は depth-24 `PutImage` `BadMatch` で恒久的黒画面 | openbox 等 WM + mesa ≥24.2 を lane 要件として固定済み（GT-05）。`--test-threads=1` 必須。コンテナ/実機展開時も同要件を適用 |
| Vulkan ICD 非同梱 | GPU ドライバ依存のため全形式で ICD 同梱不可。GPU 無し環境での fallback 未確認 | §2.5・§3 GPU fallback 行。実機で ICD 有無別に検証 |
| Flatpak × Native Dock | sandbox が XEmbed/直接埋込を阻害する構造的疑義 | §4.1・§4.2。実機で portal 経由の可否を検証。不可なら Dock 必須 Consumer では `対象外` |
| `x11-dl`/`ash` の dlopen 見落とし | `ldd` で検出できない実行時依存。CI に専用検査 lane なし | §2.5・§7.2。実機/配布 package 検証時に `LD_DEBUG`/`strace` 的手法で確認 |
| Wayland IME 未確認 | text-input protocol 対応が GPUI で未確認 | §3 IME 行。実機 Wayland session で検証 |
| 旧 WebView 知見の混入 | WebKitGTK/GTK の成功を GPUI に転用する誤り | §6 の分離ルール。test-spec で検証 |
| license 未決定 | `Cargo.toml` の `license = "MIT"` は仮置き。配布物の license 表記が未確定 | `licensing-and-distribution-policy` ADR。Consumer の署名・同梱方針に影響 |
| gpui の `rust-version` 未宣言 | manifest に `rust-version` なし。edition 2024 → rustc ≥1.85。repo pin は 1.99.0 | MSRV 1.89 方針（`workspace-msrv` ADR）との整合は問題なし。GPUI 更新時に再確認 |

## 9. Design Summary — Issue #127 AC 写像

| Issue #127 AC | 本設計の対応節 | 状態 |
| --- | --- | --- |
| AC1: distro/Backend/capability/runtime 依存を実測/未実測区別 Matrix で記録 + 既存 CI の対象 revision と成功 Run を照合 | §1・§2・§3・§7.2 + `evidence/rf127/linux-ci-crosscheck.md` | 記録完了。ubuntu-24.04 hosted の L0/headless は run 43439434737 で実測（`P(L0)`/OS-L1）。現行 HEAD CI は `NotRun(Pending)`。実機は `NotRun` |
| AC2: deb/AppImage/Flatpak の適合条件比較と採用/保留/対象外の根拠。未実測を配布可能と断定しない | §4 | 記録完了。全形式 `保留` |
| AC3: 旧 WebView/WebKitGTK を現行必須条件から除外・#465 と分離・削除済みと主張しない | §6 | 記録完了 |
| AC4: Ruviform/Consumer（AL/SV）の Owner 区別・暗黙の必須依存を生じさせない | §5 | 記録完了 |
| AC5: 採否・対象 OS・未対応範囲・確認手順・判定語彙・#419/#262/#294 リンク | Evidence Vocabulary・§4（採否=保留）・§7（手順・リンク） | 記録完了 |

## Sources and Relations

- Issue #127 本文（正本）。本 slice の spec: `docs/specs/linux-distribution.md`。先行版: `docs/designs/issue-127-linux-dist-design.md`（本版が `refines`）。
- CI 証拠正本: `evidence/rf127/linux-ci-crosscheck.md`（run 43439434737 @ `365c74ac`・lane 台帳・SHA artifact 照合・daily-ci/consumer-verify）。
- 上流: `docs/specs/linux-distribution.md`、`docs/designs/issue-384-driver-neutral-gui-contract.md`、`docs/designs/issue-257-driver-neutral-ui-contract.md`、`docs/designs/issue-386-driver-neutral-gui-host.md`（GPUI 0.2.2・Linux 依存の一次整理）、`docs/designs/issue-388-native-surface-slot.md`（D6 matrix 正本）、`docs/designs/issue-115-l1-gui-cross-process-parenting.md`、`docs/designs/issue-243-gpui-realwin-windows.md`。
- ADR: `docs/adr/driver-neutral-gui-gpui-driver-webview-first-issue-384.md`（proposed）、`docs/adr/platform-and-toolchain-targets.md`、`docs/adr/licensing-and-distribution-policy.md`、`docs/adr/ui-tech-selection-poc.md`（superseded・歴史的参照）、`docs/adr/issue-465-legacy-webview-retirement-status.md`。
- 歴史的参照: `poc/ui/RESULTS.md`、`docs/impl/exp/linux-container-device-and-arch-verification.jsonl`、`ci/verify-linux/Dockerfile`、`docs/discovery/issue-388-389-native-dock-current-state.md`、`docs/reports/issue-431-real-verify.md`。
- Consumer: AudioLens #127、Synvelis #28。
