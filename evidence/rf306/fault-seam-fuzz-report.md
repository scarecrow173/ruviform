# RF306 G07/G08 実行証拠レポート — fault seam 判定・契約試験 + bounded corpus replay

対象: Issue #306（IO-IMP-12）残件のうち headless に実装可能な
G07（publish fault 分類契約の決定的検証）と G08（bounded・再現
可能な corpus-replay 入口）スライス。

作業 worktree: `C:\Users\akiyoshi\orca\workspaces\ruviform\rf306-fault-fuzz`
branch: `scarecrow173/rf306-fault-fuzz`
base: `origin/main` = `365c74ac4b8e696044fb9bae386dfd03f86ecf32`
（branch 起点。push 前に `git fetch origin main && git merge
origin/main` を実施 — 結果は PR 本文に記録）
環境: Windows 11 x64 (MINGW64_NT-10.0-26300) / rustc 1.99.1
(58cf8f9c3 2026-10-13) / cargo 1.99.1 / NTFS (TEMP 配下の実 FS)
実行日時: 2026-11-13 (UTC)

## G07 seam 判定（技術調査の結果）

`Indeterminate`・publish 後 DirSync 失敗・`TempState::Unknown`
は実 OS で**正直に誘発不能**である（前 slice
`evidence/rf306/fault-fuzz-report.md` の Fault 分類表どおり）。
一方でこれらは `StagedFile` publish pipeline の中核契約であり、
`ruviform-io-file` の `#[cfg(test)]` `MockFileOps` が既に全 stage
（validate/sync_file/prepare/publish/sync_directory/cleanup/
inspect/readback）を script 可能な形で保有していた。

判定: **test-only・opt-in の fault 注入 seam を `ruviform-io-file`
に設け、安全に実行可能**。根拠:

- `PlatformFileOps` は private trait であり、全 OS 操作がこの境界を
  通る。mock は同 trait の in-memory 実装であり、pipeline 本体
  （`publish_pipeline`/`StageOwner`/`RecoveryHandle`）は無変更で
  再利用できる。
- seam の漏洩リスクは feature flag + `#[doc(hidden)]` + facade が
  std 型のみを公開 signature に出すことで封じ込めた
  （`PlatformFileOps`/`MockFileOps`/`MockScript`/`StageOwner`/
  `RecoveryInfo` は `pub(crate)` または private のまま）。
- `StagedFile::with_ops` は `pub(crate)` のまま
  `cfg(any(test, feature = "fault-injection"))` に拡張しただけで、
  公開 API signature・既定 feature build・RT 面は不変
  （feature 無しの check/lint/test が全て pass — 下表）。

したがって「実 OS で NotRun/Unsupported と正直に記録されていた
class」を、**公開 API のみを消費する決定的契約試験**として
`ruviform-io-e2e/tests/fault_seam.rs` に整備した。実 OS での
再現は引き続き OS capability 外であり、`Unsupported` の主張は
変えない（mock-oracle は分類契約の検証であり、実 FS の挙動証明
ではない）。

## 追加・変更したファイル

| ファイル | 変更 |
| --- | --- |
| `crates/ruviform-io-file/Cargo.toml` | `fault-injection = []` feature 追加（非 default・additive） |
| `crates/ruviform-io-file/src/lib.rs` | `mod mock` → `#[cfg(any(test, feature = "fault-injection"))] #[doc(hidden)] pub mod mock` |
| `crates/ruviform-io-file/src/mock.rs` | facade 追加: `PublishReport`/`TempReport`/`PublishScript`/`FaultScript`/`staged_file`/`MockFs`（全て std 型 + 公開 result 型のみ）。内部 `MockFileOps`/`MockScript`/`MockPublish` は `pub(crate)` のまま再利用。`MockFs` の field は `pub`（観測 evidence 用） |
| `crates/ruviform-io-file/src/staged.rs` | `StagedFile::with_ops` の cfg を `any(test, feature = "fault-injection")` に拡張（`pub(crate)` 不変） |
| `crates/ruviform-io-e2e/Cargo.toml` | dev-dep `ruviform-io-file` に `features = ["fault-injection"]`（test target のみ影響） |
| `crates/ruviform-io-e2e/tests/fault_seam.rs` | 新規: G07 契約試験 8 件（公開 API のみ消費） |
| `crates/ruviform-io-e2e/tests/corpus_replay.rs` | 新規: G08 bounded corpus replay（`#[ignore]`・opt-in） |
| `crates/ruviform-io-e2e/src/lib.rs` / `README.md` | 新 test binary・opt-in lane・証拠正本の記述更新 |
| `mise.toml` | `[tasks.test-io-e2e-corpus]` 追加（opt-in のみ。既存 task 無変更） |

## G07 契約試験（8 件・全て実走 pass）

`cargo test -p ruviform-io-e2e --test fault_seam`（`mise run
test-io-e2e` 内でも実行）:

| Case | 検証した契約 | 結果 |
| --- | --- | --- |
| `indeterminate_publish_retains_temp_and_recovers_explicitly` | `ApplyOutcome::Unknown` + 物理適用 → `PublishEffect::Indeterminate`・`PublishPhase::Publish`・temp 自動削除なし・`RecoveryHandle` の inspect→cleanup で `Absent` | Pass |
| `directory_sync_failure_after_publish_reports_published` | publish `Applied` 後の `sync_directory` 失敗 → `Published`（`NotPublished` に偽装しない）+ `DirectorySync` phase + `DurabilityEvidence::FileSynced`・call log が `sync_directory` で停止 | Pass |
| `unknown_cleanup_disposition_surfaces_temp_state_unknown` | publish 後 cleanup が `TempDisposition::Unknown` → receipt `TempState::Unknown`・handle inspect も `Unknown` | Pass |
| `abort_with_unknown_cleanup_reports_unknown_temp_state` | `abort` の cleanup `Unknown` → `CleanupFailure` info `TempState::Unknown`・retry で `Absent` | Pass |
| `validate_fault_is_not_published` | validate `Unsupported` → `NotPublished` + `Validate` phase + `PublishFailureKind::Unsupported` + `DurabilityEvidence::None` | Pass |
| `file_sync_fault_is_not_published` | sync_file `PermissionDenied` → `NotPublished` + `FileSync` phase + `Io` kind | Pass |
| `close_fault_is_not_published_with_file_synced_evidence` | prepare(close) 失敗 → `NotPublished` + `Close` phase + 実績 `FileSynced` evidence を正直に報告 | Pass |
| `destination_conflict_is_not_published` | publish `NotApplied` + `AlreadyExists` → `NotPublished` + `DestinationExists` kind・dest 非接触 | Pass |

全 assert は公開 result 型（`PublishReceipt`/`PublishFailure`/
`CleanupReceipt`/`RecoveryHandle`）と `MockFs` 観測バッファのみに
対して行い、`PlatformFileOps` 境界自体は消費しない。

## G08 bounded corpus replay（実走記録）

`mise run test-io-e2e-corpus`（= `cargo test -p
ruviform-io-e2e --test corpus_replay -- --ignored --nocapture`）:

- seed: `0x52463346555a5a45`（splitmix64・形式別に name_seed で派生）
- `CASES_PER_FORMAT = 48`、9 形式 = **432 cases**
- `INPUT_CAP = 262144`、`DECODED_CAP = 1048576`、`DEADLINE = 120s`
- 変異: byte flip / truncation / PRNG 拡張 / byte splice（各 case で
  決定的に 1 種を選択）。writer 正規 bytes を種に使用
  （XML のみ read-only API のため固定 canonical document）。
- panic は `catch_unwind` で計数し **0 でなければ fail**
  （panic = 欠陥であり reject ではない）。deadline 超過は
  fail-loud。seed 自体が reader に受理される sanity anchor 付き。

実測出力（2026-11-13・Windows 11 x64・debug profile）:

```
corpus-replay json   cases=48 accepted=11 rejected=37 panicked=0 kinds={"Syntax": 27, "TrailingData": 10}
corpus-replay yaml   cases=48 accepted=28 rejected=20 panicked=0 kinds={"Io": 10, "Syntax": 10}
corpus-replay toml   cases=48 accepted=13 rejected=35 panicked=0 kinds={"InvalidEncoding": 10, "Syntax": 25}
corpus-replay csv    cases=48 accepted=13 rejected=35 panicked=0 kinds={"DataModel": 32, "InvalidEncoding": 3}
corpus-replay xml    cases=48 accepted=14 rejected=34 panicked=0 kinds={"InvalidEncoding": 12, "Syntax": 22}
corpus-replay gzip   cases=48 accepted=1  rejected=47 panicked=0 kinds={"io:InvalidData": 28, "io:UnexpectedEof": 19}
corpus-replay zstd   cases=48 accepted=20 rejected=28 panicked=0 kinds={"io:InvalidData": 12, "io:UnexpectedEof": 16}
corpus-replay zip    cases=48 accepted=7  rejected=41 panicked=0 kinds={"ChecksumMismatch": 1, "Inconsistent": 5, "LimitExceeded": 2, "Malformed": 3, "Truncated": 28, "Unsupported": 2}
corpus-replay tar    cases=48 accepted=14 rejected=34 panicked=0 kinds={"ChecksumMismatch": 9, "Malformed": 18, "Truncated": 7}
corpus-replay done: seed=0x52463346555a5a45 cases/format=48 input_cap=262144 decoded_cap=1048576 deadline=120s elapsed=22.5091ms
test corpus_replay_bounded ... ok
```

形式別除外（理由付き・Pass 繰り上げなし）:

| 対象 | 区分 | 理由 |
| --- | --- | --- |
| deflate | NotRun | io-e2e の compression feature 集合に非含有。io-compression 自身の feature matrix lane が正本 |
| WAV | NotRun | Issue #305 の golden/fault lane が正本（`wav_golden`/`wav_fault`/`wav_staged`） |
| secure extraction・`StagedFile` publish FS 経路 | NotRun | `negative_fuzz`/`fault_boundary`/`fault_seam` が正本（FS 副作用を持つ経路は replay 対象外） |
| 持続・coverage-guided fuzz・minimized reproducer corpus | Deferred | 外部 gate。長時間 fuzz は本タスク外（Issue 明記） |
| macOS/Linux 実機・Network/Cloud FS | NotRun/Unsupported | 実環境なし（前 slice と同じ） |

## 実施した検証（全て実走・実値）

canonical = `mise.toml` タスク。OS: Windows 11 x64 /
rustc 1.99.1 / cargo 1.99.1 / debug profile（test profile）。

| 検証 | コマンド | exit | 結果 |
| --- | --- | --- | --- |
| fmt | `mise run fmt-check` | 0 | Pass |
| io-file check | `mise run check-io-file` | 0 | Pass（feature 無し = 既定 build 不変） |
| io-file lint | `mise run lint-io-file` | 0 | Pass（`clippy -D warnings`。`mock.rs` の `assigning_clones` を local clone で解消） |
| io-file test | `mise run test-io-file` | 0 | Pass（unit + doc test 全 pass） |
| io-e2e check | `mise run check-io-e2e` | 0 | Pass |
| io-e2e lint | `mise run lint-io-e2e` | 0 | Pass |
| io-e2e test | `mise run test-io-e2e` | 0 | Pass（全 test binary pass。`corpus_replay` は `#[ignore]` で通常 lane 非実行を確認） |
| corpus opt-in | `mise run test-io-e2e-corpus` | 0 | Pass（432 cases・panicked=0・elapsed≈22.5ms） |
| feature 補助 | `cargo check -p ruviform-io-file --features fault-injection --all-targets` | 0 | Pass（補助証拠。canonical lane は上記） |

波及判定: `ruviform-io-file` の直接依存先は io-archive /
io-wav / io-structured(optional) / ruviform-audio / io-e2e。
公開 API・既定 feature build は不変で、feature は io-e2e
dev-dep 経由でのみ有効化される。`check/lint/test-io-e2e` が
fault-injection 有効の io-file を含む下流一式（io-archive・
io-compression・io-structured・io-wav・media-io・io-symphonia）
を compile+test 済みのため、io-structured/io-compression の
専用 lane 追加実行は省略した（POLICIES §5 の比例検証）。

## 発見・修正した問題

1. `clippy::assigning_clones`（`mock.rs` publish の
   `fs.dest = fs.temp.clone()`）— newer clippy で検出。
   提案の `clone_from` は `MutexGuard` 経由の field 借用が
   E0502 になるため、local へ clone してから代入する形に修正
   （`#[allow]` 不使用・挙動不変）。

## 正直さの確認（NotRun/Unsupported を Pass に繰り上げない）

- G07 の 8 契約は **mock-oracle による分類契約の検証**であり、
  実 OS での `Indeterminate`/DirSync 後失敗/`TempState::Unknown`
  の再現を主張しない。当該 class の実 FS 再現は引き続き
  **Unsupported**（OS capability 外）。
- G08 は bounded replay の入口と 1 回の実行記録であり、
  sustained fuzz・coverage・minimized corpus の品質を主張しない
  （**Deferred**）。
- consumer 実測（AudioLens/Synvelis）は本 slice 対象外
  （**NotRun**、#219/#294 gate）。
- `test-io-msrv` はユーザー指定により通常開発 lane では未実行
  （**NotRun**。merge 後 CI の証拠で確認）。
