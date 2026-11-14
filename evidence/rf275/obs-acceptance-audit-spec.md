# 内部 Spec — Issue #275 OBS-01〜11 受入照合台帳と Consumer Gate 割当（docs audit slice）

- 文書種別: 内部 spec（doc-driven-dev-graph フローの spec 相当。
  対象リポジトリ内の内部文書として `evidence/rf275/` に配置。
  canonical docs chain は編集範囲外のため本ファイルが内部正本）
- 日付: 2026-11-14
- 対象 Issue: https://github.com/scarecrow173/ruviform/issues/275
- 対象 Revision（監査基準）: `20d8e7c23bc365e26060a4ef82d463bb7eed38bc`
  （監査時点の origin/main HEAD。merge 後 SHA の CI 証拠は Daily/手動
  Full CI が正本）

## What Should Be Built

Issue #275 の残り受入条件 2 件を、headless 監査・文書スライスとして
処理する。

1. **OBS-01〜11 受入照合台帳**: 各 OBS について公開 API・
   Cargo Feature・Owner（実装 Issue・gate Owner）・必要 OS-L1
   Fixture（test target/test・必要 feature・実行 lane/OS）・
   Case・未対応 Case を一意に照合し、照合結果を
   Pass/Fail/NotRun/Unsupported と対象 Revision 付きで台帳化する。
   結果が未実走のものは `NotRun` と記す。
2. **Consumer Gate 割当**: AL Logging/Privacy/f64・SV RT/DAW/GUI
   の独立 Consumer Gate と、Windows 利用機能の L2/L3 故障・
   RT/Privacy Gate を RF #281/#294・AL #127・SV #28 に割り当てて
   明記する。

## Why Now

- 前 slice（`issue-275-obs-owner-map`、PR 統合済み）は Owner/
  API/Feature/CI lane の対応表と Deferred Owner/Trigger を
  設計正本へ追加したが、Issue #275 の現行受入条件が要求する
  「必要 OS-L1 Fixture・未対応 Case の一意照合」と「結果の
  Pass/Fail/NotRun/Unsupported 判定・対象 Revision 付き台帳」
  「Consumer Gate の Issue 割当」は未作成。
- 監査基準 SHA では対象 crate が直近 Full CI 成功 SHA
  （`365c74ac`、run 43439434737）と diff ゼロであることを
  `git diff` で確認でき、証拠の SHA 引き継ぎ根拠を固定できる。

## Scope

### In Scope

- `evidence/rf275/obs-acceptance-ledger.md`（新規・台帳正本）
- `evidence/rf275/obs-acceptance-audit-{spec,design,plan,task}.md`
  （本 doc chain・内部文書）
- `docs/designs/issue-275-observability-integration-design.md`
  への追記型差分（台帳への参照と Consumer Gate 割当の要約。
  既存記述の書換えなし）
- 実測証拠の収集: `gh issue view`/`gh run view`（CI run・job
  log）、`git diff`（SHA 間同一性）、`mise run test` による
  対象 package のローカル実行（補助証拠として記録）

### Out of Scope

- `crates/`・`Cargo.toml`・`Cargo.lock`・`mise.toml`・
  `.github/` の変更（コード・CI 変更なし）
- canonical docs（`docs/specs|designs|plans|tasks|test-specs|
  impl`）への新規文書追加・index 再生成（編集範囲外。
  doc chain は `evidence/rf275/` 内部文書で代替）
- MSRV 系タスクの実行（daily CI の管轄）
- F: ドライブへのビルド/中間生成物の配置
- `.devin/ci-metrics*` の変更（無関係変更として保持）
- Consumer repo（audio-lens/synvelis）での実測・Issue 更新
- GitHub blockedBy の登録、Issue 起票/Close

## Requirements

- OBS-01〜11 の全行について、実在する公開 API/module・
  Cargo feature・Owner Issue（URL 付き）・test fixture
  （test target・test 名・必要 feature）・実行 lane/OS を
  実測に基づき一意に記述する。存在しない module/feature/
  lane/Issue を根拠にしない。
- 結果語彙は POLICIES §3 に従う: 要件状態
  `Required`/`Deferred`/`NotApplicable`、実行結果
  `Pass`/`Fail`/`NotRun`/`Unsupported`。job 定義・green・
  skipped から個別 Case の Pass を推論しない。
- 証拠には対象 Revision・OS/Arch・Feature/test 名・実行
  コマンド・run URL/ログ・期待/観測を記す。ローカル実行は
  ローカル実行として区別し、統合受入の Pass にしない。
- SHA 間のコード同一性（`git diff` 空）を確認した場合のみ、
  先行 SHA の CI 証拠を「同一コードに対する証拠」として
  引用し、対象 Revision での未実走は別途 `NotRun` と記す。
- AL #127・SV #28 の独立 Consumer Gate、Windows 利用機能の
  L2/L3 故障・RT/Privacy Gate の割当先を Issue 番号・
  Owner 責務・実行経路（lane/consumer-verify）付きで一意に
  記す。一方の Consumer の Pass を他方へ転用しない。
- Closed 履歴 Issue（AL #138・SV #81）を再 Open する記述、
  `NotRun`/`Fail` を `Pass` に置き換える記述を行わない。

## Acceptance Criteria（本 slice）

- [ ] `evidence/rf275/` に OBS-01〜11 の公開 API・Feature・
      Owner・OS-L1 Fixture・Case・未対応 Case を一意照合した
      受入照合台帳があり、各結果が Pass/Fail/NotRun/
      Unsupported と対象 Revision 付きで記録されている。
- [ ] AL Logging/Privacy/f64・SV RT/DAW/GUI の独立
      Consumer Gate と Windows 利用機能 L2/L3 故障・
      RT/Privacy Gate が RF #281/#294・AL #127・SV #28 に
      割り当てられている。
- [ ] 統合設計正本への差分は追記のみで既存記述を改変して
      いない。`crates/`・manifest・CI・`.devin/` に差分が
      ない。
- [ ] `mise run fmt-check` が成功する。
