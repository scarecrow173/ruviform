---
id: "TASK-034JgsFV6CWAjSnPuekBgs"
type: "task"
status: "done"
title: "ruviform-plugin-core 共通基盤クレートの作成"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/29"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-plugin-core/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/descriptor.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/param.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/event.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/bus.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/transport.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/lifecycle.rs"
      - type: "file"
        path: "crates/ruviform-plugin-core/src/processor.rs"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-plugin-foundation.md"
  implemented-by:
    - "../impl/ir/create-plugin-core-foundation.md"
  depends-on:
    - "../plans/implement-plugin-foundation.md"
  blocks:
    - "implement-borrowed-planar-view.md"
    - "poc-clap-binding-adapter.md"
    - "poc-vst3-binding-adapter.md"
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
  references: []
  defers: []
  deferred-by: []
---

# ruviform-plugin-core 共通基盤クレートの作成

## Work

plan `PLAN-034JcvpHIlt5JmqvgOquvI` の T1（Task Breakdown の `implement-plugin-core-foundation` 仮称行に対応）。拡張層に規格中立プラグイン基盤クレート `ruviform-plugin-core` を新設し、workspace に統合する。許可依存層は Domain foundations + Primitives のみ。

- [x] `crates/ruviform-plugin-core/` を作成（`Cargo.toml`: workspace 継承・`[lints] workspace = true`・path 依存 `ruviform-media-core` / `ruviform-audio-core`。`ruviform-rt` は `RtContext` 受け入れを RT 契約実装 Issue に委譲する確定決定のため現段階では付けない — plan Open Questions / Gaps 参照）
- [x] ルート `Cargo.toml` の `members` に `crates/ruviform-plugin-core` を追加し、層内順序コメントに拡張層を追記
- [x] `descriptor` / `param` / `event` / `bus` / `transport` / `lifecycle` / `processor` モジュールの公開型を実装。transport/event は音楽時間を持たず `TimeBase` + 整数カウントのみ、opaque `reserved` 拡張フィールドを持つ（`TransportSnapshot` と同一方針）。sample-accurate イベントの意味論を型とドキュメントで固定する: 順序（サンプルオフセットでソート済み）、オフセット表現（ブロック内フレームオフセットでブロックフレーム数未満に有界 — 絶対 `MediaTime` は wire 表現ではない）、パラメータ値域（正規化 f64 ↔ 内部表現の写像と NaN ポリシー）、ゼロフレーム時のイベントフラッシュ動作、process ステータス/テイルの中立ステータス型への写像。lifecycle は遷移表として定義し、`create → init → activate → processing → deactivate → destroy` の順方向チェーンに加え、`processing → activate` の処理停止遷移（`deactivate` は processing 停止を前提）、activate ↔ processing の再アクティベート往復、init/activate 失敗時の遷移先、`destroy` の deactivate 前提、ホストプロトコル違反（activate 前 process・processing 中 deactivate 等）の定義済み拒否を規定し、型と `Result` で固定して不正遷移を表出する
- [x] `processor` は規格中立のプラグイン処理契約（prepare/process 形状・仮称 `PluginProcessor` 相当）を plugin-core 所有で定義する — **アダプタ（`ruviform-plugin-clap` / `-vst3`）はこの契約を実装・駆動する側であり、契約自体は決して定義しない**。`process` は借用した入出力 view を受け取り、そのライフタイムはコールバックスコープに束縛される（ゼロコピー契約をシグネチャ自体で強制 — spec Requirements 参照）。将来の `RtContext` 等 capability 注入を破壊的変更なく吸収できるよう、context 引数は opaque/予約型とする
- [x] `lib.rs` に crate doc（責務・許可依存層 Domain foundations + Primitives・RT 制約・規格中立性の非目標宣言）・再エクスポートを実装。全公開 API に `**RT contract:**` ラベル、全公開 enum に enum レベル `#[non_exhaustive]`、公開 API 例を doctest として整備

## Classification

- `normal-plan-task`

## Upstream Documents

- [../plans/implement-plugin-foundation.md](../plans/implement-plugin-foundation.md) — `PLAN-034JcvpHIlt5JmqvgOquvI`（approved）。T1 チェックリスト・File Map・V-1 の正本
- [../specs/implement-plugin-foundation.md](../specs/implement-plugin-foundation.md) — `SPEC-034JcgdAKPNeh1YCFyprzh`（proposed）。Issue #29 要件の正本
- [../designs/ruviform-architecture-design-baseline.md](../designs/ruviform-architecture-design-baseline.md) — `DESIGN-034IPynWe2V30FLubzQenb`（approved）。ライフサイクル状態機械・opaque 拡張予約・crate 表の根拠
- [../adr/realtime-contract-and-verification-policy.md](../adr/realtime-contract-and-verification-policy.md) — RT ADR（accepted）。`**RT contract:**` ラベル規則の根拠
- [../adr/public-enum-non-exhaustive-policy.md](../adr/public-enum-non-exhaustive-policy.md) — 公開 enum `#[non_exhaustive]` 統一方針
- [../test-specs/plugin-foundation.md](../test-specs/plugin-foundation.md) — `TSPEC-034Jg6YSP6CLjJHnFGG5oL`。契約所有・規格中立性の不変条件

## Dependencies

- 前提: [../plans/implement-plugin-foundation.md](../plans/implement-plugin-foundation.md) — plan の depends-on 連鎖で `implement-issue-18-data-model.md`（completed）由来の `TimeBase`/`MediaTime`・`TransportSnapshot` reserved パターン・`**RT contract:**` 規則を前提表面とする
- 後続をブロック: [implement-borrowed-planar-view.md](implement-borrowed-planar-view.md)（借用 view の API 形状は plugin-core の process モデル・規格中立型と整合させて設計するため core 先行）
- plugin 系クレート間の依存は `ruviform-plugin-clap` / `-vst3` → `ruviform-plugin-core` の一方向のみ。plugin ↔ ui の依存は作らない（ベースライン規則）

## Affected Files

- `crates/ruviform-plugin-core/Cargo.toml` — 新規
- `crates/ruviform-plugin-core/src/lib.rs` — 新規（crate doc・再エクスポート・`rt_` テスト）
- `crates/ruviform-plugin-core/src/{descriptor,param,event,bus,transport,lifecycle,processor}.rs` — 新規
- `Cargo.toml` — `members` 追加・層内順序コメント追記
- `Cargo.lock` — 更新

## Done When

- [x] 全モジュールの公開型が crate root から再エクスポートされ、ライフサイクル不正遷移が型と `Result` で表出される
- [x] transport/event が音楽時間を持たず opaque `reserved` フィールドを持ち、process API の context 引数が opaque/予約型である
- [x] 規格中立性がコード要素で確認できる: `use` 行に `clack`/`vst3`/`clap_sys` 等の binding クレートへのパスがない・`cargo metadata` で依存辺がない・`cargo doc` 公開面シグネチャに規格固有型がない（doc コメント内の "CLAP"/"VST3" 非目標宣言は許容）
- [x] 全公開 enum に enum レベル `#[non_exhaustive]` が付与され、全公開 API に `**RT contract:**` ラベルがある

## Verification

- [x] `mise exec -- cargo test -p ruviform-plugin-core` exits 0（ライフサイクル遷移規則・規格中立型の不変条件テスト・doctest を含む）
- [x] `mise exec -- cargo clippy -p ruviform-plugin-core --all-targets -- -D warnings` exits 0
- [x] `mise exec -- cargo metadata --format-version 1` で `ruviform-plugin-core` から binding クレートへの依存辺がないことを確認
- [x] 全公開 enum の enum レベル `#[non_exhaustive]` 付与は `workspace.lints.clippy` の `exhaustive_enums` restriction lint（`mise run lint` の `-D warnings` ゲート）に委ねる — grep カウント照合は信頼性が低いため検証手順に採用しない
