// レベルメータ群（FW 非依存の参照実装）。
// DOM 構築 + transform scaleY 更新は「メータ更新当たり 1 DOM mutation」の
// 下限参照として vanilla がそのまま使い、Solid 等の FW 実装は
// 同一 DOM 構造を FW 固有の更新経路で駆動する（AC5 比較の基準線）。

/** メータ群の DOM を構築し、値更新関数を返す。 */
export function createMeterHost(
  container: HTMLElement,
  channels: number
): { setLevels: (levels: Float32Array) => void } {
  const fills: HTMLElement[] = [];
  for (let i = 0; i < channels; i++) {
    const meter = document.createElement("div");
    meter.className = "meter";
    const fill = document.createElement("div");
    fill.className = "meter-fill";
    meter.appendChild(fill);
    container.appendChild(meter);
    fills.push(fill);
  }
  return {
    setLevels(levels: Float32Array): void {
      for (let ch = 0; ch < fills.length; ch++) {
        const v = levels[ch] ?? 0;
        fills[ch].style.transform = `scaleY(${v.toFixed(4)})`;
      }
    },
  };
}
