---
id: "TSPEC-034Yo3wCx9u07BHC0xGCRY"
type: "test-spec"
status: "approved"
title: "Issue #253 virtual-first OS-endpoint MIDI e2e verifier test spec"
created: "2026-11-16"
updated: "2026-11-16"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/253"
  changes:
    added:
      - type: "file"
        path: "docs/test-specs/issue-253-virtual-midi-e2e.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-253-virtual-midi-e2e.md"
    - "docs/designs/issue-253-virtual-midi-e2e.md"
    - "docs/plans/issue-253-virtual-midi-e2e.md"
    - "docs/tasks/issue-253-virtual-midi-e2e.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-253-virtual-midi-e2e.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-253-virtual-midi-e2e.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #253 virtual-first OS-endpoint MIDI e2e verifier test spec

## Purpose

`tests/os_endpoint_midi_e2e.rs` が保証すべき振る舞い — 実 OS MIDI
backend から attach・plugin・typed runtime を経た out-of-process
VST3 `test_instrument` への合成 e2e — と、その fail-closed 境界
（`NotRun`/`Fail`/`Pass` の分類）を固定する。provisioned pair が
無い環境での誤 Pass を構造的に防ぐことが本 spec の主目的であり、
pair 存在時の Pass 経路は後続 slice（実 provisioning）の実走で
初めて証明される。

## Feature

OS endpoint MIDI loopback drives the attached `test_instrument`
end-to-end

## Rules

- **R1（fail-closed NotRun）**: backend 列挙失敗・`Unsupported`・
  port 空・name-match pair 0 件・timestamp 非対応 port では
  `verdict: NotRun` を eprintln して early return し、assert
  しない。Skip=Pass にしない。
- **R2（fixture は Fail）**: 本 crate cdylib または `l1-runner`
  runner binary の欠落は `assert!` 失敗 — R1 の正直な `NotRun`
  と区別する。
- **R3（pair open 以降は Fail）**: open・send・配送・plugin・
  runtime・graph・PCM/oracle のいずれかの失敗は `verdict: Fail`
  （panic）。
- **R4（独立 oracle）**: expected event 列・PCM 期待値・counter
  期待値は全て test 内の固定リテラル/独立計算であり、被試験経路
  （adapter encode/decode・bridge・plugin）が期待値を生成しない。
- **R5（stamped-offset PCM oracle）**: 観測 `NoteOn` stamp に
  較正した `ClockAnchor` の `ANCHOR_FRAME` に fixture の
  deterministic attack が着地し、attack 前は完全無音・attack
  値は bit-exact・dual-mono・release は stamped `NoteOff` +
  release 長の範囲で完了・末尾 block は無音。
- **R6（counter oracle）**: `InputStage` snapshot と `InputTx`
  の backend 透過 counter が、pump 観測値・独立期待値
  （`BeforeAnchor` reject 数 = anchor stamp より前の観測 event 数）
  と一致する。
- **R7（close semantics）**: input close 後の `pump` と output
  close 後の `send` は `DeviceError::StreamInvalidated`。
- **R8（virtual 非主張）**: name match・`MidiPortOrigin::Unknown`
  のみを根拠に `environment_kind=virtual-os-endpoint` を主張しない
  — verdict は「loopback candidate であり virtual endpoint の
  証明ではない」旨を明記する。
- **R9（stamp 契約）**: 観測 event は port 宣言 domain・`Carried`
  provenance・非減少 ticks・`TimeBase::new(1, 1_000)` base を
  満たす。foreign base は envelope 外の `NotRun`、他違反は `Fail`。
- **R10（非 Windows fail-closed）**: 非 Windows では backend が
  `Unsupported` を返し `NotRun` — `#[cfg]` 分岐なしで 3OS lane
  の compile/green を維持する。

## Examples

- **pair 未 provision（本ホスト実測予定）**: Given 実 WinMM
  backend が動く Windows ホストで loopback-style name-match pair
  が存在しない, When `--ignored` で実行, Then
  `verdict: NotRun — NOT_PROVISIONED` を出力して終了し、assert
  0 件・Pass 非主張。
- **timestamp 非対応 port のみ**: Given name-match pair が存在
  するが input port が `provides_timestamps=false` または
  `clock_domain=None`, When 実行, Then stamped-offset oracle が
  構成不能なため `verdict: NotRun`。
- **pair 存在・正常配送**: Given provisioned loopback pair, When
  channel 3 の NoteOn/CC/PitchBend/NoteOff を 8 ms 間隔で送信,
  Then 観測 stamp → anchor 較正 → attack が `ANCHOR_FRAME` に
  着地・release tail が stamped NoteOff 基準・counter 全項目が
  oracle 一致・close 後 `StreamInvalidated` → `verdict: Pass`
  （ただし `virtual-os-endpoint` は主張しない）。
- **配送欠落**: Given pair は open するが event が届かない, When
  pump deadline 5 s 超過, Then `verdict: Fail`（panic）— NotRun
  への格下げはしない。
- **PCM 逸脱**: Given event は届くが attack offset/値/release/
  dual-mono のいずれかが oracle と不一致, Then `verdict: Fail`。

## Guarantees

- 正しい実装は、環境未整備を Pass と報告しない（R1/R2/R8）。
- 正しい実装は、pair 存在時に実 OS backend → attach → plugin →
  runtime → fixture の全段を実際に駆動し、独立 oracle で PCM・
  counter・close を検証する（R3–R7）。
- stamp の frame 換算は観測値にのみ依存し、期待 stamp を test が
  捏造しない（R4/R9）。

## Non-goals

- seek/stop/queue overflow/loss の決定的再現（#628 scripted 側の
  管轄）。
- `virtual-os-endpoint` Pass の取得（provisioning は repo 外・
  後続 slice）。
- macOS/Linux backend の検証（`Unsupported`/`NotRun` が正しい
  verdict）。
- `test-l1-real-plugin`/`test-midi-real-device` lane への自動
  収録（`mise.toml` は別 Worker 占有 — 本 file は個名指定の
  手動 opt-in）。

## Risk

- oracle が強すぎて provisioning 環境でも常時 Fail になる危険 —
  attack offset・lookahead slot・release bound は scripted 先例
  と fixture 定数から導出し、許容差は「block 1 個分」までに
  限定して誤判定と見逃しの両方を抑える。
- `mapped == delivered` 等の counter 仮定が edge case で崩れる
  危険 — stamped span を `MAX_BLOCKS` で事前制限し、崩れた場合は
  観測値を verdict に残して Fail とする（silent pass にしない）。
