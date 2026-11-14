# Issue #262 → #419/#465 引き渡し文面

> 本ファイルは #419（後継 ADR 最終採用）と #465（Legacy WebView 撤去）への引き渡し資料である。**Issue コメント投稿・ADR status 変更・決裁宣言は Coordinator が行う。本 slice は推奨 verdict と根拠表のみを記録する。**

## #419 への引き渡し

### 明示すべきこと

1. **「L1 のみ評価済み、L2/L3 非保証」**
   - OS-L1（3 OS Native Build/Link・Headless UI Scenario/Theme/Driver boundary・必要 Feature・RT 非干渉）は ancestor SHA の CI 証拠で Pass。
   - Windows L2（実 Window/Input/Focus）は能力別に部分実証。IME は `Unsupported` 宣言、a11y/DPI 動的/物理入力は `NotRun`。
   - Windows L3 / Gate D（実 Native Dock）は**未達**。HWND 機構は実測 Pass だが、GPUI 0.2.2 DComp `topmost=true` が child HWND を覆い plugin 画素が非合成（Issue #532 open）。
   - macOS/Linux L2/L3 は Deferred。`native_surface_slot` backend は `Unsupported` 宣言（Capability 欠落として追跡）。
   - Gate C 性能次元・Gate E Consumer Oracle は `NotRun`。

2. **条件付き判定材料**
   - Issue #262 Verdict 規則「全必須 OS/format で D・E が未達なら GPUI 標準 Driver 方針維持、Native Dock の無条件本番採用不可と明記し #419 を accepted にしない」に該当。
   - GPUI Native First を標準 Driver とする設計方針は維持。Driver-neutral 公開契約は Gate A で裏付け済み。
   - Native Dock の無条件本番採用は不可。条件付き/未受入を明記。

3. **OS 別 Capability 記録の要求**
   - `native_surface_slot`: Windows `Supported` / macOS・Linux `Unsupported`（宣言）
   - `ime`: 全 OS `Unsupported`（宣言）
   - `focus` release: `Err(Unsupported)`（宣言）
   - これらを Supported/Unsupported/NotRun の OS 別・能力別記録として ADR に反映する。

### verdict-matrix 参照

個別 Case の expected/observed/verdict/証拠 URL は `evidence/rf262-gpui-verdict/verdict-matrix.md` を正本とする。

## #465 への引き渡し

### 独立判断の境界

- #465（Legacy WebView 撤去）は **L1 機能/既存 Consumer 互換** と **未実 GUI 製品機能の破壊** を独立判断する。
- 本 slice（#262）の L1 採用記録だけで、製品 GUI 機能や実 Plugin 表示を保証しない。
- rf465 inventory の分類（`reuse`/`provisional`）は L1 互換の傍証であり、製品 Oracle の Pass ではない。
- 未実 GUI 製品機能（実 Native Dock 画素・実 plugin editor 表示・Consumer 製品 GUI 受入）は Gate D/E が未達のため、#465 の撤去判断で「実装済み・動作確認済み」として扱ってはならない。

### 参照

- `evidence/rf465/README.md` + `inventory.json`
- `docs/specs/issue-465-legacy-webview-p0p1.md` / `docs/designs/issue-465-legacy-webview-p0p1.md`

## Coordinator への確認事項

- verdict の最終決裁（accepted / conditional / rejected）は Coordinator 権限。
- HEAD `7aab6302` の CI run が merge 後に実施されるか（統合受入の証拠）。
- 本引き渡し文面を #419/#465 の Issue コメントとして投稿するか（投稿は Coordinator）。
