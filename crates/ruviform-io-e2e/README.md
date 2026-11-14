# ruviform-io-e2e

IO-01〜05 の公開 API を層横断で合成する E2E 試験専用クレート
（Issue #306 / IO-IMP-12）。公開 API・本番コードを持たず、
`tests/` 配下の integration test のみを収録する。

## 責務

- `StagedFile → SealedFile → publish → 再 Open` の型状態・実 OS
  保証を、Structured（`write_file`/`read_file`）、TAR→Compression、
  ZIP/TAR→Secure Extraction、WAV `finalize→seal→readback→publish`
  の各合成境界で pin する（`staged_lifecycle.rs`）。seal→publish
  間の外部 temp rewrite/delete・RecoveryHandle cleanup 失敗→retry
  の実 FS 経路もここで検証する。
- 完成証明（`finish`/`finalize`）なし・Verified EOF 未完・不完全
  形式の bytes が publish されない否定経路を検証する。
- Path Traversal・canonical/casefold 衝突・resource budget 超過の
  否定 fixture で staging 非残存を検証する。
- Structured 追加境界（YAML/TOML `write_file→read_file`、XML
  event parse、publish 後 tamper の型付き decode 拒否）を検証する
  （`structured_formats_staged.rs`）。
- 公開 API のみで誘発できる fault 境界（Create/Encode/Publish の
  失敗分類・recovery handle 所有権・publish 競合・TAR+gzip 複合
  の publish 前独立検証）を検証する（`fault_boundary.rs`）。
  `Indeterminate`・DirSync 後失敗・`TempState::Unknown` は実 OS
  で正直に誘発不能なため実 FS fixture 化せず、opt-in
  `fault-injection` feature 経由の `#[doc(hidden)]` mock facade
  による公開 API 契約試験（`fault_seam.rs`、Issue #306 G07）と
  `ruviform-io-file` の `#[cfg(test)]` mock 試験を正本とする。
- Negative/fuzz fixture（ZIP symlink/special entry・ZIP bomb
  budget・XML DOCTYPE/entity 拒否・corrupt/truncated
  TAR/gzip/JSON・決定的 mutation sweep）を検証する
  （`negative_fuzz.rs`）。敵対的 TAR bytes・ZIP central
  directory `external_attrs` の patch は `tests/common` の
  helper で生成し、本番 API は変更しない。
- publish durability ladder を実 FS で probe し、測定された
  `DurabilityEvidence` のみを記録する（`os_fs_probe.rs`）。
- `RootedDir`/`RelativePath` capability を publish 済み・
  extraction 済み tree に適用し、NoFollow read・列挙・locator
  文法拒否・link escape 拒否を検証する（`rooted_boundary.rs`）。
- WAV byte golden（Issue #305）: publish 済み destination bytes を
  RIFF/WAVE 仕様から独立に組み立てた期待値と byte-exact 照合する
  （ZeroFrames・WAVEFORMATEX/EXTENSIBLE・S16 payload・U8/S16/S24/
  S32/F32 encoder edge sample・pad byte・classic RIFF 4 GiB 境界を
  discard sink で検証）（`wav_golden.rs`）。期待値は
  `tests/common/wav.rs` が公開 API と仕様のみから構築し、
  `ruviform-io-wav` の private helper には依存しない。
- WAV fault fixture（Issue #305）: `&mut StagedFile` 上の
  flush/write fault wrapper で finalize `Io`・disk-full・
  seal 後 cancel・muxer Drop best-effort finalize（completion
  証拠でないこと）を検証し、publish outcome を
  `Published`/`NotPublished`/not-provably-unpublished に分類する
  （`wav_fault.rs`）。`Published`/`Indeterminate` の失敗 effect・
  DirSync 後失敗・`TempState::Unknown` は実 OS で誘発不能のため
  NotRun（`fault_seam.rs` と `ruviform-io-file` の mock 試験が
  正本）。
- bounded corpus replay（Issue #306 G08）: writer 正規 bytes を
  種に splitmix64（固定 seed）で mutation した corpus を
  json/yaml/toml/csv/xml/gzip/zstd/zip/tar の各 reader へ replay
  し、panic しないこと・typed error 分類・byte/decoded byte/
  deadline budget を検証する（`corpus_replay.rs`）。`#[ignore]`
  付きの opt-in で、`mise run test-io-e2e-corpus` 経由でのみ
  実行される（seed/case/budget/形式別除外はファイル冒頭に明記）。
- 実 4 GiB 境界 lane（Issue #306 G01）: `#[ignore]` 付き opt-in
  で、実ファイルシステム上の公開 API のみを使い 4GiB−1 / 4GiB /
  4GiB+1 の `CreateNew`/`Replace` publish round-trip（write →
  seal → readback checksum → publish → reopen checksum）・
  `CreateNew` 競合→`RecoveryHandle` cleanup・途中 `StorageFull`
  fault→abort・post-seal temp delete→recovery を逐次検証する
  （`large_staged.rs`）。chunked FNV-1a（std のみ）で payload を
  照合し、temp drive の preflight probe（`set_len`+tail write+
  `sync_data`）で容量を実測する。容量不足は `StorageFull` 相当の
  観測として残り case を `NotRun` で停止し、Pass/Fail に偽装
  しない — skip 理由は stdout と `evidence/rf306/` に記録する。
  通常 `test-io-e2e` lane には載らない。
- 性能 baseline lane（Issue #306 G05）: criterion bench
  `benches/io_composition.rs` が staged write/seal/publish/
  readback と TAR→`gzip`/`zstd`→`StagedFile` 合成の代表経路を
  8 MiB 入力で計測する。Format/入力サイズ/Profile/閾値
  （同一環境 ±25% 目安・`peak_alloc` 1 GiB 上限）は bench 冒頭に
  事前定義し、`mise run bench`（`cargo bench --workspace`）で
  実走する。baseline 実測値と環境 metadata は
  `evidence/rf306/large-bench-report.md` に記録する。

## 依存方向

葉側 consumer。dev-dependencies で `ruviform-io-file`・
`ruviform-io-structured`・`ruviform-io-compression`・
`ruviform-io-archive`・`ruviform-io-wav`・`ruviform-io-symphonia`・
`ruviform-media-io`・`ruviform-media-core`・`ruviform-audio-core`
を参照する。G05 bench のため `criterion 0.8.2`
（`default-features = false`、dev-dep 限定）も dev-dependencies
に加える。production クレートは本クレートに依存しない
（`ruviform-ui-test` と同型の opt-in 試験クレート）。

## RT 制約

非 RT 専用。全試験が blocking FS/Parser/Compression I/O を行う。

## 検証

```sh
mise run test-io-e2e          # cargo test -p ruviform-io-e2e
mise run test-io-e2e-corpus   # opt-in: bounded corpus replay（G08、#[ignore] 試験）
mise run test ruviform-io-e2e --test large_staged -- --ignored --nocapture
                           # opt-in: 実 4 GiB 境界 lane（G01、#[ignore] 試験。
                           #  temp drive に ~4.3 GiB の空きが必要）
mise run bench               # workspace 全 bench（G05 io_composition を含む）
mise run test-io-msrv  # MSRV 1.89 toolchain で IO 系 6 crate を test
                       # （io-file/io-archive/io-structured/io-compression/
                       #  io-wav/io-e2e。Issue #306。io-e2e の dev-deps 経由で
                       #  structured 全 format・compression gzip,zstd が
                       #  feature unify され、deflate は別コマンドで補完）
```

主張境界: 実施した OS/FS・toolchain のみを実測として記録する。
実測範囲と NotRun/Unsupported の一覧・#219/#294 への引き渡し
条件は `evidence/rf306/fault-fuzz-report.md`（合成試験）と
`evidence/rf306/fault-seam-fuzz-report.md`（G07 seam 判定・
G08 corpus replay）および `evidence/rf305/wav-golden-fault-report.md`
（WAV golden/fault fixture）を正本とする。adapter 系 fault matrix
（Applied/NotApplied/Unknown・DirSync 後失敗・Indeterminate・Temp
残留）は `ruviform-io-file` の `#[cfg(test)]` MockFileOps
ユニット試験と `fault_seam.rs` の opt-in 契約試験が正本。
実 4 GiB 境界（G01）と性能 baseline（G05）の実測値は
`evidence/rf306/large-bench-report.md` を正本とする。実ファイル
WAV 4 GiB variant・macOS/Linux 実 OS・Network/Cloud FS・
consumer 固有の性能閾値は未実測のまま（NotRun/Unsupported —
WAV RIFF 4 GiB 境界自体は `wav_golden.rs` の discard sink 検証が
正本）。
