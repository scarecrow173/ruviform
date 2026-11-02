---
id: "IMPL-034QOI4N1Bil7gejYxAo6i"
type: "impl"
status: "completed"
title: "Issue #258 headless value-gesture interaction contract"
created: "2026-11-02"
updated: "2026-11-02"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/258"
    - "https://github.com/scarecrow173/ruviform/issues/257"
  changes:
    added:
      - type: "file"
        path: "ui/.gitignore"
      - type: "file"
        path: "ui/core/package.json"
      - type: "file"
        path: "ui/core/package-lock.json"
      - type: "file"
        path: "ui/core/tsconfig.json"
      - type: "file"
        path: "ui/core/README.md"
      - type: "file"
        path: "ui/core/src/index.ts"
      - type: "file"
        path: "ui/core/src/interaction.ts"
      - type: "file"
        path: "ui/core/src/value.ts"
      - type: "file"
        path: "ui/core/test/fixtures.test.ts"
      - type: "file"
        path: "ui/core/test/helpers.ts"
      - type: "file"
        path: "ui/core/test/interaction.test.ts"
      - type: "file"
        path: "ui/core/test/value.test.ts"
      - type: "file"
        path: "docs/impl/ir/issue-258-headless-value-gesture-interaction-contract.md"
    modified:
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "docs/impl/ir/README.md"
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
    - "https://github.com/scarecrow173/ruviform/issues/259"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers:
    - "https://github.com/scarecrow173/ruviform/issues/259"
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #258 headless value-gesture interaction contract

## Summary

Issue #258 が要求する framework-neutral な headless
value/gesture/interaction 契約を、vanilla TypeScript パッケージ
`ui/core`（パッケージ名 `@ruviform/ui-core`）として実装した。設計正本は
Issue #257。Rust クレート追加は需要実在化まで見送りという Issue
記載の判断に従い、Node 26 の erasable TypeScript（strip-only）で直接
実行可能な ESM パッケージとし、DOM / WebView / React / Solid 等の
ランタイム依存を一切持たない。`mise run check-ui-headless` /
`test-ui-headless` の 2 レーンを追加し、Rust 側 `fmt-check` / `check`
/ `lint` / `test` も全て green。

## Context

- Source: ruviform issue #258（受入条件の正本）/ issue #257（横断設計の
  正本）。GUI/WebView 統合と DOM リスナ、Web Components ライフサイクルは
  Issue #259 以降へ defer。
- 配置: Rust クレートではなく `ui/core` に TypeScript パッケージとして
  新設。Issue #258 が「初期実装は TS module で良い、Rust crate は実需要
  まで不要」と明記しており、Synvelis / AudioLens 双方の JS/TS 側消費を
  直接対象とする。
- 実行基盤: mise pin の `node = "26.12.1"` が `.ts` の直接実行
  （erasable syntax のみ対応）を持つため、ビルド工程なしで
  `node --test` が走る。型検査は devDep `typescript` の
  `tsc --noEmit`（DOM lib 非含有）で実施。

## Implementation

### Changes

- `ui/core/src/value.ts`（新規）— 値契約
  - `ValueDescriptor`（`min`/`max`/`defaultValue`/`step`/`stepDomain`/
    `fineStep`/`fineStepDomain`/`unit`/`mapping`/`outOfRange`/`readOnly`/
    `disabled`/`constant`）と構造検証 `validateDescriptor`（有限性・範囲・
    step 整合・端点・デフォルト範囲を `DescriptorIssue` として集約報告）。
  - `ValueMapper` が値ドメイン不変条件の authoritative owner。
    `resolvePlain` / `resolveNormalized` は 範囲ポリシー→宣言 step
    ドメインでの snap→写像変換 の順で `{plain, normalized}` の整合ペアを
    返し、非有限入力は `non-finite`、範囲外は `outOfRange` ポリシー
    （`clamp`/`reject`）で処理。写像出力の非有限・範囲外は
    `invalid-mapping` として欠陥検出（ε 許容で float 誤差はクランプ）。
  - `linear` / `log`（正の端点必須、零付近端点で有限性を維持）/
    `custom`（`toNormalized`/`fromNormalized` ポート）写像。custom 等の
    ポートコールバックの throw は `invalid-mapping`/`non-finite`/
    既定フォーマットへの degrade に正規化し dispatch へ伝播しない。
  - `stepDomain` で step が plain / normalized のどちらの単位かを明示し、
    相対調整 `adjustNormalized` は宣言ドメインで整数 step 演算するため
    非線形写像でも double conversion 誤差を生じない。fine 調整は
    `fineStep`、未指定時は normalized フォールバック格子。
  - `degenerate`（`min === max`）は endpoint に縮退、`constant` フラグで
    不変性を明示。`defaultPlain` は `defaultValue` または `min`。
  - 表示: `unit` 付加・カスタム formatter・`decibelFormatter` で
    零振幅を `−∞ dB`（`special: "negative-infinity"`）として明示
    （非有限入力は `String(plain)` にフォールバックし −∞ を詐称しない）。
  - `verifyMapping` で単調性・端点・有限性・範囲外出力・可逆性を
    プローブし `MappingViolation` を返す。
- `ui/core/src/interaction.ts`（新規）— ジェスチャ・イベント契約
  - `Idle → Active(Begin) → Preview* → Commit | Cancel` の reducer 型
    ステートマシン `InteractionControl`。Commit/Cancel は厳密に一回、
    Cancel は committed state を汚染しない（preview 値を捨てる）。
  - 入力は `begin`/`preview`/`commit`/`cancel`/`set`/`adjust`/`reset`/
    `focus-lost`/`detach` の `ControlInput` union。`set`/`adjust`/`reset`
    は gestureId なしで atomic（Begin+Commit 一括、`atomic: true`
    メタ）、gestureId ありで preview 蓄積。
  - `pointer`/`touch`/`pen`/`wheel`/`keyboard`/`programmatic` の入力源、
    Escape/lost-focus/lost-pointer-capture/unmount/dock-transfer/explicit
    の cancel reason（open string union で拡張可）。
  - `detachPolicy: "cancel" | "commit"`。`focus-lost`/`detach` は
    オプション `gestureId` で対象限定可（stale lifecycle event が
    新ジェスチャを殺さない）。ライフサイクル入力は seq drop 対象外
    （遅延配信でもジェスチャを view より長生きさせない）。
  - イベントは `begin`/`preview`/`commit`/`cancel`/`unavailable`/
    `rejected`/`noop` の判別 union。`preview` は `coalescible: true`、
    端末・能力・エラーイベントは non-droppable。メタは `controlId`/
    `target`/`originView`/`gestureId`/`sequence`（コントロール内単調増加）/
    `inputSeq`/`source`/`atomic` を保持。
  - `disabled`/`readOnly`/`unsupported` を `unavailable` reason で明示、
    `no-active-gesture`/`duplicate-input`/`no-default` を `noop`、
    非有限・範囲外・invalid-mapping・stale/terminated gesture・
    out-of-order seq を `rejected` で区別。adapter 側 `seq` で
    重複（`duplicate-input` noop）・遅延（`out-of-order`）・
    競合（`gesture-already-active`/`stale-gesture`）を検出。終端
    gestureId は有界履歴で stale 検出を保持。
  - `InteractionDispatcher` が `controlId` ルーティングと
    `unregister`（detach 委譲）を提供。
  - `ParamValueAdapter<T>` プロトコル型のみ定義（plugin-core への依存は
    作らず、`toParamValue`/`fromParamValue` の explicit adapter を
    消費側が実装する形）。
- `ui/core/test/*.ts`（新規）— 67 件の headless テスト。
  seeded PRNG の roundtrip 系、端点・非有限・log 零付近、clamp/reject、
  plain/normalized step ドメイン、fine、degenerate、format/parse、
  −∞dB、custom mapping 欠陥検出、ジェスチャ exactly-once・cancel 非汚染・
  detach ポリシー・重複/遅延/競合・unavailable/noop 区別、
  Synvelis 風（wheel+keyboard）/ AudioLens 風（解析パラメータ）
  消費者フィクスチャを `fixtures.test.ts` で共用。
- `ui/core/package.json`/`tsconfig.json`/`package-lock.json`/`ui/.gitignore`
  （新規）— ESM・devDeps は `typescript` + `@types/node` のみ。
  `noEmit`・`erasableSyntaxOnly`・`exactOptionalPropertyTypes` 等 strict。
- `ui/core/README.md`（新規）— パッケージ責務・配置・検証入口・
  #259 との境界を記述。
- `mise.toml` — `check-ui-headless`（`npm ci` + `tsc --noEmit`）と
  `test-ui-headless`（`npm ci` + `node --test`）を追加。
- `.github/workflows/ci.yml` — `ui-verify` ジョブに `node` ツール復元と
  2 headless lane ステップを追加（3 OS matrix で headless 契約も検証）。
- `AGENTS.md` — 検証入口表と `ui-verify` 記述に新 lane を追記。

### Decisions During Implementation

- Rust クレートではなく `ui/core` に置く。Issue #258 の「実需要まで
  Rust 化しない」判断と、`poc/ui/frontend`（PoC）とは別層の生産配置が
  必要という点から、repo 直下 `ui/` にパッケージを新設した。
- `tsc` emit は持たず `node --test` の erasable TypeScript 直実行を採用。
  ビルド工程を持たないため lockfile `npm ci` で最小の再現性を確保しつつ
  依存は typecheck 用 devDep 2 件のみに抑えた。
- `adjustNormalized` は normalized 近似量子でなく宣言 step ドメインで
  演算する（plain step + log 写像で normalized 側近似だと不正確になる
  ことを実装中に確認し、ドメイン変換して整数 step を加えて戻す形に修正）。
- 初期 committed 値は normalized 0 固定でなく `defaultValue ?? min` に
  初期化（Synvelis fixture で default 起点が要求されるため）。
- レビューで確認した問題として (a) detach/focus-lost が seq drop で
  取りこぼされジェスチャが残存し得る問題 → ライフサイクル入力を
  seq 免除＋`gestureId` キー化、(b) custom 写像の範囲外出力が
  committed 値に流れ込み得る問題 → 写像出力の ε 許容範囲検査
  `invalid-mapping` 化、(c) plain step 宣言コントロールで normalized
  入力が snap を bypass する問題 → 双方向 resolve で宣言ドメインに
  再 snap、(d) ポートコールバック throw の伝播 → degrade 正規化、
  (e) `decibelFormatter` の非有限入力が −∞ を詐称 → `String(plain)`
  フォールバック、を修正した。
- `constant` な degenerate は入力自体を unavailable にはせず
  （唯一可能な値への atomic commit を許す）。自動化可能な固定パラメータの
  ケースを塞がないため。

## Related Experiments

### Adopted

（なし — Node 26 erasable TS 直実行の動作確認のみ。Experiment Log は作成せず）

### Rejected

（なし）

## Validation

- `mise run check-ui-headless` — pass（`npm ci` + `tsc --noEmit`）。
- `mise run test-ui-headless` — pass（`node --test`、67 tests / 0 fail）。
- `mise run fmt-check` — pass（`cargo fmt --all -- --check`）。
- `mise run check` — pass（`cargo check --workspace --all-targets`）。
- `mise run lint` — pass（`cargo clippy --workspace --all-targets -- -D warnings`）。
- `mise run test` — pass（`cargo test --workspace` 全 doctest 含む）。
- `mise exec -- actionlint .github/workflows/ci.yml` — pass。
- `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` —
  本レコードに findings なし（他既存レコードの warning/error は既存状態）。
- review-protocol 適用: 客観検証（上記レーン全量）の後、受入条件突合・
  敵対レビューを実施し、確認した 5 件（ライフサイクル seq 免除・写像
  出力範囲検査・宣言ドメイン再 snap・ポート throw degrade・dB 非有限
  フォールバック）を修正・再検証済み。

## Risks

- `node --test` の `.ts` 直実行は Node 26 系の erasable-syntax 機能に
  依存するため、enum 等の非 erasable 構文は tsconfig の
  `erasableSyntaxOnly` で compile-time に防ぐ。Node バージョン低下は
  mise pin で保護されているが、消費側 Node が古い場合は配布用 emit 手順が
  別途必要になる可能性がある。
- `InteractionDispatcher` の synthetic event シーケンスはコントロール
  個別シーケンスと別系統（unknown-control 通知用）であり、トランスポート
  が両者を混ぜると順序解釈を誤る可能性がある。

## Follow-ups

- Issue #259: DOM リスナ・Web Components ライフサイクル・presentation
  adapter がこの headless 契約を駆動する実統合。
- `ParamValueAdapter` の実装接続（plugin-core `ParamValue` との explicit
  変換）は消費側 Issue で確定。
- React/Strict remount・フレームワーク検証フィクスチャは Issue #263 で
  共用（本レーンの `fixtures.test.ts` がその雛形）。
