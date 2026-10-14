import {
  CircleCheckIcon,
  ClockIcon,
  DownloadIcon,
  OctagonXIcon,
  PackageIcon,
  RefreshCwIcon,
} from "lucide-react";
import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import type { Draft } from "@/features/settings/model/draft";
import {
  getUpdateCheckStatus,
  INSTALL_UPDATE_EVENT,
  LATEST_VERSION_EVENT,
  LATEST_VERSION_KEY,
  MANUAL_UPDATE_CHECK_EVENT,
  UPDATE_CHECK_STATUS_EVENT,
  type UpdateCheckStatus,
} from "@/features/updater/components/update-check";
import { appReleaseLabel } from "@/lib/app-release";

export function UpdaterSection({
  draft,
  onChange,
}: {
  draft: Draft;
  onChange: (patch: Partial<Draft>) => void;
}) {
  const [latestVersion, setLatestVersion] = useState<string | null>(() =>
    localStorage.getItem(LATEST_VERSION_KEY),
  );
  const [status, setStatus] = useState(getUpdateCheckStatus);
  const checking = status === "checking";
  const statusLabels: Record<UpdateCheckStatus, string> = {
    "not-checked": "Not checked yet",
    checking: "Checking for updates…",
    "up-to-date": "Up to date",
    available: "Update available",
    failed: "Update check failed. Try again.",
  };
  const statusIcons = {
    "not-checked": ClockIcon,
    checking: RefreshCwIcon,
    "up-to-date": CircleCheckIcon,
    available: DownloadIcon,
    failed: OctagonXIcon,
  };
  const StatusIcon = statusIcons[status];

  useEffect(() => {
    const updateLatestVersion = () => {
      setLatestVersion(localStorage.getItem(LATEST_VERSION_KEY));
    };
    window.addEventListener(LATEST_VERSION_EVENT, updateLatestVersion);
    const updateChecking = () => {
      setStatus(getUpdateCheckStatus());
    };
    window.addEventListener(UPDATE_CHECK_STATUS_EVENT, updateChecking);
    updateChecking();
    return () => {
      window.removeEventListener(LATEST_VERSION_EVENT, updateLatestVersion);
      window.removeEventListener(UPDATE_CHECK_STATUS_EVENT, updateChecking);
    };
  }, []);

  return (
    <section aria-label="Launcher updates" className="flex flex-col gap-4">
      <div className="overflow-hidden rounded-lg border border-border/60 bg-muted/30">
        <div className="grid grid-cols-[minmax(0,1fr)_auto] border-b border-border/60 px-3 py-2.5 text-sm">
          <span className="flex items-center gap-2 font-medium">
            <PackageIcon aria-hidden className="size-4 text-muted-foreground" />
            Current version
          </span>
          <span className="font-brand text-muted-foreground">
            {appReleaseLabel}
          </span>
        </div>
        <div className="grid grid-cols-[minmax(0,1fr)_auto] px-3 py-2.5 text-sm">
          <span className="flex items-center gap-2 font-medium">
            <DownloadIcon
              aria-hidden
              className="size-4 text-muted-foreground"
            />
            Latest version
          </span>
          <span className="font-brand text-muted-foreground">
            {checking
              ? "Checking…"
              : status === "up-to-date"
                ? "No newer version available"
                : (latestVersion ??
                  (status === "failed" ? "Unavailable" : "Not checked yet"))}
          </span>
        </div>
      </div>
      <div className="flex flex-wrap items-center gap-3">
        {status === "available" ? (
          <Button
            type="button"
            className="cursor-pointer"
            onClick={() => {
              window.dispatchEvent(new Event(INSTALL_UPDATE_EVENT));
            }}
          >
            <DownloadIcon data-icon="inline-start" aria-hidden />
            Update
          </Button>
        ) : null}
        <Button
          type="button"
          variant="outline"
          disabled={checking}
          onClick={() => {
            window.dispatchEvent(new Event(MANUAL_UPDATE_CHECK_EVENT));
          }}
        >
          <RefreshCwIcon
            data-icon="inline-start"
            aria-hidden
            className={checking ? "animate-spin" : undefined}
          />
          {checking ? "Checking…" : "Check for updates"}
        </Button>
        <div className="flex flex-wrap items-center gap-3">
          <p
            role="status"
            className="flex items-center gap-2 text-sm text-muted-foreground"
          >
            <StatusIcon
              aria-hidden
              className={checking ? "size-4 animate-spin" : "size-4"}
            />
            {statusLabels[status]}
          </p>
        </div>
      </div>
      <div className="flex items-center justify-between gap-3">
        <div className="flex min-w-0 flex-col gap-1">
          <label htmlFor="check-for-updates" className="text-sm font-medium">
            Check for updates automatically
          </label>
          <p
            id="check-for-updates-hint"
            className="text-sm text-muted-foreground"
          >
            Check for a new launcher version when the launcher opens.
          </p>
        </div>
        <Switch
          id="check-for-updates"
          className="shrink-0"
          aria-describedby="check-for-updates-hint"
          checked={draft.checkForUpdates}
          onCheckedChange={(checkForUpdates) => onChange({ checkForUpdates })}
        />
      </div>
      <div className="flex items-center justify-between gap-3">
        <div className="flex min-w-0 flex-col gap-1">
          <label
            htmlFor="install-updates-on-restart"
            className="text-sm font-medium"
          >
            Install deferred updates on restart
          </label>
          <p
            id="install-updates-on-restart-hint"
            className="text-sm text-muted-foreground"
          >
            Install an update automatically the next time the launcher opens.
          </p>
        </div>
        <Switch
          id="install-updates-on-restart"
          className="shrink-0"
          aria-describedby="install-updates-on-restart-hint"
          checked={draft.installUpdatesOnRestart}
          onCheckedChange={(installUpdatesOnRestart) =>
            onChange({ installUpdatesOnRestart })
          }
        />
      </div>
    </section>
  );
}
