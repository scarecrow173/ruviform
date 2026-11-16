# Issue #532 foreground/compositor mutation probe — 実装実験記録（2026-11-16）

`docs/tasks/issue-532-foreground-probe.md` の slice（GPUI root の
z-order/foreground 変異 probe + `real-dcomp-foreground` case）実装で
試行・観測した事項の記録。実装結果の正本は
[`docs/impl/ir/issue-532-foreground-probe.md`](../ir/issue-532-foreground-probe.md)、
イベントログは
[`issue-532-foreground-probe-exp.jsonl`](issue-532-foreground-probe-exp.jsonl)
を参照。

## 試行 1: mutation 境界の型設計（Win32 型を出さない）

- **目的**: `osoccl`/`ospix` の query-only 契約を守りつつ、window 状態の
  snapshot・変異・復元を一箇所に隔離する module を新設する。
- **観測**: 既存 module は HWND を `u64` に narrow して受け渡す規約
  （`oschild`/`ospix`/`osoccl`）。Win32 型をそのまま公開すると境界が
  漏れる。
- **結論**: `osmut.rs` は `RootSnapshot`/`MutationOutcome`/
  `RestoreOutcome` の plain 値 record のみを公開し、HWND は `u64`、
  exstyle は `osoccl::ExstyleFacts` を再利用。unsafe は FFI wrapper に
  限定し `#[expect(unsafe_code)]` + SAFETY コメント。非 Windows は
  `supported:false` の honest stub。

## 試行 2: `GA_ROOT` 解決と z-order walk の境界条件

- **観測**: child HWND → `GetAncestor(GA_ROOT)` で GPUI root を解決。
  `GW_HWNDPREV` walk は desktop の window 数に上限が無いため cap が必要。
- **結論**: `ZWALK_CAP=512` で打ち切り `truncated` flag を記録
  （`above_count` は下限値として正直に扱う）。exstyle 読取失敗は
  `Err` → `exstyle:None` に縮退させ snapshot 全体を落とさない。
  `SetLastError(0)` → call → `GetLastError` の対で refused call と
  正当な 0 を区別する（`mutation`/`set_foreground`/`exstyle` 共通）。

## 試行 3: clippy pedantic `-D warnings` への 2 つの対応

- **観測**: `real-dock` feature の lint で `too_many_lines` が
  `real_entries`（宣言的 catalog table）と `foreground_probe_children`
  （probe 本体）の 2 箇所に出た。
- **結論**: table は `#[expect(clippy::too_many_lines)]`（既存先例に
  一致）。probe 本体は expect ではなく helper 抽出
  （`observe_all_children`/`record_apply_row`/`run_probe_stages`）で
  stage 順序と失敗記録の構造を保ったまま行数を下げた — expect で
  隠すより読みやすい側を選んだ。`map().unwrap_or()` は `map_or` に修正。

## 試行 4: fmt reflow による edit 不一致

- **観測**: `cargo fmt` 実行後、`foreground_probe_children` の
  `old_string` が不一致で edit が失敗 — fmt が let-chain 等を
  reflow していた。
- **結論**: fmt 後の実体を読み直してから refactor を適用。
  「fmt を回したら対象関数を再読する」をこの slice の運用として確認。

## 試行 5: verdict と bisect data の分離

- **目的**: green 画素・hit-test 結果を verdict 入力にしない
  （design の中核契約）。
- **結論**: `ProbeOutcome{evaluated,executed,restored,failure}` が
  verdict 入力の全て。`executed` は必須 `SetWindowPos` の適用のみで
  判定し、`SetForegroundWindow` の foreground-lock 拒否は `note`
  データ（fail にしない）。restore 検証は `topmost_restored` +
  `SetWindowPos` 系 op の適用のみ — z-order/foreground は
  `before/after` を記録する best-effort で「復元した」とは claim
  しない。

## 試行 6: review-protocol で見つかった契約ずれの修正

- **観測**: 2 名の独立 reviewer（cooperative/adversarial）の
  Issue Candidate を検証した結果、コードが自分の doc/design より
  弱い箇所が 3 件あった。
- **結論**: (a) foreground hand-back に `IsWindow` liveness guard が
  無く design の「生存 window」契約とずれていた → z-order op と同じ
  guard を追加。(b) `topmost_restored` が final exstyle 読取失敗時に
  `baseline.topmost==false` なら vacuous true になり得た →
  `exstyle.is_some()` を AND 条件にして fail-closed 化。
  (c) `not_evaluable` が reason を case detail に伝播せず generic
  fallback に落ちていた → `failure` に reason を保持。
  併せて doc drift（"thirteen"→"fourteen"、`--pixel-artifacts` の
  対象 case、PluginPair 消費 case 一覧、design の "sub-second"
  リスク記述）を修正。修正後は fmt/lint/test と interactive lane
  2 本を最終 binary で再実走した。

## 試行 7: interactive 実走で得た bisect データ

- **観測**: baseline（DComp on）と `GPUI_DISABLE_DIRECT_COMPOSITION=1`
  の 2 lane で probe は両方 `pass`、`real-dcomp-pixel` は両方 `fail`
  （authoritative 維持）。hit-test の遮蔽物は run ごとに変わる
  （`Chrome_RenderWidgetHostHWND` / `Zed::Window`）— design の
  「環境依存」想定どおり。DComp off run では `HWND_TOP` のみで
  hit-test が自 child HWND に解決したが screen green は 0 のまま。
- **結論**: 外部 HWND stacking は第一の境界として確定。hit-test が
  自 HWND を返す stage でも green が出ない = HWND z-order で説明
  できない compositor/presentation 内部の第二の境界が存在。
  `WindowFromPoint` は DComp visual/plane を見えないため残余機構は
  未断定 — `evidence/rf532/foreground-probe-bisect.md` に bounded
  な結論として記録。Issue #532 は open のまま。
