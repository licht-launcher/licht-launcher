import { useMutation, useQuery } from "@tanstack/react-query";
import { BoxIcon, LayersIcon, PencilIcon, WrenchIcon } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { getInstancePlatform, repairInstance } from "@/api/commands";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { EditInstanceVersionDialog } from "@/features/instance-settings/components/edit-instance-version-dialog";
import { LinkedModpackSettings } from "@/features/instance-settings/components/linked-modpack-settings";
import { formatPlatform } from "@/features/instance-settings/model/platform";
import type { ModpackOrigin } from "@/features/instances/lib/modpack-content";
import { versionsQueryOptions } from "@/features/instances/lib/versions-query";
import { friendlyError, messageOf } from "@/lib/errors";
import type { LinkedModpack } from "@/lib/generated/LinkedModpack";
import type { VersionSummary } from "@/lib/generated/VersionSummary";

import { instanceBaseSectionStyles } from "./instance-base-section.styles";

function versionsForPicker(
  versions: VersionSummary[] | undefined,
  versionId: string | null,
): VersionSummary[] {
  if (versions !== undefined && versions.length > 0) {
    return versions;
  }
  if (versionId === null) return [];
  return [
    {
      id: versionId,
      versionType: "old_alpha",
      installed: true,
    },
  ];
}

export function InstanceBaseSection({
  folder,
  versionId,
  busy,
  versionBusy,
  preparing = false,
  modpack = null,
  onChangeVersion,
}: {
  folder: string;
  versionId: string | null;
  preparing?: boolean;
  /**
   * The pack a linked instance came from. It sets the game version, so that cannot
   * change while the instance is linked.
   */
  modpack?: { origin: ModpackOrigin; linked: LinkedModpack | null } | null;
  busy: boolean;
  versionBusy: boolean;
  onChangeVersion: (versionId: string, installed: boolean) => void;
}) {
  const [editOpen, setEditOpen] = useState(false);
  const versions = useQuery({
    ...versionsQueryOptions,
    enabled: editOpen,
  });
  // A local profile read, so it resolves at once; it changes only with the version.
  const platform = useQuery({
    queryKey: ["instance-platform", versionId],
    queryFn: () => getInstancePlatform(versionId ?? ""),
    enabled: versionId !== null,
    staleTime: Number.POSITIVE_INFINITY,
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

  const pickerVersions = versionsForPicker(versions.data, versionId);
  const versionsLoading = editOpen && versions.isPending;
  const versionsFailed = editOpen && versions.isError;

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
            {platform.data !== undefined ? formatPlatform(platform.data) : "—"}
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
          busy={busy || versionBusy}
        />
      ) : null}

      <div className={instanceBaseSectionStyles.group}>
        <div className={instanceBaseSectionStyles.groupText}>
          <h3 className={instanceBaseSectionStyles.label}>Edit base</h3>
          <p className={instanceBaseSectionStyles.hint}>
            {modpack !== null
              ? "The linked modpack sets this instance's game version."
              : "Open the editor to pick another Vanilla release for this instance."}
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
            modpack !== null ||
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

      {versionId !== null && (
        <EditInstanceVersionDialog
          open={editOpen}
          versionId={versionId}
          versions={pickerVersions}
          versionsPending={versionsLoading}
          versionsError={versionsFailed}
          onRetryVersions={() => {
            void versions.refetch();
          }}
          busy={busy || versionBusy}
          onOpenChange={setEditOpen}
          onSave={onChangeVersion}
        />
      )}
    </div>
  );
}
