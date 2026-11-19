---
id: "KNOW-zH1AsgygRDxY67ESy3cZec"
type: "knowledge"
status: "draft"
lifecycle: "draft"
knowledge_kind: "codebase"
title: "GUI view 契約・ウィジェット・テスト基盤"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/29", "https://github.com/scarecrow173/ruviform/issues/125", "https://github.com/scarecrow173/ruviform/issues/310", "https://github.com/scarecrow173/ruviform/issues/312", "https://github.com/scarecrow173/ruviform/issues/313", "https://github.com/scarecrow173/ruviform/issues/314", "https://github.com/scarecrow173/ruviform/issues/377", "https://github.com/scarecrow173/ruviform/issues/378", "https://github.com/scarecrow173/ruviform/issues/410", "https://github.com/scarecrow173/ruviform/issues/414", "https://github.com/scarecrow173/ruviform/issues/418", "https://github.com/scarecrow173/ruviform/issues/421", "https://github.com/scarecrow173/ruviform/issues/434", "https://github.com/scarecrow173/ruviform/issues/435", "https://github.com/scarecrow173/ruviform/issues/436", "https://github.com/scarecrow173/ruviform/issues/442", "https://github.com/scarecrow173/ruviform/issues/443", "https://github.com/scarecrow173/ruviform/issues/448", "https://github.com/scarecrow173/ruviform/issues/450", "https://github.com/scarecrow173/ruviform/issues/451", "https://github.com/scarecrow173/ruviform/issues/453", "https://github.com/scarecrow173/ruviform/issues/461", "https://github.com/scarecrow173/ruviform/issues/463", "https://github.com/scarecrow173/ruviform/issues/464", "https://github.com/scarecrow173/ruviform/issues/478", "https://github.com/scarecrow173/ruviform/issues/479"]
  informs: []
  explains: []
  supersedes: []
---

# GUI view 契約・ウィジェット・テスト基盤

この文書は、GUI view 契約・ウィジェット・scenario/oracle テスト基盤に関する設計と実装の知見を、後に削除された実装記録から抽出してまとめたものである。各記述の帰属は (Issue #NNN) の形で本文中に示す。実測値や観測された挙動は事実として記し、設計判断の理由付けや解釈を含む部分はその旨が分かるように書き分ける。

## 借用 planar view の型形状と配置

channel-major の連続ストレージを前提とする既存の `AudioBuffer` を拡張せず、独立 per-channel planar バッファをゼロコピーで参照する借用 view を新設した。配置先は `ruviform-audio-core` とし、`AudioBufferView` 系と同じ「音声バッファ借用形状」の責務にまとめた。video-core の `Plane`/`PlaneMut` の先例と整合し、plugin 以外の非連続バッファ供給にも再利用できる。fallback 候補は plugin-core であった。型形状は `&'a mut [&'a mut [f32]]` 相当で、ライフタイムを process コールバックに束縛する (Issue #29)。

view 構築自体は alloc しない。コンストラクタは、アダプタが activate/prepare 時に事前確保するチャンネルリスト用スクラッチを受け取る形状にした。in-place 処理（CLAP `in_place_pair` / clack `ChannelPair::InPlace` 相当）では `&[f32]` と `&mut [f32]` を同一領域に同時構築しない。ポインタ一致検出で専用 view 分岐かスクラッチ経路かを選ぶ方式は、方式 ADR の判断材料として評価対象に残した。f64 の型形状は予約し、借用のスコープ脱退は `compile_fail` doctest（borrow-escapes-scope 系）でコンパイル時に固定した (Issue #29)。

## ViewLink の世代採用と oracle の epoch 整合

`ViewLink.attach(windowId, generation?)` の世代採用契約は「検証→冪等判定→必要時 detach→採用/既存 increment→伝搬」の順序である。同一 window・同一 epoch の呼び出しは冪等で再購読せず、異 epoch では旧 subscription/gesture/query を失効させて再購読する。負数・非整数・NaN・非 safe integer の無効 epoch は状態を変えず、generation 省略時は local increment を維持し、late-created binding も authoritative epoch を採用する。

SDK 側では、`ViewLink.registerQuery` に `if (this.attached_) query.attach(this.generation_)` を追加し、attach 済み link への新規 query 登録が registerBinding と同じ adopt-on-join 規則で現行 view epoch を継承するようにした。意味論は「join=epoch 遷移（in-flight は旧 epoch で `cancelAll()`）、unregister=membership 解除のみ」と明文化した。wire 型・公開シグネチャ・echo/stale guard は不変である。in-flight を持つ外部 query を attach 済み link に登録すると既存 request が cancel されるが、これは意図した意味論として doc comment と恒久テストで固定した。`registerQuery` の二重呼出による再 attach は既存 Set membership と同じく実害なしと判断し、新規の防御は追加しなかった (Issue #377)。

oracle 側は SDK と同じ採用順序に整合させた。`ContractModel.addBinding` は `setContext` を無条件に行い、`linkAttached` 時に `b.attach(linkGeneration)`→`subscribe` の順で処理する。`ContractModel.addQuery` も同型に `attach(linkGeneration)` を `linkAttached` 時に追加した (Issue #378, Issue #490)。`runScenario` は `addQuery`→`attach` の順で呼ぶため `linkAttached === false` の分岐は replay では未到達であり、既存 replay の観測 trace を変えないことがリスク評価で確認されている (Issue #490)。恒久テストは「初回 attach 後」「Dock（別 window attach）後」「detach→reattach 後」の 3 ケースで late-create binding が link generation を採用することを検査するが、assertion は「発行 generation が link epoch と一致する」「live-epoch echo reply が stale でない」という契約条件に限定し、oracle の内部手順を写さない形にした (Issue #378, Issue #490)。

## projection 受理の高水位と revision 規則

projection の受理は `ViewHost::projection_floors`（control_id ごとの受理済み revision 高水位、`HashMap<ControlId, Revision>`）で管理する。`apply_projection` は floor を advance しない projection を stale drop し、受理時にのみ floor を更新する。floor は unmount/detach/update で clear せず、終端は `ViewHost` の Drop（attach epoch の終了）のみである。支配する不変条件は「この host が一度でも受理した projection より古い revision は、以後どの binding epoch でも受理しない」である (Issue #410)。

floor を binding ではなく host レベルに置いたのは、binding が `unmount` で破棄されるため、epoch を跨ぐ状態の所有者は host が正しい境界だからである。revision の advance 判定（Num は厳密増加・Str は相違・型跨ぎは常に advance）は `revision_advances` helper に集約し、binding 内 guard（`last_observed` 比較）と host floor が同一規則を共有する。Str revision の unordered 性（相違=advance）は既存近似を継承し、契約として文書化した。`binding_mut()` 経由の直接 `on_projection` は floor を迂回するが、receipt 側の command_index 迂回と同型の上級 API 境界として契約化した (Issue #410)。

型面の制約として、`StateProjection`/`Revision` は `ruviform-ui-test` が struct literal で構築するため field 追加が当該スライスの編集範囲（`crates/ruviform-ui` のみ）では不可能であり、調査のうえ host-side floor を選択した。envelope への epoch segment 追加は wire-protocol 判断として deferred のままである。consumer が revision domain をリセットすると floor が全 projection を reject して display が凍結するリスクは、「revision は attach epoch 内で単調」の契約明記で対応した。これは binding 内 guard と同じ失敗型であり、新規リスクではない (Issue #410)。

## controlled display への authoritative 反映

受理した projection は `apply_projection`→`apply_external` の経路で controlled display（renderer 消費値 `snapshot.display`）へ authoritative 値を反映する。payload decode は厳格化し、有限 JSON number→`Plain`、`{"plain":n}`→`Plain`、`{"normalized":n}`→`Normalized` のみ受理し、文字列・null・bool・配列・複数 key・非有限値は `None` とする。業務 JSON を無条件に f64 解釈しない。`apply_external` は Plain/Normalized のみ受理し、`resolve_value_input`（range policy・snapping・domain 変換・非有限拒否を持つ既存 `ValueMapper`）で絶対値へ解決する (Issue #414)。

gesture 中の外部値は `pending_external` に stage し、`end_gesture` で flush して gesture 終了後の committed/display へ最終 authoritative 値のみ着地させる。idle 時は `committed_value` を即時更新する。intent/gesture event は発行しない（外部値は display/committed にのみ着地）。gesture 中に複数 projection が届いた場合、staging は上書き保持で最新 1 件のみ着地する。`apply_external` は availability を見ない。disabled/read-only control も authoritative state の表示には追従する。availability は user 入力の gate であり truth 反映の gate ではないからである。`register_control` では生存 binding の `last_observed` を新規 control の mapper 経由で復元適用し、detach→reattach・再登録で display が default に留まらないようにした。値 domain が変わる contract change でも新 mapper 経由で解決される (Issue #414)。

却下した代替案は 3 つある。`ControlInput::Set`（atomic pair）の再利用は begin+commit event が emit され `intent_from_gesture` 経由で intent が mint されるため、consumer への phantom command ループを生む。event 抑制を別途入れるより、event を出さない専用 write 経路の方が境界として明確と判断した。`CommandBinding` 内での display 更新は、binding が `InteractionControl` を所有しない（`ViewHost` が両者を対で持つ）ため層が崩れる。`pending_external` を `ViewHost` 側に持つ案は、gesture 状態機械と `committed_value` が control の所有であるため、「pending なら gesture active」の不変条件を一箇所で保てないとして棄却した (Issue #414)。

## コマンド台帳の有界保持と eviction

`command_index` には `MAX_COMMAND_INDEX=512` を導入し、`CommandBinding::record_ledger_state` が返す liveness class（Dead→Settled→LiveNonExpect→LiveExpect、同 class 内は command id 辞書順）で決定的 eviction を行う。全件 LiveExpect の場合は `MAX_PENDING_RECORDS` と同じく yield し、in-flight を stranded にしない。`outbox` は `MAX_OUTBOX_DISPATCHES=64` で最古から drop する。outbox は有界な dispatch 履歴であり、権威ある配送経路は host の `ViewEvent::Intent` stream である (Issue #410)。

`push_event` の 512-cap は正直な契約に修正した。coalescible を優先し、droppable がなければ最古を drop する。non-droppable も失われ得ることをコメントで明記した。drop された Intent の receipt routing は `retire_undelivered_intent` が除去し、owner binding の未解決 receipt-expecting record を `expire_command` で delivery-lost（Timeout）にする。settled record には触れず timeout を捏造しない。`expire_command` は `status.is_none() && expect_receipt` の record のみ expired を立てる。`on_tick` の timeout 条件と同じであり、非 receipt command には timeout 概念がない。index entry が既に eviction 済みの場合は、command id が binding namespace で一意なため bindings を走査して owner を特定する fallback を置いた (Issue #410)。

`validate_spec` で canonical mount の command owner id 重複を reject する。owner は control の実効 control_id または action-bearing component の key であり、tab alias 経由の同一 node は key.0 が同一なので重複とみなさない。`key_for_control` は同一 control_id を持つ mounted entry が複数ある場合に canonical mount を優先し、alias は辞書順最小を fallback として detach/unmount の帰属を決定的にする。`invoke`/`emit_selection` の binding key は `owner_key_for` で canonical owner id に正規化する。tab alias 経由で mount された action component が alias キーで command を mint し、dedup 空間と release の対象から外れる穴を review で検出して塞いだ。単調 u64 カウンタ（`InteractionControl::sequence`/`atomic_counter`、`InteractionDispatcher::synthetic_seq`、`CommandBinding::counter`/`observed_seq`/`generation`、`ViewHost::generation`/`mount_epoch`/`action_sequence`）は `checked_add(1).expect()` 化した。u64 枯渇は到達不能だが wrap は順序を静かに壊すため正直な panic とし、`# Panics` 節を追加した (Issue #410)。

## descriptor 更新・選択モデル・楽観的選択

`unregister_control` を `detach_control`（gesture のみ解放）と release 用に分離し、contract change（descriptor 更新）には `detach_control` を使う設計とした。これにより descriptor 更新で in-flight receipt の routing が切れない。失敗テスト `descriptor_update_keeps_in_flight_receipt_routing` を先に配置し、removal 側の stale receipt 保護（`update_reregister_keeps_binding_epoch` / `remount_absorbs_stale_receipts`）は既存のまま維持されることを確認した (Issue #451)。

Tabs の選択モデルは Table と同じ経路に揃えた。`TabsSpec.selection`/`TabsSelectionSpec`/`TAB_SELECT_ACTION`/`ViewHost::select_tab`/`emit_selection` 共有化/`carried_selection`/`clamp_selection` を追加し、表示タブを `ComponentSnapshot.selected` に寄せた。非選択 Tabs は常に `Some(active)` を返すと doc に明記し、Table の挙動は不変である (Issue #451)。

`CommandIntent::Action` には `params` を `serde(default, skip_serializing_if)` で additive に追加した。Action はそれまでどこからも発行されておらず既存 JSON も変わらないため、破壊的変更にならない。`TableSelectionSpec`/`ButtonSpec`/`ComponentSpec::Button`/`A11yRole::Button`/`TABLE_SELECT_ACTION` を追加し、`ViewHost::select_row`/`invoke` と `emit_intent` の gesture/action 共有化、`selection_undo` による receipt 連動 revert を実装した。Rejected/Unsupported receipt で直前の選択へ戻し、Accepted/Applied は phase で進捗を示す楽観的選択モデルである。undo 記録は receipt の来ない command で溜まるため削除・detach・unmount で解放し、consumer は Action に terminal receipt を返す前提（`expect_receipt`）とした。行選択は行 index ベースで、row identity・複数選択・キーボード操作はスコープ外である (Issue #448)。

## ウィジェットのレイアウト契約

canvas の `size_full` は割合指定なので親の定義済み寸法に依存する。Seek は `h(24)`、Fader は `w(32)` を wrapper が持つことを回帰テストで確認する設計とした。Waveform/Meter/TimeAxis/Spectrogram の canvas サイズ規則はこの Issue の範囲外として後続の監査に送った (Issue #450)。

サイズ規則の監査では、各 paint canvas の prepaint で `record_bounds` により bounds を記録する監査テストを先に失敗させて実測値を取った。是正として Waveform/Meter/Spectrogram の slot を `viz_slot` に統一し、Waveform の余分な wrapper を除去し、TimeAxis slot に `flex_shrink_0` を付けた。`flex_shrink_0` がないと ruler の 24px 検証が失敗することを確認している。Panel の溢れで viz slot が縮むのを slot の `min_h(120)` で防止し、監査テストは溢れる構成（Fader の `h_full` 付き）でマウントする。実窓の before/after は render.rs を HEAD 版に一時的に戻して撮影し、修正版をバックアップから復元して差分一致を確認する手法を使った (Issue #453)。

Panel 子としての Fader/Knob の縦寸法は、現状維持・固定高さ・viz_slot 型 min_h+grow・ViewSpec 契約追加の選択肢を評価し、ADR Option C（`flex_grow` + `min_h` slot）を採用した。実装は `element_for` の Fader|Knob 分岐と `control_element` 非 Seek 経路を grow+min_h slot 契約に改め、`grow_slot` helper（flex_col+flex_grow+min_h(px(120))）に集約して `viz_slot` もこれに委譲した。`h_full` ラップは除去し `.size_full()` は維持する。高さ所有権は slot 側にあり、canvas bounds = slot bounds で #450 契約を維持する。min_h は viz_slot と同じ 120px に一本化し、control 専用の別値は同一ポリシーの重複になるため置かない。Seek 経路 h(24px)・キーボード規約・driver-neutral 契約は不変である。極端に低い Panel では min_h 合計超過で依然溢れる点、Tabs body の `size_full`、orientation sizing、Panel overflow は Deferred Design Concerns として範囲外に切り分けた (Issue #478)。

## キーボード操作とフォーカス巡回

値制御のキーボード操作では、PageUp/PageDown の移動量を descriptor の絶対量ではなく effective step の 10 倍という相対量で一貫させた。絶対量方式は renderer が `effective_step` を複製する必要があるため却下した。値制御は一律 tab stop とし、`press_control_key` は mounted spec に control がある場合のみ focus handle を作り、なければ `UiError::Host` を返す。契約（`ruviform-ui`）は変更せず GPUI 描画層と seam に閉じた (Issue #479)。

フォーカス巡回では、gpui の `track_focus` が要素側 `tab_stop` を伝播しないため、`FocusHandle` 生成時に `.tab_stop(true)` が必須である。Tab/Shift-Tab 巡回は `cx.observe_keystrokes` を drain 先頭で一度だけ登録する `wire_focus_navigation` で実装し、`Inner.focus_nav_wired` フラグと `app_subs` で寿命を管理する。無修飾は `focus_next`、shift のみは `focus_prev`、その他修飾は無視する。keystroke observer は無フォーカス時も発火する（dispatch path = root ノード）が、要素が propagation を止めた場合は発火しない。これは意図した優先順位である。タブヘッダには `track_focus`/`tab_stop`/`on_key_down` と mouse-down での `window.focus` を付与した (Issue #451)。

実 OS の Shift+Tab が gpui プラットフォーム層で `key="tab", modifiers.shift` に正規化されるかは実機依存であり、headless は `Keystroke::parse("shift-tab")` の同形で検証し、実機検証は OS 実入力 lane の後続に委譲した (Issue #451)。

## GPUI paint の意味論と repaint コスト

O(n²) repaint の原因は、各 quad が個別に `BoundsTree::insert`→`find_max_ordering` を通ることだった。`window.paint_layer(bounds, ...)` で要素ごとの quad 列を 1 layer にまとめ order を共有して解消し、固定の背景 quad は layer の外に描画する。gpui 0.2.2 の `paint_layer` は layer bounds を `bounds ∩ content_mask` に clip するが primitive の content_mask を変更しないため、従来の clip 範囲と差が出ないことを確認した。layer の order 共有下でも `Scene::finish` の stable sort が同一 order 内の挿入順を維持するため、cell の 1px clamp の重なり順は保存される。Fader/Knob/Seek/NativeSlot は quad 数が固定で spec サイズに比例しないため対象外と監査結果に記録した (Issue #461)。

性能 budget は 500ms とした。修正後実測の最大約 55ms の約 10 倍で、O(n²) 回帰（秒オーダー）との識別を優先し、低速環境での偽陽性を避ける値である。TimeAxis の repaint 約 0.8s は tick ごとの label `div`+text 描画が支配的で canvas quad の問題ではないため budget 対象外とし、element tree 規模の別問題として扱った。後続の対応では、label コストの支配項が per-line `paint_layer`→`BoundsTree::insert` であることを特定し、label ink union の layer bounds にまとめて sibling との描画順を保存した。style は `window.text_style()`+`text_dim` 色で旧 div と同じ解決を行い、glyph 配置は gpui `paint_line` の単一 style 経路と同じ式を再現した。headless `TestAppContext` は Scene 構築の全経路を通すため O(n²) 回帰は検知可能と判断し、実機の体感確認は consumer 側の pin 更新後に委ねた。Table の行×列 div 生成も同種の element tree 規模コストであり、別 Issue 候補として記録した (Issue #461)。

Spectrogram cell の描画色は、`viz.waveform.opacity(intensity)` を `surface.bg` 上へ不透明事前合成する色（`cell_paint_color` helper、Rgba source-over→不透明 Rgba）に変更した。原因は列 frame 共有による alpha 累積飽和であり、`paint_layer` 案は棄却した。cell 重なり時の blend が「後勝ち上書き」に変わる既知のトレードオフがあるが、期待色と一致させる意図した修正である。実窓 capture は gpui-test が headless で pixel を返さないため、専用 example 実行+OS 画面 capture（PowerShell 等）で代替する手法を記録した (Issue #463)。

## 描画 policy の所在と parity fixture

Meter/TimeAxis の描画 policy（gap の扱い・clamp・cull 等）は driver 固有の表示規則であり、headless 契約に入れると GPUI 型を持たない共通層に描画 policy が漏れるため共通層には入れない。共通層の変更は consumer（AudioLens/Synvelis）の既存 ViewSpec 構築に影響しうるため、この Issue の範囲では必要がないという判断でもある。schema は変更せず、`ruviform-ui` に新しい描画 API も足さない。描画規則は、Meter の RMS が非有限なら peak バーのみ、peak/RMS は amplitude として [0,1] に clamp、空区間・逆順区間・viewport と完全に重ならない bin は描画しない、TimeAxis は viewport 外 tick を cull して major は全高・minor は半高、とした。viewport 外の bin/tick は payload に保持し描画だけ省く（headless 側は payload 保持検査、GPUI 側は plan 検査で両側から確認）(Issue #421)。

parity fixture は 1 つの JSON を両 driver が読む。`view`（ComponentSpec の serde 形式 ViewSpec: Toggle・Knob・Meter・TimeAxis を含む Panel）と `expect`（mount されるキー・Knob/Toggle の intent 列・Toggle の committed 値・Meter の宣言 bin 数と drawn bar・TimeAxis の宣言 tick 数と drawn mark）の 2 部から成る。Fake 側は mount/reconcile・gesture intent 列・committed 値・宣言 payload 保持（gap bin・viewport 外 tick を含む）を検査し、GPUI 側は `meter_bar_plan`/`axis_mark_plan` が expect の drawn 値と一致すること・実 pointer 入力による intent 列・RvfView 上での reconcile を検査する。NaN は JSON で表現できないため gap を `count==0` の有限値で表し、NaN を gap として扱う規則は Rust 単体テストで検査する。Toggle の atomic set は契約どおり begin+commit の 2 intent を発行する (Issue #421)。

driver-lab manifest の status は実行結果だけで決める規律とした。GPUI の新 kind mount/reconcile は gpui-test 実行で pass、Fake は driver_parity の 2 テストで pass とした一方、Cancel と遅延・順序逆転 receipt が未接続の GPUI lane は not_run のまま note を実態に合わせた。stories.json は変更しない（scale factor 2 種と状態遷移の GPUI 実行が未実施のため story の status を pass にできない）。status の過大申告は driver-lab の意味が崩れるため、pass は実行されたテストに限定する (Issue #421)。

## Spectrogram tile・Table・CellValue の契約

`Spectrogram::tile` の列区間式は `column_offsets` 単一実装に集約し、`Tile::columns`/`TileColumn` を追加して consumer への式複製を排除した。`SpectrogramColumn::count` の意味は有限セル数（0..=rows）と doc を修正し、型と validation は不変である。契約テスト 4 件（分割・cols>span・NaN 列・有限セル数）を追加し、この変更は PR #445 として merge 済みである。`cols > span` で列が重なることは契約として doc に明記し、consumer が重なりを分割と誤認しないようにした。空 frame 範囲での `Spectrogram::tile` 添字参照は既存の panic 可能性として報告し、Issue #446 で修正済みである。`ruviform-ui` 側の `count <= rows` validation 要否は follow-up 未対応のままである (Issue #443)。

Spectrogram 関連の follow-up は範囲外に分離した。AudioLens 側の Spectrogram 射影、ruviform-viz の `Tile` から `SpectrogramColumn` への consumer 側写し替え、専用 theme token（現状は `viz.waveform` の alpha 流用）、JSON 上の非有限値表現はいずれも別作業である。`ComponentSpec` は `#[non_exhaustive]` のため variant 追加は破壊的変更にならず、consumer は `other =>` arm を維持する前提である (Issue #434)。

Table の platform a11y（OS accessibility tree 接続）は gpui 0.2.2 で素直に使えず Unsupported とした。renderer は a11y を描画に使っていないため、drawn grid と contract の a11y 木の一致を検査して代替証拠とし、OS 検証は別 Gate に分離した。大量行の仮想化・スクロールは非スコープで、上限は固定値・超過は mount 時に拒否する。`CellValue::Display` を唯一の表現とし両 renderer が同じ関数を使う設計で、renderer 間の表示不一致を防いだ。`#[non_exhaustive]` enum への variant 追加は下流の exhaustive match を壊しうるが、下流は `_` arm 前提であり consumer 側変更は別 PR で扱う (Issue #435)。

## driver-neutral Scenario Contract

`ruviform-ui-test` に driver-neutral な `SemanticGuiTest` trait・Target・Capability・Result/Report を置き、Fake adapter と GPUI adapter が同一契約で同じ Scenario を実行できるようにした。依存方向は `ruviform-ui-test` が `ruviform-ui-gpui` に依存しない形で固定し、逆転を禁じる。GPUI adapter は `gpui-test` feature 限定とした。wire `Event` を `ViewHost` の既存 event 処理へ decode する正本は `ruviform-ui` の `deliver_event` であり、両 adapter がこれを共有して decode の二重化を防いだ。`GpuiObserver` の `!Send` 制約は維持する (Issue #464)。

scenario runner は executor 化せず、`run_scenario` の body を `FnOnce` とし、scenario は普通の Rust 関数のままにした。`ScenarioReport` への `timeout`/`cleanup` field の追加は additive に行い、timeout 矯正は Pass→Fail のみに限定して他の verdict を書き換えない。`settle_until` の `ready` 述語は `&G` 経由の read-only 観測のみを許し、状態を書き換える待機条件は契約上書けない (Issue #464)。

設計の上位層では、後継 ADR が Driver-neutral 構造正本（GPUI driver + webview-first）を確立し、旧 2 ADR は superseded として履歴保持した。GPUI 技術採用の最終承認は関連 Issue 群の実機証拠を待つ条件付きとし、ADR には逆反映欄を確保した (RF384/RF385)。

## ruviform-ui-test の構成と neutral 語彙

`ruviform-ui-test` crate は `rvf.uitest.v1` ops over 再利用の `rvf.ipc.v1` envelopes、`TestIo` port、`FakeHost`/`HeadlessIo`、stdio+loopback-TCP transports、`TestDriver`、`dummy_app` を実装する。全コードを新規 crate に閉じ、production crate 変更ゼロの isolation を構造で担保した。`TestIo` は headless 固有でなく `DriverEvent`/`WindowDriver`/`WindowSpec` の neutral 語彙へ進化させ、単一 `read_frame`/`write_frame` を stdio/TCP 両経路で共有して codec 乖離を防ぐ (Issue #312)。

`Service`/`UiRuntime` が非 `Send` のため `app.shutdown` は `UiJob` に capture できず、`pending_shutdown`+`UiSide::deliver` 内処理という設計判断を採用した。`FakeHost` は close を auto-approve せず、close policy は app 側の責務とする。loopback TCP の secret 生成は RandomState+time+pid で loopback test threat model に十分とし、run 間で再利用しないことを ADR に明記した。`CARGO_BIN_EXE_dummy_app` が使えない場合は `cargo run` 経路へフォールバックする計画である (Issue #312)。

## windowed uitest と UiJob seam

`rvf.uitest.v1` closed op set を実 GPUI window に配線するため、legacy host 語彙（WindowEvent/WindowHandle/WindowConfig/FakeHost）を neutral driver 語彙（DriverEvent/WindowDriver/WindowSpec/FakeDriver）へ全面移行する Option A を採用し、legacy shim は不採用とした。`GpuiDriver` は `Rc<RefCell>` で非 `Send` のため、control thread（serve loop）と UI thread を `UiJob` seam（`Send` クロージャを `mpsc` 経由で `Inner.ui_jobs` に積み、`drain` が window ops 適用後に `App` context 内で実行）で橋渡しし、全 op は job+oneshot reply で app thread を跨ぐ。`TestApp` は `Send` 保証がないため `UiSide` は app を保持せず、shutdown hook は delivery path 内の interceptor が consume する。`UiJob` の cfg は `any(real-scenario, uitest)` に widen した (Issue #313, Issue #314)。

`emit` は `Tick`/`Action`/`Focused`/`CloseRequested` のみ honest 注入とし、他 lifecycle は `emit.unsupported` notice で observably drop する。windowed で `Created`/`Ready`/`Resized`/`Moved`/`ScaleFactorChanged`/`Destroyed` の注入も同じく drop+notice であり、windowed AUT fixture は headless の `emit(Ready)` 慣習を真似ず real Ready を bounded poll する。`ipc_disconnect` は per-window 限定で `window=None` は `invalid_params` とする。action-job+confirm-job の決定論的順序付けにより、confirm job は action を apply した drain の次に着地する。`respond_close`/`apply_destroy` は bookkeeping を同期適用するが derived `DriverEvent` は次の `flush_events` でしか届かないため、confirm job 内で自前 `flush_events` を呼び、caller 復帰前に receipt/notice/destroyed を確定させた (Issue #313)。

実走で得た教訓が 2 つある。1 つ目は stdio transport の frame cap が 1 MiB で、800×600 級 RGBA→base64 の screenshot reply が cap 超過→serve が DriverGone で session 終了→AUT 死亡→後続 op 全て Disconnected になったことである。対策として adapter は encode 後に `MAX_INTENT_BYTES` を pre-flight し、超過時は service_error で op のみ失敗させ channel を生かす。2 つ目は notice 観測である。agent は毎コマンド後に `io.notices()` を drain して reply に `Notify` として同梱するため、`test.notices` op は常に `[]` を返す。lane の notice 検証は `TestDriver::notifications()`（buffered `Notify` payload）で行う。初回実走で `d.notices()` が常に空を返し `notice: false` と誤判定したことから修正した (Issue #313)。

## OS 実入力注入の証拠設計

`os_input` に `OsInputOutcome`（`Injected`/`Unsupported`/`EnvironmentBlocked`/`NotRun`、public `#[non_exhaustive]`）を追加し、`supported_capabilities()` は `os_input::supported()`（現状 Windows のみ）条件で `RealOsPointerOrKeyboard` を出す。`os_preflight` cache+`window_focused`（`Focused(active)` を `Arc<Mutex<BTreeMap<WindowId,bool>>>` に記録）+`request_focus`/`os_press_key`/`os_pointer_move_to`/`os_mouse_down`/`os_mouse_up` を実装した。Windows は preflight→focus→`run_scenario` で keyboard/pointer ops→intents→receipt→observe→verdict JSON（`RUVIFORM_REAL_OS_INPUT_VERDICT`）+driver_lab emit とし、非 Windows は窓を開かず verdict=unsupported・exit 2 で証拠を出す。exit 2 は fail(1) と区別され、未実行を失敗と偽らない (Issue #313)。

OS 注入 op は `RealOsGuiTest` の並行 port（`os_*` メソッド群）に集約し、`SemanticGuiTest`/`GuiAction` を汚さない。semantic dispatch（`ViewHost::dispatch_input`）・toolkit simulated input（`dispatch_keystroke`/`simulate_*`）・OS automation injection（`SendInput`）を証拠上分離するためで、戻り値は `OsInputOutcome` 語彙で返し、`Err(ScenarioError)` は contract 失敗のみとする。`SendInput` は OS automation injection であり物理手動入力ではないことを verdict JSON の `input_method`/`input_method_note` に明記した (Issue #313)。

pointer の照準は `screen = origin + (logical + frac*size) * scale` で実行時計測値のみから解決する。`component_bounds`（最後の paint 記録の論理 px、window client 原点相対）+`window_geometry`（`GetClientRect`+`ClientToScreen` の実測物理 px）+scale factor を使い、座標ハードコード・窓中央仮定・capture pixel 逆算は全て却下した。bounds 欠損・degenerate は `NotRun` とする。`os_input` の move は `MOUSEEVENTF_MOVE|ABSOLUTE|VIRTUALDESK` を `GetSystemMetrics` virtual screen で正規化し（負原点対応）、`send` 共通ヘルパーで受付数不足を `UiError::Host` とする（部分注入を成功にしない）。pointer move は注入後 `cursor_position`（`GetCursorPos`）で read-back し、commanded ±4px に収まることを check 化した。「注入を受け付けた」ではなく「カーソルが実際にそこへ行った」証拠とするためである (Issue #313)。

gesture の合格条件は `Begin+Commit` と `gesture_id` 一致とした。`Preview` は contract 上 coalescible で正当に 0 件になり得るため合格条件にせず件数のみ記録する。`source` は intent に載らない（`CommandIntent` Begin/Commit は `source` field を持たず `InputSource::KEYBOARD` は `ControlInput`/`InteractionEvent` meta まで）ため、keyboard 由来の保証は「OS キーイベントのみがこの経路に到達する」構造的事実と `atomic: true` の begin/commit pair で代替した (Issue #313)。

注入は UI thread の外から行う。`GpuiDriver` が `Rc<RefCell>` で `Send` でないため injector thread から `os_input::press_key` を呼び、UI thread は `Ready`+`Focused(true)` を観測してから `Go` を送り、イベント記録と verdict 判定のみ行う境界分離である。verdict 確定時に injector へ `Abort` を送り、以後のキーストロークが別 window に漏れないようにした。Win32 `SendInput` は foreground window にのみ届き、実行中にユーザーが別 window へ focus を移す・マウスを動かすと Fail になり得るため、manual lane の運用注意として明記した。IME/secure desktop 下でも阻害され得る (Issue #313)。

運用上の失敗分類は正直に記録する規律である。`Focused(true)` 後も click-to-focus で初回 click が吸収される OS 設定に対し focus ゲート+`FOCUS_SETTLE` で緩和し、残れば Fail として記録する。フォーカス観測未達なら ops は `NotRun` を返し lane は fail し、timeout 記録を保持して Pass 偽装しない。カーソル read-back 逸脱（`SendInput` 受理後に OS 側で補正）は check で Fail とし、カーソル復元失敗は best-effort+stderr 記録で verdict 不変とする。`SendInput` が環境要因で遮断される過去事例（原因未同定）があるため、preflight gate が `environment_blocked` を返したら深追いせず証拠記録で仕上げる。実 GUI lane は実カーソル/キーボードを占有するため unattended 実行前提である (Issue #313)。

実装期のバグとして、receipt leg が最初の `expect_receipt` commit（keyboard 側）へ receipt を投げていた問題があった。`phase()` は head record（最新の receipt-expecting command）を読む契約のため、`.rev().find` で最新の pointer-click commit へ修正した。初回実走で Pending が観測され、inbox probe（1→0 drain 確認）と head 契約から確定した。receipt は「最初に見つかった expect_receipt commit」ではなく「最新の head record」へ送るという契約上の教訓である (Issue #313)。

実窓での pointer 検証は、Windows のフォーカス制約でフォーカス API 経由の実窓クリックが失敗し得るため、OS 実ポインタ（`mouse_event`）でクリックする方式を採用した。実窓証拠は手動で再現性がないため、自動化された pointer テスト（`TestAppContext`）で同じ経路を検査するものを正本とし、実窓は補助証拠と明記する運用である (Issue #451)。

## uitest lane の失敗分類と正直な verdict

「Build/Link 成功 ≠ Driver 機能対応」の規律を徹底した。CI の `windowed_uitest` env-gated no-op や macOS の cfg 空 test target は compile/link のみで実行されないため、capability 記録では Pass にせず `NotRun` とした。`WindowedIo::JOB_TIMEOUT` は `ruviform-ui-gpui` 側で編集範囲外のため対象外とし、Timeout 検証は driver 実装を変更せず既存 public API（`call`/`wait_exit`/`spawn_tcp`/`kill`/`try_exit_status`）のみで行った (Issue #313)。

`--stall before-serve|after-serve` の非終了 fixture は、それぞれ `serve_from_env` 前と serve 終了後（全 outcome で）に永久 park し、TCP `CONNECT_TIMEOUT` accept loop と `wait_exit` の non-exit 契約を実演する。`d.shutdown()` は内部で固定 `EXIT_TIMEOUT`(15s) を待ち report を捨てるため、`test.shutdown` は `d.call(ops::SHUTDOWN)` で `bye:true`+`cleanup.exited:true` を確認後、caller bound の `wait_exit(500ms)` で同一契約を短時間 assert する形にした。未知 phase・引数不足は exit 2 で fail-closed である (Issue #313)。

`cargo test` は環境不通を通常の nonzero exit と区別できないため、`environment_blocked` は driver-lab JSONL の `observed` フィールドと panic message の `environment_blocked:` prefix の 2 点で記録する。real-scenario lane の session 起動時 `UiError::Host`/`BackendUnavailable` のみが対象で、`Internal`/`Unsupported`/`ScenarioError`/assertion は通常の test failure のままである。headless 環境で実行しても「未実行を pass と見せない」境界を維持する設計である (Issue #314)。

既存の flake・ノイズは修正せず記録のみとした。`ruviform-plugin-host` の `hang_converges_on_command_timeout`（`SpawnFailed{Timeout}`）は並列 build/test 負荷下で 2 連続 fail するが単独では 3/3 pass であり、当該 slice の diff はその crate に無接触なため並列負荷下の既存 spawn-timeout flake と判定した。real-window セッション teardown で `RefCell already borrowed` と HRESULT 0x80070578（invalid window handle）が ERROR ログに出る現象は、変更していない既存 lane でも同じく出るため main 由来の既存現象として別 Issue 候補に記録した (Issue #313)。

## real-scenario lane と theme×scale matrix

`real-scenario` feature と `RealOsGuiTest`（caller スレッド façade+UI スレッド job pump）を実装した。`gpui/test-support` は `real-scenario` feature に含めない。test platform は OS capture を stub するため pixel 証拠を名乗れないからである。capability は `SemanticAction`+`MultiWindow` を常時、`VisualCapture` は Windows のみ claim し、`RealOsPointerOrKeyboard` は claim しない（OS-level input は別 Issue の scope）。`dispatch_input` は semantic dispatch であり pointer/keyboard 証拠を名乗らない。canonical mise lane `test-ui-real-scenario` を追加した (Issue #314)。

`{theme × scale}` 4 cell matrix（`synvelis-dark`/`audiolens-dark` × 1.0/2.0）で cell ごとに独立した real-window session を起こす。theme/scale は `ViewMount.theme` に `ResolvedTheme::with_scale(ScaleContext{scale_factor, rem_base_px:16.0})` を渡し、scale は mapper-side hint として token 不変を cell ごとに assert する（Fake/GPUI-simulated matrix と同一規則）。`GoldenKey.scale_factor` は要求値でなく `CaptureMeta` の実測値を使う。`RUVIFORM_REAL_WINDOW_PNG_DIR` の相対 path は workspace root 基準で解決し、cell 間の PNG 上書きを防ぐ。`#![cfg(all(windows, feature="real-scenario"))]` は維持し、非 Windows の vacuous pass を作り込まない (Issue #314)。

## golden 比較基盤と semantic oracle

`ruviform-ui-test` の Golden Utility コアは `GoldenKey`（story/theme/state/driver_class/os_class/gpu_class/scale_factor/font_class/revision）で baseline を一意化し、`EnvironmentMeta`/`GpuMeta`/`CapturePath`（`#[non_exhaustive]`）で環境差を key に織り込む。`RgbaImage` は検証付き構築・`from_png`/`to_png`（`normalize_to_color8`+RGBA8 正規化+次元上限）・`from_screenshot`（png/rgba-raw のみ受理）を持つ。`DiffPolicy`（`ChannelTolerance`/`MaskRect`）は純粋関数 diff で統計+差分画像（差分=赤・masked=減光）を返し、alpha=0 両側一致規則を採用する。`BaselineStore` は layout・sidecar 検証・`write_candidate`/`promote`/`compare`→`GoldenReport` を担い、`GoldenEvidence` JSONL（`RUVIFORM_GOLDEN_EVIDENCE`）で証拠を append する (Issue #314)。

`VisualGuiTest`（`SemanticGuiTest` 継承）は `CaptureOutcome`・`golden_check` 既定実装・`record_golden_step`・`From<GoldenVerdict> for ScenarioVerdict` を定義する。Fake/GPUI の `capture_frame` は pin 再検査後に正直な `Unsupported` outcome を返し、`golden_check` が unsupported verdict の `GoldenComparison`+evidence emit を行う。`GoldenVerdict`→`ScenarioVerdict`・`lane_status`（blocked→not_run）の写像を固定し、capability 宣言・preflight・verdict 語彙・evidence 記録の Fake/GPUI 一致を contract/parity tests で検証した。pixel 証拠を名乗れない経路が pass に昇格しない capability 分離を維持する (Issue #314)。

OS capture backend は `OsCapturedFrame`（RGBA8+scale_factor+backend tag）と `GpuiDriver::capture_window`（slot 検査→OS handle→backend 委譲）で構成する。Windows backend は `PrintWindow` primary・screen `BitBlt` fallback で、32bpp top-down DIB→BGRA→RGBA 変換を行う。未知・未 open window は `UiError::Host`、OS handle なしは `UiError::Unsupported`（pixel 捏造なし）、非 Windows は `Unsupported` stub である。経路は `OsCapturedFrame::backend` に記録される。環境正規化は `GoldenKeyEnv`/`key_env`（EnvironmentMeta→env class）・`key_for_fixture`（StoryFixture 結合の GoldenKey 構築）・`registry_mismatches`/`RegistryMismatch`（StoryManifest 整合検査）が担い、`GoldenEvidence` の `lane` field（`pixel-golden`）で semantic/plan/pixel/OS-input の lane を混同しない (Issue #314)。

semantic oracle fixture は consumer 形状の 2 件（AudioLens 形状・Synvelis 形状）を driver-lab parity fixture set に追加した。Waveform（f64・frames=5_000_002_000・partial count=173・gap count=0・viewport 外 bin）+TimeAxis（fractional tick）+Seek（domain [0,200] committed）、および Meter（partial count=37・gap・over-range peak=1.7/rms=1.2・viewport clamp）+Spectrogram（gap column・magnitude_range 外・clamp）をカバーする。Fake lane は mount 前 payload の verbatim 保持（f64 `to_bits` bit-exact）を検査し、GPUI lane は `GpuiObserver` seam 経由で `DrawnPlan` が `expect.*.drawn*` と一致することを検査する。serde_json/`str::parse` の丸め差は fixture 小数選定と文書で対処した。`DrawnPlan` pass が pixel golden pass に昇格しない lane 分離を維持する (Issue #314)。

## consumer 観測 seam と DrawnPlan DTO

`drawn_plans(&ComponentSpec)` は `element_for` と同じ走査（Panel 全子・Tabs active のみ）を共有し、Waveform の gap 判定を `waveform_bin_drawn` として paint closure から抽出して paint と観測で共有した（挙動不変リファクタ）。`MeterBarPlan`/`AxisMarkPlan` の field を `pub(crate)` 化して `DrawPlan` 化した。公開 DTO の `ViewDrawn`/`ComponentDrawn`/`DrawnPlan`/`DrawnBar`/`DrawnMark`/`DrawnWaveformChannel` は `#[non_exhaustive]` で f64/usize/String/ComponentKey/Range<u64> のみを持ち gpui 型を含まないため、`check-ui-driver-boundary` を通過する (Issue #436)。

consumer 型テストの置き場は外部 test crate `crates/ruviform-ui-gpui/tests/consumer_observation.rs` とした。受入「consumer 側 test から drawnBars/drawnMarks を観測」は、AudioLens での実施（pin 更新を伴う別作業）ではなく ruviform 内の consumer 型テストで代替検証した。同ファイルは公開 `GpuiObserver` seam と `ruviform-ui` 契約型のみを使い、gpui 型・crate 内 hook を参照しない。`ruviform-ui-test` への GPUI dev-dep 追加は driverless build（UI-DVR-01）と MSRV 1.89 lane を壊すため見送り、cases.json への binding 追加も manifest `requiredCases` 完全性 pin に波及するため範囲外とした (Issue #436)。

consumer 向けの観測 seam は一度は要求されたが、この段階では公開しない判断をした。既存の `settle_for_test`/`inbox_len_for_test` は `#[cfg(all(test, feature="gpui-test"))]` の `pub(crate)` で `gpui::TestAppContext` を引数に取り、そのまま公開すると consumer に gpui の test-support 依存を強制する。consumer 向け seam は gpui 型を含まない観測 DTO（projection snapshot と intent 列）として設計する必要があり、公開 feature の追加は Cargo.toml と boundary guard の双方に影響する設計判断で当該 Issue の範囲を超えるため、follow-up Issue として観測 DTO と feature 境界を設計することにした (Issue #421)。

## 実ブラウザ harness の決定性設計

`ui/browser-test`（`@ruviform/ui-browser-test`、private）は Playwright 実ブラウザ上で Showcase 実 DOM の Gesture/Receipt/Theme/Viz/a11y/lifecycle を検証する共有 harness である。`page.route` による hermetic 静的 serve（origin `http://rvf.test`、`/pages`・`/showcase/dist`・`/browser-test/dist` マッピング、MIME・`..` 拒否）と、ページ内 `window.__rvf` protocol（events/commands/receipts/drain/queries/drawAll/stats/prop/setProp/mount/unmount/clear/ready）で構成し、`window.__rvf` page API の型契約を Node/page で共有する。import map で `@rvf/showcase`→showcase dist、`@rvf/browser-test-page`→browser-test dist を解決する (Issue #310)。

決定性の要は `page.clock` と、rAF/RO との干渉を避けるため viz を `__rvf.drawAll()` 同期 tick に限定する点である。Playwright config は chromium 既定・固定 viewport {960,640}+`deviceScaleFactor:1`・trace retain-on-failure・screenshot only-on-failure とした。WebGL2 は headless Chromium の SwiftShader で取得可能な見込みだが、未取得環境では「capability なし」を証拠化する設計である。`@playwright/test` は dev+peer 依存に閉じ、独自 DSL は禁止する (Issue #310)。

Node 側 utility として、`waitFor`（expect.poll 相当・固定 sleep を内部で禁止）、`expectGestureSequence`（begin→preview{0..N}→終端 exactly-one・同一 gestureId・sequence 単調）、`expectComposedInit`（detail/bubbles/composed 検査）、receipt 遷移検査、`canvasFillProbe`（fill ratio 行列）・`expectGapRegion`（NaN gap 中央帯 fill 低下）・`expectBackendRecorded`、`test.extend` fixture（page.clock install+harness open）を提供する。consumer が通常の Playwright API と併用する設計である (Issue #310)。

## theme 解決パイプラインとコード生成

`ruviform-ui-theme` crate で deterministic な DTCG→typed `ResolvedTheme` パイプラインを実装した。依存は serde/serde_json のみで、12 fixtures+18 contract tests で検証した。`ResolveError::Document` は将来の複合 helper 用に保持し、GPUI mapper 消費契約の確定形は別 Issue に委譲した (RF384/RF385)。

`ResolvedTheme::new` が `pub(crate)` のため生成コードは同一 crate 内 module として emit する必要があり、新規 workspace crate が task 制約で不可だったため in-crate bin `theme-codegen`（src/bin/）+`src/generated.rs` の組合せを選択した。emitter が rustfmt canonical 形を手計算で再現するのは脆弱なため、生成テキストは rustfmt の stdin/stdout 子プロセスで正規化する。rustfmt 不在は明示エラーとし、決定性のためフォールバックなしとした。resolve 規則の第二実装は持たず、意味解釈の正本は runtime pipeline のみである。manifest は `serde_json::Value` で strict parse し（未知 key・型不一致・空配列はエラー）、document path は manifest ファイルのある dir 相対で解決し、`ThemeResolver::resolve` の結果だけを emit する。token path→識別子は ASCII 英数字 snake_case 正規化+先頭数字 n 前置+keyword 末尾 `_` で、衝突・予約語・module 名衝突は決定的エラーとする。`#[non_exhaustive]` enum の未知 variant は明示エラーで fail-closed である (Issue #418)。

fixture は当初の 2 product theme（audiolens-dark/synvelis-dark）が color/dimension のみで FontFamily 等の emitter 経路が未検証だったため、全 variant を網羅する crate 内 fixture `rvf-primitive` を第 3 theme として追加し、生成コードの全 variant コンパイルと runtime 一致を end-to-end で担保した。f64 は小数部を左 3 桁区切り・整数部/指数部と u64 fingerprint を右 3 桁区切りで emit し、値は不変である。`#[allow(clippy::too_many_lines)]` は生成 `new()` のみに局所付与し、`struct_field_names` 回避で field 名 theme→resolved に改名した。rustfmt の version 差で生成 bytes が変わり得るが、toolchain は mise.toml で pin されており、toolchain 更新時は `mise run codegen-theme` で再生成する（`--check` が差分を検出する）。manifest に新規 `ResolvedValue` variant や新文書が入ると codegen が明示エラーで停止する。意図した fail-closed であり、対応は emitter 拡張として行う (Issue #418)。

## ICC fixture と色変換テスト

moxcms 0.9.1 の `encode()` が lut8Type（mft1）に lut16Type 専用の 4-byte entry-count を混入し、spec 準拠 reader（moxcms 自身を含む）が table 開始を 4 byte 早く誤読することを実装中に発見した。upstream 修正は依存 bump を伴うため scope 外と裁定し、generator 内の `fix_mft1_tags`（ICC layout を直接走査して mft1 body を圧縮・directory offset/size・header total size を再配置）に封じ込めた。committed fixture byte は正しい CMM の出力と一致し、workaround の存在・理由・pin 版 bump 時の除去判断は関数 doc と test-spec 再生成規約に記録した (Issue #125)。

fixture は合成 `ColorProfile` で作る。`ColorProfile::default()`+pub field 設定+`lut_*` tag 装着→`encode()`→`new_from_slice` round-trip が成立し `create_transform_f32` が LUT 経路を選択することを、fake success なしに fixture 再生成の実行で実確認する。生成器は書き出し直後に `new_from_slice` で自ら受理確認する構造とし、mft1/mft2 は単一 `mft_lut` helper に統合した (Issue #125)。

テスト設計は oracle bit-exact+行為断言である。intent 区別・tag 型別の観測差を `Rgba8` 量子化後に確実に残すため、×0.5/×0.75 scale・R↔B swap・channel rotation の大きい線形写像を選択し、literal golden は増やさず raw executor oracle 比較+行為断言に統一した（全 build 構成で動く）。Perceptual 時のみ掛かる engine BPC 補償の副次効果は区別根拠に使わず、LUT 内容差を主根拠とした。構造検証（`LutWarehouse` variant/`LutType`/entry 数/Store 型）と行為検証（写像が実際に走ること）は別 assertion で固定する (Issue #125)。

## 残存する Unsupported とスコープ外の判断

consumer 側（AudioLens）の受入照合は、pin 更新を伴う実 test 実行ではなく main のソース確認で代替した。gpui_parity 側に Spectrogram gap 列・NaN セル・Table 欠損・Label text の照合があること、screen 側が `tile.columns()` を使い式複製がないこと、各 screen が `label()`/`table()` helper 経由で新 kind を生成することを確認した。実 test lane は consumer repo の ruviform pin 更新後の作業である。`mise run test-viz` という lane は存在せず、ruviform-viz は workspace の `test` でカバーされる (Issue #442)。

docs audit は repository 全体に多数の既存 finding（無関係な invalid ID・status・relation 不整合）を報告するため、対象 3 chain のみを修復し、repo 全体が clean とは主張しない方針とした。`columns.is_empty()` 拒否・列見出し重複検証の hardening は既存契約の拡張でスコープ超過のため follow-up 候補に留めた (Issue #442)。

## 未確定・保留事項

- `ruviform-ui` 側の `count <= rows` validation 要否は follow-up 未対応のままである (Issue #443)。
- 極端に低い Panel での min_h 合計超過による溢れ、Tabs body の `size_full`、orientation sizing、Panel overflow は Deferred Design Concerns として範囲外のままである (Issue #478)。
- Seek/Fader/Knob のキーボード操作は共通 key 規約を決めてから扱う follow-up であり、strip のフォーカスハンドルの unmount 時解放は有界ゆえ未対処、実窓でのキーボード切替の手動証拠は NotRun のままである (Issue #451)。
- 実 OS Shift+Tab の gpui プラットフォーム層での正規化は実機未検証である (Issue #451)。
- 実機での paint 体感計測は consumer 側 pin 更新待ちで NotRun、Table の行×列 div 生成コストは別 Issue 候補のままである (Issue #461)。gpui test platform の `window.scale_factor=2.0` 固定により、window pixel の scale 1.0 実測は real-window lane 側の残件である (Issue #314)。
- consumer 向け観測 DTO と公開 feature 境界は follow-up Issue の設計待ちで、AudioLens 側の GPUI parity は NotRun のままである (Issue #421)。
- AudioLens/Synvelis consumer 側の実 test lane 実行は各 repo の ruviform pin 更新後の作業であり、現時点ではソース確認による代替証拠に留まる。Synvelis 側の project length 変化接続も consumer 側作業として分離されている (Issue #436, Issue #442, Issue #451)。
- Spectrogram 射影・`Tile`→`SpectrogramColumn` 写し替え・専用 theme token・JSON 非有限値表現は follow-up として分離されたままである (Issue #434)。
- platform a11y（OS accessibility tree 接続）は Unsupported のままで、大量行仮想化・`columns.is_empty()` 拒否・列見出し重複検証の hardening も未着手の follow-up 候補である (Issue #435, Issue #442)。
- webview driver には ViewSpec 描画経路がなく、webview parity は複数 Issue で一貫して NotRun である。描画経路の整備は別 Issue 候補である (Issue #448, Issue #451)。
- `SendInput` が環境要因で遮断される過去事例の原因は未同定で、preflight の `environment_blocked` 記録で封じ込める運用である (Issue #313)。
- real-window セッション teardown の `RefCell already borrowed`/HRESULT 0x80070578 は main 由来の既存現象として原因調査が別 Issue 候補のままである (Issue #313)。
- `WindowedIo::JOB_TIMEOUT` は `ruviform-ui-gpui` 側の編集範囲外として未対応である (Issue #313)。
- envelope への epoch segment 追加は wire-protocol 判断として deferred のままである (Issue #410)。
- `CARGO_BIN_EXE_dummy_app` が使えない環境での `cargo run` フォールバック経路は計画段階である (Issue #312)。
- task/plan 文書の status と実装記録の齟齬が複数報告されており、文書上の status だけでは各作業の完了度を確定できないという運用上の注意が残る。
