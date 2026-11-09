---
id: "PLAN-034UwaGC4jBLVC4SdHmkU4"
type: "plan"
status: "approved"
title: "Issue #305 IO E2E WAV Golden/Fault 実装計画"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/305"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-305-io-e2e-wav-golden-fault.md"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-305-io-e2e-wav-golden-fault.md"
    - "docs/specs/issue-305-media-io-staged-file-cutover.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/issue-306-io-e2e-fault-fuzz.md"
    - "docs/plans/issue-305-media-io-staged-file-cutover.md"
  refines:
    - "docs/plans/issue-305-media-io-staged-file-cutover.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-305-io-e2e-wav-golden-fault.md"
    - "docs/designs/issue-305-io-e2e-wav-golden-fault.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-305-io-e2e-wav-golden-fault.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #305 IO E2E WAV Golden/Fault 実装計画 Implementation Plan

## Goal

`ruviform-io-e2e` に WAV→`StagedFile` の byte-exact golden と
fault 分類 fixture を追加し、Issue #305 受入条件の残り行
（Byte Golden・Finalize/Cancel/DiskFull・Published/NotPublished/
Indeterminate 区別）を閉じる。

## Scope

単一実装ストリーム・1 plan・2 task。変更は
`crates/ruviform-io-e2e/tests/`（新規 2 file + 必要に応じた
`wav_staged.rs` 最小拡張）・`crates/ruviform-io-e2e/README.md`・
`src/lib.rs` doc 一覧・`evidence/rf305/`・本 doc チェーンに限定。
公開 API・内部実装・依存 DAG は変更しない。

## Upstream Documents

- [docs/specs/issue-305-io-e2e-wav-golden-fault.md](../specs/issue-305-io-e2e-wav-golden-fault.md)（approved）
- Issue #305 本文（受入正本）
- [docs/designs/issue-305-io-e2e-wav-golden-fault.md](../designs/issue-305-io-e2e-wav-golden-fault.md)（approved）
- [docs/test-specs/issue-305-io-e2e-wav-golden-fault.md](../test-specs/issue-305-io-e2e-wav-golden-fault.md)（検証正本）
- 前段 chain: [spec](../specs/issue-305-media-io-staged-file-cutover.md) /
  [design](../designs/issue-305-media-io-staged-file-cutover.md) /
  [plan](../plans/issue-305-media-io-staged-file-cutover.md)
- fixture 規約の先例: [spec](../specs/issue-306-io-e2e-fault-fuzz.md) /
  [design](../designs/issue-306-io-e2e-fault-fuzz.md) /
  [plan](../plans/issue-306-io-e2e-fault-fuzz.md)

## File Map

- `crates/ruviform-io-e2e/tests/wav_golden.rs` — byte golden:
  ZeroFrames/`WAVEFORMATEX`・`WAVEFORMATEXTENSIBLE` header、
  S16 payload byte-exact、encoder 経路の wire format 別 edge
  sample（NaN/Inf/clip/`.5` 丸め/F32 verbatim）、4GiB 境界
  （`DevNull` sink）、独立 decode 照合
- `crates/ruviform-io-e2e/tests/wav_fault.rs` — fault:
  `FlushFail`/`DiskFull` sink wrapper（`&mut StagedFile` 委譲）、
  finalize flush 失敗・mid-stream disk-full・seal 後 abort
  （Cancel）・muxer Drop best-effort finalize 非証拠化、
  publish outcome 三分類の明示区別、`StagedFile::create` 失敗
- `crates/ruviform-io-e2e/tests/wav_staged.rs` — 既存維持
  （重複回避のため原則変更しない）
- `crates/ruviform-io-e2e/README.md`・`src/lib.rs` — fixture
  一覧 doc の整合更新
- `evidence/rf305/wav-golden-fault-report.md` — 実コマンド・
  実値・OS/FS 実測・NotRun/Unsupported・旧 API 証拠・DAG 確認
- `docs/impl/ir/issue-305-io-e2e-wav-golden-fault.md` — 実装記録

## Implementation Sequence

- [x] spec → design → test-spec → plan → task の doc chain 作成
- [ ] W1 fixture 実装（`wav_golden.rs` + `wav_fault.rs` +
      README/lib.rs doc 更新）
- [ ] `mise run fmt-check`/`check-io-e2e`/`lint-io-e2e`/
      `test-io-e2e` で crate lane 検証・修正
- [ ] `mise run check`/`lint`/`test`/`test-rt` で workspace
      非回帰検証
- [ ] 補助証拠: `cargo tree -p ruviform-media-io`・旧
      `StagedOutput`/旧 `PublishOptions` 参照 grep
- [ ] evidence report + impl record + index 更新・handoff 明記
- [ ] review-protocol 再確認・`git fetch origin main &&
      git merge origin/main`・lockfile/生成物の再生成確認
- [ ] push・PR 作成（日本語・Refs #305）

## Dependencies and Risks

- 依存: Issue #305 T1 merge 済み（`WavMuxer<&mut StagedFile>`・
  `StagedFile`/`SealedFile`/`PublishFailure` API は main に存在）。
  新規 dev-dependency なし — 既存 dev-deps のみ消費。
- Risk: byte golden 期待値の誤記 → test 実行で検出、ずれた場合は
  RIFF/WAVE 仕様と宣言 policy 側を修正（実装出力を鵜呑みにしない）。
- Risk: 4GiB 境界試験のメモリ/時間 → `DevNull` sink + 64MiB
  packet の逐次生成で単一巨大 alloc を回避。実測で問題があれば
  chunk サイズを調整。
- Risk: muxer Drop fixture が OS/FS 依存 → Windows NTFS 実測で
  確認し、他 OS は NotRun 記録。
- Risk: `PublishEffect::Published`/`Indeterminate` 失敗 effect は
  `PublishFailure::new` が `pub(crate)` で外部構築不能・実 OS
  誘発不能 → fixture 化せず NotRun 記録（io-file mock 正本）。

## Task Breakdown

| Task | 内容 | 依存 |
| --- | --- | --- |
| W1 | fixture 実装一式（`wav_golden.rs`・`wav_fault.rs`・README/lib.rs doc） | — |
| W2 | 正規検証（mise lane）+ 補助証拠 + evidence report + impl record + handoff 明記 | W1 |

## Verification

- `mise run fmt-check` / `check-io-e2e` / `lint-io-e2e` /
  `test-io-e2e` — 全 pass
- `mise run check` / `lint` / `test`（workspace）— 非回帰 pass
- `mise run test-rt` — RT lane 非回帰（io-e2e は非 RT・対象外
  であることの確認を含む）
- OS/FS: Windows NTFS 実測のみ。macOS/Linux・Network/Cloud FS・
  性能・`Published`/`Indeterminate` 失敗 effect・DirSync 後失敗・
  `TempState::Unknown` の実 OS 再現は `NotRun`/`Unsupported` と
  して記録

## Open Questions / Gaps

- なし（slice 範囲は spec で確定済み。Consumer 実測・#305 の他
  slice は対象外として明記済み）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（dispatched worker 内で直接実装）
- Independent work streams: none
- User approval required before dispatch: no（Orca dispatch 済み・
  review-protocol を承認として扱う）
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- fixture が公開 API のみを消費しているか・`vfs`/mock 非依存か。
- byte golden の期待値が実装内部 helper に依存せず独立に組み立て
  られているか。
- 否定系が typed 分類 + staging/destination/temp 非残存の両方を
  assert しているか。
- publish outcome 分類が `NotPublished` 以外を未公開と誤報しない
  か。NotRun/Unsupported の記録が実測範囲と一致しているか。
