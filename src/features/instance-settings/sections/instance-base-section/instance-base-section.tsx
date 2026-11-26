import { useMutation, useQuery } from "@tanstack/react-query";
import { BoxIcon, LayersIcon, PencilIcon, WrenchIcon } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import {
  getInstancePlatform,
  type InstanceBaseInput,
  repairInstance,
} from "@/api/commands";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Spinner } from "@/components/ui/spinner";
import { EditInstanceBaseDialog } from "@/features/instance-settings/components/edit-instance-base-dialog";
import { LinkedModpackSettings } from "@/features/instance-settings/components/linked-modpack-settings";
import {
  formatPlatform,
  platformStaleTime,
} from "@/features/instance-settings/model/platform";
import type { ModpackOrigin } from "@/features/instances/lib/modpack-content";
import { friendlyError, messageOf } from "@/lib/errors";
import type { LinkedModpack } from "@/lib/generated/LinkedModpack";

import { instanceBaseSectionStyles } from "./instance-base-section.styles";

export function InstanceBaseSection({
  folder,
  versionId,
  busy,
  versionBusy,
  preparing = false,
  modpack = null,
  instanceActive = false,
  onChangeBase,
}: {
  folder: string;
  versionId: string | null;
  preparing?: boolean;
  /**
   * The pack a linked instance came from. It sets the game version, so that cannot
   * change while the instance is linked.
   */
  modpack?: {
    origin: ModpackOrigin;
    linked: LinkedModpack | null;
    /** The pack icon cached for the instance. */
    iconPath: string | null;
  } | null;
  /** The game or a download is running, so the modpack must not change under it. */
  instanceActive?: boolean;
  busy: boolean;
  versionBusy: boolean;
  onChangeBase: (base: InstanceBaseInput) => void;
}) {
  const [editOpen, setEditOpen] = useState(false);
  // A local profile read, so it resolves at once; it changes only with the version.
  const platform = useQuery({
    queryKey: ["instance-platform", versionId],
    queryFn: () => getInstancePlatform(versionId ?? ""),
    enabled: versionId !== null,
    staleTime: (query) => platformStaleTime(query.state.data),
  });
  const repair = useMutation({
    mutationFn: () => repairInstance(folder),
    onSuccess: (job) => {
      if (job.status === "completed")
        toast.success("Instance files were repaired.");
      else if (job.status === "failed")
        toast.error(job.error ?? "Repair failed.");
    },
    onError: (error: unknown) => {
      toast.error(friendlyError(messageOf(error)));
    },
  });

  const editBase = (
    <div className={instanceBaseSectionStyles.row}>
      <div className={instanceBaseSectionStyles.rowText}>
        <p className={instanceBaseSectionStyles.rowLabel}>Edit base</p>
        <p className={instanceBaseSectionStyles.rowDescription}>
          Change the platform, game version or loader this instance launches
          with.
        </p>
      </div>
      <Button
        type="button"
        variant="warning"
        className={instanceBaseSectionStyles.control}
        disabled={
          busy ||
          versionBusy ||
          preparing ||
          instanceActive ||
          versionId === null
        }
        onClick={() => {
          setEditOpen(true);
        }}
      >
        <PencilIcon data-icon="inline-start" aria-hidden />
        Edit base
      </Button>
    </div>
  );

  return (
    <div className={instanceBaseSectionStyles.root}>
      <Card title="Base">
        <div className={instanceBaseSectionStyles.row}>
          <div className={instanceBaseSectionStyles.rowName}>
            <LayersIcon
              aria-hidden
              className={instanceBaseSectionStyles.icon}
            />
            <span className={instanceBaseSectionStyles.rowLabel}>Platform</span>
          </div>
          <span className={instanceBaseSectionStyles.value}>
            {platform.data !== undefined
              ? formatPlatform(platform.data)
              : platform.isError
                ? "Could not read the platform"
                : "—"}
          </span>
        </div>
        <div className={instanceBaseSectionStyles.row}>
          <div className={instanceBaseSectionStyles.rowName}>
            <BoxIcon aria-hidden className={instanceBaseSectionStyles.icon} />
            <span className={instanceBaseSectionStyles.rowLabel}>
              Game version
            </span>
          </div>
          {versionBusy ? (
            <span className={instanceBaseSectionStyles.busy}>
              <Spinner
                className={instanceBaseSectionStyles.busySpinner}
                aria-hidden
              />
              Updating…
            </span>
          ) : (
            <span className={instanceBaseSectionStyles.value}>
              {platform.data?.gameVersion ?? versionId ?? "Preparing"}
            </span>
          )}
        </div>
        {/* The linked pack sets the base, so only an unlinked instance edits it here. */}
        {modpack === null ? editBase : null}
      </Card>

      {modpack !== null ? (
        <LinkedModpackSettings
          folder={folder}
          origin={modpack.origin}
          linked={modpack.linked}
          iconPath={modpack.iconPath}
          preparing={preparing}
          busy={busy || versionBusy || instanceActive}
        />
      ) : null}

      <Card title="Maintenance">
        <div className={instanceBaseSectionStyles.row}>
          <div className={instanceBaseSectionStyles.rowText}>
            <p className={instanceBaseSectionStyles.rowLabel}>Repair</p>
            <p className={instanceBaseSectionStyles.rowDescription}>
              Re-download missing game files and modpack content for this
              instance.
            </p>
          </div>
          <Button
            type="button"
            variant="outline"
            className={instanceBaseSectionStyles.control}
            disabled={busy || versionBusy || preparing || repair.isPending}
            onClick={() => {
              repair.mutate();
            }}
          >
            {repair.isPending ? (
              <Spinner
                className={instanceBaseSectionStyles.busySpinner}
                data-icon="inline-start"
                aria-hidden
              />
            ) : (
              <WrenchIcon data-icon="inline-start" aria-hidden />
            )}
            Repair
          </Button>
        </div>
      </Card>

      <EditInstanceBaseDialog
        open={editOpen}
        current={platform.data ?? null}
        busy={busy || versionBusy}
        onOpenChange={setEditOpen}
        onSave={onChangeBase}
      />
    </div>
  );
}
