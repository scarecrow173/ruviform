---
id: "IMPL-034eqXfRz3favrQAtrps6"
type: "impl"
status: "completed"
title: "Sync docs and verify Issue 20"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/20"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/sync-docs-and-verify-issue-20.md"
    modified:
      - type: "file"
        path: "README.md"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "docs/designs/ruviform-architecture-design-baseline.md"
      - type: "file"
        path: "docs/specs/implement-media-io-abstraction-and-first-backend-poc.md"
      - type: "file"
        path: "docs/plans/issue-20-i-o-poc.md"
      - type: "file"
        path: "docs/tasks/implement-media-io-abstraction.md"
      - type: "file"
        path: "docs/tasks/implement-io-symphonia-backend-poc.md"
      - type: "file"
        path: "docs/tasks/sync-docs-and-verify-issue-20.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/media-io-abstraction-and-backend-poc.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/sync-docs-and-verify-issue-20.md"
  implemented-by: []
  depends-on:
    - "../../plans/issue-20-i-o-poc.md"
    - "implement-media-io-abstraction.md"
    - "implement-io-symphonia-backend-poc.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../../AGENTS.md"
    - "../../plans/issue-20-i-o-poc.md"
    - "../../specs/implement-media-io-abstraction-and-first-backend-poc.md"
    - "../../adr/media-io-backend-abstraction.md"
    - "../../adr/licensing-and-distribution-policy.md"
    - "../../adr/realtime-contract-and-verification-policy.md"
    - "../../adr/public-enum-non-exhaustive-policy.md"
    - "../../designs/ruviform-architecture-design-baseline.md"
  refines: []
  refined-by: []
  derives-from:
    - "../../plans/issue-20-i-o-poc.md"
    - "../../specs/implement-media-io-abstraction-and-first-backend-poc.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../../test-specs/media-io-abstraction-and-backend-poc.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Sync docs and verify Issue 20

## Summary

task `TASK-034Jgi4vWCuaMUu3POps1c`（plan `PLAN-034JfKRCFCu8oJr5iCe6AV` Phase 3・Issue #20 タスク3）として、タスク 1・2 の実装着地（`ruviform-media-io`・`ruviform-io-symphonia` の 2 クレート追加）に伴うドキュメント同期と総合検証スイート V-1〜V-6 を実施した。`README.md` / `AGENTS.md` / 設計ベースライン L267（`AudioBuffer` → `AudioFrame` 記述）/ spec・plan・task 群の front matter / IR 索引を実態へ同期し、検証スイートは全項目 PASS（AC1-AC11 突合済み）。doc 監査（`audit_docs.js --type all`）で検出した front matter 相互参照・索引不整合 10 件も併せて解消し 0 件とした。独立レビューサイクル（Reviewer A + B → Validator → Judge）を実施し、2 件の blocker（`MediaSource::byte_len` の dead 拡張点・非標準チャンネルマスクでの無言チャンネル逆転）を修正 wave で解消した（詳細は Review Cycle 節）。

## Context

- 実施タスク: [../../tasks/sync-docs-and-verify-issue-20.md](../../tasks/sync-docs-and-verify-issue-20.md)（`TASK-034Jgi4vWCuaMUu3POps1c`）
- 上流: [plan `PLAN-034JfKRCFCu8oJr5iCe6AV` Phase 3](../../plans/issue-20-i-o-poc.md)、[spec `SPEC-034JcaMXubILS7j1FX5qob`](../../specs/implement-media-io-abstraction-and-first-backend-poc.md)（AC1-AC11 突合対象）、[test spec](../../test-specs/media-io-abstraction-and-backend-poc.md)
- 前提: タスク 1（[implement-media-io-abstraction.md](implement-media-io-abstraction.md)・`IMPL-034JxNThv4gsW2IpbkYmK4`）とタスク 2（[implement-io-symphonia-backend-poc.md](implement-io-symphonia-backend-poc.md)・`IMPL-034K7dWqYmSaPhOnIaPc02`）の完了

## Implementation

### Changes

- `Cargo.toml` — 変更不要を確認（メンバーコメントはタスク 1・2 時点で 2 クレート追加・path 依存配線の実態へ同期済み）
- `README.md` — 冒頭説明・実在リスト・未確定リスト（io-symphonia PoC の対応範囲は正式採否外）・RT 契約段落（media-io / io-symphonia は全公開 API non-RT only）・`crates/` 説明を 2 クレート追加の実態へ同期
- `AGENTS.md` — ワークスペース構成段落を同期（members +2、media-io → domain 3 クレート・io-symphonia → media-io + media-core + audio-core の path 依存、非 RT 専用、symphonia 0.6 default features のみ・`Muxer`/`Encoder` 非対応の記載）。`test-rt` 行の stale な「RT テスト未実装」記述を Issue #18 実装済みの実態へ修正
- `docs/designs/ruviform-architecture-design-baseline.md` L267 — 「抽象的な流れ」の `Decoder` → `AudioBuffer` / `VideoFrame` を `AudioFrame`（`AudioBuffer` + PTS/duration を束ねる media-io 側の出力型）/ `VideoFrame` へ同期更新（AC10）
- `docs/specs/implement-media-io-abstraction-and-first-backend-poc.md` — `status: implemented`、`implemented-by` に plan リンク（双方向整合）、`derived-by` に derives-from 元の test-spec + 3 タスクを追加（相互参照整合）
- `docs/plans/issue-20-i-o-poc.md` — `status: completed`、`changes` を実ファイル一覧へ同期（media-io の `source.rs`・io-symphonia の `error.rs`/`map.rs`/`source.rs` 追加、存在しない `capability.rs` 除去、3 IR を added へ、索引・test-spec を modified へ）
- `docs/tasks/implement-media-io-abstraction.md` / `implement-io-symphonia-backend-poc.md` / `sync-docs-and-verify-issue-20.md` — `implemented-by` に対応 IR パスをバックフィル（broken forward reference 解消）。本タスク doc は `status: in-progress`・担当項目チェックを更新（レビュー完了後にオーケストレータが `done` 化）
- `docs/impl/ir/sync-docs-and-verify-issue-20.md` — 本記録（新規）
- `docs/impl/ir/README.md` — 索引に 3 行追加（タスク 1・2 の IR と本 IR。ファイル名ソート順）
- `docs/specs/README.md` / `docs/plans/README.md` / `docs/tasks/README.md` — 索引 status を実態へ同期（implemented / completed / done / in-progress）
- `docs/test-specs/media-io-abstraction-and-backend-poc.md` — `verifies` から plan リンクを除去（監査の blocking 指摘: verifies ターゲットは spec/design/adr のみ有効）

### Decisions During Implementation

- **ベースライン更新は用語の最小同期 + 短い同位定義**: 設計ベースラインは後続 Issue 番号を本文に書かない文体（Issue #1 由来の記述のみ）のため、`AudioBuffer` → `AudioFrame` の置換に「`AudioBuffer` + PTS/duration を束ねる media-io 側の出力型」の同位定義のみ添え、Issue 番号は記さなかった
- **IR 索引へ本 IR 行も追加**: 依頼は 2 行追加であったが、索引は実在ファイルの正本であるため本タスク IR（in-progress→completed）の行も同時に追加し、ファイル名ソート規則（既存行の並びはファイル名昇順と確認）に従って配置した
- **監査指摘は上流側で整合**: `derives-from` 相互参照 4 件は派生元（task/test-spec）の削除ではなく spec 側 `derived-by` への追加で解消（派生関係は事実として存在するため）。`verifies` の plan リンクのみスキーマ違反のため除去し、plan 側 `verified-by` は plan type の required relation（test-spec 必須）として維持
- **`unsafe` grep の評価**: `crates/` 両クレートの `src/`・`tests/` は 0 件。io-symphonia `Cargo.toml` の 3 件ヒットは symphonia の `forbid(unsafe_code)` を説明する依存追加記録コメントであり、コードではない
- **RT contract カバレッジ観察**: 全公開 trait/型/fn/method に `**RT contract:** non-RT only` を確認（89 箇所）。未ラベルの公開アイテムはフラグ型の関連定数 8 件のみ（`PacketFlags::{NONE,KEYFRAME,CORRUPT}`・`Features::{NONE,DEMUX,MUX,DECODE,ENCODE}`）で、型レベルのラベルでカバーされる粒度判断 — 独立レビューの判定対象として記録

## Related Experiments

### Adopted

なし（実験ログ運用なし）

### Rejected

なし

## Validation

実施環境: Windows x64・Git Bash・Rust 1.99.0（mise pin）。検証スイート実績（2026-10-21 実行）:

| Check | コマンド | 結果 |
| --- | --- | --- |
| V-1 test | `mise exec -- cargo test -p ruviform-media-io` | PASS・40 unit + 19 doctest・0 失敗（AC2/AC7/AC11） |
| V-1 lint | `mise exec -- cargo clippy -p ruviform-media-io --all-targets -- -D warnings` | PASS・0 warnings |
| V-1 check | `mise exec -- cargo check -p ruviform-media-io` | PASS（バックエンド非依存・AC2） |
| V-1 tree | `mise exec -- cargo tree -p ruviform-media-io` | 依存辺は `ruviform-media-core` / `ruviform-audio-core` / `ruviform-video-core` のみ。`ruviform-io-*` 辺・外部依存・symphonia 辺なし（AC2/AC4） |
| V-2 test | `mise exec -- cargo test -p ruviform-io-symphonia` | PASS・11 unit + 7 e2e（`tests/wav_decode_poc.rs`）+ 5 doctest・0 失敗（AC5/AC6） |
| V-2 lint | `mise exec -- cargo clippy -p ruviform-io-symphonia --all-targets -- -D warnings` | PASS・0 warnings |
| 型純度 | `grep -rni "symphonia" crates/ruviform-media-io/src` | 0 件（AC1） |
| 型純度 | `grep -rn "pub use" crates/ruviform-io-symphonia/src` | `SymphoniaDecoder` / `SymphoniaDemuxer` のみ（自クレート型・外部型再エクスポートなし。AC1） |
| panic 経路 | `grep -rn "unimplemented!\|unreachable!\|panic!" crates/ruviform-io-symphonia/src` | 0 件（AC6） |
| lib.rs 参照 | `grep -rni "symphonia" crates/ruviform-io-symphonia/src/lib.rs` | private `use symphonia::…` import・crate doc・自クレート型名のみ（公開署名への外部型露出なし。AC1） |
| doc | `mise exec -- cargo doc -p ruviform-media-io -p ruviform-io-symphonia --no-deps` | 生成成功・警告なし。media-io は trait 5（`Demuxer`/`Muxer`/`Decoder`/`Encoder` 分離 + `MediaSource`）・enum 4・struct 8。外部型非露出（AC1/AC3） |
| V-3 fmt/check/lint/test | `mise run fmt-check` / `check` / `lint` / `test` | 全て PASS・0 warnings。test はワークスペース計 220 件・0 失敗（AC8） |
| V-4 test-rt | `mise run test-rt` | 意図した `rt_` テスト 10 件のみ選択（audio-core 3・media-core 4・video-core 3、全 pass）。新規 2 クレートのテストは 0 件選択（media-io 40・io-symphonia 11+7 全てフィルタアウト・`rt_` 部分文字列を含むテスト名なし） |
| V-5 bench | `mise run bench` | PASS・コンパイル成功・全テスト ignored・ベンチマーク未実装のまま no-op 維持 |
| V-6 metadata | `mise exec -- cargo metadata --format-version 1 --no-deps` | 全 8 パッケージ `publish: []`。依存辺: media-io→audio-core/media-core/video-core、io-symphonia→media-io/media-core/audio-core/symphonia（外部依存はバックエンドクレートのみ・設計どおり） |
| unsafe | `grep -rn "unsafe" crates/ruviform-media-io crates/ruviform-io-symphonia` | コード内 0 件（ヒットは io-symphonia `Cargo.toml` の依存追加記録コメント 3 件のみ・AC8） |
| enum | `grep -rn "pub enum" crates/ruviform-media-io crates/ruviform-io-symphonia` | 4 enum（`CodecId`/`CodecFormat`/`ErrorKind`/`Frame`）全て enum レベル `#[non_exhaustive]`。io-symphonia に公開 enum なし（AC9） |
| 非改変 | `git status crates/ruviform-audio-core crates/ruviform-video-core crates/ruviform-media-core` | 非改変確認（AC7） |
| prek | `mise exec -- prek run --all-files` | 全フック Passed・ファイル変更不要 |
| doc 監査 | `node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` | 初回 10 件検出 → 修正後 0 件（索引 status 不整合 5・`derived-by` 相互参照 4・`verifies` 不正ターゲット 1） |

AC 突合: AC1（外部型非露出）✓ / AC2（media-io 単独 check・依存辺）✓ / AC3（4 trait 分離）✓ / AC4（分離クレート + opt-in・media-io 外部依存なし）✓ / AC5（in-memory WAV e2e）✓ / AC6（両機構の非対応表現・panic なし）✓ / AC7（`AudioFrame` PTS/duration・domain クレート非改変）✓ / AC8（全検証グリーン・unsafe 0）✓ / AC9（RT ラベル + `#[non_exhaustive]`。関連定数 8 件の粒度はレビュー判定対象として記録）✓ / AC10（ドキュメント同期）✓ / AC11（codec 記述型 3 責務・単体テスト）✓

## Review Cycle

review-protocol（Reviewer A 協調的 spec 適合 + Reviewer B 敵対的欠陥探索 → Validator 実証検証 → Judge 最終判定）を独立サブエージェントで実施。Issue Candidates 計 16 件（A: 7・B: 9）に対する Judge 判定は以下。

### Blocker（PR 前修正必須 → 本 wave で修正済み）

| 候補 | 内容 | 修正 |
| --- | --- | --- |
| B1（+B8/A1 包摂） | `MediaSource::byte_len` が blanket impl `impl<T: Read+Seek+Send>` との coherence 衝突で手動 impl 不可能（dead 拡張点）→ `SourceAdapter::byte_len` 恒常 `None` → Ogg の末尾走査・二分 seek・duration 列挙が常に失敗 | 境界 trait から `byte_len` を削除（案 b 採用 — `Seek::stream_len` は Rust 1.87 安定化で MSRV 1.85 超過のため `stream_position`→`seek(End)`→復元の手動 probe を `SourceAdapter` 構築時に 1 回実行し `{seekable, len}` をキャッシュ）。真に非シーク可能なソースは `SeekError::Unseekable` → `Unsupported` に正規化され doc 契約と一致。単体テスト 3 件追加 |
| B2 | `Positioned` マスクで unnamed ビット 6–8（FLC/FRC/RC）と named ビット 9–10（SL/SR）混在時（6.1 等）に plane → channel スロットが逆転し無言スワップ | `channel_layout` / `positioned_layout` が `(ChannelLayout, Vec<usize>)`（plane → 境界スロットの permutation）を返すよう変更し、`decoder.rs` のコピー先スロットを並替。6.1 マスク permutation `[0,1,2,3,6,4,5]` の単体テスト追加 |

### 同梱した安価修正（report-only 判定だが wave に含有）

- B6/A7 — `next_ts` anchor を `saturating_add` → `checked_add` → `ErrorKind::TimeOverflow`（doc が謳う契約と整合）
- A3 — `SourceAdapter` の dead 実装 `impl Write` 除去（symphonia `MediaSource` は `Read+Seek+Send+Sync` のみ要求）
- A5 — `error.rs` の `From` impl レベル `**RT contract:**` doc 行を除去（domain クレート慣行・型/関数/メソッドのみ対象に均一化）

### report-only / by-design（後続課題として記録）

- **followup**: B3/B9（symphonia-format-riff 上流起因: `data` len=0xFFFFFFFF の末尾欠損 + `UnexpectedEos` 誤分類、block_align 非倍数の残端末静かな欠損）/ B4（Ogg gapless trim 不運搬 — spec Deferred、末尾フレームにパディング混入の観測可能差異）/ B5（チェーン Ogg `ResetRequired` → `InvalidData` 打ち切り — `streams()` スナップショットモデルとの構造的衝突。crate doc に制限明記済み）/ B7（`pending` キュー無制限 — caller 運用契約内）/ A6（`AudioFrame` Clone 非対称 — audio-core 非改変契約の継承制約）/ B6/A7 残件（PTS 欠落補外の責務境界）
- **by-design**: A2（capabilities() under-claim — 正直な能力表現として意図的）/ A4（`drain()` 追加 — send/receive ポンプ選択の必然）/ B7

### 修正 wave の検証

- `cargo clippy --workspace --all-targets -- -D warnings` — 0 warnings
- `cargo test -p ruviform-io-symphonia -p ruviform-media-io` — 15 + 7（e2e）+ 40 + doctest 全 pass
- 総合検証スイートの再実行結果は上記 Validation 節の通り全項目 PASS

## Risks

- `mise run test-rt` の substring フィルタは精度限界あり（Issue #18 IR と同じ既知制約）。現時点では新規 2 クレートに `rt_` を内包するテスト名がなく誤選択は観測されていないが、将来の命名で再発しうる
- フラグ型関連定数 8 件の `**RT contract:**` 未付与は型レベルラベルでカバーする粒度判断。レビューでは blocker 判定されなかったが、個別付与が求められた場合は追加のみの non-breaking 対応
- レビュー followup 項目（上流 symphonia 不具合 2 件・チェーン Ogg・gapless trim・キュー有界化・PTS 補外境界・`AudioFrame` Clone）は後続 Issue 起票候補 — PR 本文に転記

## Follow-ups

- チェーン Ogg / 途中リンクの stream 再列挙（`ResetRequired` 対応。`streams()` スナップショットモデルの設計論点を含む）
- symphonia-format-riff 上流不具合の起票検討（長さ不明 WAV の末尾欠損 + EOF 誤分類、block_align 非倍数残端末の静かな欠損）
- gapless trim（`trim_start`/`trim_end`）の `Packet` 運搬先決定 — PoC で末尾パディング混入を観測
- `Decoder` pending キューの有界化・backpressure 要否検討
- `AudioFrame` の `Clone`/`PartialEq` 要否（audio-core 側の別論点）
- `test-rt` フィルタの厳密化（プレフィックスマッチまたはレーン分離）は Issue #18 と共通の課題として RT 検証 Issue 側で検討
