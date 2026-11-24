# docker/linux-verify — Linux 検証イメージの構成と既存 CI との役割分担（Issue #953）

ユーザー判断「ローカル linux 検証は WSL 直実行ではなく docker 検証イメージで行う。イメージをリポジトリ管理する」にもとづく実装 slice。`docker/linux-verify/` に Dockerfile と実行 wrapper を置き、`mise run verify-docker` 経由で既存の検証 lane をコンテナ内で実行する。製品コードの変更はない（`crates/**` zero-diff）。

## 既存 CI の 3OS OS-L1 カバー範囲

正本は `.github/workflows/ci.yml`（`workflow_dispatch` 専用 — Daily Scheduler `daily-ci.yml` の判定 dispatch または手動 dispatch で起動）と `docs/development-environment.md`。

| job | runner | lane 内容 |
| --- | --- | --- |
| `verify` (ubuntu leg) | ubuntu-latest | `mise install --locked rust prek actionlint` → `libasound2-dev` 導入 → `prek run --all-files` → `fmt-check` → `lint-full` → `test-full`。Linux では `lint-full` が型検査を兼ね、`check-full`/`build-full` は実行しない（Issue #331） |
| `verify` (windows leg) | windows-latest | `check-full` → `build-full` → `test-full` |
| `verify-macos` | macos-latest | `check-full` → `build-full` → `test-full` |
| `l1-real-verify` | ubuntu + windows + macos（別 job） | `test-l1-real-plugin`（実規格プラグイン L1 E2E。Issue #114） |
| `rt-verify` | ubuntu-latest | `test-rt`（libasound2-dev 導入） |
| `icc-verify` / `compression-verify` / `io-structured-verify` / `observability-verify` | ubuntu-latest（observability は 3OS） | 各 feature matrix lane |
| `ui-verify` | ubuntu + windows + macos | webview/headless/components/browser 系（Linux leg は libwebkit2gtk-4.1-dev + node toolchain）。Issue #465 により legacy lane 扱い |
| `ui-gpui-verify` | ubuntu + windows + macos | driver-neutral 契約 family + `check/lint/test-ui-gpui`。Linux leg は X11/Wayland/fontconfig/Vulkan/EGL dev を job 内 apt install |
| `ui-gpui-offscreen-verify` | ubuntu-latest | `test-ui-offscreen-xvfb`（xvfb-run + openbox + lavapipe software Vulkan） |
| `msrv-verify` | ubuntu-latest | `mise exec rust@1.89.0` 経由の MSRV test |
| `bench` | ubuntu-latest | `bench-smoke` |

要点: **3OS の OS-L1（ネイティブ build/link/test）は `verify` + `verify-macos` で既にカバー済み**であり、本イメージで二重化する意味はない。toolchain は workflow 環境変数 `MISE_ENV=ci` で `mise.ci.toml` の pin（rust 1.99.0、rustfmt + clippy components）が有効化され、`*-full` タスクは `Cargo.toml [profile.ci]` + `CARGO_INCREMENTAL=0` で実行される。

## docker 検証イメージの役割

- **push 前の Linux ネイティブ実走**: Windows/macOS のローカル環境からは得られない Linux 実 toolchain での build/link/test。
- **CI 完了待ちを要しない高速イテレーション**: Daily Scheduler 起動の CI は dispatch→完了まで時間がかかる。named volume に target/toolchain/registry を永続化し、2 回目以降は増分実行。
- **manual/opt-in lane のローカル実行**: `test-l1-real-plugin`、`test-formal-kani` の WSL2 fallback のような lane を Linux 環境で実行する足場（既存 lane の docker 移行自体は本 slice のスコープ外 — 今後の候補として記録）。
- **Linux 固有挙動の切り分け**: `cfg(unix)` 系、ファイルシステム・パス・プロセス挙動の差分調査。

`mise run check-linux`（`cargo check/clippy --target x86_64-unknown-linux-gnu`）は既存の補助検査であり、Linux C toolchain 不要な crate（io-file/io-archive）限定・link/test を含まない cross 検査。本イメージは実 toolchain での完全な build/link/test を提供し、check-linux の代替・競合ではない。

## 構成

### ベースイメージ

`ubuntu:24.04`（タグ pin）。CI の `ubuntu-latest` runner は 24.04 系で、系統を一致させる。digest pin は採用しない — LTS 系内の minor 更新で security fix を受け取るため。実測した digest は `verification.md` に記録する。

### toolchain 再現方式

CI と同一入口を採用した: **mise をイメージに導入し、`MISE_ENV=ci` + `mise install --locked rust prek actionlint` で mount したリポジトリの `mise.toml`/`mise.ci.toml` から解決する**。

- mise 本体は GitHub release バイナリを `v2026.11.2` + SHA256 照合で pin（`SHASUMS256.txt` 公開値。ホスト側の実測バージョンと一致）。
- rust/prek/actionlint 等のツールはイメージに焼き込まず、**実行時に entrypoint が `mise install --locked` で解決**する。pin 更新（mise.ci.toml の rust バージョン等）でイメージが陳腐化しない。`MISE_DATA_DIR`・cargo registry は named volume に永続化し、初回以降の install はスキップされる。
- rust は mise 経由の rustup 管理で `rust@1.99.0` が解決される（`mise.ci.lock` が正本 — `MISE_ENV=ci` で有効化）。

代替案の評価:

| 案 | 却下理由 |
| --- | --- |
| rustup 直 pin（`rustup toolchain install 1.99.0`） | rust だけは再現できるが、verify lane が使う prek/actionlint など `mise.toml` pin ツール群の解決入口が 2 系統になり、`mise.toml`/`mise.ci.toml` 正本と乖離する |
| `rust:<ver>` 公式イメージ | floating なメジャータグ（`rust:1`）は `mise.ci.toml` pin と一致しない。`ci/verify-linux` が `rust:1-trixie` を採っているが、目的が異なる（後述） |

### システム依存

| lane | パッケージ | 採否 |
| --- | --- | --- |
| verify（cpal/alsa-sys の build script が pkg-config 経由で要求） | `libasound2-dev` | 採用 |
| 基礎 toolchain / wrapper | `build-essential` `pkg-config` `git` `ca-certificates` `curl` `rsync` `xz-utils` | 採用 |
| `ui-gpui-verify` | `libx11-dev libxi-dev libxtst-dev libwayland-dev wayland-protocols libfontconfig1-dev libfreetype-dev libvulkan-dev libegl-dev libgl1-mesa-dev mesa-vulkan-drivers libvulkan1`（ci.yml install step 準拠） | 採用 |
| `ui-gpui-offscreen-verify` | 上記 + `xvfb xauth openbox x11-utils fonts-dejavu-core fontconfig`（同） | 採用 |
| `ui-verify`（webview） | `libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev` + node/npm | **対象外** — legacy lane（Issue #465 で parity 目標対象外）で、webkit2gtk + node の導入はイメージサイズ・build 時間に見合わない。必要時に別途評価 |
| `test-ui-browser`（Playwright） | node/npm + Playwright chromium | **対象外** — 同上 |
| audio 実検証（PulseAudio/PipeWire daemon） | `pulseaudio` `pipewire` `wireplumber` `dbus` 等 | **対象外** — `ci/verify-linux` の既存責務（後述） |

## mount と target/・.git の取り扱い

- host の worktree は `/src` へ **read-only** mount し、entrypoint が named volume `/work` へ `rsync -a --delete` でミラーする。コンテナ内で `/work` 配下に独立した `target` が作られ、`CARGO_TARGET_DIR` 等の出力先変更変数は使わない（共通契約準拠）。
- `target` を同期対象外とする理由: host worktree の `target` は mbx managed target への symlink で、コンテナ内では dangling となり共有不能。
- `.git` を同期対象外とする理由: worktree の `.git` は Windows 絶対パスを指す gitfile で、コンテナ内では解決不能。`prek run --all-files` 等のため、entrypoint が `/work` 側に使い捨ての snapshot commit を作る（host の履歴・git config には触れない）。
- `/src` を read-only とすることで、fmt のような書換系 lane を誤って実行しても host の worktree は変更されない。
- volume: `ruviform-linux-verify-work`（ミラー + target）、`ruviform-linux-verify-mise`（toolchain）、`ruviform-linux-verify-cargo`（crates.io registry cache）。

## 実行手順

```bash
# イメージ build（docker/linux-verify/ が context）
mise run verify-docker-build

# コンテナ内で既存 lane を実行（引数はそのままコンテナ内の `mise run` へ）
mise run verify-docker fmt-check
mise run verify-docker test ruviform-io-file
mise run verify-docker lint ruviform-io-file
mise run verify-docker test-full          # verify lane 相当の全体実行も可能
```

`verify-docker` は `verify-docker-build` に `depends` し、image が無ければ build してから実行する。実体スクリプトは `docker/linux-verify/verify-docker.sh` で、`mise` 非経由の直接実行も可能（`bash docker/linux-verify/verify-docker.sh <task>`）。

直接 `docker` を使う場合（`mise run` と等価）:

```bash
docker build -f docker/linux-verify/Dockerfile -t ruviform-linux-verify docker/linux-verify
docker run --rm \
  -v <repo ルート>:/src:ro \
  -v ruviform-linux-verify-work:/work \
  -v ruviform-linux-verify-mise:/root/.local/share/mise \
  -v ruviform-linux-verify-cargo:/root/.cargo \
  ruviform-linux-verify <mise-task> [args]
```

任意コマンドは `--entrypoint bash` で shell に入れる（`docker run --rm -it ... --entrypoint bash ruviform-linux-verify`）。MSRV lane は `mise exec rust@1.89.0` がコンテナ内 mise に toolchain を追加取得する構成で実行可能（実測は NotRun）。

既知の挙動:

- `mise install --locked` 実行時に lazy tool `pipx:serena-agent` の shim 解決で警告（`lazy tool pipx:serena-agent has no registry bin metadata`、Linux で pywebview 依存解決不可の resolver ログ）が出るが、lazy tool であり lane 実行を妨げない。
- `verify-docker` の run 実体を inline script ではなく `.sh` ファイルに置いたのは、mise が Windows ホストで複数行 run script を一時ファイル化する際の改行コード差異（`bash\r` 失敗）を回避するため。`.sh` は `.gitattributes` で `eol=lf` 固定。

## `ci/verify-linux` との関係

`ci/verify-linux/Dockerfile`（Issue #60/#55/#57 由来の既存資産、本 slice の lease 外）は責務が異なる: PulseAudio null sink / PipeWire loopback の**サウンドサーバ仮想 endpoint 検証**、QEMU 経由 aarch64 NEON 計測、WebKitGTK PoC を目的とし、`rust:1-trixie` の floating toolchain・mise 不使用・daemon 常駐構成。本イメージは `mise.ci.toml` pin と一致する **CI verify lane 互換の検証環境**であり、lane 実行用途。置き換え・統合は本 slice では行わない。

## 今後の候補

- `test-formal-kani` 等の WSL2 fallback を持つ既存 lane の docker 移行。
- `l1-real-verify`（実規格プラグイン lane）のコンテナ実測（deps は揃っているが未実測）。
- `ui-verify` / `test-ui-browser` を回したい需要が出た場合の webkit2gtk + node 追加（別 Dockerfile 派生の方が layer 分離できる）。
