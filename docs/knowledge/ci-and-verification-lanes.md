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
  source: ["https://github.com/scarecrow173/ruviform/issues/2", "https://github.com/scarecrow173/ruviform/issues/19", "https://github.com/scarecrow173/ruviform/issues/125", "https://github.com/scarecrow173/ruviform/issues/127", "https://github.com/scarecrow173/ruviform/issues/219", "https://github.com/scarecrow173/ruviform/issues/243", "https://github.com/scarecrow173/ruviform/issues/294", "https://github.com/scarecrow173/ruviform/issues/312", "https://github.com/scarecrow173/ruviform/issues/314", "https://github.com/scarecrow173/ruviform/issues/326", "https://github.com/scarecrow173/ruviform/issues/327", "https://github.com/scarecrow173/ruviform/issues/328", "https://github.com/scarecrow173/ruviform/issues/329", "https://github.com/scarecrow173/ruviform/issues/330", "https://github.com/scarecrow173/ruviform/issues/331", "https://github.com/scarecrow173/ruviform/issues/332", "https://github.com/scarecrow173/ruviform/issues/333", "https://github.com/scarecrow173/ruviform/issues/346", "https://github.com/scarecrow173/ruviform/issues/366", "https://github.com/scarecrow173/ruviform/issues/367", "https://github.com/scarecrow173/ruviform/issues/368", "https://github.com/scarecrow173/ruviform/issues/582", "https://github.com/scarecrow173/ruviform/issues/583", "https://github.com/scarecrow173/ruviform/issues/594", "https://github.com/scarecrow173/ruviform/issues/598", "https://github.com/scarecrow173/ruviform/issues/646", "https://github.com/scarecrow173/ruviform/issues/720"]
  informs: []
  explains: []
  supersedes: []
---

# CI レーン・ゲート・検証基盤の運用

本書は、Ruviform リポジトリとその Consumer リポジトリ（audio-lens・synvelis）にまたがる CI レーン・ゲート・検証基盤について、設計判断と運用知見をまとめたものである。各記述は出所の Issue 番号を括弧で示し、実測値や観測された挙動は事実として記し、設計者の解釈や判断はその旨が分かるように書き分ける。

## 検証タスクの一本化と workspace 設定の集約

ローカル開発と CI が共有する検証入口は mise タスクに一本化し、CI 側は同一コマンドの `--profile ci` 差分として表現する（Issue #2）。ローカル通常タスク（fmt・fmt-check・check・lint・test・build）と CI 向けタスク（check-ci・lint-ci・test-ci・build-ci）を定義し、`cargo fmt` はプロファイル非依存のため `fmt-ci` は作らない。`verify-ci` は `depends` ではなく `run` 配列で fmt-check→check-ci→lint-ci→test-ci を順次呼ぶ集約入口とし、`*-ci` 系には `env = { CARGO_INCREMENTAL = "0" }` を必須付与する。task の description では、fmt を「rewrites source files in place」と書き換え系であることを明示し、それ以外を「non-destructive; writes only target/」系に統一して、書き換え系と非破壊チェックの区別を機械的に読み取れるようにした（Issue #2）。

workspace のプロファイルと lint 設定はルート `Cargo.toml` に集約する。`[profile.*]`・`[workspace.package]`・`[workspace.lints]` はルートのみが置き場で、メンバー側に profile や独自 lint を書くと Cargo が警告対象とするため、メンバーは継続宣言のみとする。`Cargo.lock` はライブラリ慣習の非追跡を採らず Git 追跡対象とし、`target/` は ignore で除外する。雛形コマンド（cargo new/init）は使わず全ファイルを手書きし、生成物を完全に制御する方針を採った（Issue #2）。`mise lock` には既知の caveat があり、`pipx:apm-cli` の `uvx_args` が dependency locking 非対応のため全 platform エントリ書き込み後に exit 1 となる。lock 書き込み自体は完了しているので、この exit 1 を退行と誤認しない運用知識である（Issue #2）。

unsafe コードの有無は `grep -rn "unsafe"` ではなく lint ゲートで検証する。grep はコメントや doc 中の言及を拾う偽陽性があるため、`mise run lint` の `unsafe_code` lint 通過を正の検証とした（Issue #19）。clippy pedantic の運用では `assert!(!x.is_empty())` が `clippy::assert_is_empty` で error になるため、非空確認の意図を保った `assert_ne!(x, "")` 形式を採用している（Issue #2）。

## ワークフローの起動方式とトリガ構成

CI ワークフローは `workflow_dispatch` 専用で運用する。初期の ci.yml は `workflow_dispatch` 専用・`permissions: contents: read`・ubuntu-latest 単一 job で作成し、全ステップを生 cargo コマンドなしの `mise run` 経由で記述した（Issue #2）。`workflow_dispatch` は default branch にワークフローが存在しないと Actions UI から起動できないため、ブランチ上では静的検証（YAML チェックと actionlint）に留め、実動作はマージ後の手動ディスパッチで確認する運用とした。ただし `workflow_dispatch` は `--ref` でブランチ指定が可能であり、「マージ前に実走できない」という前記述は後の実測で訂正されている（Issue #2・Issue #294）。

Issue #326 で ci.yml の `on:` から `pull_request`/`push` を除去し `workflow_dispatch` へ確定した。workflow 横断で `pull_request:`/`pull_request_target:`/`push:` trigger が 0 件であることを contract test で固定し、YAML を parse せず `on:` block の字句一致とコメント除去で検査する。移行の前提として branch protection を実測で確認し、`main-checkstatus` ruleset が disabled・`main-protect` が active・Classic 保護が不存在であることを押さえた（Issue #326）。残る既知のリスクとして、PR 自身の `pull_request` 起動が base 側ファイル評価によって止まらない可能性があり、その場合は base 評価と記録して受入を満たす方針である（Issue #326）。

同一 PR の連続 push に対する stale run の cancel は workflow-level `concurrency` で制御する。`pull_request` では group を `${{ github.workflow }}-refs/pull/<N>/merge` として `cancel-in-progress: true`、それ以外の event では `${{ github.workflow }}-<run_id>` で cancel 無効とする（Issue #328）。`pull_request` の `github.ref` は PR 番号で一意になるため、fork・reopen・別 base でも同一 group に入る。cancelled check run は success/skipped を満たさず合格とみなされないため、cancel は fail-safe 方向にのみ働く。既知の edge として、古い run の手動 re-run が同 group に再入して最新 run を cancel し得るが、古い revision の check が欠けて merge 不可になるだけで誤マージは起きない。merge queue（`merge_group`）導入時は group 設計を再検討する（Issue #328）。

## 判定語彙と正直性ルール

lane 結果の判定語彙は `Pass`/`Fail`/`Unsupported`/`NotRun`/`Blocked` を採用し、`NotRun` を `Pass` にしない正直性ルールを各ゲートの設計原則とした（Issue #127）。consumer-verify はこの原則を fail-closed に実装し、credential 未設定時は `not run` を出力して全 lane を not run とし、全 lane not run または fail/cancelled の場合は workflow 自体を失敗させる。証拠ゼロを緑にしないためである（Issue #219）。

`not_run` と `unsupported` は意味を分離する。`not_run` は「未接続・Owner 側実装待ち・再実行条件付き」、`unsupported` は「capability 不足」を指す。実 window 必須 lane と未接続 lane の `unsupported` 混同が review で指摘され、固定行をこの 2 語彙へ分けて修復した（Issue #294）。probe が実行可と判定したのに run step が skip された場合は `skipped` と記録され、pass 化しない（Issue #294）。

証拠の突合は SHA 単位で行い、祖先 SHA の成功を HEAD の Pass に転用しない。確立した照合手順は 4 段で、`gh run list` で run を列挙し、`gh run view --json jobs` で lane×OS の job 結果を確認し、`gh run view --job --log` でテスト集計を読み、`gh api .../artifacts` と zip 展開で checkout-sha artifact の実測 SHA を照合する。祖先と HEAD の間に `crates/`・`mise.toml`・ci.yml の差分がある例では、HEAD を `NotRun(Pending)` として分離記録した（Issue #127）。同じ正直性の原則から、Linux 配布形式（deb/AppImage/Flatpak）のような未実測の項目は「配布可能」と断定せず `保留` と記録する（Issue #127）。

## consumer-verify の lane 構成とアクセス制御

consumer-verify.yml は preflight・verify-audiolens・verify-synvelis・summary の 4 job からなる `workflow_dispatch` 専用の情報 lane である（Issue #219）。audio-lens lane は依存アクセス確認→manifest 正規化→`cargo fetch`→単一 SHA guard→`check-ruviform-lock`→`check-ci`→cli の ci profile build→`check-cli-headless`→`test-ci`（LFS pull 後）→UI 独立 chain（sync-ui-deps→`check-ruviform-rev --require-vendor`→setup-ui→check-ui→test-ui）で構成される。synvelis lane は presence check→fetch→`check-ruviform-rev`→check→test（LFS pull 後）→test-rt→headless-deps 確認→UI 系 task→build-ui→`check-ruviform-rev --require-vendor` で、manifest は build-ui の産物であるため `--require-vendor` 検証は必ず後置する（Issue #294）。synvelis lane は presence stub の拡張として実装し、Cargo.toml 不在を not run で返す判定を workspace 化前の ref を指定した場合の後方互換ガードとして残した。「Synvelis が buildable でなくても fail にしない」方針を維持したままゲート本体を実装するためである（Issue #219）。

アクセス制御は deploy_key・anonymous・not_run の 3 mode とした。設計前提は当初「全 repo private」だったが、3 repo の public 化（2026-11-03 実測）により、`CROSS_REPO_DEPLOY_KEY` 未設定のまま全 lane が not run で証拠が滞留する状態が生じた。これを解消するため、preflight が consumer 単位の匿名 probe（HTTPS の `git ls-remote`、consumer と ruviform の両 repo を要求）を出力し、secret 未設定かつ匿名 read 可能な場合に anonymous mode で lane を実行する。anonymous HTTPS は秘密を一切持たない最小権限の下限であり、`deploy_key` mode（SSH・known_hosts pin・`url.insteadOf`・cleanup）は private 復帰に備えた canonical 経路として維持し、credential 設定時はそちらを優先する。probe を consumer 単位にしたのは、片方の consumer が再 private 化しても他 lane を止めない per-consumer 独立性を保つためである（Issue #219）。

deploy_key mode の SSH clone 経路は、`RUNNER_TEMP` 配下に鍵を配置し、GitHub 公表の fingerprint 3 値（RSA・ECDSA・Ed25519）を workflow 内ハードコード定数として `ssh-keyscan` 結果と照合する真の pin で known_hosts を構成し、`GIT_SSH_COMMAND` で depth 1・`GIT_LFS_SKIP_SMUDGE=1` の clone を行う。続いて `url.insteadOf` による SSH rewrite→manifest rev 一括書換→`cargo fetch`→repo URL キーの単一 SHA guard→`mise install`→check/test（test 前に `git lfs pull`）の順で実行し、job 末尾の cleanup（`if: always()`）で `url.insteadOf` 除去と鍵削除を行う。cleanup 後の git/ssh 操作は禁止とする（Issue #219）。

## manifest 正規化と consumer 環境の非侵入的設定

consumer manifest の rev 書換は、`rev` pin 置換・`branch` 追従除去・`rev` 未指定への注入という 3 依存形態を同一の sed ルールで `rev = "$RUVIFORM_REV"` へ正規化する。既存 `rev`/`branch` を strip して `}` 直前に注入する一括処理に両 lane を統一したのは、別ルールを持つと依存形態差が保守点になるためである。書換が no-op になると単一 SHA guard（Cargo.lock の `?rev=<候補>#<候補>` 照合）の 0 行検出で必ず fail するため、silent pass にはならない。manifest が 1 行 inline-table 形態であることを前提としており、multi-line table 化されると guard で検出される（Issue #219）。

consumer 側の恒久的変更は避け、job env で補完する。`.cargo/config.toml` を consumer repo に追加せず、`CARGO_NET_GIT_FETCH_WITH_CLI=true`・`GIT_LFS_SKIP_SMUDGE=1` を job env で供給した。`CARGO_NET_GIT_FETCH_WITH_CLI` は `true`/`false` の文字列が必須で、`1` は cargo が reject する（Issue #219）。consumer の `mise install` も lane が使う tool に scope する。実 dispatch で audio-lens の全量 `mise install --locked` が actionlint/pipx:serena-agent の platform entry 不在により失敗した（consumer の mise.lock 側の gap で `--locked` は全 OS で復元不可）。lane が使う rust・git-lfs・node のみに絞り、install 高速化の副次効果も得た（Issue #219）。

## 証拠契約と証拠器 evidence.mjs

lane 結果の判定・証拠生成は `ci/consumer-verify/evidence.mjs`（Node 標準ライブラリのみ）に単一権威として集約する。classify-ref・lane・finalize の 3 コマンド構成で、preflight が run ごとに証拠器の自己検証（fixture 化された contract test）を実行してから lane を使う構造とした（Issue #294）。evidence.mjs 自身の取得には `actions/checkout`（ref 未指定＝dispatch 対象 ref）を preflight と両 verify job の先頭に置き、候補 SHA（`RUVIFORM_REV`）からは参照しない。証拠器 script 導入前の候補 revision を検証する run でも証拠器が存在するようにするためである。実行には runner 同梱の `node` を使う。`mise exec -- node` でなく plain `node` とするのは、gate=blocked/not_run 経路では mise install が走らずコマンドが壊れ得るためである（Issue #294）。

fail-closed の設計判断は具体的な失敗経路に基づく。`shaMatch` は `EXPECTED_SHA`/`RESOLVED_SHA` から finalize 側で導出し、期待 SHA 未提供は `null`、提供時に解決 SHA が未捕捉なら `false` とする。`GATE=run` のときのみ `RUVIFORM_REV` の 40-hex を要求し、blocked/not_run の終端経路では生値を「試行された入力」として記録する。gate=blocked の原因が `RUVIFORM_REV` 自身の不正形である経路で finalize が throw すると証拠が消失するためである。env は allowlist のみを読み、秘密系を転写しない（Issue #294）。

workflow 側には 2 つの実装上の制約への対応がある。`runner` context は job env で参照できず（actionlint が `jobs.<job>.env` での `${{ runner.temp }}` 使用を拒否する）、無条件先頭 step が `$GITHUB_ENV` に `CONSUMER_DIR`/`LANES_FILE`/`EVIDENCE_FILE` を供給する。resolution step は `always() && gate==run` とし、clone step 自体の失敗時に GitHub が後続 step を skip しても rev-parse 失敗を fail lane として記録できるようにした（Issue #294）。

## 候補 tuple の検証と証拠の復元

consumer-verify の検証単位は `(ruviform main SHA, audio-lens main SHA, synvelis main SHA, consumer-verify-tuple/vN)` の候補 tuple である。両 verify job の clone は `git init`＋`fetch --depth 1 origin <ref-or-sha>`＋`checkout --detach FETCH_HEAD` の統一経路で branch・tag・40-hex SHA を受理し、`git rev-parse HEAD` で解決 SHA を捕捉して任意入力の期待 SHA と fail-closed で照合する（Issue #294）。SHA fetch には境界があり、GitHub は unadvertised commit の fetch を受理するが、GC 済み・未 push の SHA は clone step fail となり `fail` 証拠として正直に出る。`extra_tasks` は operator 信頼入力で pattern は shell 注入排除のみに留め、job の `timeout-minutes: 120` が暴走境界である（Issue #294）。

daily scheduler の tuple 判定では、Actions run API が dispatch inputs を公開しないという制約から、各 run が検証した tuple は `consumer-evidence-<consumer>-attempt-<run_attempt>` artifact の証拠 JSON が唯一の復元源になる。証拠なし・期限切れ・破損・内部不整合は全て `unverifiable` とし、verified にも無視にもせず auto actor なら retry budget に保守的に計上する。証拠が完全に読めて tuple が candidate と異なる `non-target` と、証拠が欠ける・壊れる `unverifiable` を区別し、verified は success＋全 consumer `status==="pass"`＋tuple 完全一致の三重条件とする（Issue #294）。

artifact zip の解析は依存追加を避けるため最小自前 reader で行い、EOCD→central directory→local header を stdlib で走査して stored/deflate を `node:zlib` で展開する。data descriptor・zip64・method 0/8 以外は unverifiable 側に倒す。artifact zip のダウンロードは 302 `Location` を手動追従し、token を署名 URL へ転送しない（Issue #294）。判定側では consumer 一覧の正本を `candidate.consumers`（解決済み sha を持つ）とする。`{name, repo}` のみの config consumer を参照した初版では一致判定が全件 non-target になる bug があり、fixture テストで検出・修正した（Issue #294）。

contract の version 軸は contract 文字列が担う。v1→v2 への移行は hard cutover とし、producer（evidence.mjs の `CONTRACT`）・consumer（decide.mjs の既定 tupleContract）・両 workflow・contract test を一括で bump した。legacy v1 の証拠は contract 完全一致規則でそのまま non-target に分類されるため、受理集合の導入は不要と判断した。`schemaVersion`/`lanes` は consumer 側で未検査のまま維持する。consumer が必要とするのは tuple 同一性と status であり、証拠内部語彙は producer 側の契約であるという整理である（Issue #720）。

## Case schema v2 と oracle lane の記録形式

証拠の lane record は Case schema v2 で、`SCHEMA_VERSION` を 1→2 に bump しつつ `CONTRACT`・`outcome` 語彙・`deriveStatus`/`coverage`/`markdownSummary`・top-level tuple/run/consumer 構造を不変に保つ additive 拡張である。各 lane record は必須項目として `case_id`・`level`（`OS-L1`/`GUI-L2`/`EXT-L3`）・`environment_kind`（5 値閉集合）・`requirement`（`Required`/`Deferred`/`NotApplicable`、結果軸と直交する別軸）を保持する（Issue #294）。`readLanes` は `lane` コマンド経由でない行にも同一語彙の fail-closed 検証を課し、schema-v1 行・未知 enum・型不正・非 40-hex `resolved_ruviform_sha` を拒否する（Issue #294）。

確定した Case 分類は、既定を `OS-L1|deterministic-backend|Required` とし、GUI visual/OS＋実 window lane は `GUI-L2|real-window|Required`（outcome `unsupported` 固定）、実 plugin binary lane は `EXT-L3|test-plugin|Required`、macOS/実 OS lane は `OS-L1|physical-device|Required`（いずれも `unsupported` 固定）とした。`not_run` 固定行は headless fixture 系の `OS-L1|deterministic-backend|Required` に分類する（Issue #294）。

記録経路は workflow の `rec()` helper に `"<case_id>|<level>|<environment_kind>|<requirement>[|<resolved_sha>[|<oracle>[|<fixture_version>]]]"` の spec 文字列方式を採り、`IFS='|' read` で分解する。field 6/7 省略時は flag を付与しない後方互換を保ち、oracle＋fixture のペア必須を正規表現で pin する。`resolved_ruviform_sha` は GUI candidate 行のみに限定する。run 全体は top-level tuple が既に束縛しており、candidate tree 評価の区別が意味を持つのはこの行のみである（Issue #294）。

consumer oracle lane は、consumer checkout 内の canonical test file を `oracle_probe` step が file 単位で存在確認し、存在すれば `git lfs pull`（clone は `GIT_LFS_SKIP_SMUDGE=1` のため materialize が別途必要）の後に `mise run test <pkg> --test <file>` を file 粒度の別 step で実行する。record 分岐は probe output の 3 分岐（`'1'`→run outcome、`'0'`→owner 付き `not_run`、その他→probe outcome）とし、`--test` 併記の単一実行は coverage 同等でも file 別 verdict が埋没するため別 step・別 lane 行とした。`--expected`/`--observed` は記録しない。consumer test は機械可読の期待値・観測値を出力しないため、捏造せず null のままとする（Issue #294）。`fixture_version` は consumer の Cargo.lock から `ruviform-audio-test-<version>@<RUVIFORM_REV>` を構成して記録し、抽出失敗時は flag を省略して null とする（Issue #294）。

## GUI-CONTRACT lane と候補 tree 評価

GUI-CONTRACT lane は両 verify job に `gui_candidate`/`gui_boundary`/`gui_semantic` の 3 step で構成され、候補 commit tree（`RUVIFORM_REV`）上で boundary guard と Fake driver semantic を実行する。`gui_candidate` は `$RUNNER_TEMP/ruviform-candidate` へ一律 clone して候補 tree を取得し（`working-directory:` は step outputs を参照できないという制約のため）、`rev-parse HEAD` を候補 SHA と照合する。候補 `mise.toml` の `[tasks.<name>]` 存在を grep で probe し、task があれば `mise install --locked rust` と候補 dir での `cargo fetch --locked` に続けて lane を実行し、なければ owner 付き `not_run` とする。dispatch ref の task で代用する偽 pass を構造的に排除する設計である（Issue #294）。`cargo fetch --locked` の追加は実障害由来で、`cargo metadata --offline` が候補 workspace の全 package manifest を要求するのに consumer clone の `cargo fetch` cache が ruviform 全 workspace を覆えず失敗した run の修復である（Issue #294）。

`gui_semantic` は rc 捕捉＋summary 追記＋exit の構造で、driver-lab の case 別 verdict JSONL を「候補 tree で評価した証拠」として Job Summary に残しつつ、失敗時は元の exit code で step fail にする。task 不在の旧候補で `not_run` でなく `fail` になる混入を防ぐため、probe の grep 判定と record 分岐の対応を contract test で固定している（Issue #294）。

## daily scheduler と CI failure tracker の責務境界

日次検証と Issue 追跡は責務を分ける。scheduler（daily-ci.yml＋`ci/daily/decide.mjs`）は CI の dispatch 判定のみを行い Issue を作成せず、Issue 起票・close は tracker（`ci/tracker/tracker.mjs`＋ci-issue-tracker.yml）が `workflow_run` completed を契機に行う（Issue #367・Issue #368）。

daily scheduler は「現在の main SHA に有効な Full CI がなければ ci.yml を `workflow_dispatch` で起動する」判定器である。判定行列は skip-verified・suppress-in-flight・hold-unknown・budget-exhausted・dispatch で、stdlib-only・pure 判定コア＋注入可能 client＋`main()` の構造を採った（Issue #367）。`attemptsOn` は sha＋branch＋event allowlist で厳格に同定し、auto dispatch は `event===workflow_dispatch` かつ triggering_actor が bot のもののみ計上して上限 `DAILY_MAX_AUTO_DISPATCHES`（既定 2）で budget-exhausted とする。dispatch 後は preDispatchIds 差分で確認 poll（confirmed・landed-mismatch・unconfirmed）し、main-moved 再照合を行う。schedule は UTC `"17 21 * * *"`（06:17 JST）で、cron の分を 0 以外にする規則に従う。daily-ci.yml は workflow 全体 `permissions: {}`、単一 job に `contents: read`＋`actions: write` を限定付与し、concurrency で直列化（cancel-in-progress:false）する。`test-ci-daily` の自己検証を判定実行の前に置き、report artifact を upload する（Issue #367）。

tracker evaluator は `workflow_run` 完了ペイロードを scope（repo×workflow×branch×event allowlist、fork 除外）で分類し、GitHub Issue 一覧・job メタ・compare の注入 client 経由で create/update/close-recovered/close-duplicate/noop の action plan を組み立てる。conclusion の分類は `failure`/`timed_out`/`startup_failure` のみを failure とし、`cancelled`/`skipped`/`neutral`/`stale`/`action_required`/未知値は operational として起票対象にしない（Issue #368）。

run の順序付けには実運用由来の修正が 2 点ある。run の一意性は `(run.id, run_attempt)` を単位とする。GitHub の rerun は同じ run.id で run_attempt が増えるため、run.id 単独比較では失敗 run の成功 rerun が stale 扱いされ永久に close 不能になる bug があった。異 run 間の ordering は `updated_at`（レコード最終更新≈完了時刻）を採用し、欠損・同一時刻は `ambiguous` として close・成功判定から外す（Issue #368）。もう 1 点は marker 参照の suppress で、runs 一覧が各 run の最新 attempt のみを返すため、run id 単位の suppress では「失敗 attempt a1 記録済み＋一覧に同 id の成功 attempt a2」で記録済み failure が消え coverage 判定不能になる。marker の `failure` 参照は同一 attempt の実 record がある時だけ実 record に置き換え、`latest` 観測参照は同 id・それ以上の attempt の実 record がある時だけ捨てる `(runId, attempt)` 単位に修正した（Issue #368）。

marker JSON の injection 対策は二重防御とした。state に jobs が含まれる設計上、`-->` を含む job 名で HTML comment を抜け出す経路があったため、job 名を入力境界で sanitize し、marker JSON 内の `<`/`>` を unicode escape する。履歴 record は out/in/unknown に分類し、in・非 completed は `pending` 証拠（live run の完了は未来のため success 昇格・close を許さない）、unknown は close と resolved-suppression（stale failure 起票抑止）の両方を止める（Issue #368）。

ci-issue-tracker.yml は `workflow_run {workflows:[ci], types:[completed], branches:[main]}` のみで起動し、pull_request・push・schedule・workflow_dispatch の trigger を持たない。track job に `contents:read`/`actions:read`/`issues:write` を限定付与し、checkout するのは default branch の自分自身である。`workflow_run` は default branch の workflow ファイルで実行されるため、失敗 run 側のコードや artifact には触れない。self-test（test-ci-tracker）を API 呼出し前に置き、失敗時は GitHub API に一切触れず job が落ちる順序にした（Issue #368）。

## GITHUB_TOKEN の anti-recursion gap と reconcile 経路

GITHUB_TOKEN による dispatch の完了は `workflow_run` を発火させない制約（anti-recursion）がある。daily-ci.yml が GITHUB_TOKEN で consumer-verify を dispatch するため、event 経路だけでは daily dispatch 起因の失敗を取りこぼす。これを埋めるため、event 非依存の `--reconcile` モード（同一判定器が権威ある run history と marker のみを使う）を tracker に追加し、別 workflow の `schedule`＋`workflow_dispatch` 入口から呼ぶ（Issue #368）。reconcile workflow は `if: github.event_name == 'schedule' || github.ref_name == github.event.repository.default_branch` とし、schedule payload では `github.event.repository` が欠落し得るため ref 比較は dispatch 限定とする。checkout は `github.event.repository.default_branch || 'main'` に pin する（Issue #368）。

consumer-verify を tracker に配線する際も同じ境界を使う。`workflow_run` trigger を `workflows: [ci, consumer-verify]` に拡張し、`workflow_run.name` でゲートされた scope 別 job（track/track-consumer-verify）がそれぞれ `--workflow`/`--workflow-file` で scope を明示して同一 evaluator を実行する。1 job 内で両 workflow を処理すると `github.event.workflow_run` がどちらの run か曖昧になるため job を分けた。reconcile は `ci`→`consumer-verify` の順に同一判定器で逐次実行する（Issue #294）。`workflow_run.workflows` は name フィルタのため同名別ファイルの混入余地が残り、`classifyRun` の workflow identity 照合（name/path）を defense-in-depth として scope 外 run の混入を fail-closed で skip する（Issue #243・Issue #294）。

tracker は evidence JSON の verdict を読まず workflow run の conclusion のみを評価する。`not_run`/`unsupported` は consumer-verify が failure conclusion を出さない限り tracker の失敗集合に入らず、success でもない。この前提を壊す verdict 語彙の変更は tracker の分類前提を崩し得るため、contract test で `deriveStatus`/`OUTCOMES` の import と 5 値語彙を pin している。Issue title は scope 由来（`[CI failure][<branch>] <workflow> failed`）とし、`ci` と `consumer-verify` が同一 Issue に混ざらないようにする（Issue #294）。

## CI 計測の契約と観察結果

CI 計測の契約は、指標（wall・queue・job_wait・job_exec・exec_span・critical_path・runner_total）・lane 分類・Cold/Warm・Before/After 手順を固定する。収集ツールは `gh api` read-only・Node 標準のみで、critical path は workflow YAML の `needs` を静的解決する最小パーサで算出し、reusable workflow や動的 needs は `unknown` warn に落とす。API フィールド欠落は `null`＋`unknown` 表記、分類不能は `other` で名列挙、n<10 は統計値を出さず観察値のみ報告する fail-closed 規約である。`--logs-run` は log 取得が rate limit の支配項となるため opt-in とした（Issue #327）。

主要な観察は 2 点ある。1 点目は、3 repo とも critical path が verify/ui-verify の windows-latest job で、wall time≈最長 job のため単一 lane の短縮が wall に直結する構造だったこと（Issue #327）。2 点目は cache 挙動で、mise tool cache は 3 repo 全 job で一貫して miss（job あたり 13–23s の固定コスト）、cargo cache は ruviform/synvelis で hit するが audio-lens は Cargo.lock（ruviform rev）更新に伴う key 変化で頻繁に cold 化し、lockfile 更新頻度が cache 効率の支配因子だったこと（Issue #327）。必須検証の baseline は `gh api` の branch protection・rulesets から required contexts と job 集合を再導出して固定した（Issue #327）。

## コンパイル cache・npm install・重複実行の最適化

sccache によるコンパイル cache は、action を immutable commit SHA で pin し、sccache 自体のバージョンも明示 pin して導入した。`SCCACHE_GHA_*` env は `actions/github-script` 経由で `GITHUB_ENV` へ export する。`ACTIONS_RUNTIME_TOKEN`/`ACTIONS_RESULTS_URL` は JS action コンテキストでのみ確実に参照可能なためである。cache 名前空間は `ruviform-v1-<RUNNER_OS>` で OS ごとに分離し（MSRV lane の別 toolchain とも衝突しない）、kill switch は repo variable `CI_SCCACHE_DISABLE=1` で workflow YAML 変更なしに切れる。sccache は content-addressed key で誤共有を構造排除し、write error・cache 破損時も compile を継続する fail-open 設計のため cache backend は correctness の権威にならない（Issue #329）。実測では verify lane で 47–62% cache hit・job 時間 10–30% 削減を記録した。cold run で 36–100% の cache write error が発生したが warm run では verify 系が 0 で、原因は GHA cache backend の rate limit と並列書込競合（6 job 並列が同時に数百エントリを書き込む）と判断した（Issue #329）。

npm ci の重複実行は、mise の sources/outputs freshness で集約した。`install-ui-*` タスク（7 本、run=npm ci、sources=lockfile＋manifest、outputs=`node_modules/.package-lock.json`）と集約 `setup-ui` を設け、verify 系タスクは `depends` 経由で準備済み node_modules を再利用する。output マーカーに `node_modules/.package-lock.json` を選んだのは、npm ci 成功時に必ず生成され部分 install や削除で stale になるためである。freshness は副作用を持つ準備・emit 系タスクに限定し、check/test 系には付与しない。検証 skip は検証能力の削減に相当するためである。効果は ui-verify job あたり npm ci 20 回→7 回、ui/showcase dist emit 3 回→1 回、warm 再実行の install は全 skip（約 0.5s）だった（Issue #330）。

`check-ci` を `lint-ci` へ統合したのは、`cargo check` と `cargo clippy` が同一の check-mode ユニット集合を評価し、workspace メンバーが `rustc` と `clippy-driver` の compiler identity 差で二重コンパイルされていたためである。clippy-driver は rustc の全診断に clippy lint を加えた superset で、`cfg(clippy)`/`cfg(not(clippy))` の使用が `crates/` 配下にゼロであることを確認したうえで、型エラー・rustc 警告・clippy lint・cfg(test) 型エラー・example/bench 警告・失敗テストの注入実験 8 ケースで検出能力の等価を実証した。`check-ci` タスク自体は clippy-driver と素の rustc の挙動差を切り分けるローカルトリアージ用に残した（Issue #331）。他 lane（rt・icc・ui・l1-real・bench）は feature・OS・host 条件が非等価のため統合対象外で、`cargo test` は codegen と `cfg(test)` で別責務とした（Issue #331）。

Playwright の browser 準備は実テストから分離し、cache key を `playwright-<OS>-<arch>-pw<version>-chromium<revision>`（`@playwright/test` の version と playwright-core の browsers.json 由来 revision）として `restore-keys` を付けない。部分一致で旧 revision の binary を復元すると test が旧 browser で走り得るためである。launch 検証は `playwright screenshot` の実 launch probe で行う。ファイル一覧では破損 binary を検出できないためである。Linux の system deps は cache と別レイヤーで毎 run 実行を維持し、install task には mise freshness を意図的に付与しない（browser の実在と revision 一致は Playwright の revision-aware installer が authoritative に判定する）（Issue #332）。

bench-smoke lane は対象を bench target 全量の 5 crate へ拡張し、bench profile 破損を CI の bench job で恒久的に検出できる状態にした。`--workspace` 化は smoke の責務を超え、bench target を持たない crate の lib test target まで暗黙に広げるため不採用とし、`-p` 列挙追加のみとした。bench target を持たない crate の lib test は smoke 対象外のため、`cargo bench --workspace --no-run` が別途必要である（Issue #646）。

## 選択実行の試行と Full CI への移行

変更影響ベースの選択実行は、`ci/impact/`（stdlib-only）に分類器＋map＋gate を実装し、`cargo metadata` の workspace 逆依存伝播・`rt_` 保持 crate scan・mise task `-p` 引数・`ui/*` DAG・full-fallback 規則で lane 集合を機械導出する方式だった。分類不能・mixed-impact・共通設定変更は lane 推論を試みず Full CI に倒す。部分 lane 集合の誤判定は偽陰性源となるためである。diff は `git fetch --depth=1 origin $BASE` 後の two-dot tree diff で、merge-base 不要で shallow clone と両立する。`edited` type を採用したのは、title 編集での再実行は concurrency group が前 run を cancel するため実害がなく、base retarget 時の stale 期待集合を防ぐ効果の方が大きいためである（Issue #333）。

Shadow Mode では lane job の `if:` を変更せず既存の required contexts を維持したまま、`impact`（分類＋decision artifact）と `ci-gate`（`if: always()` で needs 全結果を判定）を実運転で観測した。`ci-gate` は matrix leg でなく job 粒度の `needs` result を判定する。per-OS 内訳は各 leg 自身の check run が持ち、checks API の権限追加を避けるためである。gate は fail-closed で、expected lane の missing・failure・cancelled・skipped・分類失敗は全て Fail、not-expected lane の実行 failure は偽陰性シグナルとして Fail、skipped を Pass にしない。`CI_IMPACT_DISABLE=1` で expected=ALL に退化する kill switch を持つ（Issue #333）。

選択実行の有効化では、6 lane job に `needs: [impact]`＋`if: always() && needs.impact.outputs.lane_* != 'false'` を配線した。`if:` は impact 失敗・出力欠落時に lane が実行される側に倒す fail-open coverage で、fail-closed 性は gate の classification-error Fail 側に保持する。macOS legs は job 単位の needs では matrix leg を部分的に外せないため、matrix 縮小＋`*-macos` 別 job として物理分離し ci-gate の needs から除外し、`macos-report` job（`if: always()`・non-blocking）が OS 別 status を記録した。ruleset の required contexts は ci-gate 単一へ「追加→報告確認→除去」の 2 段階で無保護期間なく移行した（Issue #346）。

その後、Full CI Stage 1 として選択実行機構を撤去した。impact job・全 `needs:[impact]`・全 lane `if:`・`CI_IMPACT_DISABLE` env を取り除き 11 lane を無条件実行にし、`ci-summary`（needs 11 件・`if: always()`）と `io-structured-verify` lane を新設し、`workflow_dispatch` に `expected_sha` 入力を追加した。required context を産む verify/l1-real-verify/ui-verify の job id・matrix os 配列・`name:` は不変に保った。`consumer-impact-check.yml` は `github.event.pull_request` を読む pull_request 専用設計のため `workflow_dispatch` へ転用できずファイル単位で除去した。同 workflow は PR 本文の Consumer 影響分類表の語彙と `unknown` 使用時の解消先記入を機械検査するものだったが、分類内容の正しさまでは検査しない設計で、除去後は分類要件を PR テンプレートの表＋レビュアーの human gate に残した（required check でないため merge デッドロックは生じない）（Issue #219・Issue #366）。

`ci-summary` は needs 全 job の `result === 'success'` のみを判定する fail-closed 評価器で、failure・cancelled・skipped・未知値・needs key 欠落・`checkout-sha-<leg>-attempt-<n>` artifact の実測証跡欠落・`EXPECTED_SHA` 非空時の不一致・NEEDS_JSON parse 失敗を全て fail とする。leg の実測値は最大 run attempt を採用し、`HEAD_SHA` は summary job 自身の `git rev-parse HEAD` 実測値を使う（Issue #366）。この fail-closed 構造の帰結として、新しい job を lane に追加する際は `needs` への追加だけでは不十分で、`summary.mjs` の `REQUIRED_JOBS`/`REQUIRED_CHECKOUT_LEGS` 宣言と列挙 test の同期更新が必須になる（Issue #594・Issue #598）。

## MSRV lane と macOS 物理分離 job

MSRV 日次 lane は `msrv-verify` job を追加し、4 つの `test-*-msrv` task を rust 1.89.0 toolchain で CI 実行する。mise.toml 側は無変更で、各 task が `mise exec rust@1.89.0` で第二 toolchain を明示選択し target dir も分離済みである。4 task は個別 step 化し各 step に `if: ${{ !cancelled() }}` を付与した。既定の `success()` ゲートでは先行 task 失敗時に後続が skip され、1 件の MSRV 破損が他 crate set の独立破損を不可視にするためである。`always()` でなく `!cancelled()` を選んだのは、cancelled run で証跡だけ残る誤判定を防ぐためで、matrix lane の `fail-fast: false` と同じ意図である（Issue #594）。初回 dispatch では feature-gated test（`--all-features` の MSRV lane でのみ実行され通常 lane では未検出だった）が upstream 依存の birth time ソート起因で flaky 失敗し、seed file の timestamp 間隔を空ける修正で安定化した（Issue #594）。

macOS の GPUI lane は matrix ではなく物理分離 job として追加した。matrix leg は `needs.<job>.result` が 1 値に集約され失敗した OS を判別できず、macOS runner の starvation が他 OS lane を道連れにするのを避けるためでもある（Issue #598・Issue #346）。`build-full` job は接続しなかった。`test-full`（`cargo test --workspace`）が workspace の実行 bin の Build/Link を実質包含し、唯一の空白だった `required-features = ["uitest"]` の bin は `test-ui-gpui` の `--features "gpui-test uitest"` 拡張で塞いだ。feature 組合せを増やすと gpui 本体の再 compile が発生するため gpui-test+uitest の 1 組に統一した（Issue #598）。この新 lane は初回実行で macOS 専用 cfg 内の lint 欠陥を検出した。Linux/Windows lane では一度も lint されていなかった潜在欠陥が fail-closed で表面化した例である（Issue #598）。

## GPUI offscreen lane の環境要件

GPUI の offscreen 検証 lane は Xvfb＋openbox＋lavapipe 上で X11 GetImage による実 pixel readback を行う構成である。必須要件は 3 点ある。openbox は bare Xvfb では present が成立せず黒画面になるため必須とした（根本原因は未確定のまま要件化）。mesa ≥24.2 は 22.3.6 で depth-24 PutImage BadMatch による恒久黒画面があったため要件化した。`--test-threads=1` は `_NET_WM_PID` が process 単位で一意であり、同一 PID の toplevel が並走すると window 解決が ambiguous になるうえ、occlusion で GetImage が下位 window を読めないため必須である（Issue #314）。

window 識別には `_NET_WM_PID` を採用する。gpui 0.2.2 の X11 PlatformWindow は `HasWindowHandle`/`HasDisplayHandle` が `unimplemented!()` で raw-window-handle 経路が panic するため、capture 側は lazy に own toplevel を解決する。候補集合は `_NET_CLIENT_LIST` 優先＋`QueryTree` fallback で、openbox が client window を frame window に reparent するため root-child 列挙では拾えない。0 件・複数件の match は推測せずエラーとする（Issue #314）。`client_geometry` の `GetGeometry` は inside size（border は drawable 外）を返すため `border_width` を引かない（Issue #314）。

却下した代替案も記録されている。bare Xvfb（mesa 25.0.7 でも黒画面）、Mesa 22.3.6（depth BadMatch）、`MESA_VK_WSI_PRESENT_MODE=FIFO`（BadMatch を解決しない）、depth-32 screen 検証・Xvnc・Xephyr・Xorg dummy（いずれも depth-32 visual を隠せず gpui の visual 選択を変えられない）、gpui headless platform（stub であり render path ではない）、真の render-target offscreen readback（gpui 0.2.2 の公開 API に存在しない）が却下対象である（Issue #314）。

依存側では `libc` を 0.2.189 に pin した。xattr 0.2.3 が参照する `libc::ENOATTR` が libc 0.2.190 で Linux cfg から削除され、gpui の Linux build が不能になった upstream 既知問題への lockfile 修正である。pin は lockfile 再生成で失われやすく、実際に後続の lockfile 再生成で 0.2.191 への drift が発生して pin が潰れた事例がある（Issue #314・Issue #125・Issue #582）。

## ベンチマーク基盤の検証契約

ベンチマーク基盤では、等価性契約の確定値を実測に基づき ulp 単位で固定した。dot は ulp≤256（実測 max 46）、fir/biquad は ulp≤8（実測 max 0）、elementwise・再帰一致系の 5 カーネルは `to_bits`＋NaN 一致の bit 完全一致とした。FMA 版 fir は約 3994 ulp の打ち消しドリフトを実測したため非融合 multiply-add を採用している。観測値が上限を超過した場合は事後緩和せず、実装バグか契約不適合として先に調査する方針である（Issue #19）。

状態を持つカーネル（biquad/sine）は criterion の `iter_batched`/`iter_with_setup` で反復ごとに状態を初期化する。denormal 観測側は denormal 状態を維持・再注入し、denormal 値の生成には `f32::from_bits(0x0040_0000)` や `f32::MIN_POSITIVE * 0.5` を使う。素朴な係数乗算は +0.0 に丸まるため使えない（Issue #19）。fir で最大約 31.8 倍の denormal 崖（性能のみ、等価性は維持）を観測したが、FTZ/DAZ は Rust 上で形式的 UB のためカーネル側では対策せず、ホスト側設定またはカーネル内 software flush の API 契約としてエンジン層へ移管した（Issue #19）。

criterion の運用面では、`BenchmarkId` の構成要素中の `/` が `_` に sanitize されるため、出力階層は `<input>_<len>` の 1 段になる。集約側は sanitize 後の結合形を参照する必要がある（Issue #19）。全量計測は時間予算を削減して約 20–30 分に収め、CI の bench job は `bench-smoke`（`cargo bench -- --test`、各ベンチ 1 回）に分離して全量計測はローカル手動実行とした。run 冒頭に OS・CPU 型名・コア数・toolchain 版・dispatch 検出結果を標準出力する環境情報出力も設けた（Issue #19）。

PoC コードは候補クレート型を公開 API に露出させない制約から、`#[doc(hidden)] pub` モジュール＋tests 配下の共有 module の二層構成とした。dev-dependencies 封じ込めでは `pub(crate)` 配置が成立しないためである。dev-dep のバージョンは caret 指定とし、追跡済み `Cargo.lock` を再現性の錨とする（exact pin は採用しない）（Issue #19）。MSRV 検証手順は `rustup run <version> ... --lib --locked` の呼出形に統一した。`cargo +<version>` は rustup proxy 前提となるためである（Issue #19）。

## GitHub Actions の実装上の落とし穴

workflow 記述で確認された落とし穴を列挙する。`url.insteadOf` の rewrite は `.git` suffix 付きでないと SSH rewrite が効かない。`secrets` コンテキストは `if:` で直接使えず、step env 経由で `[ -n "$KEY" ]` を判定する。`inputs.default` は `workflow_dispatch` で非評価となる。`${{ env.RUNNER_TEMP }}` は env context に存在せず、step の `working-directory`/`with:` には `${{ runner.temp }}`、shell 内では `$RUNNER_TEMP` を使う（Issue #219）。`if: always()` の証拠記録 step が clone 未実行時に `working-directory` 不在で失敗しないよう、`working-directory` は gate==run の step のみに限定し結果記録は repo 既定 dir＋`git -C` 参照とした（Issue #219）。`mise-action` は clone 前に配置する。cargo fetch step が `mise exec` を呼ぶためである（Issue #219）。

`workflow_run.workflows` は name フィルタであり同名別ファイルの混入余地がある（Issue #243）。schedule payload では `github.event.repository` が欠落し得るため、default branch 参照は `|| 'main'` fallback で pin する（Issue #368）。後続 step が先行失敗で skip されると独立の破損が不可視になるため、複数検証 step を並べる job では `if: ${{ !cancelled() }}` を付けて全件実行させる（Issue #594）。`workflow_dispatch` の concurrency group で候補 tuple の衝突が起きた例では、expected SHA＋extra tasks を key に追加して分離した（Issue #294）。

## 実運転で検出された不整合と修復

いくつかの欠陥は実 dispatch run で初めて顕在化し、gate が意図どおり機能した記録となっている。ある dispatch run では、`ui-gpui-verify` の冷キャッシュ時 `cargo metadata --offline` 失敗（`cargo fetch --locked` step 追加で解消）、`libc` の pin 喪失（lockfile 再生成で drift）、依存ライブラリ moxcms 0.9.2 の規格準拠 fallback 実装に対し `IccTransform::new` の missing-tag reject を境界の不変条件として再強制、および `cfg!(windows)` により Linux で常に `Unsupported` を返す backend に対し成功を仮定していた test を `#[cfg(target_os = "windows")]` で gate する、という複数系統の修復が行われた（Issue #125・Issue #582）。依存版の pin と適応のどちらを取るかの判断では、規格準拠の実装変更を pin で抑えるより、`IccTransform::new` が所有する境界の不変条件として reject を再強制する方が責務的に正しいとした。pin は lockfile 再生成で失われやすいことが同時期の drift で実証されている（Issue #582）。

consumer-verify の実 dispatch では、verify-audiolens が check-ci・smoke lane を通過したうえで `test-ci` の export test で失敗し、consumer 側の潜在欠陥（fixture の metadata JSON への absolute path 埋め込み）を検出して summary fail となった（Issue #219）。別の実 runner dispatch では、verify-synvelis が全 lane pass し、verify-audiolens は途中まで pass したうえで `check-ci` が struct field の初期化子欠落で fail した。該当 field は ruviform 側に既導入済みで consumer 側の未追随が実不整合として検出されたもので、fail 以降の lane は `skipped` として派生記録された。この修復は consumer 側の別 task に委ね、workflow 側で補償しない方針とした（Issue #294）。

再現困難な失敗の原因確定には、環境を絞った再現が使われた。consumer-verify preflight の `mise install --locked` 失敗では、旧設定の `mise.toml`/`mise.lock` を clean な global config と特定版の mise にかけて同一エラーの再現を確認し、現行設定では複数の install 形態ごとに exit code を記録して切り分けた。原因特定のみを目的とし、設定変更は行わない判断だった（Issue #583）。

## UI テスト基盤と文書生成ゲート

UI テストの control plane は、u32le length prefix＋JSON の frame で transport を構成し、peer の loopback 確認と run ごとに生成する secret による auth を行う。driver 側は stdio/TCP の 2 経路で agent を spawn し、agent 側は session/auth state・capability gate・op dispatch・EOF 時の graceful cleanup・無効化環境変数による Disabled 化を備え、auth/capability/disable の各 gate は fail-closed で動作する（Issue #312）。

文書生成の script には段階 gate があり、計画文書の生成は approved 状態の設計文書を要求し、タスク文書の生成は approved/in-progress/completed の計画文書を要求する（Issue #127）。

## 未確定・保留事項

記録時点で未解決・後続工程へ委譲された事項を列挙する。

- Issue #294 は umbrella であり、各 slice 完了後も Open 継続のままである。close 条件（`Refs` のみで継続か、別 slice で close するか）は未確定。
- consumer-verify の実 runner dispatch（Case schema v2 証拠 artifact の実走確認・daily-ci の tuple 判定）は post-merge の実測待ちで、記録時点では NotRun。
- consumer oracle lane の per-case（テスト関数単位）JSONL emitter と ingest 契約は consumer 側の後続実装待ち。Case schema v2 の `requirement` は現行全 lane `Required` で、`Deferred`/`NotApplicable` の実使用例はない。optional 項目（oracle・fixture_version・expected・observed・device 等）は schema が受理するが `resolved_ruviform_sha` 以外は未記録。証拠契約の利用者向け文書への schema v2 反映は後続の同期作業として残る。
- Issue #127 の main HEAD の必須 CI 確認は daily-ci dispatch 待ちで `NotRun(Pending)`。deb/AppImage/Flatpak の実機検証・distro 決定・形式採用は利用要件発生まで Deferred。
- Issue #219 の `consumer-impact-check.yml` 関連では、spec/ADR status の昇格と required check 化・自動 trigger の判断が未実施。`CROSS_REPO_DEPLOY_KEY` 等 credential の設定は人工作業として未実施。
- 実 window 必須 lane・macOS/実 OS 正式受入・GPUI driver・実 plugin binary lane は引き続き `not_run`/`unsupported` 固定行で、各 Owner 側の実装待ち。
- GT-05 offscreen lane の stock mesa（24.2.x）での実測は CI 初回で確認予定。bare Xvfb で present が成立しない正確な機序（gpui の visibility/refresh か mesa WSI present mode か）は未解明。
- Issue #329 sccache の GHA cache backend rate limit・並列書込競合の恒久対応、Cargo.lock/toolchain 更新時・fork PR の挙動、他 lane（RT・ICC・L1・bench）への展開は未対応。
- Issue #368 の実 workflow_run 発火・実 cron・実 Issue・実 dispatch は offline contract test と actionlint のみで検証されており、merge 後の初回実測が受入となる。
- Issue #583 の `mise install --locked` 失敗の根本原因は未解明で、旧設定での再現確認までが記録されている。
- 選択実行有効化後の After メトリクス（wall/runner_total/lane skip 率）は未実測。Issue #328 の連続 push cancel・別 PR/main push/dispatch の非 cancel の実測、Issue #330 の CI 3 OS matrix 実測、Issue #332 の version bump 時 cold 再計測、Issue #333 の部分集合 expected の実 run 証拠と kill switch 実動作はいずれも未観測。
- 各種判定器（decide.mjs・tracker.mjs・summary.mjs・evidence.mjs）の判定行列・語彙・fail-closed 規約について、実運用での誤判定実績はいずれも未観測。mise freshness の stale 検出精度、cargo cache key の lockfile 依存による cold 化頻度、dev-dep caret 指定＋`Cargo.lock` 錨の再現性も同様に実運用未検証である。
- 文書関係グラフ上の既存の broken relation は本基盤の各 slice で範囲外として未修復のまま残っている（Issue #243）。
- 本書の記述は記録時点の snapshot に基づく。各判定器の実装コードと workflow ファイル本体が記述どおりに維持されているかは別途確認を要し、記録された CI run の現在の状態（success/failure・後続 run の有無）も本書では保証しない。
