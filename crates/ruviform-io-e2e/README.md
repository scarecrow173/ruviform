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
```

主張境界: 実施した OS/FS・toolchain のみを実測として記録する。
adapter 系 fault matrix（Applied/NotApplied/Unknown・DirSync 後
失敗・Indeterminate・Temp 残留）は `ruviform-io-file` の
`#[cfg(test)]` MockFileOps ユニット試験が正本。macOS/Linux 実
OS・Network/Cloud FS・性能は未実測（NotRun）。
