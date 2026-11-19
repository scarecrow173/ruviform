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
  source: ["https://github.com/scarecrow173/ruviform/issues/3", "https://github.com/scarecrow173/ruviform/issues/23", "https://github.com/scarecrow173/ruviform/issues/41", "https://github.com/scarecrow173/ruviform/issues/192", "https://github.com/scarecrow173/ruviform/issues/193", "https://github.com/scarecrow173/ruviform/issues/384", "https://github.com/scarecrow173/ruviform/issues/419", "https://github.com/scarecrow173/ruviform/issues/553", "https://github.com/scarecrow173/ruviform/issues/586", "https://github.com/scarecrow173/ruviform/issues/590", "https://github.com/scarecrow173/ruviform/issues/627", "https://github.com/scarecrow173/ruviform/issues/628"]
  informs: []
  explains: []
  supersedes: []
---

# ドキュメント運用・エージェント作業基盤・Issue 管理

この文書は、プロジェクトの文書運用とエージェント作業手順に関する再利用可能な設計・実装知見をまとめたものである。内容は、後に削除された実装記録から抽出したものであり、出典は各記述に付した Issue 番号で示す。測定値や観測された挙動は事実として記し、解釈を含む記述はその旨を明記する。

## 文書の表現規約と検証済み記述の境界

README と AGENTS.md などの文書では、事実記述と構想・未確定事項を厳密に分離する運用が確立されている。表現ルールとして「検証済み・構想・検討中・未確定・未検証・仮置き」の区分を定義し、README の Status 節では「実在・検証済み／構想（未実装）／検討中／未確定」の4区分に整理した（Issue #3）。環境限定（検証済み環境が限定的であること）や「仮置き」の注記は、該当する全箇所に適用する。

未作成のクレートや API を実在するものとして扱う記述、検証していない事項の断定的表現、決定済みでないライセンスの表現を禁止し、レビューで担保する。Quickstart には一次記録で実機検証済みのコマンドのみを記載し、`mise.toml` に存在してもソースを破壊的に書き換えるタスクは意図的に除外した（Issue #3）。

テンプレート由来文書の固有化では、プレースホルダーの一括除去を grep で検証する。catch-all パターンは除外版と込み版の2回実行し、除外は「書き換え対象集合」の定義であって記録対象からの除外ではないと明記する。正規表現のドットはハイフン・空白の双方に一致するため、リテラルの異なる表記を一括で捕捉できる。リテラルに該当しない意味的残存は個別に指名して捕捉する（Issue #3）。最終スイープでは事前のベースライン記録との hit 数比較で判定し、除外対象は履歴記録と生成物パスに限定して、パターン外の残存が0であることを実測した（Issue #3）。

## 文書チェーンの権威構造とスナップショット運用

umbrella 設計文書・実行 Issue 本文・tracking canonical の3層で権威を分離する構造を採用した。設計合意と実装・検証は分離し、未実施項目は not run／NotRun／Unsupported として記録する（Issue #192）。参照先 Issue の状態は変動するため、記録には snapshot 日付を明記し、役割記述が古くなる可能性をリスクとして明記する。承認済みの設計権威文書の本文改訂は別 Issue の判断とし、未 merge の並行文書は参照せず、既存の broken relation は範囲外として分離報告する（Issue #192）。

版付き Snapshot 節は基準日識別子（`Snapshot-YYYY-MM-DD`）を版とし、Issue 状態の実測日を使う。既存本文は履歴 snapshot として保持し、更新は dated 節の追加で表現する append-only 方式を採る。両 canonical に同名節を追記する場合はバイト一致が必須であり、相対 markdown リンクはファイル間で解決が変わるため使わず、Issue 参照は GitHub URL、文書参照は repo 相対パスの inline code で記述する（Issue #193）。旧記述の直接書き換えは「内容は一致させる」約束と履歴保持の原則に反するため棄却し、旧 owner 記述は dated 上書き宣言で残存扱いとした（Issue #193）。

## Issue 状態の監査と受入 owner の記録

Issue 状態の census は、`gh issue list`／`gh issue view` で複数リポジトリの Issue 状態を取得し、計画関連 Open Issue の「着手条件（Blocked by）」節を照合して証拠を固定ファイルに保存する。Blocked by は全 Milestone を「なし」とし、実 OS・実機 Gate は Integration Gate／独立 Owner Issue／Deferred-Activation にのみ記述する規則を採用した（Issue #193）。

並行 Lane（Script・Creative Branch・Future capability）は Milestone の Blocked by にしない。GUI-L2／EXT-L3 は Windows で実際に提供・利用する機能のみ Required とし、macOS・Linux は利用要件が発生するまで Deferred とする。物理機材・市販 Plugin の一括 Required 化は課さない（Issue #193）。

状態語彙は軸を分離する。要求状態（Required／Deferred／NotApplicable）と実行結果（Pass／Fail／NotRun／Unsupported）は別軸であり、Issue の open／closed は実装の Blocked by と同義ではない（Issue #193）。Headless 実装 Slice の着手を L2／L3 の NotRun で止めず、製品提供の最終受入では Fail／NotRun を維持する段階分離を採る（Issue #193）。

受入 owner の remap は、受入 owner と実装・mechanism owner を列で分離し、dated addendum を append-only で tracking canonical と対応表へ追記する方式で記録する。Required→Deferred の格下げ、NotRun→Pass の置換、新規 Pass の付与は禁止する（Issue #192/#193）。

## 依存関係監査と文書生成の環境差異

`cargo metadata` の `dep_kinds` 解釈では、`kind: null` は normal dependency を意味する（serde 表現）。これを non-normal と誤ると normal edge を0と誤報告する。dev-dep の上位参照は umbrella の規定どおり循環と扱わず、kind 別に集計して報告する。manifest 宣言 edge と resolver 活性 edge の差分は feature ゲート optional edge の活性化で説明できるため、default と all-features の2集合で報告する（Issue #192）。

文書生成スクリプトはリポジトリ root を cwd として実行し、`--dir` で出力先を明示する。skill package パスから実行すると cwd 依存で skill 配下の docs ディレクトリに誤生成される。Windows Git Bash の `/tmp` と Windows Python の `/tmp` 解決が異なり FileNotFoundError になるため、Python には実パスを渡す（Issue #192）。

監査手法として `cargo tree`・CI 実走証拠の収集・crate 全読みの深掘りは代替案として棄却し、`cargo metadata` ベースの dep_kinds 解析を採用した。棄却理由は discovery 正本の Alternatives and Comparison に記録されている（Issue #192）。docs-only の変更では POLICIES の比例原則に従い cargo test/build を省略し、フォーマットチェック・lint・文書監査で検証する。文書監査の増分が非 blocking の警告のみで、blocking 件数が全て既存の範囲外 finding であることを確認した（Issue #192）。

## ADR の昇格と verdict の逆反映

ADR の status 昇格は、design の「確認項目↔証跡対応表」に従い、各確認項目を実験ログ・テスト・静的 grep の証跡へマッピングしてから行う。未観測項目は ADR Confirmation に引き継ぎとして明記し、昇格を阻害しない判断根拠を残す（Issue #23）。

Coordinator 決裁済みの verdict を後継 ADR へ逆反映する際は、「推奨」と「決裁」を分離して転記する。採否の再判定・ADR status 変更・Issue 投稿は行わない。判定語彙（NotRun／Fail／Unsupported／Deferred）を Pass にせず、宣言 Capability と実測 Pass を混同しない記述規則を固定した（Issue #419）。

逆反映欄には outcome・判定根拠・評価範囲の限定・標準 Driver 方針の維持・OS 別 Capability 宣言記録・既知制約と将来 Owner・証拠リンクを日付付きで記録する。Capability Matrix 宣言テンプレート表は原則不変とし、宣言記録は逆反映欄へ集約する。個別 CaseID は verdict-matrix を正本として参照させ、ADR には詳細を複写しない（Issue #419）。

## 実装記録と index の運用

docs/impl/exp の管理 index の生成領域は `*.jsonl` のみを対象とし、`.md` 自由形式記録は hand-curated の「Reports and raw data」節（生成領域外）に列挙する。生成 index の適用は生成領域を byte-for-byte で保持し、curated 節への1行追加は慣行どおり実施する（Issue #192/#419/#553）。`data/` サブディレクトリは raw per-run JSONL を flat な event-log 監査範囲外に分離するためのものである。

IR 記録の index 生成可能化では、front matter をスキーマに適合させ、README に generated-index marker と managed region を追加する。本文・ファイル名・既存 id は不変とし、index apply は `--expect <planHash>` 付きで実行する（Issue #586）。missing-section の補完では canonical 必須セクションの本文を各文書の実態に沿って補完し、既存の非 canonical 見出しは内容を失わず canonical へ再配置する。実態不明の推測記述は禁止し、文書・リンク先・Issue/PR 記録に根拠を限定する（Issue #590）。

## API・型設計の契約と不変条件

`AudioBuffer` の形状チェックは private helper に集約し、`try_new`／`new`／`from_interleaved` で共用する。`new` は panic 契約を維持して `try_new` に委譲する。`ChannelLayout::Custom` の payload を非空 newtype に変更し、ゼロチャンネル layout を型レベルで排除した（Issue #41）。

`const fn` では `?` が使えないため、`Result` を返す処理への委譲は match で実装する。これにより非空ルールの実装を1箇所に保ちつつ、`const fn` のシグネチャとエラー契約を維持した（Issue #41）。

時刻変換の丸め誤差は「1呼出し最大1回」として doc のみで方針を明記した。連鎖変換は誤差が蓄積するため、安定アンカーから display/IO 境界で変換する。最小変更を守るため、module doc が方針をカバーする関数は意図的に未変更とした（Issue #41）。

## 決定論的テスト基盤の設計

test-only crate に VirtualClock・決定論的 PCM fixture・production TransportScheduler 経由の MIDI trace・独立 oracle・Case Schema v1 run metadata を実装した。Sine/Sweep は libm 非依存（fixed-point phase と Taylor 近似）で bit 決定論を担保し、oracle 側は libm 独立実装と tolerance で生成経路と期待値経路を分離した（Issue #628）。

独立 oracle の原則は「期待値を被試験コードから生成しない」である。integer-domain pattern は Exact 比較、sine/sweep は libm と tolerance（AbsEpsilon 1e-5）で独立検証する（Issue #628）。

scripted backend は production port 契約の deterministic 実装であり、callback 発火は test driver の manual pump のみで OS API・thread・実デバイスを使わない。trait object と test handle の共有状態は `Arc<Slot>`（`Mutex<Core>` と `AtomicBool`）で橋渡しし、pump prologue で dead／invalidated／vanished／not-started を契約どおり処理する。公開 enum は `#[non_exhaustive]` とする（Issue #628）。

RT 計測境界では、callback firing 区間を allocation-free とし、record_output capture と pump 自体は non-RT test-driver path として分離する。pump 内で device lock を callback 中に取らない設計とし、必要 field は open 時に core へ複製する。rt_ test では `record_output: false` で計測する（Issue #628）。

pipeline wiring は「製品コード変更ゼロ」が受入条件である。`pub(crate)` API は外部から駆動不能なため、test-owned の mirror が同一アルゴリズムを駆動する構成を採る。本番コンポーネントが不在のため bounded SPSC pool で handoff し、Expected は独立生成で本番 consumer 出力を正解値として共用しない。cancellation は production 契約に概念がないため新 variant・新 API を発明せず、既存の observable（data callback 不発・stream 終端）で写像する（Issue #628）。

## orchestration 契約と言語非依存の境界

Consumer-neutral Workflow/CLI orchestration では、共通 Engine の今実装と No-go を却下し、契約語彙のみ文書化・実装 Deferred を採用した（Option B）。根拠は、Synvelis AUTX-05/06 が未確定のため generic を固定せず Synvelis 具象を先行すること、AudioLens は orchestration 不要で consumer-only 継続とすること、1 Consumer の成功は共通採用を意味せず AudioLens／第三 Consumer の独立した必要性を再評価 Trigger にすることである（Issue #553）。

契約の境界規則として、Consumer 型の流入と第二正本を禁止する。Synvelis 固有型・GPUI・AudioLens 製品型・独自 Grant Policy を generic 契約へ流入させず、Capability/Policy/Project/Job/Approval の第二正本を作らない（Issue #553）。他 repo の orchestration 契約からは具象型を流入させず「形」の語彙のみを固定し、片側成功での共通化を防ぐ（Issue #553）。

既存 API の棚卸しは `pub` アイテムの grep 抽出と設計正本の結論との照合で行う。公開 API が Plan/Step/Run journal/Resume/Idempotency/CLI Envelope を提供しないことを確認し、Job/Cancel primitive を Reuse・cli-sample を Consumer-only と判定した（Issue #553）。

## 仮想優先検証契約とゲート移管

`environment_kind` は deterministic-backend／virtual-os-endpoint／test-plugin／real-window／physical-device の kebab-case 閉集合を canonical として定義し、ポリシー本文の開いた列挙に対し契約で閉集合化した。Case schema は必須14項目の直交構造とし、境界 Case は「本番OS-API通過／実ABIバイナリ／実画素Native Dock」の3種とする（Issue #627）。

旧物理 Required Issue の移管台帳と、旧物理 Required の扱い4ルールを確定した。「仮想 Pass ≠ 物理 Pass」のルールを設け、旧物理 Required Case の実際の requirement/result 遷移は各 repo のレビュー待ちとした（Issue #627）。`environment_kind` 追加や schema 変更が必要になった場合は契約改訂として扱う（Issue #627）。

## 作業環境とツールチェーンの落とし穴

`mise install --locked` は、global mise config が要求する repo lockfile 外ツールで部分的に exit 1 になり得る。判定基準は「repo 管理ツール全解決」をもって pass とし caveat を記録し、フォールバックの `mise install` を併記する（Issue #3）。

`apm.lock.yaml` は manifest の name/description を保持しないため、apm.yml の変更は lockfile に波及しない。manifest↔lockfile の整合は `apm install --frozen` の exit 0 で担保する（Issue #3）。

lockfile の復元は `mise install --locked` を正本とし、再生成トリガと復旧手順を表に集約する。追跡方針の記述は Git 節に一元化する分担を採る（Issue #3）。

`usize::from(u32)` は std に存在しないため `as usize` を使用する。`assert_no_alloc` は `#[global_allocator]` の付与が必須であり、static のみでは allocator が差し替わらず計測が無効になる（Issue #628）。IEEE-754 仕様演算に集中する clippy の cast lint には、spec であることを理由とする `#[expect]` を対象関数に局所付与する（Issue #628）。

`MutexGuard` 越しの field-precise borrow は guard 全体を borrow するため、mutable borrow が後続の read をブロックする。open 処理を「fault 消費（scoped mut）→immutable read→slot 代入」の3相に分割して解消した。`&mut` 引数の同時渡しによる借用競合は、方向別の型で直受けし本体で rebind して解消する（Issue #628）。

## AGENTS.md の構成と検証入口

AGENTS.md は節単位の keep/rewrite/add で改稿し、原則節は verbatim 保持する。未確定事項は「暫定原則（詳細契約は別 Issue で確定）」と明記し創作を避ける。新規節は前提となる記述の後に挿入する（Issue #3）。

過剰縮小への対応として、AGENTS.md を旧版へ復元し、検証入口の節のみを参照行へ置換し、詳細は別文書へ verbatim 移動して AGENTS.md を16KB前後に収めた。復元は `git show <commit>^:AGENTS.md` を正とし、セクション境界は見出しで機械的に特定する（PR #350 フォローアップ）。

## 未確定・保留事項

- Linux/ALSA・macOS/CoreAudio・初回 timestamp=0 などの未観測項目は ADR Confirmation への引き継ぎとして残り、昇格判定では未観測のまま扱われた（Issue #23）。
- DAG 非循環の CI 常設 gate は存在せず、監視 gap として記録された（違反ではない）（Issue #192）。
- docs/impl/exp の `.jsonl` 実験ログは raw イベントログであり、対応する `.md` 記録の要約をもって内容を把握した。jsonl 個別行の精読は行われていない。
- docs/impl/ir・docs/impl/exp には本知見抽出の対象外の多数の文書が存在し、それらは未読である。Issue #586 の対象だった IR 30件の front matter 修正の内訳（分類 A〜E の個別判断）は設計文書に記録されているが未読である。
- issue-628 の3 slice（deterministic-test-runtime／scripted-backend／pipeline-wiring）の正本 IR は対象外のため、exp 記録が参照する実装結果の詳細は未確認である。
- Issue #627 の旧物理 Required Case の実際の requirement/result 遷移は各 repo のレビュー待ちであり、本 slice は台帳確定までの範囲である。
- Issue #628 の consumer oracle（AudioLens/Synvelis 独立 oracle）・CI lane 新設・3OS 実走・MIDI attach()／実 endpoint 配線は後続 slice および Issue #629/#630/#631 領域へ Deferred とされた。
