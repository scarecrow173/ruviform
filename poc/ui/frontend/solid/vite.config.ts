import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

// poc://app/solid/ 配下からの相対解決が必要なため base は "./"。
// dist は frontend/solid/dist に出し、frontend/dist への集約はルート build.mjs の責務。
// vite-plugin-solid が JSX を fine-grained DOM 更新（createSignal 駆動の
// 部分適用）へコンパイルする（babel-preset-solid 経由）。
export default defineConfig({
  base: "./",
  plugins: [solid()],
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
});
