---
id: "SPEC-034UvWaV1HSJ4d8QLZI8fF"
type: "spec"
status: "approved"
title: "Issue #305 IO E2E slice — WAV→StagedFile Byte Golden・Fault fixture"
created: "2026-11-09"
updated: "2026-11-09"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/305"
    - "https://github.com/scarecrow173/ruviform/issues/287"
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
    - "docs/specs/issue-305-media-io-staged-file-cutover.md"
    - "docs/specs/issue-306-io-e2e-fault-fuzz.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-305-media-io-staged-file-cutover.md"
    - "docs/designs/issue-306-io-e2e-fault-fuzz.md"
    - "docs/designs/issue-295-io-file-staged-publish.md"
    - "docs/designs/issue-296-io-file-os-backend.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/issue-191-pcm-wav-output-backend-ruviform-io-wav-crate-mediasink-port-staged-pub.md"
  refines:
    - "docs/specs/issue-305-media-io-staged-file-cutover.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-305-media-io-staged-file-cutover.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #305 IO E2E slice — WAV→StagedFile Byte Golden・Fault fixture

## What Should Be Built

Issue #305（Media I/O 旧 `StagedOutput` 撤去・WAV→`StagedFile` 移行、
IO-IMP-11）の残り fixture 行。T1（`docs/specs/
issue-305-media-io-staged-file-cutover.md`、done）で確立した
`WavMuxer<&mut StagedFile>` → `finalize` → `seal` → readback 独立
decode → `publish` の合成境界に対し、Issue 受入条件の未カバー行を
`crates/ruviform-io-e2e` の試験専用 fixture として追加する。
公開 API・内部実装は一切変更しない（`ruviform-io-wav` の src は
真の defect 発見時のみ最小修正）。

追加する合成境界:

1. **WAV Byte Golden**: `ZeroFrames`/`PCM`/`RIFF` 構造（`fmt `
   chunk・`WAVEFORMATEX`/`WAVEFORMATEXTENSIBLE`・chunk offset・
   size backpatch）/`4GiB` 境界/`NaN`/`Inf`/`Precision` の決定的
   byte 期待値。`StagedFile` publish 後の destination bytes を
   テスト側で RIFF/WAVE 仕様から独立に組み立てた期待 byte 列と
   比較し、symphonia 独立 decode で照合する。encoder 経路
   （canonical `f32` → wire 変換）の NaN/Inf/clip/丸め policy は
   `convert.rs` の宣言済み規則を独立に符号化した期待値で pin
   する。
2. **Fault fixture**: `Finalize` 失敗（sink の flush 失敗 →
   `ErrorKind::Io`・receipt 無し → publish せず abort）・
   `Cancel`（finalize 成功後の seal→abort、および muxer Drop の
   best-effort finalize を完了証拠にしないこと）・`DiskFull`
   相当（sink `StorageFull` → `Io` → abort → destination 無変更）
   を実 FS の `StagedFile` 上で検証する。publish outcome は
   `PublishEffect` の `NotPublished`/`Published`/`Indeterminate`
   三分類を明示的に区別し、`Published`/`Indeterminate` を
   「未公開」と誤報しない分類規則を pin する。
3. **旧 API 非存在のコンパイル時証拠**: trybuild は workspace に
   存在せず、新規 dev-dependency は追加しない。`compile_fail`
   doctest は `ruviform-io-file`/`ruviform-io-symphonia` 等の
   編集範囲外クレートにのみ存在するため、本 slice では
   workspace 内の `StagedOutput`/media-io 旧 `PublishOptions`
   参照ゼロ（`docs/` 歴史資料を除く）と `mise run check`/`test`
   緑をもって証拠とし、evidence report に記録する。
4. **API/Feature/依存 DAG 維持の確認記録**: `MediaSource`/
   `MediaSink`/Backend API/Feature と依存 DAG
   （`ruviform-media-io` の Domain Core3 依存、`ruviform-io-wav`
   の dev-deps 限定 `ruviform-io-file`/`ruviform-io-symphonia`、
   `ruviform-io-e2e` の葉側 consumer 位置）を `cargo tree` 等の
   補助証拠で確認・記録する。

## Why Now

Issue #305 T1 は merge 済みで旧 API 撤去と `StagedFile` 移行は完了
しているが、Issue 受入条件の「WAV 正常・ZeroFrames/PCM・RIFF/
4GiB・NaN/Inf/Precision/Byte Golden・Finalize 失敗/Cancel/
DiskFull・Published/Indeterminate を試験」のうち、io-e2e 層での
**決定的 byte golden** と **fault 分類の明示区別** が残っている。
`ruviform-io-wav` の contract/roundtrip 試験は in-memory sink 中心
で、byte 期待値の独立検証・実 FS 上の finalize/disk-full/publish
outcome 分類は未整備。Consumer 受入 Gate（#294・AL #143/SV #84）
が参照する正本 command `mise run test-io-e2e` の証拠範囲を確定
させるため、本 slice で閉じる。

## Users and Value

- RF #294/#219: consumer-verify が参照する Ruviform 側 Gate の
  WAV byte 決定性・fault 分類の実測根拠。
- Consumer（AudioLens #143・Synvelis #84）: WAV 保存経路が
  byte-exact に決定的であり、finalize/publish の失敗分類が
  「未公開」の誤報を含まないことの検証済み根拠。
- 後続 slice: #305 の Candidate revision 受入材料。

## Scope

### In Scope

- `crates/ruviform-io-e2e/tests/` への `wav_golden.rs`・
  `wav_fault.rs` 追加（`wav_staged.rs` は必要に応じた最小拡張）。
- `crates/ruviform-io-e2e/README.md`・`src/lib.rs` の fixture
  一覧 doc の整合更新。
- `evidence/rf305/wav-golden-fault-report.md` — 実コマンド・
  実値・OS/FS 実測範囲・NotRun/Unsupported・旧 API 証拠・DAG
  確認の記録。
- 本 doc チェーン（spec/design/test-spec/plan/task/impl）。

### Out of Scope

- `ruviform-io-wav`/`ruviform-media-io`/`ruviform-io-file` 等の
  公開 API・内部実装の変更（真の defect 発見時のみ io-wav src
  の最小修正を許容）。
- trybuild・compiletest 等の新規 dev-dependency 追加。
- 実 4GiB ファイルの生成（境界は破棄 sink で検証）。
- `PublishEffect::Published`/`Indeterminate`・DirSync 後失敗・
  `TempState::Unknown` の実 OS 再現（誘発不能 — `ruviform-io-file`
  の `#[cfg(test)]` MockFileOps 試験が正本）。
- Consumer repo（audio-lens/synvelis）への編集・実測、
  macOS/Linux 実機・Network/Cloud FS・性能計測。
- `crates/ruviform-ui-test`・`crates/ruviform-observability`・
  `crates/ruviform-ui-*`（別 dispatch 稼働中のため不可触）。

## Requirements

- 全 fixture は公開 API のみを消費し、backend 型・内部 mock・
  `#[doc(hidden)]` の `vfs` module に触れない。
- Byte Golden の期待値はテスト側で RIFF/WAVE 仕様と宣言済み
  変換 policy から独立に組み立てる。crate 内部 helper
  （`wav::header`/`convert::write_sample` 等 private 実装）は
  参照しない — 同一 bug の二重記述を避けるため期待値は literal
  または独立した符号化で与える。
- encoder 経路の edge 値（`NaN`/`±Inf`/`|x|>1`/`.5` LSB 丁度の
  丸め）は `f32 → wire` の宣言 policy（NaN→0・clamp・round half
  away from zero・`F32` verbatim）を独立に符号化した期待 byte
  列で pin する。
- `4GiB` 境界は write を受理するが内容を保持しない sink で
  cumulative `data_bytes` を `u32::MAX` 超過させ、
  `ErrorKind::InvalidData`・receipt 無しを assert する
  （実ファイル 4GiB 生成はしない）。
- Fault fixture は実 FS で誘発可能な class のみを対象とし、
  誘発不能な分類（`Published`/`Indeterminate` の失敗 effect・
  DirSync 後失敗・`TempState::Unknown`）は偽装せず `NotRun`/
  `Unsupported` として記録する。
- publish outcome の分類規則は「`NotPublished` のみが未公開を
  意味し、`Ok` receipt は `Published`、それ以外の effect
  （`Published`/`Indeterminate`/将来 variant）は未公開と
  断定しない」ことを fixture コードと試験で pin する。
- `#[non_exhaustive]` enum の variant 追加・API 変更は行わない。
  追加する dev-dependency はない（既存 dev-deps のみ消費）。

## Acceptance Criteria

- [ ] `cargo test -p ruviform-io-e2e` が新規 fixture を含め全
      pass（実施 OS = Windows NTFS の実測結果を記録）。
- [ ] ZeroFrames・PCM payload・RIFF chunk 構造
      （`WAVEFORMATEX`/`WAVEFORMATEXTENSIBLE`）・4GiB 境界・
      NaN/Inf/Precision の byte golden が存在し、独立 decode
      （symphonia）と照合される。
- [ ] Finalize 失敗・Cancel（seal 後 abort・muxer Drop）・
      DiskFull 相当（sink `StorageFull`）で publish されず、
      destination 無変更・temp 非残存が assert される。
- [ ] publish outcome が `Published`/`NotPublished`/
      `Indeterminate` を明示区別し、`Published`/`Indeterminate`
      を未公開と誤報しない分類規則が pin される（実測可能分は
      実測、誘発不能分は `NotRun` 記録）。
- [ ] 旧 `StagedOutput`/media-io 旧 `PublishOptions` の
      workspace コード・rustdoc・crate README 参照がゼロである
      ことを確認・記録する（`docs/` 歴史資料は対象外）。
- [ ] `MediaSource`/`MediaSink`/Backend API/Feature・依存 DAG
      維持を `cargo tree` 等で確認・記録する
      （`ruviform-media-io` は Domain Core3 のみ）。
- [ ] `mise run fmt-check` / `check-io-e2e` / `lint-io-e2e` /
      `test-io-e2e` / `check` / `lint` / `test` / `test-rt` が
      pass。

## Deferred Design Concerns

- `PublishEffect::Published`/`Indeterminate`・DirSync 後失敗・
  `TempState::Unknown` の実 OS 再現 | 実 OS で deterministic に
  誘発不能・公開 fault-injection API なし | fault-injection 公開
  面の将来追加 | mock 正本依存の分類盲点が残る。
- 実 4GiB ファイル生成を伴う publish 試験 | 実コスト・disk 使用量
  に見合わない | sparse file/FS 層 fixture が将来必要になった
  場合 | 実 FS での 4GiB 超境界挙動が未測定のまま。
- macOS/Linux 実機・Network/Cloud FS | 本環境は Windows のみ |
  実機 dispatch | OS/FS 差異が未検出のまま残る。

## Sources and Relations

- 正本: Issue #305 本文（作業範囲・Acceptance・横断レビュー
  追補の推奨 E2E 順序）。
- 設計基準: Issue #287（IO-05）対応表・順序・非互換方針。
  前提実装: Issue #295（`StagedFile` 型状態）・#296（OS Atomic
  Publish・Durability・Readback）— 両者 Closed。
- 前段 doc chain: `docs/{specs,designs,plans,test-specs,tasks,
  impl/ir}/issue-305-*`（T1 done）— 本 slice はその残 fixture
  行を担う `refines` 関係。
- fixture 規約の先例: `docs/{specs,designs,plans,test-specs,
  tasks,impl/ir}/issue-306-io-e2e-fault-fuzz.md` 系（slice-2、
  merged）— 実 OS 誘発可能 class のみ fixture 化・誘発不能分は
  `NotRun`/`Unsupported` 記録の確立済み規約。
- 既存カバレッジ（重複しない増分の根拠）:
  `crates/ruviform-io-e2e/tests/wav_staged.rs`（finalize→seal→
  readback→publish→reopen・truncated・second-finalize）、
  `crates/ruviform-io-wav/tests/contract.rs`（capability・
  in-memory sink fault・staged publish 各種）、
  `crates/ruviform-io-wav/tests/roundtrip.rs`（wire format
  roundtrip・NaN/Inf policy の decode 照合）、
  `crates/ruviform-io-e2e/tests/staged_lifecycle.rs`・
  `fault_boundary.rs`（publish outcome 分類の先例）。
