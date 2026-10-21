import { defineConfig } from "vite";

// poc://app/ipc/ 配下からの相対解決が必要なため base は "./"。
// dist は frontend/ipc/dist に出し、frontend/dist への集約はルート build.mjs の責務。
export default defineConfig({
  base: "./",
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
});
