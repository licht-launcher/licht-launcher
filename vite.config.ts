import { execFileSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const rootDir = path.dirname(fileURLToPath(import.meta.url));
const buildRevision = execFileSync("git", ["rev-parse", "--short=7", "HEAD"], {
  cwd: rootDir,
  encoding: "utf8",
}).trim();
const buildPlatform =
  process.env.TAURI_ENV_PLATFORM ??
  (process.platform === "win32" ? "windows" : process.platform);
const buildArchitecture = process.env.TAURI_ENV_ARCH ?? process.arch;
if (
  !/^[a-f0-9]{7,40}$/.test(buildRevision) ||
  !/^[a-z0-9_]+$/.test(buildPlatform) ||
  !/^[a-z0-9_]+$/.test(buildArchitecture)
) {
  throw new Error("Invalid build identification metadata");
}

export default defineConfig({
  define: {
    "import.meta.env.VITE_BUILD_REVISION": JSON.stringify(buildRevision),
    "import.meta.env.VITE_BUILD_PLATFORM": JSON.stringify(buildPlatform),
    "import.meta.env.VITE_BUILD_ARCHITECTURE":
      JSON.stringify(buildArchitecture),
  },
  plugins: [
    tanstackRouter({
      target: "react",
      autoCodeSplitting: true,
      quoteStyle: "double",
      semicolons: true,
    }),
    react(),
    tailwindcss(),
  ],
  resolve: {
    alias: {
      "@": path.resolve(rootDir, "src"),
    },
  },
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    port: 5173,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**", "**/crates/**", "**/target/**"],
    },
  },
});
