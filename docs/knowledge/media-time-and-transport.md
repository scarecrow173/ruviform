---
id: "KNOW-0P0KowVAC1GxkKgglQP9xz"
type: "knowledge"
status: "draft"
lifecycle: "draft"
knowledge_kind: "architecture"
title: "時間表現・音楽時間・トランスポート"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/41", "https://github.com/scarecrow173/ruviform/issues/58", "https://github.com/scarecrow173/ruviform/issues/59", "https://github.com/scarecrow173/ruviform/issues/91", "https://github.com/scarecrow173/ruviform/issues/187", "https://github.com/scarecrow173/ruviform/issues/189", "https://github.com/scarecrow173/ruviform/issues/190"]
  informs: []
  explains: []
  supersedes: []
---

# 時間表現・音楽時間・トランスポート

この文書は、メディア時間の内部表現と演算規則、音楽時間（拍・tick）への変換契約、およびブロック単位で進行するトランスポート制御の設計をまとめたものである。各記述の末尾に出所となる Issue 番号を付す。観測・確認された挙動はその旨を明記し、それ以外は記録された設計判断または契約として述べる。

## 時刻値演算の丸めと範囲チェック

時刻値の delta 変換では、中間値を i128 の厳密な有理数商のまま保持し、丸めを最後に 1 回だけ適用する。i64 の範囲チェックも最終の start+delta に対してのみ行う (Issue #41)。

この方針を成立させるため、変換に使う分子の絶対値が |num| <= 2^63 * (2^32 - 1)^2 < 2^127 に収まることを証明し、scaled_fraction を全域関数化して Option による失敗経路を排除した。丸めの magnitude と符号の方針は round_quotient に一元化し、変換の実体は convert_value_wide に集約して convert_value と convert_value_exact は薄い wrapper とした。round_quotient 内の unwrap_or(i128::MAX) による narrowing fallback はこの bound のもとでは到達不能だが、panic に頼らず TryFrom を満たすために残されている (Issue #41)。

残るリスクは、全域性の証明が TimeBase のフィールド幅（u32/i64）に依存していることである。フィールド幅が拡張された場合は bound の再導出が必要になる。この前提条件は定義箇所に文書化されている (Issue #41)。

範囲の終端について、MediaRange::end() は真の最終 overflow のみで飽和する全域関数として維持し、失敗を返せる checked_end(rounding) -> Result<MediaTime> を別途新設した。checked_sub は i64 の duration 値そのものではなく i128 の delta を否定する実装とし、i64::MIN のエッジケースに対応した (Issue #41)。

## ドロップフレーム時計の対応レートと入力検証

from_media_time_drop が対応するドロップフレームレートは 30000/1001 と 60000/1001 の 2 種のみであり、それ以外は Err(UnsupportedDropFrameRate) を返す。FrameRate は TimeBase::new で正規化済みであるため、60_000/2_002 のような非正規化スペルも受理される (Issue #58)。

入力の拒否条件は 2 つある。nominal_fps() == 0 の場合は Err(Error::InvalidValue("frame_rate"))、負の時刻は Err(InvalidValue("negative time")) である (Issue #58)。

## RT パスのチャンクタイムスタンプ

RT（リアルタイム）パスが生成するチャンクタイムスタンプは、整数演算のみ・メモリ確保なし・有界計算という RT 契約のもとで合成する。frame_offset の u128 への変換では u128::from(usize) が対応しないため、u128::try_from(frame_offset).unwrap_or(u128::MAX) で飽和変換する (Issue #59)。

このタイムスタンプは、チャンク先頭フレームのバッファ内位置に基づく推定 DAC/callback 時刻であり、実測値ではない。実測値と誤認されないよう、この意味づけを doc コメントに明記する方針である (Issue #59)。

## 音楽時間変換の失敗処理と投影方法

音楽時間への変換 API では、musical_position を Result<Option>、time_info を Result を返す形に改めた。beat_offset の分母が u32 を超える場合と tick への投影で i64 が溢れる場合に、失敗を暗黙に捨てないためである。この失敗条件は設計判断 D8 として記録されている (Issue #189)。

media_to_tick は絶対拍への Floor 投影を 1 回だけ行う。整数拍アンカーでは旧実装と結果が完全に一致し、非整数拍アンカーでも「変換結果が時刻を超えない」という定義として健全である (Issue #189)。

独立した Time crate の抽出は見送られた。抽出条件として「media-core の非時間型を必要とする consumer が存在すること」を設けていたが、該当する consumer が不在だったためである。この条件は設計判断 D7 として記録されている (Issue #189)。

## consumer 向けの変換精度契約

consumer が利用する変換経路ごとの精度契約は次のとおりである。ppqn は quarter note あたりの tick 数を指す (Issue #189)。

| 変換経路 | 変換方式・精度 |
| --- | --- |
| Beat → Frame（beat_to_media） | NearestEven の単回適用。誤差 ≤ 0.5 frame |
| MediaTime → Beat | 正確。分母が u32 を超える場合は TimeOverflow |
| Beat → Tick(ppqn) | 呼出側が明示する Rounding。誤差 ≤ 選択した丸めの 1 tick |
| MediaTime → Tick(ppqn) | Floor。誤差 ≤ 1 tick |
| beat → CLAP clap_beattime | bits = round_half_away(beat × 2^31)。誤差 ≤ 0.5（変換先の単位） |
| CLAP clap_beattime → Tick | bits × ppqn / 2^31（half-away）。誤差 ≤ 0.5（変換先の単位） |
| VST3 TQuarterNotes | f64 の拍値 |
| Beat → MusicalTimeInfo | time_info(beat, ppqn) 経由。NearestEven で誤差 ≤ 0.5 tick @ ppqn |

## TickPosition の型による等価性の制御

TickPosition は Ord を実装せず、PartialEq は (tick, ppqn) のペア比較とした。分解能の異なる tick 値が暗黙に等価とみなされることを型で防ぐためである。異分解能の値を比較する場合は cmp_same_resolution を使い、分解能が一致しない比較は InvalidValue("tick_ppqn_mismatch") で拒否する。表現不能な値は暗黙に量子化せず、TimeOverflow で明示的に拒否する (Issue #189)。

旧 time_info は bar_start_tick の溢れ時に unwrap_or(0) で 0 を返していたが、Err(TimeOverflow) を返すよう意図的に変更した。失敗を沈黙の 0 返却で隠す振る舞いを除く変更（semantic tightening）である。一方、musical_position・time_info・media_to_tick のシグネチャ変更は、consumer 側（Synvelis）で移行を要する意図的な破壊的変更である (Issue #189)。

## プラグイン規格からの inbound 写像

CLAP・VST3 の双方で、inbound トランスポート写像の音楽時間フィールドはフラグでゲートする。対応フラグが未設定のフィールドは None を返し、有効な音楽時間として解釈しない (Issue #91)。

CLAP の clap_beattime から tick への変換は、公式定数 CLAP_BEATTIME_FACTOR = 1 << 31 に基づき bits × 960 / 2^31（half-away 丸め）で行う。設計文書では係数が 65536 と記述されていたが、実際の ABI に合わせて 2^31 に修正済みである (Issue #91)。この変換経路は後の検証で正しいことが確認され、コードを変更せず Golden テストで固定された (Issue #189)。

## トランスポートの正準契約型

トランスポート制御の正準契約は media-core の transport_schedule.rs に新設した。構成要素は、EngineFrame・ClockEpochId・TransportGeneration などの u64 newtype、TransportCommand・AppliedReceipt・ScheduleFault・BlockContract・InBlockTransportSupport・TransportPlan/TransportPlanBuilder・ScheduledEvent の各型、および legal_len・partition_len・FrameWindow などの純粋関数である。音楽時間側の型は Issue #189 の MediaTime・TempoMap・BeatPosition・Ppqn を消費し、固定の 960/3840 リテラル（PPQN 値）の再導入を禁止する (Issue #190)。

EngineFrame は epoch（クロック世代）フィールドを持ち、PartialOrd を意図的に実装しない。異なる epoch の frame 番号を大小比較できない構造にし、窓への所属判定は checked_offset の Some/None で行う (Issue #190)。

新規 API は Copy・no_std で media-core の RT 方針に整合させ、メモリ確保は builder/controller の非 RT 側に限定する。全公開 enum には #[non_exhaustive] を付与する (Issue #190)。

InBlockTransportSupport は Unknown・Declared・Verified の 3 値を持ち、Unknown と Declared はいかなる場合も supported として扱わない。TransportPlan は Arc で共有し、RT 側は clone せず参照する。plan event の engine frame への写像は PoC（概念実証）では 1:1（tick = frame、varispeed なし）であり、media_time_ticks がこの変換を行う唯一の場所である (Issue #190)。

## 非 RT controller と RT scheduler の分離

トランスポート制御は、非 RT の TransportController と RT の TransportScheduler に分離した。controller は request・preflight・install_plan・device_restart・receipts/faults の drain を担い、scheduler は callback 内で run_callback を実行する。pending・receipts・faults の各バッファは request 時と構築時に capacity を確保し、RT 側は push のみを行う (Issue #190)。

表現できない境界を持つコマンドの扱いは適用モードで分かれる。Exact では illegal な境界に対して UnrepresentableBoundary fault と Rejected receipt を記録し、状態は変更しない。Degraded では max_deferral で有界化した範囲で next_legal_boundary を走査したうえで適用を遅延し、DeferredApply を receipt に記録する (Issue #190)。

preflight（非 RT）は、command の適用点だけでなく、適用後の transport 状態で生じうる loop wrap まで SimState 上で全シミュレートし、loop 自体が unrepresentable な場合を早期に検出する。巻き戻し seek などで同一 frame に複数の due apply が生じうるため、drain は defensive に行う。event window の有効性は generation の一致で判定し、max_events_per_span を超えた場合は EventBudgetExceeded fault を記録する (Issue #190)。

配信経路について、SpanSink trait が span と event 窓の唯一の配信口であり、CallPartition は BlockContract::partition で算出する。算出は均等割で、全 chunk が min/max の範囲内に収まることのみを保証する。device_restart は ClockEpochId の更新として扱い、transport generation とは独立である (Issue #190)。

## plugin bridge の責務と呼出の構成

plugin bridge は、media-core の正準契約を plugin の wire 契約（TransportInfo・EventList・ProcessConfig）へ変換する薄い層である。座標変換の正本は media-core 側の transport_schedule.rs に置き、bridge 側で座標変換を再定義しない。変換ロジックを一箇所に集約する設計判断である (Issue #190)。

event の受け渡しでは、呼出側が供給する &mut [Event] の scratch 領域に Event を書き込み、EventList::new(&scratch, frames) で検証してから process へ渡す。検証条件は offset の非減少・offset < frames・zero-frame flush の整合であり、この方式により RT パスでメモリ確保を行わない (Issue #190)。

process 呼出の frame 数は plugin が BufferView::frames() から読む。観測された挙動として、mock driver（MockDriver・RecordingProcessor）は call 長に合わせた slice を渡す。「min_frames 違反の call を呼ばない」という保証は、callback 側で違反を検査するのではなく、呼出前に partition を要求する bridge の構造で実現する。違反しうる call を構造的に発行しない設計判断であり、受入条件として min_frames 違反の process call が 1 件も記録されないことを RecordingProcessor 側でも assert する (Issue #190)。

calls_for_span は span の合法な physical partition を BlockContract::partition に委譲して求め、deliver 不能な span は None を返して plugin へ届かないようにする。partition の正当性判定は scheduler 側の契約に一本化されている (Issue #190)。

## ProcessConfig の写像と capability の扱い

contract_from_process_config は、activated な ProcessConfig の min_frames/max_frames を BlockContract へ写像する。in_block capability は adapter が attest した入力のみを信頼し、Unknown・Declared は supported として扱わない。申告値ではなく検証済みの証明で判断する方針である (Issue #190)。

capability の差による挙動は、BlockContract::from_process_config(&ProcessConfig, InBlockTransportSupport) と CallPlan::for_span の引数で明示的に区別し、テスト可能な構成とした。L0 は gate 内で frames < min_frames を拒否し、L1 は子 contract へ委譲する。このため呼出側は、実効の activated minimum を渡す必要がある (Issue #190)。

## 呼出単位のイベント窓とトランスポート情報

build_call_event_list は、span の event 窓のうち半開区間 [call_start, call_start + frames) に入る event を抽出し、process 開始位置を基準とする相対の frame_offset へ変換して scratch 配列へ書き込む。engine frame の半開窓判定には EngineFrame::checked_offset を使い、epoch 安全で PartialOrd に依らない (Issue #190)。

zero-frame flush は callback 経由ではなく wire 契約として直接検証する。観測された挙動として、frames = 0 の EventList では offset = 0 の event は受理され、offset > 0 の event は拒否される (Issue #190)。

call_transport_info は 1 つの physical call に対する TransportInfo（status・call offset の transport position・loop range）を構築する。tempo map と ppqn が指定された場合は、TempoMap::media_to_beat と time_info による musical snapshot を併せて生成する。Issue #189 の rational beat 経路を用いる設計で、固定 PPQN は持たない (Issue #189・#190)。

## 未実装の Verified 経路とフォローアップ

in-block transport event を扱う InBlockTransportSupport::Verified 経路は、PoC では未実装である。リスクとして記録されたうえで、adapter 側の責務として Issue #196 へ委譲された (Issue #187)。

フォローアップとして、Issue #196（adapter capability attestation と multi-bus/stateful process）と、Issue #187 自体に残る実 device clock backend への device_restart 結線が挙げられている (Issue #187)。

## 未確定・保留事項

- InBlockTransportSupport::Verified 経路の現在の実装状況は、Issue #196 側の記録を参照しないと確定できない (Issue #187)。
- 「tempo map と ppqn が指定された場合」という条件と「固定 PPQN を持たない設計」という記述の関係は、記録から一意に読み取れない。ppqn が呼出ごとの外部入力を指すかどうかは未確認である (Issue #189・#190)。
- L0・L1 が指す層または capability レベルの定義は記録に含まれない。文脈上は capability の段階と読めるが、正確な定義は未確認である (Issue #190)。
- TempoMap::media_to_beat と time_info が構成する musical snapshot の具体的なフィールド契約は記録に含まれない (Issue #189・#190)。
- Issue #187 に帰属させた知見の記録には (Issue #190) という出所表記も混在しており、Issue #187 と Issue #190 のどちらの作業に基づくか一意に決まらない箇所がある。本文では記録の見出しに従い Issue #187 として記した。
- Issue #190 の実装は T-1・T-2・T-3 の 3 分割で記録されている。これらを統合した完了記録の有無は未確認である。
- 記録の収集対象には Issue #190 関連の計画文書・タスク文書が含まれておらず、スコープ決定や検証方針に関する知見がそれらに存在する場合、本文書には反映されていない可能性がある。
