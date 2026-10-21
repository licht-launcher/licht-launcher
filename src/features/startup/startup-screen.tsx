import { useMutation } from "@tanstack/react-query";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Spinner } from "@/components/ui/spinner";
import { useStartupProgress } from "@/features/startup/hooks/use-startup-progress";
import type { StartupPhase } from "@/lib/generated/StartupPhase";

const labels: Record<StartupPhase, string> = {
  preparing: "Preparing launcher…",
  checkingUpdates: "Checking for updates…",
  downloadingUpdate: "Downloading update…",
  verifyingUpdate: "Verifying update signature…",
  installingUpdate: "Installing signed update…",
  migratingDatabase: "Migrating launcher database…",
  migratingData: "Migrating shared game files…",
  migratingInstances: "Migrating instances…",
  migratingSettings: "Migrating settings…",
  restoringCache: "Preparing icons and installation files…",
  startingServices: "Starting launcher services…",
  ready: "Ready",
  failed: "Launcher preparation failed",
};

function formatBytes(bytes: number) {
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function StartupScreen() {
  const startup = useStartupProgress();
  const close = useMutation({ mutationFn: () => getCurrentWindow().close() });
  const snapshot = startup.data;
  const phase = snapshot?.phase ?? "preparing";
  const error =
    snapshot?.error ??
    (startup.error instanceof Error
      ? startup.error.message
      : startup.error
        ? String(startup.error)
        : null);
  const value =
    phase === "downloadingUpdate" &&
    snapshot?.totalBytes &&
    snapshot.totalBytes > 0
      ? Math.min(100, (snapshot.processedBytes / snapshot.totalBytes) * 100)
      : undefined;
  return (
    <ScrollArea className="h-full">
      <main
        className="flex min-h-full flex-col justify-center gap-3 bg-background p-6 text-foreground"
        aria-label="Launcher startup"
      >
        <div className="flex items-center gap-3">
          {!error && phase !== "ready" ? (
            <Spinner aria-hidden className="size-5 text-primary" />
          ) : null}
          <h1 className="font-brand text-lg font-medium">Licht Launcher</h1>
        </div>
        {snapshot?.preview ? (
          <p className="text-xs font-medium text-warning">
            Development preview · updates are simulated and migration uses
            temporary fixtures.
          </p>
        ) : null}
        <p role="status" aria-live="polite" className="text-sm">
          {labels[phase]}
        </p>
        {!error && phase !== "ready" ? (
          <Progress value={value} aria-label="Launcher preparation progress" />
        ) : null}
        {phase === "downloadingUpdate" && snapshot ? (
          <p className="text-xs text-muted-foreground">
            {formatBytes(snapshot.processedBytes)}
            {snapshot.totalBytes
              ? ` / ${formatBytes(snapshot.totalBytes)}`
              : ""}
            {value !== undefined ? ` · ${Math.floor(value)}%` : ""}
          </p>
        ) : snapshot && snapshot.processedFiles > 0 ? (
          <p className="text-xs text-muted-foreground">
            {snapshot.processedFiles.toLocaleString()} files verified ·{" "}
            {formatBytes(snapshot.processedBytes)}
          </p>
        ) : null}
        {snapshot?.version ? (
          <p className="text-xs text-muted-foreground">
            Update: {snapshot.version}
          </p>
        ) : null}
        {snapshot?.warning ? (
          <p className="text-sm text-warning">{snapshot.warning}</p>
        ) : null}
        {startup.eventError ? (
          <p className="text-xs text-muted-foreground">
            Live progress is unavailable. Polling startup status.
          </p>
        ) : null}
        {error ? (
          <p role="alert" className="text-sm text-destructive">
            {error}
          </p>
        ) : null}
        {close.error ? (
          <p role="alert" className="text-sm text-destructive">
            Could not close the startup window.
          </p>
        ) : null}
        {error || snapshot?.preview ? (
          <Button
            variant="outline"
            disabled={close.isPending}
            onClick={() => close.mutate()}
          >
            Close
          </Button>
        ) : null}
      </main>
    </ScrollArea>
  );
}
