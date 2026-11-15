---
id: "SPEC-034Y3ehWbCBEZwEl47ROsE"
type: "spec"
status: "approved"
title: "Issue #628 close-out — RunMetadata JSON・cancellation fault・docs 整合・AC 棚卸し"
created: "2026-11-15"
updated: "2026-11-15"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "docs/specs/issue-628-deterministic-test-runtime.md"
    - "docs/specs/issue-628-scripted-backend.md"
    - "docs/specs/issue-628-pipeline-wiring.md"
    - "docs/designs/issue-628-scripted-backend.md"
    - "POLICIES.md"
  changes:
    added:
      - type: "file"
        path: "docs/specs/issue-628-close-out.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "docs/plans/issue-628-close-out.md"
  depends-on:
    - "docs/specs/issue-628-pipeline-wiring.md"
    - "docs/designs/issue-628-scripted-backend.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-628-close-out.md"
    - "docs/test-specs/issue-628-close-out.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-628-pipeline-wiring.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-628-close-out.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #628 close-out — RunMetadata JSON・cancellation fault・docs 整合・AC 棚卸し

## What Should Be Built

`crates/ruviform-audio-test` の残存小項目を統合する close-out slice:

1. **RunMetadata JSON serialization** — `RunMetadata`/`RunParameters`/
   `CaseRequirement`/`CaseResult`/`VerificationLevel`/`EnvironmentKind`
   （`metadata.rs`）と `VirtualClockConfig`/`JitterSpec`（`clock.rs`）に
   serde derive を付け、JSON serialize/save/load API と round-trip
   test を追加する。wire format は Case Schema v1 語彙
   （`EnvironmentKind::as_str` の kebab-case 等）に揃える。
2. **cancellation fault class** — Issue の Fault Injector リストの
   `cancellation` を `StreamFault` の新 variant でカバーし、production
   契約の observable への正直な写像と Expected/Observed 正負テストを
   付ける。本番契約への新 variant・新 API は追加しない。
3. **docs 陳腐化修正** — merge 済み 3 slice の `docs/tasks/` status、
   `crates/README.md`・crate `Cargo.toml` description・crate README の
   "first slice" 記述を現状へ更新する。
4. **AC 棚卸し表** — Issue 全受入条件 × 現状 verdict（Pass/Partial/
   NotRun + 根拠・root 証拠）を `evidence/rf628/` に記録する。

## Why Now

1st slice（deterministic core、PR `scarecrow173/ruviform#638`）、
2nd slice（scripted backend、PR `scarecrow173/ruviform#659`）、
3rd slice（pipeline wiring、PR `scarecrow173/ruviform#664`）が merge
済みで、consumer oracle（AC 第 6 項目分）も両 repo で merge 済み。
残るのは Run metadata の保存手段・Fault Injector リストの
`cancellation`・docs の陳腐化・Issue 全体の AC 棚卸しであり、これらを
束ねて Issue #628 の受入判定材料を揃える。

## Users and Value

- consumer harness（AudioLens/Synvelis）が deterministic run を
  JSON metadata として保存・再実行・比較できる。
- `cancellation` fault により、consumer が abort/中断系の障害を
  deterministic に再現できる。
- Issue #628 の close 判定に必要な AC × verdict の棚卸しが証拠として
  残る。

## Scope

### In Scope

- `metadata.rs`/`clock.rs` への serde derive、JSON to/from・
  save/load API、round-trip test。
- `backend.rs`/`backend/stream.rs` への `StreamFault::Cancellation`
  variant と正負テスト。
- `docs/tasks/issue-628-{deterministic-test-runtime,scripted-backend,
  pipeline-wiring}.md` の status 修正、`crates/README.md`・
  `crates/ruviform-audio-test/{Cargo.toml,README.md}` の陳腐記述更新。
- `evidence/rf628/` への AC 棚卸し表と検証記録。
- 本 slice の docs chain（spec→design→plan→task→test-spec→
  impl/ir→impl/exp）と index 再生成。

### Out of Scope

- MIDI→Host adapter `attach()` 配線・実 endpoint 接続
  （`scarecrow173/ruviform#630` runtime integration slice の所有）。
- CI lane 新設・workflow/mise 変更 — AC「既存CIに統合」は新規テストが
  既存 `test`/`test-rt` lane に自動収集される構成で充足済みという
  設計判断の明記が本 slice の仕事。
- production crates の src 変更 — cancellation 写像のため本番
  `DeviceError` 等へ新 variant・新 API を追加しない
  （test-only 境界不変）。
- `src/{pcm.rs,oracle.rs,midi.rs,pipeline.rs,pipeline/}` の変更。
- 前 slice の docs chain（specs/designs/plans/test-specs/impl）の
  status・内容変更 — approved 履歴として保持。
- 実 OS/実デバイス/実スレッド timing・xrun・duplex・latency の実走証拠・
  Windows 仮想 endpoint（EXT-L3・`scarecrow173/ruviform#629` 領域）。
- `*-msrv`・`*-full` 系検証・3OS 横断（daily/手動 Full CI 委譲）。
- `serde`/`serde_json` 以外の新規依存追加。
- Issue の Close（PR 本文は `Refs #628` のみ）。

## Requirements

- serde wire format は Case Schema v1 語彙と整合させ、独自の第 2
  語彙を生やさない（`EnvironmentKind::as_str` の kebab-case と
  一致する表現を選ぶ）。
- JSON save/load は非 RT の test-driver API として配置し、alloc/IO を
  `rt_` 契約経路へ持ち込まない。
- cancellation の写像は production `DeviceError` の既存 observable
  （`Backend(&'static str)` 分類名 + stream 終了）の範囲で設計し、
  採用した解釈を rustdoc と design doc に明記する。
- 実行していない検証を Pass と書かない。実 OS/実デバイス/実スレッド
  timing・EXT-L3・Windows 仮想 endpoint は NotRun/Deferred と記録する。
- test-only 境界維持 — 製品配布に混入しない・本番 crate からの依存を
  作らない・production API に Test-only 具象型を露出しない。

## Acceptance Criteria

- [ ] `RunMetadata`（および `RunParameters`・4 enum・
  `VirtualClockConfig`/`JitterSpec`）が serde `Serialize`/
  `Deserialize` を実装し、JSON save/load と round-trip test がある。
- [ ] wire format が Case Schema v1 語彙（kebab-case）と一致することを
  test で pin する。
- [ ] `StreamFault::Cancellation` が audio stream に対して Expected/
  Observed 一致の正負テストを持つ。
- [ ] cancellation の写像解釈（何を cancel と見なすか・production
  契約のどの observable で表すか）が rustdoc・design doc・棚卸し表に
  明記される。
- [ ] `docs/tasks/` 3 件の status・`crates/README.md`・crate
  `Cargo.toml` description・crate README の陳腐記述が現状に揃う。
- [ ] `evidence/rf628/` に Issue 全 AC × 現状 verdict の棚卸し表がある。
- [ ] `mise run fmt-check`/`check`/`lint`/`test ruviform-audio-test`/
  `test-rt`/`prek run --all-files` が pass し、docs index が再生成
  されている。
- [ ] 製品 crate の src に差分がない（test-only 境界）。

## Deferred Design Concerns

- MIDI 側 `backend/midi.rs` の fault への cancellation 拡張 — Issue
  の Fault Injector リストは audio stream を主対象と判断。本 slice で
  audio stream のみカバーし、MIDI 側は棚卸し表で Deferred と記録する
  | reason: `MockPort` の inject 系は既に dropped/unsupported を持ち、
  cancel 相当の意味付けが audio stream と異なる | re-engagement
  trigger: consumer が MIDI cancel 経路を要求した時点 | risk if
  ignored: MIDI harness で cancel 系障害を再現できない（現状要求なし）。

## Sources and Relations

- Issue `scarecrow173/ruviform#628`（Fault Injector リスト・AC）。
- 前 slice docs chain:
  `docs/{specs,designs,plans,tasks,test-specs}/issue-628-deterministic-test-runtime*.md`、
  `issue-628-scripted-backend*.md`、`issue-628-pipeline-wiring*.md`。
- `POLICIES.md`（OS-L1/GUI-L2/EXT-L3、Required/Deferred/NotApplicable ×
  Pass/Fail/NotRun/Unsupported、2026-11-15 仮想優先改訂）。
- 実装対象: `crates/ruviform-audio-test/src/{metadata,clock,backend}.rs`、
  `src/backend/stream.rs`、`tests/`。
