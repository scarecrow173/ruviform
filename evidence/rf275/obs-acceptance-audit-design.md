# 内部 Design — Issue #275 OBS-01〜11 受入照合台帳と Consumer Gate 割当

- 文書種別: 内部 design（`obs-acceptance-audit-spec.md` の設計相当）
- 日付: 2026-11-14
- 対象 Revision（監査基準）: `20d8e7c23bc365e26060a4ef82d463bb7eed38bc`

## Context

前 slice（`issue-275-obs-owner-map`）が設計正本へ追加した
対応表は「どの lane がその Case を実行し得るか」と要件状態を
記すが、Issue #275 現行受入が要求する以下を所有しない:

- OBS ごとの**必要 OS-L1 Fixture の具体的列挙**（test target・
  test 数・必要 feature・実効 feature unify）
- **未対応 Case**（既存 lane/fixture が存在しない Case）の
  明示的棚卸し
- **照合結果の対象 Revision 付き記録**（どの SHA のどの
  lane/実行で何が観測されたか）
- **Consumer Gate の Issue 割当表**（AL/SV/Windows L2/L3 を
  どの Issue が Owner として受け持つか）

本設計は台帳の構造・語彙・証拠規則・canonical doc への
差分方針を定める。per-module の詳細契約は per-issue 正本
（#276〜#293 の設計・test-spec）に委ね、本 slice は索引・
照合・状態記録のみを所有する。

## Scope

### In Scope

- `evidence/rf275/obs-acceptance-ledger.md`: 台帳正本。
- `docs/designs/issue-275-observability-integration-design.md`
  への追記型セクション（台帳参照 + Gate 割当要約）。
- 本 doc chain（spec/design/plan/task、`evidence/rf275/` 内）。

### Out of Scope

- コード・CI・manifest・canonical docs chain の変更。
- 新規 CI lane・workflow の提案（既存 lane のみを証拠源とする）。
- Consumer repo での実測、Issue の起票/更新/Close。

## Components and Boundaries

### 台帳ファイル構成（`evidence/rf275/obs-acceptance-ledger.md`）

1. **ヘッダ**: 監査日・対象 Revision・前提正本（PR #521/#547・
   owner-map slice）・方法・語彙定義。
2. **§1 OBS-01〜11 照合表**: OBS ごと 1 行。列:
   OBS / Owner Issue / Module・crate / Feature / 公開 API（代表）/
   OS-L1 Fixture / Case / 照合結果（対象 Revision 付き）/
   未対応 Case。
3. **§2 Fixture × lane 証拠行列**: test target 単位で
   必要 feature・compile される lane/OS・観測結果・証拠
   （run URL・job log・ローカル実行）を記す。
4. **§3 Consumer Gate 割当表**: Gate / Case 組 /
   RF 側 Owner / Consumer Owner / 実行経路 / 状態。
5. **§4 SHA 証拠引き継ぎ規則と NotRun 棚卸し**:
   `365c74ac..20d8e7c2` の crate 別 diff 結果と、HEAD で
   未実走の項目一覧。
6. **§5 判定メモ**: Pass 推論の禁止・ローカル実行と統合
   受入の区別・Deferred の扱い。

### OS-L1 Fixture の定義

ある OBS の OS-L1 Fixture とは、その能力の契約を検証する
**既存の test target・test・example**の集合とする。単位は
`tests/*.rs`（`required-features` 明記）・`src/<module>/
tests.rs`・inline `#[cfg(test)]`・`rt_*` 名 test・
`examples/`（compile fixture）・plugin-host 側 test
（OBS-04）。test 数は `#[test]` 属性の実測値。

### Feature unify の実効値

- `cargo test --workspace`（`test-full`）: default +
  `integration`（`ruviform-audio` dev-dep による unify、
  owner-map audit で `cargo tree` 実測済み）。
- `cargo test -p ruviform-observability --all-features`
  （`test-observability-msrv`、rust@1.89.0・Linux）:
  全 feature 有効。logging/tracing-export/allocation-probe
  の test target を compile する唯一の既存 lane。
- `cargo test --workspace rt_`（`test-rt`/`rt-verify`、
  Linux）: `rt_` 名 test を substring 選択。feature 付き
  test target は compile されない。
- ローカル `mise run test <pkg> [--features]`:
  補助証拠（canonical = mise task 経由だが CI lane ではない）。

### SHA 証拠引き継ぎ規則

- 証拠は「SHA・OS・feature・test・期待/観測・run URL/ログ」
  の組で記す（POLICIES §3）。
- `git diff <evidence-sha>..<target-sha> -- <crate paths>`
  が空である crate についてのみ、先行 SHA の CI 証拠を
  「同一コードへの証拠」として引用できる。diff がある
  crate（本監査では `ruviform-plugin-host`・
  `ruviform-plugin-core`）の証拠は stale とし、対象
  Revision では `NotRun` とする。
- 対象 Revision 自体の CI 未実施分は `NotRun`（Pending
  ではなく監査時点の事実として `NotRun`）。merge 後 SHA の
  Daily/手動 Full CI が統合検証の証拠となる。

### Consumer Gate 割当の語彙

Issue #275・#281 現行正本に基づく割当:

| Gate | Case 組（#281） | RF 側 Owner | Consumer Owner | 実行経路 |
| --- | --- | --- | --- | --- |
| AL Logging/Privacy/f64 | OBS-LOG・OBS-PRIV（Logging/機密）・OBS-AL（f64/長尺/CLI/Dataset） | #281（gate 判定）・#294（candidate tuple 集約）・#219（運用契約） | AL #127 | consumer-verify `verify-audiolens` lanes（check-cli-headless/test-full/test-ui/GUI-CONTRACT/GUI-SEMANTIC/extra）+ AL repo 側 Oracle |
| SV RT/DAW/GUI | OBS-RT・OBS-SV（Record/Plugin/Project）・GUI（利用機能 L2） | 同上 | SV #28 | consumer-verify `verify-synvelis` lanes（+ `test-rt` lane）+ SV repo 側 Oracle・SV #114/#377 等の native lane |
| Windows 利用機能 L2/L3 故障・RT/Privacy | OBS-LOG/OBS-PRIV/OBS-RT のうち Windows で提供・利用する機能に限定 | #281/#294（集約）・#293（API 境界）・#313/#294 real-window lane 設計 | AL #127・SV #28（製品機能 Owner） | 各 Consumer repo の Windows L2/L3 Case + RF real-window lanes。未利用機能は Required にしない |

- Closed 履歴 Issue（AL #138・SV #81）は継続受入 Owner ではなく
  再 Open しない。
- 一方の Consumer の Pass を他方へ転用しない（独立 Oracle）。

### canonical doc への差分方針

`docs/designs/issue-275-observability-integration-design.md`
の `## Deferred 範囲の Owner/Trigger` 節と
`## 証拠項目対応` 節の間（`#101/#103 前提の更新記録` の前）
に、新規節 `## OBS-01〜11 受入照合台帳と Consumer Gate 割当`
を**追記のみ**で追加する。内容は (a) 台帳ファイルへの参照、
(b) Issue #275 残受入 2 件が台帳で処理された旨、
(c) Consumer Gate 割当の要約表（上表の圧縮版）、
(d) 結果語彙と「台帳は実行済みを意味しない」注意。
既存節・「Workerへの共通契約」節・歴史的 snapshot は
改変しない。`docs/specs/issue-275-obs-owner-map.md` と
`docs/designs/issue-275-obs-owner-map-design.md` は前 slice の
凍結記録であり本 slice では変更しない。

## Data and Control Flow

文書のみ。流れは:

1. `gh issue view`/`gh run view`/`git diff`/`mise run test`
   で証拠を収集し `evidence/rf275/` に記録。
2. 台帳が OBS ごとの照合結果を固定。
3. 設計正本へ参照節を追記し、gate 段階（#281/#294/AL#127/
   SV#28）が台帳を引く構造にする。

## Non-Functional Constraints

- **正確性**: Issue 状態・feature・module・lane・test 数・
  run 結果は全て実測。存在しないものを根拠にしない。
- **非昇格**: `NotRun`/`Unsupported`/`Fail` を `Pass` に
  置き換えない。lane 定義・job green・skipped・fake/
  headless 成功から個別 Case の Pass を推論しない。
- **分離**: ローカル実行・CI lane 実行・統合受入・
  consumer 製品受入を別段階として記す。
- **保護対象**: コード・manifest・CI・canonical docs 既存
  記述・「Workerへの共通契約」節を変更しない。

## Risks and Trade-offs

- **SHA 証拠引き継ぎの誤用**: diff 空の確認を crate 単位で
  限定し、依存 crate（rt/media-core/plugin-core）の diff も
  別途確認した（plugin-core は diff あり → OBS-04 の
  plugin-host 側証拠は stale 扱い）。
- **台帳の肥大化**: test 名の全列挙は per-issue test-spec
  との drift を生む。test target 単位 + 代表 test 名 +
  test-spec 参照に留める。
- **consumer-verify の job-level pass の誤読**: tuple の
  job success は個別 OBS Case の Pass を意味しない。
  lane 記録（pass/not_run/unsupported 区別）と artifact
  が正本である旨を明記する。

## Deferred Design Concerns

- 対象 Revision（merge 後 SHA）での Daily/手動 Full CI・
  consumer-verify 実走 | 日次 CI・#294 dispatch | 台帳の
  `NotRun` が統合受入を妨げないよう、証拠引き継ぎ規則で
  先行 SHA 証拠を明示的に引用可能にした。
- Feature compile matrix（`--no-default-features`・各
  feature 単独）| #281 G0 継続 gate | 既存 lane なし。
- 実 OS GUI/実 Device/長時間 Profiler/実外部 Provider |
  利用要件未発生（owner-map の Deferred 表が正本） |
  各 Trigger | 台帳では Deferred として Owner/Trigger を
  再確認するのみで新規割当はしない。

## Verification Notes

- [ ] 台帳の全行が実在する Issue/module/feature/lane/
      test target に基づくことを `git diff`・`gh` 出力・
      `Cargo.toml`/`src/`/workflow/mise 走査と突合。
- [ ] `NotRun` が `Pass` に置き換わっていないこと、
      SHA 引き継ぎが diff 空の crate に限定されていること
      を diff で確認。
- [ ] `mise run fmt-check` 成功。
- [ ] 設計正本への差分が追記のみであることを `git diff` で
      確認。
