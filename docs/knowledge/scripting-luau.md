---
id: "KNOW-8R7vrK8mT8KIt9ZLPgVHI5"
type: "knowledge"
status: "draft"
lifecycle: "draft"
knowledge_kind: "codebase"
title: "スクリプト基盤・Luau アナライザ"
created: "2026-11-20"
updated: "2026-11-20"
owners: []
relations:
  source: ["https://github.com/scarecrow173/ruviform/issues/558", "https://github.com/scarecrow173/ruviform/issues/559", "https://github.com/scarecrow173/ruviform/issues/560", "https://github.com/scarecrow173/ruviform/issues/562", "https://github.com/scarecrow173/ruviform/issues/692", "https://github.com/scarecrow173/ruviform/issues/716"]
  informs: []
  explains: []
  supersedes: []
---

# スクリプト基盤・Luau アナライザ

この文書は、スクリプト基盤と Luau アナライザの設計・実装から得られた再利用可能な知見をまとめたものである。対象となった実装記録は後に削除されており、この文書はそれらの記録から抽出した知見だけを単独で読める形で保持する。記述は Issue #558、#559、#560、#562、#692、#716 の実装記録に基づき、各主張には出典の Issue 番号を付記する。実測値や観測された挙動は事実として記し、解釈を含む部分はその旨を明示する。

## 言語非依存 Port と実行時 Registry/Dispatch の採用

スクリプト基盤の中核となる設計判断は、言語非依存の Port と実行時 Registry/Dispatch を採用したことである（Issue #558）。却下した代替案は四つある。Vendor 型公開（Option A）は mlua や Rhai の型が公開面へ漏れ、二つの Consumer の製品型非依存という要件を壊すため却下した。単一言語固定（Option B）は Rhai 共存 PoC（Issue #559）と将来の言語追加の余地を失うため却下した。ビルド時 feature 排他（Option D）は Issue が明示的に禁止しており、同一バイナリ内での共存を実現できないため却下した。採用の根拠は Issue #559 の Rhai 共存 PoC と将来の言語追加余地である。

Issue #558 は文書スライスとして、実装 crate・公開 API・PoC コードを生成せず、契約レベルの定義（型・不変条件・エラー分類）に留めた。実 Runtime の共存、三 OS への配布、native FFI は Issue #559・#560 へ委譲し、共通 API の製品昇格と Rhai の本番採用は実証後に判断する Deferred とした。

契約の Fail-Closed 性は、判定表（F-01〜F-18）と Fault Matrix（M-01〜M-13）を Fixture（FX-P-01〜P-08・FX-N-01〜N-24）へ割り当て、test-spec の Rule（R-REG〜R-DOC）で検証可能な形に固定した。受入条件から契約条項、根拠への対応は記録として残されている（Issue #558）。また、二つの Consumer を独立に判定し、「一つの Consumer の成功は共通採用を意味しない」ことを明記した。AudioLens への Script 適用は、Ruviform 昇格で依存木に影響が出た時点まで Deferred とした（Issue #558）。

## analyzer と runtime の hybrid 構成

Luau の解析と実行は、同一 crate 内で異なる依存を持つ hybrid 構成を採用した。analyzer は vendored Luau.Analysis を C ABI shim 経由で呼ぶ native bridge であり、runtime は mlua の Rust API を使う（Issue #692、#716）。

analyzer 側の選定では、mlua API のみでは Analysis が含まれない点、dynlink 共有ライブラリは配布と版整合のリスクがある点を理由に両案を却下した（Issue #692）。runtime 側では、interim 実装が同一 mlua 0.12.2 で評価・budget・artifact を実証済みであり、必要な API がすべて safe API として存在するため mlua Rust API を採用した。新規依存・unsafe・C++ shim の変更はゼロで済んだ（Issue #716）。runtime 専用 crate の新設も却下した。backend_id `luau-mlua` は language=luau の統合 backend であり、analyzer と同一 crate の責務で、依存とリンク供給の配線を重複させないためである（Issue #716）。

mlua は crates/ruviform-script-luau では dev-dependency ではなく必須依存である。shim がリンクする Luau の静的ライブラリは mlua→mlua-sys→luau0-src の build script が供給するため、dev-dependency に置くと本番依存グラフに入らず、下流 consumer の最終リンクで解決不能になる。Rust API としては未使用のままである（Issue #692）。

## canonical 表現と digest の契約

canonical 表現は手書きの writer 一箇所に集約し、serde derive 方式は棄却した。wire format が未決定の段階で公開契約型に serde を露出しない判断によるもので、serde_json は文字列 escape のみに内部利用する。record の key は UTF-8 byte 順に sort し、Absent の field は省略し、array の順序は保持する（Issue #692）。

非有限 Number（NaN・±inf）の canonical 表現は、引用符 sentinel から `{"$rv":"number","value":"nan"|"inf"|"-inf"}` の tag record に変更した。当初の sentinel は通常の String と衝突し得たためである。codec の materialize が出力する形と一致させ、`canonical_value(v) == canonical_value(materialize(v))` を有限範囲で保つ（Issue #692）。

Integer と Number の canonical 一致は意図的な仕様である。canonical_number に f64 の Display shortest round-trip を使い整数値は小数点なしで出力するため、Number(1.0) と Integer(1) が同一の canonical・digest を共有する。これは「同一意味は同一 digest」という設計に基づく。>2^53 の Integer は typed canonical が正確な十進を保持し codec tag 形と意図的に異なる唯一の例外として文書化されている。-0.0 は `to_bits()<<1==0` で検出し、float の `==` は使わない（clippy float_cmp を回避、MSRV 1.89）（Issue #692）。

digest は canonical payload の SHA-256 であり、version と revision は source_revision と payload 内 version で表現する。値の codec は `$rv` reserved tag record で、i64 の厳密な往復、Time、ResourceId、Absent/Null を vendor 非依存に扱う（Issue #559）。

## backend 契約と fail-closed の検証順序

LuauBackend::open_session は、VM 生成前に全 fail-closed 検証を実行する。順序は language、host surface（validate_neutral と schema_version）、source size（1 MiB）、source digest の再計算検証、resume digest、source schema_version、budget 次元（Instructions 拒否、MemoryBytes の 0・usize 拒否、WallTime の overflow 拒否）、artifact kind（広告外・複数を拒否）、そして VM 生成である（Issue #716）。source.digest の再計算検証は FakeBackend より厳格であり、ArtifactSource.source_digest が artifact provenance になるため、宣言 digest と text の乖離を境界で拒否する。

evaluate は single-shot である。契約は evaluate 回数を規定しないが、VM 大域状態を持つ session での再評価は大域汚染の曖昧さを生むため、2 回目は Internal を返し、consumer は再評価に新しい session を開く。named entry points を持つ entry=Some も同様に Unsupported とし、silent ignore しない（Issue #716）。

budget の写像は次の通りである。wall_time は interrupt deadline（Instant+Duration の checked_add で overflow を拒否）、memory は set_memory_limit、output は eval 後の canonical 直列化サイズ、host_calls は vacuous に受理、instructions は UnsupportedBudget として fail-closed で拒否する（Issue #716）。mlua には命令数 API がなく、将来 native 側の instruction hook が検討対象である。

host call の上限は vacuous に受理するが、これは silent fail-open ではない。HostApiSurface が記述子のみで handler 実行 seam が契約に存在しないため「host call ≤ limit」は構造的に常に真であり、capability_ids に "host.call" を広告しないことで非対応を正直に表現した（Issue #716）。

## 型による所有権と配置特性の表現

Send/Sync を trait に無条件要求せず、配置特性は BackendCapabilities（send/sync/owner_thread_only/shared_vm_state）と placement_hint で表現する（Issue #692）。runtime では mlua の send feature を有効化しないことで Lua を !Send に保ち、owner-thread-only を型で表現した。send feature を付けると Lua: Send になり ScriptSession の owner-thread-only 性を型で表現できなくなるためである（Issue #716）。analyzer 側では analyze(&self) の内部可変性を Mutex<AnalyzerState> で実装し、NonNull フィールドにより Send/Sync を自動的に非実装として owner-thread affinity を型レベルで表現した（Issue #692）。

中立契約の CancelToken は session 保持の contract token として interrupt closure に clone 注入し、hit flag で cancel と deadline を区別する。pcall 内の interrupt は飲み込まれるため、post-eval で hit flag を再検査する。session.cancel()（既定実装は cancel_token().cancel()）と consumer 側 cancel の両方が同一 flag を立てる（Issue #716）。

## Luau lexer・型定義・診断の実測知見

Luau Lexer の改行規約は `\n` のみである。isNewline は `\n` のみを計数し、`\r` は空白扱いで、単独 `\r` は改行にならず `\r\n` は `\n` で一回計数される。この規約に合わせて shim の offsetToPosition と Rust の position_offset の両方を `\n` のみに統一した。col は行末で clamp し、line_start + col は saturating add とする（Issue #692）。

add_type_definition の再定義は last-write-wins である。Luau の loadDefinitionFile は declared name で global binding を上書きする。shim に unload 経路がないため、loaded_definitions の dedup で冪等にした（Issue #692）。

typecheck の timeout は部分診断を成功で返してはならない。CheckResult.timeoutHits が非空のとき shim は status 6（Internal）を返す。中立契約に partial 信号がなく、部分結果を完全な診断として返すのは fail-open になるため棄却した（Issue #692）。負・NaN の module_time_limit_sec は connect 時に拒否する。

Luau の空 table {} は Record、Rhai の [] は Array と decode される差異がある。demo workflow では references キーを省略して canonical 一致を保ち、inspect_artifact は省略を「参照なし」と受理する。Rhai の map literal は key: value 構文である。真の循環は digest 上構築不能で、防御的に CycleDetected を実装した（Issue #559）。

## 値の lift と sandbox

値の lift 規則は depth≤64、nodes≤1M とする。string key は Record、連続 int key は Array、空は Record、mixed・sparse は Internal、非 UTF-8 は Internal、Function・Thread・UserData 等は Unsupported とする。Number は整数かつ i64 範囲内なら Integer に降格し、非有限は Number のまま canonical tag record で決定的に表現する（Issue #716）。

sandbox は StdLib::ALL_SAFE を基盤とし、require・loadstring・print・debug を Nil で除去する。digest pin 外のコード実行、stdout 汚染、debug 公開を遮断するためである（Issue #716）。

## 版整合とビルド・リンクの運用知見

Luau revision は 0.740 に固定する。版整合は Cargo.lock の解決結果と crates.io の yank 状態を正本として照合する。当初 Luau 0.744 を vendored したが、luau0-src 0.22.1+luau744 が yanked と判明し、mlua 0.12.2→mlua-sys 0.13.0→luau0-src ^0.22.0 の解決が非 yanked の 0.22.0+luau740 に確定したため、Analysis 側も upstream tag 0.740（commit c0e346e）で再 vendor した。shim が使う API は 0.740/0.744 間で同一であることを header 照合で確認済みである（Issue #560）。

VM シンボルは luau0-src の静的ライブラリに依存させ、重複コンパイルによる ODR 違反を回避する。MSVC の左→右リンク走査で bridge ライブラリが Luau ライブラリ群より後に配置され 118 件の未解決シンボルになったため、build.rs が同名 lib を bridge 直後に再 emit して解消した。lib 名は mlua-sys 0.13.0/luau0-src 0.22.0+luau740 の生成物に対応する PoC 固定値であり、更新時に再照合が必要である（Issue #560）。

Windows では Rust と C++ で環境変数の読み書きが別系統になる。Rust の env::set_var/remove_var は Win32 PEB の env block を更新するが、C++ の std::getenv が読む CRT _environ は起動時 snapshot のままであり、fault injection が in-process で観測不能になった。shim の envFlag を Windows では GetEnvironmentVariableA で読むよう修正し、非 Windows は std::getenv とした。教訓は、Rust↔C++ を跨ぐ環境変数 seam では、読む側の API が更新側と同じ env store を参照するかを検証することである（Issue #560）。

MSVC の vcvars 未初期化環境では cl.exe が stddef.h 不在で失敗するため、build.rs は find-msvc-tools で検出した INCLUDE/LIB/PATH を build script プロセスへ適用する。edition 2024 の env::set_var unsafe 規約に SAFETY コメント付きで適合した。また shim の日本語コメント（UTF-8）が CP932 既定で C1020 を起こすため、MSVC target に /utf-8 を付与して解消した（Issue #560）。

shim は C ABI + status code + JSON で境界を構成し、opaque handle・status code・UTF-8 検証・cursor 範囲検査・owner-thread 検査・環境変数 fault injection・JSON 出力を提供する。C++ 型・Luau 型・例外・ポインタを公開境界へ出さない。unsafe は ffi モジュールと cfg(test) の ffi_boundary_tests に #[expect(unsafe_code)] と SAFETY コメントで局所許可する。Analyzer と Runtime は独立であり、解析成功は実行 Grant を意味しない（契約 C5）（Issue #560）。

## budget の限界と capability 広告

instruction/wall-time budget は VM safepoint 近似であり、厳密な命令数ではない。CancelGranularity::Safepoint を capability として広告する（Issue #559）。Rhai backend は MaxMemoryBytes を強制できないため、capability 広告で非対応を明示し、要求時は UnsupportedBudget で fail-closed とする。製品で memory quota が必須なら、Rhai 側に計測 wrapper か process 分離が必要である（Issue #559）。Luau の safe state は stdlib 非公開の最小構成であり、製品で stdlib を開く場合は host surface（os/io/debug/loadstring の遮断）を別途設計する必要がある（Issue #559）。

## 実装上の対策パターン

risk register に記録された対策のうち、再利用可能なものを挙げる。shim ABI/contract version の不一致は fail-closed negotiation と ffi_boundary_tests での fault 検証で対処する。Mutex poison は lock() の Err を Internal typed error に変換し、panic の連鎖を遮断する。vendor tree の line-ending は .gitattributes の eol=lf で正規化する。Luau revision と luau0-src の不一致は両方 0.740 で固定し REVISION を記録する。JSON→中立型写像の情報落ちは spec の写像表で明示的に破棄 field を記録する。cargo の直接実行は補助証拠のみとし、canonical は mise を使う。clippy::struct_excessive_bools や float_cmp は reason 付き #[expect] で局所許可し、#[allow] は使わない（Issue #692）。

trait object の Result（Result<&dyn ScriptBackend,_>、Result<Box<dyn ScriptSession>,_>）は Ok 側が Debug を持たず expect_err が使えないため、match helper を tests/common に追加した。clippy pedantic の指摘（collapsible_if、manual_is_multiple_of、chunks_exact_to_as_chunks 等）はすべてソース修正で解消し、#[allow] は追加しなかった（Issue #692）。

## 未確定・保留事項

Issue #558 の文書スライスでは、実装 crate・公開 API・PoC コードを生成せず契約レベル定義に留め、実 Runtime 共存・三 OS 配布・native FFI は後続 Issue へ、共通 API の製品昇格と Rhai 本番採用は実証後判断へ Deferred とした。これらの最終的な充足状況は文書群からは確定できない。

Issue #692 の AC2（実 Luau backend）と AC3（synvelis 接続）の最終的な充足は、consumer gate（synvelis#271〜#274）の結果次第であり、文書群からは確定できない。

Issue #716 の prek run --all-files の実施結果は記録がプレースホルダのままであり、最終結果は不明である。

Issue #558 のタスク記録には index 再生成・監査・fmt-check・prek・verification 記録・コミットの checkbox が未チェックのまま残っており、これらが実際に完了したかは対象文書の範囲外のため未確認である。

mlua に命令数 API がないため instructions budget は UnsupportedBudget で拒否しており、native 側 instruction hook の検討が将来の課題として残る。Rhai の memory quota を製品で必須とする場合の計測 wrapper または process 分離も未決定である。
