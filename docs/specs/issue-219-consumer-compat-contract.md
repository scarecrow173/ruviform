---
id: "SPEC-034QdDDqIQgpFoBr9E1Z1T"
type: "spec"
status: "proposed"
title: "Issue #219 両 Consumer 互換契約・影響判定・証拠要件と Cross-Repo CI Gate 仕様"
created: "2026-11-02"
updated: "2026-11-03"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "docs/discovery/issue-219-consumer-cross-repo-ci.md"
  changes:
    added: []
    modified: []
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
    - "docs/adr/cross-repo-ci-revision.md"
    - "docs/discovery/issue-194-m0-baseline-audit.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-219-consumer-cross-repo-ci.md"
    - "https://github.com/scarecrow173/ruviform/issues/219"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/synvelis/issues/28"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/audio-lens/blob/main/docs/designs/ruviform-dependency-ci.md"
    - "https://github.com/scarecrow173/ruviform/issues/257"
    - "https://github.com/scarecrow173/synvelis/issues/47"
    - "https://github.com/scarecrow173/synvelis/issues/48"
    - "https://github.com/scarecrow173/audio-lens/issues/131"
    - "https://github.com/scarecrow173/audio-lens/issues/132"
  defers: []
  deferred-by: []
---

# Issue #219 両 Consumer 互換契約・影響判定・証拠要件と Cross-Repo CI Gate 仕様

## What Should Be Built

Ruviform を Synvelis（設計段階 DAW）と AudioLens（高精度オフライン解析）の対等な共通基盤として並行開発するための **互換契約の運用仕組み** と **Cross-Repo CI Gate** を作る。

1. **影響判定の定型化**: Ruviform の公開 API / feature / behavior / dependency / CI 基盤の変更ごとに、Synvelis と AudioLens を独立した Consumer として `impacted` / `no impact` / `unknown` に分類し、根拠を Issue/PR に残すテンプレート運用。`unknown` を互換性確認済みとみなさない。
2. **証拠要件**: Ruviform 単体・Synvelis・AudioLens それぞれについて、該当テスト・対象 OS/feature・使用 ruviform revision・成功/失敗/未実測・許容例外と Rollback を分けて記録する形式。片方だけ緑で両方合格とはしない。
3. **M0〜M7 の Consumer 別受入証拠の可視化**: Gate ごとの Consumer 別証拠リンクと未実測事項を一覧化し、M1 着手 Gate（#199/#201）の現状を飛ばさない追跡形。
4. **Cross-Repo CI Gate**: Ruviform 候補 revision に対して両 Consumer の check/test/smoke を `workflow_dispatch` 先行で実行できる workflow。private credential は最小権限、秘密のログ露出を防ぐ。自動 trigger の有効化は初回実測後に判断する。

## Why Now

- #192/#193 で定義された Target Architecture 実行計画（M0〜M7）が始まり、#193 が Cross-Consumer 受入条件を追加済み。Consumer 別の証拠記録形式が未定義のままでは Gate 受入を判断できない。
- AudioLens 側の Git 依存・private CI 設計（`ruviform-dependency-ci.md`）は完成済みだが、credential が未プロビジョニングで実走実績がなく、Ruviform 側から見た Cross-Repo 検証も未設計。
- Issue #219 のチェックリストが「棚卸し・影響判定採用・Consumer 別証拠リンク・Cross-Repo CI の設計と実装可能範囲の構築」を要求している。

## Users and Value

- **Ruviform 実装者/レビュアー**: 変更が両 Consumer に与える影響を定型分類・記録でき、`unknown` の見落としを防ぐ。
- **Consumer 側実装者（Synvelis #28 / AudioLens #127）**: 追従更新 PR の検証経路が CI で再現可能になり、不適合時の責務切り分け（Ruviform 修正か Consumer Adapter 更新か）を相互リンクで追跡できる。
- **M0〜M7 の Gate 判定者**: Consumer 別の pass/fail/not run 証拠と未実測事項を一箇所で確認できる。

## Scope

### In Scope

- `.github/PULL_REQUEST_TEMPLATE.md`（全 PR 必須の Consumer 影響分類欄）
- `.github/ISSUE_TEMPLATE/`（Consumer 影響・追従用の Issue テンプレートと contact link 設定）
- `.github/workflows/consumer-verify.yml`（`workflow_dispatch` の Cross-Repo 検証 lane・証拠 summary 出力）
- 上記の設計・credential 一次設定手順・証拠記録形式を定める canonical 文書（本 spec・ADR・design・plan・task）
- M0〜M7 Consumer 別証拠の初期状態記録（discovery 済み）と、運用中の追跡形
- AGENTS.md の stale な CI 記述の修正（#199/#201 後の実態に合わせる）
- Issue 追記（2026-11-03・UI Component System 横断ゲート）の契約反映と、直近 merged 変更（`ui/*`・numeric FFT・reader EOF 修正・#219 CI 基盤）の Consumer 影響分類記録

### Out of Scope

- 各 Capability 実装（#184〜#198 等）や M0〜M7 Gate 判定そのもの — Owner は #192/#193 及び詳細 Issue のまま
- Consumer リポジトリ自体への変更（必要対応は Issue 起票案として報告）
- credential の実設定（Deploy Key 登録・Actions secret 投入は owner/admin 作業 — 手順のみ確定する）
- CI の自動 trigger 有効化（初回実測後に別判断）
- Cross-Repo 検証結果を required check へ昇格させる変更
- Synvelis 側の buildable workspace 化・E2E 実装（Synvelis #23/#28 の責務）
- 3 OS での Cross-Repo 検証 matrix（v1 は単一 OS、拡張は実測後）

## Requirements

- **R1 影響分類**: Ruviform の変更 PR は本文の Consumer 影響欄で両 Consumer を `impacted` / `no impact` / `unknown` に分類し、`impacted` なら対応 Issue リンク、`no impact` なら根拠、`unknown` なら未検証の理由と解消先を記す。`unknown` 残置のまま「互換性確認済み」扱いをしない。
- **R2 Issue 運用**: 共通 API/基盤の変更・不具合は Ruviform の Capability Issue へ、製品固有の Adapter/移行/回帰対応は該当 Consumer Issue へ起票し相互リンクする。影響なし時は根拠のみ記録し不要 Issue を乱発しない。
- **R3 依存追従**: Git 依存の特定 commit SHA を恒久的な更新停止策としてハードコードしない運用を明文化し、追従先既定ブランチと更新頻度を示す。更新 PR は Cargo.lock の resolved commit を更新し、複数 ruviform crate が想定外に別 revision に割れないことを機械確認可能にする。
- **R4 証拠形式**: Consumer 別の検証記録は少なくとも「検証対象（lane/test 名）・対象 OS/feature・ruviform resolved revision・結果（pass/fail/not run）・許容例外・Rollback」を含む。`not run` と `fail` は区別する。
- **R5 Cross-Repo workflow**: `workflow_dispatch` で起動し、ruviform 候補 revision（既定: dispatch 対象 ref の SHA、job env で `inputs.ruviform_rev || github.sha` として解決）と Consumer ref（branch/tag 名のみ）を入力に取り、Consumer scratch checkout 内で依存を候補 revision に切り替えて `check`/`test`/`smoke` lane を実行、job summary に Consumer 別の証拠表を出力する。`smoke` は headless 境界検証（audio-lens `check-cli-headless` ＋ `al-cli` の `ci` profile build）を指し、Golden/baseline 計測は含めない。
- **R6 credential**: private repo アクセスは最小権限の専用 credential（read-only Deploy Key 方式）とし、秘密は `secrets.*` 経由のみ・ログ/成果物に出力しない。未設定時は `not run` を明示して終了し、`unknown` を `pass` に見せない。
- **R7 非一律停止**: Consumer 別 job は独立し、片 Consumer の失敗/未検証が他 Consumer の検証・合否を止めない。共有契約の破壊/未検証が該当経路を止める場合は `not run (blocked: <理由>)` の注記形で記録する（結果語彙は R4 の `pass`/`fail`/`not run` に統一）。全 lane が `not run` の run は証拠ゼロとして workflow 失敗にし、緑表示で `unknown` を `pass` に見せない。
- **R8 M0〜M7 追跡**: 各 Gate の Consumer 別受入証拠リンクと未実測事項が一箇所で見える記録を持ち、M1 着手 Gate（#199/#201）の状態を飛ばさない。
- **R9 自動化は実測後**: Cross-Repo lane の自動 trigger・required check 化は初回実測と credential 稼働確認の後に判断する。
- **R10 UI 横断ゲート（Issue 追記 2026-11-03）**: `ui/*` 共通基盤を含む API/behavior/schema/renderer/性能差分の PR でも両 Consumer を独立評価する。評価軸は Consumer 固有: Synvelis は制御/Transport/RT 非阻害、AudioLens は f64・NaN 欠測・Validity/Provenance・bounded Dataset 可視化。Consumer 側未検証は追跡 Issue（Synvelis #47 controls・#48 viz、AudioLens #131 controls・#132 viz）へリンクする。本追記だけで consumer-verify 情報レーンを required check に昇格しない。

## Acceptance Criteria

- [ ] AC1: `.github/PULL_REQUEST_TEMPLATE.md` が存在し、両 Consumer の `impacted`/`no impact`/`unknown` 分類・根拠・リンク・検証欄を持つ。
- [ ] AC2: `.github/ISSUE_TEMPLATE/` に Consumer 影響・追従用テンプレートがあり、Ruviform↔Consumer 相互リンク欄を持つ。
- [ ] AC3: `.github/workflows/consumer-verify.yml` が `workflow_dispatch` で定義され、actionlint を通過し、`R5`/`R6`/`R7` を満たす（credential 未設定でも `not run` を明示して終了する）。
- [ ] AC4: 棚卸し（依存形態・required features・runner/toolchain・private repo アクセス）が canonical 文書に記録されている（本 spec の `derives-from` discovery）。
- [ ] AC5: M0〜M7 の Consumer 別受入証拠と未実測事項が文書で一覧化されている（初期状態は not run 可視化を含む）。
- [ ] AC6: credential 一次設定手順が最小権限の Deploy Key 方式で文書化され、秘密露出防止（key を echo しない・`secrets.*` のみ・pinned host key）が設計に含まれる。
- [ ] AC7: 実行した検証と未検証の区別が報告可能な状態（どの lane がどの revision で走った/走っていないか追跡可能）。
- [ ] AC8: Consumer 依存追従方針（追従先既定ブランチ・更新頻度・SHA 非恒久固定）が design/runbook に明文化され、manifest pin が更新停止策でないことが記録されている（R3）。
- [ ] AC9: M0〜M7 Consumer 別証拠 tracker の正本置き場（discovery `## Evidence` への dated snapshot 追記運用）が文書で指定されている（R8）。
- [ ] AC10: `AGENTS.md` の stale な `ci.yml` 記述（「`workflow_dispatch` 専用」）が現行の pull_request/push trigger・required checks 構成に修正されている。
- [ ] AC11: Issue 追記（2026-11-03）後の運用として、直近 merged 変更（`ui/*` 系 #258〜#263 のうち merged PR #267〜#271・numeric FFT #255/#265 系、および随伴する reader EOF 修正 #254・#219 CI 基盤 #224）の両 Consumer 分類・根拠・追跡 Issue リンクが canonical 文書（discovery `## Evidence` の dated snapshot）に記録され、検証済み/情報 lane/未実測の区別が明示されている（R10）。

## Deferred Design Concerns

- Cross-Repo lane の pull_request/schedule 自動 trigger | credential 稼働実績と頻度・コスト実測が必要 | 初回実走の証拠確認後 | 自動化しないままだと手動忘れで未検証が滞留する。
- Cross-Repo 検証の 3 OS matrix 化 | audio-lens 側 CI と同じ対象 OS 網羅が望ましい | v1 の実走が安定し消費時間が見えてから | OS 依存の不整合を単一 OS で見逃す。
- Consumer Golden/`baseline` 計測 lane（audio-lens `mise run baseline` 系） | 実行時間と fixture（LFS）要件が未測 | v1 smoke/check 実走後 | 性能・Golden 退行を check/test だけでは拾えない。
- GitHub App / fine-grained PAT への credential 移行 | Deploy Key で現要件を満たす | 監査・期限管理要件の変化 | 長期の鍵管理負債。
- audio-lens #127 の `rev`→`branch` 追従形移行後の override 経路更新 | 現行は rev-pin が正 | Consumer 側の依存形態変更時 | CI の override が実運用経路と乖離する。
- Cross-Repo 結果を required check に昇格 | 現状は情報 lane（ルール6の一律停止禁止と整合） | 両 Consumer CI が安定稼働した後 | 未稼働のまま required 化すると全 PR が止まる。

## Sources and Relations

- 主 Issue: [ruviform#219](https://github.com/scarecrow173/ruviform/issues/219)。上位計画: [#192](https://github.com/scarecrow173/ruviform/issues/192)/[#193](https://github.com/scarecrow173/ruviform/issues/193)（Cross-Consumer 受入条件）。Consumer 追従 Issue: [synvelis#28](https://github.com/scarecrow173/synvelis/issues/28)/[audio-lens#127](https://github.com/scarecrow173/audio-lens/issues/127)。
- 棚卸し・実測の正本: [docs/discovery/issue-219-consumer-cross-repo-ci.md](../discovery/issue-219-consumer-cross-repo-ci.md)。
- 方式決定の記録: [docs/adr/cross-repo-ci-revision.md](../adr/cross-repo-ci-revision.md)。
- 保護する契約（Issue 本文）: [#197 Reader/Prefetch](https://github.com/scarecrow173/ruviform/issues/197)、[#189 Time](https://github.com/scarecrow173/ruviform/issues/189)、[#191 WAV](https://github.com/scarecrow173/ruviform/issues/191)、[#198 Offline](https://github.com/scarecrow173/ruviform/issues/198)、[#181 UI/Job](https://github.com/scarecrow173/ruviform/issues/181)。これら Capability 契約の実装 Owner は変更しない。
- UI 横断ゲート（Issue 追記 2026-11-03）: 設計正本 [ruviform#257](https://github.com/scarecrow173/ruviform/issues/257)・実装 #258〜#263（merged PR #267〜#271・#262 は deferred lane のため対象外）。Consumer 側受入: [Synvelis #47 controls](https://github.com/scarecrow173/synvelis/issues/47)・[#48 viz](https://github.com/scarecrow173/synvelis/issues/48)、[AudioLens #131 controls](https://github.com/scarecrow173/audio-lens/issues/131)・[#132 viz](https://github.com/scarecrow173/audio-lens/issues/132)。
