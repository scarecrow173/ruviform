---
id: "EVI-dOTPKg9cxfj1VDg3UJ3vsM"
type: "evidence"
status: "draft"
lifecycle: "draft"
title: "CI レーン・ゲート・ツールチェーンの実行記録"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/1", "https://github.com/scarecrow173/ruviform/issues/2", "https://github.com/scarecrow173/ruviform/issues/19", "https://github.com/scarecrow173/ruviform/issues/21", "https://github.com/scarecrow173/ruviform/issues/52", "https://github.com/scarecrow173/ruviform/issues/53", "https://github.com/scarecrow173/ruviform/issues/56", "https://github.com/scarecrow173/ruviform/issues/125", "https://github.com/scarecrow173/ruviform/issues/127", "https://github.com/scarecrow173/ruviform/issues/219", "https://github.com/scarecrow173/ruviform/issues/223", "https://github.com/scarecrow173/ruviform/issues/243", "https://github.com/scarecrow173/ruviform/issues/265", "https://github.com/scarecrow173/ruviform/issues/294", "https://github.com/scarecrow173/ruviform/issues/306", "https://github.com/scarecrow173/ruviform/issues/313", "https://github.com/scarecrow173/ruviform/issues/314", "https://github.com/scarecrow173/ruviform/issues/326", "https://github.com/scarecrow173/ruviform/issues/327", "https://github.com/scarecrow173/ruviform/issues/328", "https://github.com/scarecrow173/ruviform/issues/329", "https://github.com/scarecrow173/ruviform/issues/330", "https://github.com/scarecrow173/ruviform/issues/331", "https://github.com/scarecrow173/ruviform/issues/332", "https://github.com/scarecrow173/ruviform/issues/333", "https://github.com/scarecrow173/ruviform/issues/346", "https://github.com/scarecrow173/ruviform/issues/366", "https://github.com/scarecrow173/ruviform/issues/367", "https://github.com/scarecrow173/ruviform/issues/368", "https://github.com/scarecrow173/ruviform/issues/386", "https://github.com/scarecrow173/ruviform/issues/411", "https://github.com/scarecrow173/ruviform/issues/582", "https://github.com/scarecrow173/ruviform/issues/583", "https://github.com/scarecrow173/ruviform/issues/594", "https://github.com/scarecrow173/ruviform/issues/598", "https://github.com/scarecrow173/ruviform/issues/629", "https://github.com/scarecrow173/ruviform/issues/646", "https://github.com/scarecrow173/ruviform/issues/650", "https://github.com/scarecrow173/ruviform/issues/720"]
  informs: []
  explains: []
  supersedes: []
---

# CI レーン・ゲート・ツールチェーンの実行記録

この文書は、CI のレーン構成、必須ゲート、ツールチェーン管理、およびそれらの検証・実行結果を記録したものである。内容は後に削除された過去の実装記録から抽出した証拠に基づき、この文書はそれらなしで読めるよう自己完結させている。各事実には由来する Issue 番号を付記する。実測値や観測された動作は事実として記し、記録上の評価や解釈はその旨を明示して区別する。

## ローカル検証レーンと mise タスクの基礎

開発基盤の検証は mise タスク列と prek フックで構成され、Windows x64・mise 2026.10.1・Rust 1.99.0 の環境で全タスクが exit 0 で完了した (Issue #2)。

`cargo test --workspace` と `cargo build --workspace` はいずれも exit 0 で、`--profile ci` ビルドも exit 0 だった。`target/ci` と `target/debug` の成果物は分離して生成され、`git check-ignore` で `target/` が除外対象・`Cargo.lock` が非除外であることを確認した。`Cargo.toml` の `[profile.` 定義はルートに 3 件（dev・ci・ci.package."*"）で重複なく、workspace 継承は edition・rust-version・license・lints の 4 行で確認した (Issue #2)。

mise タスクは新規 11 件を含め全て exit 0 で、`verify-ci` は fmt-check→check-ci→lint-ci→test-ci の順に実行され、check-ci・lint-ci・test-ci が `ci` profile で `Finished` し、テストバイナリが `target/ci/deps/` 配下に生成された。`CARGO_INCREMENTAL=0` は mise.toml に 5 行明示され、`prek run --all-files` は全フック Passed、`.pre-commit-config.yaml` と `mise.lock` に差分はなかった (Issue #2)。

リポジトリ反映の検証では、`cargo metadata` の publish が `null` から `[]` に変化し、`mise run fmt-check`/`check`/`lint`/`test` が doctest 1 件を含め全パス、`audit_docs.js --type all` が 40 ファイルで findings 0 だった。edition・rust-version・license の値、mise.toml、ci.yml、Cargo.lock に変更がなく、新規クレートや LICENSE ファイルの作成もないことを確認した (Issue #1)。

## ツールチェーン解決と mise.lock の整合

`mise install --locked` は当初 pipx 経由ツールの lockfile 欠陥で exit 1 を返したが、`uvx_args` の除去・GitHub shorthand spec への変更・`uv` 更新・`.mise/locks/` の dep-graph コミットにより、repo 全 7 ツールが `mise.lock` から解決することを実機再検証した (Issue #2)。

`apm install --frozen` は apm-cli 0.33.0 で対応済みで、`--frozen` フラグの存在を `apm install --help` で確認した。main リポジトリとクリーン worktree の双方で exit 0 となり、`git diff --exit-code -- apm.lock.yaml` も exit 0 で冪等だった (Issue #2)。

クリーン `git worktree` での V-1〜V-11 検証マトリクスは全件 pass で、`apm install --frozen`・setup 3 コマンド・`verify-ci`（`--profile ci`・`CARGO_INCREMENTAL=0`）・`test` が exit 0、`target/ci` と `target/debug` の分離生成と意図した生成物のみの `git status --ignored` を実測した。V-8（CI ディスク計測）は静的確認 pass とし、実ディスパッチは default branch マージ後のため未実施として区別した (Issue #2)。

## MSRV 検証と toolchain 別実測

MSRV は `rustup run <version>` 経由の呼出形に統一し、`cargo +<version>` は rustup proxy 前提のため使わない方針が記録された (Issue #19)。dev-dep のバージョン方針は caret 指定とし、追跡済み `Cargo.lock` を再現性の錨とする（`=x.y.z` exact pin は不採用）(Issue #19)。

Rust 1.89 での実測では、`cargo +1.89 check --workspace --all-targets` が let-chains を使う bench/test ターゲットを含めて成功し、`cargo +1.89 check -p ruviform-math --lib` でも退行がなかった。1.85 時点の「`--lib` のみ成立」という既知制限は解消した。MSRV 引上げに伴い clippy が 1.87〜1.88 安定化 API を提案するようになり、計 16 箇所に機械的修正を適用した後、`mise run fmt-check`/`check`/`lint`/`test`/`test-rt`/`bench-smoke`・`prek run --all-files` が全パスした (Issue #52)。

Rust 1.89.0 の clippy では 5 系統の失敗を実測した。内訳は `float_cmp` 272 箇所・`similar_names` 35 箇所・`unfulfilled_lint_expectations` 11 箇所・`must_use_candidate` 1 箇所・`range_plus_one` 2 箇所で、pin toolchain（clippy 1.99.0）では全て不発火だった。修復後は `cargo +1.89.0 clippy --workspace --all-targets -- -D warnings -A clippy::float_cmp -A clippy::similar_names` が exit 0 となり、文書化除外のみが残存した (Issue #223)。

MSRV 1.89 の GUI guard lane では、`mise run test-ui-test-msrv`（rust@1.89.0）が当初 `cargo-platform@0.3.3 requires rustc 1.91` で exit 101 を返したが、修正後は Rust 1.89.0 で全 76 テストが pass した。merge 後は Cargo.lock union（削除行 0）で 88 テスト pass となった (Issue #411)。

## bench レーンとプロファイル分離

bench スモークは当初 `cargo bench -p ruviform-math -- --test` で、CI の `bench` ジョブを `mise run bench-smoke` へ変更する CI スモーク分離方針が記録された (Issue #19)。

dev-dep 導入前後のコールド計測では、`cargo clean` 直後の `cargo check --workspace --all-targets` が 0.60s（cargo 報告）/ 0.81s（wall、2026-10-20・Windows x64・rustc 1.99.0）で、dev-dep 導入後は同条件で 30.14s / 30.38s だった (Issue #19)。

Issue #650 で `bench-smoke` の対象を bench target 全量の 5 crate（ruviform-math・ruviform-rt・ruviform-viz・ruviform-numeric・ruviform-io-e2e）へ拡張し、`cargo bench -p <5 crate> -- --test` とした。`--workspace` 化は bench target を持たない crate の lib test target まで広げるため不採用とした。`mise run bench-smoke`・`cargo bench --workspace --no-run`・`mise run fmt-check`・`prek run --all-files` は全て exit 0 で、CI `bench` job は既存の `mise run bench-smoke` 経路のため workflow 変更は不要だった (Issue #650)。

## SIMD dispatch と RT 検証レーン

SIMD dispatch の実装検証では、`SimdToken::detect()` が開発機で `"avx2"` を返し、forced 各 token で全 8 カーネル・全サイズ帯がスカラー参照と契約適合した。`tests/simd_equiv.rs` の本番列と `rt_dsp_kernels_no_alloc`（assert_no_alloc）がグリーンで、公開 API に fearless_simd 型が露出しないことを cargo doc シグネチャで確認した (Issue #53)。

Phase F 総合検証では、規約監査（4 公開 enum の `#[non_exhaustive]`・64 箇所の `**RT contract:**` ラベル・29 件の `rt_*` 単体テスト選択・test-scoped allocator 以外の unsafe ゼロ・外部型非露出）を全て確認し、`mise run test` は単体 115 + doctest 97 = 212 件合格、`test-rt`/`bench`/`prek`/`cargo metadata`（全 6 パッケージ publish=[]）も全パスした (Issue #21)。

## CI ワークフローの trigger と required context

`ci.yml` は `workflow_dispatch` 専用化され、`on:` から `pull_request`/`push` を除去し、concurrency を `ci-<run_id>`・`cancel-in-progress:false` に簡素化した。workflow 横断の trigger 不存在は contract test（`ci/daily/tests/ci-yml-contract.test.mjs`）で固定し、全 `.github/workflows/*.yml` に `pull_request:`/`pull_request_target:`/`push:` trigger が 0 件であることを確認した。前提として `main-checkstatus` disabled（Ruleset 26547674）・`main-protect` active（26529830）・Classic 404・reconcile main 成功 run 42178261480・`ci-issue-tracker-reconcile` workflow active（id 409034138）を実測した (Issue #326)。

PR concurrency では `ci.yml` と `consumer-impact-check.yml` に workflow-level `concurrency` block を追加し、`pull_request` では group を `${{ github.workflow }}-refs/pull/<N>/merge` で `cancel-in-progress: true`、それ以外では `${{ github.workflow }}-<run_id>` で cancel 無効とした。cancelled check run は合格とみなされないため偽装経路はなく、cancel 方向は fail-safe（古い revision の check が欠ける=merge 不可）と記録された。12 required checks・job 名・trigger は不変 (Issue #328)。

## CI 性能計測とレーン最適化

3 repo の CI Baseline を `ci/metrics/collect-ci-metrics.mjs`（Node 標準のみ・`gh api` read-only）で実測し、Issue 観察値（313/313/218s）を正確に再現した。代表 run は ruviform 41780346388（wall 318s・critical path 313s=ui-verify windows）・audio-lens 41770611162（wall 326s・cp 319s）・synvelis 41782019309（wall 228s・cp 221s）で、統計は ruviform n=14 wall p50=302s p95=458s、synvelis n=10 p50=242s p95=301s、audio-lens n=2 は統計値を出さず観察値のみ報告した。required contexts は ruviform のみ branch protection strict=true で 12 contexts、audio-lens/synvelis は保護なし（404）だった (Issue #327)。

計測上の主要観察として、3 repo とも critical path は `verify`/`ui-verify` の windows-latest job で、wall time はほぼ最長 job に一致し単一 lane の短縮が wall に直結する構造だった。mise tool cache は 3 repo 全 job で一貫して miss（job あたり 13–23s の固定コスト）、rust lane が step 時間の支配項、audio-lens の cargo cache は lockfile hash 直結で頻繁に cold 化した。分類不能は `other` で名列挙する fail-closed とし、`--logs-run` は opt-in 化した (Issue #327)。

macOS runner の逼迫は実害として観測された。Before 計測（2026-11-05・15 run）で wall p50 307s・p95 497s・runner_total 1474s、push run 41851785028 は queued 2279s 超を観測した。PR 自身の CI run 41863215849 では macOS legs が queue 約 17–25 分で 3 連続 cancel（step 未実行で runner 割当前に cancel）され、他 10 job は全緑で、macOS 逼迫が required lane に局所化していることを実証した。After メトリクスは未実測 (Issue #346)。

## コンパイルキャッシュと依存セットアップの最適化

sccache は `mozilla-actions/sccache-action` を immutable commit SHA `fc920bf0ec8de6ee65d409111f7ec508035751ba`（v0.0.11）で pin し、sccache v0.19.0 を GHA cache backend で導入した。`RUSTC_WRAPPER=sccache` と `SCCACHE_GHA_VERSION=ruviform-v1-<RUNNER_OS>`（OS 名前空間）を `actions/github-script` 経由で `GITHUB_ENV` へ export し、kill switch は repo variable `CI_SCCACHE_DISABLE=1` とした。warm run で verify lane の hit rate は ubuntu 47.32%・windows 62.50%・macos 51.59%、job 時間は ubuntu -51s・windows -78s・macos -32s 削減した。cold run で write error 36–100%（GHAC rate limit / 並列書込競合由来）が発生したが sccache は fail-open で compile/test 完走・全緑を維持し、kill switch で 4 sccache step 全て skipped・job success を実証した (Issue #329)。

npm ci の集約では、`ui-verify` ジョブ内の `npm ci` を package-lock ごとに原則 1 回へ集約し、freshness 付き `install-ui-*` タスク（7 本）と集約 `setup-ui` へ分離した。freshness 判定は mise 標準の sources/outputs freshness check を採用し、`node_modules/.package-lock.json` を output マーカーに選定した。check/test 系タスクには freshness を付与しない（検証 skip は検証能力の削減に相当）。Windows 11・mise 2026.10.1 での実測は npm ci 実行回数 20→7/ジョブ、npm ci 時間合計 約 86s 逐次→約 27s 逐次換算・setup-ui 並列 wall 約 20s、ui/showcase dist emit 3→1/ジョブ、warm 再実行の install は全 skip（~0.5s）だった。検証は test-ui-headless 67/67・test-ui-browser 30/30・test-ui-showcase 73 件で全 pass。CI 3OS matrix の実測は PR CI run での確認事項として未検証項に明記した (Issue #330)。

check-ci の除去では、`verify` job から `check-ci` step を除去して `lint-ci` に型検査を統合した。`cargo check` と `cargo clippy` は同一の 292 check-mode ユニット（152 依存 + 140 workspace）を評価し、workspace メンバー 140 ユニットは compiler identity 差で二重コンパイルされていた。注入実験 8 ケース（lib 型エラー・rustc 警告・clippy lint・cfg(test) 型エラー・example/bench 警告・失敗テスト）で等価性を実証し、`cfg(clippy)`/`cfg(not(clippy))` の使用は `crates/` 配下にゼロだった。`check-ci` は CI lane から外れた単独タスクとして `mise.toml` に残存した。変更後 CI run 41825566750 で check+lint 合計は 3OS とも -28〜-37s 改善（ubuntu 57→27s・windows 110→82s・macos 93→56s）したが、step 時間は run 間変動が大きく単発差分を恒常的改善と断定するには継続計測が必要と明記した (Issue #331)。

Playwright Chromium の cache では、`install-ui-browser-chromium` mise task を新設し `test-ui-browser` は depends 経由で解決した。CI `ui-verify` では `PLAYWRIGHT_BROWSERS_PATH` を workspace 内 `.cache/ms-playwright` に pin し、`playwright-<OS>-<arch>-pw<version>-chromium<revision>` key（restore-keys なし）で復元→専用 install step→launch 検証→実 test step の構造とした。cache key に browser 種別を含めず `chromium<revision>` に固定し、`restore-keys` は付けない（部分一致で旧 revision binary を復元すると test が旧 browser で走り得る）。cold run 41815446750（全 OS miss→download→save）と warm run 41816549243（全 OS `Cache hit`→install 1s no-op→download ゼロ）を確認し、warm で browser 関連 work は windows 40→19s・macos 44→31s・ubuntu 28→23s に減少した。cold でも test step 単体は Before より短く（ubuntu 28→10s 等）、download が install step へ分離され実 test 時間が可視化されたのが主効果だった。ubuntu の install-deps 16-18s は cache 対象外で別レイヤーの支配項 (Issue #332)。

## Full CI への移行と集約ゲート

impact-based CI は Shadow Mode で導入され、`ci/impact/`（stdlib-only Node `.mjs`）に classifier + map + gate を実装した。分類は `cargo metadata` の workspace 逆依存伝播・`rt_` 保持 crate scan・mise task `-p` 引数・`ui/*` DAG・full-fallback 規則で機械導出し、曖昧/分類不能/共通設定変更は Full CI に倒す。lane job の `if:` は一切変更せず既存 12 required contexts を維持したまま、`impact`（分類+decision artifact）と `ci-gate`（`if: always()` で needs 全結果を判定）が実運転で観測された。diff は `git fetch --depth=1 origin $BASE` 後の two-dot tree diff で、merge-base 不要で shallow clone と両立した。PR #345 run 41835836190 で impact-classify 18s pass・decision=`full`（ci-config+shared-config fallback 発火）・ci-gate 10s pass・6 lane 全 Pass・12 required contexts 全 pass を確認し、impact+ci-gate の追加分は ~28s で critical path 外だった。部分集合 expected の実測は未観測 (Issue #333)。

Full CI Stage1 では `ci.yml` を全 lane 常時実行に変換し、`ci-summary` fail-closed 集約・`io-structured-verify` lane・`expected_sha` 契約を導入した。`ci/summary/summary.mjs` は needs JSON 評価・verdict・Job Summary・report JSON 出力で、verdict 行列は all-success→pass / failure/cancelled/skipped/missing/unknown→fail、expected_sha は空=照合なし・一致・不一致、NEEDS_JSON 不整合は parse 失敗→fail とした。`*-macos` 3 job を無条件化し、`macos-report`/`ci-gate` を `ci-summary`（needs 11 件・`if: always()`）へ置換した。`verify`/`l1-real-verify`/`ui-verify` の job id・matrix os 配列・`name:` は編集せず required context 不変を維持し、`ci/impact/` と `consumer-impact-check.yml` は削除した (Issue #366)。

macOS GPUI lane は matrix ではなく物理分離 job `ui-gpui-verify-macos` とし、matrix leg は `needs.<job>.result` が 1 値に集約され失敗 OS を判別できないためとした。`build-full` は接続せず、`test-full`（cargo test --workspace）が実行 bin の Build/Link を実質包含し、唯一の空白 `windowed_test_app` は `test-ui-gpui` の `--features "gpui-test uitest"` 拡張で塞いだ。初回 Full CI run 43421898733 で `ui-gpui-verify-macos` が `lint-ui-gpui` の `clippy::doc_markdown`（capture.rs:883 `NSView`）で失敗した。macOS 専用 cfg module 内のため Linux/Windows lane では一度も lint されていなかった潜在欠陥を新 lane が fail-closed で検出し、crate 全体の cfg 監査で macOS 固有の compile surface はこの 1 module のみと確認して comment-only 修正（NSView をバックティックで囲む）を適用した。2 回目 run 43424042262 で全 20 job success・ci-summary verdict PASS。walltime は 8m29s→13m44s（+5m15s）、runner 時間合計は ~46min→~62min（+~16min）で、増分は新規 macOS lane 8m35s と feature 組合せ変更による GPUI 系 3 lane の compile 増加だった (Issue #598)。

## daily scheduler と CI failure tracker

daily scheduler は `daily-ci.yml` + `ci/daily/decide.mjs` で「現在 main SHA に有効な Full CI がなければ `ci.yml` を `workflow_dispatch` で起動する」判定を実装した。判定行列は skip-verified / suppress-in-flight / hold-unknown / budget-exhausted / dispatch で、attemptsOn 厳格性・auto dispatch 計上（bot actor のみ）・latestVerified・dispatch 確認（confirmed/landed-mismatch/unconfirmed）・main-moved 再照合を `node --test` で検証した。schedule は UTC `"17 21 * * *"` = 06:17 JST で、`workflow_dispatch`（`dry_run` 入力）・`permissions: {}` → job `contents: read`/`actions: write`・concurrency（直列・非 cancel）・`test-ci-daily` 先行・report artifact の構成とした (Issue #367)。

CI failure Issue tracker は `ci/tracker/tracker.mjs` を stdlib-only Node `.mjs` として実装し、`workflow_run` 完了ペイロードを scope（repo × workflow `ci` × branch `main` × event ∈ {push, workflow_dispatch}）で分類して action plan を組み立てた。63 件の mock-client contract test で、`(run.id, run_attempt)` 順序・`updated_at` 中心 ordering・run-history reconciliation・marker suppress の `(runId, attempt)` 単位修正・履歴 record の 3 分類（out/in/unknown）を実装した。`mise run test-ci-tracker` 63/63 pass。`ci-issue-tracker.yml` は `workflow_run {workflows:[ci], types:[completed], branches:[main]}` のみで起動し、`track` job に最小権限（contents:read / actions:read / issues:write）を限定付与、self-test を API 呼出し前に置き、checkout は default branch の自分自身とした。`GITHUB_TOKEN` dispatch の完了が `workflow_run` を発火させない gap は、event 非依存の `--reconcile` モードと別 workflow の `schedule`/`workflow_dispatch` 入口で埋めた。実 cron・実 Issue・実 dispatch は行わず、全て mock client + actionlint + `git diff` の offline 検証とした (Issue #368)。

## consumer-verify ワークフロー

consumer-verify は `workflow_dispatch` 専用の情報 lane（preflight + verify-audiolens + verify-synvelis + summary）で、credential 未設定時は `not run` 出力＋全 lane not run で workflow 失敗として正直に示す（証拠ゼロを緑にしない）契約とした。`CROSS_REPO_DEPLOY_KEY` 未設定 + 匿名 HTTPS read 可能な場合に `anonymous` mode で lane を実行し、`deploy_key` mode（SSH・known_hosts pin・`url.insteadOf`・cleanup）は private 前提の canonical 経路として維持した。manifest rev 書換は `rev = "$RUVIFORM_REV"` への正規化 sed に両 lane で統一し、書換 no-op は単一 SHA guard の 0 行検出で必ず fail とした (Issue #219)。

synvelis lane の実測（`f7aadd0` scratch clone）では、manifest 正規化→`CARGO_NET_GIT_FETCH_WITH_CLI=true` で `cargo fetch`→単一 SHA guard（total=15・bad=0）→`mise run check` exit 0→`git lfs pull`+`mise run test` exit 0（23 suite・0 failed）だった。audio-lens の scratch clone（987f236）では同一 sed 後に 14 dep の rev が候補に置換され、lock 15 source 全てが候補 SHA に解決し、`cargo check --workspace --all-targets` が exit 0・25.78s だった (Issue #219)。

実 dispatch run 41604118237（anonymous mode・preflight deploy_key=absent）では preflight 4s・verify-synvelis 全緑 1m42s・verify-audiolens が fetch step の全量 `mise install --locked` で失敗（consumer mise.lock gap）→ summary は正しく fail だった。run 41604498852（scoped install 後）では preflight 5s・verify-synvelis 全緑 1m14s・verify-audiolens は check-ci・smoke 2 lane 通過のうえ test-ci で `al-cli --test cli` export 3 test fail を検出→summary fail となった。検出されたのは consumer 側潜在 portability 欠陥（export file の metadata.json が absolute path を embed）で、ruviform 起因ではないと記録された (Issue #219)。

consumer-verify の証拠契約では、候補 tuple の証拠（解決 SHA 捕捉・期待 SHA 照合・構造化 JSON artifact）を実装し、判定ロジックは `ci/consumer-verify/evidence.mjs` に単一権威として集約（classify-ref/lane/finalize の 3 コマンド）した。`shaMatch` は `EXPECTED_SHA`/`RESOLVED_SHA` から finalize 側で導出し、`GATE=run` 時のみ `RUVIFORM_REV` の 40-hex を要求、env は allowlist のみ読み秘密系を転写しない。両 verify job に候補 tree 評価 step（`gui_candidate`/`gui_boundary`/`gui_semantic`）を追加し、`gui_candidate` は `$RUNNER_TEMP/ruviform-candidate` へ `git init`+`fetch --depth 1 origin $RUVIFORM_REV`+`checkout --detach FETCH_HEAD` で候補 tree を取得して `rev-parse HEAD` を候補 SHA と照合し、候補 `mise.toml` の `[tasks.<name>]` 存在を grep probe した (Issue #294)。

実 runner dispatch（基準 head `30205f2`・run 42302538464）では、preflight success、両 Consumer clone・expected-SHA 照合 success（AL resolved=expected=`04f1861`、SV resolved=expected=`d8bf8ca`、`shaMatch: true`）だった。verify-synvelis 全 lane pass、verify-audiolens は dep/pin/fetch/`check-ruviform-lock` まで pass し `check-ci` で fail した。原因は `al-observability/src/lib.rs:264` の `SinkConfig.span_events` 初期化子欠落（E0063）で、該当 field は RF #335（PR #348・`33d93dd`）で RF main `49de02cc` に既導入済みであり、AL main `04f1861` 側の未追随が実不整合として検出された。fail 以降の AL lane は `skipped` 派生記録、summary の not-run-is-not-pass 強制も発火した (Issue #294)。

GUI-CONTRACT lane の初回実 candidate run（run42348131128）では synvelis job が exit 2 で失敗した。`check-ui-driver-boundary` の `cargo run` が必要な build 依存を online で取得するが、guard binary 内部の `cargo metadata --offline` は候補 workspace の Cargo.lock 内全 package（dev-dep・非 host target dep 含む）の manifest を要求し、Consumer clone の `cargo fetch` cache では ruviform 全 workspace を覆えず `alloca v0.4.0` 未取得で失敗した。修復として `gui_candidate` の task probe 成功分岐で `mise install --locked rust` に続けて `mise exec -- cargo fetch --locked` を候補 dir で実行し、修復後 test-ci-consumer-verify 64 tests pass・check-ui-driver-boundary verdict pass・test-ui-driver-contract 5 tests pass となった (Issue #294)。

consumer-verify の証拠 schema は Case schema v2 へ additive に拡張し、`SCHEMA_VERSION` を 1→2 に bump、`CONTRACT = "consumer-verify-tuple/v1"`・`outcome` 語彙・`deriveStatus`/`coverage`/`markdownSummary`・top-level tuple/run/consumer 構造は不変とした。`readLanes` は `lane` コマンド経由でない行にも同一語彙の fail-closed 検証を課し、schema-v1 行・未知 enum・型不正・非 40-hex `resolved_ruviform_sha` を拒否した。`ci/daily/decide.mjs` は `kind`/`contract`/run/consumer/tuple/status のみ検証し `schemaVersion`・`lanes` を読まないため `contract` v1 固定で回復経路は不変だった。Issue #720 で contract を v1→v2 へ一括 bump し、`evidence.mjs finalize` smoke で発行証拠 JSON の `contract`/`tuple.contract` が `consumer-verify-tuple/v2` であることを実測した (Issue #294, Issue #720)。

consumer oracle lanes では、consumer checkout 内の canonical test file を `oracle_probe` step が file 単位で存在確認し、存在すれば `git lfs pull` の後に canonical command（`mise run test <pkg> --test <file>`）を file 粒度の別 step で実行した。Record step は probe output の 3 分岐（`'1'`→run outcome / `'0'`→owner 付き `not_run` / else→probe outcome）で lane 行を記録し、`fixture_version` は consumer `Cargo.lock` の `ruviform-audio-test` version から構成し抽出失敗時は flag を省略（値を捏造しない）した。SV の 2 oracle file（`rf628_oracle.rs`・`rf628_oracle_rt_alloc.rs`）は別 step・別 lane 行で実行し、`--expected`/`--observed` は記録しない（consumer test は機械可読の期待値/観測値を出力しない—捏造禁止で null のまま）とした (Issue #294)。

consumer-verify → ci-issue-tracker 配線では、`ci-issue-tracker.yml` の `workflow_run` trigger を `workflows: [ci, consumer-verify]` に拡張し、`workflow_run.name` でゲートされた scope 別 job（`track`/`track-consumer-verify`）がそれぞれ `--workflow`/`--workflow-file` で scope を明示して同一 evaluator を実行した。`ci-issue-tracker-reconcile.yml` は GITHUB_TOKEN dispatch の anti-recursion 補完として `ci`→`consumer-verify` の 2 scope を逐次実行し、daily-ci.yml は GITHUB_TOKEN で consumer-verify を dispatch するため anti-recursion で `workflow_run` completed が発火しない—event 経路だけでは daily dispatch 起因の失敗を取りこぼすため、reconcile が権威ある run history を走査する既存境界をそのまま 2 scope 化した。tracker は evidence JSON の verdict を読まず workflow run の conclusion のみを評価し、`not_run`/`unsupported` は consumer-verify.yml が failure conclusion を出さない限り tracker の失敗集合に入らず success でもない。Issue title は scope 由来で `[CI failure][<branch>] <workflow> failed` を `issueTitle(scope)` で生成した (Issue #294)。

daily tuple trigger では `ci/daily/decide.mjs` に `--mode tuple` を追加し、daily scheduler の検証単位を単一 SHA から候補 tuple `(ruviform main SHA, audio-lens main SHA, synvelis main SHA, consumer-verify-tuple/v1)` へ一般化した。Actions run API は dispatch inputs を公開しないため、各 run が検証した tuple は `consumer-evidence-<consumer>-attempt-<run_attempt>` artifact の証拠 JSON から復元する fail-closed 経路を実装し、証拠なし・期限切れ・破損・内部不整合は全て `unverifiable`（verified にも無視にもせず、auto actor なら retry budget に保守的に計上）とした。zip は最小自前 reader（EOCD/central directory/local header を stdlib で走査）で、data descriptor・zip64・method 0/8 以外は `ApiError`→unverifiable。artifact zip DL は 302 `Location` を手動追従し token を署名 URL へ転送しない (Issue #294)。

## CI 失敗の修復記録

run 43270011255 の 3 系統失敗は原因確定のうえ修正された (Issue #582)。

`ui-gpui-verify` の冷キャッシュ `cargo metadata --offline` 失敗は、`ci.yml` の `ui-gpui-verify` に `mise exec -- cargo fetch --locked` step を追加して解消した。`Cargo.lock` の `libc` は 0.2.191 への drift（`4bcf1402`（PR #572）の lockfile 再生成で発生）を `cargo update -p libc --precise 0.2.189` で pin 復元した。moxcms 0.9.1→0.9.2 の ICC.1:2022 §8.10.2 c) perceptual fallback に対しては、pin ではなく 0.9.2 適応を選択し、`IccTransform::new` が Issue #125 AC3 の missing-intent-tag reject を境界で再強制した。pin は lockfile 再生成で失われやすく（libc pin 喪失が同一 failure mode）、`IccIntent` の missing-tag reject は `IccTransform::new` が所有する境界の不変条件として再強制するのが責務的に正しいと判断した。encoder workaround（`write_lut_entry`）は 0.9.1/0.9.2 で同一のため `fix_mft1_tags` は 0.9.2 でも必要・有効だった (Issue #582)。

さらに dispatch run 43318635475 で顕在化した既存破損として、`BACKEND_AVAILABLE = cfg!(windows)` により Linux では `declare_slot` が常に `Unsupported` を返すのに `rvf_view_native_slot_*` 5 test が成功を仮定していた問題を `#[cfg(target_os = "windows")]` で gate した。`gh workflow run ci.yml --ref bugfix/582-ci-lanes-failures` で対象 4 job（ui-gpui-verify ×2 / ui-gpui-offscreen-verify / icc-verify）が success になったことを確認した (Issue #582)。

consumer-verify preflight の `mr_boxington` 失敗は、旧設定（sha `4f31460e`）の `mise.toml`/`mise.lock` を clean global config で mise 2026.11.7 windows-x64 にかけ、`mise install --locked` が同一エラーで exit 1 することを再現確認して原因を確定した。現行 main では `mise install --locked node`・`MISE_ENV=ci mise install --locked`・`MISE_ENV=ci mise install --locked rust git-lfs node`・fresh `MISE_DATA_DIR` での node install が全て exit 0 で、設定変更は行わなかった。`gh workflow run consumer-verify.yml --ref bugfix/583-consumer-verify-mise` で run 43305677667 が全 job success（preflight 24s / verify-audiolens 3m13s / verify-synvelis 5m44s / summary 3s）を確認した (Issue #583)。

MSRV daily lane では `ci.yml` に `msrv-verify` job を追加し 4 つの `test-*-msrv` task を rust@1.89.0 で日次 CI 担保した。`mise.toml` は無変更（task が `mise exec rust@1.89.0` 内蔵・target dir 分離済み）で、`ci-summary` は needs だけでなく `summary.mjs` の REQUIRED_JOBS/REQUIRED_CHECKOUT_LEGS 宣言も fail-closed で検査するため 3 箇所同期が必須だった。初回 dispatch run 43383950490 で `test-observability-msrv` が `rolling_file_retention_*` 2 件失敗した。upstream tracing-appender 0.2.5 の `prune_old_logs` が birth time でソートするため seed file の同一 timestamp tick で削除順が壊れたことが原因で、feature-gated test target は `--all-features` の MSRV lane でのみ実行され通常 lane 未検出だった。修正は SEED_FILE_BIRTH_GAP 100ms sleep で、2 回目 run 43385858552 success・ci-summary PASS となった。4 task を個別 step + `if: ${{ !cancelled() }}` にしたのは、先行 task 失敗時に後続 task が skip されると 1 件の MSRV 破損が他 crate set の独立破損を不可視にする coverage gap を避けるためで、`always()` でなく `!cancelled()` を選び cancelled run での誤判定を防いだ (Issue #594)。

## Linux offscreen lane の実機検証

GT-05 の Linux Xvfb offscreen lane では、bare Xvfb では present 不成立（黒画面）で openbox が必須要件、mesa ≥24.2 必須（sw X11 WSI の `chain->depth` 修正、22.3.6 では depth-24 PutImage BadMatch で恒久黒画面）、`--test-threads=1` 必須（`_NET_WM_PID` が process 単位一意・occlusion で GetImage が下位 window を読めない）と記録された。window 解決は `_NET_WM_PID` + `_NET_CLIENT_LIST` 優先 + `QueryTree` fallback で、openbox は reparent するため root-child 列挙では拾えない。gpui 0.2.2 の `X11Window::window_handle()` が `unimplemented!()` で rwh seam が panic するため、capture 側は lazy に own toplevel を `_NET_WM_PID` sentinel（`usize::MAX`）で解決した。`client_geometry` の `GetGeometry` は inside size（border は drawable 外）なので `border_width` を引かない。`libc` を 0.2.189 に pin したのは、xattr 0.2.3 が参照する `libc::ENOATTR` が libc 0.2.190 で Linux cfg から削除され gpui の Linux build が不能になった upstream 既知問題の lockfile 修正だった (Issue #314)。

Docker Debian + mesa 25.0.7 + Xvfb + openbox + lavapipe で `cargo test -p ruviform-ui-gpui --features real-scenario --test real_window -- --test-threads=1` を実行すると 2 tests が pass（~96 s）した。lane と同一形状（xvfb-run + sh -c + openbox + `_NET_SUPPORTING_WM_CHECK` probe + `--test-threads=1`）で再現し、bare Xvfb 対照実験では同一 build が黒画面（openbox/mesa の寄与を分離）だった。xtrace で lavapipe present が window drawable への PutImage（depth-24 vs window depth-32）で BadMatch を観測し、mesa 24.2.8 source で `chain->depth` 使用を確認した。Windows の test-ui-gpui/check-ui-gpui/lint-ui-gpui/test-ui-consumer-observation/test-ui-scenario-contract-gpui 等は全 pass、Linux `cargo check`/`clippy -D warnings`（gpui feature）は Docker で pass した。ubuntu-24.04 stock mesa（24.2.x）による初回実測は PR CI で確認予定（source 上 `chain->depth` 修正済みだが実機未検証）と記録された (Issue #314)。

## CI 照合と証拠の SHA 固有性

CI 照合は SHA 固有で、祖先 SHA の成功を転用しない原則が記録された (Issue #127)。

gh での照合では、run 43439434737（@ `365c74ac`）の全 lane 成功・テスト集計・checkout-sha artifact を SHA 単位で確認した。`365c74ac..621490e8` に `crates/`・`mise.toml`・`ci.yml` 差分があるため HEAD は `NotRun(Pending)` とし、CI 証拠は SHA 固有で祖先 SHA の成功を HEAD の Pass に転用しない。照合手法は `gh run list/view`・`gh api .../artifacts`・`gh run view --job --log` の 4 段照合だった。既存 Ubuntu CI（ci.yml/daily-ci.yml/consumer-verify.yml）の lane 定義と main 上の成功 run を gh で照合し、対象 SHA・run URL・job・結果・テスト集計を台帳化した。macOS/Linux L1（必須）では Native Build/Link・必要 Feature・Headless/RT の既存 CI を同候補 SHA（365c74ac）で確認し、HEAD（621490e8）は NotRun(Pending) とした。audit_docs.js・audit_impl_record.js・audit_experiment_log.js・doc_status.js で blocking finding なし、`mise run fmt-check` exit 0 だった (Issue #127)。

判定語彙は `Pass`/`Fail`/`Unsupported`/`NotRun`/`Blocked` で、正直性ルール（`NotRun` を `Pass` にしない）が設計された (Issue #127)。

## 記録の範囲と限界

以下の検証は記録上 NotRun または deferred として扱われ、実施済みの Pass には含めない。

consumer-verify の実 runner dispatch は複数 slice で NotRun と記録された。Issue #294 の各 slice（tuple evidence・Case schema v2・oracle lanes・daily tuple trigger・GUI-CONTRACT lane）では、workflow_dispatch lane の実走は post-merge の Coordinator 管轄で未実施とし、fixture ベースの実装で実 GitHub API に対する tuple 判定・dispatch は merge 後の daily/手動 `workflow_dispatch`（`dry_run` 推奨）で初めて実測されるとした。Issue #720 の Issue AC5（post-merge 実 run gate）も NotRun で Coordinator 所有の post-merge dispatch で追跡する。Issue #219 の consumer-verify 初回 dispatch 実走も credential 設定後の別運用で未実施だった (Issue #219, Issue #294, Issue #720)。

実 daily dispatch の実走証跡・reconcile の consumer-verify scope の実データ処理・both-consumer-pass の実測・`CROSS_REPO_DEPLOY_KEY` 等 credential 設定・GUI-VISUAL / OS lanes・`PLUGIN-NATIVE-DOCK` は NotRun と記録された。実 daily dispatch は run API 制限・ops 事項、reconcile の consumer-verify scope は次回 scheduled/manual 実行で初めて実データを処理、both-consumer-pass は audio-lens 側 export portability 修正待ち、credential 設定は人手の credential 作業、GUI-VISUAL / OS lanes・`PLUGIN-NATIVE-DOCK` は実機 gate（#313/#314 系）とされた (Issue #294)。

bench-smoke の `*-msrv`・`*-full`・3 OS 横断は本 slice の明示対象外で Daily Full CI 委譲・NotRun とした。Issue #346 の After メトリクス（選択実行有効化後の wall/runner_total/lane skip 率）は未実測で merge 後の selective run で収集予定。Issue #328 の AC3（同一 PR 連続 push で旧 run cancel の実測）と別 PR/main push/dispatch の非 cancel 実測は coordinator 引き継ぎで worker 環境では push・PR 操作を行わない規約のため未実施。Issue #330 の CI 3 OS matrix の実測は PR CI run での確認事項でローカル検証は Windows のみ。Issue #333 の部分集合 expected の実 run 証拠（docs-only/単一 crate/Ui-only 変更 PR）と workflow_dispatch・base retarget・kill switch の実動作観測は Shadow 期の確認対象として未観測。Issue #368 の実 workflow_run 発火・実 cron・実 Issue・実 dispatch は全て未検証（offline contract test + actionlint のみ）で merge 後の初回実測が受入。Issue #127 の現行 main HEAD（621490e8）の CI 結果は未取得（daily-ci dispatch 待ち・NotRun(Pending)）で merge 後の対象 SHA の必須 CI 確認は Coordinator 責務とした (Issue #127, Issue #294, Issue #328, Issue #330, Issue #333, Issue #346, Issue #368, Issue #650)。

その他の NotRun・deferred 項目として、Issue #329 の Cargo.lock 更新時・toolchain 更新時・fork PR の GHAC 挙動・RT/ICC/L1/bench 展開・Consumer（AL #148/SV #147）適用、Issue #332 の Playwright version bump 時の cold 再計測（実際の bump PR での観測は未発生）、Issue #411 の macOS/Linux 実機（Windows 11 x64 のみ実測）、Issue #314 の ubuntu-24.04 stock mesa（24.2.x）での実測（CI 初回で確認予定）、Issue #594 の retention fixture の 100ms sleep が filesystem timestamp 粒度への依存を残すこと（粒度 >100ms の FS では再発し得る・upstream 恒久修正は別 Issue 候補）、Issue #598 の `docs/development-environment.md` lane 構成記述の更新と `POLICIES.md` L3-4 の行末空白修復（編集範囲外・未対応）が記録された (Issue #329, Issue #332, Issue #411, Issue #314, Issue #594, Issue #598)。
