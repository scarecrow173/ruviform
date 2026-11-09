# RF306 実行証拠レポート (Issue #306 / IO-IMP-12)

作業 worktree: `C:\Users\akiyoshi\orca\workspaces\ruviform\rf306-io-gates`
branch: `feature/io-composition-candidate-gates`
base: `origin/main` = `ad99d181c34cded1e56489fa9542027fe0dfc966`（branch 起点。完了時点で `origin/main` は RF393 統合後の `ccf61a21abf4cc7ad669e9d5eb7320edc97b4c9a` へ進んでおり、push 前に merge 済み）
環境: Windows 11 x64 (MINGW64_NT-10.0-26200) / rustc 1.99.0
(b940084d7 2026-09-28) / cargo 1.99.0 / NTFS
実行日時: 2026-11-07 (UTC)

## 実施した検証（全て実走・実値）

| 検証 | コマンド | 結果 | 備考 |
| --- | --- | --- | --- |
| fmt | `mise run fmt-check` | Pass (exit 0) | `cargo fmt --all -- --check` |
| crate check | `mise run check-io-e2e` | Pass (exit 0) | `cargo check -p ruviform-io-e2e --all-targets` |
| crate lint | `mise run lint-io-e2e` | Pass (exit 0) | `cargo clippy -p ruviform-io-e2e --all-targets -- -D warnings` |
| crate test | `mise run test-io-e2e` | Pass (exit 0) | 31 tests 全 pass |
| workspace check | `CARGO_BUILD_JOBS=2 mise run check` | Pass (exit 0, 1m07s) | `cargo check --workspace --all-targets` |
| workspace lint | `CARGO_BUILD_JOBS=2 mise run lint` | Pass (exit 0, 23.65s) | `cargo clippy --workspace --all-targets -- -D warnings` |
| doc index 再生成 | `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js apply --type all` | applied 6 | managed index のみ再生成（whitelisted repairs） |
| merge 後再検証 | `origin/main` = `ccf61a21` を merge（衝突は managed index 4 件のみ・union 解決）後に `check-io-e2e` / `test-io-e2e` / `fmt-check` / `mise run check`（CARGO_BUILD_JOBS=2）を再実走 | 全 Pass (exit 0) | merge commit `d40a9ba`、31 tests 変わらず全 pass |
| workspace test | `CARGO_BUILD_JOBS=2 mise run test`（`cargo test --workspace`） | Pass (exit 0) | **2773 passed / 0 failed / 1 ignored**（merge head `a99e953` 上で全実行）。main `ccf61a2` で既知の plugin-host 起動 Timeout flake は本 run では**未発生** — 失敗が観測されなかったため分類対象なし（非発生を同一 flake 推測で記録しない） |

独立 review は runtime subagent 2 名（Reviewer A: コード/契約・B: docs/証拠の
正直さ）で実施し、Validator→Judge disposition を個別に記録する。

### Validator → Judge disposition（31 合成試験・review findings）

| finding | disposition | 事実根拠 |
| --- | --- | --- |
| A-B1: `(v % 251) as u8` が `cast_possible_truncation` で lint fail | **REJECTED** | `mise run lint-io-e2e` 実走 exit 0。clippy の既知値解析が `v % 251 ≤ 250` を証明し lint 非発火（workspace lint も merge 後再実走で exit 0） |
| A-B2: `(‖{…})()` IIFE・`map_err(\|e\| e.to_string())`・`typeflag as char` が pedantic lint fail | **REJECTED** | 同上: `lint-io-e2e` 実走 exit 0（該当 lint は workspace lints で非対象/非発火） |
| A-N1: crafted ustar header の version field が `"0\0"`（POSIX `"00"` 非準拠） | **ADOPTED** | `tar/header.rs` の `VERSION=b"00"` と照合して実バグ確認。`header[263..265]="00"` に修正済み |
| A-N2: case-fold 試験名/コメントが「plan phase 拒否」を過剰主張 | **ADOPTED** | TAR extraction は streaming（先 entry は staging 済み）— `case_fold_collision_is_refused_and_tree_removed` に改名 + `staging_root` 不在 assert 追加 |
| A-N3: `zip_extract_verifies_and_limits_entries` が limit 未設定で名不実 | **ADOPTED** | `zip_extract_verifies_extracted_tree` に改名 |
| A-N4: special entry の refusal kind が一律 `is_err` で区別なし | **ADOPTED** | reader 由来 `Unsupported`（b'6'）と policy 由来 `InvalidInput`（b'1'/b'2'）を kind 別 assert に分離 |
| A-N5/N6: lifecycle 競合・traversal 試験が staging/temp 残存を未 assert | **ADOPTED** | `take_recovery_handle`+`cleanup`+temp 不在 assert、失敗時 `staging_root` 不在 assert を追加（追加後 test-io-e2e 31/31 再 pass） |
| A-N7/N10: zstd は entry 数のみ・WAV decode は byte 数のみの弱い照合 | **ADOPTED** | zstd 全 `(name,payload)` 照合、WAV 独立 decode を payload バイト一致に強化 |
| A-N8: budget 試験が 2 つの半試験を接着した命名 | **REJECTED（名称のまま維持）** | 内容は declared-size 拒否と streamed budget 拘束の 2 観点を 1 試験で assert しており、doc にはその旨明記済み。分割は試験粒度の好みであり契約上の誤記ではない |
| A-N9 + B/Q3 系: staged_lifecycle・wav の一部試験が io-file/os_publish・muxer 単体試験と overlap | **PARTIAL-ADOPTED / 維持** | seal→verify→publish→reopen の合成フレーミングが境界試験としての差分。extraction 重複指摘は「層越え失敗分類 + staging 非残存 + 公開 API 経路の統合確認」として残置 |
| B-1: report base SHA が branch point で誤読余地 | **ADOPTED** | `（branch 起点…）` 注記 + merge 実施を明記 |
| B-3/B-4: test-spec R8・design wav 行が falsified 前提（未 finalize drop→decode 失敗）を記述 | **ADOPTED** | backend が drop 時 best-effort finalize する事実（実走で確認）に基づき truncated payload 表現へ書換え |
| B-5: IR `generated` に impl/ir index 未記載 | **ADOPTED** | impl index は tool 非管理のため手動 row 追加 + `modified` に記載 |
| Judge 最終判定 | **no blocker** | A の唯一の実バグ（N1）を含む採用指摘を全て反映・再実走で緑維持。未採用は全て事実根拠つきで REJECTED/維持を記録 |

### `test-io-e2e` 内訳（31 件全 pass）

- `staged_lifecycle`: 6 — seal→readback→publish→reopen、CreateNew
  競合→`NotPublished`+`DestinationExists`+先入主 bytes 保全、
  `Replace` owner 差替え、sealed abort、未 seal drop temp 回収、
  readback seek。
- `structured_staged`: 6 — json/csv → `&mut StagedFile` 直列化→
  seal→独立 decode→publish、`write_file` compose、Serialize
  失敗→不 publish、truncated JSON → strict decode 失敗→abort、
  `Publish` 失敗で `PublishFailure` 保持。
- `archive_compression_staged`: 6 — TAR→gzip/zstd→`&mut StagedFile`
  round-trip（各層 finish/verified EOF）、encoder 未 finish drop→
  verified EOF 不成立→abort、truncated gzip member→finish 失敗→
  abort、declared size overrun→write 時点拒否、streamed budget
  拘束、ZIP indexed write/read on staged。
- `extraction`: 10 — TAR/ZIP secure extraction round-trip、
  `..`/nested traversal/absolute path/symlink/hardlink/special
  refuse、case-fold canonical collision、total-bytes budget、
  失敗時 staging 回収 or recovery handle。
- `wav_staged`: 3 — `WavMuxer<&mut StagedFile>` → `finalize` →
  `into_sink` 借用解除 → `seal` → readback symphonia 独立 decode →
  `publish` → 再 decode、truncated WAV → 独立 decode 不一致→
  abort、二重 finalize 拒否。

## RF #305 候補 scratch 検証（T6）

- source SHA: `82256cff2caff45c41ce169e6aa8bda506923579`
  （PR #391 head、"feat(media-io): 旧StagedOutput撤去・StagedFile型
  状態APIへ移行（RF305）"）

### scratch run 1 — 旧 API 不在 + io-wav 単体（補助証拠・直接 cargo）

- scratch 配置: `%TEMP%\rf306\rf305-scratch`（`git worktree add
  --detach`、検証後 `git worktree remove` で除去。自 worktree と
  TEMP 外の書込なし）
- 補助証拠の位置付け: 本 run は直接 cargo 実行であり正規 Pass では
  ない。合成候補受入の根拠は run 2（正規 mise lane + 本 31 fixture
  接続）を使う。
- 実行コマンドと結果:
  - `grep -rn "StagedOutput" $SCRATCH/crates` → **0 件**
    （旧 StagedOutput は候補から撤去済み）→ Pass
  - `grep -rn "pub struct PublishOptions\|pub use.*PublishOptions\|
    StagedOutput" $SCRATCH/crates/ruviform-media-io/src/` →
    **0 件**（media-io の旧 publish API 不在）→ Pass
  - `ls $SCRATCH/crates/ruviform-media-io/src/` → `staged.rs` 不在
    → Pass
  - `cd $SCRATCH && CARGO_BUILD_JOBS=2 cargo check -p
    ruviform-io-wav --all-targets` → **Pass (exit 0)**
    （`WavMuxer<&mut StagedFile>` パターンを含む contract tests が
    候補上で type-check 通過）
  - `cd $SCRATCH && CARGO_BUILD_JOBS=2 cargo test -p
    ruviform-io-wav` → **全 38 件 Pass**（unit 7 + contract 19 +
    roundtrip 9 + doctest 3）。contract.rs L435 以降が finalize→
    借用解除→seal→readback 独立 decode→publish→reopen の
    required order を実装・実走している。
- 注: candidate 上の io-wav dev-dep `ruviform-io-file` は
  scratch Cargo.toml L25 に実在確認。

### scratch run 2 — 本 31 合成 fixture を候補へ実接続（正規 mise lane）

- scratch 配置: `%TEMP%\rf305-scratch2`（`git worktree add --detach
  $TEMP/rf305-scratch2 82256cff…`）。**uncommitted** 接続のみ:
  `crates/ruviform-io-e2e/` をコピー配置、workspace `members` に
  1 行追加、`mise.toml` に本 PR と同一の check/lint/test-io-e2e
  task 定義を追記。候補側ソース・他 worktree・main への変更なし。
- 実行コマンドと結果（全て scratch 内・`CARGO_BUILD_JOBS=2`）:
  - `mise run check-io-e2e` → **Pass (exit 0, 19.24s)** —
    旧 API 撤去後の候補公開面に対し 31 fixture 全てが型整合
    （`WavMuxer<&mut StagedFile>`・`PublishOptions`・
    `ExtractionSession` 等の候補 API をそのまま使用）。
  - `mise run lint-io-e2e` → **Pass (exit 0, 2.43s)**
    （`--all-targets -D warnings`）。
  - `mise run test-io-e2e` → **Pass (exit 0)・31/31 tests 全 pass**
    — staged lifecycle 6 / structured 6 / archive-compression 6 /
    extraction 10 / wav 3 が候補 revision 上で同一緑。
- 意味: 合成 fixture は main と候補の双方で同一公開面・同一結果を
  示した（grep 不在 + io-wav 単体 green だけを受入に転用せず、
  fixture 自体を候補へ接続して実走した根拠）。
- scratch worktree は検証後 `git worktree remove` で除去予定。

## NotRun / Blocked / Unsupported（正直な記録）

- `mise run test`（`cargo test --workspace`）: **実施済み Pass
  (exit 0)・2773/0 failed/1 ignored**（上表参照）。既知の
  plugin-host 起動 Timeout は本 run では未発生 — 失敗名/原因の
  同一性を推測せず、観測された事実のみ記録。
- `check-io-compression` / `lint-io-compression` /
  `test-io-compression`、`check-io-structured` /
  `lint-io-structured` / `test-io-structured`: **NotRun** —
  対象 crate のソース変更なし。合成側は `test-io-e2e` で
  json,csv,file + gzip,zstd feature 組合せを実走。
- Consumer 両候補受入（AudioLens/Synvelis 実 runner）:
  **Blocked** — AL 候補は AL127 `SinkConfig.span_events` 修復
  待ち（別 owner、現 AL main は RF 候補 check-ci fail）、
  SV 候補 head `e8153846…` は DirSync 未達 P1 merge 保留かつ
  synvelis-render 側に旧 `StagedOutput`/`PublishOptions` 参照残
  （export.rs:27, spec.rs:13, player/tests/pc_b.rs:40 —
  coordinator msg_a63a685660ed 通知済み事実、SV repo は編集
  していない）。正式両 Consumer 受入は coordinator が候補
  tuple/dispatch を管理。
- 実 fault injection（dirsync 後失敗/Indeterminate/Unknown
  temp/owner identity 衝突）の OS 強制再現: **NotRun（実装層
  カバー済みを明記）** — io-file の mock adapter `#[cfg(test)]`
  群が同分類を既検証。合成層では実 OS で再現可能な
  NotPublished/DestinationExists/Replace/abort/drop/recovery
  handle を実走で検証。
- Fuzz/性能計測/全 OS 組合せ: **NotRun** — scope 外（無根拠
  pass 禁止の契約どおり）。
- macOS/Linux 実走: **NotRun** — 本検証は Windows NTFS のみ。

## 既存資産との重複回避（監査）

- `crates/ruviform-io-compression/tests/composition.rs` は
  gzip→raw-deflate の crate 内合成。本 crate は公開 API 横断の
  staged-publish 契約を試す層であり mirror ではない。
- 新規証拠 DB/独立 workflow/runner/CI framework は追加してい
  ない（RF294 `consumer-verify` 再利用方針どおり）。
- docs/designs,test-specs,impl/ir の issue-295/296/304 系旧文書に
  `AlreadyExists`（旧 `PublishFailureKind` variant 名）の記述が
  残る — 本 Issue 範囲外のため未修正（報告のみ）。

## 成果物

- `crates/ruviform-io-e2e/`（test-only クレート、公開 API なし）
- `mise.toml` +3 task（check/lint/test-io-e2e）
- docs チェーン 9 本（spec/design/plan/test-spec/task×6、
  index 反映済み）+ 本 evidence

---

# Slice 3 追記 — IO MSRV lane・DAG/RT 境界 audit・License 状態（Issue #306 残存受入行）

本節は slice 3（MSRV lane + DAG/RT audit 証跡）の記録であり、
上記 slice 1 の worktree/branch/base とは別の実行環境である。

- 作業 worktree: `C:\Users\akiyoshi\orca\workspaces\ruviform\rf306-msrv`
- branch: `scarecrow173/rf306-msrv`（PR head）
- base: `origin/main` = `5267141369fe63821f5eb2aba8b0223f7aa67319`
  （branch 起点時点で origin/main tip と一致）
- 環境: Windows 11 x64 / rustc 1.99.0（pin toolchain）+
  rust@1.89.0（MSRV lane）/ cargo 1.99.0 / NTFS
- 実行日時: 2026-11-10 (UTC)
- doc チェーン: `docs/{specs,designs,test-specs,plans,tasks,
  impl/ir}/issue-306-*msrv-dag-rt-audit*`（slice 3）

## MSRV lane（`test-io-msrv`）実走結果

`mise.toml` に `[tasks.test-io-msrv]` を追加
（`test-ofx-msrv`/`test-ui-test-msrv` の直後。`mise exec
rust@1.89.0` + `CARGO_TARGET_DIR = "target/msrv-io"`）。
2 コマンド構成:

1. `mise exec rust@1.89.0 -- cargo test -p ruviform-io-file
   -p ruviform-io-archive -p ruviform-io-structured
   -p ruviform-io-compression -p ruviform-io-wav
   -p ruviform-io-e2e`
2. `mise exec rust@1.89.0 -- cargo test -p
   ruviform-io-compression --features deflate`

実行コマンド: `mise run test-io-msrv` → **Pass (exit 0)**。

- toolchain 確認: `mise exec rust@1.89.0 -- rustc --version`
  → `rustc 1.89.0 (29483883e 2025-08-04)`
- 合計 **680 passed / 0 failed / 1 ignored**（unittest/
  integration/doc test の全 53 集計行が `test result: ok`）。
  ignored 1 件は
  `ruviform-io-file/tests/os_publish.rs` の gate spawned
  `#[ignore]` helper（`create_new_contention_multi_process`
  が spawn する非 standalone test）であり既存の意図的
  ignore で、本 lane の回帰ではない。

| crate | tests（cmd1、feature unify 適用後） | doctests | 備考 |
| --- | --- | --- | --- |
| ruviform-io-archive | 154 | 10 | 単体 feature なし |
| ruviform-io-compression | 96 | 7 | io-e2e dev-deps 経由で gzip+zstd が unify（deflate_format は 0 件 = feature 未 enable を確認） |
| ruviform-io-e2e | 77 | 0 | 全統合 fixture（staged 6・structured 6+8・archive_compression 6・extraction 10・fault 6・negative_fuzz 10・os_fs_probe 1・rooted 4・wav_fault 9・wav_golden 8・wav_staged 3） |
| ruviform-io-file | 132（1 ignored） | 11 | mock `#[cfg(test)]` 群を含む |
| ruviform-io-structured | 117 | 12 | io-e2e dev-deps 経由で json,yaml,toml,csv,xml,file 全 format が unify |
| ruviform-io-wav | 35 | 3 | contract 19 + roundtrip 9 + lib 7 |
| ruviform-io-compression（cmd2, `--features deflate`） | 21 | 5 | deflate_format 11・decoder/encoder contract の deflate 系・api_surface。gzip_format/zstd_format は 0 件（feature 非 enable を確認） |

feature unify の実測確認: cmd1 で io-structured の
`tests/structured.rs` が 117 tests（全 format 有効時の規模）
・io-compression の `gzip_format` 20 + `zstd_format` 34 +
`deflate_format` 0、cmd2 で `deflate_format` 11 +
gzip/zstd 各 0 — E2E 消費 feature と deflate 補完が意図どおり
分離・網羅されている。

## DAG/RT 境界 audit（`cargo metadata` / `cargo tree` 実測）

全て既定 toolchain（rustc 1.99.0）で実行。判定は実コマンド
出力に基づく。

### A1. `ruviform-io-e2e` は leaf

- `cargo tree -i ruviform-io-e2e` → 出力は
  `ruviform-io-e2e v0.1.0 (...)` 自身のみ（逆依存なし）。
- `cargo tree -i ruviform-io-e2e --edges dev` → 同上（dev 辺
  を含めても逆依存なし）。
- `grep -rn "ruviform-io-e2e" crates/*/Cargo.toml Cargo.toml`
  → 自身の `name` 行と workspace `members` 行のみ。依存宣言
  0 件。
- `cargo metadata` parse（python）: 全 workspace member の
  `dependencies` を走査し `ruviform-io-e2e` を参照する
  package は **0 件** → **Pass（leaf 確認）**。

### A2. `ruviform-media-io` の Domain-Core3 依存方向

- `cargo tree -p ruviform-media-io --depth 1` →
  `ruviform-audio-core`・`ruviform-media-core`・
  `ruviform-video-core` の 3 path 依存のみ。
- `cargo tree -p ruviform-media-core` / `-p
  ruviform-audio-core` / `-p ruviform-video-core --depth 1`
  → いずれも `ruviform-media-io`・`ruviform-io-*` への
  依存なし（media-core は dev-dep `assert_no_alloc` のみ、
  audio-core/video-core は media-core への下向き依存のみ）。
- `cargo metadata` parse: `media-io.dependencies` の名前集合
  == `{ruviform-media-core, ruviform-audio-core,
  ruviform-video-core}`（Domain-Core3）で一致、3 core の
  dependencies に media-io/io-* を含むものなし →
  **Pass（下向き一方向を確認）**。

### A3. `ruviform-rt` は io/blocking 依存を持たない

- `cargo tree -p ruviform-rt` → 通常依存は `arc-swap`・
  `rtrb` のみ（いずれも外部 crate）。dev-dependencies は
  criterion bench harness（`alloca`/`ciborium`/`clap` 等の
  bench 計測用）で production 依存ではない。
- `cargo metadata` parse: `ruviform-rt.dependencies` =
  `arc-swap`(normal)・`rtrb`(normal)・`criterion`(dev) の
  3 件のみ。`ruviform-io-*`・blocking 系名・workspace path
  依存は **0 件** → **Pass**。

## License 状態の記録

- `[workspace.package] license = "MIT"` は配布方針確定までの
  **仮置き**であり、リポジトリの license は**未決定**
  （`LICENSE` ファイル非存在）。AGENTS.md「ライセンス注記」
  の記述どおりであり、本 slice で決定・承認はしていない。
- `publish.workspace = true` の `publish = false` は未決定
  クレートを公開しない licensing ADR の原則であり、license
  決定を意味しない。

## Slice 3 の NotRun / Blocked / Unsupported（正直な記録）

- macOS/Linux 実機での MSRV/IO 実測: **NotRun** — 本検証は
  Windows 11 x64 / NTFS のみ。
- Network/Cloud FS での durability/IO 実測:
  **Unsupported** — 実測環境なし。durability 保証は FS 依存
  のため主張しない（slice 2 report と同じ扱い）。
- MSRV clippy 検証: **NotRun（lane 未追加）** — `test-io-msrv`
  は test 実行のみ。MSRV clippy の扱いは `docs/development-
  guidelines.md`（Issue #223）の情報 lane 方針に従う。
- Consumer 実測（AudioLens #139/#143・Synvelis #82/#84）:
  **NotRun/Blocked** — #219/#294 cross-repo gate の責務
  （slice 2 report の引き渡し節を参照）。
- 実 4 GiB ファイル publish・性能計測: **NotRun** — scope 外。
- `Indeterminate`・DirSync 後失敗・`TempState::Unknown`:
  **Unsupported（fixture 化せず）** — 公開 API から誘発不能。
  `ruviform-io-file` `#[cfg(test)]` MockFileOps 試験が正本
  （slice 1・2 と同じ分類。本 slice で偽装 fixture は追加
  していない）。
- Issue #306 の close: **対象外** — 残存受入行は本 slice で
  全てではない（consumer 実測等が残る）。

## Slice 3 で変更したもの

- `mise.toml`: `[tasks.test-io-msrv]` 追加（+15 行・task 定義
  と説明コメントのみ。既存 task 無変更）。
- `evidence/rf306/report.md`: 本節追記。
- `crates/ruviform-io-e2e/README.md`: 検証節に `test-io-msrv`
  を追記。
- `docs/development-environment.md`: opt-in task 一覧に
  `test-io-msrv` を追記。
- docs チェーン 6 本（spec/design/test-spec/plan/task/impl）
  + managed index 再生成。
- crate の公開 API・実装・feature・依存: **無変更**。
