---
id: "SPEC-034JQubfZRC7nRmTsYvLiq"
type: "spec"
status: "approved"
title: "SIMD dispatch 層（prepare 固定 + カーネル/サイズ別ルーティング + 本番精度契約）"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/53"
  changes:
    added:
      - type: "file"
        path: "crates/ruviform-math/src/dsp.rs"
    modified:
      - type: "file"
        path: "crates/ruviform-math/Cargo.toml"
      - type: "file"
        path: "crates/ruviform-math/src/lib.rs"
      - type: "file"
        path: "crates/ruviform-math/src/simd_poc.rs"
      - type: "file"
        path: "crates/ruviform-math/tests/simd_equiv.rs"
      - type: "file"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../adr/dsp-simd-backend-adoption.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/dsp-simd-backend-adoption.md"
  refines: []
  refined-by: []
  derives-from:
    - "../adr/dsp-simd-backend-adoption.md"
  derived-by:
    - "../adr/simd-ulp.md"
    - "../designs/simd-dispatch-layer.md"
    - "../plans/implement-issue-53-simd-dispatch-layer.md"
  verifies: []
  verified-by:
    - "../test-specs/simd-dispatch-layer-tests.md"
  references:
    - "../designs/ruviform-architecture-design-baseline.md"
  defers: []
  deferred-by: []
---

# SIMD dispatch 層（prepare 固定 + カーネル/サイズ別ルーティング + 本番精度契約）

Issue [#53](https://github.com/scarecrow173/ruviform/issues/53)。採否 ADR（`docs/adr/dsp-simd-backend-adoption.md`）決定 1–3 の本番実装側。`ruviform-math` に dispatch 層を集約し、スカラー参照 + レーン別 SIMD カーネル + カーネル・サイズ応答ルーティングを本番コードとして設計・実装する。

## What Should Be Built

`ruviform-math` の公開 API としての SIMD カーネル層:

- **prepare 固定の dispatch トークン**: 呼出し側（engine/processor の prepare・ブロック境界）が一度だけ構築する不透明トークン型。`Level`/`Simd` 系バックエンド型は内部実装に閉じ込め、公開 API に露出しない（crate 契約: 「their types must not appear in the public API」）。トークン構築時に ISA 選択を確定し、per-call の `dispatch!` 全経路（`Level::new()` 読出し + arm 選択）を繰り上げる
- **公開カーネル関数**: PoC で検証済みの 8 カーネル — `add`/`scale`/`clamp_unit`/`fade`/`dot`/`fir`/`biquad`/`sine`。スカラー参照実装を正とする二層構造（決定 1）を本番でも維持
- **カーネル・サイズ応答ルーティング**: dispatched 経路が小サイズ・メモリ帯域律速域でスカラーに実測劣後する帯（#56 計測: add で N≤256 は +6–7ns の固定コスト支配）をスカラー経路へ回すルーティング規則。閾値は計測レポート由来の確定値として設計書で固定する
- **本番精度契約**: 各カーネルの公開 API doc に精度基準を明示する。PoC の `dot` ulp ≤256 等は PoC 入力クラス基準の上限であり、本番契約では入力前提（有限入力・対象入力クラス）と基準形（ulp / f64 真値基準 / スケール相対誤差）を選択肢評価のうえ確定する（ADR で決定）
- **`fearless_simd` の本番昇格**: `dev-dependencies` から `[dependencies]` へ移す（前提: MSRV 1.89 = Issue #52 で決定済み）

## Why Now

- 採否 ADR が決定 1–3（二層構造採用・fearless_simd 条件付き採用・等価基準確定値）を確定し、「条件」だった残課題が後続 Issue で消化された: #56 で dispatched 劣後の原因が **per-call 固定コスト実在**（`dispatch_noop` ≈1.29ns）と分離確定、#52 で MSRV 1.89 への引上げが実測決定
- dispatch 層の不在のままでは engine 層（後続 Issue 群）が PoC ベンチコードを参照するしかなく、公開 API 契約が確定しない

## Users and Value

- **engine 層 / DSP 利用側**: prepare 境界でトークンを取得し、RT パスから allocation なし・panic なし・契約付きでカーネルを呼べる
- **保守担当**: ルーティング規則と精度契約が文書化され、性能回帰時の調査プロトコル（超過はまず不具合として調査）が確立される

## Scope

### In Scope

- `ruviform-math` 公開 API: dispatch トークン型 + 8 カーネル公開関数（スカラー参照 + SIMD 経路 + ルーティング）
- カーネル別・サイズ別ルーティング規則（設計書で閾値を確定値として定義）
- 本番精度契約の形選択（ulp 固定 / f64 真値基準 / スケール相対誤差 — ADR で決定）と各カーネル doc への明記
- `fearless_simd` の `[dependencies]` 昇格 + lockfile 追跡
- 公開 API の等価性検証（PoC matrix の本番面への移行・拡張）と RT 適合（allocation-free）の確認

### Out of Scope

- AVX-512 / aarch64 NEON の実機有効化（Issue #55 — 未計測 ISA は dispatch 対象に含めない）
- denormal 対策の実装（Issue #54 — エンジン層責務。カーネルは denormal 入力を契約上正しく処理するが性能崖対策は持たない）
- engine 層への実接続（本番利用側の構築は後続 Issue）
- `pulp`/`multiversion` の本番昇格・再評価（検証済み代替として PoC 資産に残る）
- `simd_poc` 実験モジュール自体の整理（公開 API 化とは別件。必要な内部共有は許容するが `#[doc(hidden)]` のまま）

## Requirements

- dispatch トークンは prepare/初期化境界で一度構築し、per-call では `dispatch!` 全経路を再実行しないこと（#56 実測の固定コスト除去）
- 公開 API に `fearless_simd` 系型を露出させないこと（トークンは不透明 struct。公開 enum を導入する場合は `#[non_exhaustive]` 方針 ADR に従う）
- 全カーネル呼出しは allocation-free・panic-free・有限 bounded（RT 契約対象クレート）
- スカラー参照実装を契約の「正」として維持し、SIMD 経路は既存等価性 matrix と同等の検証で契約適合を確認すること
- ルーティング閾値は計測根拠を設計書に記録し、マジックナンバー化しないこと
- 精度契約はカーネル単位で公開 doc に明記し、前提入力クラスを超える入力へのプロトコル（事後緩和せず不具合/契約不適合として調査）を含めること
- `unsafe` は原則禁止（`assume_supported`/`new_unchecked`/`std::arch` 不使用。fearless_simd の安全 API のみ）

## Acceptance Criteria

- [x] dispatch トークン型が公開 API として存在し、一度の構築で全カーネル呼出しに再利用できる
- [x] 8 カーネルが公開 API 経由で呼べ、トークンとサイズに応じたルーティング（SIMD / スカラー）が設計書の規則どおり動作する
- [x] 各カーネルの公開 doc に精度契約（bit 完全一致 or ulp 上限 + 入力前提）が明記されている
- [x] `fearless_simd` が `[dependencies]` にあり lockfile が追跡されている
- [x] 公開 API 経路の等価性テスト（スカラー参照対照）がグリーン
- [x] `mise run fmt-check` / `check` / `lint` / `test` / `test-rt` / `bench-smoke` パス
- [x] doc 監査で本チェーン起因 findings 0

## Deferred Design Concerns

- ブロック単位のトークン消費でさらに繰り上げる設計（process 呼出し全体を単一 `dispatch!` 内に置く構造）| 現段階ではカーネル単位 API が engine 契約の最小単位であり過剰 | engine 層で per-block 呼出し頻度が問題化した時点 | なし（追加最適化は非破壊に可能）
- AVX-512 トークン arm の有効化 | 実機未計測のため列に含めない | Issue #55 の実機/CI 計測 | なし
- denormal 入力での性能崖 | エンジン層責務（ADR 決定） | Issue #54 | なし

## Sources and Relations

- 採否 ADR `docs/adr/dsp-simd-backend-adoption.md` 決定 1–3（本 spec の `derives-from` / `related`）。決定 2 の「条件付き採用」の条件消化: #56 原因分離済み + #52 MSRV 決定済み
- 計測レポート `docs/impl/exp/dsp-simd-poc-20261021.md`（採否判定表）・`docs/impl/exp/dispatched-overhead-20261021.md`（dispatch 固定コスト分離 — ルーティング閾値の根拠データ）
- 設計ベースライン A.4「DSP／SIMD 設計」: 二層構造・dispatch 層の `ruviform-math` 集約・起動時検出トークンキャッシュの構造仮説（本 spec で確定化）
