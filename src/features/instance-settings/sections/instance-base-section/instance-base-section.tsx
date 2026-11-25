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

      {/* Right under the card it changes: the pack's version, or the platform and game version. */}
      {modpack !== null ? (
        <LinkedModpackSettings
          folder={folder}
          origin={modpack.origin}
          linked={modpack.linked}
          preparing={preparing}
          busy={busy || versionBusy || instanceActive}
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
