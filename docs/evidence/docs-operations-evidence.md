---
id: "EVI-gLjCsFayK6t3vsrDL6yoI4"
type: "evidence"
status: "draft"
lifecycle: "draft"
title: "ドキュメント運用・エージェント基盤・スクリプト・ジョブ実行の記録"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/3", "https://github.com/scarecrow173/ruviform/issues/19", "https://github.com/scarecrow173/ruviform/issues/23", "https://github.com/scarecrow173/ruviform/issues/41", "https://github.com/scarecrow173/ruviform/issues/172", "https://github.com/scarecrow173/ruviform/issues/184", "https://github.com/scarecrow173/ruviform/issues/192", "https://github.com/scarecrow173/ruviform/issues/193", "https://github.com/scarecrow173/ruviform/issues/276", "https://github.com/scarecrow173/ruviform/issues/312", "https://github.com/scarecrow173/ruviform/issues/419", "https://github.com/scarecrow173/ruviform/issues/456", "https://github.com/scarecrow173/ruviform/issues/553", "https://github.com/scarecrow173/ruviform/issues/558", "https://github.com/scarecrow173/ruviform/issues/559", "https://github.com/scarecrow173/ruviform/issues/560", "https://github.com/scarecrow173/ruviform/issues/562", "https://github.com/scarecrow173/ruviform/issues/586", "https://github.com/scarecrow173/ruviform/issues/590", "https://github.com/scarecrow173/ruviform/issues/627", "https://github.com/scarecrow173/ruviform/issues/628", "https://github.com/scarecrow173/ruviform/issues/692", "https://github.com/scarecrow173/ruviform/issues/716", "https://github.com/scarecrow173/ruviform/issues/727", "https://github.com/scarecrow173/ruviform/issues/744"]
  informs: []
  explains: []
  supersedes: []
---

# ドキュメント運用・エージェント基盤・スクリプト・ジョブ実行の記録

この文書は、ドキュメント運用・エージェント基盤・スクリプト基盤・ジョブ実行に関する検証と実行の結果を記録したものである。記録は実施した検証手順と観測結果を中心とし、実行されなかった検証は NotRun または Deferred として明記する。各項目には出典の Issue 番号を付記する。

## プレースホルダー除去と CI 実ディスパッチの検証

Issue #3 では、テンプレート由来のプレースホルダー残存を実測で確定してから書き換えを行った。`git ls-files` が返す 55 ファイルを母集団に複数系統の git grep を実行し、書き換え対象は README.md の 3 行、AGENTS.md の 5 行、apm.yml の 2 行、および mise.toml L8 のコメント（リテラル非該当だが意味的に残存）に確定した。apm.lock.yaml の 198 hit はすべて APM パッケージ由来のパスを指す生成物として除外し、docs/ 配下の hit は履歴記録として、.pre-commit-config.yaml の hit は無変更既定のレビュー対象として分類した（Issue #3）。

AGENTS.md の改稿後検証では、「template/テンプレート」と「派生リポジトリ/派生プロジェクト」の git grep がともに 0 hit、AGENTS.md が記載する mise タスク 12 件がすべて mise.toml の `[tasks.*]` に実在すること、`prek run --all-files` が exit 0、ci.yml に LFS 取得がないこと、LICENSE が存在しないことを確認した。doc-status/doc-impl の監査も findings 0 だった（Issue #3）。

README.md の改稿後検証では、「Use this template」「project.template」「テンプレート|template」の grep がすべて 0 hit、節構成・検証済みコマンド・区分ラベルの grep 一致、`test ! -f LICENSE`、prek exit 0、doc 監査 findings 0 を確認した。LICENSE 存在・未作成クレートの実在・RT-safe の断定・CI 実ディスパッチ成功の記載・macOS/Linux 検証済み・再利用手順・存在しないリンクという禁止記載 7 項目はすべて非該当だった（Issue #3）。

終端スライスでは F-1〜F-8 を実行してベースラインと比較し、V-1〜V-11 の判定基準をすべて満たした。prek・fmt-check・check・lint・test・verify-ci・`apm install --frozen`・audit_docs.js がすべて pass し、非 docs/ 配下の残存 hit は 0、apm.lock.yaml の残存 198 hit も APM 生成物パスのみでパターン外 hit は 0 だった（Issue #3）。

追検証として main で GitHub Actions を 2 回ディスパッチした。初回 run 39249234364 は、fresh な Ubuntu runner の rustup minimal profile が rustfmt と clippy を含まないため `cargo-fmt is not installed` で失敗した。mise.toml の rust エントリに `components = ["rustfmt","clippy"]` を pin して lockfile を再生成し（PR #7、merge 6aeadf6）、run 39249585284 は約 30 秒で全ステップ green となった。あわせて `df`/`du` の実測で target/ci が 7.1M、ディスク使用率 84% であることを記録した（Issue #3）。

この失敗は Windows ローカルでは既存インストールに同梱されるため顕在化しなかったクロスプラットフォーム差異である。手動ディスパッチによる検証設計が、静的な確認だけでは捕捉できない実在の問題を検出した例として記録されている（Issue #3）。

## 文書監査・索引・契約文書の検証

監査スクリプトのグリーンと、変更ファイルが front matter の changes 宣言スコープに収まることの確認は、文書系変更の受入条件として用いられた。Issue #19 では audit_docs.js --type all・audit_impl_record.js・audit_experiment_log.js がすべて findings 0 件で、`git diff --name-only` の変更ファイル群が changes 宣言スコープ内であることを確認した。実績の個別判断として、mise.toml は先行タスクで整合済みのため変更不要、生成マーカー未付与の索引ファイルは索引形式どおり手編集としたことが記録されている（Issue #19）。

Issue #586 では、front matter 適合化と索引再生成の効果と副作用が監査数値として記録された。索引の apply 後に plan が in-sync、引数なし apply が exit 0 であることを確認し、audit_impl_record.js で invalid-front-matter が 104 件から 0 件、index-not-managed warning が 1 件から 0 件に解消した。error 総数は 187 件から 230 件になった（Issue #586）。

error 総数の増加は missing-section が 83 件から 230 件に増えたことによる。これは front matter 適合化で新たに検査対象となった既存文書の、以前から存在した不足分が可視化されたものであり、新規に発生した欠損ではない。対象 30 ファイルの本文が HEAD と byte 一致・既存 id 不変であることをスクリプトで確認している（Issue #586）。

可視化された不足は Issue #590 で本文補完により解消した。missing-section が 230 件から 0 件になり、broken-relation-link 5 件・missing-experiment-link 6 件は baseline を超えないことを確認条件とし、変更 50 ファイル全件で front matter ブロックが HEAD と byte 一致・canonical 見出しの重複なしであることをスクリプトで確認した。review-protocol（Reviewer A 協調・Reviewer B 敵対の独立パスのあと Validator・Judge）は ACCEPT WITH NON-BLOCKING NOTES で、CONFIRMED 指摘は P3 の簿記未完了のみで修正済みである。bare prose の Related Experiments、HTML コメントのみの Adopted/Rejected、未コミット .log 参照という既存負債は別 Issue 候補として記録された（Issue #590）。

3 リポジトリにまたがる POLICIES.md 改訂の照合は Issue #627 で行われた。改訂が RF PR #634（3b8b7322）・SV PR #449（8aa5cb0）・AL PR #212（eb62440）として merge 済みであることを確認し、baseline SHA・取得コマンド・行単位 diff・残差分（audio-lens の行末 `\` スタイルなど）を証跡として記録した。POLICIES.md 本文は変更せず照合のみで完了とした（Issue #627）。

Issue #627 では、review-protocol の blocker なしを design gate（approved design 必須）の代替として用いた。Reviewer A/B の独立パスで検出した指摘（陳腐化した「承認前」表現、environment_kind の適用範囲、resolved_ruviform_sha の Ruviform 自身に対する扱い）を修正して spec/design/test-spec/plan を approved に遷移させた。doc-driven-dev-graph router は repo 全体の既存 broken relation により blocked を返したが、本 slice 範囲外の既存状態として記録し、索引の整合は別の保守手順で担保した。`prek run --all-files` の Pass と、変更範囲が許可パスに限定されることの git status 確認も実施した（Issue #627）。

docs-only の変更では検証 lane を絞る運用が Issue #727 で確認された。doc_status lint は baseline 採取済みで、新規文書由来の finding は index 未同期に由来する index-missing-entry・orphan-index・orphan-navigation の warning のみであり、traceability-missing-upstream と inconsistent-reciprocal-relation は先行 Issue と同型の許容 warning と記録した。route_graph は新規文書追加後も同等の blocked で finding 増加なし、`mise run fmt-check`・`prek run --files`・adr-doc 系スクリプトは Pass だった。cargo 系 lane・`*-msrv`・`*-full`・3 OS は docs-only のため POLICIES.md §5 に基づき NotRun とした（Issue #727）。

## 採用判断・依存構造・Issue 状態の照合

cpal の採用判断は、検証項目と証跡の対応づけで確定された。オーディオデバイス backend として cpal を採用し、選択 ADR を proposed から accepted へ昇格した。検証 6 項目（audio-core の公開 API に cpal が露出しないこと、feature 分離、実機での play_sine による列挙・開始・コールバック到達、assert_no_alloc、第 2 backend を追加可能な trait の形状、mise run 系タスクの動作）をすべて証跡に対応づけ、再評価トリガ（duplex 要件の発生、cpal の再停滞、致命的な挙動不良）はいずれも未観測と記録した（Issue #23）。

小規模修正では workspace 全体のグリーンを受入基準とした。Issue #41 では `cargo test -p ruviform-audio-core` で 32 unit と 20 doctest、`-p ruviform-media-core` で 30 unit と 24 doctest、`cargo test --workspace` で計 141 件がすべて pass し、`cargo clippy --workspace --all-targets -- -D warnings` が 0 warnings、fmt-check が clean だった。新規テストとして try_new の失敗経路・SpeakerMaskSet の doctest・`new` の should_panic 契約維持を追加した（Issue #41）。

依存構造の非循環性は cargo metadata の静的読みで数値化された。Issue #192 では、TA-001〜TA-012 の invariant を現行 API・crate・設計判断・実装 Issue・既存 OS-L1 テスト・未達の Headless Gap へ 1 対 1 で対応づける正本を作成し、索引側の tracking 設計へ参照を追記した。`cargo metadata --locked` の実行結果は次のとおりである（Issue #192）。

- default: exit 0、42 member、98 normal+build edge、26 dev edge、循環 0
- all-features: exit 0、103 normal+build edge、129 全 kind edge、循環 0

`kind: null` は normal 扱い、dev/build/target は dep_kinds で区別し、dev-dependency の上位参照（plugin-host→audio 等）は規定どおり循環と扱わなかった。

受入 owner の remap では、Issue 状態を実測で照合した。`gh issue view --json state` で ruviform・audio-lens・synvelis の参照 16 Issue の state を照合し、記載の OPEN/CLOSED とすべて一致することを確認した。#243/#313 は 2026-11-14 の snapshot 時点では OPEN で現行は CLOSED であり、追記では日付根拠付きで陳腐化を明記した。#628〜#631、#294、AL #127、SV #28 はすべて OPEN の未実装 Harness であり、対応表は受入 owner の対応づけのみを記録し、Case の実走や Pass を意味しない（NotRun 維持）と明記した（Issue #192）。

remap 表の owner 割当は、Windows 仮想 Endpoint→#629、正規 Plugin ABI→#630、実 GUI→#631、deterministic runtime→#628、OS-L1・両 Consumer 独立検証→#294・AL #127・SV #28、方針基盤→#627（CLOSED）と記録した。Issue #193 側では M3（#253→#629+#630）・M4（#388/#389/#532→#631+#630）・M5（#251→#629）・M7/GUI lane（→#631+#630）のみ Harness owner を記し、L2/L3 対象のない M0/M1/M2/M6 は「なし/変更なし」と記した。両正本の新規節は 142 行の byte 一致を diff で確認した（Issue #192）。

Issue 状態の census では、3 リポジトリの Issue 状態を `gh issue list --state all --json` と `gh issue view --json body` で取得し、計画関連の全 Open Issue（RF 25 件超、SV・AL 各件）の「着手条件（Blocked by）」節が「なし」であることを照合した。実 OS GUI/Device/Plugin Gate は統合・受入側の節に分離済みで Blocked by へ転記しない構造とし、新規 Script/Creative Branch 群は M0〜M7 の Blocked by にしない並行 Lane として接続する規則を採用した。merge 後の同日再実測で #243/#313 の CLOSED 遷移を検出して文書を更新した（Issue #193）。

2026-11-14 snapshot の census でも、M0〜M7 実装 Issue #184〜#198、M1 着手 Gate #199/#201、IO/OBS/GUI Test/GUI lane 基盤、3OS OS-L1 CI #598 がすべて CLOSED で、確認した全 Open Issue の着手条件は「なし」だった。Closed/Deferred/NotRun を Blocked by と誤記しない構造は、2026-11-13 のゲート正規化で確立されたものである（Issue #184）。

GPUI の採用判定では、verdict を後継 ADR へ逆反映した。最終採用判定は outcome=CONDITIONAL（L1 のみ評価済み、L2/L3 非保証、標準 Driver 方針維持、Native Dock 無条件採用不可、OS 別 Capability 宣言記録）で、後継 ADR（status: proposed 不変）へ逆反映した。状態は OS-L1 Pass・Windows L2 部分実証・Windows L3/Gate D 未達・macOS/Linux L2/L3 Deferred・Gate C 性能/Gate E NotRun であり、#532 の DComp `topmost=true` は open の Blocker として維持し、native_surface_slot/ime/focus release は宣言値として記録した。Follow-up として #532（Windows Native Dock 画素の根本 Blocker）・#388/#389（Native Dock 残件）・#243/#313（実 GUI 残件、非 Blocking 既知制約）・Gate C 性能/Gate E の Consumer Oracle（NotRun のまま AL #166・SV #176 へ後続）を整理した（Issue #419）。

## テスト基盤と受入テストの検証

契約試験の整備は、test-spec との対応を明示する形で記録された。Issue #276 では observability の logging 機能について、test-spec の E1〜E6・E11〜E12 を pin する 19 件の契約試験（dual filter routing、JSONL/Text 内容、retention、失敗分類、scoped dispatch、health 非再帰など）を tests/logging.rs に実装し、examples/logging_host.rs で Host の dispatch 構築から shutdown までの流れを実演した。`cargo test -p ruviform-observability --features logging` は 13 unit・19 contract・3 doctest がすべて pass、MSRV rust@1.89.0 の check/clippy、`--no-default-features` で normal dependency ゼロを確認し、origin/main は 0 commit behind で merge 不要だった（Issue #276）。

UI テスト専用の制御基盤は、本番 crate を変更せずに追加された。Issue #312 では crates/ruviform-ui-test を新設し、`rvf.uitest.v1` ワイヤ、stdio/loopback-TCP トランスポート、TestAgent、TestIo/HeadlessIo/FakeHost、fixture、journal、TestDriver、dummy_app を実装した。`cargo test -p ruviform-ui-test` は headless contract 32 件と e2e_process 15 件の計 47 件が pass、`mise run test` は 120 の test binary すべてで 0 failed だった。差分は root Cargo.toml の members、Cargo.lock、crates/README.md のみで、ruviform-ui と ruviform-ui-webview のソース変更はゼロだった（Issue #312）。

進捗通知の wire 契約は専用テストで固定された。Issue #172 では `mise run fmt/check/lint/test/build` と `check-ui-webview`/`lint-ui-webview`/`test-ui-webview`（scheme 7 tests）がすべて exit 0 で、`job_progress_detail_reaches_the_wire` テストが detail 付き report の伝播・wire 形状（detail キーの有無）・detail なし report の `detail: None` 維持を検証した。sticky detail 方式は「過剰」として却下し、採用実験は行われなかった（Issue #172）。

受入テストの感度確認では、テストが実際に失敗し得ることを先に示した。Issue #456 では終端 receipt の生成を一時的に外すと runtime・Fake・GPUI の新規テストがすべて失敗することを確認してから元に戻し、`git merge origin/main` 後に検証を再実行した。`selection-command.json` の競合は #451 の tabs 節と結合して解消した（Issue #456）。

別リポジトリ側の作業として、ViewLink Task 1 の記録がある。恒久テストは RED 8 fail から GREEN 87/0 へ遷移し、`check-ui-binding`/`test-ui-binding`/`build-ui-binding` が green、oracle 変更なしで `check-ui-guitest`/`test-ui-guitest` が 135/0 成立した。独立 review-protocol は Reviewer A/B→Validator→Judge で A1 の P1 blocking を検出し、修正後に ACCEPT WITH NON-BLOCKING NOTES で通過した。自 branch push・日本語 PR・候補 SHA の manual Full CI 結果報告まで実施した。

## スクリプト基盤の文書チェーンと PoC 検証

Issue #558 は script contract の DESIGN-ONLY スライスで、spec/design/ADR/test-spec/plan/task/IR/exp の文書チェーンと証跡（contract-trace・consumer-applicability・verification）を作成した。検証は audit_docs/audit_adr/audit_impl_record/doc_status・`mise run fmt-check`・`prek run --files` で行い、未実行項目（実 Runtime 共存・3OS・native FFI・Rhai 本番採用・AL Script 適用）は NotRun/Deferred と明記する方針を採った（Issue #558）。

同一バイナリでの 2 言語共存は PoC で実証された。Issue #559 では headless PoC で mlua 0.12.2（luau+send）と rhai 1.27.0（sync）を同一実行ファイルに同時リンク・初期化し、実行時選択、Versioned Typed Definition の canonical digest 一致（両言語で sha256:v1:254a… が一致）、混種 Workflow 参照（Luau 産 recipe を Rhai 産 workflow が digest 参照）、Fail-Closed を実測した。36 fixture test がすべて Pass、demo binary で同一 digest を実演し、`mise run fmt-check`/`poc-script-coexist-{check,lint,test,run}` も全 Pass（Windows 11 x86_64）だった（Issue #559）。

FFI 境界の fail-closed は計画項目ごとに検証された。Issue #560 の ffi-poc では、検証計画 V1〜V6 がすべて Pass し、cargo test は 22/22（lib unit 8・bridge 13・faults 1）、clippy -D warnings は clean（Windows MSVC）だった（Issue #560）。

- V1: Analysis FFI shim 経由の構造化 diagnostics/completion
- V2: mlua VM と Analysis の同一バイナリ・同一 revision 共存
- V3: version/struct_size/cursor/UTF-8/size/thread の fail-closed
- V4: `RF_FFI_POC_FAIL_{INIT,ANALYZE,LOADDEF}` で status 7、解除後に正常復帰
- V5: Unavailable/Unsupported{completion}/StaleResult/revision 保持
- V6: canonical 検証

alternate evaluation の経路は bit 単位の一致で検証された。Issue #562 では headless 2 候補 PoC を実装し、CB-01〜CB-05 の全 7 test が Pass した（2026-11-14、Windows 11 x86_64、mise toolchain）。interleaved 実行と単独再実行が bit-identical、Preview A/B で Main 記録列が control と bit-identical かつ assert_no_alloc Pass、sidechain は main·(1+side·key) と一致、offline render は total_latency 0/32・tail_evidence=Known・再 render bit-identical、preflight failure 分離・seek/loop 決定性・probe cross-candidate 整列（anchor 必須の fail-closed、Aligned{EngineFrame}）を確認した。`mise run fmt-check`/`check`/`lint`/`test ruviform-audio` はすべて Pass だった（Issue #562）。

ruviform-script crate の新設は slice 単位で進められた。Issue #692 の first slice では crates/ruviform-script を新設し、C1 registry・C2 runtime port・C3 値モデル・C4 budget/cancel・C5 analyzer port・C6 host surface・canonical digest・$rv codec を headless 実装し、`mise run fmt-check`/`check`/`lint`/`test ruviform-script` と `prek run --all-files` が全 Pass（54 test、Windows 11 x64・rustc 1.99.1）だった。AC4（mlua vendored と native bridge の選定）は s2 へ Deferred、実 Luau backend（AC2）と synvelis 接続（AC3）は対象外として文書に明記した（Issue #692）。

s2 の Luau backend では review-protocol の 2 cycle で P1/P2/P3 を検出・修正した。検出内容は次のとおりである（Issue #692）。

- cycle 1 の P1: cursor presence だけで native completion が起動する問題。wanted.completion を条件化して cursor_offset=-1 とする修正と回帰 test を追加。
- cycle 1 の P2（1 件目）: schema-invalid な shim JSON が empty successful results を返す問題。トップレベル object と diagnostics/completions array を必須化し、違反は AnalyzerError::Internal とした。
- cycle 1 の P2（2 件目）: 改行意味論のずれ。Luau Lexer は `\n` のみを計数するため、shim 側と Rust 側の `\r` 計数を両側 `\n` のみに統一した。
- cycle 2 の P2: CheckResult.timeoutHits が shim で drop され timeout が部分診断を Ok で返す問題。status 6 Internal への fail-closed 化と、MIN_POSITIVE limit・200-local module による決定的検証を行った。
- cycle 2 の P3 batch: non-object diagnostics entries の silently skip・garbage range/position の捏造・position_offset col clamp・load_definition freeze-in-catch・kLuauVersion hardcoded drift への対処。

Reviewer A の BLOCKER COUNT は 0 だった。s2 の検証では `mise run check/lint/test ruviform-script-luau`・fmt-check・`prek run --all-files` が全 Pass（48 test：lib unit 16・bridge 22・faults 1・registry 9、Windows 11 x64/MSVC）だった。fault fixture（invalid UTF-8・cursor range・source size・completion disabled・ABI/struct_size mismatch・thread mismatch・fault injection status 7）を製品契約型へ写像して実測し、registry の fail-closed 行列（UnknownLanguage/UnknownAnalyzer/NoCandidates/AmbiguousAnalyzer/UnsupportedContractVersion/MissingCapability/StaleResult）を実 analyzer と stub で検証した（Issue #692）。

runtime 側の拡張でも検証は headless に留めた。Issue #716 では `mise run check/lint/test ruviform-script-luau` がすべて exit 0、91 tests pass（unit 16・bridge 22・faults 1・registry 9・runtime 43）、ruviform-script 側の非回帰 pass、fmt-check clean（Windows x86_64）を確認した。実 OS/実 VM 経路の受入は consumer gate（synvelis#516・synvelis#267 AUTX-08）へ委譲し、当該 slice は deterministic OS-L1 のみとした。Cargo.lock 差分なし（mlua 既依存を Rust API として使用開始）（Issue #716）。

## 実行基盤の棚卸しとジョブ実行の検証

既存 API の棚卸しは、新設しない判断の根拠として実施された。Issue #553 では、ruviform-exec が CancellationToken・Progress・OfflineDriver・OfflineJob（JobState{Running,Cancelling,Finished}・JobOutcome・JobEvent）を提供する一方、Plan/Step/Run journal/Resume/Idempotency/CLI Envelope が存在しないことを確認した。ruviform-cli-sample は利用例であって CLI framework ではなく、「新設 CLI フレームワークは作らない」「sample は例で API に昇格しない」という #155 の方針と一致した（Issue #553）。

Synvelis 側の棚卸しでは、Gateway が Capability/Policy/Grant/Approval/CommitGate/Project/Job の唯一正本、AUTX-05 が WorkflowDefinition/PlanCompiler、AUTX-06 が Coordinator/Run journal/Resume/Cancel、AUTX-03 が synvelis-cli/CliEnvelope の Owner であることを確認した（Issue #553）。

deterministic backend の検証は複数 slice に分けて記録された。Issue #628 の 1st slice では `mise run fmt-check`/`check`/`lint`/`test ruviform-audio-test`/`test-rt` がすべて pass し（37 tests + 3 doctests、Windows 11 x64・rustc 1.99.1）、rt_ prefix テストが assert_no_alloc（`#[global_allocator]` 付き）で clock read・PCM fill・run_callback+sink 経路の allocation-free を実測し、`mise run test-rt` の出力で実際に収集・実行されたことを確認した。PCM oracle は silence/impulse/ramp/square/noise/constant が Exact、sine/sweep は libm と tolerance で独立検証し、MIDI oracle は play-through trace が期待列と一致し、seek/loop で receipt/fault ledger が production 契約どおり記録されることを確認した。run metadata は environment_kind=deterministic-backend・level=OS-L1・seed/fixture_version/clock を保持する（Issue #628）。

後続 slice も同じ lane で検証された。2nd slice は backend 15・backend_faults 9・rt_contract 6 の各 test、3rd slice（pipeline wiring）は pipeline 35 integration tests・rt_contract 9・doctest 3 がすべて pass し、`mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`prek run --all-files` が全 Pass だった。新規 `rt_pipeline_*` 3 test が `mise run test-rt`（workspace 全体の `rt_` filter）で収集・実行されることを出力で確認した（Issue #628）。

scripted backend では、steady-state pump（fault 無し・`record_output: false`）で allocation 0 を assert_no_alloc で確認した。`collect_faults` の `Vec::new()` は push 時のみ alloc するため steady state では不発であり、caller-owned scratch 化は将来の最適化余地として残した（Issue #628）。

観測限界は finding として記録し、製品側は変更しなかった。Session::open は no-op error callback を install するため、production callback 内の DeviceError 値は session surface から観測不能で、PumpReport の errors_fired/invalidated count のみが残る。InputPump は production InputTx::pump の mirror であり、production 側アルゴリズムが将来変わると drift し得る（現行 source との一致は test assertion 済み）。deterministic-backend の Pass は OS-L1 証拠のみで、実 OS API・実デバイス・実 thread timing・duplex・latency は範囲外（EXT-L3/#629 領域）とされた（Issue #628）。

close-out では、117 tests + 3 doctests pass、test-rt で rt_contract.rs 9 件の非回帰、製品 crate の src 差分なし（git diff で確認）、RunMetadata JSON round-trip・wire 語彙 pin・save/load test・StreamFault::Cancellation の Expected/Observed 正負テストの pass・docs index 再生成の in-sync を確認した。`clippy::unnecessary_wraps` 2 件は `cancelled_pump_*` の戻り値を PumpReport 直返しにして解消し、serde/serde_json は workspace 既存 pin 範囲（serde_json 1.0.154）で lockfile version 変動なしだった。実験ログには PCM planar slice の test bug 修正・`usize::from(u32)` 非存在・assert_no_alloc の `#[global_allocator]` 要件といった観察も残されている（Issue #628）。

close-out の受入整理では、Issue 全 AC × verdict の棚卸し表を証跡として作成し、先行 3 slice の状態を done へ同期した。review-protocol は協調/敵対の 2 pass を同一 actor で分離実施して verdict: ACCEPT（CONFIRMED P0/P1 なし、Issue Candidate はいずれも REJECTED/非 blocking）だった。CI lane 新設は不要（既存の `test`/`test-rt` が `rt_` を自動収集する）と判断し根拠を明記した。MIDI 側 fault への cancellation 拡張・MIDI→Host adapter の `attach()` 配線（#630 領域）・実 OS/EXT-L3 証拠（#629 領域）・3OS/consumer 別集計は後続へ Deferred と整理した（Issue #628）。

ruviform-exec の additive 変更は、dependent crate の実測で下流互換を確認した。Issue #744 では `mise run check/lint/test ruviform-exec` がすべて Pass（unit 39 + doctest 9）、`fmt-check` と `prek run --files` も Pass で、`JobState`/`JobError` の additive 変更について dependent 4 crate（ruviform-audio・ruviform-cli-sample・ruviform-ui・ruviform-viz）の `check` を実測してすべて Pass だった。clippy pedantic 対応として、`needless_pass_by_value` は `run_job`/`worker_loop` を参照渡しに変更し、`similar_names` は `submitter`/`submitted` を `submit_thread` へ rename、`redundant_closure` は `|| task()` を `AssertUnwindSafe(task)` に修正した。`dropping_a_handle_requests_cancellation` テストは drop が queued 状態に着地すると body skip（正しい Cancelled semantics）で flake したため、body 開始を `poll_until` で待ってから drop するよう決定的に制御した（Issue #744）。

## 記録の範囲と限界

この文書が記録する検証には、実行されなかった項目と後回しにされた項目が含まれる。NotRun または Deferred として記録された主な項目は次のとおりである。

- Issue #192/#193: remap 後の Required Case は Harness（#628〜#631 等）が未構築のため NotRun のまま。#629（OS 仮想 Endpoint）・#630（ABI Test Plugin）・#631（real-GUI runner）の進捗は記録対象の範囲外。
- Issue #419: macOS/Linux L2/L3 は Deferred、Gate C 性能と Gate E（Consumer Oracle、AL #166/SV #176）は NotRun。後継 ADR の status は proposed のままで、正式決裁は design authority の Follow-up として残った。
- Issue #558: 実 Runtime 共存・3OS・native FFI・Rhai 本番採用・AL Script 適用は NotRun/Deferred と明記。
- Issue #559・#560: macOS/Linux は NotRun（検証は Windows 環境のみ）。
- Issue #628: deterministic-backend の Pass は OS-L1 証拠のみで、実 OS API・実デバイス・実 thread timing・duplex・latency は範囲外（EXT-L3/#629 領域）。MIDI fault への cancellation 拡張・MIDI→Host adapter `attach()` 配線（#630 領域）・実 OS/EXT-L3 証拠・3OS/consumer 別集計は後続へ Deferred。consumer oracle は両 repo merge 済みと記録されるが、oracle 実行結果・判定は記録範囲外。
- Issue #692: AC4（bridge 選定）は s2 へ Deferred、AC2（実 Luau backend）・AC3（synvelis 接続）は対象外。Issue は open 継続（AC checkbox・close 操作なし）で、最終的な充足は consumer gate（synvelis#271〜#274）次第であり記録範囲からは確定しない。
- Issue #716: 実 OS/実 VM 経路の受入は consumer gate（synvelis#516・synvelis#267 AUTX-08）へ委譲し、deterministic OS-L1 のみ。`prek run --all-files` の最終結果は証跡側へ記録するとの記述に留まる。
- Issue #727: cargo 系 lane・`*-msrv`・`*-full`・3 OS は POLICIES.md §5 に基づき NotRun。「JobState queued 表現」は実装側で Queued=3 として確定済みだが、worker naming/priority・queue 具象型の最終確定は design 文書側の Deferred 記述に依存する。
- Issue #744: `*-msrv`/`*-full`/3 OS/実機は POLICIES.md §5 と Issue の「実機/native gate: 不要」で NotRun。consumer 配線（synvelis#518 の RenderExecutor port への注入）は範囲外と明記されており、完了状況は不明。

記録の信頼性に関わる観察も残す。Issue #628 の観測限界として、Session::open の no-op error callback により production callback 内の DeviceError 値は session surface から観測不能であること、InputPump が production InputTx::pump の mirror であり将来 drift し得ることが記録された。Issue #627 の doc-driven-dev-graph router の blocked（repo 全体の既存 broken relation）は記録時点で未解決であり、全体の解消状況は不明である。複数のタスク文書で、front matter の status は done なのにチェックボックスが未チェックのまま残る事例が観測されたが、実施記録側は完了を記録しており、実際の未完了か更新漏れかは記録対象の範囲からは判別できなかった。
