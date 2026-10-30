export const playerHeadPathFixture =
  "C:\\Licht\\cache\\player-heads\\default.png";

export const snapshot = {
  settings: {
    schema: 1,
    theme: "dark",
    username: "Steve",
    fullscreen: false,
    width: 1280,
    height: 720,
    maxMemoryMb: 2048,
    jvmArguments: [],
    java25: null,
    java21: null,
    java17: null,
    java8: null,
    downloadConcurrency: 8,
    writeConcurrency: 8,
    downloadSpeedLimitBytesPerSecond: null,
    dataDirectory: null,
    disableCustomTitleBar: false,
    disableTransparency: false,
    automaticUpdates: true,
    language: "en",
    hardwareAcceleration: true,
    minimizeToTray: true,
    minimizeOnGameStart: false,
    restoreOnGameExit: false,
  },
  applicationDirectory: "C:\\Users\\Mateus\\AppData\\Roaming\\Licht",
  memoryTotalMb: 16384,
  memoryRecommendedMb: 4096,
  memoryMinimumMb: 512,
  memoryMaximumMb: 16384,
  memoryStepMb: 64,
} satisfies SettingsSnapshot;

import type { SettingsSnapshot } from "@/lib/generated/SettingsSnapshot";
