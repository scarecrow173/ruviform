---
id: "TSPEC-034UvWxgBRObLuwfTjwqMx"
type: "test-spec"
status: "approved"
title: "Issue #305 IO E2E WAV Golden/Fault テスト仕様"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/305"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/wav_golden.rs"
      - type: "file"
        path: "crates/ruviform-io-e2e/tests/wav_fault.rs"
      - type: "file"
        path: "docs/specs/issue-305-io-e2e-wav-golden-fault.md"
      - type: "file"
        path: "docs/designs/issue-305-io-e2e-wav-golden-fault.md"
      - type: "file"
        path: "docs/test-specs/issue-305-io-e2e-wav-golden-fault.md"
      - type: "file"
        path: "docs/plans/issue-305-io-e2e-wav-golden-fault.md"
      - type: "file"
        path: "docs/tasks/issue-305-w1-fixture-extension.md"
      - type: "file"
        path: "docs/tasks/issue-305-w2-verification-handoff.md"
      - type: "file"
        path: "evidence/rf305/wav-golden-fault-report.md"
      - type: "file"
        path: "docs/impl/ir/issue-305-io-e2e-wav-golden-fault.md"
    modified:
      - type: "file"
        path: "crates/ruviform-io-e2e/README.md"
      - type: "file"
        path: "crates/ruviform-io-e2e/src/lib.rs"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "Cargo.lock"
  implements:
    - "https://github.com/scarecrow173/ruviform/issues/305"
  implemented-by: []
  depends-on:
    - "docs/designs/issue-305-io-e2e-wav-golden-fault.md"
    - "docs/test-specs/issue-306-io-e2e-fault-fuzz.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/test-specs/issue-305-media-io-staged-file-cutover.md"
    - "docs/test-specs/issue-306-io-e2e-fault-fuzz.md"
    - "docs/specs/issue-295-io-file-staged-publish-contract.md"
  refines:
    - "docs/test-specs/issue-305-media-io-staged-file-cutover.md"
  refined-by: []
  derives-from:
    - "docs/designs/issue-305-io-e2e-wav-golden-fault.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-305-io-e2e-wav-golden-fault.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #305 IO E2E WAV Golden/Fault テスト仕様

## Purpose

`wav_staged.rs`（合成境界の存在証明）と `ruviform-io-wav`
contract/roundtrip（in-memory sink 単体契約）で未カバーの、io-e2e
層における **publish 済み WAV の byte-exact 決定性** と **実 FS
上の fault 分類** を pin する fixture 規約を固定する。将来
consumer gate が `mise run test-io-e2e` の結果を参照する際の
証拠範囲を確定する目的があり、実装手順ではなく観測可能な保証を
規定する。

## Feature

- WavMuxer→`StagedFile` 経路で publish された WAV が、RIFF/WAVE
  仕様と宣言済み変換 policy から独立に組み立てた期待 byte 列と
  一致し、symphonia 独立 decode とも一致する。
- finalize/publish の fault が公開 API 経由で誘発可能な class に
  限定され、各失敗が「未公開」または「未公開と証明できない」に
  正しく分類され、偽の完了証拠・偽の未公開報告を生まない。

## Rules

- Byte golden の期待値はテスト側で RIFF/WAVE 仕様から手組みし、
  crate 内部 helper を参照しない。publish 後の destination
  bytes と独立 decode payload の両方を assert する。
- encoder 経路の edge sample（NaN/±Inf/|x|>1/`.5` LSB 丸め）は
  宣言 policy を独立に符号化した期待 byte で pin する。
- `data_bytes` が `u32::MAX` を超える WAV は `ErrorKind::
  InvalidData`・receipt 無しで失敗し、破棄 sink で検証する
  （実 4GiB ファイルを生成しない）。
- finalize の sink flush 失敗・書込 `StorageFull` 失敗は
  `ErrorKind::Io`・receipt 無しで失敗し、publish せず abort
  し、destination 無変更・temp 非残存を assert する。
- muxer の `Drop`（best-effort finalize）や `into_sink()` は
  完了証拠に使わない。finalize receipt と readback が publish
  の判断材料であり、readback decode 可能な bytes でも publish
  せず abort できる。
- publish outcome の分類は `Ok`→Published、`Err`+effect
  `NotPublished`→NotPublished、その他の effect→
  NotProvablyUnpublished とし、`Published`/`Indeterminate` を
  未公開と誤報しない。誘発不能 class は偽装せず NotRun 記録。

## Examples

- ZeroFrames: `WavMuxer` header 即時書込 + `finalize` → RIFF
  size=36・`fmt `=16・`data`=0・WAVE_FORMAT_PCM の期待 byte
  列と publish 済み bytes が一致。
- S16 stereo payload: 既存 `wav_staged.rs` の決定的 packet
  payload が data chunk と byte 一致し、symphonia decode 後も
  一致。
- Encoder edge samples: U8 `NaN`→0x80、S16 `+Inf`→i16::MAX、
  F32 `+Inf`→`0x7F800000`、`.5` LSB 丁度が round half away
  from zero になることを file bytes の data chunk で assert。
- 4GiB 境界: `data_bytes` 累積が `u32::MAX` 超過した時点で
  `InvalidData`・`receipt().is_none()`。
- Finalize flush 失敗: `FlushFail` wrapper が finalize 時に
  `Io` を返し、`abort()` 後 destination 不在・temp 残存なし。
- Cancel: seal 済み `SealedFile` を `abort()` → publish され
  ない。muxer 未 finalize drop でも seal→readback decode は
  可能だが abort で publish しない。
- Publish outcome: CreateNew 成功→Published、destination 既存
  で CreateNew→Err+NotPublished+DestinationExists。
  `Published`/`Indeterminate` failure effect は実 OS 誘発不能
  → NotRun（io-file mock 正本参照）。

## Guarantees

- WAV 保存経路の byte 出力は決定的であり、fixture 追加後も
  変更があれば byte golden が検出する。
- finalize/disk-full/cancel の各 fault は publish に到達せず、
  staging/destination/temp の状態が契約通り正直に処理される。
- 「publish していない」ことの主張は `NotPublished` effect の
  実測のみに基づき、`Published`/`Indeterminate` を未公開と
  誤報する経路が存在しない。

## Non-goals

- 実 4GiB ファイル生成、macOS/Linux・Network/Cloud FS、性能
  計測、trybuild 等の新規コンパイル失敗試験基盤、Consumer
  repo での実測、Published/Indeterminate・DirSync 後失敗・
  TempState::Unknown の実 OS 再現。

## Risk

- golden 期待値の誤記・policy 解釈違いで fixture が実装バグを
  見逃す/誤検出する → test 実行と RIFF/WAVE 仕様照合で検出。
  ずれた場合は実装出力ではなく仕様・宣言 policy 側を修正。
- Drop fixture が OS/FS 依存で不安定 → Windows NTFS 実測で
  確認し、他環境は NotRun 記録。fixture 自体は公開 API のみ
  使用し backend 詳細に依存しない。
