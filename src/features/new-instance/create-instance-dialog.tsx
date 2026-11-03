import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { ArrowLeftIcon, PlusIcon, WrenchIcon, XIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { PageTransition } from "@/components/ui/page-transition";
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
  const [direction, setDirection] = useState<"left" | "right">("left");
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

  const HeaderIcon = view === "source" ? PlusIcon : WrenchIcon;

  return (
    <>
      <DialogHeader className="flex-row items-center gap-4 border-b border-border/40 pb-5">
        <span
          data-slot="create-header-icon"
          className="flex size-11 shrink-0 items-center justify-center rounded-xl bg-instance/20 text-instance"
        >
          <HeaderIcon aria-hidden className="size-5" strokeWidth={2} />
        </span>
        <div className="flex min-w-0 flex-1 flex-col gap-1">
          <div className="flex items-center justify-between gap-2">
            <DialogTitle className="text-lg leading-tight">
              Create instance
            </DialogTitle>
            <DialogClose asChild>
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                className="-my-1"
              >
                <XIcon aria-hidden />
                <span className="sr-only">Close</span>
              </Button>
            </DialogClose>
          </div>
        </div>
      </DialogHeader>

      <PageTransition animationKey={view} direction={direction}>
        {view === "source" ? (
          <SourcePicker
            busy={busy}
            onScratch={() => {
              setDirection("left");
              setView("details");
            }}
            onUpload={() => {
              upload.mutate();
            }}
          />
        ) : (
          <div className="flex flex-col gap-5">
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
            <DialogFooter className="-mx-6 -mb-6 px-6 py-4 sm:justify-between">
              <Button
                type="button"
                variant="outline"
                disabled={busy}
                onClick={() => {
                  setDirection("right");
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
          </div>
        )}
      </PageTransition>
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
      <DialogContent
        showCloseButton={false}
        aria-describedby={undefined}
        className="max-h-[calc(100%-2rem)] gap-5 overflow-y-auto p-6 sm:max-w-xl"
      >
        <CreateInstanceFlow
          onClose={() => {
            onOpenChange(false);
          }}
        />
      </DialogContent>
    </Dialog>
  );
}
