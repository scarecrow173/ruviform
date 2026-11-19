---
id: "KNOW-Srs0KJ5zXoKjcryv3hrk8m"
type: "knowledge"
status: "draft"
lifecycle: "draft"
knowledge_kind: "operation"
title: "ドキュメント運用・エージェント作業基盤・Issue 管理"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/1", "https://github.com/scarecrow173/ruviform/issues/2", "https://github.com/scarecrow173/ruviform/issues/3", "https://github.com/scarecrow173/ruviform/issues/19", "https://github.com/scarecrow173/ruviform/issues/23", "https://github.com/scarecrow173/ruviform/issues/41", "https://github.com/scarecrow173/ruviform/issues/138", "https://github.com/scarecrow173/ruviform/issues/192", "https://github.com/scarecrow173/ruviform/issues/193", "https://github.com/scarecrow173/ruviform/issues/276", "https://github.com/scarecrow173/ruviform/issues/312", "https://github.com/scarecrow173/ruviform/issues/384", "https://github.com/scarecrow173/ruviform/issues/385", "https://github.com/scarecrow173/ruviform/issues/419", "https://github.com/scarecrow173/ruviform/issues/553", "https://github.com/scarecrow173/ruviform/issues/586", "https://github.com/scarecrow173/ruviform/issues/590", "https://github.com/scarecrow173/ruviform/issues/627", "https://github.com/scarecrow173/ruviform/issues/628"]
  informs: []
  explains: []
  supersedes: []
---

# ドキュメント運用・エージェント作業基盤・Issue 管理

この文書は、リポジトリの文書運用・エージェント作業基盤・Issue 管理に関する運用知識をまとめたものである。各記述の出典は括弧内の Issue 番号で示す。測定・観測された事実はその旨を明記し、設計判断や解釈を含む記述は方針・判断として記して区別する。

## 文書の表現規約：検証済み記述と未確定事項の分離

README・AGENTS.md などの利用者向け文書では、事実記述と構想・未確定事項を厳密に分離する運用が確立されている。記載できる事実は作業の一次記録で実機検証済みのものに限定し、「検証済み・構想・検討中・未確定・未検証・仮置き」の区分をラベル明示するルールを設計段階で定義した (Issue #2/#3)。実機検証は Windows 11 x64 の Git Bash 環境のみで実施されたため、macOS/Linux は未検証と明記し、「仮置き」の注記は該当する全箇所に適用する。README の Status 節は「実在・検証済み／構想（未実装）／検討中／未確定」の4区分に整理された (Issue #3)。

禁止記載は項目として列挙し、レビューと自己監査で担保する。未作成のクレートや API を実在するものとして扱う記述、検証していない事項（RT-safe 断定、CI 実ディスパッチ成功など）の断定的表現、決定済みでないライセンスの表現、存在しない文書へのリンクが禁止対象として排除された (Issue #2/#3)。

Quickstart に記載できるのは一次記録で実機検証済みのコマンドのみである。タスク定義に存在してもソースを書き換える破壊的タスクは一次記録の列挙外として意図的に除外した。また文書内の語彙制約（catch-all grep の対象語）への適応として、説明文では特定の語を避けて言い換える編集も行われた (Issue #3)。

## テンプレート由来文書の固有化とプレースホルダースイープ

汎用プロジェクトテンプレート由来の識別子をリポジトリ固有のものへ置き換える際は、まず書き換え対象集合を「docs/ 配下でも生成物でもない追跡ファイル」と定義した。docs/ 配下の spec・ADR・design・plan・task・実装記録は履歴記録、APM の lockfile 内のパッケージ由来パスは生成物として全件除外し、比較基準は非 docs/ 集合とする。除外は書き換え対象の定義であって記録対象からの除外ではなく、docs/ 側 hit の増加は回帰ではないと明記した (Issue #3)。

プレースホルダー残存の計測は、catch-all パターンの grep を docs/ 除外版（書き換え対象集合の確定用）と docs/ 込み版（記録完全性の独立集計）の2回実行する。`git grep -in "project.template"` の正規表現のドットは任意1文字であり、ハイフン区切りと空白・大文字の両表記に一致することを実測で確認した (Issue #3)。

リテラルパターンに一致しない意味的残存は、grep ベースのスイープでは検出不能である。設定ファイル内のテンプレート前提コメントがその例であり、個別指名で捕捉し、その確認は個別検証項目に依存するリスクとして明記した (Issue #3)。

最終スイープは、事前に記録したベースラインとの hit 数比較で判定する。書き換え対象リテラルの残存が全件 0 になったこと、および除外パターン（履歴記録と生成物パス）以外の残存が 0 であることを除外フィルタ付き grep で実測してから完了とした (Issue #3)。

## AGENTS.md の改稿・縮小・暫定規約の確定契約化

AGENTS.md の改稿は節単位の keep/rewrite/add で行い、開発原則を述べる節は verbatim 保持を原則とする。ワークスペース構成や依存方針などの未確定事項は「暫定原則（詳細契約は別 Issue で確定）」と明記して創作を避け、新規節は前提となる記述（タスク正本の定義など）の後に挿入する (Issue #3)。

暫定規約の内容は、unsafe 原則禁止と SAFETY コメント付き局所許可、default feature の最小化、依存追加時の理由記録、workspace.lints を正本とする `-D warnings` ゲート、公開 API 例の doctest 化であった (Issue #3)。

暫定原則は、アーキテクチャ設計ベースラインの確定後に確定契約へ差し替えられた。確定契約の内容は、RT/オフライン分離・prepare/process 分離・RT パスでの alloc/dealloc 禁止・保証範囲は API 単位・未確定 API を RT-safe と断定しない・CI 成功は完全証明ではない・OS のハード RT 保証なし、である。設計文書が逐語確定していないため、執筆は引用元節の範囲に限定し、陳腐化した「後続タスクで確定」参照の残存は git grep で確認する検証を定義した (Issue #1)。

過剰に縮小された版への対応としては、縮小前のコミット時点へ復元し、検証入口の節のみを参照行へ置換して詳細を別文書へ verbatim 移動し、AGENTS.md を16KB前後に収めた。復元は `git show <commit>^:AGENTS.md` を正とし、セクション境界は見出しで機械的に特定し、diff を旧版と比較してレビューする手法を採った (PR #350 へのフォローアップ作業)。

## 開発ツールチェーンと lockfile の運用

`mise install --locked` は、global mise config がリポジトリ lockfile 外のツールを要求する環境で部分的に exit 1 になり得る。実測ではリポジトリ管理ツールは全件解決済みであり、失敗はリポジトリ外ツールの要求に起因した。判定基準は「リポジトリ管理ツールの全解決」をもって pass とし caveat を記録する運用とし、フォールバックの `mise install` を併記した (Issue #3)。

`apm.lock.yaml` は依存パッケージの pin のみを記録し、manifest の name/description を保持しない。このため apm.yml の manifest 変更は lockfile に波及せず、main ブランチ比で無差分となる。manifest と lockfile の整合は `apm install --frozen` の exit 0 で担保する (Issue #3)。

lockfile の復元は `mise install --locked` を正本とし、各 lockfile（mise.lock・apm.lock.yaml・Cargo.lock）の再生成トリガと復旧手順を表に集約する。追跡方針の記述は Git 運用の節に一元化し、既に実在する生成物の作成手順は重複記載しない (Issue #3)。

## 実装記録の文書チェーン整合と status 遷移規則

実装記録 (IR: implementation record) を作成する際は、文書間の参照整合を双方向で取る。spec/design 側の `derived-by` には incoming の `derives-from` 全件を記載する reciprocal 整合が必要であり、ADR/plan の編集では ADR 側に plan へのバックリンクを追加し、plan 側は `derives-from` に ADR を追加して `related` から除去する (Issue #3/#19)。

実装記録の作成に伴う status 遷移規則は、spec→implemented・approach ADR→accepted・plan→completed・test-spec→approved・discovery→resolved であり、これに伴い各 README の生成索引を再生成する。生成索引は手編集せず既定の索引更新手順で再生成し、最終検証スイートはオーケストレータに委譲する分担とした (Issue #19)。

task 文書と IR の役割分担として、task 文書は計画とチェックリストを保持し、検証結果の実測値は対応する IR に記録される構造が観測されている。task 文書自体に検証数値はなく、一次記録は IR 側である (Issue #276/#312 等の記録群で観測)。

## 実装記録の索引生成管理と監査駆動の補完

実験記録（exp）の管理 index（生成領域）は `*.jsonl` のみを対象とし、`.md` 自由形式の実験記録は hand-curated の「Reports and raw data」節（生成領域外）に列挙する慣行が複数の作業で踏襲されている (Issue #192/#419/#553)。生成 index の適用は生成領域を byte-for-byte で保持し、curated 節への1行追加は慣行どおり手動で実施する。

exp 配下の `data/` サブディレクトリは、計測点ごとの raw per-run JSONL や CI 計測 rows を、flat な `*.jsonl` event-log の監査範囲外に分離するために設けられたものである。索引生成やイベント監査の対象はディレクトリ直下の `*.jsonl` に限定される。

ir 文書群の索引を生成管理下に置く作業では、対象30件の front matter をスキーマ `implementationRecordSchema` に適合させた。形式のみの修正で意味は変えず、type と status の語彙を正規化し、`relations.changes` の文字列を `{type, path}` オブジェクト化（ディレクトリは `type: "directory"`）した。front matter を持たない記録には新規に追加し、id は `IMPL-<22 Base62>` で採番、created/updated は git 初回コミット日、owners はコミット author とした。両 README に generated-index marker と managed region を追加して index apply を可能にし、index apply は `--expect <planHash>` 付きで実行する (Issue #586)。

監査が報告する missing-section（50ファイル・230件）に対しては、canonical 必須セクション（Summary/Context/Implementation/Related Experiments/Validation/Risks/Follow-ups）の本文を各文書の実態に沿って補完した。既存の非 canonical 見出しは内容を失わないよう rename・格下げで canonical セクションへ再配置し、front matter とファイル名は不変とした。実態が読み取れない箇所は、文書・リンク先・Issue/PR 記録に根拠を限定した簡潔記述に留め、推測による記述を禁止した。ファイル単位で独立の作業のため複数系統の並列で実施した (Issue #590)。

機械的正規化は既存負債を可視化する副作用を持つ。front matter 適合後に missing-section が残ること、broken-relation-link や missing-experiment-link が作業前から存在する既存警告であることを Risks/Follow-ups に記録し、機械修正の範囲（形式のみ・意味を変えない）とスコープ外を分離して別 Issue の対象とした (Issue #586/#590)。

## 文書チェーンの権威分離と版付きスナップショット

複数の実装 Issue を束ねる大規模作業では、umbrella 設計文書（設計権威）・実行 Issue 本文（実行権威）・tracking canonical（状態を追跡する正本文書）の3層で権威を分離する構造を採用した。設計合意と実装・検証は分離し、未実施項目は not run/NotRun/Unsupported として記録する。これは解消すべき gap ではなく意図的な分離である (Issue #192)。承認済みの設計権威文書の本文改訂は別 Issue の判断とし、未 merge の並行文書は参照せず、既存の broken relation は範囲外として分離報告する (Issue #192/#193)。

参照先の Issue 状態は変動するため、記録には snapshot 日付と取得方法を明記し、対応表の役割記述が古くなる可能性をリスクとして明記する。版付き Snapshot 節は基準日識別子 `Snapshot-YYYY-MM-DD` を版とし、Issue 状態を `gh` で実測した日を使う。既存本文は履歴 snapshot として保持し、更新は dated 節の追加で表現する append-only 方式を採る。旧 Snapshot の直接書き換えは「両文書の内容を一致させる」約束と履歴保持の原則に反するため棄却し、旧 owner 記述は dated 上書き宣言で残存扱いとした (Issue #193)。

2つの canonical 文書（設計正本と計画正本）に同名節を追記する場合はバイト一致が必須であり、設計正本側を詳細正本・計画正本側を同内容の実行記録として相互参照する規則とした。相対 markdown リンクはファイル間で解決が変わるため使わず、Issue 参照は GitHub URL、文書参照はリポジトリ相対パスの inline code で記述する (Issue #193)。

## Issue 状態の census と受入 owner の記録

Issue 状態の census（全件調査）は、`gh issue list`/`gh issue view` で本リポジトリと連携する Synvelis・AudioLens の3リポジトリの Issue 状態を取得し、計画関連 Open Issue の「着手条件 (Blocked by)」節を照合して、証拠をリポジトリ内の証跡ディレクトリに固定ファイルとして保存する。取得した Issue 状態と umbrella 正本の記録との差分（調査間で closed 化した Issue など）も記録に残す (Issue #192/#193)。

着手条件の記述規則として、Blocked by は全 Milestone を「なし」とし、実 OS・実機 Gate は Integration Gate・独立 Owner Issue・Deferred-Activation にのみ記述する。並行 Lane（Script・Creative Branch・Future capability）は Milestone の Blocked by にしない (Issue #193)。

状態語彙は軸を分離する。要求状態（Required/Deferred/NotApplicable）と実行結果（Pass/Fail/NotRun/Unsupported）は別軸であり、Issue の open/closed は実装の Blocked by と同義ではない (Issue #193)。

要求レベルの適用規則として、OS-L1（3OS の Native Build/Link・Headless/RT/CLI・必要 Feature・Consumer 独立 Oracle）は Required、GUI-L2/EXT-L3 は Windows で実際に提供・利用する機能のみ Required とし、macOS/Linux は利用要件が発生するまで Deferred、物理機材・市販 Plugin の一括 Required 化は課さない。Headless 実装の着手を L2/L3 の NotRun で止めず、製品提供の最終受入では Fail/NotRun を維持する段階分離を採る (Issue #193)。

受入 owner の remap では、受入 owner（Harness 実装群・各製品側 Issue）と実装・mechanism owner を列で分離し、dated addendum を append-only で tracking canonical と対応表へ追記する方式で記録する。Required→Deferred の格下げ、NotRun→Pass の置換、新規 Pass の付与は禁止する (Issue #192/#193)。

## 依存関係監査の手法と証拠の等級分離

`cargo metadata` の `dep_kinds` 解釈では、`kind: null` は normal dependency を意味する（serde 表現）。これを non-normal と誤ると normal edge を0と誤報告し、実際に初回スクリプトで発生した。kind 別に分類し直すことで、default と all-features のそれぞれで normal+build edge・dev edge・循環 0 の集計を得た (Issue #192)。

dev-dep による下位クレートへの上位参照は、設計正本が「既存テスト用 dev-dependencies の上位参照を同じ意味の循環と扱わない」と規定するため循環と扱わない。誤認を避けるため、normal+build を本番 DAG・dev を別集計とする kind 別の報告形式を採用し、dev 込みの全 kind でも循環 0 であることを確認した (Issue #192)。

manifest 宣言 edge（`packages[].dependencies`）と resolver 活性 edge（`resolve.nodes[].deps[].dep_kinds`）は区別する。両者の差分は feature ゲート optional edge の活性化で説明できるため、default と all-features の2集合で報告し、all-features でのみ活性化する workspace 間 edge を個別に特定した (Issue #192)。

監査正本（不変条件と証拠の対応表）では、各不変条件を「現行 API（代表シンボルに限定）・crate・設計判断・実装 Issue・既存の OS-L1 テスト・未達の Headless Gap」へ1対1で対応付け、実装詳細は各正本への参照に委ねる記録粒度とした (Issue #192)。

対応表の記録では証拠の等級を分離する。実測・棚卸し・既存テスト記録・未実行の等級を分けて記録し、grep 棚卸しに基づく型境界の記述は review 責務の補完であって機械的網羅保証ではないと明記する。集計は補助スクリプトで実施し、canonical な検証入口は mise タスクであると分離して記録した (Issue #192)。

監査手法の選択では、`cargo tree`・CI 実走証拠の収集・crate 全読みによる詳細調査は代替案として棄却し、`cargo metadata` ベースの dep_kinds 解析を採用した (Issue #192)。docs-only の変更では検証の比例原則に従い cargo test/build を省略し、フォーマットチェック・lint・文書監査で検証する。文書監査の増分が非 blocking の警告のみで、blocking 件数が全て既存の範囲外 finding であることを確認してから完了とした (Issue #192)。

## 文書生成スクリプトと環境差異

文書生成スクリプトはリポジトリ root をカレントディレクトリとして実行し、`--dir` で出力先を明示する。スクリプト配置パスから実行すると cwd 依存でスキル配下のディレクトリに誤生成される。生成物の front matter はスクリプト出力を正とし、relations/status は手動で整合させる運用とした (Issue #192)。

Windows Git Bash の `/tmp` と Windows 版 Python の `/tmp` 解決は異なり、Git Bash で `/tmp` へ書いた JSON を Windows Python で読むと FileNotFoundError になる。以降の集計は OS の一時ディレクトリ配下の実パスを Python に渡す方式へ変更して成功させた (Issue #192)。

## ADR の昇格判定と verdict の転記規則

ADR の status 昇格は、設計文書の「確認項目↔証跡対応表」に従い、各確認項目を実験ログ・テスト・静的 grep の証跡へマッピングしてから行う。未観測項目は ADR の Confirmation に引き継ぎとして明記し、昇格を阻害しない判断根拠を残す (Issue #23)。

決裁済みの verdict を後継 ADR へ逆反映する際は、「推奨」と「決裁」を分離して転記する。判定資料が verdict を「推奨」と記すのに対し outcome が決裁済みという状況では、「判定の決裁は Coordinator が行い、本欄はその結果の記録である」と明記した上で outcome を記録し、推奨→決裁の読み替えを禁止事項として spec・design・test-spec で固定した。記録側が採否を再判定したり ADR status を変更したりする余地をなくすための規則である。判定語彙（NotRun/Fail/Unsupported/Deferred）を Pass にしない誠実性規則も固定し、宣言した Capability と実測 Pass を混同しない (Issue #419)。

逆反映欄には、outcome・判定根拠・評価範囲の限定（例: 「L1 のみ評価済み、L2/L3 は非保証」）・標準 Driver 方針の維持・OS 別 Capability 宣言記録・既知制約と将来 Owner・証拠リンクを日付付きで記録する (Issue #419)。

Capability Matrix の宣言テンプレート表は、宣言と実測の混同を避けるため原則不変とし、宣言記録は逆反映欄へ集約する。個別 CaseID は判定マトリクスを正本として参照させ、ADR には詳細を複写しない (Issue #419)。

## driver-neutral GUI 契約と theme crate の境界

driver-neutral GUI contract の文書チェーンと、テーマ定義を解釈するクレート `ruviform-ui-theme` を実装した。同クレートは serde/serde_json のみに依存し、解決済みテーマ型に GPUI/toolkit の型を含まず、deterministic な正規化出力を持つ (Issue #384/#385)。置き換えられた旧2 ADR は superseded とし双方向リンクで履歴を保持し、GPUI 最終承認は実機証拠の取得後に ADR 逆反映欄へ記録する分離とした (Issue #384/#385)。

## メディア基盤 API の契約設計と不変条件

`AudioBuffer` の形状検証（`channels * frames` の usize オーバーフローと `isize::MAX` バイト上限）は private helper `checked_slab_len` に集約し、`try_new`（Result 返却・panic しない）・`new`（panic 契約を維持し `try_new` に委譲）・`from_interleaved`（上限超過を従来の panic から Err に改善）で共用する。`new` と `try_new` の doc を相互参照させ、panic を避けたい呼び出しを `try_new` へ誘導する設計とした (Issue #41)。

`ChannelLayout::Custom` の空 mask 不変条件は、doc 注記ではなく型で閉じる方針とし、非空 mask の newtype `SpeakerMaskSet` を導入して payload を差し替えた。`SpeakerMask` 自体は空 mask を合法のまま変更せず、不変条件を課すのは Custom payload 側のみである。`custom()` は `const fn` のため `?` が使えず、`SpeakerMaskSet::new` への委譲を match で実装し、非空ルールの実装を1箇所に保ちつつ `const fn` シグネチャと Err 契約を維持した (Issue #41)。

時刻変換の丸め誤差は「1呼び出し最大1回」として doc のみで方針を明記した。変換連鎖は誤差が蓄積するため、安定アンカーから display/IO 境界で変換する。module doc が方針をカバーする関数は、最小変更を守るため意図的に未変更とした (Issue #41)。

## ログ検証の決定性確保と制御ポートの fail-closed 設計

file sink の試験は、主に `RotationPeriod::Never`（ファイル名固定）の seeded files で構築時 prune を検証する。実 rotation の時間経過は flaky になるため検証範囲外とし、global subscriber 未設定の検証は「`with_default` 無しで emit しても file に書かれない」で代替した。stdout 非出力は、`SinkDestination` に stdout 経路が型上存在しない構造的保証と、example を stderr リダイレクトして stdout 0 byte を確認する実測の二重で担保した (Issue #276)。実 rotation の日跨ぎ・readonly-dir/disk-full・非 Windows OS は未検証として CI の3-OS matrix 導入 Issue へ引き渡し、外部依存の rotation 時ファイル作成失敗が upstream 側で標準エラー出力＋旧ファイル継続となり監視 counter に到達しない経路は、外部依存の振る舞いの限界として契約文書に明記した (Issue #138)。

GUI 制御ポートは、env opt-in なしには serve しない fail-closed 構造とした（env 未設定・off・不正値は Off/拒否）。`TestIo` をアプリ内制御橋とし、エージェント側は既存の `WindowEvent`/`WindowHandle` のみを使用して本番 API を変更しない。journal seq は1起点で `entries_since` が `seq > since` と整合し、`truncated` は `since + 1 < floor` で判定して未溢れ journal で誤判定しない。oversized frame は宣言長分を drain して desync を防ぐ。Windows で accept した socket は listener の nonblocking を継承して `WouldBlock`(10035) になるため、accept 直後に `set_nonblocking(false)` を実施する (Issue #312)。

能力宣言の誠実性として、`DeniedCapabilities` を `#[serde(flatten)]` で wire に常時宣言した（トグルではなく構造的拒否）。headless 環境の screenshot は `unavailable` を返す Unsupported 宣言、`ipc_disconnect` は inbound Message の抑制のみ、`ShutdownMode::Now` は headless では teardown skip の契約として検証し、実 window・実 OS 経路は NotRun として後続の検証 Issue へ明示的に引き渡した (Issue #312)。

## Consumer 中立 orchestration 契約の採否と境界規則

本基盤を利用する下流製品 (Consumer) に中立な Workflow/CLI orchestration 契約では、共通 Engine の今実装・No-go の両案を却下し、契約語彙のみ文書化して実装を Deferred とする案を採用した。根拠は3点である。Synvelis 側の要件（AUTX-05/06）が未確定のため generic を固定すると手戻りリスクがあること、AudioLens は既存の stage-job モデルで要件を充足し分岐・非同期 Job 待機・外部 I/O 混在・Condition を持たないため consumer-only 継続と判定されたこと、「1 Consumer の成功は共通採用を意味しない」ため各 Consumer の独立した必要性を再評価 Trigger としたことである (Issue #553)。

契約の境界規則として、Consumer 固有型（Synvelis の内部型・GPUI 型・AudioLens 製品型・独自 Grant Policy）を generic 契約へ流入させること、および Capability/Policy/Project/Job/Approval の第二正本を作ることを禁止した。他 repo の orchestration 契約からは具象型を流入させず「形」の語彙のみを固定し、片側の成功で共通化を採用することを防ぐ (Issue #553)。

別系統の Script Runtime/Analyzer 契約との重複も明示的に分離した。本契約の Plan は Script lowering の出力先であり、VM を orchestration に含めない。実装 crate 配置と公開 API の確定は、再評価 Trigger 成立後の別 Issue とした (Issue #553)。

既存 API の棚卸しは、`pub` アイテムの grep 抽出と設計正本の結論との照合で行う。公開 API が Plan/Step/Run journal/Resume/Idempotency/CLI Envelope を提供しないことを確認し、Job/Cancel primitive は Reuse、cli-sample は Consumer-only と判定した (Issue #553)。

## 仮想優先検証契約と物理ゲートの移管

検証・受入方針の正本 (POLICIES.md) の仮想優先改訂について、3リポジトリの改訂が作業中に全て main merge 済みと判明したため、deliverable を「改訂案」から「着地済み改訂の照合記録と残件整理」に転換した。ポリシー本文・本番コード・CI は非変更のまま、baseline SHA を証拠に固定して行単位 diff を記録する docs-only の作業とした (Issue #627)。

契約では `environment_kind` を deterministic-backend/virtual-os-endpoint/test-plugin/real-window/physical-device の kebab-case 閉集合として canonical に定義した。ポリシー本文の開いた列挙に対し、契約で閉集合化した形である。Case schema は必須14項目の直交構造とし、境界 Case は「本番 OS-API 通過・実 ABI バイナリ・実画素 Native Dock」の3種とする (Issue #627)。

あわせて、独立 Oracle 規則と repo 別所有権、3リポジトリ受入マトリクス、Dogfooding 責務を契約として確定した。これらは後続の Harness 実装（Issue #628〜#631）の入力契約となる (Issue #627)。

旧物理 Required Issue の移管台帳と、旧物理 Required の扱いに関する4ルールを確定した。「仮想 Pass は物理 Pass を意味しない」ルールを設け、旧物理 Required Case の実際の requirement/result 遷移は各 repo のレビュー待ちとした。後続実装で `environment_kind` の追加や schema 変更が必要になった場合は契約改訂として扱う (Issue #627)。

## 決定論的テスト基盤の構成と独立 oracle

test-only クレート `ruviform-audio-test` を新設し、VirtualClock（整数演算のみの wait-free read）・決定論的 PCM fixture（全 pattern が絶対 frame の純関数）・production の transport 実装経由の MIDI trace・独立 oracle・Case Schema v1 の run metadata を実装した (Issue #628)。

bit 決定論は libm 非依存で実現した。Sine は 2^32 fixed-point phase と Taylor 近似、Sweep は瞬時周波数の閉形式積分、Noise は stateless な per-frame splitmix64 hash で、いずれも絶対 frame の純関数である。MIDI trace は production の `TransportPlanBuilder`/`TransportScheduler::run_callback` を block 駆動し、`SpanSink` 境界で delivered event を捕捉する（リポジトリに production の MIDI output port が存在しないため）(Issue #628)。依存追加なし・unsafe なし・公開 enum は `#[non_exhaustive]` が境界条件であり、pattern 定義は spec-as-data として公開し `validate()` で検証する (Issue #628)。

独立 oracle の原則は「期待値を被試験コードから生成しない」である。PCM oracle は libm `f64::sin` の独立実装、MIDI oracle は scheduler 非経由の独立 sort・frame 変換とし、生成経路と期待値経路を分離した。integer-domain pattern（silence/impulse/ramp/square/noise/constant）は Exact 比較、sine/sweep は libm と tolerance（AbsEpsilon 1e-5）で独立検証する (Issue #628)。

## scripted backend の構造と RT 計測境界

scripted backend は production port 契約（DeviceEnumerator/Device/InputStream/OutputStream）の deterministic 実装であり、callback 発火は test driver の manual pump のみで、OS API・thread・実デバイスを使わない。enumerate/default/vanish/fault 注入 API を持ち、trait object と test handle の共有状態は `Arc<Slot>`（`Mutex<Core>` と `AtomicBool` の dead flag）で共有し、invalidate/drop で dead を立てて direction の再 open を可能にする (Issue #628)。

manual pump の prologue は状態を契約どおり処理する。dead/invalidated は `StreamInvalidated`、vanished は invalidate latch と error callback 1回通知、not-started は `Backend("not_started")` の順で扱う (Issue #628)。

この実装で確認された port 契約の事実として、契約正本には close API が存在せず `stop()`+Drop が配送停止境界であり、`start()` は delivery を開始済みにするだけで callback を発火させない (Issue #628)。

RT 計測境界では、callback firing 区間（view 構築・timestamp・callback 起動・counter 更新）を allocation-free とし、record_output capture と pump 自体は non-RT の test-driver path として分離する。pump 内で device lock を callback 中に取らない設計とし、必要 field は open 時に core へ複製する (Issue #628)。

`rt_` prefix のテストは `#[global_allocator]` 付きの `assert_no_alloc` で計測し、post-callback の capture が意図的に確保する allocation を除外するため `record_output: false` で計測する。`rt_` テストは既存の test-rt lane が自動収集するため CI lane の新設は不要と判断し、その旨を README に明記する慣行とした (Issue #628)。

## pipeline wiring・RunMetadata・cancellation の写像

pipeline wiring（scripted backend から production 消費者口への投入）は「製品コード変更ゼロ」が受入条件である。駆動用の `pub(crate)` API は外部から駆動不能なため、test-owned の mirror（`InputPump`）が同一アルゴリズムで public API を駆動する構成を採る。本番の入力記録コンポーネントが意図的に未存在のため、bounded SPSC pool で PCM と入力メタデータを handoff し、Expected は `pcm_reference_block` の独立生成で本番 consumer 出力を正解値として共用しない (Issue #628)。

配線作業で判明した既知制約として、Session の error callback は no-op であるため production callback 内の `DeviceError` 値自体は session surface から観測不能で、発火・失効の計数のみが生存する。この観測限界は finding として記録し、製品側は変更しない。session 側に error observer を公開するかは製品側の将来判断とした (Issue #628)。

cancellation は production 契約に概念がないため、新 variant・新 API を発明せず既存の observable（data callback 不発・`Backend("cancelled")` error・stream 終端）で写像する。callback_index は進めるが position/frames は進めず、error 単独では終端しないことを負例 test で固定した (Issue #628)。

RunMetadata の JSON serialization は、fieldless enum に `#[serde(rename_all = "kebab-case")]` を付与して Case Schema v1 語彙（`deterministic-backend`/`os-l1`/`not-run` 等）に一致させ、`EnvironmentKind::as_str` との一致を test で pin した。入出力を伴う API は非 RT の test-driver API として rustdoc に明記する (Issue #628)。

全体の作業は deterministic core → scripted backend → pipeline wiring → close-out の4 slice で構成された。close-out slice は製品クレートの src 変更ゼロを境界条件に、残存項目（serialization・fault 種別・docs 陳腐化修正・受入条件の棚卸し）を test クレート内に限定して統合し、前提となる merge 済み PR を依存として明記し、全 AC × verdict の棚卸し表を証跡として残す運用とした (Issue #628)。

## Rust 実装と検証環境の落とし穴

`usize::from(u32)` は std に存在しないため `as usize` を使う。`assert_no_alloc` は `#[cfg(debug_assertions)] #[global_allocator]` の付与が必須であり、static の宣言のみでは allocator が差し替わらず計測が無効になる。IEEE-754 仕様演算に集中する clippy の cast lint には、spec であることを理由とする `#[expect]` を対象関数に局所付与し、`#[allow]` は使わない (Issue #628)。

`MutexGuard` 越しの field-precise borrow は guard 全体を borrow するため、mutable borrow が後続の read をブロックする。open 処理を「fault 消費（scoped mut）→immutable read→slot 代入」の3相に分割して解消した。`&mut` 引数の同時渡しによる借用競合は、方向別の型（`&mut OutputCore`/`&mut InputCore`）で直受けし、本体で `let core = &mut *guard` に rebind して解消する (Issue #628)。

試験側の失敗が実装バグとは限らない例が記録されている。絶対 frame の純関数性（position independence）を検証する際、planar layout で channel 毎の slice 対応を考慮せず連続領域を直接比較して失敗した件、および契約どおりの挙動（pump 時のみ latch される仕様・drop 前再 open の `DeviceBusy`）を誤読した件では、全て test 側を修正して実装は契約どおりだった (Issue #628)。

テスト記法の制約として、`PartialEq`/`Debug` 非実装の型に対する Err 検証は `unwrap_err` ではなく `matches!` を使い、`expect_err` の `T: Debug` 要求には test 側の手動 Debug impl で対応し、`#[non_exhaustive]` 型には wildcard arm を割り当てる (Issue #628)。

## 未確定・保留事項

- Linux/ALSA・macOS/CoreAudio・初回 timestamp=0 などの未観測項目は、ADR Confirmation への引き継ぎとして残っており、昇格判定では未観測のまま扱われた (Issue #23)。
- DAG 非循環を常設監視する CI gate は存在せず、監視 gap として記録されたままである（違反ではない）(Issue #192)。
- 旧物理 Required Case の実際の requirement/result 遷移は各 repo のレビュー待ちであり、当該作業は移管台帳の確定までの範囲である (Issue #627)。
- consumer oracle（AudioLens/Synvelis 側の独立 oracle）・3OS 実走・MIDI attach()/実 endpoint 配線は後続 slice および Issue #629/#630/#631 の領域へ Deferred とされた。consumer oracle は両 repo で merge 済みと記録されるが、実行結果・判定の詳細は未確認である (Issue #628)。
- Issue #419 の ADR status は `proposed` のままであり、正式決裁（accepted/conditional/rejected）は design authority 権限の Follow-up として残っている。
- Issue #586 で行われた IR 30件の front matter 修正の内訳（分類 A〜E の個別判断）は設計文書に記録されているが、その詳細は本資料では確認していない。
- 実装記録（IR）と実験記録（exp）には本抽出の対象外の文書が多数存在し未読である。実験ログ（jsonl）は対応する記録文書の要約で内容を把握しており、個別行の精読は行われていない。
- Issue #628 の各 slice（deterministic core・scripted backend・pipeline wiring）の正本 IR と証跡の棚卸し表は存在するが、本資料では未確認である。
- 監査で洗い出された既存負債（broken-relation-link・missing-experiment-link・invalid-event-* 系の警告、bare prose の Related Experiments、HTML コメントのみの Adopted/Rejected 節、`.gitignore` 対象の未コミットログ参照）は別 Issue の対象として未解消のままである (Issue #586/#590)。
- status が done の task 文書でチェックボックスが未チェックのまま残る記録があり、実施完了か更新漏れかは判別できていない。また文書監査ツールが報告する repo 全体の既存 broken relation は、本資料の対象作業では解消されていない。
