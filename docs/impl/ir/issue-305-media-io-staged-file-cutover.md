---
id: "IMPL-0Rj3kVwF2mN8qXbCeLpY4d"
type: "impl"
status: "completed"
title: "Issue 305 media-io 旧 StagedOutput 完全削除・StagedFile 移行"
created: "2026-11-07"
updated: "2026-11-07"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/305"
    - "https://github.com/scarecrow173/ruviform/issues/287"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-305-media-io-staged-file-cutover.md"
      - type: "file"
        path: "docs/designs/issue-305-media-io-staged-file-cutover.md"
      - type: "file"
        path: "docs/plans/issue-305-media-io-staged-file-cutover.md"
      - type: "file"
        path: "docs/test-specs/issue-305-media-io-staged-file-cutover.md"
      - type: "file"
        path: "docs/tasks/issue-305-t1-staged-file-cutover.md"
      - type: "file"
        path: "docs/impl/ir/issue-305-media-io-staged-file-cutover.md"
    modified:
      - type: "file"
        path: "crates/ruviform-media-io/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-media-io/src/sink.rs"
      - type: "file"
        path: "crates/ruviform-audio/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-audio/src/render.rs"
      - type: "file"
        path: "crates/ruviform-audio/tests/render.rs"
      - type: "file"
        path: "crates/ruviform-io-wav/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-io-wav/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-io-wav/src/muxer.rs"
      - type: "file"
        path: "crates/ruviform-io-wav/README.md"
      - type: "file"
        path: "crates/ruviform-io-wav/tests/contract.rs"
      - type: "file"
        path: "Cargo.lock"
    deleted:
      - type: "file"
        path: "crates/ruviform-media-io/src/staged.rs"
    renamed: []
    moved: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-305-media-io-staged-file-cutover.md"
    - "docs/designs/issue-305-media-io-staged-file-cutover.md"
    - "docs/plans/issue-305-media-io-staged-file-cutover.md"
    - "docs/impl/ir/issue-295-ruviform-io-file-staged-publish-contract.md"
    - "docs/impl/ir/issue-296-ruviform-io-file-os-backend.md"
    - "https://github.com/scarecrow173/ruviform/issues/295"
    - "https://github.com/scarecrow173/ruviform/issues/296"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-305-media-io-staged-file-cutover.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue 305 media-io 旧 StagedOutput 完全削除・StagedFile 移行

## Summary

Issue #287 の設計に従い、`crates/ruviform-media-io/src/staged.rs`
（旧 `StagedOutput` / 旧 `PublishOptions`、371 行）を互換層なしで
完全削除し、全 consumer を `ruviform-io-file` の type-state API
（`StagedFile` → `SealedFile` → `publish`）へ移行した。
`ruviform-audio` のオフラインレンダードライバは muxer を
`&mut StagedFile` 借用で構築する形に再構成し
（`WavMuxer<&mut StagedFile>` が `MediaSink` blanket impl 経由で成立）、
`File::try_clone` による raw handle 経路を撤去した。
`RenderError::Publish` は `MediaError` ではなく `PublishFailure`
全体を保持し、`PublishEffect` / phase / `RecoveryHandle` が
失われない。公開 API から旧 staged 型はゼロ。
`MediaSource`/`MediaSink`/Encoder/Muxer/Decoder/Demuxer の公開契約と
media-io の依存 DAG（domain core 3 クレートのみ）は変更していない。

## Context

- 実施タスク: [T1](../../tasks/issue-305-t1-staged-file-cutover.md)
- 上流: [plan](../../plans/issue-305-media-io-staged-file-cutover.md)、
  [design](../../designs/issue-305-media-io-staged-file-cutover.md)、
  [spec](../../specs/issue-305-media-io-staged-file-cutover.md)、
  [test-spec](../../test-specs/issue-305-media-io-staged-file-cutover.md)、
  設計権威 Issue #287、親 Issue #282
- 前提: IO-IMP-01/02（#295/#296）の `StagedFile` type-state・OS atomic
  publish・durability・readback が main にマージ済み
- 境界: WAV backend 実装は #191 の責務であり本 Issue では保存経路
  API 移行のみ。Consumer repo（AudioLens/Synvelis）移行は別 Issue、
  cross-repo gate は #219/#288。`ruviform-io-file` への新契約追加・
  `ruviform-media-io` → `ruviform-io-file` の依存追加は行わない。

## Implementation

### Changes

- `crates/ruviform-media-io/src/staged.rs`（削除）— 旧 `StagedOutput`/
  旧 `PublishOptions` 実装一式。`lib.rs` の `mod staged` と
  `pub use staged::{PublishOptions, StagedOutput}` を除去。`sink.rs`
  の durability 説明は `ruviform-io-file` の
  `StagedFile`/`SealedFile`/`PublishFailure` を指す記述に更新。
- `crates/ruviform-audio/Cargo.toml` — `ruviform-io-file` path 依存を
  追加（render の staged publish 消費者として）。
- `crates/ruviform-audio/src/render.rs`:
  - `RenderOutput.staged` を `StagedFile` に、`publish` フックを
    `FnOnce(SealedFile, PublishOptions) -> Result<PublishReceipt,
    PublishFailure>` に変更（`SealedFile::publish` がそのまま fn 値に
    なる）。`verify` フックは `FnOnce(&mut SealedFile, &OutputFacts)` —
    sealed object の `open_readback` 経由でのみ検証でき、publish を
    bypass できない境界。
  - `make_muxer` 束縛を `for<'a> FnOnce(&'a mut StagedFile) ->
    Result<Box<dyn Muxer + 'a>, MediaError>` に変更。
    `run_pipeline` 内で muxer は `&mut staged` を借用し、
    `drop(muxer)` で借用解除してから `staged.seal()` する。
  - 出力 tail の順序を `finalize 成功 → muxer drop → seal →
    readback verify（任意）→ publish` に再構成。
  - publish 失敗の写像: `PublishFailureKind::DestinationExists` →
    `RenderError::DestinationExists`（recovery handle の temp を
    cleanup）、`PublishEffect::NotPublished` → recovery handle で
    temp cleanup 後 `RenderError::Publish`、それ以外（`Published`/
    `Indeterminate`）は handle を failure 内に保持したまま
    `RenderError::Publish` — 決して自動削除しない。
  - `RenderReceipt` に `publish: PublishReceipt` フィールド追加
    （achieved durability evidence と TempState を保持）。
    `PublishReceipt` が `Clone` でないため `RenderReceipt` の
    `Clone` derive を除去。
  - preflight: `PublishMode::CreateNew` かつ `dest_path.exists()` で
    早期 `DestinationExists`。原子性の根拠はあくまで publish 側の
    atomic no-replace primitive であり、これはレンダリング省略の
    preflight に過ぎない。
- `crates/ruviform-audio/tests/render.rs`:
  - `StagedOutput::create` → `StagedFile::create`、`PublishOptions::
    no_overwrite()` → `const NO_OVERWRITE =
    PublishOptions::new(PublishMode::CreateNew,
    Durability::FileAndDirectorySync)`（旧 no_overwrite の
    拒否+完全 durability 契約を維持）。
  - `FailSink` は `&mut StagedFile` を借用する `FailSink<'a>` に
    変更（注入失敗までの bytes は引き続き staged temp に落ちる）。
  - `verify_wav` は `&mut SealedFile` を受け取り `open_readback` で
    sealed bytes を読み、`Cursor` 経由で symphonia 独立 decode
    （`open_demuxer` が `'static MediaSource` を要求するため）。
    frames/channels/rate/PCM 比較は全て維持。
  - publish 失敗注入 (of06-d) は mock 不可（io-file mock は private）
    のため、dest に既存 directory を置き `PublishMode::Replace` で
    rename 失敗を誘発する「実」失敗に変更。`RenderError::Publish`
    + `PublishEffect::NotPublished` を検査し temp cleanup を確認。
  - `wav_muxer`/`fail_muxer`/`wav_muxer_ctor`/`fail_muxer_ctor`
    helper を追加 — 裸 closure の `as _` は trait object を
    `'static` に潰すため、`for<'a>` 署名を持つ named fn 経由で
    HRTB 一般化を保証。
- `crates/ruviform-io-wav/Cargo.toml` — dev-dependency として
  `ruviform-io-file` を追加（backend 本体は `MediaSink` generic の
  まま、io-file 非依存）。
- `crates/ruviform-io-wav/tests/contract.rs` — staged publish 系
  テストを `StagedFile` API に移行。
  `staged_publish_produces_reopenable_wav` は必須順序
  （`finalize` → borrow 解除 → `seal` → `open_readback` で RIFF
  magic 検証 → `publish` → 公開後 reopen）を実証する E2E に強化。
  conflict テストは `PublishFailureKind::DestinationExists` +
  `PublishEffect::NotPublished` + recovery handle cleanup を検査。
- `crates/ruviform-io-wav/{src/lib.rs,src/muxer.rs,README.md}` —
  durability 参照を `ruviform-media-io::StagedOutput` から
  `ruviform-io-file` の `SealedFile::publish` に更新。
- `Cargo.lock` — 上記 2 辺（ruviform-audio→io-file、io-wav dev-dep）
  のみを cargo 再解決で追加。

### Non-goals / 境界確認

- `ruviform-media-io` → `ruviform-io-file` の依存は追加していない
  （DAG 維持: domain core 3 のみ）。
- `StagedFile` への raw `&mut File` accessor、Deprecated alias、
  facade は一切追加していない。
- `ruviform-io-file` 自体の API 変更なし（#295/#296 契約の消費のみ）。
- WAV wire format・変換 policy・finalize 契約は無変更。

## Verification

正規入口 `mise run <task>`、`CARGO_BUILD_JOBS=2`。実環境:
Windows 11 x64 / Git Bash、rustc は mise 管理 toolchain。

| task | 結果 |
| --- | --- |
| `mise run fmt` → `mise run fmt-check` | PASS（差分なし） |
| `mise run check`（workspace, all-targets） | PASS |
| `mise run lint`（clippy `-D warnings`） | PASS |
| `mise run test`（workspace + doctests） | PASS（全 crate 0 failed） |

主要 test evidence:

- `of01..of06`（ruviform-audio/tests/render.rs）全 PASS —
  `of05_wav_reopen_verify` が seal→readback→独立 decode→publish の
  順序を実測、`of06_failure_injection`/`of06_staging_policy` が
  cancel/disk-full/flush/publish-failure/dest-exists/verify の
  型付き失敗と temp cleanup・dest-untouched を検査。
- `staged_*`（ruviform-io-wav/tests/contract.rs）全 PASS —
  `staged_publish_produces_reopenable_wav` が必須順序 E2E、
  `staged_no_overwrite_conflict_preserves_destination` が
  `DestinationExists`+`NotPublished`+recovery cleanup、
  abort/drop/replace/disk-full を網羅。
- `ruviform-io-file` 自身の staged/publish/recovery 単体試験群は
  既存のまま全 PASS（本 Issue では変更していない）。
- 旧 API 残存検査: `grep -rn "StagedOutput" crates/` → 0 件。
  旧 `PublishOptions` constructor（`no_overwrite`/`replace`/
  `dest_existed`/`.sink()`/`try_clone`）の使用 → 0 件（docs の
  歴史的 ADR/design/task 記録は対象外）。

### Post-merge re-verification（merge commit `7ead144`）

`origin/main` `e8adcc0` を merge した commit `7ead144` で同一正規入口を
再実行（Windows 11 x64 / Git Bash、mise pin toolchain）。merge は ort で
conflict なし。`Cargo.toml` members・`crates/README.md`・`AGENTS.md`・
`mise.toml` は union 解消の結果 main と byte 一致、`Cargo.lock` は
`ruviform-io-file` 依存追加の +2 行のみ差分（`ruviform-audio`・
`ruviform-io-wav` の Cargo.toml 変更と整合）。

| task | 結果 |
| --- | --- |
| `mise run fmt-check` | PASS |
| `mise run check` | PASS |
| `mise run lint`（clippy `-D warnings`） | PASS |
| `mise run test`（workspace + doctests） | PASS（166 test-result blocks・0 failed） |
| `mise run test-io-e2e`（Issue #306 lane、main 由来） | PASS（31 件: archive_compression_staged 6・extraction 10・staged_lifecycle 6・structured_staged 6・wav_staged 3、0 failed） |

- 旧 API 残存検査（merged head）: `StagedOutput`・`media-io` 内
  `PublishOptions`・`mod staged`/`staged::` → 各 0 件。
- merge 整合性は独立 Validator（read-only subagent）で確認: cross-diff
  （`git diff origin/main HEAD`）が RF305 scope の 18 file のみで、
  main の新規成果（io-e2e / ui-gpui / ui-theme / ui-test /
  poc-native-dock / observability collector / plugin-host
  editor_session / consumer-verify workflow）への diff はゼロ。

### Post-merge re-verification（combined merge `16eacc48` + `ce8bc99b`）

`d68b14d` 上へ `origin/main` `5d48298`（RF401 rooted read capability・
RF402 real VST3 attach・RF403 consumer-verify GUI-CONTRACT lane 含む）と
RF407 候補 `4c67519`（Issue #400 `with_borrowed_demuxer` +
`Demuxer for &mut D` + `SourceReader<'a>`、main 未 merge の
unapproved candidate）を順次 merge した combined head `ce8bc99b` で
同一正規入口を再実行（Windows 11 x64 / Git Bash、mise pin toolchain）。
両 merge とも ort で conflict なし。RF407 実装ファイル
（io-symphonia・`media-io/src/demux.rs`・media-reader 全体）への diff は
ゼロで、唯一の交差は `media-io/src/lib.rs` の `mod staged` 除去
（RF305 scope）。`SourceReader<'a>` の named 型位置は RF407 側で
`<'static>`/`'_>` 注釈済み（workspace 内の未注釈残存ゼロ、追加修正不要）。

| task | 結果 |
| --- | --- |
| `mise run fmt-check` | PASS |
| `mise run check` | PASS（exit 0） |
| `mise run lint`（clippy `-D warnings`） | PASS |
| `mise run test`（workspace + doctests） | PASS（170 test-result blocks・0 failed） |
| `mise run test-io-e2e` | PASS（31 件・0 failed、`wav_staged` 3 件は実 sealed `ReadbackFile` 上の `with_borrowed_demuxer` decode） |
| focused（補助 `mise exec -- cargo test -p ruviform-io-symphonia -p ruviform-media-reader --jobs 2`） | PASS（borrowed 12・borrowed_source 3・prefetch 27・reader 11・`compile_fail` escape doctest 含む、canonical test 内でも同 PASS） |

- 旧 API 残存検査（combined head）: `StagedOutput`・`media-io` 内
  `PublishOptions`・`mod staged`/`staged::` → 各 0 件。
- merge 整合性（cross-diff 検査、実測）:
  `git diff origin/main...HEAD` は RF305+RF400 scope の 41 file のみ。
  main 新規成果（`ruviform-io-file` rooted/vfs・io-archive・
  consumer-verify workflow・plugin-vst3 host・ui-theme/ui-test/
  ui-gpui・poc-native-dock・mise.toml）への diff はゼロ。
  `git diff 4c67519 HEAD -- <RF407 paths>` は `media-io/src/lib.rs` の
  staged 除去のみ。managed index（`docs/*/README.md`）は両側 union で
  issue-305・issue-400・main 由来 entry が共存。`docs/adr/README.md` の
  Rooted Read ADR 行と `docs/discovery/README.md` の DISC-034T9T71
  final 化・DISC-im2uskbe 行は RF407 側の正規再生成が main の index
  欠落を補完する意味不変差分（対応 doc file は main に実在）。
- 独立 Validator（read-only subagent）へ cross-diff 検査を委譲済み。
  判定結果が本ファイルへ追記されるまでは暫定記録。

## Acceptance ledger

| AC | 状態 | 根拠 |
| --- | --- | --- |
| 公開 API から旧 StagedOutput/旧 PublishOptions ゼロ + 新 API コンパイル試験 | PASS | staged.rs 削除 + grep 0 件 + workspace check/test PASS |
| MediaSource/Sink/Backend API・依存 DAG 維持 | PASS | media-io の trait/exports 無変更、io-file 非依存を維持、io-wav は dev-dep のみ |
| WAV 正常/ZeroFrames/PCM/RIFF/4GiB/NaN/Inf/Precision/Byte Golden/Finalize 失敗/Cancel/DiskFull/Published・Indeterminate | 一部 PASS / 一部 NotRun | 既存試験すべて維持・PASS（contract.rs + io-wav 単体試験 + render of05/of06）。`PublishEffect::Published`/`Indeterminate` の render 経路は io-file 自身の mock 単体試験で既検証済み；本 Issue では新規注入経路は未整備（NotRun でなく io-file 側 covered） |
| AudioLens/Synvelis 候補 Revision 連携・#219/#288 gate | BLOCKED（本 worker 範囲外） | consumer repo 編集・cross-repo gate は coordinator/integration 責務。candidate SHA と検証 command を worker_done で報告 |
| 旧 API 維持・SHA 凍結回避 | PASS | 互換層なし・完全撤去 |
| 推奨 E2E 順序（finalize→借用解除→seal→readback 検証→publish→reopen） | PASS | contract.rs E2E + render.rs verify hook で実証 |
| `into_sink`/Drop を完了証拠にしない | PASS | `finalize` 成功のみが receipt；Drop は temp cleanup のみ |
| 安全な Readback 未対応時は Unsupported/Blocked | PASS | `open_readback` が `FileErrorKind::Unsupported` を返す契約（io-file 側試験済み） |

## Review

review-protocol 実施（runtime subagent 並列派遣、両者 read-only）:

- **Reviewer A（cooperative）** — 全 AC をコード根拠で確認。候補:
  doc overclaim（`Err` 時「temp 除去・dest 無変更」が
  `Published`/`Indeterminate` で偽）、cleanup 失敗時の
  `CleanupFailure`+retry handle 破棄、test-spec 要求の独立 decode
  E2E 不足、publish 時 DestinationExists 写像の未試験、io-wav
  README の stale 行、`into_sink`/`Output` doc 不整合。
- **Reviewer B（adversarial）** — 上記に加え verify hook の
  `mem::swap` 差し替えで receipt/publish 先不整合を指摘、
  cancel phase・receipt.publish evidence・temp 残存の未検査を指摘。
  借用/lifetime 健全性・DAG・API 残存・test (d) 到達性・
  cancel 後 seal drop cleanup は clear 確認。
- **Validator/Judge 判定** — 全候補 CONFIRMED として以下で修正:
  1. render.rs の 3 箇所の doc を `Published`/`Indeterminate` を
     含む正直な契約に書換（`Publish` variant は `cleanup:
     Option<Box<CleanupFailure>>` を保持し retry handle を保全）。
  2. verify 前後で `operation_id()` を照合する swap-guard 追加。
  3. `staged_seal_readback_publish_e2e` を readback→独立
     decode→PCM/frame count 検証→publish→byte 一致 reopen の
     完全 E2E に強化（test-spec 準拠）。
  4. `of06_publish_lifecycle` 追加: publish 時
     DestinationExists 写像（実 PublishFailure 経由）と
     `Cancelled{phase:"publish"}` + temp cleanup を試験。
  5. `of05` に `receipt.publish` の `durability()`/`temp_state()`
     evidence assertion 追加。
  6. contract.rs staged 全試験に `assert_no_temps`（`.rvf-tmp-*`
     残存検査）追加。
  7. io-wav README 原子性記述・dev-dep 一覧、`into_sink`/`Output`
     doc、task/design doc の実装 drift を修正。
- 非 block note: `failure.info().temp_state()` は cleanup 成功後も
  スナップショット時点の `Retained` を示す（io-file 契約上の
  snapshot semantics）。verify hook が temp_path を FS 経由で再 open
  して書換える可能性は同一 process 信頼境界の制限として残置。
  media-io lib.rs/README の「no implementor yet」記述は
  本 Issue 範囲外の pre-existing staleness（別 Issue 候補）。
- 判定: **ACCEPT**（blocking 指摘は全て解消、正規検証全 PASS）。

## Remaining / handoff

- Consumer repo（AudioLens #143 / Synvelis #84）の移行・lockfile 更新・
  cross-repo gate は coordinator/integration 側で実施。
- 本 PR は `Refs #305` 相当（全 AC 充足を主張しない）。candidate SHA・
  検証 command を coordinator に報告。
