import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  BoxIcon,
  LayersIcon,
  PackageIcon,
  PencilIcon,
  WrenchIcon,
} from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import {
  changeInstanceModpackVersion,
  getInstancePlatform,
  type InstanceBaseInput,
  repairInstance,
} from "@/api/commands";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { ChangeModpackVersionDialog } from "@/features/instance-settings/components/change-modpack-version-dialog";
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
  modpack?: { origin: ModpackOrigin; linked: LinkedModpack | null } | null;
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

  return (
    <div className={instanceBaseSectionStyles.root}>
      <div className={instanceBaseSectionStyles.summary}>
        <div className={instanceBaseSectionStyles.summaryItem}>
          <div className={instanceBaseSectionStyles.summaryLabel}>
            <LayersIcon
              aria-hidden
              className={instanceBaseSectionStyles.summaryIcon}
            />
            <span className={instanceBaseSectionStyles.summaryName}>
              Platform
            </span>
          </div>
          <div className={instanceBaseSectionStyles.summaryValue}>
            {platform.data !== undefined
              ? formatPlatform(platform.data)
              : platform.isError
                ? "Could not read the platform"
                : "—"}
          </div>
        </div>
        <div className={instanceBaseSectionStyles.summaryItem}>
          <div className={instanceBaseSectionStyles.summaryLabel}>
            <BoxIcon
              aria-hidden
              className={instanceBaseSectionStyles.summaryIcon}
            />
            <span className={instanceBaseSectionStyles.summaryName}>
              Game version
            </span>
          </div>
          <div className={instanceBaseSectionStyles.summaryValue}>
            {versionBusy ? (
              <span className={instanceBaseSectionStyles.busy}>
                <Spinner
                  className={instanceBaseSectionStyles.busySpinner}
                  aria-hidden
                />
                Updating…
              </span>
            ) : (
              <span className={instanceBaseSectionStyles.valueText}>
                {platform.data?.gameVersion ?? versionId ?? "Preparing"}
              </span>
            )}
          </div>
        </div>
      </div>

      {modpack !== null ? (
        <LinkedModpackSettings
          folder={folder}
          origin={modpack.origin}
          linked={modpack.linked}
          preparing={preparing}
          busy={busy || versionBusy || instanceActive}
        />
      ) : null}

      {modpack !== null ? (
        <ModpackVersionAction
          folder={folder}
          origin={modpack.origin}
          disabled={busy || versionBusy || preparing || instanceActive}
        />
      ) : (
        <div className={instanceBaseSectionStyles.group}>
          <div className={instanceBaseSectionStyles.groupText}>
            <h3 className={instanceBaseSectionStyles.label}>Edit base</h3>
            <p className={instanceBaseSectionStyles.hint}>
              Change the platform, game version or loader this instance launches
              with.
            </p>
          </div>
          <Button
            type="button"
            variant="warning"
            className={instanceBaseSectionStyles.action}
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
      )}

      <div className={instanceBaseSectionStyles.group}>
        <div className={instanceBaseSectionStyles.groupText}>
          <h3 className={instanceBaseSectionStyles.label}>Repair</h3>
          <p className={instanceBaseSectionStyles.hint}>
            Re-download missing game files and modpack content for this
            instance.
          </p>
        </div>
        <Button
          type="button"
          variant="outline"
          className={instanceBaseSectionStyles.action}
          disabled={busy || versionBusy || repair.isPending}
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

/**
 * In place of Edit base for a linked instance: the pack sets the base, so the
 * choice is which version of the pack to install.
 */
function ModpackVersionAction({
  folder,
  origin,
  disabled,
}: {
  folder: string;
  origin: ModpackOrigin;
  disabled: boolean;
}) {
  const [open, setOpen] = useState(false);
  const queryClient = useQueryClient();
  const change = useMutation({
    mutationFn: (versionId: string) =>
      changeInstanceModpackVersion(folder, versionId),
    onSuccess: (job) => {
      if (job.status === "completed") toast.success("Modpack version changed.");
      else if (job.status === "failed")
        toast.error(job.error ?? "Changing the modpack version failed.");
    },
    onError: (error: unknown) => toast.error(friendlyError(messageOf(error))),
    // Also after a failure: the change waits to retry, or a cancel put the old version back.
    onSettled: async () => {
      await queryClient.invalidateQueries({ queryKey: ["instances"] });
      await queryClient.invalidateQueries({
        queryKey: ["instance-content", folder],
      });
    },
  });
  const fromFile = origin.type === "modpackFile";
  return (
    <div className={instanceBaseSectionStyles.group}>
      <div className={instanceBaseSectionStyles.groupText}>
        <h3 className={instanceBaseSectionStyles.label}>Modpack version</h3>
        <p className={instanceBaseSectionStyles.hint}>
          {fromFile
            ? "A modpack imported from a file has no other versions to install."
            : "The linked modpack sets this instance's platform and game version. Install another version of it; content you added stays."}
        </p>
      </div>
      <Button
        type="button"
        variant="warning"
        className={instanceBaseSectionStyles.action}
        disabled={disabled || fromFile || change.isPending}
        onClick={() => setOpen(true)}
      >
        {change.isPending ? (
          <Spinner
            className={instanceBaseSectionStyles.busySpinner}
            data-icon="inline-start"
            aria-hidden
          />
        ) : (
          <PackageIcon data-icon="inline-start" aria-hidden />
        )}
        Change version
      </Button>
      {origin.type === "modpack" ? (
        <ChangeModpackVersionDialog
          open={open}
          origin={origin}
          busy={change.isPending}
          onOpenChange={setOpen}
          onSave={(versionId) => change.mutate(versionId)}
        />
      ) : null}
    </div>
  );
}
