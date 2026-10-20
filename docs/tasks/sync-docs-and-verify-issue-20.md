---
id: "TASK-034Jgi4vWCuaMUu3POps1c"
type: "task"
status: "todo"
title: "Issue #20 ドキュメント同期と総合検証"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/20"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/implement-media-io-abstraction.md"
      - type: "file"
        path: "docs/impl/ir/implement-io-symphonia-backend-poc.md"
      - type: "file"
        path: "docs/impl/ir/sync-docs-and-verify-issue-20.md"
    modified:
      - type: "file"
        path: "Cargo.toml"
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
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/issue-20-i-o-poc.md"
  implemented-by: []
  depends-on:
    - "../plans/issue-20-i-o-poc.md"
    - "implement-media-io-abstraction.md"
    - "implement-io-symphonia-backend-poc.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../../AGENTS.md"
    - "../adr/media-io-backend-abstraction.md"
    - "../adr/licensing-and-distribution-policy.md"
    - "../adr/realtime-contract-and-verification-policy.md"
    - "../adr/public-enum-non-exhaustive-policy.md"
    - "../specs/implement-media-io-abstraction-and-first-backend-poc.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/issue-20-i-o-poc.md"
    - "../specs/implement-media-io-abstraction-and-first-backend-poc.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/media-io-abstraction-and-backend-poc.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #20 ドキュメント同期と総合検証

## Work

plan `PLAN-034JfKRCFCu8oJr5iCe6AV` の Phase 3（直列クリティカルパスの終端）。タスク 1・2 の完了を受け、ドキュメント記述の同期・総合検証スイート（V-1〜V-6）・独立レビューサイクル・実施記録の作成を行う。

- [ ] ルート `Cargo.toml`: メンバーコメントを 2 クレート追加・path 依存配線の実態へ同期
- [ ] `README.md`: クレート一覧・状態の同期（`ruviform-media-io` = 非 RT 抽象境界クレート、`ruviform-io-symphonia` = 第一バックエンド PoC・分離クレート + opt-in）
- [ ] `AGENTS.md`: ワークスペース構成段落の同期（members・依存配線 `ruviform-media-io → media-core/audio-core/video-core`・`ruviform-io-symphonia → media-io (+domain)`・非 RT 専用の記載）
- [ ] `docs/designs/ruviform-architecture-design-baseline.md`: L265 相当行（「抽象的な流れ」の `Decoder` → `AudioBuffer` / `VideoFrame` 記述）を `Decoder` → `AudioFrame` / `VideoFrame` へ同期更新（spec 決定済みの差異の上流反映・AC10）
- [ ] `docs/specs/implement-media-io-abstraction-and-first-backend-poc.md`: front matter 整合（`status` → `implemented`・`implemented-by` へ本 plan リンク — plan の `implements` との双方向整合。`derived-by` の plan リンクは記入済み）
- [ ] `docs/plans/issue-20-i-o-poc.md`: `status` → `completed`・`changes` の実態確認・同期
- [ ] 実施記録 `docs/impl/ir/` の作成・集約（タスク 1・2 の検証結果を各 IR `implement-media-io-abstraction.md` / `implement-io-symphonia-backend-poc.md` へ集約。本タスクの IR `sync-docs-and-verify-issue-20.md` を作成）。IR 作成後に各タスク front matter の `implemented-by` へ対応 IR パスを記入する（broken forward reference 回避のため現時点では空）
- [ ] 総合検証スイート V-1〜V-6 の全項目を実行（下記 Verification 節。`cargo check -p ruviform-media-io` / `cargo tree -p ruviform-media-io` による `ruviform-io-*` 依存辺非存在確認を含む）
- [ ] 独立レビューサイクル（Reviewer → Validator → Judge → 修正 wave）。レビュー焦点: 型純度・能力交渉両機構・EOS/終端表現・`rt_` 命名・`#[non_exhaustive]`・`**RT contract:**` カバレッジ・ベースライン L265 同期・実装時確定の設計裁量点（decoder 出力表現・`Data` ストリーム packet 扱い・`Packet` フラグ集合・codec ID/名管理・`Muxer`/`Encoder` 契約詳細）の過剰汎化/表現不足判定

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/issue-20-i-o-poc.md](../plans/issue-20-i-o-poc.md) — `PLAN-034JfKRCFCu8oJr5iCe6AV`（approved）。Phase 3（step 3-1〜3-5）・検証スイート V-1〜V-6・AC 対応表・Review Handoff の正本
- [../specs/implement-media-io-abstraction-and-first-backend-poc.md](../specs/implement-media-io-abstraction-and-first-backend-poc.md) — `SPEC-034JcaMXubILS7j1FX5qob`（approved）。AC1-AC11 の受け入れ基準突合対象
- [../test-specs/media-io-abstraction-and-backend-poc.md](../test-specs/media-io-abstraction-and-backend-poc.md) — 振る舞い保証（demux/decode 正しさ・PTS 写像・能力交渉・エラー正規化・`AudioFrame` 形状契約）の意図の正本（`relations.verified-by`）
- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — `DESIGN-034IPynWe2V30FLubzQenb`（approved）。L265 相当行の同期更新対象

## Dependencies

- 前提タスク（全て完了していること）:
  - [implement-media-io-abstraction.md](implement-media-io-abstraction.md)（`TASK-034JghH9i5p72LmOoWIPGe`）
  - [implement-io-symphonia-backend-poc.md](implement-io-symphonia-backend-poc.md)（`TASK-034Jgi3OFauA1SWRPdvnnb`）
- ベースライン L265 同期は `AudioFrame` 確定（タスク 1）に依存。既存 6 クレート（media-core/audio-core/video-core/math/rt/exec）は非改変の前提（AC7）

## Affected Files

- `Cargo.toml` — メンバーコメントのみ同期（値行は実装タスクで追加済み）
- `README.md` — クレート一覧・状態記述の更新
- `AGENTS.md` — ワークスペース構成段落の更新
- `docs/designs/ruviform-architecture-design-baseline.md` — L265 相当行の `AudioBuffer` → `AudioFrame` 記述更新
- `docs/specs/implement-media-io-abstraction-and-first-backend-poc.md` — front matter 整合（status / implemented-by）
- `docs/plans/issue-20-i-o-poc.md` — status / changes の実態同期
- `docs/impl/ir/implement-media-io-abstraction.md` — 新規（タスク 1 実施記録）
- `docs/impl/ir/implement-io-symphonia-backend-poc.md` — 新規（タスク 2 実施記録）
- `docs/impl/ir/sync-docs-and-verify-issue-20.md` — 新規（本タスク実施記録）
- `crates/` 配下 — レビュー修正 wave による既存ファイルへの追補が発生しうる（各 IR に記録）

## Done When

- [ ] `Cargo.toml` / `README.md` / `AGENTS.md` / ベースライン L265 相当行の記述が実装済み状態（2 クレート追加・依存配線・非 RT 専用・`AudioFrame`）と一致する（AC10）
- [ ] spec の front matter が実装済み状態（`implemented`）と整合し、plan との双方向リンクが成立する
- [ ] 検証スイート V-1〜V-6 が全て PASS し、AC1-AC11 の受け入れ基準に突合済み
- [ ] レビュー findings が全て処理・判定され、Judge の最終 verdict が記録されている
- [ ] 実施記録 3 件（`docs/impl/ir/`）に検証結果・実装時決定（設計裁量点）・レビュー記録が残っている

## Verification

- [ ] `mise exec -- cargo test -p ruviform-media-io` + `mise exec -- cargo clippy -p ruviform-media-io --all-targets -- -D warnings`（V-1。AC2/AC7/AC11）
- [ ] `mise exec -- cargo check -p ruviform-media-io` + `mise exec -- cargo tree -p ruviform-media-io` — `ruviform-io-*` 依存辺・外部依存・symphonia 辺が存在しない（V-1。AC2/AC4）
- [ ] `mise exec -- cargo test -p ruviform-io-symphonia` + `mise exec -- cargo clippy -p ruviform-io-symphonia --all-targets -- -D warnings`（V-2。AC5/AC6）
- [ ] `grep -rn "symphonia" crates/ruviform-media-io/src crates/ruviform-io-symphonia/src/lib.rs` / `grep -rn "pub use" crates/ruviform-io-symphonia/src` / `grep -rn "unimplemented!\|unreachable!\|panic!" crates/ruviform-io-symphonia/src` — 型純度・非対応表現監査（AC1/AC6）
- [ ] `mise exec -- cargo doc -p ruviform-media-io -p ruviform-io-symphonia --no-deps` — 公開 API レビュー（外部型非露出・4 trait 分離。AC1/AC3）
- [ ] `mise run fmt-check` + `mise run check` + `mise run lint` + `mise run test` + `mise exec -- prek run --all-files` — 全グリーン・0 warnings（V-3。AC8）
- [ ] `mise run test-rt` — `rt_` 部分文字列フィルタで意図した既存 RT テストのみ選択され、新規非 RT テストが誤選択されない（V-4・命名規約監査）
- [ ] `mise run bench` — ベンチマーク未実装のまま入口が no-op を維持（V-5）
- [ ] `mise exec -- cargo metadata --format-version 1 --no-deps` — 全パッケージ `publish` が `[]`・依存辺が設計どおり（media-io → 3 domain クレート、io-symphonia → media-io + domain。V-6）
- [ ] `grep -rn "unsafe" crates/ruviform-media-io crates/ruviform-io-symphonia` — コード内 0 件（AC8）
- [ ] `grep -rn "pub enum" crates/ruviform-media-io crates/ruviform-io-symphonia` + doc レビュー — 公開 enum `#[non_exhaustive]`・全公開 API `**RT contract:**` ラベル監査（AC9）
- [ ] `git status crates/ruviform-audio-core crates/ruviform-video-core crates/ruviform-media-core` — 非改変確認（AC7）
- [ ] Manual: README/AGENTS/Cargo.toml/ベースライン L265 相当行の実態突合（AC10）・レビュー findings の全件処理記録
