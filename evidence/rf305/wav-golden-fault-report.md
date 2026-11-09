# RF305 実行証拠レポート — WAV→StagedFile Byte Golden・Fault fixture

対象: Issue #305（WAV→`StagedFile` 統合）の io-e2e fixture 残行
slice。`crates/ruviform-io-e2e/tests/` のみを拡張し、本番コード・
公開 API・`ruviform-io-wav` の src は一切変更していない
（dev-dependencies の追加もなし）。

作業 worktree: `C:\Users\akiyoshi\orca\workspaces\ruviform\rf305-wav`
branch: `feature/305-wav-golden-fault`
base: `origin/main` = `b0d06328`（branch 起点）。push 前に
`git fetch origin main && git merge origin/main` を実施し
`cf1d2c8b`（PR #515・ui fix + docs のみ、io-e2e 非関連）を
merge 済み。merge 後に全検証 lane を再実行した（下表は merge 後の
実値）。
環境: Windows 11 x64 (MINGW64_NT-10.0-26300) / rustc 1.99.0
(b940084d7 2026-09-28) / cargo 1.99.0 / NTFS (TEMP 配下の実 FS)
実行日時: 2026-11-09 (UTC)

## 追加した試験（17 件・公開 API のみ消費）

| ファイル | 件数 | 内容 |
| --- | --- | --- |
| `tests/wav_golden.rs` | 8 | publish 済み destination bytes を RIFF/WAVE 仕様から独立に組み立てた期待値と byte-exact 照合。ZeroFrames（44B WAVEFORMATEX）・5.1 WAVEFORMATEXTENSIBLE（68B・tag 0xFFFE・cbSize 22・mask 0x3F・PCM subtype GUID）・S16 stereo payload・U8/S16/S24/S32/F32 encoder edge sample（NaN→0・±Inf/clamp・`.5` LSB round-half-away・F32 verbatim）・odd payload pad byte・`f32→wire` policy literal oracle・classic RIFF 4 GiB 境界（正側 finalize 成功 `data_bytes=4_294_967_256`・負側 write/finalize とも `InvalidData`・receipt なし）を discard sink で検証（実 4 GiB ファイルは生成しない） |
| `tests/wav_fault.rs` | 9 | `&mut StagedFile` 上の公開 API のみの `MediaSink` wrapper（`FlushFail`/`DiskFull`）で finalize flush 失敗・finalize backpatch write 失敗・packet write 途中 disk-full を `ErrorKind::Io`・receipt なし・publish なしで検証。seal 後 abort（cancel）・muxer `Drop` best-effort finalize が completion 証拠でないこと（receipt 不在のまま bytes は decode 可能になる罠を pin）・publish outcome classifier（`Published`/`NotPublished`/not-provably-unpublished、`#[non_exhaustive]` wildcard 込み）・CreateNew 競合 `DestinationExists`/`PublishPhase::Publish`/`NotPublished` + 先勝ち bytes 保全 + recovery handle cleanup・Replace publish・`StagedFile::create` 失敗の型付き `FileErrorKind::Io` と residue なし |
| `tests/common/wav.rs` | — | 共有 fixture helper。`expected_wav_bytes`（RIFF/WAVE 仕様からの独立 file 構築）・`expected_wire_sample`/`expected_edge_payload`/`edge_samples`（変換 policy の独立 oracle）・`s16_payload`/`packet`/`spec_for`/`s16_stereo_spec`/`independent_decode`（symphonia demux oracle） |
| `tests/common/mod.rs` | — | `pub mod wav;` 追加のみ |

既存 `wav_staged.rs`（3 件・finalize→borrow release→seal→
independent decode→publish→reopen・truncated/second-finalize
reject）は変更なし。`ruviform-io-e2e` は 60 → **77 件**。

## 実施した検証（全て実走・実値・merge 後 HEAD `22a6324` 系で再実行）

| 検証 | コマンド | 結果 | 備考 |
| --- | --- | --- | --- |
| fmt | `mise run fmt-check` | Pass (exit 0) | `cargo fmt --all -- --check` |
| crate check | `mise run check-io-e2e` | Pass (exit 0) | `cargo check -p ruviform-io-e2e --all-targets` |
| crate lint | `mise run lint-io-e2e` | Pass (exit 0) | `cargo clippy -p ruviform-io-e2e --all-targets -- -D warnings` |
| crate test | `mise run test-io-e2e` | Pass (exit 0) | 77 tests 全 pass（0 failed / 0 ignored） |
| workspace check | `mise run check` | Pass (exit 0) | `cargo check --workspace --all-targets` |
| workspace lint | `mise run lint` | Pass (exit 0) | `cargo clippy --workspace --all-targets -- -D warnings` |
| workspace test | `mise run test` | Pass (exit 0) | `cargo test --workspace` 全 pass |
| RT lane | `mise run test-rt` | Pass (exit 0) | `ruviform-io-e2e` は非 RT のため対象外（crate README に明記済み） |

補助証拠（canonical = 上記 mise タスク）: `cargo check`/
`cargo clippy`/`cargo test -p ruviform-io-e2e --test wav_golden
--test wav_fault`・`cargo tree`・`grep` はデバッグ/証拠収集
用途のみで、合否判定には使用していない。

## OS/FS 実測範囲

実行環境: Windows 11 x64 / NTFS / rustc 1.99.0 のみ。

| OS/FS | 状態 | 根拠 |
| --- | --- | --- |
| Windows 11 x64 / NTFS | **Pass（実測）** | 全 17 件実 FS 上で pass |
| macOS 実機 | **NotRun** | 実機環境なし |
| Linux 実機 | **NotRun** | 実機環境なし |
| Network/Cloud FS | **Unsupported** | 実測環境なし。durability/atomicity 保証は FS 依存のため主張しない |
| 性能特性 | **NotRun** | 本 slice の対象外 |

## Fault/publish 分類の正直さ

| fault/effect class | 本 slice での扱い | 根拠 |
| --- | --- | --- |
| finalize flush `Io` / backpatch write `Io` / packet write `Io`（disk-full 注入） | **実測 Pass** | `wav_fault.rs` の `FlushFail`/`DiskFull` wrapper（`&mut StagedFile` への公開 API のみの `MediaSink`） |
| `PublishEffect::NotPublished`（CreateNew 競合） | **実測 Pass** | `publish_conflict_classifies_as_not_published` で `DestinationExists`/`Publish`/`NotPublished` + 先勝ち bytes 保全を実測 |
| publish 成功（CreateNew/Replace） | **実測 Pass** | `Outcome::Published` + `DurabilityEvidence::FileSynced` + destination bytes 検証 |
| `PublishEffect::Published` / `Indeterminate` の**失敗** effect | **NotRun** | constructor は crate-private で公開 API から誘発不能。`ruviform-io-file` の `#[cfg(test)]` mock 試験が正本。classifier は両者を not-provably-unpublished bucket に落とすことを型で pin |
| Directory-sync 後失敗（`PublishPhase::DirectorySync`） | **NotRun** | 同上。実 OS で正直に誘発不能 |
| `TempState::Unknown` | **NotRun** | 同上 |
| 実 4 GiB ファイルの生成・publish | **Unsupported** | discard sink（`DevNull`）で境界のみ検証。実ファイルは生成しない（Issue の「sink が受理/破棄しつつ累積 byte を追跡」の指定どおり） |
| trybuild 等の旧 API 非存在コンパイル時証拠 | **docs 証拠** | 新依存追加禁止のため trybuild 未導入。下記 grep 証拠で代替 |

## 旧 API 非存在の証拠（補助・canonical = mise ではない）

- `grep -rn "StagedOutput" --include="*.rs" crates/` → **0 件**
  （公開 API・全 crate ソースに残存なし）。docs/ 配下の残存は
  歴史文書（adr/designs/discovery/impl/plans/README）のみで、
  タスク仕様が削除対象外と明記する範囲。
- `PublishOptions` の全 `.rs` 参照は `ruviform_io_file::
  PublishOptions`（新 API）のみ。旧 media-io `PublishOptions`/
  `StagedOutput` の import は存在しない。
- `cargo tree -p ruviform-media-io --depth 1` → 依存は
  `ruviform-audio-core`・`ruviform-media-core`・
  `ruviform-video-core` の **Domain Core3 のみ**（DAG 維持）。
- `Cargo.toml`/`Cargo.lock` の変更なし（新規 dev-dependency なし）。

## 発見・修正した試験側の問題（本番コードは無変更）

1. `with_borrowed_demuxer` 呼び出しブロック末尾の `;` で戻り値が
   `()` になっていた — セミコロン除去で修正（wav_staged.rs と
   同じ式形に揃えた）。
2. clippy pedantic 指摘（`double_must_use`・`doc_markdown`・
   `assert_is_empty`）を fixture 側で修正。本番コードへの
   変更・lint 抑制は一切なし。
3. `ruviform-io-wav` の真の defect は**発見されず**、src 変更は
   行っていない（byte golden が初回実行で全件 pass）。

## #219/#294 への引き渡し（Consumer 実測は本 slice 対象外）

AudioLens（#143）・Synvelis（#84）の consumer Revision/lockfile
依存更新連携・#219/#288 での個別 Gate PASS は consumer 側の
別 Gate であり、本 slice では **NotRun**。以下を判断材料として
引き渡す。consumer 側 repository は変更していない。

- **Test Command（推奨）**: `mise run test-io-e2e`
  （= `cargo test -p ruviform-io-e2e`）。consumer 側 CI で
  ruviform を path/git 依存として取り込む場合は同等の
  `cargo test -p ruviform-io-e2e` を実行する。
- **Fixtures**: `wav_golden.rs`/`wav_fault.rs` は公開 API のみを
  消費し、`tests/common/wav.rs` の期待値は RIFF/WAVE 仕様と
  `f32→wire` 変換 policy から独立に構築される（`ruviform-io-wav`
  の private helper に依存しない）。consumer が WAV 出力経路を
  変更した場合、byte golden の期待値は仕様側を更新すること
  （実装に合わせて oracle を追従させない）。
- **判断基準**: consumer 側で IO 経路を変更した場合、
  `test-io-e2e` 全 77 件の pass を合格条件とする。4 GiB 境界・
  fault 分類は Windows NTFS のみ実測であり、consumer が
  macOS/Linux・Network/Cloud FS を対象にする場合は当該環境での
  再実測が必要（本証拠を根拠にそれら環境の Supported を
  主張しない）。
- **片側 pass の扱い**: AudioLens と Synvelis のいずれか一方
  のみ pass した状態を両 Consumer 合格とはみなさない
  （Issue 契約どおり）。

## 対象外として記録する残件

- AudioLens #143・Synvelis #84 の consumer 実測・候補
  Revision/lockfile 依存更新連携 — **NotRun**（#219/#294 gate）
- 非互換導入順序と rollback の実装 PR 合意 — consumer 移行時
- macOS/Linux 実機 — **NotRun**
- Network/Cloud FS — **Unsupported**
- `Published`/`Indeterminate` 失敗 effect・DirSync 後失敗・
  `TempState::Unknown` — **NotRun**（公開 API から誘発不能。
  `ruviform-io-file` `#[cfg(test)]` mock 試験が正本）
- 実 4 GiB ファイルの生成・publish — **Unsupported**
  （discard sink で境界のみ検証）
- 性能計測 — **NotRun**
