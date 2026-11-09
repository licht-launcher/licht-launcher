import { mkdtempSync, rmSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { stopDriversStartedByThisRun } from "./support/drivers.ts";
import { switchToMainWindow } from "./support/windows.ts";

const rootDir = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const binary = process.platform === "win32" ? "licht.exe" : "licht";
// Built by `pnpm build:e2e`; the Cargo workspace puts it in the root target folder.
const appBinaryPath = path.join(rootDir, "target", "debug", binary);
// The launcher process loads this file first and creates the folder; test workers
// load it again but inherit the variable, so they reuse it instead of leaking theirs.
const ownsDataDirectory = process.env.LICHT_E2E_DATA_DIR === undefined;
const dataDirectory =
  process.env.LICHT_E2E_DATA_DIR ??
  mkdtempSync(path.join(os.tmpdir(), "licht-e2e-"));
process.env.LICHT_E2E_DATA_DIR = dataDirectory;

// The launched app inherits these: a throwaway data folder instead of the user's,
// and no background update checks reaching the network (debug builds only).
process.env.LICHT_DATA_DIR = dataDirectory;
process.env.LICHT_SKIP_UPDATE_CHECK = "1";

const tauriDriverPath = process.env.TAURI_DRIVER_PATH;
// CI keeps detailed WebdriverIO, driver and app logs as files to upload on failure.
const ci = process.env.CI === "true";

export const config: WebdriverIO.Config = {
  runner: "local",
  specs: ["./specs/**/*.e2e.ts"],
  maxInstances: 1,
  capabilities: [{ browserName: "tauri" }],
  services: [
    [
      "@wdio/tauri-service",
      {
        appBinaryPath,
        driverProvider: "external",
        captureBackendLogs: ci,
        ...(tauriDriverPath === undefined ? {} : { tauriDriverPath }),
      },
    ],
  ],
  logLevel: ci ? "info" : "warn",
  ...(ci ? { outputDir: path.join(rootDir, "e2e", "logs") } : {}),
  waitforTimeout: 15_000,
  connectionRetryTimeout: 120_000,
  connectionRetryCount: 1,
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: {
    ui: "bdd",
    timeout: 120_000,
  },
  async before() {
    await switchToMainWindow();
  },
  onComplete() {
    stopDriversStartedByThisRun();
    // Never delete a folder this run did not create.
    if (ownsDataDirectory) {
      rmSync(dataDirectory, { recursive: true, force: true });
    }
  },
};
