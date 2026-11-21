# Issue #857 — `real-dcomp-pixel` 実走記録（class B 処遇）

`mise run test-ui-plugin-dock-real`（interactive Windows / GPUI 実
window lane `rf631.plugin-dock-real`）の実走記録。対象コードは
`bugfix/857-pixel-occlusion` ブランチ上の変更（base SHA
`9c9c1e896a1276adfcfce61e4008caa94476341d`、run row の `repo.sha`
も同一）。canonical lane 出力は
`evidence/rf631/plugin-dock-real-scenario.jsonl`（lane が再生成）で、
本ディレクトリには `real-dcomp-pixel` の case 行抜粋
（`plugin-dock-real-pixel-pass.jsonl`）と run row
（`plugin-dock-real-run.jsonl`）を残す。

## 結論（最終 run — AC1 達成）

| 項目 | 実測値 | 判定 |
| --- | --- | --- |
| case verdict（`real-dcomp-pixel`） | `pass` | AC1 達成 |
| `screen_pixels`（backend `win32-bitblt-screen`） | green_fraction **0.999994**（green 1,019,838 / 1,019,844 px、1278x798） | `>= 0.5` 達成 |
| `occlusion_hit_test` role（5 probe point） | 全点 `editor_child`（`RuviformTestEditor` hwnd 27856070） | 被測定 hierarchy 内 — AC3 達成 |
| `occlusion_exstyle`（root 282857608） | `noredirectionbitmap: false`（DComp on、GPUI fork `v0.2.2-rf2`）・`topmost: true`（bounded 昇格の採取時点の正直な記録） | DComp on 実証 |
| `deocclusion_apply_raise` | `applied: true` — `after.above_count` 36→33 | mutation 記録済 |
| `deocclusion_apply_topmost` | `applied: true` — `after.topmost: true`・`above_count: 8`（topmost band 先頭） | mutation 記録済 |
| `deocclusion_restore` | `topmost_restored: true`・z-order 36→36・foreground 34018736 に hand-back | bounded + restore 契約達成 |
| lane exit code | 1（class C `real-dcomp-bandescape`/`real-dcomp-childinv` 既知 Fail のみ — 想定どおり・判定対象外） | 非回帰 |

## 開発中の 3 回実走（fail-closed gate の実証 — AC2 証拠）

遮蔽除去 arm の確定過程で同一ホスト上の interactive lane を 3 回
実走した。初回・2 回目は遮蔽が解けない環境状態（外部
`Chrome_RenderWidgetHostHWND` が topmost band で被測定 rect 全域を
被覆）をそのまま再現しており、**fail-closed gate が `NotRun` に
退避する実挙動**を実走で確認した（AC2 の live 証拠）:

1. **`raise_to_top` 単独 arm**: `deocclusion_apply_raise`
   applied=true（`above_count` 36→33 = non-topmost band 先頭）の後も
   `occlusion_hit_test` 全 5 点が外部 `Chrome_RenderWidgetHostHWND`
   （0,0,1920x1032、topmost band）role `other` を返した → gate が
   `PixelObservation::Occluded` → case verdict **`not_run`**（detail:
   `...external top-level window (hwnd 6625744 class
   Chrome_RenderWidgetHostHWND) — pixels outside the measured
   hierarchy are never read as Pass or Fail`）。`screen_pixels` は
   green 0.0（外部 window の画素）だが verdict には反映されず、
   `screen_pixels.occluded { occluded_points: 5, covering: ... }`
   extras として記録。
2. **`set_topmost` 単独 arm（cold `HWND_TOPMOST`）**:
   `SetWindowPos(HWND_TOPMOST)` が `applied: true` を返しながら
   after-snapshot 上 `topmost: false`・`above_count` 36 のまま変化
   なし — この時点での cold band 変更が（既知順序 `raise` →
   `topmost` と異なり）OS に受理されない実測を採取。hit-test は
   引き続き `other` を返し → case verdict **`not_run`**（同 gate
   経路）。`deocclusion_restore` は baseline 復元を達成。
3. **`raise` → `set_topmost` pair（`real-dcomp-foreground` の
   実績順序に合わせた確定 arm）**: `deocclusion_apply_raise`
   applied → `deocclusion_apply_topmost` applied・`after.topmost:
   true`・`above_count: 8`（topmost band 先頭）→ 全 hit
   `editor_child` → `screen_pixels` green 0.999994 → case verdict
   **`pass`**（上表）。retry 経路（`deocclusion_settle` +
   `deocclusion_apply_topmost_retry`）は未発火 — `raise` 先行で
   band 変更が受理された。

## run 条件

- OS: Windows（interactive desktop、本ホスト実行）・arch `x86_64`
- コマンド: `mise run test-ui-plugin-dock-real`
- lane: `rf631.plugin-dock-real`（4 window mount — 被測定 w1 +
  sentinel w2–w4、単一 display で w2–w4 が display clamp により
  client origin 192,120・1536x864 に着地し w1 と重複する配置）
- GPUI 依存: fork `v0.2.2-rf2`（#839 の DComp fix 統合済み —
  `occlusion_exstyle` root `noredirectionbitmap: false` で DComp
  有効を実証）
- 外部環境: 本ホスト上に `Chrome_RenderWidgetHostHWND` 等の
  topmost band window が存在する実デスクトップ — class B の姉妹
  window 遮蔽に加え外部 topmost 遮蔽も同時に存在する環境で
  de-occlusion が機能した

## AC 対応まとめ

- **AC1**: 最終 run で case verdict `pass`・screen green 0.999994 —
  達成（`plugin-dock-real-pixel-pass.jsonl`）。
- **AC2**: fail-closed gate はコード契約＋初回/2 回目 run の実走
  `not_run`（外部 topmost 残存時に外部画素を Pass/Fail へ読み替え
  ない退避）で実証。`docs/specs/issue-857-pixel-occlusion.md`・
  `docs/test-specs/issue-857-pixel-occlusion.md` に意味論を確定記録。
- **AC3**: 本ディレクトリの JSONL 抜粋＋本記録（SHA・run 条件・
  hit role・green・DComp on・mutation/restore outcome）。
- lane exit code 1 は class C 既知 Fail のみ — `real-dcomp-pixel`
  自体の verdict は `pass`。
