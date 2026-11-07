# ruviform-ui-webview

`ruviform-ui` ポートの `wry` + `tao` 実装: ウィンドウ/イベントループ・
WebView ホスト・`rvf://` カスタムプロトコル・組込みアセット・
バックエンド検出を提供するオプションのプラットフォーム adapter
(Issue #156)。採択済み UI ADR(WebView-first hybrid)に従う。

## 責務

- セッションホスト(`session` モジュール・feature `webview`・Issue #181 / #423):
  `run_multi` が 1 本の tao イベントループで複数 window/webview を管理し、
  driver-neutral な `WindowDriver` を consumer へ渡す。各 window に安定
  `WindowId` を発行し、`DriverEvent::Windowed` で起点を明示する。
  `CloseRequested` は veto 可能な要求(既定は対象 window のみ `Approve`)で、
  破棄は `Destroyed` で通知。`EventLoopProxy` による WebView2 コールバックの
  wakeup、resize / scale-factor での WebView bounds 再適用、Tick 転送を担う。
  全 window で 1 つの `WebContext`(user-data dir)を共有し、
  last-window / app-exit 判断は consumer 側の `WindowDriver::exit` 呼出しに委ねる。
- 写像(`host` モジュール・feature `webview`・Issue #423 / 移行表 H1–H5):
  - `post_event` → `evaluate_script(dispatch_script(..))`。未 `Ready` WebView へは
    有界 backlog(古い順 drop)、既破棄宛ては黙って捨てる。
  - wry の page load → `WindowLifecycle::Ready`(ページ URL は契約へ出さない)。
  - `window.ipc.postMessage` 本文 → `parse_intent` → `WindowLifecycle::Action(Intent)`。
    不正な IPC は当該 window へ `Event::error` を返し、consumer へは渡さない。
  - 旧 `HostEvent` / `HostHandle` / `WindowConfig` はこの crate で使わない。
    WebView 固有の URL・devtools は `WebViewWindowConfig` に局所化する。
- スキーム(`scheme` モジュール・feature 非依存): `rvf://app` アセット配信と
  `rvf://data/<name>` 有界 range pull。WebView2 が書き換える
  `http://rvf.app` / `http://rvf.data` ホスト形も受理する。
  ナビゲーションはアプリ起点 URL と `about:blank` のみ許可。
- 組込みアセット(`embed` モジュール): `rust-embed` で
  `examples/demo_app.html` 等を単一バイナリへ埋め込む。
- 検出(`detect` モジュール): `wry::webview_version` による
  `BackendReport` 生成(バックエンド未導入時は `BackendState::Missing`)。

## feature

- `webview`(非 default): `wry`/`tao`/`http`/`dpi`/`rust-embed` を有効化し
  ホスト実装を公開する。feature なしでは `scheme` の純粋ロジックのみ
  コンパイルされるため、headless 環境でもスキーム単体テストが可能。

## OS 依存

| OS | バックエンド | 要件 |
| --- | --- | --- |
| Windows | WebView2 | WebView2 Runtime(Win10 1803+ の多くに同梱・未導入時は Evergreen ブートストラップ)。プロセスごとの user-data フォルダを使う(PoC 実測のシングルトンロック対策) |
| macOS | WKWebView | macOS 10.15+(wry 要件)。実機未検証 |
| Linux | WebKitGTK | `libwebkit2gtk-4.1-dev` `libgtk-3-dev` `libsoup-3.0-dev` `libjavascriptcoregtk-4.1-dev`。実機未検証 |

既知の制約(PoC 実測由来・詳細は discovery 文書):
WebView2 はカスタムプロトコル URL を `http://` ホストへ書き換える、
composition でネイティブ面が常に最背面にならない制限があるため
`wgpu` 面は別 adapter、小さな pull(~64 KiB)は JSON RTT より遅い
(閾値以上のバルクのみ pull を使う)。

## `rvf://data` paging 契約

`DataRegistry` 登録済みの名前付きバイナリを frontend が範囲 pull
する wire 契約(Issue #173)。大容量データを JSON IPC に乗せない
3-path 境界の pull 側であり、以下は `scheme` モジュールの実装と
1:1 に対応する(docs/designs/issue-156-gui-host-foundation.md の
プロトコル節が上位仕様)。

### リクエスト

`rvf://data/<name>?offset=N&len=M`(Windows の WebView2 書換え形
`http://rvf.data/<name>?...` も同じ解約を受ける)。

| 要素 | 契約 |
| --- | --- |
| `<name>` | Rust 側が `DataRegistry::register` で発行した URI-safe slug。`/` 区切りの多セグメント可(`demo/peaks` 等)。handler は percent-decode しないため slug をそのまま使う。空セグメント・`..` セグメントは拒否。frontend から新規ソースは開けない |
| `offset` | 省略時 `0`。`u64` の非負整数でなければ 400 |
| `len` | 省略時「cap まで」。`u64` の非負整数でなければ 400。実返却長は `min(len, total - offset, max_chunk)` で**要求より短くなり得る** |
| その他 query key | 無視。`=` を含まない pair は malformed で 400 |

### レスポンス

成功(200)の header:

| header | 値 |
| --- | --- |
| `Content-Type` | `application/octet-stream` |
| `Access-Control-Allow-Origin` | `*` |
| `Cache-Control` | `no-store` |
| `X-Rvf-Total` | ソース全長(バイト・10 進文字列)。**全 200 応答に付く — 全長はこの header で取得する** |

エラー(status):

| status | 条件 |
| --- | --- |
| 400 | name 検証拒否・`offset`/`len` の非数・負数・`=` なし pair・`offset > total` |
| 404 | `<name>` が未登録(`DataError::NotFound`) |
| 500 | 登録済みソースの read 失敗(`DataError::Read` 等) |

`offset == total` は範囲外ではなく境界値 — 200 で空 body
(`X-Rvf-Total` 付き)を返す。paging loop の正常終端到達点として
扱える。

### per-request cap

1 リクエストの返却上限は `DataRegistry::max_chunk`。既定値は
`ruviform_ui::DEFAULT_MAX_CHUNK` = **16 MiB**(16 × 1024 × 1024)で、
アプリ側が `DataRegistry::with_max_chunk` で変更する。根拠は
「webview 境界をまたぐ単発転送の応答・メモリ常駐を抑え、進捗可
観測なチャンク列に分割する」pull 設計(PoC 実測 pull ~94–146
MB/s)であり、全量一括レスポンスを意図的に提供しない。

cap を超える `len` はエラーではなく黙って切詰められる — 実際に
届いた量は常に返却 body 長で判断すること。cap の実値と登録済
みソース一覧は `sys.capabilities` の `data.max_chunk` /
`data.sources` で実行時に取得できる(frontend がハードコード
せず済む)。

### paging loop(frontend)

全量が `max_chunk` を超える場合は `X-Rvf-Total` まで繰返し pull
する。`offset` は要求した `len` ではなく**実返却バイト数**で進
め、累積が `X-Rvf-Total` に達したら終了する。

```js
// __rvf.dataUrl(name, offset, len) は bridge が注入する URL 生成。
async function pullAll(name) {
  const caps = await __rvf.call('sys.capabilities');
  const CHUNK = caps.data.max_chunk; // 実行時の実 cap(既定 16 MiB)
  const parts = [];
  let total = 0, offset = 0;
  do {
    const r = await fetch(__rvf.dataUrl(name, offset, CHUNK));
    if (!r.ok) throw new Error(`rvf://data ${name}: HTTP ${r.status}`);
    total = Number(r.headers.get('X-Rvf-Total'));
    const chunk = new Uint8Array(await r.arrayBuffer());
    parts.push(chunk);
    offset += chunk.byteLength; // 実返却長で進める
  } while (offset < total);
  const out = new Uint8Array(total);
  let at = 0;
  for (const c of parts) { out.set(c, at); at += c.byteLength; }
  return out;
}
```

- `X-Rvf-Total` は全 200 応答に付くため、1 回目で全長を確定できる。
- `DataSource` は登録後不変 — 同一 `<name>` の `total` は pull 途中
  で変わらない(更新は別 name で公開される)。
- 結果 event が既に byte 数を返す場合は `X-Rvf-Total` 確認を省略
  できる — `examples/demo_app.html` の `pullPeaks` がその形。

## 層・許可依存

- 層: Optional platform adapter(利用側層)。
- 許可依存: `ruviform-ui` のみ(foundations への下向き強制・
  adapter→adapter 依存は作らない)。

## RT 契約

非 RT 専用。ウィンドウ/イベントループ/UI スレッド処理であり
RT パスからは呼ばない。

## 利用

```sh
# GUI デモ(ruviform-ui の demo_cli と同一 DemoService)
cargo run -p ruviform-ui-webview --features webview --example demo_app
# マルチウィンドウ デモ(Issue #181・実 OS webview backend が必要)
cargo run -p ruviform-ui-webview --features webview --example demo_multi
```

検証タスク(feature 有効 lane):
`mise run check-ui-webview` / `lint-ui-webview` / `test-ui-webview`。
CI は `ui-verify` ジョブが 3 OS matrix で実行する
(Linux は WebKitGTK 開発パッケージをジョブ内で導入)。

## 関連

- コア: [`ruviform-ui`](../ruviform-ui/README.md)
- 設計: docs/designs/issue-156-gui-host-foundation.md
- UI ADR: docs/adr/ui-architecture-webview-hybrid.md
