import { useMutation, useQuery } from "@tanstack/react-query";
import { BoxIcon, LayersIcon, PencilIcon, WrenchIcon } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { EditInstanceVersionDialog } from "@/features/instance-settings/components/edit-instance-version-dialog";
import { versionsQueryOptions } from "@/features/instances/lib/versions-query";
import { repairInstance } from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";
import type { VersionSummary } from "@/lib/generated/VersionSummary";

function versionsForPicker(
  versions: VersionSummary[] | undefined,
  versionId: string,
): VersionSummary[] {
  if (versions !== undefined && versions.length > 0) {
    return versions;
  }
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
  onChangeVersion,
}: {
  folder: string;
  versionId: string;
  busy: boolean;
  versionBusy: boolean;
  onChangeVersion: (versionId: string, installed: boolean) => void;
}) {
  const [editOpen, setEditOpen] = useState(false);
  const versions = useQuery({
    ...versionsQueryOptions,
    enabled: editOpen,
  });
  const repair = useMutation({
    mutationFn: () => repairInstance(folder),
    onSuccess: () => {
      toast.success("Instance files were repaired.");
    },
    onError: (error: unknown) => {
      toast.error(friendlyError(messageOf(error)));
    },
  });

  const pickerVersions = versionsForPicker(versions.data, versionId);
  const versionsLoading = editOpen && versions.isPending;
  const versionsFailed = editOpen && versions.isError;

  return (
    <div className="flex flex-col gap-6">
      <div className="grid gap-4 rounded-xl bg-background/60 p-4 sm:grid-cols-2">
        <div className="flex min-w-0 flex-col gap-2">
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <LayersIcon aria-hidden className="size-4 shrink-0" />
            <span className="font-medium text-foreground">Platform</span>
          </div>
          <div className="flex h-8 items-center rounded-lg bg-muted/40 px-2.5 text-sm font-medium">
            Vanilla
          </div>
        </div>
        <div className="flex min-w-0 flex-col gap-2">
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <BoxIcon aria-hidden className="size-4 shrink-0" />
            <span className="font-medium text-foreground">Game version</span>
          </div>
          <div className="flex h-8 items-center rounded-lg bg-muted/40 px-2.5 text-sm font-medium">
            {versionBusy ? (
              <span className="flex items-center gap-2">
                <Spinner className="size-3.5" aria-hidden />
                Updating…
              </span>
            ) : (
              <span className="truncate">{versionId}</span>
            )}
          </div>
        </div>
      </div>

      <div className="flex flex-col gap-2">
        <div className="flex flex-col gap-1">
          <h3 className="text-sm font-medium">Edit base</h3>
          <p className="text-sm text-muted-foreground">
            Open the editor to pick another Vanilla release for this instance.
          </p>
        </div>
        <Button
          type="button"
          variant="warning"
          className="self-start"
          disabled={busy || versionBusy}
          onClick={() => {
            setEditOpen(true);
          }}
        >
          <PencilIcon data-icon="inline-start" aria-hidden />
          Edit base
        </Button>
      </div>

      <div className="flex flex-col gap-2">
        <div className="flex flex-col gap-1">
          <h3 className="text-sm font-medium">Repair</h3>
          <p className="text-sm text-muted-foreground">
            Re-download missing game files and modpack content for this
            instance.
          </p>
        </div>
        <Button
          type="button"
          variant="outline"
          className="self-start"
          disabled={busy || versionBusy || repair.isPending}
          onClick={() => {
            repair.mutate();
          }}
        >
          {repair.isPending ? (
            <Spinner
              className="size-3.5"
              data-icon="inline-start"
              aria-hidden
            />
          ) : (
            <WrenchIcon data-icon="inline-start" aria-hidden />
          )}
          Repair
        </Button>
      </div>

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
    </div>
  );
}
