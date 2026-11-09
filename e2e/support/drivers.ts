import { execFileSync } from "node:child_process";

/**
 * `@wdio/native-core` 1.3.1 starts tauri-driver through a shell on Windows and stops
 * only that shell, which leaves tauri-driver and its msedgedriver child running after
 * every run. User `onComplete` hooks run before the service's, while the launcher →
 * shell → tauri-driver chain still exists, so stop exactly the drivers this run started.
 */
export function stopDriversStartedByThisRun(): void {
  if (process.platform !== "win32") {
    return;
  }
  const query = [
    `Get-CimInstance Win32_Process -Filter "ParentProcessId=${process.pid}"`,
    `ForEach-Object { Get-CimInstance Win32_Process -Filter "ParentProcessId=$($_.ProcessId) AND Name='tauri-driver.exe'" }`,
    "ForEach-Object { $_.ProcessId }",
  ].join(" | ");
  const output = execFileSync(
    "powershell.exe",
    ["-NoProfile", "-NonInteractive", "-Command", query],
    { encoding: "utf8" },
  );
  for (const pid of output.split(/\s+/)) {
    if (/^\d+$/.test(pid)) {
      // /T also stops the msedgedriver child.
      execFileSync("taskkill", ["/PID", pid, "/T", "/F"], { stdio: "ignore" });
    }
  }
}
