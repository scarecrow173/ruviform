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
- scratch 配置: `%TEMP%\rf306\rf305-scratch`（`git worktree add
  --detach`、検証後 `git worktree remove` で除去。自 worktree と
  TEMP 外の書込なし）
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

## NotRun / Blocked / Unsupported（正直な記録）

- `mise run test`（`cargo test --workspace`）: **NotRun** — 全
  crate の非回帰は workspace `check`/`lint`（両 Pass）と新 lane
  `test-io-e2e`（31/31 Pass）で担保。workspace test は他 crate
  単体試験の再実行であり、本変更は leaf 試験クレート追加のみで
  既存コード無変更のため skip。CI の test-ci lane が最終担保。
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
