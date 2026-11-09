import { mergeConfig } from "vite";
import { defineConfig } from "vitest/config";
import viteConfig from "./vite.config.ts";

export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      environment: "jsdom",
      setupFiles: ["./src/test/setup.ts"],
      include: ["src/**/*.test.{ts,tsx}"],
      // Router + Tauri mocks are slower when many files run in parallel.
      testTimeout: 15_000,
      coverage: {
        provider: "v8",
        include: ["src/**/*.{ts,tsx}"],
        exclude: [
          "src/routeTree.gen.ts",
          "src/lib/generated/**",
          "src/test/**",
          "src/**/fixtures/**",
          "src/**/tests/support/**",
        ],
        // `summary.txt` feeds the CI job summary; the HTML report is uploaded as an artifact.
        reporter: ["text", ["text-summary", { file: "summary.txt" }], "html"],
      },
    },
  }),
);
