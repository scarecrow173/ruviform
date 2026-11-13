# Issue #306 Slice V+T — 既存 CI 証拠検査と OS-L1 Case 補完

- 日付: 2026-11-13
- 対象 SHA: `e93c438adc4af5b498e10926fe859ac1338ac390`（main HEAD・PR #599 merge）
- 対象 Issue: https://github.com/scarecrow173/ruviform/issues/306
- 文書チェーン: `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/issue-306-io-e2e-cases.md`

## 1. 検査対象 CI run

Coordinator が同 SHA で dispatch した Full CI run を採用。

| 項目 | 値 |
| --- | --- |
| Run | `43428172820` |
| URL | https://github.com/scarecrow173/ruviform/actions/runs/43428172820 |
| Workflow | `ci`（手動 Full CI dispatch） |
| Head SHA | `e93c438adc4af5b498e10926fe859ac1338ac390` |
| 結果 | **completed / success**（全 job success。検査時点で `ui-gpui-verify (windows-latest)` が in_progress だったが後に success で完了。io 系 job は全て先に success） |

証拠の取り方: `gh run view` / `gh api` で job 一覧・conclusion を取得し、
`gh api repos/.../actions/jobs/{id}/logs` で各 job の生ログを取得。
ログ内の `Running unittests` / `Running tests/<binary>` ブロックと
`test result:` 行を crate・binary 単位で対応付けて集計した
（集計は作業用 parser による補助証拠。canonical な検証入口は
`mise run test ruviform-io-e2e` 等の mise task）。
**job conclusion が green であることだけを Pass の根拠にせず、
対象 crate の test binary が実際に実行され 0 failed であることを
ログ行で確認した。**

## 2. OS-L1 実走証拠（verify lane・3 OS）

`verify (ubuntu-latest)` / `verify (windows-latest)` / `verify-macos`
の 3 job はいずれも同一手順:

- `mise run fmt-check` → `cargo fmt --all -- --check`
- `mise run lint-full` → `cargo clippy --workspace --all-targets
  --profile ci -- -D warnings`（io 系 crate の Native Compile/Link
  を `--all-targets` で検証）
- `mise run test-full` → `cargo test --workspace --profile ci`
  （io-e2e / io-file / io-archive / io-structured / io-compression /
  io-wav / io-symphonia / io-avi / media-io を含む）

### 2.1 job / test 実行対応（抜粋・全 job success）

| Job | Job ID | OS | io-e2e | io-file | 備考 |
| --- | --- | --- | --- | --- | --- |
| verify (ubuntu-latest) | 131894691153 | Ubuntu / x86_64 | 77 passed / 0 failed / 0 ignored（12 binary + lib） | 141 passed / 0 failed / **1 ignored**（os_publish の 1 件は `#[ignore]` 付き意図的 skip） | fmt/lint/test 全 step success |
| verify (windows-latest) | 131894691209 | Windows / x86_64 | 77 passed / 0 failed / 0 ignored | 143 passed / 0 failed / 1 ignored | OS 条件付き test 数差（lib 92・rooted_read 25）は既存の cfg 差で fail ではない |
| verify-macos | 131894691243 | macOS / aarch64 | 77 passed / 0 failed / 0 ignored | 143 passed / 0 failed / 1 ignored | 同上 |

io-e2e の 12 integration binary 全件が 3 OS で実行された
（archive_compression_staged 6 / extraction 10 / fault_boundary 6 /
negative_fuzz 10 / os_fs_probe 1 / rooted_boundary 4 /
staged_lifecycle 6 / structured_formats_staged 8 /
structured_staged 6 / wav_fault 9 / wav_golden 8 / wav_staged 3、
lib unittests 0、計 77）。**本 Slice T の +3 件はこの run には
含まれない**（run は `e93c438a` 時点のコード。補完後の 80 件は
ローカル `mise run test ruviform-io-e2e` で実測・merge 後 CI で
統合検証される）。

他 io 系 crate の ubuntu 集計（windows/macos も同構成で 0 failed）:

| crate | ubuntu passed / failed / ignored |
| --- | --- |
| ruviform-io-archive | 172 / 0 / 0（9 binary） |
| ruviform-io-compression | 126 / 0 / 0（gzip+zstd unify・deflate 0） |
| ruviform-io-structured | 150 / 0 / 0（dev-dep 経由全 format unify） |
| ruviform-io-wav | 38 / 0 / 0 |
| ruviform-io-symphonia | 50 / 0 / 0 |
| ruviform-media-io | 66 / 0 / 0 |

## 3. Feature lane 証拠（ubuntu のみ lane）

| Job | Job ID | 内容 | io 関連実測 |
| --- | --- | --- | --- |
| compression-verify | 131894690963 | `check/test/lint-io-compression` × 5 feature 組合せ | default / gzip / **deflate** / zstd / gzip+deflate+zstd を個別 `cargo test -p` で実走。`deflate_format` 11 passed を deflate 単独 run で確認 |
| io-structured-verify | 131894691092 | `check/test/lint-io-structured` × 4 組合せ | default / xml / xml+file / --all-features を実走。all-features で `structured` 129 passed 等 |
| rt-verify | 131894691112 | `cargo test --workspace rt_`（名前 filter） | io 系 test binary は compile/link され filter 実行。io-e2e は `assert_` 部分文字列一致の 4 件のみ pass（**io-e2e に rt_ 契約 test は存在しない** — 後述 §5） |
| msrv-verify | 131894691257 | `test-ofx-msrv` / `test-ui-test-msrv` / `test-observability-msrv` / `test-io-msrv`（`rust@1.89.0`） | test-io-msrv: io-file/archive/structured/compression/wav/e2e を MSRV toolchain で実走（io-e2e 77 passed）。第 2 コマンド `cargo test -p ruviform-io-compression --features deflate` で `deflate_format` 11 passed |
| l1-real-verify ×3 | — | plugin host L1-real lane | io 系 crate 非対象（範囲外として記録） |

### 3.1 Feature 分離の正直な記録

- `cargo test --workspace`（verify lane）では io-e2e の
  dev-dependencies 経由で io-compression `gzip,zstd`・
  io-structured 全 format が unify されるが、**`deflate` は
  io-e2e が消費しないため verify lane では 0 test**。
- `deflate` の実走証拠は **ubuntu のみ**（compression-verify・
  msrv-verify cmd2）。Windows/macOS での deflate Native
  Compile/Link は `clippy --workspace --all-targets` が default
  feature のみを見るため **NotRun**（deflate 未使用 Capability
  の必須化はしない — POLICIES の Required/Deferred 規範どおり）。
- `verify-macos` job 名は workflow 上 `macos-latest`（arm64）。

## 4. Slice T — 追加した L1 Case

`crates/ruviform-io-e2e/tests/staged_lifecycle.rs` に 3 件追加
（既存 6 → 9、io-e2e 合計 77 → 80）。いずれも公開 API のみで
実ファイルシステム上の振る舞いを検証し、production code・
公開 fault seam は変更していない。

| Case | 内容 | 実測結果（Windows 11 ローカル） |
| --- | --- | --- |
| `post_seal_temp_rewrite_is_not_reverified_before_publish` | seal→publish 間に外部 actor が temp file を rewrite（長さ変更あり）→ publish | publish 成功・destination に **改竄後 bytes がそのまま publish される**。seal 時検証は seal までの write のみを保証し、post-seal の temp 改竄は integrity scope 外であることを契約として固定 |
| `post_seal_temp_delete_fails_publish_not_published` | seal→publish 間に外部 actor が temp file を削除 → publish | `PublishEffect::NotPublished`・destination 非作成・recovery handle の cleanup が absent temp で成功（`TempState::Absent`） |
| `recovery_cleanup_failure_returns_handle_for_retry` | CreateNew conflict で publish 失敗→temp retained の recovery handle 取得後、temp を directory に置換して cleanup 失敗→handle が返却される→復元後 retry で cleanup 成功 | `CleanupFailure::take_recovery_handle` で handle 返却・retry `cleanup()` が `TempState::Absent` で成功。incumbent destination 無傷 |

### 4.1 観測された境界（レビュー材料・本 slice では変更しない）

- **post-seal temp 改竄は publish で検出されない**（test 1 が
  実測で固定）。seal の integrity 保証は「seal 時点までに
  staging writer が書いた bytes」の検証であり、temp path は
  publish の rename まで外部から変更可能。これは現行契約の
  正直な記録であり、publish 時の再検証や temp 権限強化は
  `ruviform-io-file` の設計判断（本 slice 対象外）。
- `OsFileOps` は publish 試行前に `prepare_for_publish` で
  temp handle を release するため、失敗後の recovery handle が
  保持する ops の file handle は既に close 済み。test 3 の
  sabotage は rename 先行で Windows の delete-pending 名前
  再利用制約を回避する設計。

## 5. RT Fake Fault / G07 seam の判定材料

- **RT Fake Fault**: io-e2e・io 系 crate の公開 API は全て
  「non-RT only — filesystem I/O」と doc 明記されており、
  rt_ 契約試験の追加対象ではない。rt-verify lane の実測でも
  io-e2e の rt_ 一致は `assert_` 部分文字列の誤検出 4 件のみで
  実 rt_ test は 0。**io-e2e への rt_ 契約 test 追加は不要**
  と判定（rt_ 名称 filter が io binary に非混入であることは
  DAG audit・lane 実測で確認済み）。
- **G07 public fault seam**: `PlatformFileOps`/`MockFileOps` は
  `pub(crate)`/`cfg(test)` で io-e2e から到達不能。公開化・
  実装の判定（Option A/B）は別 slice の設計判断であり、本
  slice は判定材料の整理のみ（本節と §4.1）。

## 6. #288 / #294 との分離（Issue AC 語彙）

- `evidence/rf288/io06-gate-matrix.md` の 38 OS/FS security
  Case は Issue #288 の正本であり、本 slice はその**再実行・
  変更を行わない**。io-e2e 側の L1 実走証拠（§2）と #288 gate
  行列の consumer 受入を混同しない。
- #294 consumer 結果（AudioLens/Synvelis）は cross-repo gate
  の責務で Deferred。io-e2e の L1 Pass を consumer 受入 Pass
  と読み替えない。

## 7. NotRun / Unsupported / Deferred（正直な記録）

| 項目 | 状態 | 理由・後続 |
| --- | --- | --- |
| macOS/Linux 実機での io-e2e 実行 | NotRun | CI runner 上の macOS/Linux は OS-L1 として Pass。物理実機差異は未検証。実機 dispatch が必要 |
| Network/Cloud FS 上の publish/extraction | NotRun | 実 FS 固有保証と区別するため対象外。後続 slice |
| 実 4 GiB file の publish/extraction | NotRun | 容量・時間制約。headless 不可 |
| `deflate` の Windows/macOS compile/test | NotRun | io-e2e 非消費 feature。使用時に lane 追加 |
| `Indeterminate`・`TempState::Unknown`・dir-sync 失敗の公開 API 再現 | Unsupported | fault seam が `pub(crate)`/`cfg(test)`。io-file mock 正本が canonical（G07 別 slice） |
| Windows GUI-L2 / EXT-L3 | Deferred | io-e2e は headless I/O 層。GUI/実デバイス Capability 非提供・未使用のため Required 化しない |
| macOS/Linux GUI-L2 / EXT-L3 | Deferred | 同上（POLICIES: 利用時まで Deferred） |
| #219/#294 consumer cross 集約 | Deferred | cross-repo gate の責務 |
| 持続 Fuzz・性能比較 | Deferred | Issue #306 対象外 |

## 8. Slice T ローカル検証（canonical = mise）

| コマンド | OS | toolchain | exit | 結果 |
| --- | --- | --- | --- | --- |
| `mise run fmt-check` | Windows 11 | repo pin | 0 | Passed |
| `mise run check ruviform-io-e2e` | Windows 11 | repo pin | 0 | Passed |
| `mise run lint ruviform-io-e2e` | Windows 11 | repo pin | 0 | Passed（`-D warnings`） |
| `mise run test ruviform-io-e2e` | Windows 11 | repo pin | 0 | Passed（80 tests / 0 failed / 0 ignored） |

ローカル検証は変更範囲（io-e2e tests）の検証であり、統合受入は
merge 後の対象 SHA の CI 証拠（Coordinator 確認）。
