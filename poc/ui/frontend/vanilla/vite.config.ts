import { defineConfig } from "vite";

// poc://app/vanilla/ 配下からの相対解決が必要なため base は "./"。
// dist は vite.config ではなく poc/ui 直下ではなく frontend/vanilla/dist に出す
// （frontend/dist への集約はルート build.mjs の責務）。
export default defineConfig({
  base: "./",
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
});
