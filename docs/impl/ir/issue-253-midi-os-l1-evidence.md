---
id: "IMPL-034WUeMyAfuU4RnkO7aDgw"
type: "impl"
status: "completed"
title: "Issue #253 OS-L1 既存CI証拠照合・Backend能力/未実装の正直な記録"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/issue-253-midi-os-l1-evidence.md"
      - type: "file"
        path: "evidence/issue-253-real-midi-lane/os-l1-ci-evidence.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "docs/impl/ir/README.md"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/issue-253-os-midi-input-backend-adapter-windows-winmm-os-unsupported.md"
    - "docs/impl/ir/issue-253-midi-backend-fixture-and-vst3-translation.md"
    - "docs/impl/ir/issue-253-vst3-midi-canonical-path.md"
    - "docs/impl/ir/issue-253-unsupported-transparency-slice.md"
    - "docs/impl/ir/issue-388-os-l1-ci-evidence.md"
    - "docs/impl/ir/issue-313-os-l1-ci-evidence.md"
    - "docs/designs/issue-253-os-midi-backend.md"
    - "evidence/issue-253-real-midi-lane/README.md"
    - "evidence/issue-253-real-midi-lane/os-l1-ci-evidence.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #253 OS-L1 既存CI証拠照合・Backend能力/未実装の正直な記録

## Summary

Issue #253 の OS-L1 証拠 slice: 既存 Full CI run
`43428172820`（`e93c438a`・全 20 job success）から MIDI 関連
crate（`ruviform-midi-device` / `ruviform-audio-plugin-bridge` /
`ruviform-plugin-vst3` / `ruviform-plugin-clap` /
`ruviform-audio-core` / `ruviform-media-core`）の Build/Link・
Mock/Headless Backend 変換・Timing/Error/RT 回帰を照合し、
`evidence/issue-253-real-midi-lane/os-l1-ci-evidence.md` に
OS×Backend×Device×Case の Pass/Unsupported/NotRun/Deferred 一覧を
記録した。対象 SHA `b9eaf488`（main HEAD・PR #602 merge）に
対応する completed CI run は存在しないため、Coordinator 裁定
（orchestration `msg_482da7ea856e`）で `e93c438a` run を採用し、
`e93c438a..05c7fc3b` の差分に MIDI 影響 path が 0 件であることを
`git diff` で検証済みである旨を証拠内に明記した。補足として
`05c7fc3b` の dispatch run `43436082797`（conclusion=
failure: `verify (ubuntu-latest)` の prek `actionlint` hook
失敗 + `ci-summary`）でも MIDI 関連 lane（Windows/macOS
verify・rt-verify・l1-real-verify×3）は全て success で同一の
MIDI test 結果を確認した。コード変更はなく文書のみの
slice（文書チェーンは本 IR に集約 — spec/design 等の重複生成は
行わない）。Issue #253 は open 継続。

## Context

- 受入条件（本 slice 転記）: 既存 CI の Windows/macOS/Linux で
  MIDI 関連コードの Build/Link、Mock/Headless の Backend 変換・
  Timing/Error/RT 回帰を対象 SHA で確認する（Feature opt-in の
  場合はその Feature 実行範囲も明示）。未実装・未提供の
  Backend/Device 能力を正確に記録し、存在しない実動作を主張
  しない。実測済みと NotRun/Unsupported の OS×Backend×Device×
  Case 一覧を証拠 URL とともに確定する。実 MIDI device/実 OS
  権限等の L3 相当は Deferred — NotRun を Pass にしない。
- 編集範囲: `evidence/issue-253-real-midi-lane/`（既存 dir への
  追記のみ・既存ファイルの改変なし）、本 IR のみ。
- 対象 SHA は `b9eaf4880681734b2d65dbf200ca8c49c6e4c81e` で固定
  予定だったが、記録時点で `git rev-parse origin/main` は
  `05c7fc3bff54fbf73264408dc5a33dd4bff8f9d8`（PR #603 merge）に
  進行していた。タスク指定の「新しい main 進行があれば最新に
  合わせて記録に明記」に従い両 SHA を記録した。
- 検証レベルの規範は `POLICIES.md` §2〜§5。実機 lane
  （`test-midi-real-device`）は opt-in・CI 非搭載であり、本
  slice のローカル必須検証に含めない（過去証拠の引用対象）。

## Implementation

### Changes

#### `evidence/issue-253-real-midi-lane/os-l1-ci-evidence.md`（新規）

- 検証スコープと Oracle（採用 OS・Feature・Device/Format・Run
  URL/Oracle・新規 Workflow 不作成の明記）。
- 主証拠 run `43428172820`（`e93c438a`）の SHA 照合:
  `headSha`・MIDI 関連 7 leg の `checkout-sha-*` artifact 実測・
  `e93c438a..b9eaf488`（11 files）と `b9eaf488..05c7fc3b`
  （15 files）の `git diff --name-only` で MIDI 影響 path 0 件を
  検証した記録。`b9eaf488` の completed run 不在（`gh run list
  --commit` = 0 件）と Coordinator 裁定の経緯も明記。
- 補足証拠 run `43436082797`（`05c7fc3b`、conclusion=
  failure・completed）の job 別結果: MIDI 関連 lane
  （`verify (windows-latest)`/`verify-macos`/`rt-verify`/
  `l1-real-verify`×3）は全て success、`verify (ubuntu-latest)`
  は prek `actionlint` hook の shellcheck 指摘で cargo 未到達
  failure（05c7fc3b での Linux `test-full` は NotRun）、
  `ci-summary` は fail-closed で failure、他 16 job success。
- lane × OS カバレッジ表と crate 別 test 集計（`test-full` log
  実測: midi-device lib 17+1ig/1/1、ports.rs 2+1ig/1+1ig/1+1ig、
  bridge midi_delivery 6・midi_wiring 15+1ig、plugin-vst3 41
  （default）/74（`host`）、plugin-clap 48/65、rt-verify strict
  `rt_` = 166 で `rt_midi_*` 2 件 `ok`、l1-real-verify の
  `host::tests::imidi_mapping_*` 3 件 `ok`・ignored E2E は audio
  のみで MIDI coverage 0、msrv-verify は MIDI crate 対象外）。
- OS×Backend×Device×Case 判定一覧（Pass/Unsupported/NotRun/
  Deferred を証拠 URL・job URL 付きで確定）。
- Feature opt-in 実行範囲の明示（default feature / `host`
  feature / `test-midi-real-device` manual lane / `real-device`
  feature は audio-device 専用で MIDI 無関係）。

#### `docs/impl/ir/issue-253-midi-os-l1-evidence.md`（本書）

- 実施内容・証拠・判定を IR として記録。

#### `docs/impl/ir/README.md`

- `sync_impl_index.js --kind ir --apply`（doc-maintenance 経路）
  で再生成（本 IR 追加分）。

### Decisions During Implementation

- **証拠 run の選定**: タスク指定 SHA `b9eaf488` の completed CI
  run が存在しないため `orca orchestration ask` で確認。
  Coordinator 回答（`msg_482da7ea856e`）は選択肢 (b): `e93c438a`
  run `43428172820` を MIDI capability matrix の OS-L1 証拠とし
  て引用し、対象 HEAD との差分に MIDI 影響変更なしを検証済みと
  明記すること。加えて Coordinator が `05c7fc3b` で run
  `43436082797` を dispatch 済みのため、PR 時点の状態を併記
  する指示に従った。
- **SHA 差分検証の実施**: `git diff --name-only` を
  `e93c438a..b9eaf488`・`b9eaf488..05c7fc3b`・合算
  `e93c438a..05c7fc3b` の 3 通りで実行し、MIDI/plugin/audio 系
  path への変更が 0 件であることを確認（変更は ci.yml の
  actionlint/pre-commit 配線・ruviform-ui-test・ruviform-io-e2e・
  docs・evidence のみ）。
- **能力判定の語彙**: macOS/Linux の実 backend は「実装なし」
  ではなく `DeviceError::Unsupported` を返す公開契約どおりの
  明示境界として扱い、契約 test の Pass（Unsupported 実装の
  正しさ）と実 port 能力の Unsupported（backend 未提供）を分離
  して記録した。Windows 実ポート経路・実 Plugin/DAW 往復・
  MSRV での MIDI crate は NotRun/Deferred のままとし、CI green
  や ignored test の存在を Pass に繰り上げていない。
- **feature 実行範囲の分離**: `host` feature でのみ compile
  される `host::tests::imidi_mapping_*`（IMidiMapping CC→
  Parameter routing）と、default feature の plugin 側
  `midi::tests::*`/`adapt::tests::*` を区別して記録。
  `l1_real_adapter_e2e.rs` に MIDI 参照がないことを grep で確認
  し、L1 実 adapter E2E の MIDI coverage = 0 を明記。
- **branch**: Orca worktree の既定 branch `scarecrow173/
  rf253-evidence` から、タスク指定の `docs/253-midi-os-l1-
  evidence` を作成し `origin/main`（`05c7fc3b`）を merge して
  作業した（#388 slice と同じ手順）。

## Related Experiments

### Adopted

なし — Experiment Log は作成していない（機械的な証拠照合+
文書記録の slice で、採用設計は task spec と既存 IR/design で
固定済み）。

### Rejected

なし。

## Validation

### CI 証拠照合（補助証拠: GitHub API/gh 実測。canonical = CI run 自体）

- `gh run view 43428172820` → status=completed/conclusion=
  success、headSha=`e93c438a…`、全 20 job success。
- `gh api .../actions/runs/43428172820/jobs` で job 一覧・ID を
  取得し、MIDI 関連 7 leg（verify×3・rt-verify・l1-real×3）の
  job log を `gh api .../jobs/<id>/logs` で取得。crate 別の
  `test result` 集計・個別 test 行（`ok`/`ignored`）を照合。
- `gh run download 43428172820 -p 'checkout-sha-verify-*'
  -p 'checkout-sha-rt-verify-*' -p 'checkout-sha-l1-real-verify-*'`
  → 7 leg 全件の実測 SHA が `e93c438a…` で一致。
- `gh run list --commit b9eaf488` → 0 件（対象 SHA の completed
  run 不在を実測）。`gh run view 43436082797` で補足 run の
  status/job 結果を取得し、完了済み job の log でも MIDI test
  行を確認。
- `git diff --name-only` 3 通りで MIDI 影響 path 0 件を検証。
- `mise.toml`（`test-midi-real-device`/`test-l1-real-plugin`/
  `test-rt`/`lint-full`/`test-full` 定義）・`ci.yml`（lane 構成・
  `checkout-sha` 機構）・対象 crate の `Cargo.toml`/`src`（cfg
  構造・`unsupported` 経路・feature gate）を静的確認。

### ローカル検証

| コマンド | exit | 結果 |
| --- | --- | --- |
| `mise run fmt-check` | 0 | `cargo fmt --all -- --check` |
| `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` | 0 | 23 findings は全て既存 file（issue-306/313/582/299・poc-clap-binding-adapter）の pre-existing で、本 IR の finding は 0 件 |
| `node .agents/skills/impl-doc/scripts/sync_impl_index.js --kind ir --apply` | 0 | `docs/impl/ir/README.md` 再生成（本 IR 追加の 1 change） |

文書・証拠のみの slice のため Workspace build/test は未実施
（`POLICIES.md` §5: Markdown/リンクの軽量チェックが原則）。
prek pre-commit hook の commit 時チェックを実施。

レビュー: review-protocol を本 IR と証拠 file に対して実施
（実施回数・結果は PR 本文に記録）。

## Risks

- **SHA 厳密性**: 主証拠は `e93c438a` であり対象 SHA
  `b9eaf488`/`05c7fc3b` の同一 SHA CI 証拠ではない。差分検証
  （MIDI 影響 path 0 件）は build/test 結果の読み替え根拠だが、
  POLICIES の「対象 SHA の CI 証拠が統合検証の正本」の原則上、
  merge 後 main SHA の必須 CI 結果は Coordinator の統合検証に
  委譲される。特に `05c7fc3b` では `verify (ubuntu-latest)` が
  prek/actionlint で failure しており Linux `test-full` の
  同一 SHA 証拠は未取得（workflow 修正は別対応が必要）。
- **run 43436082797 の conclusion=failure**: MIDI 関連 lane は
  全て success だが `verify (ubuntu-latest)`（prek
  `actionlint` の shellcheck 指摘・cargo 未到達）と
  `ci-summary` が failure で run 全体は failure。main の CI
  破損であり workflow/設定修正は本 slice 範囲外。
  merge 後の統合検証で Linux `test-full` が NotRun のままに
  なる可能性がある。
- **checkout-sha artifact の retention**: 30 日。証拠 file に
  test 集計値と job URL を転記済みのため artifact 失効後も判定
  根拠は残る。
- **evidence の capability 判定は `e93c438a` 時点の CI/lane に
  基づく**。merge 後の main SHA で lane 構成が変われば齟齬し
  得る（統合検証は merge SHA の CI が正本）。

## Follow-ups

- Issue #253 open 継続: 実 MIDI ポートでの L3 実測（Windows
  利用機能として Required — ポート保有環境での
  `mise run test-midi-real-device`）、macOS/Linux 実 backend
  実装の要否判断（現状 `DeviceError::Unsupported` 明示境界）、
  実 Plugin/DAW での MIDI mapping・output reports 受入
  （Synvelis #126/#127 が Owner）、MIDI2/UMP/MPE（future
  #317）。
- run `43436082797`（`05c7fc3b`）は completed・
  conclusion=failure で確定済み: `verify (ubuntu-latest)` の
  prek/actionlint failure（PR #602 で追加された hook が
  `consumer-verify.yml`/`ci.yml` の shellcheck style を検出）
  と `ci-summary` が失敗し、main の CI は現時点で破損している。
  workflow/設定側の修正が別途必要（本 slice 範囲外・
  `.github/` は編集禁止）。
- merge 後: merge SHA の `ci.yml` で MIDI 関連 test が 3 OS
  verify lane・rt-verify・l1-real-verify で実走したことを
  Coordinator の統合検証で確認（Linux は `verify
  (ubuntu-latest)` の prek failure が解消されるまで NotRun
  の可能性あり）。
