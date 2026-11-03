import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { ArrowLeftIcon, PlusIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  VERSIONS_QUERY_KEY,
  versionsQueryOptions,
} from "@/features/instances/lib/versions-query";
import { createVersionChoices } from "@/features/instances/model/create-versions";
import { ScratchDetails } from "@/features/new-instance/components/scratch-details";
import { SourcePicker } from "@/features/new-instance/components/source-picker";
import {
  canCreateScratch,
  initialScratchState,
  platformNeedsLoader,
  type ScratchState,
  uploadInstanceName,
} from "@/features/new-instance/model/create-state";
import {
  browseModpackFile,
  createInstanceFromModpackFile,
  createInstanceWithPlatform,
  listLoaderVersions,
} from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

type View = "source" | "details";

function CreateInstanceFlow({ onClose }: { onClose: () => void }) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [view, setView] = useState<View>("source");
  const [scratch, setScratch] = useState<ScratchState>(initialScratchState);

  function update(patch: Partial<ScratchState>) {
    setScratch((current) => ({ ...current, ...patch }));
  }

  const versions = useQuery({
    ...versionsQueryOptions,
    enabled: view === "details",
  });

  const loaderVersions = useQuery({
    queryKey: [
      "loader-versions",
      scratch.platform,
      scratch.gameVersion,
      scratch.loaderChannel,
    ],
    queryFn: () =>
      listLoaderVersions(
        scratch.platform,
        scratch.gameVersion,
        scratch.loaderChannel,
      ),
    enabled:
      view === "details" &&
      platformNeedsLoader(scratch.platform) &&
      scratch.gameVersion !== "",
    retry: false,
  });

  useEffect(() => {
    if (view !== "details" || scratch.gameVersion !== "") {
      return;
    }
    const newest = createVersionChoices(
      versions.data ?? [],
      scratch.includeSnapshots,
    )[0];
    if (newest !== undefined) {
      setScratch((current) => ({ ...current, gameVersion: newest.id }));
    }
  }, [view, scratch.gameVersion, scratch.includeSnapshots, versions.data]);

  useEffect(() => {
    if (
      view !== "details" ||
      !platformNeedsLoader(scratch.platform) ||
      scratch.loaderChannel === "manual"
    ) {
      return;
    }
    const first = loaderVersions.data?.[0];
    if (first !== undefined && scratch.loaderVersion !== first) {
      setScratch((current) => ({ ...current, loaderVersion: first }));
    }
  }, [
    view,
    scratch.platform,
    scratch.loaderChannel,
    scratch.loaderVersion,
    loaderVersions.data,
  ]);

  async function finish(entry: InstanceEntry) {
    await queryClient.invalidateQueries({ queryKey: ["instances"] });
    await queryClient.invalidateQueries({ queryKey: VERSIONS_QUERY_KEY });
    await navigate({ to: "/", search: { prepare: entry.folder } });
    onClose();
  }

  const createFromScratch = useMutation({
    mutationFn: () =>
      createInstanceWithPlatform({
        name: scratch.name.trim(),
        platform: scratch.platform,
        gameVersion: scratch.gameVersion,
        loaderChannel: scratch.loaderChannel,
        loaderVersion:
          scratch.loaderChannel === "manual" ? scratch.loaderVersion : null,
      }),
    onSuccess: finish,
    onError: (error: unknown) => {
      toast.error(friendlyError(messageOf(error)));
    },
  });

  const upload = useMutation({
    mutationFn: async () => {
      const path = await browseModpackFile();
      if (path === null) {
        return null;
      }
      return createInstanceFromModpackFile({
        name: uploadInstanceName(path),
        path,
      });
    },
    onSuccess: async (entry) => {
      if (entry !== null) {
        await finish(entry);
      }
    },
    onError: (error: unknown) => {
      toast.error(friendlyError(messageOf(error)));
    },
  });

  const busy = createFromScratch.isPending || upload.isPending;

  return (
    <>
      <DialogHeader>
        <DialogTitle>Create instance</DialogTitle>
        <DialogDescription>
          {view === "source"
            ? "How do you want to start?"
            : "Name it, pick a platform and a game version."}
        </DialogDescription>
      </DialogHeader>

      {view === "source" ? (
        <SourcePicker
          busy={busy}
          onScratch={() => {
            setView("details");
          }}
          onUpload={() => {
            upload.mutate();
          }}
        />
      ) : (
        <>
          <ScratchDetails
            state={scratch}
            versions={versions.data ?? []}
            versionsPending={versions.isPending}
            versionsError={versions.isError}
            loaderVersions={loaderVersions.data ?? []}
            loaderVersionsPending={loaderVersions.isPending}
            onRetryVersions={() => {
              void versions.refetch();
            }}
            onChange={update}
          />
          <DialogFooter className="sm:justify-between">
            <Button
              type="button"
              variant="outline"
              disabled={busy}
              onClick={() => {
                setView("source");
              }}
            >
              <ArrowLeftIcon data-icon="inline-start" aria-hidden />
              Back
            </Button>
            <Button
              type="button"
              disabled={!canCreateScratch(scratch) || busy}
              onClick={() => {
                createFromScratch.mutate();
              }}
            >
              <PlusIcon data-icon="inline-start" aria-hidden />
              {createFromScratch.isPending ? "Creating…" : "Create"}
            </Button>
          </DialogFooter>
        </>
      )}
    </>
  );
}

export function CreateInstanceDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[calc(100%-2rem)] overflow-y-auto sm:max-w-4xl">
        <CreateInstanceFlow
          onClose={() => {
            onOpenChange(false);
          }}
        />
      </DialogContent>
    </Dialog>
  );
}
