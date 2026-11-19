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
  source: ["https://github.com/scarecrow173/ruviform/issues/3", "https://github.com/scarecrow173/ruviform/issues/23", "https://github.com/scarecrow173/ruviform/issues/41", "https://github.com/scarecrow173/ruviform/issues/172", "https://github.com/scarecrow173/ruviform/issues/184", "https://github.com/scarecrow173/ruviform/issues/192", "https://github.com/scarecrow173/ruviform/issues/193", "https://github.com/scarecrow173/ruviform/issues/419", "https://github.com/scarecrow173/ruviform/issues/456", "https://github.com/scarecrow173/ruviform/issues/553", "https://github.com/scarecrow173/ruviform/issues/586", "https://github.com/scarecrow173/ruviform/issues/590", "https://github.com/scarecrow173/ruviform/issues/627", "https://github.com/scarecrow173/ruviform/issues/628", "https://github.com/scarecrow173/ruviform/issues/727", "https://github.com/scarecrow173/ruviform/issues/744"]
  informs: []
  explains: []
  supersedes: []
---

# ドキュメント運用・エージェント基盤・スクリプト・ジョブ実行の記録

この文書は、過去の実装記録から抽出した検証と実行の結果を記録する。抽出元の記録は後に削除されたため、この文書はそれ単独で読める形で残してある。各項目には出典の Issue 番号を付記し、測定値や観測された動作は事実として、それらの解釈は解釈として区別して記す。

## ドキュメントのプレースホルダー除去と CI 実ディスパッチの検証

Issue #3 では、テンプレート由来のプレースホルダーを除去する作業を、実測に基づいて進めた。`git ls-files` が返した 55 ファイルを母集団に 4 系統の grep を実行し、書き換え対象は README.md の 3 行、AGENTS.md の 5 行、apm.yml の 2 行、および mise.toml L8 のコメント（意味的残存）に確定した。apm.lock.yaml の 198 hit はすべて APM パッケージ由来のパスを指す生成物のため除外し、docs/ 配下の hit は履歴記録として除外した。

AGENTS.md の改稿後は、`git grep` で「template/テンプレート」と「派生リポジトリ/派生プロジェクト」がともに 0 hit であること、mise.toml の 12 タスクがすべて `[tasks.*]` に実在すること、`prek run --all-files` が exit 0 であること、ci.yml に LFS 取得がないこと、LICENSE が存在しないことを確認した。doc-status/doc-impl の監査も findings 0 だった。

README の改稿後は、「Use this template」「project.template」「テンプレート|template」の grep がすべて 0 hit であること、設計の節構成・検証済みコマンド・6 区分ラベルの grep 一致、LICENSE の非存在、prek の exit 0、doc 監査の findings 0 を確認した。禁止記載 7 項目（LICENSE 存在、未作成クレートの実在、RT-safe の断定、CI 実ディスパッチ成功の記載、macOS/Linux 検証済みの記載、再利用手順、存在しないリンク）はすべて非該当だった。

Issue #3 の終端スライスでは、V-1〜V-11 の判定基準をすべて満たした。追検証として main で GitHub Actions を 2 回ディスパッチしたところ、初回 run 39249234364 は fresh な Ubuntu runner の rustup minimal profile に rustfmt と clippy がなく、`cargo-fmt is not installed` で失敗した。mise.toml の rust エントリに `components = ["rustfmt","clippy"]` を pin して lockfile を再生成し（PR #7、merge 6aeadf6）、run 39249585284 は約 30 秒で全ステップが green になった。あわせて `df`/`du` の実測で target/ci が 7.1M、ディスク使用率が 84% であることを記録した。この差異は Windows ローカルでは既存インストールに同梱されているため顕在化しなかったクロスプラットフォーム差異であり、手動ディスパッチによる検証設計の妥当性を実証したと記録されている。

## エージェント基盤の採用判断と受入検証

オーディオデバイス backend として cpal を採用し、ADR select-audio-device-backend を proposed から accepted へ昇格した。検証項目は 6 つあり、audio-core の公開 API に cpal が露出しないこと、feature 分離、実機での play_sine による列挙・開始・コールバック到達、assert_no_alloc、第 2 backend を追加可能な trait の形状、mise run 系の動作をすべて証跡に対応づけた。再評価トリガ（duplex 要件の発生、cpal の再停滞、致命的な挙動不良）はいずれも未観測と記録した（Issue #23）。

Issue #41 の修正後は、`cargo test -p ruviform-audio-core` で 32 unit と 20 doctest、`-p ruviform-media-core` で 30 unit と 24 doctest、`cargo test --workspace` で計 141 件がすべて pass した。`cargo clippy --workspace --all-targets -- -D warnings` は 0 warnings、fmt-check も clean だった。

Issue #172 では、`mise run fmt/check/lint/test/build` と `check-ui-webview`/`lint-ui-webview`/`test-ui-webview`（scheme 7 tests）がすべて exit 0 だった。`job_progress_detail_reaches_the_wire` テストで、detail 付き report の伝播、wire 形状（detail キーの有無）、detail なし report の `detail: None` 維持を検証した。sticky detail 方式は「過剰」として却下され、採用実験は行われなかった。

Issue #192 の TA Invariant Map 監査では、umbrella §7 の TA-001〜TA-012 invariant を現行 API、crate、設計判断、実装 Issue、既存 OS-L1 テスト、未達 Headless Gap へ 1 対 1 で対応付けた正本を作成した。cargo metadata の静的読みでは、default で 42 member・98 normal+build edge・26 dev edge・循環 0、all-features で 103 edge・129 全 kind edge・循環 0 を記録した。

同じ Issue #192 の owner remap では、`gh issue view --json state` で参照 16 Issue の state を照合し、記載の OPEN/CLOSED とすべて一致することを確認した。#243/#313 は 2026-11-14 snapshot 時点では OPEN で、現行は CLOSED であり、追記では日付根拠付きで陳腐化を明記した。#628〜631、#294、AL #127、SV #28 はすべて OPEN の未実装 Harness であり、対応表は受入 owner の対応付けのみを記録し、Case の実走や Pass を意味しない（NotRun 維持）と明記した。

Issue #193 では、3 リポジトリ（ruviform/synvelis/audio-lens）の Issue 状態を `gh issue list --state all --json` と `gh issue view --json body` で取得し、計画関連の Open Issue の `### 着手条件（Blocked by）` 節を照合した。照合対象の全 Open Issue（RF 25 件超、SV、AL 各件）で「なし」を確認し、実 OS GUI/Device/Plugin Gate は `統合・受入で関係する Issue（着手は止めない）` 側に分離済みであることを検証した。M0〜M7 実装 Issue、M1 着手 Gate、OS-L1 CI はすべて CLOSED で、新規 Script/Creative Branch 群は並行 Lane として接続する規則を採用した。Snapshot-2026-11-14 時点の census でも、M0〜M7 実装 Issue #184〜#198、M1 着手 Gate #199/#201、IO/OBS/GUI Test/GUI lane 基盤、3OS OS-L1 CI #598 がすべて CLOSED であり、確認した全 Open Issue で着手条件は「なし」だった（Issue #184）。

Issue #419 では、GPUI 最終採用判定 verdict（outcome=CONDITIONAL）を後継 ADR driver-neutral-gui-gpui-driver-webview-first-issue-384（status: proposed 不変）へ逆反映した。状態は OS-L1 Pass、Windows L2 部分実証、Windows L3/Gate D 未達、macOS/Linux L2/L3 Deferred、Gate C 性能/Gate E NotRun であり、#532 DComp topmost=true は open Blocker として維持した。native_surface_slot/ime/focus release は宣言値として記録した。

Issue #456 では、受入テストの感度確認として、終端 receipt の生成を一時的に外すと runtime・Fake・GPUI の新規テストがすべて失敗することを確認してから元に戻した。`git merge origin/main` 後に検証を再実行し、`selection-command.json` の競合は RF#451 の tabs 節と結合して解消した。

Issue #627 では、review-protocol で spec の陳腐化した「承認前」表現、environment_kind の適用範囲、resolved_ruviform_sha の扱いを修正し、blocker なしから approved への遷移を確認した。doc-driven-dev-graph router は repo 全体の既存 broken relation で blocked を返したが、本 slice 範囲外の既存状態として記録し、index 整合は doc-maintenance で個別に担保した。prek run --all-files と git scope の確認も実施した。

Issue #727 の docs-only slice では、doc_status lint の baseline 採取済みで、新規文書由来の finding は index 未同期由来の `index-missing-entry`/`orphan-index`/`orphan-navigation` warning のみだった。`traceability-missing-upstream` と `inconsistent-reciprocal-relation` は issue-721/716 と同型の許容 warning と記録した。route_graph は新規文書追加後も同等の blocked で finding 増加なし、`mise run fmt-check`、`prek run --files`、adr-doc scripts は Pass だった。cargo 系 lane、`*-msrv`、`*-full`、3 OS は docs-only のため POLICIES.md §5 で NotRun とした。

## CLI・実行基盤の棚卸しと additive 変更の検証

Issue #553 の棚卸しでは、ruviform-exec は CancellationToken/Progress/OfflineDriver/OfflineJob（JobState{Running,Cancelling,Finished}、JobOutcome、JobEvent）を提供するが、Plan/Step/Run journal/Resume/Idempotency/CLI Envelope は存在しないことを確認した。ruviform-cli-sample は利用例であり CLI framework ではなく、#155 方針の「新設 CLI フレームワークは作らない」「sample は例・API に昇格しない」と一致した。Synvelis では Gateway が Capability/Policy/Grant/Approval/CommitGate/Project/Job の唯一正本、AUTX-05 が WorkflowDefinition/PlanCompiler、AUTX-06 が Coordinator/Run journal/Resume/Cancel、AUTX-03 が synvelis-cli/CliEnvelope を Owner としている。

Issue #744 では、`mise run check/lint/test ruviform-exec` がすべて Pass（unit 39 + doctest 9）、`fmt-check` と `prek run --files` も Pass だった。`JobState`/`JobError` の additive 変更について、dependent 4 crate（ruviform-audio/ruviform-cli-sample/ruviform-ui/ruviform-viz）の `check` を実測し、すべて Pass で下流互換を確認した。clippy pedantic 対応として、`needless_pass_by_value` は `run_job`/`worker_loop` を参照渡しに変更し、`similar_names` は `submitter`/`submitted` を `submit_thread` に rename し、`redundant_closure` は `|| task()` を `AssertUnwindSafe(task)` に修正した。`dropping_a_handle_requests_cancellation` テストは drop が queued 状態に着地すると body skip（正しい Cancelled semantics）で flake したため、body 開始を `poll_until` で待ってから drop するよう決定的に制御した。

Issue #628 の deterministic core 1st slice では、`mise run fmt-check/check/lint/test/test-rt` がすべて pass（37 tests + 3 doctests）した。rt_ prefix テストが assert_no_alloc（#[global_allocator] 付き）で clock read・PCM fill・run_callback+sink 経路の allocation-free を実測し、`mise run test-rt` で実際に収集・実行されたことを出力で確認した。run metadata は environment_kind=deterministic-backend、level=OS-L1、seed/fixture_version/clock を保持する。

同じ Issue #628 の pipeline wiring では、`mise run fmt-check`/`check`/`lint`/`test ruviform-audio-test`/`test-rt`/`prek run --all-files` がすべて pass（Windows 11 x64、rustc 1.99.1 mise pin）した。新規 `rt_pipeline_*` 3 test が `mise run test-rt`（workspace 全体 `rt_` filter）で収集・実行されることを出力で確認した。scripted backend では、steady-state pump（fault 無し・`record_output: false`）で allocation 0 を assert_no_alloc で確認した。`collect_faults` の `Vec::new()` は push 時のみ alloc のため steady state では不発であり、caller-owned scratch 化は将来の最適化余地として残された。

Issue #628 の観測限界として、Session::open は no-op error callback を install するため、production callback 内の DeviceError 値は session surface から観測不能である（PumpReport の errors_fired/invalidated count のみ生存）という finding を記録し、製品側は変更しなかった。InputPump は production InputTx::pump の mirror であり、production 側アルゴリズムが将来変わると drift し得る（現行 source との一致は test assertion 済み）。deterministic-backend の Pass は OS-L1 証拠のみで、実 OS API・実デバイス・実 thread timing・duplex・latency は範囲外（EXT-L3/#629 領域）と記録した。

Issue #628 の close-out では、test 117 tests + 3 doctests pass、test-rt で rt_contract.rs 9 件非回帰、製品 crate の src 差分なし（git diff で確認）を確認した。clippy::unnecessary_wraps 2 件を cancelled_pump_* 戻り値 PumpReport 直返しで解消し、serde/serde_json は workspace 既存 pin 範囲（serde_json 1.0.154）で lockfile version 変動なしを確認した。docs/tasks 前 3 slice を done へ同期し、evidence/rf628/ に Issue 全 AC × verdict 棚卸し表を作成した。

Issue #586 の audit では、audit_impl_record.js で invalid-front-matter が 104→0、index-not-managed warning が 1→0 に解消した一方、missing-section が 83→230 に増加した。これは front matter 適合化で新たに検査対象となった既存文書の事前存在する不足分が可視化されたもので、後続 Issue #590 で本文補完して解消した。Issue #590 の補完後は missing-section が 230→0、broken-relation-link 5/missing-experiment-link 6 は baseline 同数で非増加、50 ファイル全件で front matter が HEAD と byte 一致することを script で確認した。既存負債として、bare prose の Related Experiments、HTML コメントのみの Adopted/Rejected、未コミット .log 参照（issue-156）は別 Issue 候補として記録された。

ViewLink Task 1 では、恒久テストが RED 8 fail から GREEN 87/0 になった。`mise run check-ui-binding`/`test-ui-binding`/`build-ui-binding` が green、`check-ui-guitest`/`test-ui-guitest` は oracle 変更なしで 135/0 成立を確認した。独立 review-protocol は Reviewer A/B → Validator → Judge で A1 P1 blocking を検出し、修正後に ACCEPT WITH NON-BLOCKING NOTES で通過した。自 branch push、日本語 PR、候補 SHA manual Full CI 結果報告まで実施した。

## 記録の範囲と限界

この文書が記録する検証には、実行されなかった項目と後回しにされた項目が含まれる。Issue #419 では macOS/Linux L2/L3 が Deferred、Gate C 性能と Gate E が NotRun と記録された。Issue #628 の deterministic-backend 検証は OS-L1 証拠に限定され、実 OS API・実デバイス・実 thread timing・duplex・latency は範囲外（EXT-L3/#629 領域）とされた。Issue #744 では `*-msrv`/`*-full`/3 OS/実機が POLICIES.md §5 と Issue の「実機/native gate: 不要」で NotRun とされた。Issue #727 の docs-only slice では cargo 系 lane、`*-msrv`、`*-full`、3 OS が POLICIES.md §5 で NotRun とされた。Issue #192/#193 の remap 後の Required Case は Harness 未構築のため NotRun のままと明記された。

また、Issue #628 の観測限界として、Session::open の no-op error callback により production callback 内の DeviceError 値は session surface から観測不能であること、InputPump が production InputTx::pump の mirror であり将来 drift し得ることが記録された。Issue #744 の consumer 配線（synvelis#518 の RenderExecutor port への注入）は範囲外と明記されており、完了状況は不明である。Issue #727 の「JobState queued 表現」は実装 Issue（#744）で Queued=3 として確定済みだが、worker naming/priority・queue 具象型の最終確定は design doc 側の Deferred 記述に依存する。
