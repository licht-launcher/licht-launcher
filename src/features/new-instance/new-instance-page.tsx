import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import {
  CopyIcon,
  FileUpIcon,
  PackageIcon,
  PlusIcon,
  WrenchIcon,
  XIcon,
} from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  VERSIONS_QUERY_KEY,
  versionsQueryOptions,
} from "@/features/instances/lib/versions-query";
import { createVersionChoices } from "@/features/instances/model/create-versions";
import { StepClone } from "@/features/new-instance/components/step-clone";
import { StepDetails } from "@/features/new-instance/components/step-details";
import { StepFile } from "@/features/new-instance/components/step-file";
import { StepModpack } from "@/features/new-instance/components/step-modpack";
import { StepModpackVersion } from "@/features/new-instance/components/step-modpack-version";
import { StepPlatform } from "@/features/new-instance/components/step-platform";
import {
  type CreateState,
  canCreate,
  fileBaseName,
  type InstanceSource,
  initialCreateState,
  platformNeedsLoader,
} from "@/features/new-instance/model/create-state";
import {
  browseModpackFile,
  createInstanceFromModpack,
  createInstanceFromModpackFile,
  createInstanceWithPlatform,
  duplicateInstance,
  listInstances,
  listLoaderVersions,
  listModpackVersions,
  searchModpacks,
} from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";

const SOURCES: readonly InstanceSource[] = [
  "scratch",
  "modpack",
  "file",
  "clone",
];

function isInstanceSource(value: string): value is InstanceSource {
  return (SOURCES as readonly string[]).includes(value);
}

export function NewInstancePage() {
  const navigate = useNavigate({ from: "/instances/new" });
  const queryClient = useQueryClient();
  const [state, setState] = useState<CreateState>(initialCreateState);

  function update(patch: Partial<CreateState>) {
    setState((current) => ({ ...current, ...patch }));
  }

  const versions = useQuery(versionsQueryOptions);

  const modpacks = useQuery({
    queryKey: ["modpacks", state.modpackQuery],
    queryFn: () => searchModpacks(state.modpackQuery),
    enabled: state.source === "modpack",
    retry: false,
  });

  const packVersions = useQuery({
    queryKey: [
      "modpack-versions",
      state.modpack?.source ?? "modrinth",
      state.modpack?.id ?? "",
    ],
    queryFn: () =>
      listModpackVersions(
        state.modpack?.source ?? "modrinth",
        state.modpack?.id ?? "",
      ),
    enabled: state.source === "modpack" && state.modpack !== null,
    retry: false,
  });

  const loaderVersions = useQuery({
    queryKey: [
      "loader-versions",
      state.platform,
      state.gameVersion,
      state.loaderChannel,
    ],
    queryFn: () =>
      listLoaderVersions(
        state.platform,
        state.gameVersion,
        state.loaderChannel,
      ),
    enabled:
      state.source === "scratch" &&
      platformNeedsLoader(state.platform) &&
      state.gameVersion !== "",
    retry: false,
  });

  const instances = useQuery({
    queryKey: ["instances"],
    queryFn: listInstances,
    enabled: state.source === "clone",
  });

  useEffect(() => {
    if (state.source !== "scratch" || state.gameVersion !== "") {
      return;
    }
    const choices = createVersionChoices(
      versions.data ?? [],
      state.includeSnapshots,
    );
    if (choices[0] !== undefined) {
      setState((current) => ({
        ...current,
        gameVersion: choices[0]?.id ?? "",
      }));
    }
  }, [state.source, state.gameVersion, state.includeSnapshots, versions.data]);

  useEffect(() => {
    if (
      state.source !== "scratch" ||
      !platformNeedsLoader(state.platform) ||
      state.loaderChannel === "manual"
    ) {
      return;
    }
    const first = loaderVersions.data?.[0];
    if (first !== undefined && state.loaderVersion !== first) {
      setState((current) => ({ ...current, loaderVersion: first }));
    }
  }, [
    state.source,
    state.platform,
    state.loaderChannel,
    state.loaderVersion,
    loaderVersions.data,
  ]);

  useEffect(() => {
    if (state.source !== "modpack" || state.modpackVersion !== null) {
      return;
    }
    const first = packVersions.data?.[0];
    if (first !== undefined) {
      setState((current) => ({ ...current, modpackVersion: first }));
    }
  }, [state.source, state.modpackVersion, packVersions.data]);

  const create = useMutation({
    mutationFn: async () => {
      const name = state.name.trim();
      switch (state.source) {
        case "scratch":
          return createInstanceWithPlatform({
            name,
            platform: state.platform,
            gameVersion: state.gameVersion,
            loaderChannel: state.loaderChannel,
            loaderVersion:
              state.loaderChannel === "manual" ? state.loaderVersion : null,
          });
        case "modpack":
          if (state.modpack === null || state.modpackVersion === null) {
            throw new Error("Pick a modpack version first.");
          }
          return createInstanceFromModpack({
            name,
            source: state.modpack.source,
            projectId: state.modpack.id,
            versionId: state.modpackVersion.id,
            iconUrl: state.modpack.iconUrl,
          });
        case "file":
          return createInstanceFromModpackFile({
            name,
            path: state.filePath,
          });
        case "clone":
          return duplicateInstance(state.cloneFolder, name);
      }
    },
    onSuccess: async (entry) => {
      await queryClient.invalidateQueries({ queryKey: ["instances"] });
      await queryClient.invalidateQueries({ queryKey: VERSIONS_QUERY_KEY });
      await navigate({
        to: "/",
        search: { prepare: entry.folder },
      });
    },
    onError: (error: unknown) => {
      toast.error(friendlyError(messageOf(error)));
    },
  });

  async function browseFile() {
    try {
      const filePath = await browseModpackFile();
      if (filePath === null) {
        return;
      }
      setState((current) => ({
        ...current,
        filePath,
        name:
          current.name.trim() === "" ? fileBaseName(filePath) : current.name,
      }));
    } catch (error) {
      toast.error(friendlyError(messageOf(error)));
    }
  }

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden px-8 pt-6 pr-12 pb-8">
      <Tabs
        value={state.source}
        onValueChange={(next) => {
          if (isInstanceSource(next)) {
            update({ source: next });
          }
        }}
        className="min-h-0 flex-1 gap-4"
      >
        <TabsList variant="sliding" className="w-fit shrink-0">
          <TabsTrigger value="scratch" className="cursor-pointer gap-1.5 px-3">
            <WrenchIcon aria-hidden className="size-4 shrink-0" />
            From scratch
          </TabsTrigger>
          <TabsTrigger value="modpack" className="cursor-pointer gap-1.5 px-3">
            <PackageIcon aria-hidden className="size-4 shrink-0" />
            Modpack
          </TabsTrigger>
          <TabsTrigger value="file" className="cursor-pointer gap-1.5 px-3">
            <FileUpIcon aria-hidden className="size-4 shrink-0" />
            Pack file
          </TabsTrigger>
          <TabsTrigger value="clone" className="cursor-pointer gap-1.5 px-3">
            <CopyIcon aria-hidden className="size-4 shrink-0" />
            Clone
          </TabsTrigger>
        </TabsList>

        <TabsContent
          value="scratch"
          className="grid min-h-0 flex-1 gap-6 overflow-y-auto md:grid-cols-3"
        >
          <StepPlatform
            value={state.platform}
            onChange={(platform) => {
              update({ platform, loaderVersion: "" });
            }}
          />
          <div className="md:col-span-2">
            <StepDetails
              platform={state.platform}
              name={state.name}
              gameVersion={state.gameVersion}
              includeSnapshots={state.includeSnapshots}
              versions={versions.data ?? []}
              versionsPending={versions.isPending}
              versionsError={versions.isError}
              onRetryVersions={() => {
                void versions.refetch();
              }}
              loaderChannel={state.loaderChannel}
              loaderVersion={state.loaderVersion}
              loaderVersions={loaderVersions.data ?? []}
              loaderVersionsPending={loaderVersions.isPending}
              onNameChange={(name) => {
                update({ name });
              }}
              onGameVersionChange={(gameVersion) => {
                update({ gameVersion, loaderVersion: "" });
              }}
              onIncludeSnapshotsChange={(includeSnapshots) => {
                update({ includeSnapshots });
              }}
              onLoaderChannelChange={(loaderChannel) => {
                update({ loaderChannel, loaderVersion: "" });
              }}
              onLoaderVersionChange={(loaderVersion) => {
                update({ loaderVersion });
              }}
            />
          </div>
        </TabsContent>

        <TabsContent
          value="modpack"
          className="grid min-h-0 flex-1 gap-6 md:grid-cols-2"
        >
          <StepModpack
            query={state.modpackQuery}
            items={modpacks.data ?? []}
            selectedId={state.modpack?.id ?? null}
            pending={modpacks.isPending}
            error={
              modpacks.isError ? friendlyError(messageOf(modpacks.error)) : null
            }
            onQueryChange={(modpackQuery) => {
              update({ modpackQuery });
            }}
            onSelect={(modpack) => {
              update({ modpack, modpackVersion: null, name: modpack.name });
            }}
          />
          {state.modpack === null ? (
            <p className="text-sm text-muted-foreground">
              Pick a modpack to choose its version.
            </p>
          ) : (
            <StepModpackVersion
              name={state.name}
              versions={packVersions.data ?? []}
              versionId={state.modpackVersion?.id ?? ""}
              versionsPending={packVersions.isPending}
              onNameChange={(name) => {
                update({ name });
              }}
              onVersionChange={(versionId) => {
                update({
                  modpackVersion:
                    packVersions.data?.find(
                      (entry) => entry.id === versionId,
                    ) ?? null,
                });
              }}
            />
          )}
        </TabsContent>

        <TabsContent value="file" className="min-h-0 flex-1 overflow-y-auto">
          <StepFile
            filePath={state.filePath}
            name={state.name}
            onBrowse={() => {
              void browseFile();
            }}
            onNameChange={(name) => {
              update({ name });
            }}
          />
        </TabsContent>

        <TabsContent value="clone" className="flex min-h-0 flex-1 flex-col">
          <StepClone
            instances={instances.data ?? []}
            pending={instances.isPending}
            selectedFolder={state.cloneFolder}
            name={state.name}
            onSelect={(instance) => {
              update({
                cloneFolder: instance.folder,
                name: `${instance.name} copy`,
              });
            }}
            onNameChange={(name) => {
              update({ name });
            }}
          />
        </TabsContent>
      </Tabs>

      <footer className="flex shrink-0 justify-between gap-3 pt-4">
        <Button
          type="button"
          variant="outline"
          onClick={() => {
            void navigate({ to: "/", search: {} });
          }}
        >
          <XIcon data-icon="inline-start" aria-hidden />
          Cancel
        </Button>
        <Button
          type="button"
          disabled={!canCreate(state) || create.isPending}
          onClick={() => {
            create.mutate();
          }}
        >
          <PlusIcon data-icon="inline-start" aria-hidden />
          {create.isPending ? "Creating…" : "Create instance"}
        </Button>
      </footer>
    </div>
  );
}
