# RF306 G01/G05 実行証拠レポート — 実 4GiB 境界 opt-in lane + 性能 baseline lane

対象: Issue #306（IO-IMP-12）残件のうち G01（実 4GiB 境界の
StagedFile publish lane）と G05（IO 合成の性能 baseline lane）。

作業 worktree: `G:\orca\workspaces\ruviform\rf306-iolanes`
branch: `feat/306-io-lane-completion`
base: `origin/main` = `d512334ad533c924e8e35c0c02a7f5b2ce13a676`
（最終検証前に `git fetch origin main && git merge origin/main`
済み。merge commit `a7e35772`。G01/G05 の計測コードは merge で
無変更 — io-e2e 関連の差分は本 slice のみ）
環境: Windows 11 x64 / rustc 1.99.1 (58cf8f9c3 2026-10-13) /
cargo 1.99.1 / CPU Intel Core i9-9900K @ 3.60GHz (16 cores) /
NTFS（`std::env::temp_dir()` =
`C:\Users\akiyoshi\AppData\Local\Temp\`、C: drive、計測時空き
約 288 GiB）
実行日時: 2026-11-15

## G01 実 4GiB 境界 lane（`tests/large_staged.rs`）— **15/15 Pass**

canonical 実行コマンド（task spec 指定の `mise run test` 経由）:

```sh
mise run test ruviform-io-e2e --test large_staged -- --ignored --nocapture
# → cargo test --locked -p ruviform-io-e2e --test large_staged -- --ignored --nocapture
```

結果: `test result: ok. 1 passed; 0 failed` — **1008.22s**。
`#[ignore]` opt-in のため通常 `test-io-e2e` lane には載らない。

### Preflight 観測

| 項目 | 観測値 |
| --- | --- |
| temp_dir | `C:\Users\akiyoshi\AppData\Local\Temp\`（NTFS, C:） |
| sizes | `[4294967295, 4294967296, 4294967297]`（4GiB−1 / 4GiB / 4GiB+1） |
| required_probe_bytes | `4563402753`（4GiB+1 + 256MiB margin） |
| probe 手法 | `set_len` + 末尾 byte 書込 + `sync_data` → 削除 |
| 結果 | `G01 preflight: probe allocation OK` |

注意（正直な記録）: NTFS は `set_len` の伸長を遅延割当し得るため、
probe は「書き込み可能な最大ケースが存在し得る」ことの下限確認
であり、実空き容量の厳密な保証ではない。実行中に真の
`StorageFull` が出た場合は残り case を `NotRun` で停止する設計
であり、今回は最後まで `StorageFull` は観測されなかった
（= 15 case 全てが実容量で完走）。

### Case 結果（サイズ × シナリオ、逐次実行・逐次 cleanup）

| size (bytes) | create_new_roundtrip | replace_roundtrip | create_new_conflict_recovery | mid_write_fault_abort | post_seal_temp_delete |
| --- | --- | --- | --- | --- | --- |
| 4294967295 (4GiB−1) | Pass | Pass | Pass | Pass | Pass |
| 4294967296 (4GiB)    | Pass | Pass | Pass | Pass | Pass |
| 4294967297 (4GiB+1)  | Pass | Pass | Pass | Pass | Pass |

各 case の検証内容（公開 API + 実 FS のみ）:

- `create_new_roundtrip`: write(pattern) → seal → `open_readback`
  checksum → `publish(CreateNew, FileAndDirectorySync)` → reopen
  checksum + metadata len + dir children 照合。
- `replace_roundtrip`: incumbent seed → 同経路を `Replace` で publish
  → reopen 照合。
- `create_new_conflict_recovery`: 既存 dest への `CreateNew` が
  `PublishEffect::NotPublished` / `DestinationExists` /
  `PublishPhase::Publish` で拒否され、incumbent bytes が保全され、
  `RecoveryHandle::cleanup` が retained temp を除去することを確認。
- `mid_write_fault_abort`: `&mut StagedFile` への fault wrapper
  （capacity = size/3 で `ErrorKind::StorageFull` 注入、
  `wav_fault.rs` 先例）で write 途中失敗 → `abort` 後に dest 不在・
  residue なし。
- `post_seal_temp_delete`: seal 後に staging temp を外部削除 →
  publish が `NotPublished` → `RecoveryHandle::cleanup` が
  `TempState::Absent` を報告。

payload は位置依存 deterministic pattern（`mix64`）を 4MiB chunk
で生成し、write 時と readback/reopen 時の chunked FNV-1a を照合。
heap は chunk buffer のみ（≈4MiB）で bounded、ピーク disk 使用量は
1 case 分（≈4GiB+1 + margin）。

### G01 の NotRun/Unsupported 記録

- 容量不足: **NotRun は発生しなかった**（preflight OK + 実行中
  `StorageFull` なし）。設計どおり発生時は Pass/Fail に偽装せず
  `NOT RUN` 行を stdout に出して残 case を停止する。
- 実ファイル WAV 4GiB variant: **NotRun（意図的に未実装）**。
  classic RIFF 4GiB 境界は `wav_golden.rs` の discard-sink 検証が
  既存正本（`docs/designs/issue-306-io-lane-completion.md` 参照）。
- macOS/Linux 実機・Network/Cloud FS: 本 lane の対象外
  （Unsupported/NotRun、Issue #306 の後続 gate）。

## G05 性能 baseline lane（`benches/io_composition.rs`）

canonical 実行コマンド: `mise run bench`（`cargo bench --workspace`）。

### Canonical lane の状態 — **workspace 全体は既存破損で Fail**

`mise run bench`（`cargo bench --workspace`）は exit 101 で失敗。
原因は **本 slice と無関係の既存破損**: `ruviform-numeric` の
lib test target が bench profile でコンパイル不能
（`assert_no_alloc::AllocDisabler` が `disable_release` feature +
release で cfg 外、`corr.rs` の `assert_eq!(vec, [])` が serde_json
の `PartialEq<Value> for f64` で型推論不能 — 計 14 errors）。
`crates/ruviform-numeric` は本 slice で未変更、io-e2e からの依存
経路もなく、`cargo test -p ruviform-numeric --lib`（dev profile）
は 116 tests 全 pass を確認済み — bench profile 固有の潜在不具合。
`bench-smoke`（CI）が math/rt/viz のみ対象のため CI では未検出。
**本 slice の lease 外のため未修正** — 別 Issue/担当への引き継ぎ事項。

### 補助証拠（canonical = mise、明記）

`cargo bench -p ruviform-io-e2e --bench io_composition`（本 lane
単独の実走 — io_composition 自体は全 cell 完走・panic なし）:

計測環境 metadata（bench が実行開始時に stdout 出力）:

```text
os: windows x86_64
cpu: Intel(R) Core(TM) i9-9900K CPU @ 3.60GHz
cores: 16
rustc: rustc 1.99.1 (58cf8f9c3 2026-10-13)
criterion: 0.8.2
ruviform-io-e2e: 0.1.0
temp_dir: C:\Users\akiyoshi\AppData\Local\Temp\
```

baseline revision: merge commit `a7e35772`（origin/main
`d512334a` 取込済み、io-e2e 差分は本 slice のみ）。
feature/profile: `bench` profile、io-e2e dev-dep feature set
（`ruviform-io-file` の `fault-injection` が feature unification で
有効 — `#[cfg]` gated のみで hot path 不変）。
FS/OS: 実 FS（NTFS, temp_dir 配下）。

#### peak_alloc probe（計測区間外・heap のみ、reps=3）

| cell | live_before | peak_delta | 判定（1 GiB bound） |
| --- | --- | --- | --- |
| staged_write | 1204 | 1,049,298 | Pass（≈1.0 MiB = chunk buffer） |
| tar_gzip_staged | 1936 | 1,401,354 | Pass（≈1.34 MiB） |

#### Cell 結果（post-merge tree での実走、2026-11-15）

| cell | time [low, med, high] | throughput [low, med, high] | 備考 |
| --- | --- | --- | --- |
| `staged_write` | 11.518 / 11.846 / 12.247 ms | 653.22 / 675.36 / 694.56 MiB/s | 165 iter |
| `staged_seal` | 39.881 / 43.071 / 45.470 ns | —（設計どおり ≈0、type transition） | 2475 iter |
| `staged_publish` | 10.359 / 13.105 / 14.739 ms | 542.77 / 610.47 / 772.26 MiB/s | 110 iter |
| `staged_readback` | 9.1277 / 9.3252 / 9.5774 ms | 835.30 / 857.89 / 876.45 MiB/s | 110 iter、outlier 1 (high mild) |
| `tar_gzip_staged` | 226.77 / 231.91 / 238.10 ms | 33.600 / 34.496 / 35.279 MiB/s | 10 iter（2.0s 不足→3.75s 推定）、outlier 1 (high mild) |
| `tar_zstd_staged` | 14.103 / 16.753 / 19.984 ms | 400.31 / 477.52 / 567.27 MiB/s | 165 iter、outlier 2 (1 mild + 1 severe high) |

#### 計測ブレの正直な記録（2 回実走の比較）

同一コードで merge 前（G01 直後・disk cache 状態が異なる）と
merge 後の 2 回実走した。`staged_publish` は 1 回目
24.1–50.8 ms（20 iter のみ、med 36.956 ms）→ 2 回目
10.4–14.7 ms（110 iter、med 13.105 ms）と大きく変動した。
`staged_write`/`readback`/tar 系も数〜20% の run 間差がある。
provisional ±25% 閾値の意味するところどおり、**同一マシン・
同一 profile の連続再計測でもこの程度のブレがある**ことを
baseline 記録として残す。consumer 固有閾値は #294 系の後続
gate に委譲され、本 baseline は回帰調査のトリガとして使う。

### G05 の NotRun/Unsupported 記録

- `mise run bench` workspace 全量: **Fail（既存破損、上記）**。
  io_composition を含む他 bench は compile 失敗のため未実行。
- 4 GiB 入力の反復計測: 対象外（G01 lane の責務、bench 設計で
  8 MiB に pin 済み）。
- consumer 固有閾値（f64/RT/long-media/project-save）: NotRun —
  #127/#28/#84 + #294 の各 consumer gate が正本。
- WAV/structured/secure-extraction の性能 cell: 本 bench の
  scope 外（IO composition 代表経路のみ、設計で pin 済み）。

## 実行した検証一覧

| コマンド | 結果 |
| --- | --- |
| `cargo check --locked -p ruviform-io-e2e --all-targets`（補助、check-io-e2e 同等） | Pass（merge 後も Pass） |
| `cargo clippy -p ruviform-io-e2e --all-targets -- -D warnings`（lint-io-e2e 同等） | Pass |
| `cargo fmt --all -- --check`（fmt-check 同等） | Pass |
| `cargo test -p ruviform-io-e2e`（test-io-e2e 同等） | Pass（全 test + doctest） |
| `cargo test -p ruviform-io-e2e --test corpus_replay -- --ignored --nocapture`（test-io-e2e-corpus 同等） | Pass（48 cases/format、panic なし、約 26.6ms） |
| `mise run test ruviform-io-e2e --test large_staged -- --ignored --nocapture`（G01 canonical） | Pass（15/15、1008.22s） |
| `mise run bench`（G05 canonical） | **Fail exit 101** — ruviform-numeric lib test の bench-profile 既存コンパイル破損（本 slice 範囲外） |
| `cargo bench -p ruviform-io-e2e --bench io_composition`（G05 補助） | Pass（全 6 cell 完走、panic/peak 違反なし） |
| `cargo test -p ruviform-numeric --lib`（破損切り分け確認） | Pass（116 tests — bench profile 固有の破損と確認） |

OS/Backend/FS: Windows 11 x64 / NTFS / 実 FS（temp_dir）。
toolchain: rustc/cargo 1.99.1、criterion 0.8.2、bench profile。
