# Issue #532 presentation-probe — 実験試行記録（2026-11-16）

`docs/tasks/issue-532-presentation-probe.md` の diagnostic probe
slice で試行した手法と結果の記録。raw event log は
[`issue-532-presentation-probe-exp.jsonl`](issue-532-presentation-probe-exp.jsonl)、
判別結果の正本は
[`evidence/rf532/presentation-probe.md`](../../../evidence/rf532/presentation-probe.md)
を参照。

## 試行 1: `EnumWindows` top-level inventory による cover 列挙

- **目的**: child rect と交差する top-level window を HWND 層で
  網羅列挙し、hit-test-transparent な外部 cover の有無を調べる。
- **試行**: `ospres::inventory()`（cap 256 + truncated 記録）を
  probe 冒頭で 1 回取得し、rect 交差・visible・alive で
  cover candidate を選定（`COVER_CAP=16`）。
- **観測**: 449 top-level windows 中 cap 256 で打ち切り — 自
  process window は snapshot に 0 件。child rect 交差の visible
  foreign cover（`Chrome_WidgetWin_1` 等）は複数記録されたが、
  `WindowFromPoint` が既に foreign `Chrome_RenderWidgetHostHWND`
  を返しており HWND 層の cover は osoccl facts で確定済みだった。
- **結論**: inventory は「見えていない cover」の検出には寄与したが
  cap により z-order 全体像（自 window の位置）は取れず、
  `osmut` snapshot の `above_count` が実用上の代替指標となった。
  cap/選定順の改善は残課題として記録（probe verdict には不使用）。

## 試行 2: `PrintWindow` flag 4 variants による capture 経路分離

- **目的**: child の green が「window 自身の描画結果」に存在するか、
  「DWM/合成経路が返す内容」に存在するかを分離する。
- **試行**: flags ∈ {0, `PW_CLIENTONLY`, `PW_RENDERFULLCONTENT`,
  both} を child/container/root に適用し、既存 window-DC・
  screen BitBlt と併記。
- **観測**: flags=0/`PW_CLIENTONLY` は child で green=1.0
  （WM_PRINT 需要駆動で描画可能）。`PW_RENDERFULLCONTENT` 系は
  全 target で flat dark raster（green=0）。window-DC は
  DComp-off で child/root green 優位、DComp-on で child/root=0。
- **結論**: green の存在は描画要求への応答と back surface の
  状態に依存し、合成経路（RENDERFULLCONTENT）では観測されない。
  「child が surface に green を保持しているか」は DComp mode で
  異なり、screen=0 の説明として「surface 非保持」と「surface 保持
  するが合成に載らない」の両方が残る — settle 付き再観測が必要。

## 試行 3: screen-vs-suspect quantized fingerprint

- **目的**: screen raster が cover candidate/chain のいずれかの
  raster と一致するかで「何が写っているか」を推定する。
- **試行**: 5bit/channel bin fingerprint を overlap 領域で比較
  （`FINGERPRINT_CAP=8`）。
- **観測**: chain 2 + suspect 4 全て match_fraction=0.0。screen
  top color は light-gray — Chrome 系 window の `PrintWindow` は
  実画素を返さないため、Chrome そのものとの一致判定は不能。
- **結論**: fingerprint は「screen ≠ 既知 suspect raster」までを
  示すに留まり、外部 compositor（Chrome）相手には無力。
  pixel artifact PNG の histogram 解析（本 slice で追加実施）が
  実質的な代替証拠となった — topmost の flat dark が新規 surface
  であることを確認。

## 試行 4: 旧 run JSONL との対比で再現性を確認

- **目的**: 今回の topmost での screen=0 が環境偶然でないか検証。
- **試行**: HEAD 保存の foreground-probe slice JSONL 2 本
  （baseline/dcomp-disabled）から stage 別 hits/screen を抽出。
- **観測**: 旧 2 run + 新 2 run の計 4 run 全てで topmost hits=
  自 child・screen green=0。baseline の cover 相手は環境ごとに
  異なる（Chrome / 別 GPUI アプリ `Zed::Window`）。
- **結論**: 「uncovered（HWND 層）でも screen に child 画素が
  現れない」は環境非依存に再現する観測事実。ただし全 run で
  settle 無し即時 capture のため pre-paint frame 説は残る。
