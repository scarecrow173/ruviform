# RF288 IO-06 Gate matrix 実走記録（Issue #288）

対象: [Issue #288](https://github.com/scarecrow173/ruviform/issues/288)
（IO-06 Cross-Consumer・OS/FS・Security・性能・互換性 Gate）の
受入 matrix 実走記録 slice。判定語彙・matrix 構造・否定系判定表・
Owner/境界接続表の正本は
[issue-288 design](../../docs/designs/issue-288-io-06-cross-consumer-os-fs-security-gate.md)
（PR #510 で main 統合済み）であり、本書はその**実走結果の記録**
のみを行う。判定基準の再定義・新規証拠基盤の作成は行わない
（Issue #288「専用 YAML・重複レポートは不要」）。

作業 worktree: `C:\Users\akiyoshi\orca\workspaces\ruviform\rf288-gaterun`
branch: `test/288-io06-gate-run`
HEAD（merge 後）: `1b78607ce86b786704457c6e15b91f6286bd71c8`
（`origin/main` fast-forward。lane 実走時の base は `588d29a2`、
両者の差分は docs 簿記のみでコード差分なし — 実走結果はそのまま
有効）
環境: Windows 11 x64 (MINGW64_NT-10.0-26300, build 10.0.26300.0) /
NTFS (C:) / Git Bash / rustc 1.99.0 (b940084d7 2026-09-28) /
cargo 1.99.0 / MSRV lane は `mise exec rust@1.89.0`
(rustc 1.89.0 29483883e 2025-08-04)
実行日時: 2026-11-12 (UTC前後、ローカル実行)

## Lane 実走サマリ（exit code 記録）

全 lane は `mise run <task>` 経由（canonical = mise タスク）。
exit code は `mise run` 自体の終了コード。workspace 系 lane は
`CARGO_BUILD_JOBS=2` 付き（rf306 と同じ運用）。

### 第 1 run（salvage 済み生ログ `lane-run-raw/lanes.log`）

前セッションが実走した記録。io 系 12 lane は `exit=` 記録あり・
全て Pass。workspace 系の記録は次節の再実走で補完した。

| lane | コマンド | 結果 | test 数 | 記録 |
| --- | --- | --- | --- | --- |
| fmt-check | `mise run fmt-check` | Pass (exit 0) | — | lanes.log L1–3 |
| check-io-file | `mise run check-io-file` | Pass (exit 0) | — | lanes.log L4–20 |
| lint-io-file | `mise run lint-io-file` | Pass (exit 0) | — | lanes.log L21–27 |
| test-io-file | `mise run test-io-file` | Pass (exit 0) | **315 passed / 0 failed / 1 ignored**（ignored は `child_create_new_once` — `create_new_contention_multi_process` が spawn する `#[ignore]` helper の既知の意図的 ignore） | lanes.log L28–457 |
| check-io-structured | `mise run check-io-structured` | Pass (exit 0) | — | lanes.log L458–509 |
| lint-io-structured | `mise run lint-io-structured` | Pass (exit 0) | — | lanes.log L510–520 |
| test-io-structured | `mise run test-io-structured` | Pass (exit 0) | **251 passed / 0 failed / 0 ignored** | lanes.log L521–915 |
| check-io-compression | `mise run check-io-compression` | Pass (exit 0) | — | lanes.log L916–940 |
| lint-io-compression | `mise run lint-io-compression` | Pass (exit 0) | — | lanes.log L941–953 |
| test-io-compression | `mise run test-io-compression` | Pass (exit 0) | **330 passed / 0 failed / 0 ignored** | lanes.log L954–1579 |
| check-io-e2e | `mise run check-io-e2e` | Pass (exit 0) | — | lanes.log L1580–1619 |
| lint-io-e2e | `mise run lint-io-e2e` | Pass (exit 0) | — | lanes.log L1620–1632 |
| test-io-e2e | `mise run test-io-e2e` | Pass (exit 0) | **77 passed / 0 failed / 0 ignored** | lanes.log L1633–1834 |
| check | `mise run check` | **Fail (exit 101)** → 下記参照 | — | lanes.log L1835–4242 |
| lint | `mise run lint` | **Fail (exit 66)** → 下記参照 | — | lanes.log L4243–4246 |

### 第 1 run の workspace 系失敗の診断（infra flake）

- `check` (exit 101): `cargo check --workspace --all-targets` が
  `ruviform-video-core` の lib test target で 211 errors。
  先頭 error は `only metadata stub found for rlib dependency
  core please provide path to the corresponding .rmeta file with
  full metadata`（lanes.log L2019 相当）— rustc の incremental/
  キャッシュ成果物の破損 signature であり、コード起因ではない。
  直後に走った `cargo test --workspace`（同一 log 内）では
  `ruviform-video-core` が正常 compile・全 test pass しており、
  同一コードで再現しないことを確認済み。
- `lint` (exit 66): `cargo clippy --workspace --all-targets --
  -D warnings` が診断出力なく即時 `ERROR task failed`。
  clippy の compile 診断が 1 行も出ていないため、check と同じ
  キャッシュ破損による早期終了が疑われる（exit code のみが
  記録され、失敗 test/診断は存在しない）。
- いずれも**コード・契約の失敗ではなく環境起因（infra flake）**
  と診断し、第 2 run で再実走して確定値を得た（下表）。
  本 slice はコード変更を伴わないため「修正して再検証」の対象は
  なく、flake の記録と再実走結果をそのまま証拠とする。

### 第 2 run（本セッション再実走 `lane-run-raw/lanes-rerun.log`）

exit= 記録のない lane（test / test-rt / test-io-msrv）と、
infra flake と診断した check / lint の再実走。全て
`run-lanes-rerun.sh` 経由で逐次実行。

| lane | コマンド | 結果 | test 数 | 記録 |
| --- | --- | --- | --- | --- |
| check | `mise run check` | **Pass (exit 0)** | — | lanes-rerun.log（Finished 1.11s — キャッシュ済み） |
| lint | `mise run lint` | **Pass (exit 0)** | — | lanes-rerun.log（clippy 40.20s） |
| test | `mise run test` | **Pass (exit 0)** | **3772 passed / 0 failed / 10 ignored**（201 result 行全て ok） | lanes-rerun.log |
| test-rt | `mise run test-rt` | **Pass (exit 0)** | **290 passed / 0 failed / 0 ignored** | lanes-rerun.log |
| test-io-msrv | `mise run test-io-msrv` | **Pass (exit 0)** | **742 passed / 0 failed / 1 ignored**（54 result 行全て ok） | lanes-rerun.log |
| check-linux | `mise run check-linux` | **Pass (exit 0)** | —（compile/lint のみ） | check-linux.log |

- `check-linux` は spec の必須 lane 一覧外だが、非 Linux ホストで
  コンパイルされない unix cfg コード経路（io-file/io-archive の
  Linux backend）を `x86_64-unknown-linux-gnu` target で
  check+clippy する補助 lane（`rustup target` 導入済みを確認後
  実走）。OS/FS 軸の Linux 側の静的証拠として記録する
  （実行・実測ではなく cross compile 検証）。

- `test-io-msrv` は `mise.toml` の 2 コマンド構成どおり
  `rust@1.89.0` で io-file/io-archive/io-structured/
  io-compression/io-wav/io-e2e（cmd1）+ io-compression
  `--features deflate`（cmd2）を実走。ignored 1 件は
  `ruviform-io-file/tests/os_publish.rs` の gate spawned
  `#[ignore]` helper（rf306 slice-3 と同一の意図的 ignore）。
- `test` の ignored 10 件は failed=0 を確認した上での記録
  （内訳の個別監査は本 slice 対象外。rf306 slice-4 時点の
  7 ignored から増えているが、増分の同一性は推測しない）。

### 補助証拠（canonical = 上記 mise タスク）

- 旧 API 不在 grep（M-X1 補助確認・本 worktree HEAD `1b78607c`
  上で実施）: `crates/` 配下の `StagedOutput` 参照 **0 件**
  （docs/adr・designs の歴史資料を除く）、`ruviform-media-io`
  の旧 `PublishOptions` struct/re-export **0 件**、同 crate の
  `[dependencies]` は Domain Core 3（media-core・audio-core・
  video-core）のみで io-* 依存なし。
- `consumer-verify` evidence JSON（`lane-run-raw/al/`・`sv/`、
  run 42937524742 artifact の salvage）: tuple・lane status の
  転記元。`al.zip`/`sv.zip` は同 artifact の原本 archive で、
  各々 `consumer-evidence-*.json` 1 件を内包し展開済み JSON と
  byte 一致を確認済み（`unzip -p | diff`）。判定根拠は JSON、
  zip は原本 provenance として保存。

## 受入 matrix（Issue #288 design 正本の行 ID 準拠）

判定語彙: `pass` / `fail` / `blocked` / `not run` /
`unsupported`。venue 略称:

- **local-ntfs**: 本 worktree 実走（Windows 11 x64 / NTFS /
  rustc 1.99.0、上表の exit code）。
- **rf306**: `evidence/rf306/` の先行実走記録（同一 Windows
  NTFS 環境・過去 SHA）への参照。内容の重複実走はせず、主要
  lane は本 slice で最新 SHA にて再実走済み。
- **ci**: `ci.yml` run
  [42937526176](https://github.com/scarecrow173/ruviform/actions/runs/42937526176)
  （main `9865cd57`・2026-11-11・workflow_dispatch・success。
  `9865cd57` は現 main `1b78607c` の祖先で、間の差分は docs
  簿記のみ）。OS 軸セルの一次証拠は `verify (ubuntu-latest)`・
  `verify (windows-latest)`・`verify-macos`・`rt-verify`・
  `compression-verify`・`io-structured-verify` の各 job。
- **consumer-verify**: `consumer-verify.yml` run
  [42937524742](https://github.com/scarecrow173/ruviform/actions/runs/42937524742)
  （main `9865cd57`・2026-11-11・workflow_dispatch・success）の
  `consumer-evidence-*.json`（salvage: `lane-run-raw/{al,sv}/`）。

### IO-01 File（Issue #283 → #295/#296）

| 行 | 系統 | local-ntfs 判定 | 補助 venue | 根拠 |
| --- | --- | --- | --- | --- |
| F-N1 | 正常: StagedFile ライフサイクル + Receipt | **pass** | rf306 Pass | `test-io-file` exit 0・315 tests（staged/os_publish/rooted_read 含む）。型状態遷移と durability evidence は `issue-295` R1–R3・`issue-296` R1–R2 が正本 |
| F-N2 | 正常: CreateNew/Replace mode | **pass** | rf306 Pass | `test-io-file` exit 0。CreateNew 原子排他・Replace 区別は `issue-296` R3–R4 |
| F-N3 | 正常: Durability 3 段階 | **pass** | rf306 Pass（os_fs_probe 6/6 実測） | `test-io-file` + `test-io-e2e`（`os_fs_probe::publish_durability_ladder_reports_measured_evidence` pass・lanes.log L1734 付近）。NTFS で FileSync/DirSync evidence 一致を実測。rf306 fault-fuzz-report の 6/6 出力が詳細根拠 |
| F-X1 | 否定: CreateNew 競合（複数プロセス） | **pass** | rf306 Pass | `test-io-file` exit 0（`create_new_contention_multi_process` — spawned `#[ignore]` helper は親 test が通常実行。rf306 design risk #3 で確認済み）+ `test-io-e2e` fault_boundary の CreateNew 競合試験。※現行コードの子プロセス数は `N=6`（`os_publish.rs` L459）— design/test-spec の「8 子プロセス」記述との齟齬は末尾「観察メモ」参照 |
| F-X2 | 否定: Disk Full / write 途中失敗 | **pass** | rf306 Pass | `test-io-file` の MockFileOps `#[cfg(test)]` fault matrix（正本）+ `test-io-e2e` fault_boundary（実 FS 分: missing-parent・destination dir 競合）。`NotPublished`/RecoveryHandle 回収を確認 |
| F-X3 | 否定: publish 途中失敗・rename 後 DirSync 失敗 | **pass** | rf306 Pass | Mock 系は `issue-296` R5 で pin（DirSync 強制不能分は Mock 正本）。実 FS 分は `test-io-e2e` os_fs_probe/fault_boundary。`Indeterminate` は下記 Unsupported 節 |
| F-X4 | 否定: abort/drop/verify 失敗の temp 残留 | **pass** | rf306 Pass | `test-io-file`（abort→CleanupReceipt・CleanupFailure→RecoveryHandle）+ `test-io-e2e` staged_lifecycle（abort/drop temp 回収） |

### IO-02 Structured（Issue #284 → #297/#298/#299）

| 行 | 系統 | local-ntfs 判定 | 補助 venue | 根拠 |
| --- | --- | --- | --- | --- |
| S-N1 | 正常: JSON/YAML/TOML Serde round-trip | **pass** | rf306 Pass・ci io-structured-verify | `test-io-structured` exit 0・251 tests（feature matrix 4 組合せ: default/xml/xml,file/all-features） |
| S-N2 | 正常: CSV Record streaming | **pass** | 同上 | 同上（csv feature 含む全組合せで pass） |
| S-N3 | 正常: XML Event streaming | **pass** | 同上 | 同上（xml feature 組合せで pass） |
| S-N4 | 正常: write_file/read_file StagedFile 合成 | **pass** | rf306 Pass | `test-io-structured` file feature 組合せ + `test-io-e2e` structured_staged（6）/structured_formats_staged（8: YAML/TOML/XML） |
| S-X1 | 否定: 不正構文・TrailingData・型不一致 | **pass** | 同上 | `test-io-structured` 各 format の negative 系試験 + `test-io-e2e`（truncated JSON decode 拒否等） |
| S-X2 | 否定: 過大データ budget | **pass** | 同上 | `test-io-structured`（max_bytes/max_depth/max_records/max_events の LimitExceeded・境界包含） |
| S-X3 | 否定: CSV field/型不一致・encoding | **pass** | 同上 | `test-io-structured` csv 系 N7–N11 相当試験 |
| S-X4 | 否定: YAML 複数文書・alias | **pass** | 同上 | `test-io-structured` yaml 系試験（trailing/未対応構文拒否） |
| S-X5 | 否定: XML DTD/entity/encoding/mismatched | **pass** | 同上 | `test-io-structured` xml 系試験 + `test-io-e2e` negative_fuzz（DOCTYPE/内部 subset/未定義 entity 拒否） |
| S-X6 | 否定: backend 型の公開 API 露出 | **pass** | rf306 Pass | `test-io-structured` api_surface 試験（serde trait のみ公開）+ feature matrix で backend 非依存を確認 |

### IO-03 Compression（Issue #285 → #300/#301）

| 行 | 系統 | local-ntfs 判定 | 補助 venue | 根拠 |
| --- | --- | --- | --- | --- |
| C-N1 | 正常: Gzip/Raw Deflate round-trip + finish | **pass** | rf306 Pass・ci compression-verify | `test-io-compression` exit 0・330 tests（5 組合せ: default/gzip/deflate/zstd/gzip,deflate,zstd）。VerifiedEnd・finish 冪等は `issue-300` R1–R3 |
| C-N2 | 正常: Zstd frame receipt | **pass** | 同上 | 同上（zstd_format 34 tests/組合せ） |
| C-X1 | 否定: Truncated/Checksum/Trailing | **pass** | 同上 | `test-io-compression`（TruncatedInput/ChecksumMismatch/TrailingData 分類）+ `test-io-e2e` negative_fuzz（corrupt checksum・truncated gzip・deterministic mutation sweep） |
| C-X2 | 否定: 上限ちょうど/+1 超過 | **pass** | 同上 | `test-io-compression`（max_decoded/max_compressed/max_units/max_window_log の境界包含） |
| C-X3 | 否定: Window/Frame・先読み境界 | **pass** | 同上 | `test-io-compression`（SingleUnit/WholeInput・prefetch 非混同） |
| C-X4 | 否定: 未 finish/Drop を完成証拠にしない | **pass** | rf306 Pass | `test-io-compression` encoder/decoder contract + `test-io-e2e` archive_compression_staged（encoder 未 finish drop→verified EOF 不成立→abort） |

### IO-04 Archive（Issue #286 → #302/#303/#304）

| 行 | 系統 | local-ntfs 判定 | 補助 venue | 根拠 |
| --- | --- | --- | --- | --- |
| A-N1 | 正常: ZIP/TAR read・verify receipt | **pass** | rf306 Pass | `test-io-file`（io-archive 側: tar/zip header・round-trip・pax 系）exit 0 + `test-io-e2e` archive_compression_staged |
| A-N2 | 正常: publish 済み archive → ExtractionSession → ExtractedTree | **pass** | rf306 Pass | `test-io-e2e` extraction（10: TAR/ZIP secure extraction round-trip）+ `test-io-file` extract_staging/extract_tar 系 |
| A-X1 | 否定: Path Traversal | **pass** | rf306 Pass | `test-io-file` extract path 系（`..`/絶対/drive/UNC/予約名/NTFS 禁止文字/非 UTF-8 拒否）+ `test-io-e2e` negative_fuzz・extraction（staging 非残存） |
| A-X2 | 否定: Link/重複/casefold 衝突 | **pass** | rf306 Pass | `test-io-file`（rejects_symlink_and_hardlink・case_fold_and_unicode_collisions・exact_duplicate）+ `test-io-e2e` negative_fuzz（external_attrs patch による symlink/special 分類） |
| A-X3 | 否定: Zip Bomb/budget 超過 | **pass** | rf306 Pass | `test-io-file`（entry/total size budget・enforces_entry_count_and_depth）+ `test-io-e2e` negative_fuzz（declared total・entry 数・streamed budget） |
| A-X4 | 否定: 構造破損・Pending・TOCTOU | **pass** | rf306 Pass | `test-io-file`（truncated archive・special typeflags・pax malformed）+ `test-io-e2e` negative_fuzz（corrupt checksum・truncated TAR・corrupt gzip・invalid UTF-8 JSON・mutation sweep）。TOCTOU は脅威モデル境界として `issue-304` design が正本（race-safe 非主張） |
| A-X5 | 否定: extract 途中失敗の staging cleanup | **pass** | rf306 Pass | `test-io-file` extract_staging（abort_removes_the_staging_root・cleanup_refuses_to_delete_a_swapped_root_tree 等 12 tests）+ `test-io-e2e` extraction 失敗経路 |

### IO-05 Migration（Issue #287 → #305・#306）

| 行 | 系統 | local-ntfs 判定 | 補助 venue | 根拠 |
| --- | --- | --- | --- | --- |
| M-N1 | 正常: WAV finalize→seal→readback→publish→reopen | **pass** | rf306 Pass | `test-io-e2e` wav_staged（3）/wav_golden/wav_fault + `test-io-msrv`（rust@1.89）exit 0。`WavMuxer<&mut StagedFile>` の MediaSink 成立は `issue-305` R2–R4 |
| M-N2 | 正常: MediaSource/Sink/Codec 公開契約継続 | **pass** | rf306 Pass | workspace `test` exit 0（3772 tests — media-io/audio-core/video-core 契約試験を含む） |
| M-X1 | 否定: 旧 StagedOutput/旧 PublishOptions 参照 0 件 | **pass** | rf306 Pass（scratch 検証） | 本 worktree grep: `crates/` の `StagedOutput` 0 件・media-io 旧 `PublishOptions` 0 件・media-io deps = Domain Core3 のみ（上記補助証拠）。`issue-305` R1 |
| M-X2 | 否定: Publish 失敗の情報喪失なし | **pass** | rf306 Pass | `test-io-e2e` fault_boundary（PublishFailure の effect/phase/kind/durability/recovery handle 所有権）+ workspace test の media-io error 写像試験 |
| M-X3 | 否定: Media/PCM/RT 非回帰 | **pass**（4GiB 実測を除く） | rf306 Pass・ci rt-verify | workspace `test` 3772/0 failed + `test-rt` 290/0 failed + `test-io-e2e` wav_golden。**実 4GiB ファイル境界は lane なし → not run**（下記 NotRun 節）。RT callback 非混入は DAG audit（rf306 slice-3: `ruviform-rt` deps = arc-swap/rtrb のみ）+ architecture baseline |

### 合成 E2E（Issue #306 → `ruviform-io-e2e`）

| 行 | 系統 | local-ntfs 判定 | 補助 venue | 根拠 |
| --- | --- | --- | --- | --- |
| E-N1 | 正常: 層横断合成（Structured/Archive/Compression/WAV → publish） | **pass** | rf306 Pass | `test-io-e2e` exit 0・77 tests（staged_lifecycle 6・structured_staged 6・structured_formats_staged 8・archive_compression_staged 6・extraction 10・wav_staged 3・wav_golden・wav_fault・fault_boundary 6 等） |
| E-X1 | 否定: 完成証明なし publish 阻止 | **pass** | rf306 Pass | `test-io-e2e`（encoder 未 finish drop・truncated WAV decode 不一致→abort・destination 不存在の否定経路） |
| E-X2 | 否定: 実 OS 再現可能 fault（CreateNew 競合・Replace・abort/drop） | **pass** | rf306 Pass | `test-io-e2e` fault_boundary（6）+ os_fs_probe（1）+ staged_lifecycle の実 FS 分。Mock 系とは重複しない実 FS 検証として `issue-306` 境界 5 どおり |

## 否定系判定表（5 領域）

issue-288 design「否定系判定表」の各行について、実走 lane の
結果を記録する。全て local-ntfs venue で **pass**（根拠は
上記 matrix 行と同一 lane）。

### 1. publish/recovery

| 条件 | 判定 | 根拠 |
| --- | --- | --- |
| CreateNew 競合 → NotPublished+DestinationExists・先入主保全 | **pass** | `test-io-file`（`create_new_contention_multi_process`・現行 N=6 子プロセス）+ `test-io-e2e` fault_boundary |
| FileSync/DirSync 失敗 → Err+正しい phase/effect・Ok 偽装なし | **pass** | `test-io-file` MockFileOps 試験（正本）+ `test-io-e2e` os_fs_probe（実 FS で evidence 一致または Unsupported 正直拒否） |
| rename 後 DirSync 失敗 → Err{Published,FileSynced} | **pass** | Mock 系は `issue-296` R5 で pin。実 FS での DirSync 失敗注入は公開 API から誘発不能 → fixture 化せず Unsupported（下記） |
| Indeterminate → 自動 cleanup/再 publish なし | **pass**（Mock 正本）/ 実 FS 誘発は **unsupported** | `test-io-file` `#[cfg(test)]` MockFileOps 試験が正本（rf306 fault-fuzz-report「Fault 分類の正直さ」どおり） |
| Cleanup 失敗 → CleanupFailure+RecoveryHandle・publish 成功を覆さない | **pass** | `test-io-file` + `test-io-e2e` fault_boundary（recovery handle 所有権） |

### 2. 安全性（Security / Threat Model）

| 条件 | 判定 | 根拠 |
| --- | --- | --- |
| Archive Traversal/Link/重複/casefold → 書込前拒否・staging 非残存 | **pass** | `test-io-file` + `test-io-e2e` negative_fuzz/extraction/rooted_boundary |
| Zip Bomb/展開 budget → LimitExceeded・途中遮断 | **pass** | `test-io-file` + `test-io-e2e` negative_fuzz（宣言事前拒否+実測遮断の両層） |
| XML DTD/外部 entity・YAML alias・encoding → Unsupported/InvalidEncoding | **pass** | `test-io-structured` + `test-io-e2e` negative_fuzz（DOCTYPE/内部 subset/未定義 entity） |
| Structured 過大/深さ/複雑度 → LimitExceeded | **pass** | `test-io-structured`（InputLimits 各軸・境界包含） |
| TOCTOU → race-safe 非主張 | **pass** | `issue-304` design の脅威モデル境界どおり（lexical check 限界を明記。コード試験ではなく設計契約） |
| CRC 等は整合性検出で真正性保証でない | **pass** | receipt 語彙・文書が真正性を主張していないことを `issue-301`/`issue-302` test-spec・docs で確認 |

### 3. resource budget

| 条件 | 判定 | 根拠 |
| --- | --- | --- |
| 各種上限 → LimitExceeded・境界包含・超過 byte 非返却 | **pass** | `test-io-structured`/`test-io-compression`/`test-io-file` の LimitExceeded 系試験（S-X2/C-X2/A-X3 相当） |
| streaming 増分消費 | **pass** | `test-io-structured` S1 系・`test-io-compression` 境界試験（BufRead/Read 単位消費） |
| record/event buffer reuse・capacity 単調非減 | **pass** | `test-io-structured` S2 系試験 |
| 性能比較（速度/Peak Memory/Allocation） | **not run** | 許容差が比較前に未定義（issue-288 design「性能許容差の具体値」Deferred）。lane なし |

### 4. Media/PCM/RT 非回帰

| 条件 | 判定 | 根拠 |
| --- | --- | --- |
| WAV finalize→seal→readback→publish→reopen | **pass** | `test-io-e2e` wav_staged/wav_golden + `test-io-msrv` |
| PCM precision・4GiB 境界・ZeroFrames・NaN/Inf・golden | **pass**（4GiB 実測を除く） | workspace `test`（io-wav contract/roundtrip・media 系 golden 含む 3772 tests）。**実 4GiB は lane なし → not run** |
| MediaSource/Sink/Codec 公開契約非破壊 | **pass** | workspace `test` + M-N2 行 |
| RT callback で I/O/Parser/Compression 非呼出 | **pass** | `test-rt` 290 tests + DAG audit（rf306 slice-3: ruviform-rt の依存は arc-swap/rtrb のみ・io/blocking 依存 0）+ architecture baseline |
| 旧 API 参照 0 件 | **pass** | 本 worktree grep（上記補助証拠） |

### 5. Consumer 非回帰

| 条件 | 判定 | 根拠 |
| --- | --- | --- |
| AudioLens tuple（Ruviform 側 lane） | **pass** | consumer-verify run 42937524742: status=pass・19 lanes（15 pass/2 unsupported/1 skipped/1 not_run）・tuple ruviformRev `9865cd57` × AL `9c25df99`・anonymous・shaMatch=true |
| Synvelis tuple（Ruviform 側 lane） | **pass** | 同 run: status=pass・21 lanes（16 pass/3 unsupported/1 skipped/1 not_run）・SV `fb069c93`・shaMatch=true |
| 片側 PASS を両 Consumer 受入とみなさない | **pass**（独立記録） | 両 consumer を独立に上記のとおり記録。consumer 固有受入（AL #139/#140/#141/#143・SV #82/#84/#25）の完了判定は各 Consumer repo が Owner — 本 slice では consumer-verify の ruviform 側 lane 結果のみを証拠とする |
| consumer 固有受入（Golden・性能・OS 実機・GUI） | **not run / unsupported** | consumer-verify evidence の not_run（GUI-CONTRACT consumer-side: AL #166/#127・SV #175-177 が Owner）・unsupported（GUI visual/OS・macOS/実 OS・PLUGIN-NATIVE-DOCK）をそのまま転記。Ruviform 側 lane の pass を consumer 受入完了と混同しない |

## OS/FS capability 軸（実測 venue 別）

issue-288 design の capability matrix に対し、本 slice の
実測 venue ごとの結果を記録する。FS 種別が runner 既定で
ある点は明記し、「ext4/XFS・APFS を個別に実測した」とは
記録しない。

| OS/FS | venue | 判定 | 根拠・範囲 |
| --- | --- | --- | --- |
| Windows 11 x64 / NTFS | local-ntfs | **pass（実測）** | 全 io lane + workspace lane を実 FS で実走。os_fs_probe が durability 3 段階 × CreateNew/Replace の 6 通りで evidence 一致を実測（rf306 fault-fuzz-report の 6/6 出力・本 run でも同試験 pass） |
| Linux（ubuntu-latest runner 既定 FS） | ci verify (ubuntu-latest) + 補助: local check-linux | **pass（CI 実測 + 静的補助）** | run 42937526176 `verify (ubuntu-latest)` の lint-ci/test-ci が success。io 系 crate を含む workspace test が緑。FS 種別は runner 既定（ext4 等）であり個別 FS 保証の実測ではない。補助として `mise run check-linux`（x86_64-unknown-linux-gnu cross check+clippy・exit 0）で unix cfg 経路の静的健全性を確認 |
| Windows（windows-latest runner 既定 FS） | ci verify (windows-latest) | **pass（CI 実測）** | 同上 `verify (windows-latest)` success |
| macOS（macos-latest runner 既定 FS=APFS） | ci verify-macos | **pass（CI 実測）** | 同上 `verify-macos` success。FS 種別は runner 既定 APFS であり、dir fsync 等の OS 差異の個別実測は含まない |
| Linux vfat/exFAT・NFS | — | **not run** | 該当 lane・実機なし。未実測を Supported としない（issue-288 design 原則） |
| Windows FAT32/exFAT/ReFS | — | **not run** | 同上 |
| macOS/Linux 実機（CI runner 以外） | — | **not run** | 実機環境なし。CI lane 結果を一次証拠とする（design D4 方針） |
| Network/Cloud FS（SMB/NFS/OneDrive 等） | — | **not run / unsupported** | 実測環境なし。durability 保証は FS 依存のため主張しない（rf306 fault-fuzz-report と同一分類） |
| `Indeterminate`・DirSync 後失敗・`TempState::Unknown` の fixture 化 | — | **unsupported** | 公開 API から正直に誘発不能。`ruviform-io-file` の `#[cfg(test)]` MockFileOps 試験が正本（rf306 と同一分類。fixture 化は別 Issue 対象） |

## 残存 NotRun / Unsupported / Blocked 一覧

| 項目 | 分類 | 理由・引き継ぎ先 |
| --- | --- | --- |
| macOS/Linux 実機での OS/FS 別 Gate 実走 | not run | 実機環境なし。CI lane（verify-macos/ubuntu-latest）が一次証拠。実機固有 FS 保証差異（dir fsync 等）は未実測のまま |
| Network/Cloud FS の durability/保証範囲 | not run / unsupported | 実測環境なし。FS 依存保証を主張しない |
| Linux vfat/exFAT/NFS・Windows FAT32/exFAT/ReFS の個別実測 | not run | 該当 FS の lane・実機なし |
| 実 4GiB ファイル publish・境界 | not run | lane 未実装（M-X3/PCM 4GiB 境界の実測部分） |
| 性能比較（速度/Peak Memory/Allocation） | not run | 許容差が比較前に未定義（issue-288 design Deferred Design Concerns） |
| consumer 固有受入（AL Golden/性能/OS 実機・SV Snapshot/Backup/RT 等） | not run / unsupported | consumer-verify の not_run/unsupported lane を転記。各 Consumer repo が Owner（AL #166/#127・SV #175-177 等） |
| `Indeterminate`・DirSync 後失敗・`TempState::Unknown` の公開 fixture 化 | unsupported | MockFileOps `#[cfg(test)]` 試験が正本。fixture 化は別 Issue（Coordinator 起票） |
| MSRV clippy 検証（io 系） | not run | `test-io-msrv` は test 実行のみ。MSRV clippy lane は未追加（rf306 slice-3 と同じ記録） |

## matrix の穴（どの lane にも対応しない必須セル）

以下は issue-288 受入 matrix の必須セルのうち、**既存 lane・
既存証拠のいずれにも実測根拠がない**もの。新規 Issue 起票は
Coordinator が行うため番号は付けない。

1. **実 4GiB ファイル publish/boundary の実測**（M-X3・PCM
   4GiB 境界の一部）— lane 未実装。`test-io-file`/`test-io-e2e`
   に大容量 fixture を流す経路がない。
2. **Network/Cloud FS（SMB/NFS/OneDrive 等）の durability
   実測** — lane・実機なし。OS/Server/Mount 条件ごとの保証範囲
   実測が必要（issue-288 design Deferred）。
3. **非既定 FS 種別の個別実測**（Linux vfat/exFAT/NFS、
   Windows FAT32/exFAT/ReFS）— runner 既定 FS の CI 実測のみで、
   FS 種別ごとの capability 差異（dir fsync 失敗等）は未実測。
4. **macOS/Linux 実機（CI runner 以外）での OS/FS 別 Gate** —
   CI lane は runner 既定環境の実測であり、実機固有条件
   （APFS 固有・ext4/XFS 個別）の検証 lane はない。
5. **性能比較 Gate**（速度/Peak Memory/Allocation、許容差は
   比較前定義が前提）— lane 未実装・許容差未定義。
6. **consumer 固有受入の残り**（AL #166/#127 GUI-CONTRACT
   consumer-side・GUI visual/OS・macOS 実機 lanes、SV
   #175-177 同系・PLUGIN-NATIVE-DOCK）— consumer-verify が
   not_run/unsupported と記録しており、Ruviform 側 lane では
   担保できない Consumer repo Owner の作業。
7. **`Indeterminate`/DirSync 後失敗/`TempState::Unknown` の
   fixture 化** — 公開 API から誘発不能のため unsupported
   記録に留まる。Mock 正本のままか、公開 fault 注入点を
   設けるかの設計判断が必要（rf306 からの持ち越し）。

## 観察メモ（既存文書の齟齬・本 slice 対象外）

- `docs/test-specs/issue-296-io-file-os-backend.md` の Examples
  は CreateNew 競合試験を「8 子プロセス」と記述するが、現行
  コード `crates/ruviform-io-file/tests/os_publish.rs` L459 は
  `const N: u8 = 6`。issue-288 design F-X1 行の証拠/根拠列も
  「8 子プロセス競合」を転記している。**判定への影響なし**
  （試験自体は pass・契約は N に依存しない）が、正本側の
  数値記述と実装の drift として記録する。修正は対象外
  （`crates/`・他 docs は編集範囲外）。

## 参照一覧（実在確認済み）

- Issue: [#288](https://github.com/scarecrow173/ruviform/issues/288)・
  #283/#284/#285/#286/#287（IO-01〜05 設計）・#294/#295〜#306
  （実装・test-spec）・#219（consumer-compat 契約）。
- PR: #391（IO-05 merge）・#395（io-e2e 新設）・#510（本 Gate
  設計）・#512/#533/#549（IO-IMP-12 slice-2/3/4）。
- CI: ci.yml run 42937526176（main `9865cd57`・success）、
  consumer-verify.yml run 42937524742（同 SHA・success）。
- evidence: `evidence/rf306/report.md`・`fault-fuzz-report.md`・
  `feature-matrix-report.md`、本 dir `lane-run-raw/lanes.log`・
  `lanes-rerun.log`・`check-linux.log`・`run-lanes.sh`・
  `run-lanes-rerun.sh`・`al/consumer-evidence-audiolens.json`・
  `sv/consumer-evidence-synvelis.json`。`al.zip`/`sv.zip` は
  同 artifact の原本 archive（各 JSON 1 件を内包・展開済み
  JSON と byte 一致確認済み）。
- test-spec 正本: `docs/test-specs/issue-295`〜`issue-306`
  （`issue-305` は media-io cutover と io-e2e wav-golden-fault の
  2 件、`issue-306` は composition-e2e・fault-fuzz・
  msrv-dag-rt-audit の 3 件が存在）。
