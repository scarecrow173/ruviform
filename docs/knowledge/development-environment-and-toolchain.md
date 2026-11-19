---
id: "KNOW-QnMonN3Oz14duTQ9HZCgHg"
type: "knowledge"
status: "draft"
lifecycle: "draft"
knowledge_kind: "operation"
title: "開発基盤・ツールチェーン・MSRV"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/1", "https://github.com/scarecrow173/ruviform/issues/2", "https://github.com/scarecrow173/ruviform/issues/18", "https://github.com/scarecrow173/ruviform/issues/21", "https://github.com/scarecrow173/ruviform/issues/34", "https://github.com/scarecrow173/ruviform/issues/52", "https://github.com/scarecrow173/ruviform/issues/53", "https://github.com/scarecrow173/ruviform/issues/223", "https://github.com/scarecrow173/ruviform/issues/411"]
  informs: []
  explains: []
  supersedes: []
---

# 開発基盤・ツールチェーン・MSRV

この文書は、開発環境・ツールチェーン・MSRV・ビルド基盤に関する設計判断と運用知見を、後に削除された過去の実装記録から抽出してまとめたものである。記録元の文書は残っていないため、本書はそれらに依存せず単独で読めるように記述する。各記述の末尾に Issue 番号で出所を示す。実測値や観測された挙動は事実として記し、記録時点の判断理由や解釈はその区別が分かるように記す。

## 検証タスクの集約と CI プロファイル分離

ローカルと CI で共通の検証入口を mise タスクに一本化し、CI 用に `[profile.ci]` と `check-ci`/`lint-ci`/`test-ci`/`build-ci` タスクを `CARGO_INCREMENTAL=0` 付きで定義した。`verify-ci` は `fmt-check`→`check-ci`→`lint-ci`→`test-ci` の逐次実行であり、CI プロファイルにより `target/ci/` と `target/debug/` が分離生成される（Issue #2）。`*-ci` タスクは容量消費が大きいため CI 用途に限定する運用知見が記録されている（Issue #2）。

`fmt-ci` タスクは意図的に作成していない。cargo fmt はプロファイルに依存しないためであり、`verify-ci` の先頭はプロファイル非依存の `fmt-check` とした。`verify-ci` は `depends` ではなく `run` 配列で `mise run` を順次呼ぶ形とし、順次実行を保証している。`*-ci` 系と `verify-ci` には `env = { CARGO_INCREMENTAL = "0" }` を付与した（Issue #2）。

`verify-ci` はローカルの総合検証には使わない。CI 用途限定で `target/ci/` が併存する構成を避けるためであり、ローカル総合検証は通常タスク群で代替した（Issue #21）。

## ツールチェーンと依存の固定

Rust ツールチェーンは `mise ls-remote rust` で `minimum_release_age = "7d"` を考慮して具体バージョンを選定し、`stable`/`latest` の浮動指定を禁止した。`mise.toml` の `rust = "1.99.0"` はこの方針で pin された開発ツールのバージョンであり、MSRV とは別の値である（Issue #2、Issue #52）。

`Cargo.lock` はライブラリ慣習の非追跡を不採用とし、Git で追跡する ADR 決定を行った（Issue #2）。CI の cargo cache は `~/.cargo` の registry index・registry cache・git db のみを `Cargo.lock` ハッシュでキー付けし、`target/` 全体はキャッシュしない限定形を採った（Issue #2）。初期化コマンド（`cargo new`/`init`・`apm init`）は既存ファイルを破壊するリスクがあるため使わず、全ファイルを手書きで作成した（Issue #2）。

pipx 経由のツールを `mise install --locked` で復元可能にするには、`.mise/locks/<tool>/<version>/` 配下に `pyproject.toml` と `uv.lock` からなる dep-graph 生成物をコミットし、`mise.lock` 側が `path`/`digest` で参照する形が必要である。`uvx_args`/`pipx_args` は dependency locking に非対応のため指定禁止とし、lockfile 再生成条件表にも明記した（Issue #2）。

`.gitattributes` の LFS 対象は audio/video/大型編集ソースのみで JSON/YAML は対象外である。このため「小さいテストフィクスチャは非メディア形式で Git に置く」方針を LFS 設定が阻害しないことを確認済みで、CI は `git lfs pull`/`lfs: true` を持たず、フィクスチャが必要になるまで LFS 取得を省略する方針と一致する（Issue #2）。

## workflow_dispatch 限定 CI の検証手順

`workflow_dispatch` は default branch にワークフローが存在しないと起動できない。この制約から、導入ブランチ上では静的検証（prek check-yaml・actionlint）に留め、マージ後の手動ディスパッチで `df -h`/`du -sh target` の実測ログを取得する運用を前提化した（Issue #2）。actionlint は mise registry でバージョンを pin して採用する方針（推奨案として記録）とし、解決不能時は check-yaml のみにフォールバックする判断基準が記録された（Issue #2）。

## 新規・クリーン環境の再現

新規 clone 後の手順は `mise install --locked`→`mise run setup`→検証タスク群の順であり、前提条件としてネットワーク到達・権限・APM git 認証・`GIT_TERMINAL_PROMPT=0`・`mise trust` が記録されている。lockfile 再生成条件表（mise.lock/apm.lock.yaml/Cargo.lock それぞれのトリガと復旧手順）、暫定開発規約、未検証 OS の一覧は作業メモに集約された（Issue #2）。暫定開発規約には unsafe 局所許可・default feature 最小・依存理由の記録・workspace.lints 正本と `-D warnings`・doctest・`*-ci` の CI 限定・LFS 方針が含まれる（Issue #2）。

クリーン環境のシミュレーションには `MISE_CONFIG_DIR`/`MISE_DATA_DIR` を空ディレクトリに向け、global mise config と既インストールツールを切り離して新規マシン相当を再現する手法を確立した。ただしこの手法には検証手法自体に起因する落とし穴が記録されている。分離 data dir に node が無い条件で mise shim が `mise x -- node`→shim→`mise x -- node` の無限リスポーンを起こし、約 2800 プロセスまで増殖して kill された。これを受けて `mise run` 系検証はクリーン worktree と通常環境へ切り替えた。global config 由来ツールの `--locked` 失敗は環境 caveat であり、クリーン CI では非発生である（Issue #2）。

## MSRV の宣言・引上げ・言及同期

workspace の `rust-version` は 1.85 から 1.89 へ引き上げた。根拠は `fearless_simd` 1.1.0 が MSRV 1.89 を要求することであり、Issue #34 で 1.85 が確定した後の再評価で決まった（Issue #52）。適用範囲は `[workspace.package]` の宣言値のみで、メンバーは `rust-version.workspace = true` で継承する。`rust-version` の変更は依存解決（resolve）に影響しないため `Cargo.lock` の再生成は不要であり、`mise.toml` の `rust = "1.99.0"` pin は開発ツール pin であって MSRV ではないため据え置いた（Issue #52）。

MSRV への言及は同期対象と非対象を分けて管理した。同期対象は `Cargo.toml`（宣言値とコメント）・`AGENTS.md`・`README.md`・`crates/ruviform-io-symphonia/Cargo.toml` のコメントの 4 箇所である。一方、`poc/ui/RESULTS.md` の wgpu MSRV メモと `docs/specs/dsp-simd-poc-adr.md` の前提記述は発行時点の記録であるため意図的に変更しなかった。現行宣言と過去の記録を区別するこの判断は、MSRV 更新時の同期範囲を決める先例として記録されている（Issue #52）。

## MSRV 検証と lint ゲートの方針

lint gate は pin toolchain（rust/clippy 1.99.0）のみで判定し、MSRV 1.89 は build/test で検証する。両バージョンで同一の lint 結果を要求しないのが Owner 方針である。MSRV 上で失敗した場合は「実契約違反か toolchain の版差か」を個別に判定し、意味のあるものはコードを修正し、単純な版差は文書化された除外ポリシーで扱う分類アプローチを採った（Issue #223）。

`float_cmp`/`similar_names` を `workspace.lints.clippy` で一括 allow する案は却下した。両 lint は clippy 1.99 でも pedantic に残るため、中央 allow は MSRV 1.89 には効くが pin gate を将来の違反に対しても閉じてしまい、「pin gate を弱めない」という Owner 方針に反する。代わりに `AGENTS.md` に MSRV 検査用の `-A` 除外コマンド `cargo +1.89.0 clippy --workspace --all-targets -- -D warnings -A clippy::float_cmp -A clippy::similar_names` を明文化した（Issue #223）。

版差 lint への対処には個別パターンが記録されている。`suboptimal_flops` の 10 箇所は検出境界が toolchain 間でずれるため `expect` から `allow` へ変更した。`expect` は充足を要求するため版差を表現できず、`allow` は両版で安全だが検出境界の変動に気付かないという意図的なトレードオフである。`invert` は `split_at_mut`+`iter_mut`/`zip` への書き換えで `needless_range_loop` の `expect` を削除し、bit-exact を維持した。`must_use_candidate` は `#[must_use]` を付与すると pin toolchain で `double_must_use`（`Iterator` が trait レベルで must_use）に違反するため、理由付きの `allow` で処理した。`range_plus_one` は `Vec<Range<usize>>` の要素型制約で `..=` が型不適合となる 1.89 の誤検出として `allow` した（Issue #223）。

MSRV 検証タスクは opt-in の mise タスクとして新設するパターンを採った。`test-ui-test-msrv` は既存の `test-ofx-msrv` と同型で、`mise exec rust@1.89.0 -- cargo test -p <crate>` を実行し、`CARGO_TARGET_DIR=target/msrv-<crate>` で既定レーンと target dir を分離する。検証は RED を先に実測してから修復し GREEN を確認する順で進める検証駆動フローとした（Issue #411）。

## MSRV 適合を条件とする依存選定

`cargo_metadata` は 0.23 から 0.19.2 へ修復した。0.20/0.21 系列も評価したが、platform 0.2 系は MSRV に適合する一方で cargo-util-schemas 系の追加 transitive tree を引くため不採用とした。0.19.2 は manifest 制約 `^0.1.2`（cargo-platform）により通常解決が MSRV 範囲へ収束し、cargo-platform@0.3.3（rustc 1.91 要求）への再発経路がない構造である。対象は test-support クレートのため本番依存グラフへの波及もない（Issue #411）。

0.19.2 で name 系が `String` 化したことへの対応は、`*.name.to_string()` を `*.name.clone()` へ置き換える 12 箇所の最小互換調整で行った。目的は clippy `implicit_clone` の解消であり、生成される値は同一の `String` である。guard の型付き検査の意味は、fixture の正負ケースと live workspace の semantic test がすべて 0.19.2 で通ることで保持を確認した。型の差のみで意味の差はないと実測されている（Issue #411）。

## SIMD dispatch 層の設計と検証

`ruviform-math` には prepare 時に固定する不透明な `SimdToken`・8 カーネルの公開 API・カーネル/サイズ別のスカラールーティングを実装した。設計前提はスカラー参照実装の必須・等価性契約（ビット一致または ulp 上限）・stable-only・`unsafe` 局所許可である（Issue #53）。

`fearless_simd` は feature なしで `[dependencies]` に昇格させ、dev-dependency 側に `force_support_fallback` と `fearless_simd_macros` を維持する二層構造とした。本番経路と実験経路を分離する構成である（Issue #53）。

カーネル/サイズ別のスカラールーティング閾値は design のデータ根拠に基づく。公開 API 経由のルーティングが forced-token 経路の小 N 検証をマスクしうるリスクがあるため、単体テストでは内部の `*_impl` を直接呼び出して両経路を個別に検証する設計とした。また `#[simd]` マクロが dev-dependency の `fearless_simd_macros` 由来であるため、本番コードへの proc-macro 依存の持ち込みを確認対象とした（Issue #53）。

## rt_ 命名規約と test-rt レーンの既知制約

`mise run test-rt` は `cargo test --workspace rt_` の部分文字列フィルタで RT 契約テストを選択する。このため非 RT のテスト・ヘルパー・公開 API の名前に `rt_` を含めない規約を定め、`assert_*`/`start_*` を禁止して `expect_*`/`spawn_*` 等を使う。Issue #18 では `convert_exact` の doctest 名が `rt_` を内包して誤選択された先例があり、フィルタの厳密化は RT 検証 Issue へ deferred とされた（Issue #21）。

`cargo test` の name フィルタは doctest ステージを実行しない。Rust 1.99/edition 2024 のマージ doctest 動作で `cargo test --workspace rt_` が doctest を実行しないことを実測確認した。`rt_` を含む doctest 名は media-core の `convert_exact` ×2 のみで既知制約だが、test-rt レーンへは混入しない。外部型の非露出は cargo doc ではなく公開シグネチャレビュー（全 pub アイテムの列挙と grep で private 領域のみヒットすることの確認）で実施した（Issue #21）。

## RT プリミティブの依存・メモリ管理・構築防止

RT 基盤で採用した依存は `rtrb` 0.4（SPSC チャネル）・`arc-swap` 1（スナップショット交換）・`audio_thread_priority` 0.38 `default-features = false`（dbus 経路を `with_dbus`/`dep:dbus` の排除で切断）である。採用依存の型・エラー・feature 名は公開 API に露出させない方針で、依存は private 実装詳細に留める（Issue #21）。

退役チャネルは SPSC 上に自前で構成し、`retire` は `try_push` の意味で満杯時は `mem::forget` による意図的な leak とした。backpressure で RT スレッドをブロックしない設計である。`DropCollector` は Vec の pending を持ち、`publish` 経路で追い出された `Arc` を `pub(crate) push_local` で同一の drain 経路へ流す seam を備える。ローカル pending Vec は non-RT only の契約内で伸長を許容する（Issue #21）。

`ArcSwap::load_full` の `Arc` を生で返すと最終 drop が RT スレッドで dealloc を走らせうるため、参照は `SnapshotGuard` 経由のみに限定し、guard の `Drop` を退役チャネルへ送出する設計とした。reader 内の `RefCell<Retirer<Arc<T>>>` により `SnapshotReader` は `!Sync` となり、単一 RT reader・単一論理 writer の前提を API 契約として明示した（複数 reader は Out of Scope）（Issue #21）。

`RtContext` の構築防止は `#[doc(hidden)] mint` と駆動者限定規約・コードレビューによるもので、型レベルの完全な抑止ではない。design のベースライン文言の弱体化として ADR に記録済みである。`promote_current_thread_to_realtime` は非特権 Linux で典型的に `Err` を返すのが仕様であり、テストは `Ok`/`Err` のいずれも受理してパニックしないことのみを検証し、実効差の計量は audio-device Issue に委ねた（Issue #21）。

最終監査では公開 enum の `#[non_exhaustive]`・全公開 API の `**RT contract:**` ラベルと doctest・非 RT 識別子の `rt_` 非含有・`unsafe` 不在・外部型非露出の 5 系統を確認した（Issue #21）。`unsafe` は `alloc_counter.rs` の `#[expect(unsafe_code)]` と SAFETY コメント付きの test-scoped counting allocator が唯一の使用である。std のみの counting allocator は `unsafe impl GlobalAlloc` を必然とするため、design 不変条件「局所許可も要求しない」からの逸脱だが、spec が許容した局所許可として規約内に収まると整理された（Issue #21）。

## ワークスペース設定と公開制御

`[workspace.package]` の値は `*.workspace = true` の宣言でのみメンバーに継承される。`[workspace.package]` に `publish = false` を置くだけではメンバーに効かず、`crates/ruviform-core/Cargo.toml` に `publish.workspace = true` の継承宣言が必須である。記述漏れがあると member の `publish` が `null` のまま残り、`cargo metadata --format-version 1 --no-deps` で機械検出できる設計になっている（Issue #1）。

`publish = false` の適用は licensing ADR の「未決定クレートは公開しない」原則に基づく誤公開防止策である。ライセンス未決定の時期に workspace 全体へ `publish = false` を適用し、当時の唯一のメンバーである ruviform-core にのみ `publish.workspace = true` を継承させた。将来の公開決定時はクレート単位で解除する運用が前提であり、edition/rust-version/license の値行は変更せずコメントのみを更新した（Issue #1）。

## 文書への反映方法論と記述規律

README/AGENTS への RT 契約の注意書きには、design ベースラインの「RT 契約（API 単位の保証定義）」「RT 違反検出の分担」節に基づく 4 点の明記が要求された。「RT 契約は API 単位で定義・明示する」「保証は Ruviform 自身の RT 対応実装に限定する」「CI 成功は完全証明ではない」「OS レベルのハードリアルタイム保証は提供しない」の 4 点であり、未確定の API を RT-safe と断定しない規律も含む（Issue #1）。

design が逐語の文案を確定していない文書反映では、執筆を design の引用元節（依存方向・RT 分離・SIMD 採否・採否表）の範囲に限定し、表現の最終担保は実装 PR のレビュー観点とした。編集前に `git grep` でベースラインを記録し、完了時は `git diff --name-only` が宣言したファイルに限定されることを検証する手順を採用した（Issue #1）。

README の「決定済み・未確定」の再構成では、対応プラットフォーム（Windows/macOS/Linux・x86_64+aarch64）を platform ADR で決定済みとして「決定済み」側へ分類し、公開 API・サポート形式・ライセンス・最終クレート分割・edition/MSRV を「未確定」側へ分類した。「検討中」節の各項目は WebView ファースト（UI ADR）・wgpu opt-in・ネイティブ UI 要 PoC・CLAP/VST3 同格（plugin ADR）に対応付け、対応付けられない項目は未確定側に残す方針とした（Issue #1）。

## 未確定・保留事項

記録が残す未解決・保留事項を列挙する。

- 対象文書の選定は限定的であり、計画・タスク系の文書群（500 件超）と IR 文書群（200 件超）の大部分は未読で、それらに含まれる知見は本書に未収録である。また各文書の front-matter の relations が示す上流の spec/design/ADR 文書の本文は未確認で、設計決定の一次根拠は本書では検証されていない。
- `rt_` フィルタの厳密化は RT 検証 Issue へ deferred とされ、`promote_current_thread_to_realtime` の実効差の計量は audio-device Issue に委ねられており、いずれも結果は本書の範囲外である（Issue #21）。
- カーネル/サイズ別スカラールーティング閾値の具体値は記録になく、design のデータ根拠の内容も未確認である（Issue #53）。
- `#[simd]` マクロ由来の proc-macro 依存が本番コードに持ち込まれていないかは「確認対象」と記録されているが、確認結果は記録にない（Issue #53）。
- macOS/Linux のローカル環境・Actions の実ディスパッチ・素の新規 Windows 環境は未検証と記録されている（Issue #2）。
- RT 契約注意書きの 4 点が design ベースラインからの逐語指定かどうかは記録からは判別できない（Issue #1）。
