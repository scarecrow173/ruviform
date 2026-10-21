---
id: "TASK-034JgsI5J4HU4qcoEBQfDx"
type: "task"
status: "done"
title: "薄い CLAP/VST3 規格アダプタの完成"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/29"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/implement-plugin-adapters.md"
    modified:
      - type: "file"
        path: "crates/ruviform-plugin-core/src/event.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/adapt.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-clap/src/plugin.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/block.rs"
      - type: "file"
        path: "crates/ruviform-plugin-vst3/src/component.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-plugin-foundation.md"
  implemented-by:
    - "../impl/ir/implement-plugin-adapters.md"
  depends-on:
    - "../plans/implement-plugin-foundation.md"
    - "decide-plugin-binding-method-adr.md"
  blocks:
    - "sync-docs-and-verify-plugin-foundation.md"
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../../Cargo.toml"
    - "../../Cargo.lock"
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

# 薄い CLAP/VST3 規格アダプタの完成

## Work

plan `PLAN-034JcvpHIlt5JmqvgOquvI` の T6（Task Breakdown の `implement-plugin-standard-adapters` 仮称行に対応）。方式 ADR（T5）の決定に従い、`ruviform-plugin-clap` / `ruviform-plugin-vst3` の薄い規格アダプタを完成させる。T3/T4 の PoC で成立したエントリポイントと借用経路を土台に、規格準拠のアダプタとしての全責務を実装する。

- [x] `ruviform-plugin-clap` / `ruviform-plugin-vst3` を完成させる: 規格エントリポイント、ABI ↔ 共通基盤型（バッファ・時刻・パラメータ・イベント）の境界限定一方向翻訳、規格の状態遷移を共通ライフサイクル状態機械へ写像（規格ごとの遷移規則対応を単体テストで検証 — spec Requirements の遷移表要件: `processing → activate` の処理停止遷移・activate ↔ processing の再アクティベート・init/activate 失敗時の遷移先・deactivate 前提の destroy・ホストプロトコル違反の定義済み拒否を含む）。**アダプタは plugin-core が所有する処理契約を実装・駆動する側であり、契約自体は決して定義しない**（T1・spec In Scope 参照）
- [x] 境界バリデーションを実装する（spec Requirements）: `numSamples`（0 以下・prepare 済み最大フレーム数超過）、`numChannels`（activate 時スクラッチ容量超過）、`symbolicSampleSize`/`kSample64`（f64 入力は f32 へ再解釈せず定義済みの拒否/スキップ）、チャンネルポインタのアライメント（f32 は 4 バイト）・null ポインタを検証し、失敗時は定義済みの結果（process 呼び出しの拒否・バッファスキップ・エラーステータス）を返す。未定義動作・無言のチャンネルインデックスずらしを生じさせない
- [x] in-place 処理に方式 ADR の決定方式を適用し、エイリアス検出は per-channel ポインタ対比較で行う（部分エイリアス `out[i]==in[j]`・出力チャンネルポインタ重複を検出。同一領域上の `&[f32]`+`&mut [f32]` 同時構築・`&mut [f32]` 同士の重複を作らない）。スクラッチ経路のデータバウンスバッファを採る場合は activate/prepare 時に事前確保する per-instance 状態とする（`static`/`thread_local` 共有は禁止）
- [x] 外部型非露出を監査する（公開 API シグネチャに `clack`/`vst3`/`clap-sys` 等の binding クレート型・エラー・feature 名が現れないこと。`use` 行・re-export による短縮名での露出を含め、grep と `cargo doc` 公開面で確認 — V-6 参照）
- [x] 両クレートの `mise exec -- cargo test -p ruviform-plugin-clap -p ruviform-plugin-vst3` と `mise exec -- cargo clippy -p ruviform-plugin-clap -p ruviform-plugin-vst3 --all-targets -- -D warnings` を実行し全合格を確認

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-plugin-foundation.md](../plans/implement-plugin-foundation.md) — `PLAN-034JcvpHIlt5JmqvgOquvI`（approved）。T6 チェックリスト・V-6 の正本
- [../specs/implement-plugin-foundation.md](../specs/implement-plugin-foundation.md) — `SPEC-034JcgdAKPNeh1YCFyprzh`。アダプタ責務・境界バリデーション・状態写像・外部型非露出の要件正本
- [../test-specs/plugin-foundation.md](../test-specs/plugin-foundation.md) — `TSPEC-034Jg6YSP6CLjJHnFGG5oL`。契約所有の一方向性・エイリアス禁止・unsafe 局所化の不変条件
- `docs/adr/<binding-method-adr>.md` — 方式 ADR（T5 で作成）。アダプタの構築方式・in-place 対応方式の決定ゲート

## Dependencies

- 前提タスク: [decide-plugin-binding-method-adr.md](decide-plugin-binding-method-adr.md)（`TASK-034JgsHEkIUa3P4jzOizoM`）— 構築方式（binding クレート採否・退路）は方式 ADR の決定に従う
- 後続をブロック: [sync-docs-and-verify-plugin-foundation.md](sync-docs-and-verify-plugin-foundation.md) — 全実装完了後にドキュメント同期と総合検証を実施
- 経由前提: T1 の規格中立型・処理契約と T2 の借用 view は T3/T4 PoC 経由で土台となるが、方式変更があった場合は本タスクで整合させる

## Affected Files

- `crates/ruviform-plugin-clap/src/lib.rs` ほか — 規格エントリポイント・ABI→中立型翻訳・状態写像の完成（PoC 資産を発展）
- `crates/ruviform-plugin-vst3/src/lib.rs` ほか — 同上（VST3 側）
- `crates/ruviform-plugin-clap/Cargo.toml` / `crates/ruviform-plugin-vst3/Cargo.toml` — 方式 ADR 決定に伴う依存確定
- `Cargo.lock` — 依存確定の記録

## Done When

- [x] 両アダプタが規格エントリポイント・ABI↔中立型の一方向翻訳・共通状態機械への状態写像を備え、規格ごとの遷移規則対応（逆遷移・再アクティベート・失敗遷移・ホスト違反の拒否を含む）が単体テストで検証されている
- [x] 境界バリデーション失敗時に定義済みの拒否が返り、UB・チャンネルインデックスずらしがない。エイリアス検出が per-channel ポインタ対比較で実装され、全スクラッチが per-instance かつ activate/prepare 時事前確保である
- [x] 公開 API シグネチャに `clack`/`vst3`/`clap-sys` 等の binding クレート型・エラー・feature 名が露出していない
- [x] `pub unsafe fn` が公開 API になく、unsafe が SAFETY コメント付き局所許可に限定されている

## Verification

- [x] `mise exec -- cargo test -p ruviform-plugin-clap -p ruviform-plugin-vst3` exits 0（規格遷移→状態機械写像・境界バリデーション拒否・`rt_` プレフィックスの no-alloc 契約テストを含む）
- [x] `mise exec -- cargo clippy -p ruviform-plugin-clap -p ruviform-plugin-vst3 --all-targets -- -D warnings` exits 0
- [x] 外部型非露出監査: `grep -rn "use \|pub use " crates/ruviform-plugin-clap/src crates/ruviform-plugin-vst3/src` と `cargo doc` の公開面シグネチャで `clack`/`vst3`/`clap_sys` 等のパターンが露出しない（`pub.*vst3` のみの grep に限定しない — V-6）
- [x] エクスポートシンボル確認: cdylib 成果物に `clap_entry`・VST3 の `GetPluginFactory` + プラットフォーム条件付きモジュールエントリ（方式 ADR の決定どおり macOS `bundleEntry`/`bundleExit`・Linux `ModuleEntry`/`ModuleExit`・Windows `InitDll`/`ExitDll` 任意）が存在する
