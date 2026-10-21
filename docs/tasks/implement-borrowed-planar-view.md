---
id: "TASK-034JgsGmcPdPRoeRTvicjW"
type: "task"
status: "done"
title: "独立 per-channel planar バッファのゼロコピー借用 view/newtype の実装"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/29"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-audio-core/src/planar.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-audio-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/processor.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lib.rs"
      - type: "file"
        path: "docs/plans/implement-plugin-foundation.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-plugin-foundation.md"
  implemented-by:
    - "../impl/ir/implement-borrowed-planar-view.md"
  depends-on:
    - "../plans/implement-plugin-foundation.md"
    - "create-plugin-core-foundation.md"
  blocks:
    - "poc-clap-binding-adapter.md"
    - "poc-vst3-binding-adapter.md"
    - "sync-docs-and-verify-plugin-foundation.md"
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../../crates/ruviform-audio-core/src/lib.rs"
    - "../../crates/ruviform-audio-core/src/buffer.rs"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-plugin-foundation.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/plugin-foundation.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/22"
  defers: []
  deferred-by: []
---

# 独立 per-channel planar バッファのゼロコピー借用 view/newtype の実装

## Work

plan `PLAN-034JcvpHIlt5JmqvgOquvI` の T2（Task Breakdown の `implement-planar-borrow-view` 仮称行に対応）。プラグイン API が供給する独立 per-channel planar バッファを、所有権移転なし・変換ゼロでラップする借用 view/newtype を実装する。

- [x] 配置先を確定する。**lean: `ruviform-audio-core`**（`AudioBufferView`/`AudioBufferViewMut` と同じ「音声バッファ借用形状」の責務、video-core の `Plane`/`PlaneMut` 先例と整合。ホスト側・デバイス境界等 plugin 以外の per-channel 非連続バッファ供給にも再利用可能）。fallback: `ruviform-plugin-core/src/buffer.rs`（借用形状がプラグイン ABI 境界固有と判明した場合）。確定結果と根拠を crate doc と plan 追記に反映する
- [x] 独立したチャンネルごとのポインタ/スライス列を所有権移転なしでラップする view/newtype を実装（読み取り用 `&[f32]` 列・書き込み用 `&mut [f32]` 列の双方。既存 `AudioBuffer` 系はチャンネルメジャー連続ストレージ前提のため拡張ではなく新設。`&'a mut [&'a mut [f32]]` 相当の型形状で `'a` を process コールバックスコープに束縛）。canonical planar f32 と同一レイアウトで変換回数ゼロを API 設計で担保。**コンストラクタはアダプタ側が activate/prepare 時に事前確保するチャンネルリスト用スクラッチを受け取れる形状とし、view 構築自体が alloc しないこと**（T3/T4 のスクラッチ事前確保と対応）
- [x] in-place 処理への対応方針を本タスクの評価事項として決定する: ホストが入出力に同一バッファを渡す場合（CLAP `in_place_pair` / clack `ChannelPair::InPlace` 相当）、アダプタはエイリアスした同一領域上に `&[f32]` と `&mut [f32]` を同時に構築してはならない。境界でポインタ一致を検出し「専用の in-place view 分岐」または「スクラッチ経路」のどちらを採用するかを評価し、選択と根拠を方式 ADR（T5: [decide-plugin-binding-method-adr.md](decide-plugin-binding-method-adr.md)）の判断材料として記録する
- [x] view のチャンネル契約を API に固定する: 1 つの view 内の全チャンネルスライスは均一なフレーム長を共有し（異種長は構築時に拒否）、チャンネル順はバスのチャンネルインデックス順に一致し、view のチャンネル数は activate 時にネゴシエートされたバス構成と整合する（spec Requirements 参照）
- [x] f64 型形状予約を閉塞しない: view の型形状は 64-bit サンプル経路の予約を収容できること（ジェネリック化または文書化された拡張経路。具象 f32 を先行してよいが予約を保持する — spec Requirements・Out of Scope の f64 予約方針に対応）
- [x] エッジ契約を API ドキュメントに明記する: ゼロチャンネル・ゼロフレーム・非アクティブバスの view は空のチャンネルリストとして構築可能だがサンプルを公開しない。null ポインタ上に view を構築しない
- [x] 「借用中にホスト側が解放・再配置しない」前提契約と `**RT contract:**` ラベルを API ドキュメントに明記。借用がコールバックを脱退できないことを `compile_fail` doctest で固定（型シグネチャのライフタイム束縛によるコンパイル時担保。期待するエラー種別は borrow-escapes-scope 系であることを明記し、正常系 doctest・単体テストと対で整備する）
- [x] 独立チャンネルバッファから構築した view が各チャンネルの読み書きを提供する単体テスト、`rt_` プレフィックスの no-alloc 契約テストを追加。Issue #22 の alloc 検証基盤が未整備の場合は `rt_` テスト + レビュー可能なドキュメント契約で代替し、その旨を crate doc と plan に明記する（spec 受け入れ基準準拠）

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-plugin-foundation.md](../plans/implement-plugin-foundation.md) — `PLAN-034JcvpHIlt5JmqvgOquvI`（approved）。T2 チェックリスト・File Map（配置 lean/fallback）・V-2 の正本
- [../specs/implement-plugin-foundation.md](../specs/implement-plugin-foundation.md) — `SPEC-034JcgdAKPNeh1YCFyprzh`。ゼロコピー借用の受け入れ基準
- [../test-specs/plugin-foundation.md](../test-specs/plugin-foundation.md) — `TSPEC-034Jg6YSP6CLjJHnFGG5oL`。借用ライフタイム束縛・in-place エイリアス禁止・エッジ契約の不変条件
- [../adr/realtime-contract-and-verification-policy.md](../adr/realtime-contract-and-verification-policy.md) — RT ADR（accepted）。process 経路 no-alloc・`**RT contract:**` 規則

## Dependencies

- 前提タスク: [create-plugin-core-foundation.md](create-plugin-core-foundation.md)（`TASK-034JgsFV6CWAjSnPuekBgs`）— view の API 形状は plugin-core の process モデル・規格中立型と整合させて設計するため core 先行（audio-core 配置時も T1 完了後に確定する順序）
- 後続をブロック: [poc-clap-binding-adapter.md](poc-clap-binding-adapter.md)・[poc-vst3-binding-adapter.md](poc-vst3-binding-adapter.md) — 両 PoC は規格中立型と借用 view の上に構築する
- Issue #22 の RT alloc 検証基盤は未整備前提。`rt_` テスト + ドキュメント契約で代替する

## Affected Files

- `crates/ruviform-audio-core/src/planar.rs` — 新規（lean 配置時）
- `crates/ruviform-audio-core/src/lib.rs` — 再エクスポート・crate doc 追記（lean 配置時）
- `crates/ruviform-plugin-core/src/buffer.rs` — 新規（fallback 配置時のみ）
- `docs/plans/implement-plugin-foundation.md` — 配置確定結果・根拠の追記

## Done When

- [x] 配置先が確定し、根拠が crate doc と plan 追記に反映されている
- [x] view が独立 per-channel `&[f32]`/`&mut [f32]` 列を所有権移転なしでラップし、ライフタイム `'a` が process コールバックスコープに束縛される（f32 経路の変換回数ゼロ）
- [x] view 構築が alloc しない（アダプタ事前確保スクラッチを受け取るコンストラクタ形状）
- [x] in-place 対応（専用 view 分岐 vs スクラッチ経路）の評価結果が方式 ADR 入力として記録されている
- [x] `compile_fail` doctest で借用のスコープ脱退がコンパイル阻止される（borrow-escapes-scope 系）。ゼロチャンネル・ゼロフレーム・非アクティブバス・null ポインタのエッジ契約が API ドキュメントに明記される

## Verification

- [x] `mise exec -- cargo test -p ruviform-audio-core` exits 0（独立チャンネル構築・読み書き・`rt_` テストを含む。fallback で plugin-core 配置の場合は `-p ruviform-plugin-core`）
- [x] `mise exec -- cargo clippy -p ruviform-audio-core --all-targets -- -D warnings` exits 0（局所許可のない `unsafe` が混入しないこと）
- [x] `compile_fail` doctest が借用のスコープ脱退をコンパイル阻止することを `cargo test` の doctest 結果で確認
- [x] canonical planar f32 との変換が f32 経路でゼロ回であることをドキュメントで確認
