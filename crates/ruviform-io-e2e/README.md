# ruviform-io-e2e

IO-01〜05 の公開 API を層横断で合成する E2E 試験専用クレート
（Issue #306 / IO-IMP-12）。公開 API・本番コードを持たず、
`tests/` 配下の integration test のみを収録する。

## 責務

- `StagedFile → SealedFile → publish → 再 Open` の型状態・実 OS
  保証を、Structured（`write_file`/`read_file`）、TAR→Compression、
  ZIP/TAR→Secure Extraction、WAV `finalize→seal→readback→publish`
  の各合成境界で pin する。
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
  で正直に誘発不能なため fixture 化せず、`ruviform-io-file` の
  `#[cfg(test)]` mock 試験を正本として参照する。
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
  NotRun（`ruviform-io-file` の mock 試験が正本）。

## 依存方向

葉側 consumer。dev-dependencies で `ruviform-io-file`・
`ruviform-io-structured`・`ruviform-io-compression`・
`ruviform-io-archive`・`ruviform-io-wav`・`ruviform-io-symphonia`・
`ruviform-media-io`・`ruviform-media-core`・`ruviform-audio-core`
を参照する。production クレートは本クレートに依存しない
（`ruviform-ui-test` と同型の opt-in 試験クレート）。

## RT 制約

非 RT 専用。全試験が blocking FS/Parser/Compression I/O を行う。

## 検証

```sh
mise run test-io-e2e   # cargo test -p ruviform-io-e2e
mise run test-io-msrv  # MSRV 1.89 toolchain で IO 系 6 crate を test
                       # （io-file/io-archive/io-structured/io-compression/
                       #  io-wav/io-e2e。Issue #306。io-e2e の dev-deps 経由で
                       #  structured 全 format・compression gzip,zstd が
                       #  feature unify され、deflate は別コマンドで補完）
```

主張境界: 実施した OS/FS・toolchain のみを実測として記録する。
実測範囲と NotRun/Unsupported の一覧・#219/#294 への引き渡し
条件は `evidence/rf306/fault-fuzz-report.md`（合成試験）と
`evidence/rf305/wav-golden-fault-report.md`（WAV golden/fault
fixture）を正本とする。adapter 系 fault matrix（Applied/
NotApplied/Unknown・DirSync 後失敗・Indeterminate・Temp 残留）は
`ruviform-io-file` の `#[cfg(test)]` MockFileOps ユニット試験が
正本。macOS/Linux 実 OS・Network/Cloud FS・性能・実 4 GiB
ファイルの publish は未実測（NotRun/Unsupported）。
