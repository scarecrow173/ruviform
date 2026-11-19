---
id: "KNOW-OHeByH98CHHL21HPfXPdZi"
type: "knowledge"
status: "draft"
lifecycle: "draft"
knowledge_kind: "architecture"
title: "データモデルと media/audio/video core の契約"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/18", "https://github.com/scarecrow173/ruviform/issues/26", "https://github.com/scarecrow173/ruviform/issues/41", "https://github.com/scarecrow173/ruviform/issues/53", "https://github.com/scarecrow173/ruviform/issues/96", "https://github.com/scarecrow173/ruviform/issues/97", "https://github.com/scarecrow173/ruviform/issues/99", "https://github.com/scarecrow173/ruviform/issues/121", "https://github.com/scarecrow173/ruviform/issues/171", "https://github.com/scarecrow173/ruviform/issues/173"]
  informs: []
  explains: []
  supersedes: []
---

# データモデルと media/audio/video core の契約

この文書は、データモデルと media/audio/video core の型・契約に関する設計知見を、過去の実装記録から抽出してまとめたものである。記録元の文書は後に削除されたため、本書はそれらに依存せず単独で読めるように記述する。事実は各記述の末尾に Issue 番号で出所を示す。実測値や観測された挙動は事実として記し、解釈が含まれる箇所はその旨を明示する。

## 有理数時間モデルと checked 演算

media-core の時間型は f64 を中核表現にせず、i128/u128 中間表現の純粋整数関数で実装した。Add/Sub trait は変換や i64 オーバーフローで panic/saturate が必要になり、「no silent f64, no panic」契約に反するため、checked_add/checked_sub の明示 Result 形を選んだ（Issue #18）。

MediaRange::end() は i128 計算後に i64 へ飽和するトータル関数とし、to_seconds_f64 のみ display-only の f64 例外として #[expect] 付きで文書化した（Issue #18）。MediaDuration::to_seconds_f64 の追加と MediaDuration 命名（std 衝突回避・Media* 系統で統一）は仕様曖昧への coordinator ruling で確定した（Issue #18）。

TimeBase に Ord を実装しない。有理数フィールドの比較は意味論的に誤りになるためである（Issue #18）。負タイムコードは 0 floor せず Err(InvalidValue) とし、pre-roll の暗黙誤ラベルを防ぐ。drop-frame 判定は正規化後の TimeBase で行い、60000/2002 を 30000/1001 として受理する（Issue #18）。

cross-base の Eq/Ord/Hash は手実装とした。u128 マグニチュード比較は < 2^128 に束ねられオーバーフローせず、最低項正規化ハッシュで Eq と一貫し、0 は全 base で等値になる。MediaTime::ZERO と MediaDuration::ZERO（0 at 1/1）は全 base の 0 と cross-base 等値である（Issue #18）。

## 時間演算の早期飽和バグと i128 中間精度化

MediaRange::end()/MediaTime::checked_add/checked_sub には「duration の中間変換が i64 超過・最終和は表現可能」の隅で誤値や Err になる早期飽和バグがあった。修正方針は delta を i128 正確有理数（value * num * other_den / (den * other_num)）として計算し、丸めは最後に 1 回、i64 範囲検査は最終値のみに統一するものである（Issue #41）。

checked_end(rounding) -> Result<MediaTime> を新設し、checked_sub の i64::MIN duration 否定は i128 上で行う。i64 上の否定はオーバーフローするためである（Issue #41）。convert の契約は変更しない方針とし、回帰テストを先行追加する TDD 順序を取った（Issue #41）。

## 公開 enum の #[non_exhaustive] ポリシー

公開 enum には enum レベルの #[non_exhaustive] を付与するポリシーを採った。クレート外から到達可能な全公開 enum が対象で、兄弟ワークスペースクレートも外部扱いとし、クレート内部 enum は免除する。これは enum ADR（ADR-034J6OnZouPI8HQy6cIg9D、Option 1）に基づく適用である（Issue #41）。

Issue #18 時点では exhaustive 列挙型を publish = false 内部クレートの文書化済み判断として維持していたが、Issue #41 の enum ADR で覆された（Issue #18, Issue #41）。Issue #41 では workspace 全体の pub enum を grep 監査し 11 件全てを確認し、9 件へ新規付与した。PixelFormat/SampleFormat は既存維持だった（Issue #41）。

3 つの lib.rs の #![allow(clippy::exhaustive_enums)] を撤去し、clippy pedantic で属性漏れを lint ゲート検出する構成にした。各 enum には SampleFormat 先例に倣った rustdoc 注記を付与し、AGENTS.md の暫定規約へ 1 行追記した（Issue #41）。

## media-core のデータ型とコンストラクタ契約

MediaTime::ZERO/MediaDuration::ZERO は brief 任意項を採用した。AudioBuffer::new は -> Self 形状契約を維持し、絶対容量上限 isize::MAX バイトでの panic を文書化した。形状エラーは Result コンストラクタ側が担当する分担である（Issue #18）。AudioBuffer::try_new(spec) -> Result<Self> を新設し、from_interleaved と形状検証を共有した。new() の panic 契約は維持し、doc で相互参照する（Issue #41）。

ChannelLayout::Custom の構造的不変条件は非空 mask newtype（SpeakerMaskSet、new(mask) -> Result<Self>）で payload 化した。custom()/channels()/speaker_mask() とテストを更新し、SpeakerMask 自体は不変とした（Issue #41）。

MediaTime::convert/checked_add には「呼出しごとに 1 回の丸め」「境界で変換して誤差累積を避ける」doc 注記を付けた（Issue #41）。

## lint 方針とクレート内部の公開範囲

media-core の lint 方針は module_name_repetitions/exhaustive_enums/exhaustive_structs を crate-level allow とした。root 再エクスポート名と publish = false 内部クレートの判断に基づくが、enum のみは Issue #41 で撤去された（Issue #18, Issue #41）。

正本要件と baseline が競合する場合は brief を優先し、報告義務を持たせる運用とした（Issue #18）。

## video-core のデータモデルと所有権契約

VideoFrame の Clone は Arc 参照共有でありコピーではない。Arc::get_mut が書込ゲートで、共有中は plane_mut() が None を返す。これが共有状態の唯一の観測点であり、introspection API は追加しない既定方針（D5-i）とした（Issue #26）。drop は最後の Arc 解放で backing を dealloc するため、RT パスでは所有を drop せず、退役フレームは非 RT 層へ送出する（Drop 退避）。フレームプールは非 RT 層が所有し、返却・所有権移転は明示 API として後続設計とした（Issue #26）。

from_shared_planes が唯一の外部 zero-copy 供給経路である。SharedPlane は offset-less 制約を持ち、plane 先頭オフセットを表現できないため、media-io Issue で再評価対象とした（Issue #26）。

VideoFrame に PartialEq/Eq/Hash を実装しない。Arc 内容を持つ identity 的セマンティクスになるためである（Issue #26）。MediaTime/MediaDuration/Error/Result は media-core から直接使用し、video-core で再エクスポートしない（Issue #26）。

plane 寸法導出は frame::plane_dimensions に一元化し、PixelFormat::chroma_subsampling() 駆動とした。420/422 は ceil chroma で、subsample 表の二重管理を回避した。Yuv444p.chroma_subsampling() は Some((1,1)) の統一グリッド解釈とし、None 意図なら plane_dimensions 特殊ケースが必要という open question を残した（Issue #26）。bytes_per_sample は範囲外 plane に 0 を返すトータル化とした（Issue #26）。

ビューは使用可能領域 stride*(h-1)+row_bytes へ trim し、backing 末尾・行間パディングを除外する。required_len に isize::MAX 上限を設け、vec! 容量 panic を Err(InvalidValue("dimensions")) へ写像した（Issue #26）。

plane_mut()==None は「共有中 or index 範囲外」の合成シグナルである点を doc で限定明記した。VideoFrame 型 doc に clone/drop の RT ラベルを集約した。derive Clone と暗黙 Drop では per-impl doc を置けないため、型レベルで「Clone=refcount bump・Drop=最後の Arc で dealloc、いずれも non-RT」を規定した（Issue #26）。

## RT 契約ラベルと型レベル強制の限界

video-proc の二相カーネル API は「非 RT の構築（new()/prepare で係数・ワーク領域確定）→ 準備済みオブジェクトの alloc-free process(&mut self, src, dst)」形状である（Issue #26）。

process は RT 適合設計だが、保証は API 単位の **RT contract:** ラベルで個別明示し、型レベル強制はない。誤用防止は doc 契約とレビュー依存である。RtContext 引数・Processor impl は持たず、将来 ruviform-rt の Processor trait へ impl 追加で接続する後方互換の拡張余地のみ残した（Issue #26）。

## カーネル共通のエラー・検証規約

検証順序は「format → 寸法 → 共有 dst 拒否」を全て書込み前に実施し、失敗時の部分書き込みをなくした。共有 dst は dst.plane_mut(..) == None を Err(InvalidValue("dst plane")) へ写像し、T1 の書込ゲート観測点をそのまま利用した（Issue #26）。

エラーモデルは media-core Error::InvalidValue(&'static str) のみで variant 追加なしとした（非破壊性）。payload は "src format"/"dst format"/"dimensions"/"color matrix" 等の field-ish 文字列で、video-core 既存規約に従う。#[non_exhaustive] 未知 variant は _ arm で graceful に Err とし、unreachable!() を禁止した（Issue #26）。

pts/duration は src から dst へ伝播する。色変換は時刻を変えない確定済み既定である（Issue #26）。

## f16 の primitive 不成立と binary16 コーデック

primitive f16（Rust 1.82+、MSRV 1.85 で可という plan 前提）は x86_64-pc-windows-msvc で pinned ツールチェーン 1.99.0 時点でも unstable（E0658・rust-lang/rust#116909）であり、stable MSRV クレートでは使用不可と判明した（Issue #26）。

nightly 化・外部クレート・unsafe のいずれも制約違反のため、LE binary16 バイトレイアウト自体が契約なので f64 経由の IEEE 754 RNE エンコード（f16::from_f64 と全入力ビット一致）と exact デコードを crate 内部に実装した。等価性契約は forward 1 ulp（binary16 ドメイン）・inverse ビット完全一致である。コーデックは実装前に Python struct 'e' 対比で全 65536 パターンをビット一致確認済みである（Issue #26）。

plan は approved の歴史的記録なので D-2/Q3 に「実績注記」を付記する形で訂正した。本文は書き換えない方針である（Issue #26）。

## 色変換カーネルの数値設計

YUV↔RGB カーネルは Q16 固定小数点の整数演算のみで、等価性契約はビット完全一致とした。浮動小数点を含まないため SIMD 化時も参照実装と一致する（Issue #26）。

YuvToRgb::new() は全 6 組（3 matrix × 2 range）の係数を bake し、process が src.color() を毎回解決して選択する。「変換は入力フレームの ColorInfo を参照」の spec 文言と二相形状を両立させた設計である（Issue #26）。

chroma upsample は最近傍複写とし、siting 前提は水平 co-sited・垂直 center を up/down 対の規約として doc 明記した。半画素ずれは既知リスク受容である（Issue #26）。

RgbToYuv の chroma downsample は Q16 ドメインのままセル平均してから clip8 する。先に u8 化する経路とは結果が異なり得るため、識別テスト chroma_downsample_420_averages_in_q16_domain で固定した。i32 和は |Q16 chroma| ≲ 3×10⁷/画素 × セル ≤4 画素で ≲ 1.2×10⁸ ≪ i32::MAX でオーバーフロー不敗である（Issue #26）。

係数は構築引数とし、RESOLVED_MATRIX/RESOLVED_RANGE で適用値を出力 ColorInfo に記録する。plan Q2 既定は Unspecified→Bt601/Limited、Identity/未知 variant→構築時 Err である（Issue #26）。

## HDR 対応範囲の決定

HDR 対応範囲は「ColorInfo/HdrMetadata がフレームに付随して運ばれ、変換カーネルは matrix/range の code-value 演算のみ適用し primaries/transfer/hdr を無変更で伝播」と決定した（ADR D3-a）。transfer 関数適用・トーンマッピング・広範色管理は範囲外である（Issue #26）。

Unspecified 既定規則は matrix→Bt601・range→Limited とした。出力 ColorInfo 更新規則として、YUV→RGB では matrix→Identity・range→Full（RGB 出力は常にフルコード空間）・残り伝播、RGB→YUV では適用係数を記録する（Issue #26）。

## pixel format の追加と working-space 設計

PixelFormat::RgbaF32 は packed RGBA・4× IEEE-754 binary32 little-endian・1 plane・16 bytes/pixel とした。plane_count()==1・is_planar()==false・chroma_subsampling()==None である。working space は BT.2020 primaries + Linear transfer + Identity matrix + Full range + L_DS = 100 nits とした（Issue #99）。

transfer.rs は pub(crate) mod とし、公開 API 面を広げない。transfer decode/encode（Linear/Bt709/Smpte170m/Pq/Hlg — 式・定数は design 記載値を逐語実装・独自改変なし）と ColorPrimaries→BT.2020 結合 3×3 行列の f64 導出テーブル（chromatic adaptation なし・RGB→XYZ D65 正規化）を集約した。primaries_slot/transfer_slot は Unspecified→Bt709 既定・未知 variant は _ arm で InvalidValue 拒否とした。既存カーネルは RgbaF32 を知らなくても _ arm で graceful 拒否を維持する（#[non_exhaustive] の additive 拡張）（Issue #99）。

## ToWorking/FromWorking カーネルの設計

ToWorking（new() 無引数 + Default）は src ∈ {Rgba8（v/255）, RgbaF16（既存 pub(crate) コーデック再利用）, RgbaF32（raw LE）} → dst RgbaF32 とした。RgbaF32→RgbaF32 identity 入力でも画素コピーを通す no-op early return なし設計である（Issue #99）。

src.color() を呼出し毎に読み、primaries_slot/transfer_slot で既定解決する。matrix/range は RGB 入力に非適用で検証しない。画素毎に 4ch decode → RGB のみ 3×3 行列（src primaries→BT.2020）→ alpha パススルーとし、dst.color = {Bt2020, Linear, Identity, Full, hdr: src.hdr} とした（Issue #99）。

FromWorking は new(dst_primaries, dst_transfer) -> Result<Self> で結合行列 + encode パラメータを ctor で焼付ける（非 RT by convention）。src RgbaF32 → dst ∈ {Rgba8, RgbaF32} とし、Rgba8 出力は round(clamp(v,0,1)·255)（RHU）量子化とした。ctor Bt2020/Linear の identity 経路が RgbaF32 純粋量子化を表現し、shuffle 専用カーネルは持たない（Issue #99）。

## ToneMapSdr のパラメータ導出と passthrough 規則

ToneMapSdr は輝度写像に設定面を持たない既定動作（固定 L_DS = 100 nits）である。L_HS 導出優先順位は mastering_display.max_luminance（/10_000→nits・>0 のみ）→ content_light_level.max_cll（>0 のみ）→ 既定 1000 nits とした（Issue #99）。

L_min は mastering_display.min_luminance（max_luminance 未満の整合値のみ、不正順序は b=0 フォールバック）とした。Y_src（BT.2020 luma）に EETF の f(E₁)/ks = 1.5·L_DS/L_HS − 0.5・3 次補間式を適用し、比率保持 RGB スケール（dst.rgb = src.rgb × (Y_dst/Y_src)・alpha パススルー）とした（Issue #99）。

L_HS ≤ L_DS → ks ≥ 1 で全域 passthrough、Y_src ≤ 0 は画素 passthrough とした。dst.color は src を保持して hdr: None のみクリアし、配下 stage への再適用二重化を防ぐ消費規則とした（Issue #99）。

## ruviform-edit のデータモデルと評価射影

ruviform-edit は NLE 非破壊編集データモデル（Timeline/Track/Clip + mutation）と評価射影（evaluate → Evaluation/ActiveClip）のみを担う。engine 構造体生成・undo/redo・UI・素材管理・直列化・トランジション・再生速度変更は Out of Scope である。全 API 非 RT で、evaluate の Vec 確保を含み RT パスからの呼出し禁止を crate doc に明記した（Issue #96）。

依存は ruviform-media-core path のみとした。audio-core/video-core は spec で許可されるが最小属性セットでは不要なため YAGNI で実需時追加とし、engine・他 foundations への依存禁止で射影の中立性を依存構造で保証した。格納モデルは private + accessor、ClipSpec は pub フィールド plain data、ID は単調採番・再利用なしとした（Issue #96）。

## mutation 検証と evaluate の算術経路

全 mutation（add_track/remove_track/insert_clip/remove_clip/move_clip/trim_clip/enabled）で完全検証ルールセットを再実行する: 正 duration・checked_end 厳密性と base-collapse・range.duration() <= source_range.duration()・kind 整合・属性サニティ・*NotFound。range は timeline base へ NearestEven 正規化（normalize_range/validate_clip に集約）する（Issue #96）。

evaluate(at) は at を timeline base へ 1 回だけ変換し、enabled トラックごとに半開区間包含の最後のクリップを勝者とする（後開始・同開始後挿入優先が stable sort 順から導出）。source_time = source_range.start().checked_add(offset)、残区間 [source_time, min(out, in + range.duration())) とした。丸め回数固定（at 1 回 + checked_add 最大 2 回）がレビュー焦点で、O(tracks × clips) 線形走査である（Issue #96）。

## DataRegistry の eviction・保持・解放契約

DataRegistry の内部表現は BTreeMap<String, Arc<dyn DataSource>> から Inner { map, tick } に変え、Entry.last_used に単調カウンタを記録する世代番号式 LRU を実装した。FIFO/insertion-order ではなく LRU を選んだ理由は、pull model では frontend が複数 payload を並行 fetch しうるため、登録順追い出しだと「表示中の最古 payload」が落ち得るからである。get/read が recency を更新する（write lock 経由）ことで、pull 中の source は追い出されない契約にした。I/O や時計を使わない単純な世代番号で、issue の "generation/LRU 上限" の generation に相当する（Issue #171）。

上限は件数（max_sources: Option<usize>）とし、byte ベース上限は却下した。DataSource::len が仮想・lazy source で実メモリと一致しないためで、連番名の増殖は件数の問題なので件数上限が最も単純かつ予測可能（KISS）と判断した。既定の new()/with_max_chunk() は無制限のまま維持し、有界化は with_max_sources/with_limits の opt-in とした。暗黙の追い出しによる突然の 404 を既存呼出（AL GUI 含む）へ持ち込まないためである（Issue #171）。

解放は Arc ベース deferred-free とした。unregister/clear/eviction は登録名の解決を止めるが、進行中の pull（get 済み Arc）は完走し、メモリは最後の Arc drop 時に解放される。unregister の返り値を Option<Arc<dyn DataSource>> ではなく bool にしたのは、解放 API の返値で payload を延命させる事故を防ぐためである。eviction 通知 API は YAGNI で見送り（観測は names()/len()/sys.capabilities で可能）、追い出しは登録時・上限超過時のみの O(n) 線形走査で、双方向リスト+HashMap は過剰と判断した（Issue #171）。

## rvf://data の wire 契約の所有層

rvf://data の paging 契約（リクエスト slug/query 規則、X-Rvf-Total 等のレスポンス header、status 対応表、per-request cap、sys.capabilities による実行時 discovery、paging loop 例コード）の canonical 文書は adapter 層（ruviform-ui-webview README）に置いた。ruviform-ui の data モジュールは registry 側の意味論のみ保持し module doc にポインタを置く。wire 契約は adapter が所有する層に置く設計である（Issue #173）。

DataError→status 対応は design 準拠へ修正した。approved design は offset/len 範囲外を 400 と定めるが、実装は全 DataError を 404 に潰していた。「未登録名」と「範囲外 offset」を frontend が判別できないため、NotFound→404・OutOfRange→400・Read 系その他→500 に修正した。#[non_exhaustive] のため wildcard arm 必須で将来 variant も 500 系になる。offset == total は avail=0/want=0 の正常経路として 200 空 body を維持し、paging loop の終端到達点として明文化した（Issue #173）。

per-request cap 16 MiB の根拠は、DataRegistry::max_chunk 既定 DEFAULT_MAX_CHUNK = 16 MiB が design/PoC 由来（PoC 実測 ~94–146 MB/s、全量一括を提供しない pull 設計）で、cap 超過は黙って切詰めることである。cap/ソース一覧は sys.capabilities の data.max_chunk/data.sources で実行時 discovery 可能とした。design の HostConfig.max_chunk_bytes 項目は実装に存在せず、cap は DataRegistry が保有する（headless demo_cli の registry 単体利用とも整合）。design 側更新は別 Issue 候補として記録された（Issue #173）。

## 診断レコード・DiagRegistry の保持契約

stderr 診断は Mutex 保護の 64 KiB tail 優先 byte ring + total_read_bytes/dropped_bytes counter + flags（eof/drain_deadline_exceeded/unread_bytes_observed）とした。StderrSnapshot は同一 lock 内で同一時点に組み立て、不変条件 total_read = bytes.len() + dropped が常に成立する。bytes は Vec<u8> のまま保持し、stderr は任意バイト列で String 変換しない（Issue #121）。

DiagRegistry は host bounded retention とした。完了失敗レコード 8 件・retained stderr payload 合計 512 KiB 上限（active capture・snapshot コピー・メタデータは別計上）、超過は最古 evict + evicted_records 計数とした。retention lock 内で連続採番 diagnostics_id（欠番・再利用・試行前採番禁止）+ evicted_watermark/issued_max で DiagnosticQuery::{Found, Evicted, NotFound} の 3 値識別とした。stderr は他 sink へ暗黙 echo しない（Issue #121）。

## #[path] harness の crate:: パス禁止

tests/l1_proxy_e2e.rs が #[path] でモジュールを graft する harness では crate:: が test crate root を指すため、l1/diag.rs 等の共有モジュールは crate:: パスを使えない。参照は ruviform_plugin_core::/ruviform_rt::/libc::/windows_sys::/std:: 等の crate 名か self::/super:: で行う。「crate:: を使わない」が不変条件であり、ホワイトリストではない。これは l1/mod.rs を肥大化させない責務境界（mod diag を別ファイルに分離）とセットの再利用可能な手法である（Issue #121）。

## SIMD 基盤の公開範囲と段階的導入

fearless_simd = "1.1" を dev-dependencies から [dependencies] へ昇格し、crates/ruviform-math/src/dsp.rs を新設した。SimdToken は内部 Sel enum + detect() + #[doc(hidden)] force_*（テスト用強制）+ level() で構成し、PoC の #[simd] 8 カーネル本体を *_impl<S: Simd> として移植した。公開 API に fearless_simd 型を露出しないのが Done When の条件である。ルーティング・公開関数は次タスクに分離し、「昇格が解決し内部カーネルがコンパイルする」までをスコープとする段階的導入とした（Issue #53）。

## 合成・変換カーネルの量子化・等価性設計

CompositeOver（source-over in-place 合成・process(src, dst, opacity)）と Crossfade（全 4ch 加重混合・process(from, to, dst, t)）は new() 無引数 + Default impl の unit 形状とした。Q8 規約: w/tq は呼出し毎に 1 回だけ量子化し両ピクセルパスが共有する。Rgba8 は b（backdrop 項）を画素毎 1 回計算して oa/oc で再利用する自己無撞着展開（oa == 0 → (0,0,0,0)）。RgbaF16 は f32 評価 + RNE binary16 丸め（oa < 2⁻¹⁴ denormal 域ガード → (0,0,0,0)、非有限は f32 セマンティクス伝播）。w == 0 は全検証通過後・画素書込み前の early return で dst 不変だが、共有 dst 拒否は opacity = 0 でも先行する。等価性契約は参照実装（f64 評価→binary32 丸め）に ≤ 2 ulp とした（Issue #97）。

AffineTransform（process(src, dst, map)）は整数矩形写像 + セル中心最近傍 + transparent-black 充填の完結変換とした。cell_center_offset が (2·offset·src_len + src_len)/(2·dst_len) を u128 で正確評価し（商は構造的に < src_len でクランプ不要）、境界検査は i64 拡幅とした。非交差/空 map.dst はエラーとせず全画素 transparent black の定義済み結果とした。map.src 非空・x/y ≥ 0・x+w ≤ width 等の違反は InvalidValue("src rect") で書込み前に全検証する。design 表記 &RectMap から値渡しへの意図的変更（Copy plain data で literal を借用せず渡せる形状）である。画素値を演算しない sample_bytes 単位複写のみのため Rgba8/RgbaF16 共通パスで bit-exact、dst メタデータへ src の ColorInfo/pts/duration を伝播する（Issue #97）。

## テスト・検証運用の教訓

mise run test-rt は rt_ の substring マッチでテストを選択するため、非 RT テスト名に rt_ を内包する語（assert_/convert_/support_/start_/short_ 等）があると RT レーンへ混入する。実害例として video-core の view_rejects_short_backing が rt_ フィルタに誤選択され view_rejects_undersized_backing へ改名され、media-core の convert_exact doctest が既知の誤検出候補になった。対策は命名規約（非 RT テスト・公開 item 名に rt_ 部分文字列を含めない）で、厳密プレフィックスマッチ化は別 Issue の検討事項として残る（Issue #18, Issue #26）。

cargo metadata の publish 値は禁止時に [] であり null ではないため、チェックリスト文言どおり null を grep すると誤判定になる。verify-report の attention item として記録済みである（Issue #18）。

Issue #18 の docs 同期・総合検証では、Reviewer A（spec 適合）と Reviewer B（健全性）が指摘を出し、Validator が各指摘を CONFIRMED/UNVERIFIED 判定、Judge が blocking を決め、修正 wave で対応する独立レビューサイクルを回した。方針は「blocking のみマージ前必須」だが、Judge 判定で A1・A2・A3/C3 を blocking、C1+C2・C4・C5 を非 blocking とし 7 件すべてを修正 wave で処理した（C5 は文書化済み判断のため変更不要 ruling）。最終判定は ACCEPT WITH NON-BLOCKING NOTES である（Issue #18）。

Issue #18 の spec→plan→test-spec→4 task→4 IR の doc チェーンは Issue 完了後の遡及的記録整備であり、今後の実装ストリームでは同一チェーンを事前作成で運用する方針が follow-up として記録された。docs 更新は事実突合ベース（grep で実クレート状態を確認してから文言確定）で行われた（Issue #18）。

Issue #26 の doc 同期方針として、baseline・plan は approved の歴史的記録なので本文を書き換えず「実績注記」の付記に留め、注記の重複を避けるため D-2/Q3 のみへ付記した（Risk Register 3 の stale 記述は残置）。AGENTS.md/README.md は ruviform-video-proc 新設後の実態（7 クレート・path 依存・RT ラベル対象）へ同期し、「RT テスト未実装」等の事実と食い違う stale 記述は範囲内で訂正した。spec の status は approved のまま（implemented 遷移は別ライフサイクルステップ）である（Issue #26）。

## 未確定・保留事項

- Issue #26 plan の Q5 で、gh CLI 不在環境のため Issue #26 コメント本文の最終確認が未実施のまま残った。AC 認識は #41 トリアージ由来の二次情報であり、一次確認は未了である（Issue #26）。
- Issue #18 由来の open question として、PixelFormat::Yuv444p.chroma_subsampling() が Some((1,1)) を返す統一グリッド解釈は文書化済みだが、None が意図の場合は plane_dimensions の特殊ケースが必要であり、最終確定は未記録である（Issue #26）。
- mise run test-rt の substring フィルタ精度改善（厳密プレフィックスマッチ化）は複数文書で別 Issue 検討事項として残っており、対応状況は不明である（Issue #18, Issue #26）。
- Issue #41 T4 の新規 Issue 起票（FrameRate/TimeBase 分離・Timecode 拡張・丸め誤差累積ポリシー）の起票番号は plan 実施時に確定する記述だが、実際の番号は対象文書内に記録されていない。他の延期項目は借用 planar ビュー→#29/#23/#25、RT テスト選択・alloc 検証→#22、VideoFrame 所有権・pool→#26 と記録済みである（Issue #41）。
- Issue #171 の max_sources 具体値は caller 依存で本 crate は機構のみ提供しており、audio-lens #105 側で採用される値は本リポジトリの文書からは不明である（Issue #171）。
- Issue #26 の Cargo.toml メンバーコメントの不完全さ（media-core 依存未列挙）は「別途報告事項」として据え置かれており、修正済みかは不明である（Issue #26）。
