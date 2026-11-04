/**
 * Playwright config (Issue #310).
 *
 * One pinned browser project — Chromium is the harness's reference
 * lane; additional engines are additive capability evidence, never a
 * different semantic contract. Determinism: fixed viewport, DPR=1
 * (a `dpr2` describe in the viz spec covers the scaled path), the
 * harness-installed clock, and the hermetic route serve layer.
 *
 * Evidence: `test-results/evidence.json` (env/revision/seed/outcome)
 * + Playwright trace on failures (retain-on-failure) — the CI lane
 * uploads `test-results/` as its pass/fail/not-run record.
 */

import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "test",
  testMatch: "**/*.spec.ts",
  outputDir: "test-results",
  timeout: 30_000,
  expect: { timeout: 5_000 },
  // Browser E2E asserts semantic DOM/canvas state — a retry would mask
  // a genuinely flaky contract, so keep retries off locally; CI gets
  // the same single attempt and records the honest outcome.
  retries: 0,
  reporter: [
    ["list"],
    ["./src/evidence.ts"],
  ],
  use: {
    viewport: { width: 1280, height: 720 },
    deviceScaleFactor: 1,
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
    video: "off",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
});
