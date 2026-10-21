// Solid 版レベルメータ群（AC5 比較候補の FW 側駆動部）。
// vanilla の bench-shared/meters.ts と同一の DOM 構造
// （#meters > .meter > .meter-fill × METER_CHANNELS）を、
// チャンネル毎の独立 createSignal で fine-grained に駆動する。
// setLevels 1 回で最大 channels 個の signal が更新され、各 .meter-fill の
// style.transform 書き込みだけが起きる部分更新となる
// （VDOM 再描画やノード再生成は発生しない — MutationObserver で検証する対象）。

import { createSignal, For, type JSX } from "solid-js";
import { METER_CHANNELS, type MeterDriver } from "@ui-poc/bench-shared";

/** BenchRunner がフレーム毎に呼ぶメータ更新ハンドル。 */
export type MeterHandle = MeterDriver;

interface MetersProps {
  /** メータチャンネル数（省略時は bench-shared の METER_CHANNELS）。 */
  channels?: number;
  /** runner が setLevels を得るためのハンドル受け渡しコールバック。 */
  ref: (handle: MeterHandle) => void;
  /** #meters コンテナ要素の参照受け渡し（BenchElements.metersEl 用）。 */
  metersRef: (el: HTMLElement) => void;
}

export function Meters(props: MetersProps): JSX.Element {
  const channels = props.channels ?? METER_CHANNELS;
  // チャンネル毎に独立した signal を持つことが fine-grained 更新の要点:
  // 値を再描画で再生成するのではなく、対応 fill の transform だけが書き換わる。
  const levels = Array.from({ length: channels }, () => createSignal(0));
  props.ref({
    setLevels(next: Float32Array) {
      for (let ch = 0; ch < levels.length; ch++) {
        levels[ch][1](next[ch] ?? 0);
      }
    },
  });
  return (
    <div id="meters" ref={props.metersRef}>
      <For each={levels}>
        {([get]) => (
          <div class="meter">
            <div
              class="meter-fill"
              style={{ transform: `scaleY(${get().toFixed(4)})` }}
            />
          </div>
        )}
      </For>
    </div>
  );
}
