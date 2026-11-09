---
id: "DESIGN-034UvWqyRcaTQap2SH5FfT"
type: "design"
status: "approved"
title: "Issue #305 IO E2E slice — WAV Byte Golden・Fault fixture 設計"
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
    - "docs/specs/issue-305-io-e2e-wav-golden-fault.md"
    - "docs/specs/issue-305-media-io-staged-file-cutover.md"
    - "docs/designs/issue-306-io-e2e-fault-fuzz.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-295-io-file-staged-publish.md"
    - "docs/designs/issue-296-io-file-os-backend.md"
    - "docs/designs/ruviform-architecture-design-baseline.md"
    - "docs/adr/issue-191-pcm-wav-output-backend-ruviform-io-wav-crate-mediasink-port-staged-pub.md"
  refines:
    - "docs/designs/issue-305-media-io-staged-file-cutover.md"
  refined-by: []
  derives-from:
    - "docs/specs/issue-305-io-e2e-wav-golden-fault.md"
    - "docs/specs/issue-305-media-io-staged-file-cutover.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #305 IO E2E slice — WAV Byte Golden・Fault fixture 設計

## Context

`wav_staged.rs` は finalize→borrow release→seal→独立 decode→
publish→reopen の順序と truncated/second-finalize reject を既に
pin 済み。`ruviform-io-wav` の contract/roundtrip は in-memory
sink で capability・encoder/muxer 単体・wire roundtrip・NaN/Inf
policy をカバーする。本 slice が埋めるのは **io-e2e 層での
byte-exact golden**（publish 済み destination bytes を仕様から
独立に組み立てた期待値と比較）と **実 FS 上の fault 分類**
（finalize/disk-full/cancel/publish outcome の明示区別）である。
Issue #306 slice-2 が確立した「実 OS で誘発可能な class のみ
fixture 化し、誘発不能分は `NotRun`/`Unsupported` で正直に記録」
の規約をそのまま継承する。

## Scope

### In Scope

- `crates/ruviform-io-e2e/tests/wav_golden.rs`（新規）— byte
  golden fixture。
- `crates/ruviform-io-e2e/tests/wav_fault.rs`（新規）— fault・
  publish outcome fixture。
- `crates/ruviform-io-e2e/README.md`・`src/lib.rs` の fixture
  一覧 doc 更新。
- `evidence/rf305/`・doc チェーン・IR。

### Out of Scope

- 公開 API・内部実装の変更、新規 dev-dependency、実 4GiB ファイル
  生成、誘発不能 fault class の偽装 fixture、Consumer repo・
  macOS/Linux 実機、`ruviform-ui-*`/`ruviform-observability`。

## Components and Boundaries

### Fixture file 構成

| file | 境界 | 内容 |
| --- | --- | --- |
| `tests/wav_golden.rs` | WavEncoder/WavMuxer→`&mut StagedFile`→seal→publish→destination bytes | ZeroFrames header golden（`WAVEFORMATEX`）・extensible header golden（`WAVEFORMATEXTENSIBLE`）・S16 PCM payload byte-exact・wire format 別 edge sample byte golden（U8/S16/S24/S32/F32 の NaN/Inf/clip/`.5` 丸め）・4GiB 境界（破棄 sink）・独立 decode 照合 |
| `tests/wav_fault.rs` | 同上 + publish outcome 分類 | finalize flush 失敗→`Io`+receipt 無し→abort、muxer Drop（best-effort finalize）を完了証拠にしない、sink `StorageFull`→`Io`→abort→dest 無変更、seal 後 abort（Cancel）、publish outcome 三分類の明示区別 |

### 期待値の独立性

- header/golden 期待値はテスト内で RIFF/WAVE 仕様（chunk 順・
  LE field・size backpatch 規則）から手組みする。`wav::header`・
  `convert::write_sample` 等の private helper は呼ばない。
- encoder 経路の edge sample 期待値は宣言済み policy
  （`NaN→0`・`clamp(-1,1)`・`round` half away from zero・
  `F32` verbatim）をテスト側の独立符号化で与える。期待 byte は
  literal または独立計算で記述し、実装出力をそのまま期待値に
  しない。
- 独立 decode 照合は `ruviform_io_symphonia`（demuxer packet
  payload / `with_borrowed_demuxer`）を oracle とし、byte golden
  と decoded payload の両方を assert する。

### Sink 注入の形

- `DevNull`（`Write`+`Seek`+`Send`）: write を受理し位置のみ
  追跡・内容を保持しない。4GiB 境界で cumulative `data_bytes`
  を `u32::MAX` 超過させるためだけに使う（実ファイルを作ら
  ない）。
- `FlushFail<'a>`/`DiskFull<'a>`: `&'a mut StagedFile` へ委譲
  しつつ flush 失敗・累積 write byte 数の capacity 超過で fault
  を注入する薄い wrapper。
  `WavMuxer<S: MediaSink>` の `S` として使い、注入点は muxer→
  sink の公開境界のみ（`StagedFile` 自体・io-file 内部には
  触れない）。fault 後は `into_sink()` で wrapper を回収して
  `&mut StagedFile` 借用を解放し、`abort()`/`seal()` へ進む。

### Publish outcome 分類

- 分類器 `classify(Result<PublishReceipt, PublishFailure>)` を
  fixture 内に定義し、`Ok` → `Published`、`Err` かつ
  `effect()==NotPublished` → `NotPublished`、それ以外の effect
  （`Published`/`Indeterminate`/将来 variant）→
  `NotProvablyUnpublished` に写像する。`#[non_exhaustive]` のため
  wildcard arm が必須であり、未知 variant も安全側 bucket に
  落ちることを契約として pin する。
- 実測可能 class（成功 publish・CreateNew 競合の
  `NotPublished`+`DestinationExists`）のみ実 FS で実行し、
  `Published`/`Indeterminate` の失敗 effect は構築不能
  （`PublishFailure` ctor は `pub(crate)`）かつ実 OS 誘発不能
  のため `NotRun` — `ruviform-io-file` の `#[cfg(test)]`
  MockFileOps 試験を正本として evidence に記録する。

## Data and Control Flow

- **Golden**: `StagedFile::create` → `WavMuxer::new(&mut staged)`
  （header 即時書込）→ `write_packet`/`WavEncoder` packet 列 →
  `finalize()`（receipt assert）→ `into_sink()` で借用解除 →
  `seal()` → `publish(CreateNew, FileSync)` → `std::fs::read`
  で destination bytes を取得 → 期待 byte 列と `assert_eq!` →
  必要に応じ symphonia 独立 decode で照合。
- **encoder 経路 golden**: `make_encoder(&info)` →
  `send_frame(AudioFrame)` → `receive_packet` → 同一 muxer 経路。
  `f32` edge 値の wire byte を file bytes から直接 assert する
  ため、data chunk offset（header 長）を chunk 構造から導出する。
- **Fault**: wrapper sink で `finalize`/`write_packet` を失敗
  させ → `ErrorKind::Io`・`receipt().is_none()` を assert →
  `into_sink` 回収 → `staged.abort()` → destination 不在・
  `children(dir)` 空を assert。
- **Drop fixture**: `WavMuxer` を finalize せず drop（Drop の
  best-effort `finalize_inner` は観測不能）→ `staged.seal()` →
  `open_readback` で独立 decode が成功し得ることを確認 →
  それでも `abort()` で publish しないことを assert
  （「decode 可能に見える bytes ≠ 公開許可」）。

## Non-Functional Constraints

- **非 RT**: 全 fixture は blocking FS/Parser I/O。`rt_*` 命名・
  RT lane への混入なし。
- **DAG**: io-e2e は葉側 consumer のまま。dev-deps 追加なし、
  production crate の feature 集合・依存方向を変えない。
- **MSRV/license**: workspace 継承のまま。新規 code は MSRV
  1.89 互換の std/既存 API のみ。
- **実行時間**: 4GiB 境界は破棄 sink への ~4GiB packet 転送で
  秒オーダー。他は小 fixture のみ。
- **unsafe**: 使用しない（`#[forbid(unsafe_code)]` の lib target
  方針と整合）。

## Risks and Trade-offs

- **期待値の誤記**: RIFF field・wire 変換の手組み期待値が実装と
  ずれる可能性 → test 実行で検出される。ずれた場合は RIFF/WAVE
  仕様と宣言 policy に照らして期待値側を修正する（実装出力を
  鵜呑みにしない）。
- **4GiB 試験のメモリ/時間**: packet `data` は実 `Vec<u8>` が
  必要だが、64MiB chunk を逐次生成・破棄し単一巨大 alloc を
  避ける。実測で問題があれば chunk サイズを調整する。
- **wrapper sink の `Send`/`Seek` 実装漏れ**: `MediaSink` は
  `Write + Seek + Send` を要求。`&mut StagedFile` は `Send`
  （io-file が `Send` 保証）であり wrapper は自動的に `Send`。
  `Seek` は委譲のみで finalize を呼ばない試験では未使用だが
  trait bound のため実装する。
- **Drop fixture の環境依存**: best-effort finalize が実 FS で
  失敗する環境では readback decode assert が落ち得る →
  Windows NTFS 実測で確認し、他 OS は `NotRun` 記録。

## Deferred Design Concerns

- `Published`/`Indeterminate` 失敗 effect・DirSync 後失敗・
  `TempState::Unknown` の合成層検証 | 実 OS 誘発不能・公開 fault
  API なし | fault-injection 公開面の将来追加 | mock 正本依存の
  分類盲点。
- 実 4GiB ファイル生成を伴う publish 試験 | 実コスト | sparse/
  FS 層 fixture の将来追加 | 実 FS 境界挙動の未測定。

## Verification Notes

- [ ] `mise run fmt-check` / `check-io-e2e` / `lint-io-e2e` /
      `test-io-e2e` pass
- [ ] `mise run check` / `lint` / `test`（workspace 非回帰）・
      `test-rt` pass
- [ ] evidence report に OS/FS 実測・NotRun/Unsupported・
      旧 API 証拠・DAG 確認を記録
