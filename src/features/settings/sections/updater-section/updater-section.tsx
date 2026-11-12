import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { Switch } from "@/components/ui/switch";
import type { Draft } from "@/features/settings/model/draft";
import {
  DOWNLOAD_UPDATE_EVENT,
  INSTALL_UPDATE_EVENT,
  MANUAL_UPDATE_CHECK_EVENT,
} from "@/features/updater/components/update-check";
import { useAppUpdateState } from "@/features/updater/hooks/use-app-update-state";
import { appReleaseLabel } from "@/lib/app-release";
import { messageOf } from "@/lib/errors";
import type { AppUpdatePhase } from "@/lib/generated/AppUpdatePhase";

const labels: Record<AppUpdatePhase, string> = {
  idle: "Not checked yet",
  checking: "Checking for updates…",
  upToDate: "Up to date",
  available: "An update is available to download.",
  downloading: "Downloading update…",
  verifying: "Verifying update…",
  cached:
    "A downloaded update is cached. Check online to prepare installation.",
  ready: "Ready to update",
  installing: "Installing update…",
  failed: "Update failed. Try again.",
  externallyManaged: "Updates for this installation are managed by Flatpak.",
};

export function UpdaterSection({
  draft,
  onChange,
}: {
  draft: Draft;
  onChange: (patch: Partial<Draft>) => void;
}) {
  const query = useAppUpdateState();
  const state = query.data;
  const busy =
    query.isPending ||
    ["checking", "downloading", "verifying", "installing"].includes(
      state?.phase ?? "",
    );
  const externallyManaged = state?.phase === "externallyManaged";
  const error = state?.error ?? (query.error ? messageOf(query.error) : null);
  const action =
    state?.phase === "ready"
      ? { label: "Restart to update", event: INSTALL_UPDATE_EVENT }
      : state?.phase === "available"
        ? { label: "Download update", event: DOWNLOAD_UPDATE_EVENT }
        : {
            label:
              state?.phase === "cached"
                ? "Prepare update"
                : "Check for updates",
            event: MANUAL_UPDATE_CHECK_EVENT,
          };
  const percentage =
    state?.totalBytes && state.totalBytes > 0
      ? Math.min(100, (state.downloadedBytes / state.totalBytes) * 100)
      : undefined;
  return (
    <Card title="Launcher updates">
      <div className={updaterSectionStyles.row}>
        <div className={updaterSectionStyles.rowText}>
          <label
            htmlFor="automatic-updates"
            className={updaterSectionStyles.rowLabel}
          >
            Automatic updates
          </label>
          <p
            id="automatic-updates-hint"
            className={updaterSectionStyles.rowDescription}
          >
            Check and download updates in the background.
          </p>
        </div>
        <Switch
          id="automatic-updates"
          className={updaterSectionStyles.control}
          aria-describedby="automatic-updates-hint"
          disabled={externallyManaged}
          checked={draft.automaticUpdates}
          onCheckedChange={(automaticUpdates) => onChange({ automaticUpdates })}
        />
      </div>
      <div className={updaterSectionStyles.statusRow}>
        <div className={updaterSectionStyles.statusText}>
          <p>Version {appReleaseLabel}</p>
          <p
            role="status"
            className={
              state?.phase === "ready"
                ? updaterSectionStyles.statusReady
                : updaterSectionStyles.statusIdle
            }
          >
            {state?.phase === "ready" && state.version
              ? `Version ${state.version} is ready. Restart to apply.`
              : state
                ? labels[state.phase]
                : "Loading update status…"}
          </p>
          {state?.phase === "cached" && state.version ? (
            <p className={updaterSectionStyles.rowDescription}>
              Cached version: {state.version}
            </p>
          ) : null}
        </div>
        <Button
          type="button"
          variant={state?.phase === "ready" ? "default" : "outline"}
          size="default"
          disabled={busy || externallyManaged}
          onClick={() => window.dispatchEvent(new Event(action.event))}
        >
          {action.label === "Restart to update" && (
            <DownloadCloud data-icon="inline-start" aria-hidden />
          )}
          {action.label === "Download update" && (
            <DownloadCloud data-icon="inline-start" aria-hidden />
          )}
          {action.label === "Prepare update" && (
            <DownloadCloud data-icon="inline-start" aria-hidden />
          )}

          {action.label === "Check for updates" &&
          (!busy || state?.phase === "checking") ? (
            <RefreshCwIcon data-icon="inline-start" aria-hidden />
          ) : null}
          {busy
            ? state?.phase === "downloading"
              ? "Downloading…"
              : state?.phase === "installing"
                ? "Installing…"
                : state?.phase === "verifying"
                  ? "Verifying…"
                  : "Checking…"
            : action.label}
        </Button>
      </div>
      {state?.phase === "downloading" || state?.phase === "verifying" ? (
        <Progress value={percentage} aria-label="Update download progress" />
      ) : null}
      {error ? (
        <p role="alert" className={updaterSectionStyles.error}>
          {error}
        </p>
      ) : null}
      {query.eventError ? (
        <p className={updaterSectionStyles.note}>
          Live update progress is unavailable. Polling update status.
        </p>
      ) : null}
    </Card>
  );
}

import { DownloadCloud, RefreshCwIcon } from "lucide-react";

import { updaterSectionStyles } from "./updater-section.styles";
