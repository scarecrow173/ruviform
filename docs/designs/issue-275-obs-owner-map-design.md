---
id: "DESIGN-034WZCcSruHEmlEMFX1Mlu"
type: "design"
status: "approved"
title: "Issue #275 OBS-01〜11 Owner/API/Feature Matrix 対応付け設計"
created: "2026-11-14"
updated: "2026-11-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "docs/specs/issue-275-obs-owner-map.md"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-275-obs-owner-map.md"
      - type: "doc"
        path: "docs/designs/issue-275-obs-owner-map-design.md"
      - type: "doc"
        path: "docs/test-specs/issue-275-obs-owner-map.md"
      - type: "doc"
        path: "docs/plans/issue-275-obs-owner-map-plan.md"
      - type: "doc"
        path: "docs/tasks/issue-275-obs-owner-map-task.md"
      - type: "doc"
        path: "docs/impl/ir/issue-275-obs-owner-map.md"
      - type: "doc"
        path: "docs/impl/exp/issue-275-obs-owner-map.md"
      - type: "file"
        path: "evidence/rf275/obs-owner-map-audit.md"
    modified:
      - type: "doc"
        path: "docs/designs/issue-275-observability-integration-design.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-275-obs-owner-map.md"
  implemented-by: []
  depends-on:
    - "docs/specs/issue-275-obs-owner-map.md"
    - "docs/designs/issue-275-observability-integration-design.md"
    - "docs/specs/issue-275-observability-integration-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/issue-275-observability-integration-design.md"
    - "docs/specs/issue-294-cross-consumer-candidate-gate.md"
    - "docs/designs/issue-294-cross-consumer-candidate-gate.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-275-obs-owner-map.md"
    - "docs/specs/issue-275-observability-integration-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-275-obs-owner-map.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/275"
    - "https://github.com/scarecrow173/ruviform/issues/281"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/synvelis/issues/28"
    - "evidence/rf275/obs-owner-map-audit.md"
  defers: []
  deferred-by: []
---

# Issue #275 OBS-01〜11 Owner/API/Feature Matrix 対応付け設計

## Context

`docs/designs/issue-275-observability-integration-design.md`
（以下「統合設計正本」）は OBS-01〜11 の単一クレート構成・
横断契約・Consumer 利用面・証拠項目対応を所有する approved
正本である。しかし Issue #275 本文の 2026-11-13 現行受入
（Windows L1＋使用機能の L2/L3、macOS/Linux L1 のみ。
実外部 Provider・実 OS GUI・実 Device・長時間 Profiler は
利用要件発生まで Deferred）に対し、残件の「OBS-01〜11 と
実装 Owner/公開 API/Feature Matrix/既存 CI Case の対応
付け・AL/SV 別 Oracle の引き渡し・Deferred 範囲の
Owner/Trigger 明記」が未記述である。

実装側は進んでいる: OBS-01〜05・OBS-07〜11 の実装 Issue
（#276〜#280・#289〜#293）は CLOSED で、
`crates/ruviform-observability` に全モジュールと 4 つの
optional feature が存在する。一方で検証側は継続中で、
OBS-06（#281・OPEN）が gate Owner、#294・#219 が
cross-repo 機構、AL #127・SV #28 が Consumer Oracle を
所有する。Issue 状態と CI lane の実態は
`evidence/rf275/obs-owner-map-audit.md` に snapshot
として固定した。

本設計は「どこに何を書くか」と「表の語彙・記述規則」だけを
定める。per-module の詳細契約は既存の per-issue 正本
（#276〜#293 の設計・test-spec）に委ね、本 slice は
索引・対応付け・状態区分のみを追加する。

## Scope

### In Scope

- 統合設計正本への 2 つの新規セクション追記:
  (a) OBS-01〜11 Owner/API/Feature/CI Case 対応表、
  (b) Deferred 範囲の Owner/Trigger 表。
- AL #127・SV #28 の独立 Headless 受入の #281/#294 への
  引き渡しを対応表内に明記する記述。
- `issue-275-obs-owner-map` 文書チェーンと監査証拠
  `evidence/rf275/obs-owner-map-audit.md`。
- 触った dir の生成 index の再生成（`doc-maintenance`）。

### Out of Scope

- コード・CI・manifest の変更。
- per-module の詳細 API/内部実装/test 規則の再定義
  （per-issue 正本が所有）。
- AL/SV repo での実測・実機検証・実外部 Provider/GUI/
  Device/長時間 Profiler の実施。
- GitHub blockedBy の登録・削除、新規 Issue 起票
  （必要作業が判明した場合は PR 本文で報告）。
- 「Workerへの共通契約」節を含む既存設計書本文の
  構造的改変・歴史的 snapshot の書き換え。

## Components and Boundaries

### 追加する節（統合設計正本内）

`## 証拠項目対応 — 受入 Gate 9 項目の索引` 節の直後
（`## #101/#103 前提の更新記録` の前）に、次の 2 節を
新設する（既存節の順序・内容は維持）。証拠・対応表系の
節を隣接させる配置とした。

#### 節 1: `## OBS-01〜11 Owner/API/Feature/CI Case 対応表`

OBS ごとに 1 行の表。列は次の 8 列とする。

| 列 | 内容 | 語彙規則 |
| --- | --- | --- |
| OBS | OBS-01〜11 | `OBS-NN` |
| Owner Issue | 実装/検証の Owner Issue（URL 付き） | CLOSED 実装 Issue は実装正本として、OPEN gate Issue は検証 Owner として区別して記す |
| Module / crate | 公開 module・所在 crate | `ruviform-observability` の module 名。OBS-04 は `ruviform-plugin-host` 側であることを明記 |
| Feature | Cargo feature | `default=[]`・`logging`・`tracing-export`・`allocation-probe`・`integration`・なし の実在値のみ |
| 公開 API（代表） | 代表的な公開型/関数 | per-issue 正本への参照を主とし、代表型名のみ列挙（詳細は正本） |
| 正本 doc | per-issue 設計/test-spec/ADR への参照 | 実在する `docs/` パスまたは Issue URL |
| 既存 CI Case | lane・case 名・到達範囲 | 下記「CI Case 記法」に従う |
| 状態区分 | 要件状態と実行結果の分離 | 下記「状態語彙」に従う |

#### CI Case 記法

既存 lane のみを参照し、新規 workflow を提案しない。lane
名は `mise.toml` タスク名と `.github/workflows/` の job 名に
一致させる。観測 crate に関係する既存 lane は:

- `verify`（ci.yml・ubuntu+windows）: `fmt-check`/`lint-full`/
  `check-full`/`build-full`/`test-full`。`test-full` =
  `cargo test --workspace` で `default + integration`
  （ruviform-audio dev-dep による feature unify、実測済み）
  を 3 OS で実行。
- `verify-macos`（ci.yml・macos-latest）: `check-full`/
  `build-full`/`test-full`。
- `rt-verify`（ci.yml・ubuntu-latest）: `mise run test-rt` =
  `cargo test --workspace rt_`（strict 選択・0 件拒否）。
- `msrv-verify`（ci.yml・ubuntu-latest）: `mise run
  test-observability-msrv` = rust@1.89.0 で `cargo test -p
  ruviform-observability --all-features`。feature 付き
  module（`logging`/`tracing_export`/`profiling`）を compile
  + test する唯一の既存 lane。
- `consumer-verify.yml`: `verify-audiolens`/`verify-synvelis`
  （候補 tuple gate、fail-closed `summary`）。

#### 状態語彙

POLICIES の 2 軸を混同しない。

- 要件状態: `Required` / `Deferred` / `NotApplicable`
- 実行結果: `Pass` / `Fail` / `NotRun` / `Unsupported`

本表は「その Case を実行し得る既存 lane」と「本 slice 時点で
の要件状態」を記す。個別 SHA での実測 Pass は各 run の
artifact/summary が正本であり、本表から Pass を推論しない。
特に `ci.yml` は dispatch-only（daily-ci.yml が main SHA で
起動）であり、lane 定義の存在は実行済みを意味しない。

#### 節 2: `## Deferred 範囲の Owner/Trigger`

Deferred 項目ごとに 1 行の表。列は: Deferred 項目 / 現行の
扱い / 再開 Owner / Activation Trigger / 放置時リスク。
対象は少なくとも次の 4 項目:

- 実外部 Provider/Exporter（実 OTLP endpoint・外部 tracing
  backend 等）
- 実 OS GUI（実 Window 上の Observability 購読/表示）
- 実 Device（物理 Audio/MIDI 機器・実機負荷）
- 長時間 Profiler（samply 等の長時間・実負荷計測）

加えて feature compile matrix（`--no-default-features`・
各 feature 単独）は「Deferred ではなく #281 G0 の継続
gate 項目」として区別して記す（Deferred と Required-
NotRun を混同しない）。

### AL/SV 引き渡しの記述位置

対応表の OBS-06 行と、節 2 の直前または節 1 内の小節
「Consumer 独立 Oracle の引き渡し」として記す。内容:

- AL #127（Logging/Privacy/f64 精度）・SV #28
  （Subscriber/RT/DAW/精度）の独立 Headless 受入は
  RF #281（gate）と #294（candidate tuple 集約）経由で
  要求される。
- AL #138・SV #81 は CLOSED の導入履歴 Issue であり、
  継続受入 Owner ではない。再 Open しない。
- 一方の Consumer の Pass を他方へ転用しない（独立
  Oracle）。

## Data and Control Flow

文書のみの変更であり、データフローは存在しない。記述の
流れは:

1. `evidence/rf275/obs-owner-map-audit.md` に Issue 状態・
   crate/feature・CI lane の実測 snapshot を固定。
2. 統合設計正本に対応表・Deferred 表を追記し、各行は
   監査証拠と per-issue 正本への参照で裏付ける。
3. `issue-275-obs-owner-map` 文書チェーンが本設計の
   derives-from/implements 関係を記録し、test-spec が
   対応表の検証規則を固定する。

## Non-Functional Constraints

- **正確性**: Issue 状態・feature・module・lane 名は全て
  実測（`gh issue view`・`Cargo.toml`・`src/lib.rs`・
  `mise.toml`・workflow）に基づく。存在しない Issue/
  文書/module/feature を根拠にしない。
- **分離の維持**: 設計承認・実装完了・OS-L1 証拠・
  Windows 使用機能 L2/L3 証拠・Consumer 受入・
  candidate-SHA gate 完了を別段階として記述する。
- **非昇格**: `NotRun`/`Fail`/`Unsupported` を `Pass`
  に置き換えない。job 定義・skipped job・fake/headless
  結果から Pass を推論しない。
- **保護対象**: 「Workerへの共通契約」節を変更しない。
  歴史的 snapshot 節の記述は改変しない（新規追記のみ）。
- **スコープ**: `docs/` と `evidence/rf275/` のみ。
  `crates/`・manifest・workflow・`mise.toml` に差分を
  作らない。

## Risks and Trade-offs

- **Issue 状態の陳腐化**: #281/#294/#219・AL #127/SV #28 の
  状態は変動する。対策: snapshot 日付（2026-11-14）を
  明記し、将来の状態変化は gate 段階で再照合する旨を
  記す。
- **対応表の肥大化**: 公開 API を全列挙すると per-issue
  正本と drift する。対策: 代表型名のみ列挙し詳細は
  正本参照とする（既存設計書の「詳細は per-issue 正本」
  方針と同じ）。
- **CI lane の実行証拠との混同**: lane 定義と実測 Pass を
  区別する語彙規則を設けたが、読み手が lane 名から Pass
  を推論する余地は残る。対策: 状態語彙節と証拠 file の
  判定メモで明示的に否定する。
- **Deferred の乱用**: 何でも Deferred にすると Required
  が空洞化する。対策: Deferred は「利用要件未発生」のみ
  とし、feature compile matrix のような継続 gate 項目は
  Deferred と区別して `NotRun`/Required のまま記す。

## Deferred Design Concerns

- OBS-06（#281）以降の gate 実行・Consumer 受入・
  candidate-SHA gate の完了 | 本 slice は headless 監査・
  文書のみ | RF #281/#294/AL #127/SV #28 が継続 Owner |
  対応表が「実行済み」を意味すると誤読されるリスク
  （状態語彙で緩和）。
- feature compile matrix の実走 | 既存 lane に per-feature
  分離なし | #281 G0 で実施 | 分離 compile 破壊の見逃し。
- 実 OS GUI/実 Device/長時間 Profiler/実外部 Provider |
  利用要件未発生 | 各 Trigger（設計正本の Deferred 表に
  明記） | 実機条件下の問題の顕在化遅延。

## Verification Notes

- [ ] 対応表の OBS-01〜11 全行が実在する Issue・module・
      feature・lane に基づくことを `evidence/rf275/
      obs-owner-map-audit.md` と突合して確認する。
- [ ] `NotRun`/`Deferred` の記述が `Pass` に置き換わって
      いないことを diff で確認する。
- [ ] `mise run fmt-check` と `mise exec -- prek run
      --files <変更ファイル>` が成功すること。
- [ ] `doc-maintenance` の index 再生成と `doc-status`
      audit で整合を確認する。
