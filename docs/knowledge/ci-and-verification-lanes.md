---
id: "KNOW-GRmD7p9xDYORobz06ZaKNN"
type: "knowledge"
status: "draft"
lifecycle: "draft"
knowledge_kind: "operation"
title: "CI レーン・ゲート・検証基盤の運用"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/2", "https://github.com/scarecrow173/ruviform/issues/19", "https://github.com/scarecrow173/ruviform/issues/125", "https://github.com/scarecrow173/ruviform/issues/127", "https://github.com/scarecrow173/ruviform/issues/219", "https://github.com/scarecrow173/ruviform/issues/243", "https://github.com/scarecrow173/ruviform/issues/294", "https://github.com/scarecrow173/ruviform/issues/312", "https://github.com/scarecrow173/ruviform/issues/313", "https://github.com/scarecrow173/ruviform/issues/314", "https://github.com/scarecrow173/ruviform/issues/326", "https://github.com/scarecrow173/ruviform/issues/327", "https://github.com/scarecrow173/ruviform/issues/328", "https://github.com/scarecrow173/ruviform/issues/329", "https://github.com/scarecrow173/ruviform/issues/330", "https://github.com/scarecrow173/ruviform/issues/331", "https://github.com/scarecrow173/ruviform/issues/332", "https://github.com/scarecrow173/ruviform/issues/333", "https://github.com/scarecrow173/ruviform/issues/346", "https://github.com/scarecrow173/ruviform/issues/366", "https://github.com/scarecrow173/ruviform/issues/367", "https://github.com/scarecrow173/ruviform/issues/368", "https://github.com/scarecrow173/ruviform/issues/386", "https://github.com/scarecrow173/ruviform/issues/582", "https://github.com/scarecrow173/ruviform/issues/583", "https://github.com/scarecrow173/ruviform/issues/594", "https://github.com/scarecrow173/ruviform/issues/598", "https://github.com/scarecrow173/ruviform/issues/629", "https://github.com/scarecrow173/ruviform/issues/646", "https://github.com/scarecrow173/ruviform/issues/650", "https://github.com/scarecrow173/ruviform/issues/720"]
  informs: []
  explains: []
  supersedes: []
---

# CI レーン・ゲート・検証基盤の運用

この文書は、CI レーン・ゲート・検証基盤の設計と運用に関する再利用可能な知見を、過去の実装記録から抽出してまとめたものである。記録元の文書は後に削除されたため、本書はそれらに依存せず単独で読めるように記述する。事実は各記述の末尾に Issue 番号で出所を示す。実測値や観測された挙動は事実として記し、解釈が含まれる箇所はその旨を明示する。

## ワークフローの起動方式とトリガ構成

CI ワークフローは `workflow_dispatch` 専用で運用する設計を採った。初期の ci.yml は `workflow_dispatch` 専用・`permissions: contents: read`・ubuntu-latest 単一 job・`CARGO_INCREMENTAL: "0"` で作成した（Issue #2）。この方式では default branch にファイルが存在しないと Actions UI からディスパッチできないため、ブランチ上の検証は静的検証（prek check-yaml・actionlint）に留め、実動作はマージ後の手動ディスパッチで確認する運用とした（Issue #2）。この制約は後の実測で訂正されており、`workflow_dispatch` は `--ref` でブランチ指定が可能である（Issue #294）。

Issue #326 で ci.yml の `on:` から `pull_request`/`push` を除去し `workflow_dispatch`（`expected_sha` 宣言は不変）へ確定し、concurrency を `ci-<run_id>`・`cancel-in-progress: false` に簡素化した。workflow 横断で `pull_request:`/`pull_request_target:`/`push:` trigger が 0 件であることを contract test で固定した（Issue #326）。この転換の前提として、`main-checkstatus` ruleset が disabled、`main-protect` が active、Classic branch protection が 404 であることを実測で確認している（Issue #326）。

PR の連続 push に対する stale run の cancel は、workflow-level `concurrency` block で実現した。`pull_request` では group を `${{ github.workflow }}-refs/pull/<N>/merge` として `cancel-in-progress: true`、それ以外の event では `${{ github.workflow }}-<run_id>` で cancel 無効とする（Issue #328）。cancelled check run は success/skipped を満たさないため合格とみなされる経路はなく、cancel 方向は fail-safe である（Issue #328）。既知の edge として、古い run の手動 re-run は同 group に再入して最新 run を cancel し得るが、方向は fail-safe であり復旧は最新 run の再実行または再 push で行う（Issue #328）。

## 判定語彙と正直性ルール

判定語彙は `Pass`/`Fail`/`Unsupported`/`NotRun`/`Blocked` を採用し、`NotRun` を `Pass` にしない正直性ルールを設計の中核に置いた（Issue #127）。証拠ゼロを緑にしない、という原則が各所の設計判断の根拠になっている。consumer-verify では credential 未設定時に `not run` 出力で全 lane を not run とし workflow を失敗させることで、証拠がない状態を成功に見せない構造にした（Issue #219）。

`not_run`（未接続・Owner/再実行条件付き）と `unsupported`（capability 不足）の分離は、review 指摘を受けて固定行の語彙を分ける形で修復した。GUI-CONTRACT/fake oracle と実 window 必須 lane の `unsupported` 混同を解消するためである（Issue #294）。この分離は以後の schema 設計でも維持されている。

証拠の突合は SHA 単位で行い、祖先 SHA の成功を HEAD の Pass に転用しない。CI 照合は `gh run list`・`gh run view --json jobs`・`gh run view --job --log`・`gh api .../artifacts` の 4 段で行い、checkout-sha artifact の実測 SHA まで確認する（Issue #127）。`365c74ac..621490e8` に crates/・mise.toml・ci.yml の差分があるため、HEAD は `NotRun(Pending)` として分離記録した実例がある（Issue #127）。

## 証拠契約と fail-closed 設計

consumer-verify の証拠契約は `ci/consumer-verify/evidence.mjs` に単一権威として集約し、classify-ref/lane/finalize の 3 コマンドで構成した。判定ロジックを workflow YAML ではなくスクリプトに集約したことで、preflight が run ごとに証拠器を自己検証してから lane を使う構造になった（Issue #294）。

`evidence.mjs` の fail-closed 設計にはいくつかの具体的な判断がある。`shaMatch` は `EXPECTED_SHA`/`RESOLVED_SHA` から finalize 側で導出し、期待未提供は `null`、提供時に解決 SHA 未捕捉は `false` とした（Issue #294）。`GATE=run` のときのみ `RUVIFORM_REV` の 40-hex を要求し、blocked/not_run 終端経路では生値を「試行された入力」として記録する。`RUVIFORM_REV` 自身の不正形で finalize が throw すると証拠が消失するためである（Issue #294）。env は allowlist のみを読み、秘密系を転写しない（Issue #294）。

証拠器は runner 同梱の `node` で実行する。`mise exec -- node` ではなく plain `node` を使うのは、gate=blocked/not_run 経路では mise install が走らず壊れ得るためである（Issue #294）。また `runner` context は job env で参照できず（actionlint が `jobs.<job>.env` での `${{ runner.temp }}` 使用を拒否）、無条件先頭 step が `$GITHUB_ENV` に `CONSUMER_DIR`/`LANES_FILE`/`EVIDENCE_FILE` を供給する構成にした（Issue #294）。resolution step は `always() && gate==run` とし、clone step 自体の失敗時に GitHub が後続を skip するため、`always()` でなければ rev-parse 失敗を fail lane として記録できず証拠が欠落する（Issue #294）。

consumer-verify の証拠契約は Case schema v2 へ additive に拡張された。`SCHEMA_VERSION` を 1→2 に bump し、`CONTRACT = "consumer-verify-tuple/v1"`・`outcome` 語彙・`deriveStatus`/`coverage`/`markdownSummary`・top-level tuple/run/consumer 構造は不変とした。各 lane record は必須の `case_id`/`level`（`OS-L1`/`GUI-L2`/`EXT-L3`）/`environment_kind`（5 値閉集合）/`requirement`（`Required`/`Deferred`/`NotApplicable`、結果軸と直交）を保持する（Issue #294）。`readLanes` は `lane` コマンド経由でない行にも同一語彙の fail-closed 検証を課し、schema-v1 行・未知 enum・型不正・非 40-hex `resolved_ruviform_sha` を拒否する（Issue #294）。

contract の version 軸は contract 文字列が担い、`schemaVersion`/`lanes` は consumer 側で未検査とする方針を維持した。consumer が必要とするのは tuple 同一性と status であり、証拠内部語彙は producer 側契約であるという整理である（Issue #720）。v1→v2 の移行は hard cutover とし、legacy v1 証拠は `classifyRunEvidence` の contract 完全一致規則でそのまま non-target に分類される。受理集合の導入は不要と判断した（Issue #720）。

## 候補 tuple の検証と証拠の復元

consumer-verify は `(ruviform main SHA, audio-lens main SHA, synvelis main SHA, consumer-verify-tuple/vN)` の候補 tuple を検証単位とする。各 Consumer lane が `git rev-parse HEAD` の解決 SHA を捕捉し、任意入力の期待 SHA と fail-closed 照合する。ref 入力は branch/tag に加え 40-hex SHA を統一 fetch 経路で受理する（Issue #294）。

Actions run API は dispatch inputs を公開しないため、各 run が検証した tuple は `consumer-evidence-<consumer>-attempt-<run_attempt>` artifact の証拠 JSON から復元する fail-closed 経路を実装した（Issue #294）。証拠なし・期限切れ・破損・内部不整合は全て `unverifiable` とし、verified にも無視にもせず auto actor なら retry budget に保守的に計上する。証拠が完全に読めて tuple が candidate と異なる `non-target` と、証拠が欠ける/壊れる `unverifiable` を区別する（Issue #294）。

zip の解析には依存追加を避けるため最小自前 reader を使い、EOCD/central directory/local header を stdlib で走査する。data descriptor・zip64・method 0/8 以外は `ApiError`→unverifiable とする。artifact zip DL は 302 `Location` を手動追従し、token を署名 URL へ転送しない（Issue #294）。

`decide.mjs` の consumer 一覧は `candidate.consumers` が正本である。`--consumer-repo` flag は env 既定を置換し、append すると env 既定と明示 flag が暗黙 union されるため、初回 flag 出現で env 由来リストをクリアする（Issue #294）。初版は config consumer（`{name, repo}`、sha を持たない）を参照して全件 non-target になる bug があり、fixture テストで検出・修正した（Issue #294）。

## consumer-verify の lane 構成

consumer-verify.yml は preflight + verify-audiolens + verify-synvelis + summary の 4 job 構成の `workflow_dispatch` 専用情報 lane である（Issue #219）。verify-audiolens の lane 集合は fetch+lockfile guard→`check-ruviform-lock`→`check-ci`→`al-cli` ci build+`check-cli-headless`→`test-ci`（LFS pull）→UI 独立 chain（sync-ui-deps→check-ruviform-rev --require-vendor→setup-ui→check-ui→test-ui）である。verify-synvelis は presence check→fetch+lockfile guard→`check-ruviform-rev`→`check`→`test`（LFS pull）→`test-rt`→`check-headless-deps`→sync-ui-deps→setup-ui→check-ui→test-ui→build-ui→`check-ruviform-rev --require-vendor` で、manifest は build-ui 産物のため `--require-vendor` 検証は必ず後置する（Issue #294）。

synvelis lane を presence check stub の拡張としたのは、Cargo.toml 不在を not run で返す presence 判定を workspace 化前の ref を `synvelis_ref` に指定した場合の後方互換ガードとして残しつつ、存在時のみ実 lane を実行するためである。「Synvelis 未 buildable を fail にしない」方針を維持したままゲート本体を実装できた（Issue #219）。

consumer oracle lane は consumer checkout 内の canonical test file を `oracle_probe` step が file 単位で存在確認し、存在すれば `git lfs pull` の後に canonical command（`mise run test <pkg> --test <file>`）を file 粒度の別 step で実行する。Record step は probe output の 3 分岐（`'1'`→run outcome / `'0'`→owner 付き `not_run` / else→probe outcome）で lane 行を記録する（Issue #294）。SV の 2 oracle file（`rf628_oracle`・`rf628_oracle_rt_alloc`）を別 step・別 lane 行にしたのは、`--test` 併記の単一実行では coverage は同等でも file 別 verdict が埋没するためである（Issue #294）。`--expected`/`--observed` は記録しない。consumer test は機械可読の期待値/観測値を出力せず、捏造禁止で null のままとする（Issue #294）。

`fixture_version` は consumer `Cargo.lock` の `ruviform-audio-test` version から `ruviform-audio-test-<version>@<RUVIFORM_REV>` を構成する。抽出失敗時は flag を省略して null とし、値を捏造しない（Issue #294）。この形式は「同一 fixture version」の tuple 比較可能性を満たす可読形式として採用し、opaque hash 化は却下した（Issue #294）。

GUI-CONTRACT lane は候補 commit tree（`RUVIFORM_REV`）上で boundary guard（check-ui-driver-boundary）と Fake driver semantic（test-ui-driver-contract）を実行する 3 step（gui_candidate/gui_boundary/gui_semantic）を両 verify job に追加した。一律 clone で候補 tree を取得し、候補 `mise.toml` の `[tasks.<name>]` 存在を grep probe して実行可否を先に分岐する。task 不在は owner 付き `not_run` とし、dispatch ref での代用は禁止して偽 pass を構造的に排除する（Issue #294）。`gui_semantic` は rc 捕捉+summary 追記+exit で、失敗時は元の exit code で step fail にする（Issue #294）。

## consumer-verify のアクセス制御と manifest 正規化

consumer-verify には access mode fallback を実装した。3 repo（ruviform/audio-lens/synvelis）の public 化（2026-11-03 実測）で設計前提「全 repo private」が変化し、`CROSS_REPO_DEPLOY_KEY` 未設定のまま全 lane `not run` で証拠が滞留する構造を解消するため、secret 未設定かつ匿名 HTTPS read 可能な場合に `anonymous` mode で lane を実行する。`deploy_key` mode（SSH・known_hosts pin・`url.insteadOf`・cleanup）は private 前提の canonical 経路として維持し、secret 設定時は従来どおり優先される。anonymous HTTPS は秘密を一切持たない最小権限の下限であり、private 復帰に備えて deploy_key 経路を残しつつ public 期間は実走可能にした。probe は consumer 単位とし、spec の per-consumer 独立性を維持する（片 consumer の再 private 化が他 lane を止めない）（Issue #219）。

gate に `gate`（run/not_run/blocked）と `mode`（deploy_key/anonymous/not_run）の 2 output を持たせたのは、既存の `gate == 'run'` 条件を変えずに mode 別条件を追加でき、diff が最小で意図が明確なためである（Issue #219）。

manifest rev 書換は「`rev` pin 置換・`branch` 追従除去・`rev` 未指定への注入」を同一ルールで `rev = "$RUVIFORM_REV"` へ正規化する sed に両 lane で統一した。audio-lens の rev pin dep も synvelis の rev 未指定 dep も同じ guard で検証でき、`rev`→`branch` 形態変更にも追随する。書換 no-op は単一 SHA guard の 0 行検出で必ず fail し、silent pass にならない（Issue #219）。

consumer 側の mise install は lane が必要とする tool に scope する。audio-lens の `mise install --locked`（全 tool）が actionlint/pipx:serena-agent の platform entry 不在で失敗した実測（consumer mise.lock 側の gap・`--locked` は全 OS で復元不可）を受け、lane が使う rust/git-lfs/node のみに絞った。install 高速化の副次効果もある（Issue #219）。synvelis 側の `.cargo/config.toml` は consumer repo に追加せず job env で補完し、`CARGO_NET_GIT_FETCH_WITH_CLI=true` は CI 標準の env 経路で同一設定を実現した。synvelis 側の恒久的変更は当該 Issue の scope 外であった（Issue #219）。

## consumer-verify と issue tracker の配線

consumer-verify の失敗 run を既存の tracker 機構で追跡するため、`ci-issue-tracker.yml` の `workflow_run` trigger を `workflows: [ci, consumer-verify]` に拡張した。`workflow_run.name` でゲートされた scope 別 job（`track`/`track-consumer-verify`）がそれぞれ `--workflow`/`--workflow-file` で scope を明示して同一 evaluator を実行する。1 job 内で両 workflow を処理すると `github.event.workflow_run` がどちらの run か曖昧になるため、workflow 名で job を分けた（Issue #294）。

`ci-issue-tracker-reconcile.yml` は GITHUB_TOKEN dispatch の anti-recursion 補完として `ci`→`consumer-verify` の 2 scope を逐次実行する。daily-ci.yml が GITHUB_TOKEN で consumer-verify を dispatch するため anti-recursion で `workflow_run` completed が発火せず、event 経路だけでは daily dispatch 起因の失敗を取りこぼす。reconcile が権威ある run history を走査する既存境界をそのまま 2 scope 化し、新 framework は追加していない（Issue #294）。

tracker は evidence JSON の verdict を読まず、workflow run の conclusion のみを評価する。`not_run`/`unsupported` は consumer-verify.yml が failure conclusion を出さない限り tracker の失敗集合（`failure`/`timed_out`/`startup_failure`）に入らず、success でもない。close は既存の「新しい一意な success が失敗 revision と main tip を cover」規約のみに従う。verdict 語彙の変更が conclusion 写像を変えた場合は tracker の分類前提が崩れ得るため、この境界を contract test で固定した（Issue #294）。Issue title は scope 由来で `[CI failure][<branch>] <workflow> failed` を `issueTitle(scope)` で生成し、`ci` と `consumer-verify` が同一 Issue に混ざらないようにする（Issue #294）。

## daily scheduler と issue tracker

責務境界として、scheduler（daily-ci.yml/decide.mjs）は CI の dispatch 判定のみを行い Issue を作成しない。Issue 起票・close は tracker（ci/tracker/tracker.mjs + ci-issue-tracker.yml）が `workflow_run` completed を契機に行う（Issue #367）。

daily scheduler は「現在 main SHA に有効な Full CI がなければ ci.yml を `workflow_dispatch` で起動する」判定器で、判定行列は skip-verified/suppress-in-flight/hold-unknown/budget-exhausted/dispatch である。`decide.mjs` は stdlib-only・pure 判定コア + 注入可能 client + `main()` の構造で、ci/summary/summary.mjs と同一構造を採った。auto dispatch は event===workflow_dispatch ∧ triggering_actor bot のみ計上し、上限 `DAILY_MAX_AUTO_DISPATCHES`（既定 2）で budget-exhausted とする（Issue #367）。schedule は UTC `"17 21 * * *"`（06:17 JST）で、分を 0 以外にする規則を採った（Issue #367）。`daily-ci.yml` は workflow 全体 `permissions: {}`、単一 decide job（ubuntu-latest・contents:read + actions:write・timeout 15 分）に権限を限定し、concurrency で scheduler 自身を直列化（cancel-in-progress:false）する。`test-ci-daily` 自己検証を先行させ、report artifact を upload する（Issue #367）。

tracker evaluator（tracker.mjs）は `workflow_run` 完了ペイロードを scope（repo × workflow × branch × event allowlist）で分類し、GitHub Issue 一覧・job メタ・compare の注入 client 経由で create/update/close-recovered/close-duplicate/noop の action plan を組み立てる。close は多重 guard（最新成功・曖昧なく新しい・失敗 coverage・現在 main tip coverage・pending/unknown 履歴なし）を持つ（Issue #368）。

run の順序付けは `(run.id, run_attempt)` を単位とする。GitHub rerun は同じ run.id で run_attempt が増えるため、run.id 単独比較では失敗 run の成功 rerun が stale 扱いで永久に close 不能になる bug があった（Issue #368）。異 run 間は `updated_at` を採用し、欠損・同一時刻は `ambiguous` として close/成功判定から外す。run-history reconciliation で concurrency pending 置き換え・通知逆転・eventual consistency による完了通知の欠落/遅延に対応する（Issue #368）。

marker の suppress は `(runId, attempt)` 単位に修正した。runs 一覧は各 run の最新 attempt のみを返すため、run id 単位の suppress では「失敗 a1 記録済み + 一覧に同 id 成功 a2」で記録済み failure attempt が消え、coverage 判定不能で永久に auto-close 不能になることを reviewer mock replay が実証した（Issue #368）。marker JSON の injection 対策は二重化し、job 名を入力境界で sanitize し、marker JSON 内の `<`/`>` を unicode escape する（Issue #368）。履歴 record は `classifyHistoryRecord` で out/in/unknown に分類し、in・非 completed は `pending` 証拠、unknown は close と resolved-suppression の両方を止める（Issue #368）。

`ci-issue-tracker.yml` は `workflow_run {workflows:[ci], types:[completed], branches:[main]}` のみで起動し、pull_request/push/schedule/workflow_dispatch trigger を持たない。workflow 直下 `permissions: contents:read` を fail-closed baseline とし、track job のみ `contents:read`/`actions:read`/`issues:write` を限定付与する。checkout するのは default branch の自分自身であり、`workflow_run` は default branch の workflow ファイルで実行されるため失敗 run 側のコードには触れない。self-test（test-ci-tracker）を API 呼出し前に置き、失敗時は GitHub API に一切触れず job が落ちる順序にした（Issue #368）。

GITHUB_TOKEN dispatch の完了は `workflow_run` を発火させない anti-recursion gap がある。これを event 非依存の `--reconcile` モード（同一判定器・権威履歴 + marker のみ）と別 workflow の `schedule`/`workflow_dispatch` 入口で埋めた。`ci-issue-tracker.yml` には触れず、既存 fail-closed 契約・最小権限・既存 secret のみで実装した（Issue #368）。reconcile workflow の `if:` は `github.event_name == 'schedule' || github.ref_name == github.event.repository.default_branch` とし、schedule payload では `github.event.repository` が欠落し得るため ref 比較は dispatch 限定とした。checkout は `${{ github.event.repository.default_branch || 'main' }}` に pin する（Issue #368）。

## CI 計測と性能改善

CI 計測の正本契約は指標（wall/queue/job_wait/job_exec/exec_span/critical_path/runner_total）・lane 分類・Cold/Warm・Before/After 手順を固定する。収集ツールは `ci/metrics/collect-ci-metrics.mjs`（gh api read-only・Node 標準のみ）で、n<10 は統計値を出さず観察値のみ報告し、分類不能は `other` で名列挙する fail-closed 設計とした。`--logs-run` は opt-in 化し、log 取得が rate limit の支配項であることに対応した（Issue #327）。

計測の主要観察として、3 repo とも critical path は `verify`/`ui-verify` の windows-latest job であり、wall time ≈ 最長 job で単一 lane の短縮が wall に直結する構造だった（Issue #327）。mise tool cache は 3 repo 全 job で一貫して miss（job あたり 13–23s の固定コスト）で、cargo cache は ruviform/synvelis で hit するが audio-lens は Cargo.lock（ruviform rev）更新に伴う key 変化で頻繁に cold 化した。lockfile 更新頻度が cache 効率の支配因子と特定された（Issue #327）。

sccache は `mozilla-actions/sccache-action` を immutable commit SHA で pin し、sccache v0.19.0 を明示 pin して導入した。`SCCACHE_GHA_*` env は `actions/github-script` 経由で `GITHUB_ENV` へ export する。`ACTIONS_RUNTIME_TOKEN`/`ACTIONS_RESULTS_URL` は JS action コンテキストでのみ確実に参照可能なためである。kill switch は repo variable `CI_SCCACHE_DISABLE=1` で workflow YAML 変更不要とした。content-addressed key（compiler hash+args+preprocessed source）で誤共有を構造排除する。GHAC は repo 単位 ~10GB・write rate limit が既知の制約である（Issue #329）。

sccache の cold/warm 実測では verify lane で 47–62% cache hit・job 時間 10–30% 削減（ubuntu -51s・windows -78s・macos -32s）を記録した。cold run で 36-100% の cache write error が発生したが warm run で verify 系は 0 であり、原因は GHA cache backend の rate limit / 並列書込競合（6 job 並列が同時に数百エントリを書き込み）と判断した。sccache は write error でも compile を継続・fail-open するため CI green を維持する（Issue #329）。

npm ci の集約は、`install-ui-*` タスク（7 本、run=npm ci、sources=lockfile+manifest、outputs=`node_modules/.package-lock.json`）と集約 `setup-ui` を新設し、verify 系タスクは `depends` 経由で準備済み node_modules を再利用する形で実現した。freshness は「副作用を持つ準備・emit 系タスクのみ」に限定し、検証 skip は検証能力削減に相当するため check/test 系には付与しない設計とした。`node_modules/.package-lock.json` を output マーカーに選定したのは、npm ci 成功時に必ず生成され、部分 install や node_modules 削除で stale になるためである。Before/After で npm ci 実行回数は ui-verify job あたり 20→7、ui/showcase dist emit は 3→1 となった（Issue #330）。

`check-ci` を `lint-ci` に統合する等価性は、注入実験 8 ケースで実証した。`cargo check` と `cargo clippy` は同一の 292 check-mode ユニット（152 依存 + 140 workspace）を評価し、workspace メンバー 140 ユニットは `rustc` vs `clippy-driver` の compiler identity 差で二重コンパイルされていた。`cfg(clippy)`/`cfg(not(clippy))` の使用は `crates/` 配下にゼロで、check と clippy でコードパス分岐はない。型エラー・rustc 警告・clippy lint・cfg(test) 型エラー・example/bench 警告・失敗テストを全て期待 lane で検出することを確認した。変更後 CI run で check+lint 合計比 3OS とも −28〜−37s であった。`check-ci` タスクはローカルトリアージ向けに `mise.toml` に残存させた（Issue #331）。

Playwright Chromium cache は `install-ui-browser-chromium` mise task を新設し、`test-ui-browser` は depends 経由で解決する。cache key は `playwright-<OS>-<arch>-pw<version>-chromium<revision>` で restore-keys なしとした。部分一致で旧 revision binary を復元すると test が旧 browser で走り得るため、fail-closed 設計とした。`PLAYWRIGHT_BROWSERS_PATH` を workspace 内 `.cache/ms-playwright` に pin し、launch 検証は playwright screenshot の実 launch probe で行う（ファイル一覧では破損 binary を検出不能）。Linux system deps（install-deps）は cache と別レイヤーで毎 run 実行を維持した（Issue #332）。

bench-smoke lane は対象を bench target 全量の 5 crate（ruviform-math/rt/viz/numeric/io-e2e）へ拡張し、bench profile 破損を CI lane で恒久的に検出できる状態にした。`--workspace` 化は smoke の責務を超え、bench target を持たない crate の lib test target まで暗黙に広げるため不採用とし、`-p` 列挙追加のみとした（Issue #650）。

## 選択実行と Full CI への移行

impact-based CI は `ci/impact/`（stdlib-only Node .mjs）に classifier + map + gate を実装した。分類は `cargo metadata` の workspace 逆依存伝播・`rt_` 保持 crate scan・mise task `-p` 引数・`ui/*` DAG・full-fallback 規則で機械導出し、曖昧/分類不能/共通設定変更は Full CI に倒す。diff は `git fetch --depth=1 origin $BASE` 後の two-dot tree diff で、merge-base 不要で shallow clone と両立する。`edited` type を採用したのは、title 編集でも再実行されるが concurrency group が前 run を cancel するため実害がなく、base retarget の stale 期待集合を防ぐ方が重要なためである（Issue #333）。

Shadow Mode では lane job の `if:` を一切変更せず既存 12 required contexts を維持したまま、`impact`（分類+decision artifact）と `ci-gate`（`if: always()` で needs 全結果を判定・gate-report artifact）を実運転で観測した。`ci-gate` は matrix leg ではなく job 粒度の `needs` result を判定し、per-OS 内訳は各 leg 自身の check run が持つ。権限追加（checks API）を避けるためである（Issue #333）。gate は fail-closed で、expected lane の missing/failure/cancelled/skipped・分類失敗は全て Fail とし、skipped を Pass にしない（Issue #333）。

選択実行の有効化（Issue #346）では 6 lane job に `needs: [impact]` + `if: always() && needs.impact.outputs.lane_* != 'false'` を配線した。`if:` は impact 失敗・出力欠落時に lane が実行される側に倒す fail-open coverage で、fail-closed 性は gate の classification-error Fail 側に保持する。macOS legs は job 単位の needs では matrix leg を部分的に外せないため matrix 縮小 + `*-macos` 別 job として物理分離し、ci-gate の needs から存在ごと除外した。ruleset required contexts は ci-gate 単一へ「追加→報告確認→除去」の 2 段階で無保護期間なく移行した（Issue #346）。

Full CI Stage 1（Issue #366）では ci.yml から impact job・全 needs:[impact]・全 lane if:・CI_IMPACT_DISABLE env を撤去し、11 lane を無条件実行にした。required context を産む verify/l1-real-verify/ui-verify の job id・matrix os 配列・`name:` は不変に保ち、`ci-summary`（needs 11 件・`if: always()`）と `io-structured-verify` lane を新設、`workflow_dispatch` に `expected_sha` 入力を追加した。`ci/impact/` と mise task test-ci-impact は削除した。`consumer-impact-check.yml` は `github.event.pull_request` を読む pull_request 専用設計のため workflow_dispatch へ転用できず、ファイル単位で除去した。Consumer 影響分類要件自体は PULL_REQUEST_TEMPLATE.md の表 + レビュアー human gate に残り、required check でないため merge デッドロックは生じない（Issue #366）。

`ci-summary` の verdict 行列は all-success→pass / failure/cancelled/skipped/missing/unknown→fail で、expected_sha は空=照合なし・一致・不一致を評価し、NEEDS_JSON 不整合（parse 失敗）は fail とする（Issue #366）。

MSRV lane（Issue #594）では ci.yml に `msrv-verify` job を追加し、4 つの `test-*-msrv` task を rust@1.89.0 で日次 CI 担保した。`ci-summary` は needs だけでなく `summary.mjs` の REQUIRED_JOBS/REQUIRED_CHECKOUT_LEGS 宣言も fail-closed で検査するため、job 追加だけでは不足し、宣言一覧と列挙 test の更新が唯一の整合的変更だった。4 task を個別 step 化し各 step に `if: ${{ !cancelled() }}` を付し、先行 task 失敗時も後続 task が skip されず全件実行される構造にした。`always()` でなく `!cancelled()` を選んだのは、cancelled run で証跡だけ残る誤判定を防ぐためである（Issue #594）。

macOS GPUI lane（Issue #598）は matrix ではなく物理分離 job `ui-gpui-verify-macos` として追加した。matrix leg は `needs.<job>.result` が 1 値に集約され失敗 OS を判別できないためである。`build-full` は接続しない。`test-full`（cargo test --workspace）が実行 bin の Build/Link を実質包含し、唯一の空白だった `windowed_test_app` は `test-ui-gpui` の `--features "gpui-test uitest"` 拡張で塞いだ。feature 組合せは gpui-test+uitest の 1 組に統一し、gpui 本体の再 compile を増やさない（Issue #598）。この新 lane が macOS 専用 cfg 内の潜在 lint 欠陥（`clippy::doc_markdown`）を fail-closed で検出した実績がある（Issue #598）。

## GPUI offscreen lane の実装知見

GPUI offscreen lane は Xvfb+openbox+lavapipe で X11 GetImage による実 pixel readback を行う。window 解決は `_NET_WM_PID` + `_NET_CLIENT_LIST` 優先 + `QueryTree` fallback とする。openbox は client window を frame window に reparent するため root-child 列挙では拾えない。`_NET_CLIENT_LIST` が無い bare Xvfb では tree walk を使う。0/複数 match は推測せず `UiError::Host` とする（Issue #314）。

window 識別に `_NET_WM_PID` を採用したのは、gpui 0.2.2 の X11 PlatformWindow は `HasWindowHandle`/`HasDisplayHandle` が `unimplemented!()` で rwh seam が panic するためである。gpui 自身が open 時に `_NET_WM_PID = process::id()` を設定するため、capture 側は lazy に own toplevel を解決する。`usize::MAX` sentinel は XID（29-bit）や HWND/NSView と衝突しない（Issue #314）。

bare Xvfb（WM 無し）では present 不成立（黒画面）で openbox が必須要件である。mesa ≥24.2 も必須で、22.3.6 では depth-24 PutImage BadMatch で恒久黒画面となる。`--test-threads=1` も必須で、同一 PID の toplevel が並走すると `_NET_WM_PID` 解決が ambiguous になり、occlusion で GetImage が下位 window を読めない（Issue #314）。`client_geometry` の `GetGeometry` は inside size（border は drawable 外）なので `border_width` を引かない（Issue #314）。

却下した代替案を記録する。bare Xvfb は mesa 25.0.7 でも黒画面で、根本原因は未確定だが lane 要件として openbox を採用したため追求しない。Mesa 22.3.6 は depth-24 PutImage BadMatch で恒久的黒画面。`MESA_VK_WSI_PRESENT_MODE=FIFO` は depth BadMatch を解決しない。`-pixdepths 24`/depth-32 screen 検証・Xvnc/Xephyr/Xorg dummy はいずれも depth-32 visual を隠せず gpui の visual 選択を変えられない。gpui headless platform は stub で render path ではない。真の render-target offscreen readback は gpui 0.2.2 の公開 API に存在しない（copy_texture_to_buffer の対象 texture は private）（Issue #314）。

`libc` を 0.2.189 に pin したのは、xattr 0.2.3 が参照する `libc::ENOATTR` が libc 0.2.190 で Linux cfg から削除され、gpui の Linux build が不能になった upstream 既知問題の lockfile 修正である。pin は lockfile 再生成で失われやすく、実際に `4bcf1402`（PR #572）の lockfile 再生成で 0.2.191 への drift が発生し gt-05 の pin が潰れた（Issue #125・Issue #582）。

## mise タスクとローカル/CI の共有

mise task は fmt-ci を作らない。cargo fmt はプロファイル非依存のため fmt-ci は定義せず、verify-ci は fmt-check→check-ci→lint-ci→test-ci の順次実行の集約入口とし、先頭はプロファイル非依存の fmt-check とした。verify-ci は `depends` ではなく `run` 配列で mise run を順次呼ぶ形とし、順次実行を保証した。`*-ci` 系と verify-ci には `env = { CARGO_INCREMENTAL = "0" }` を付与した（Issue #2）。

description の副作用表記は、fmt は「rewrites source files in place」と書き換え系であることを明示し、それ以外は「non-destructive; writes only target/」系の表現で統一した。書き換え系と非破壊チェックの区別が読み取れる規則を満たすためである（Issue #2）。

mise lock には既知 caveat がある。`pipx:apm-cli` の `uvx_args` で dependency locking が非対応のため `mise lock` が exit 1 となる。原因は既存の `"pipx:apm-cli" = { uvx_args = "--prerelease allow" }` であり、actionlint 追記は正常完了しており退行ではない（Issue #2）。

## CI キャッシュと依存管理

cargo キャッシュは target/ 全体・中間成果物はキャッシュせず、`~/.cargo` の registry index・registry cache・git db のみを Cargo.lock ハッシュでキー付けする限定形を採った（Issue #2）。dev-dep のバージョン方針は caret 指定とし、追跡済み `Cargo.lock` を再現性の錨とする。`=x.y.z` exact pin は採用しない（Issue #19）。MSRV 検証手順は `rustup run 1.85 ... --lib --locked` に統一した（`cargo +1.85` は rustup proxy 前提のため）（Issue #19）。

冷キャッシュ時の `cargo metadata --offline` 失敗に対しては、`mise exec -- cargo fetch --locked` step を追加して解消した（Issue #125・Issue #582）。`cargo fetch --locked` は yaml 構文・job 構造を壊す可能性があるため actionlint/prek hook と CI run で確認する（Issue #582）。

## 実測で判明した落とし穴と修復

いくつかの失敗は実 dispatch run で初めて顕在化した。run 43270011255 の 3 系統失敗では、`ui-gpui-verify` の冷キャッシュ `cargo metadata --offline` 失敗、`libc` の 0.2.189 pin 喪失、moxcms 0.9.2 の ICC fallback に対する `IccTransform::new` の missing-intent-tag reject 再強制、`BACKEND_AVAILABLE = cfg!(windows)` により Linux で `declare_slot` が常に `Unsupported` を返すのに `rvf_view_native_slot_*` 5 test が成功を仮定する既存破損を `#[cfg(target_os = "windows")]` で gate する修復を行った（Issue #125・Issue #582）。

consumer-verify の実 dispatch run 41604498852 では verify-audiolens が check-ci・smoke 2 lane 通過のうえ `test-ci` で `al-cli --test cli` export 3 test fail を検出し summary fail となった。gate の意図どおり consumer 側潜在欠陥（`analysis/inputs/IN-*.metadata.json` の absolute path embed）を検出した（Issue #219）。

Issue #294 の実 runner dispatch（run 42302538464）では、verify-synvelis 全 lane pass、verify-audiolens は `check-ruviform-lock` まで pass し `check-ci` で fail した。`al-observability/src/lib.rs:264` の `SinkConfig.span_events` 初期化子欠落（E0063）で、該当 field は RF #335 で RF main に既導入済みであり AL main 側の未追随が実不整合として検出された。fail 以降の AL lane は `skipped` 派生記録、summary の not-run-is-not-pass 強制も発火した。この修復は AL repo 側の別 task に委ね、workflow で補償しない方針とした（Issue #294）。

synvelis lane の実測（`f7aadd0` scratch clone）では、manifest 正規化 → `CARGO_NET_GIT_FETCH_WITH_CLI=true` で `cargo fetch` → 単一 SHA guard（total=15・bad=0）→ `mise run check` exit 0 → `git lfs pull` + `mise run test` exit 0（23 suite・0 failed）を確認した（Issue #219）。

## GitHub Actions の落とし穴

workflow の review で確定した落とし穴を記録する。`url.insteadOf` の `.git` suffix、`secrets` の `if:` 非対応、`inputs.default` 非評価である（Issue #219）。`$RUNNER_TEMP` は env context に無く（default env var は `env:` 宣言値のみ）、step の `working-directory`・`with:` には `${{ runner.temp }}` を使い shell 内は `$RUNNER_TEMP` を使う形に統一した（Issue #219）。証拠出力 step の `working-directory` は gate==run の step のみに限定し、結果記録は repo 既定 dir + `git -C` 参照にした。`if: always()` の証拠記録 step が clone 未実行（not run）時に working-directory 不在で失敗しないようにするためである（Issue #219）。

`mise-action` は clone 前に配置する。cargo fetch step が `mise exec` を呼ぶためである（Issue #219）。fingerprint 定数は GitHub 公表の 3 値（RSA・ECDSA・Ed25519）を workflow 内ハードコードとし、`ssh-keyscan` 結果を照合する真の pin とした（Issue #219）。

`workflow_run.workflows` は name フィルタのため同名別ファイルの混入余地が残る。`classifyRun` の name/path 照合で証拠化を防ぐ defense-in-depth とした（Issue #243・Issue #294）。

## 未確定・保留事項

素材が記録する未解決・保留事項を列挙する。

- Issue #294 umbrella の close 条件は未確定で、`Refs #294` のみで Open 継続のままか別 slice で close するかは判断されていない。
- Issue #294 の実 runner dispatch（consumer-verify workflow_dispatch・daily-ci tuple 判定）は全 slice で NotRun であり、post-merge の Coordinator 管轄で実測待ち。
- Issue #294 consumer-oracle-lanes の per-case（per-test fn）JSONL emitter + ingest 契約は consumer repo 側後続 slice（AL #127 / SV #115）待ち。
- Issue #294 virtual-first-evidence-schema の `docs/development-environment.md` 証拠契約記述の後続同期は別 Worker リース待ち。
- Issue #127 linux-distribution の現行 main HEAD（`621490e8`）の CI 結果は未取得（daily-ci dispatch 待ち・NotRun(Pending)）。実機 Linux GUI/Plugin・対象 distro 決定・package 形式採用は利用要件発生まで Deferred。
- Issue #219 consumer-verify の初回 dispatch 実走（credential 設定後）と `consumer-impact-check.yml` の spec/ADR status 昇格・required check 化・自動 trigger は Issue Done 基準達成後に判断。
- Issue #313/#314（実窓 host・macOS/実 OS 正式受入）、#386（GPUI driver）、#388/#389（実 plugin）、#629/#630/#631（実 OS lane）の完了待ち。
- Issue #646（bench profile 破損）の再発防止は Issue #650 で対応したが、`*-msrv`・`*-full`・3 OS 横断は NotRun。
- Issue #720 consumer-tuple-v2 の post-merge 実 run gate（AC5）は Coordinator 所有で NotRun。
- GT-05 offscreen lane の ubuntu-24.04 stock mesa（24.2.x）での実測は CI 初回で確認予定。bare Xvfb で present が成立しない正確な機序（gpui の visibility/refresh か mesa WSI present mode か）は未解明。
- Issue #594 MSRV lane の `docs/development-environment.md` lane 数記述の stale 更新と Issue #598 macOS GPUI lane の lane 構成記述の更新は編集範囲外のため未対応。Issue #598 `POLICIES.md` L3-4 の行末空白（main 既存欠陥）の修復も未対応。
- Issue #329 sccache の GHA cache backend rate limit / 並列書込競合の恒久対応、Cargo.lock 更新時・toolchain 更新時・fork PR の GHAC 挙動・RT/ICC/L1/bench 展開・Consumer 適用は NotRun/範囲外。
- Issue #330 npm ci dedup の freshness 判定の実運用での stale 検出精度は未検証。Issue #331 の `cfg(clippy)` 使用ゼロ確認の継続監視、Issue #332 の cache miss 率、Issue #333 の偽陰性検出器の発動実績、Issue #366/#367/#368 の各判定行列の実運用での誤判定実績はいずれも未観測。
- Issue #368 の実 workflow_run 発火・実 cron・実 Issue・実 dispatch は全て未検証（offline contract test + actionlint のみ）で、merge 後の初回実測が受入。
- Issue #582 の CI 実測（dispatch run の最終 verdict）は PR 本文に記載とあるが結果 URL が未記載で、PR 側での確認が必要。
- Issue #583 の consumer-verify mise `mr_boxington` エラーの根本原因（mise 2026.11.7 windows-x64 の `mise install --locked`）は未解明。
- Issue #346 の After メトリクス（選択実行有効化後の wall/runner_total/lane skip 率）は未実測。
- Issue #328 の AC3（同一 PR 連続 push で旧 run cancel の実測）と別 PR/main push/dispatch の非 cancel 実測は coordinator 引き継ぎで未実施。
- Issue #330 の CI 3 OS matrix の実測（typecheck/test/build/Evidence・workflow wall time Before/After）は PR CI run での確認事項。
- Issue #332 の Playwright version bump 時の cold 再計測は実際の bump PR での観測が未発生。
- Issue #333 の部分集合 expected の実 run 証拠と workflow_dispatch・base retarget・kill switch の実動作観測は Shadow 期の確認対象として未観測。
- bench harness dev-deps の caret 指定 + `Cargo.lock` 錨の実運用での再現性と MSRV 検証手順の有効性は未検証。
- Issue #127 linux-dist 設計の判定語彙と正直性ルールの実運用での遵守実績、Issue #219 の `url.insteadOf` suffix・`secrets` `if:` 非対応・`inputs.default` 非評価の再発防止、manifest rev 書換 sed の no-op 検出精度、anonymous HTTPS fallback の repo visibility 変化時の影響はいずれも実運用での検証が未了。
- Issue #294 candidate tuple evidence の `shaMatch` 導出・`GATE=run` 時のみ 40-hex 要求・env allowlist の実運用での誤判定/漏洩防止精度、daily tuple trigger の tuple 復元経路・zip reader・token 非転送の精度、GUI-CONTRACT lane の候補 tree 取得・task probe・cargo fetch の精度、Case schema v2 の非破壊性・fail-closed 検証・`rec()` spec 解析・既定分類・`resolved_ruviform_sha` 限定の精度はいずれも未検証。
- Issue #294 consumer-verify → ci-issue-tracker 配線の scope 分離・anti-recursion 補完・conclusion 評価・scope 由来 title の実運用での精度、daily-ci.yml の GITHUB_TOKEN dispatch 起因の失敗取りこぼし防止の実効性は未検証。
- Issue #326 ci.yml の trigger 不存在 contract test の実運用での精度、Issue #327 計測契約の遵守精度・n<10 報告・fail-closed 分類・opt-in ログ取得の精度、critical path 特定・mise cache miss・cargo cache cold 化の精度は未検証。
- Issue #328 concurrency group の実運用での精度・required checks 不変性・cancelled check run 分類・手動 re-run 影響・fail-safe 精度・復旧手順・merge queue 対応は未検証。
- Issue #329 sccache pin・GITHUB_ENV export・kill switch・cache hit・write error 原因・fail-open・kill switch 実証の実運用での精度は未検証。
- Issue #650 bench-smoke 5 crate 拡張の CI `bench` job 時間への影響の実測は未実施。
