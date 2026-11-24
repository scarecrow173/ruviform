# ruviform#953 検証記録 — docker/linux-verify イメージ構築と lane 実測

実施日: 2026-11-25 / worktree: `rf953-docker-validation` /
branch: `scarecrow173/rf953-docker-validation`。
host OS: Windows + Docker Desktop（Docker Engine 29.9.2、linux engine）。
コンテナ内 OS: Ubuntu 24.04.5 LTS。base SHA `593d5f7b`
（branch 起点・作業開始時点の origin/main）。

構成の設計意図と既存 CI との役割分担は
[docker-linux-verify.md](docker-linux-verify.md) を参照。

## 成果物

- `docker/linux-verify/Dockerfile` — `ubuntu:24.04` + mise binary pin
  （v2026.11.2、SHASUMS256.txt 公開値で sha256 照合）+ 検証 lane 依存群
- `docker/linux-verify/entrypoint.sh` — /src(read-only)→/work rsync・
  `mise install --locked rust prek actionlint`・git snapshot・`mise run "$@"`
- `docker/linux-verify/verify-docker.sh` — host 側 wrapper
  （`git rev-parse` + `MSYS_NO_PATHCONV=1` + named volume 付き `docker run`）
- `mise.toml` `[tasks.verify-docker-build]` / `[tasks.verify-docker]`
  （additive 追記のみ — `[tools]`/`[env]`/`[settings]`・既存タスク非変更。

## 実測記録

### イメージ build

| コマンド | 結果 |
| --- | --- |
| `docker build -f docker/linux-verify/Dockerfile -t ruviform-linux-verify docker/linux-verify` | exit 0。ubuntu:24.04 = `sha256:562111b88caa79c71391c0c4d3c2114a7d5c35915c20561fd893cb1e07929e6b`、イメージ = `sha256:1166b21d6ab9dad7c26abcde6fa5f2495d24b511e549164777443eacefbe6edc` |

mise binary は `SHASUMS256.txt` 公開値
`c914dc3be1ad3588c3a7fa8ddae86c65e77eaf1777aa4a900c215e467f4c94a4` と
sha256 照合して導入（`sha256sum -c` → OK）。

### mise タスクの発見・self-run

| コマンド | 結果 |
| --- | --- |
| `mise tasks` | `verify-docker` / `verify-docker-build` が定義・発見されることを確認 |
| `mise run verify-docker <task>` | 下記各 lane を本タスク経由で実行（`verify-docker-build` depends 付きで image 未存在時は先に build） |

### コンテナ内 lane 実測（`mise run verify-docker <lane>`）

| lane | exit | 内容 |
| --- | --- | --- |
| `mise run verify-docker fmt-check` | 0（51.0s） | コンテナ内で `cargo fmt --all -- --check` 完走。初回実行のため `mise install --locked rust prek actionlint` の toolchain 解決を含む |
| `mise run verify-docker test ruviform-io-file` | 0（60.6s） | `cargo test --locked -p ruviform-io-file`。lib 26 tests + doctest 9 + compile-fail doctest 2 = 計 37 tests 全 pass。Linux ネイティブ build/link/test を実走 |
| `mise run verify-docker lint ruviform-io-file` | 0（38.3s） | `cargo clippy --locked --all-targets -p ruviform-io-file -- -D warnings` clean |
| `mise run verify-docker check-ui-gpui` | 0（117.3s） | gpui crate 本体（v0.2.2-rf2 tag）・blade-graphics・wayland/x11 系依存を含む Linux 実コンパイル完走 — GPUI system 依存群の有効性を実測。warning は依存 crate `proc-macro-error2` の future-incompat 通知（本 slice の変更起因ではない） |
| `mise exec -- prek run --all-files`（コンテナ内、`--entrypoint bash` で `/work` 直接実行） | 0 | git snapshot 経由で全 hook Passed（check-toml/actionlint 含む）— CI verify lane 先頭 step と等価 |

コンテナ内 toolchain 実測: `rustc 1.99.0 (b940084d7 2026-09-28)`
（`mise.ci.toml` pin と一致）、`prek 0.5.7`、mise `2026.11.2`、
Ubuntu 24.04.5 LTS。rust は mise 経由の rustup 管理で `rust@1.99.0`
として解決（`mise install --locked` ログ: `rust@1.99.0 already installed`）。

`mise install --locked` 時に lazy tool `pipx:serena-agent` の shim 解決警告
（`lazy tool ... has no registry bin metadata` と pywebview 依存 resolver
ログ）が出るが、lazy tool につき lane 実行を妨げない。2 回目以降の実行は
toolchain install がスキップされ増分動作することを volume 永続化で確認
（test/lint/check-ui-gpui はいずれも install skip 済みログ + 増分 build）。

### host 側 canonical gate

| コマンド | 結果 |
| --- | --- |
| `mise run fmt-check` | exit 0（`cargo fmt --all -- --check` clean） |
| `mise exec -- prek run --all-files` | exit 0 — 全 hook Passed（check-toml が `mise.toml` の TOML 検査を兼ねる。mixed-line-ending・trailing-whitespace・check-yaml も Passed） |

## NotRun / 対象外

| 項目 | 状態 | 理由 |
| --- | --- | --- |
| `test-full`（コンテナ内） | **Fail（exit 101、260s）** | `ruviform-io-archive` `extract_staging` で 2 件 Fail — 既存の Linux 固有差異（後述「発見した既存不具合」）。lane 自体は完走し、残りは全 pass |
| `lint-full`（コンテナ内） | NotRun | 時間比例の原則（POLICIES.md §5）。`lint` 単 package で Linux clippy 実走は確認済み |
| `ui-verify` / `test-ui-browser` / `build-ui-*` | Unsupported（イメージ対象外） | webkit2gtk-4.1-dev + node/npm + Playwright 未導入。legacy lane（Issue #465）+ サイズのため設計上除外（docker-linux-verify.md 参照） |
| MSRV lane（`test-ofx-msrv` 等） | NotRun | `mise exec rust@1.89.0` で実行時 toolchain 追加取得が可能な構成だが未実測 |
| `test-l1-real-plugin` 等の manual lane | NotRun | 実測対象外（構成的には実行可能なはず — 今後の候補） |
| `test-formal-kani` の docker 移行 | 対象外 | 本 slice のスコープ外（既存 WSL2 fallback の移行ではなく検証環境の新設） |
| audio 仮想 endpoint 系 | Unsupported | `ci/verify-linux` の既存責務（daemon 常駐構成で分離） |
| `*-full`・3OS 横断（host） | NotRun | Daily/手動 Full CI 管轄（本 slice はコード非変更の検証環境構築） |

## 発見した既存不具合（本 slice 起因ではない — `crates/**` は zero-diff で修正しない）

`mise run verify-docker test-full`（exit 101、260.1s）で
`ruviform-io-archive` `tests/extract_staging.rs` の foreign-object
refusal 系 2 件が Linux 上で Fail:

| test | コンテナ内実測 | Windows 実測 |
| --- | --- | --- |
| `cleanup_retains_a_foreign_file_at_a_recorded_name`（`extract_staging.rs:112`） | **Fail（2 回連続で再現）** — `cleanup must fail` 期待に対し `CleanupReceipt { attempted: 4, removed: 4, retained: 0 }`。staged file を別内容で置き換えた foreign object が拒否されず削除される | Pass |
| `cleanup_refuses_to_delete_a_swapped_root_tree`（`extract_staging.rs:162`） | **Flaky** — `test-full` では Fail（`attempted: 2, removed: 2`）、単独再実行では Pass | Pass |

同一実行で `cleanup_retains_a_symlink_at_a_recorded_name`（symlink 版、
unix cfg）は Pass — リンク種別の差異（tag `Other`）は正しく拒否されるが、
**file→file 置き換えが identity check を通過する**。

- 対象は Issue #286/#304 系（IO-IMP-10 Archive Secure Extraction・
  `92feab3b` で追加、2026-11-10）の per-node identity refusal 契約。
- `NodeIdentity`（`ruviform-io-file/src/vfs.rs`）は
  (volume, index, tag, discriminator, handle) で比較する設計で、
  `name_to_handle_at` の inode generation と birth/change time
  discriminator が inode reuse 窓を閉じる前提。docker コンテナの
  overlayfs 上では delete+recreate が同一 inode に着地した際に
  identity が一致しうる観測（`name_to_handle_at` は非特権コンテナで
  `CAP_DAC_READ_SEARCH` 不足により失敗し handle=None となる可能性、
  overlayfs の btime/ctime 報告差異が候補 — 未確定、切り分けは対象
  crate の lease 外のため深追いしていない）。
- **CI `verify` ubuntu leg の `test-full` と同一 lane での失敗**であり、
  CI 上でも再現する可能性がある（ubuntu-latest は実 fs で inode reuse・
  handle 可否の条件が異なり得るため確定は CI 証拠待ち）。
- 修正は `crates/**` lease 外のため実施しない — Coordinator へ報告し、
  別 Issue 起票要否の判断を委ねる。

## 追記

なし（上記「発見した既存不具合」に test-full 実測を記録済み）。
