import { useMutation } from "@tanstack/react-query";
import { FolderOpenIcon } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { StoragePanel } from "@/features/downloads/components/storage-panel";
import type { Draft } from "@/features/settings/model/draft";
import { browseApplicationDirectory } from "@/lib/commands";
import {
  downloadLimitKbs,
  formatDownloadLimit,
  parseDownloadLimit,
} from "@/lib/download-speed";
import { friendlyError, messageOf } from "@/lib/errors";

import { downloadsSectionStyles } from "./downloads-section.styles";

export function DownloadsSection({
  draft,
  onChange,
}: {
  draft: Draft;
  onChange: (patch: Partial<Draft>) => void;
}) {
  const speedLimit = parseDownloadLimit(draft.downloadSpeedLimitKbs);
  const [speedLimitEnabled, setSpeedLimitEnabled] = useState(
    () => speedLimit !== null && speedLimit !== "invalid",
  );
  const browseDirectory = useMutation({
    mutationFn: browseApplicationDirectory,
    onSuccess: (path) => {
      if (path) onChange({ dataDirectory: path });
    },
    onError: (error: unknown) => toast.error(friendlyError(messageOf(error))),
  });

  return (
    <div className={downloadsSectionStyles.root}>
      <Card title="Storage">
        <StoragePanel active embedded />

        <div className={downloadsSectionStyles.group}>
          <div className={downloadsSectionStyles.groupText}>
            <label
              htmlFor="application-directory"
              className={downloadsSectionStyles.rowLabel}
            >
              Application directory
            </label>
            <p className={downloadsSectionStyles.rowDescription}>
              Where instances, game files, and managed Java runtimes are stored.
              Existing data will be moved after restart. Launcher settings and
              the database stay in your profile directory.
            </p>
          </div>
          <div className={downloadsSectionStyles.folderRow}>
            <Input
              id="application-directory"
              aria-label="Application directory"
              className={downloadsSectionStyles.folderInput}
              value={draft.dataDirectory}
              onChange={(event) =>
                onChange({ dataDirectory: event.target.value })
              }
            />
            <Button
              type="button"
              variant="outline"
              size="default"
              aria-label="Browse application directory"
              disabled={browseDirectory.isPending}
              onClick={() => browseDirectory.mutate(draft.dataDirectory)}
            >
              <FolderOpenIcon data-icon="inline-start" aria-hidden />
              Browse
            </Button>
          </div>
        </div>
      </Card>
      <Card title="Network">
        <div className={downloadsSectionStyles.row}>
          <div className={downloadsSectionStyles.rowText}>
            <label
              htmlFor="enable-download-speed-limit"
              className={downloadsSectionStyles.rowLabel}
            >
              Limit download speed
            </label>
            <p
              id="download-speed-limit-hint"
              className={downloadsSectionStyles.rowDescription}
            >
              Set a shared speed limit for game downloads.
            </p>
          </div>
          <Switch
            id="enable-download-speed-limit"
            className={downloadsSectionStyles.control}
            aria-describedby="download-speed-limit-hint"
            checked={speedLimitEnabled}
            onCheckedChange={(enabled) => {
              setSpeedLimitEnabled(enabled);
              if (!enabled) onChange({ downloadSpeedLimitKbs: "" });
            }}
          />
        </div>
        {speedLimitEnabled ? (
          <div className={downloadsSectionStyles.row}>
            <div className={downloadsSectionStyles.rowText}>
              <label
                htmlFor="download-speed-limit"
                className={downloadsSectionStyles.rowLabel}
              >
                Speed limit (kB/s)
              </label>
              <p
                id="download-speed-value-hint"
                className={downloadsSectionStyles.rowDescription}
              >
                Enter the maximum download speed in kilobytes per second.
              </p>
            </div>
            <Input
              id="download-speed-limit"
              className={downloadsSectionStyles.numberInput}
              aria-describedby="download-speed-value-hint"
              type="number"
              min={0.001}
              max={downloadLimitKbs(Number.MAX_SAFE_INTEGER)}
              step={0.001}
              placeholder="Unlimited"
              value={draft.downloadSpeedLimitKbs}
              onChange={(event) => {
                const value = event.target.value;
                onChange({
                  downloadSpeedLimitKbs: Number(value) <= 0 ? "" : value,
                });
              }}
            />
          </div>
        ) : null}
        {speedLimitEnabled &&
        speedLimit !== null &&
        speedLimit !== "invalid" ? (
          <p className={downloadsSectionStyles.note}>
            Downloads limited to {formatDownloadLimit(speedLimit)}
          </p>
        ) : null}
      </Card>
    </div>
  );
}
