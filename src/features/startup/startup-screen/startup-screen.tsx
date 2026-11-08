import { useMutation } from "@tanstack/react-query";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useRef } from "react";
import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useStartupProgress } from "@/features/startup/hooks/use-startup-progress";
import { messageOf } from "@/lib/errors";
import type { StartupPhase } from "@/lib/generated/StartupPhase";
import logo from "../../../../src-tauri/icons/128x128@2x.png?no-inline";

import { startupScreenStyles } from "./startup-screen.styles";

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
  const previewWindow = getCurrentWindow().label.startsWith("startup-preview-");
  const { mutate: closePreview, ...closing } = useMutation({
    mutationFn: () => getCurrentWindow().close(),
  });
  useEffect(() => {
    if (!previewWindow) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (
        event.key === "Escape" &&
        !event.defaultPrevented &&
        !closing.isPending
      ) {
        event.preventDefault();
        closePreview();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [previewWindow, closing.isPending, closePreview]);
  const logoRef = useRef<HTMLImageElement>(null);
  const revealing = useRef(false);
  const { mutate: revealWindow, ...reveal } = useMutation({
    mutationFn: async () => {
      try {
        await document.fonts?.ready;
        await logoRef.current?.decode();
      } finally {
        const window = getCurrentWindow();
        await window.center();
        await window.show();
      }
    },
  });
  useEffect(() => {
    if (revealing.current) return;
    revealing.current = true;
    revealWindow();
  }, [revealWindow]);
  const startup = useStartupProgress(false, reveal.isSuccess || reveal.isError);
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
    <ScrollArea className={startupScreenStyles.scrollArea}>
      <main
        className={startupScreenStyles.root}
        aria-label="Launcher startup"
        aria-busy={closing.isPending}
      >
        <h1 data-tauri-drag-region className={startupScreenStyles.logoWrapper}>
          <img
            ref={logoRef}
            src={logo}
            alt="Licht Launcher"
            draggable={false}
            className={startupScreenStyles.logo}
          />
        </h1>
        <div className={startupScreenStyles.progress}>
          {!error && phase !== "ready" ? (
            <Progress
              value={value}
              aria-label="Launcher preparation progress"
            />
          ) : null}
          {phase === "downloadingUpdate" && snapshot ? (
            <p className={startupScreenStyles.detail}>
              {formatBytes(snapshot.processedBytes)}
              {snapshot.totalBytes
                ? ` / ${formatBytes(snapshot.totalBytes)}`
                : ""}
              {value !== undefined ? ` · ${Math.floor(value)}%` : ""}
            </p>
          ) : snapshot && snapshot.processedFiles > 0 ? (
            <p className={startupScreenStyles.detail}>
              {snapshot.processedFiles.toLocaleString()} files verified ·{" "}
              {formatBytes(snapshot.processedBytes)}
            </p>
          ) : null}
          <p
            role="status"
            aria-live="polite"
            className={startupScreenStyles.message}
          >
            {labels[phase]}
          </p>
          {snapshot?.version ? (
            <p className={startupScreenStyles.detail}>
              Update: {snapshot.version}
            </p>
          ) : null}
          {snapshot?.warning ? (
            <p className={startupScreenStyles.warning}>{snapshot.warning}</p>
          ) : null}
          {snapshot?.preview ? (
            <p className={startupScreenStyles.warningLabel}>
              Development preview · updates are simulated and migration uses
              temporary fixtures.
            </p>
          ) : null}
          {startup.eventError ? (
            <p className={startupScreenStyles.detail}>
              Live progress is unavailable. Polling startup status.
            </p>
          ) : null}
          {error ? (
            <p role="alert" className={startupScreenStyles.error}>
              {error}
            </p>
          ) : null}
          {reveal.error ? (
            <p role="alert" className={startupScreenStyles.error}>
              Could not prepare the startup window: {reveal.error.message}
            </p>
          ) : null}
          {closing.error ? (
            <p role="alert" className={startupScreenStyles.error}>
              Could not close preview: {messageOf(closing.error)}
            </p>
          ) : null}
        </div>
      </main>
    </ScrollArea>
  );
}
