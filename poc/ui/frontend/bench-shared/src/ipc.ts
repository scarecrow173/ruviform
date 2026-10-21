// window.ipc.postMessage ブリッジ + Rust evaluate_script 受け口（FW 非依存）。
// wry は JS 側に `window.ipc.postMessage(string)` を注入し、
// 呼ばれた JSON 文字列を Rust 側 `with_ipc_handler` へ届ける。
// 逆方向は `evaluate_script` で `window.__poc.*` を呼ぶ形を共通契約とする
// （host 実験ページと同じ受け口名を使う）。

/** `window.ipc.postMessage` で JSON intent を Rust へ送る（ブリッジ不在なら無視）。 */
export function postMessage(obj: unknown): void {
  const w = window as unknown as {
    ipc?: { postMessage?: (msg: string) => void };
  };
  w.ipc?.postMessage?.(JSON.stringify(obj));
}

/** Rust 側が evaluate_script 経由で呼ぶ `window.__poc` 受け口を登録する。 */
export function installApi(api: Record<string, unknown>): void {
  (window as unknown as { __poc?: unknown }).__poc = api;
}
