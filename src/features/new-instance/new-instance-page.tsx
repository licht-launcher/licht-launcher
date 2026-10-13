import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { ArrowLeftIcon, ArrowRightIcon, PlusIcon, XIcon } from "lucide-react";
import { useCallback, useEffect, useMemo } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  VERSIONS_QUERY_KEY,
  versionsQueryOptions,
} from "@/features/instances/lib/versions-query";
import { createVersionChoices } from "@/features/instances/model/create-versions";
import { StepConfirm } from "@/features/new-instance/components/step-confirm";
import { StepDetails } from "@/features/new-instance/components/step-details";
import { StepModpack } from "@/features/new-instance/components/step-modpack";
import { StepModpackVersion } from "@/features/new-instance/components/step-modpack-version";
import { StepPlatform } from "@/features/new-instance/components/step-platform";
import { StepSource } from "@/features/new-instance/components/step-source";
import { WizardStepper } from "@/features/new-instance/components/wizard-stepper";
import {
  canAdvance,
  initialWizardState,
  type ModpackListItem,
  type ModpackVersionItem,
  nextStep,
  platformNeedsLoader,
  previousStep,
  stepIndex,
  stepsFor,
  type WizardState,
} from "@/features/new-instance/model/wizard";
import {
  type NewInstanceSearch,
  searchFromWizardState,
  wizardStateFromSearch,
} from "@/features/new-instance/model/wizard-search";
import {
  createInstanceFromModpack,
  createInstanceWithPlatform,
  listLoaderVersions,
  listModpackVersions,
  searchModpacks,
} from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";
import { Route } from "@/routes/_app/instances.new";

function resolveModpack(
  search: NewInstanceSearch,
  items: ModpackListItem[],
): ModpackListItem | null {
  if (search.modpackId === undefined) {
    return null;
  }
  const found = items.find((item) => item.id === search.modpackId);
  if (found !== undefined) {
    return found;
  }
  return {
    id: search.modpackId,
    source: search.modpackSource ?? "modrinth",
    name: search.name ?? search.modpackId,
    description: "",
    downloads: 0,
    iconUrl: null,
    author: "",
  };
}

function resolveModpackVersion(
  search: NewInstanceSearch,
  versions: ModpackVersionItem[],
): ModpackVersionItem | null {
  if (search.modpackVersionId === undefined) {
    return null;
  }
  const found = versions.find((item) => item.id === search.modpackVersionId);
  if (found !== undefined) {
    return found;
  }
  return {
    id: search.modpackVersionId,
    name: search.modpackVersionId,
    gameVersions: [],
    loaders: [],
  };
}

export function NewInstancePage() {
  const navigate = useNavigate({ from: "/instances/new" });
  const search = Route.useSearch();
  const queryClient = useQueryClient();

  const versions = useQuery(versionsQueryOptions);

  const modpacks = useQuery({
    queryKey: ["modpacks", search.modpackQuery ?? ""],
    queryFn: () => searchModpacks(search.modpackQuery ?? ""),
    enabled: search.step === "modpack" || search.modpackId !== undefined,
    retry: false,
  });

  const packVersions = useQuery({
    queryKey: [
      "modpack-versions",
      search.modpackSource ?? "modrinth",
      search.modpackId ?? "",
    ],
    queryFn: () =>
      listModpackVersions(
        search.modpackSource ?? "modrinth",
        search.modpackId ?? "",
      ),
    enabled:
      (search.step === "modpack-version" ||
        search.modpackVersionId !== undefined) &&
      search.modpackId !== undefined,
    retry: false,
  });

  const modpack = useMemo(
    () => resolveModpack(search, modpacks.data ?? []),
    [search, modpacks.data],
  );
  const modpackVersion = useMemo(
    () => resolveModpackVersion(search, packVersions.data ?? []),
    [search, packVersions.data],
  );

  const state = useMemo(
    () => wizardStateFromSearch(search, modpack, modpackVersion),
    [search, modpack, modpackVersion],
  );

  const loaderVersions = useQuery({
    queryKey: [
      "loader-versions",
      state.platform,
      state.gameVersion,
      state.loaderChannel,
    ],
    queryFn: () =>
      listLoaderVersions(
        state.platform ?? "fabric",
        state.gameVersion,
        state.loaderChannel,
      ),
    enabled:
      state.step === "details" &&
      platformNeedsLoader(state.platform) &&
      state.gameVersion !== "",
    retry: false,
  });

  const replaceState = useCallback(
    (next: WizardState) => {
      void navigate({
        to: "/instances/new",
        search: searchFromWizardState(next),
        replace: true,
      });
    },
    [navigate],
  );

  const pushState = useCallback(
    (next: WizardState) => {
      void navigate({
        to: "/instances/new",
        search: searchFromWizardState(next),
        replace: false,
      });
    },
    [navigate],
  );

  useEffect(() => {
    if (state.step !== "details" || state.gameVersion !== "") {
      return;
    }
    const choices = createVersionChoices(
      versions.data ?? [],
      state.includeSnapshots,
    );
    if (choices[0] !== undefined) {
      replaceState({ ...state, gameVersion: choices[0].id });
    }
  }, [state, versions.data, replaceState]);

  useEffect(() => {
    if (
      state.step !== "details" ||
      !platformNeedsLoader(state.platform) ||
      state.loaderChannel === "manual"
    ) {
      return;
    }
    const first = loaderVersions.data?.[0];
    if (first !== undefined && state.loaderVersion !== first) {
      replaceState({ ...state, loaderVersion: first });
    }
  }, [state, loaderVersions.data, replaceState]);

  useEffect(() => {
    if (state.step !== "modpack-version" || state.modpackVersion !== null) {
      return;
    }
    const first = packVersions.data?.[0];
    if (first !== undefined) {
      replaceState({
        ...state,
        modpackVersion: first,
        name:
          state.name.trim() === "" ? (state.modpack?.name ?? "") : state.name,
      });
    }
  }, [state, packVersions.data, replaceState]);

  const create = useMutation({
    mutationFn: async () => {
      if (state.source === "modpack") {
        if (state.modpack === null || state.modpackVersion === null) {
          throw new Error("Pick a modpack version first.");
        }
        return createInstanceFromModpack({
          name: state.name.trim(),
          source: state.modpack.source,
          projectId: state.modpack.id,
          versionId: state.modpackVersion.id,
          iconUrl: state.modpack.iconUrl,
        });
      }
      if (state.platform === null) {
        throw new Error("Pick a platform first.");
      }
      return createInstanceWithPlatform({
        name: state.name.trim(),
        platform: state.platform,
        gameVersion: state.gameVersion,
        loaderChannel: state.loaderChannel,
        loaderVersion:
          state.loaderChannel === "manual" ? state.loaderVersion : null,
      });
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

  const path = stepsFor(state);
  const index = stepIndex(state);

  function goNext() {
    if (!canAdvance(state)) {
      return;
    }
    if (state.step === "confirm") {
      create.mutate();
      return;
    }
    pushState({ ...state, step: nextStep(state) });
  }

  function goBack() {
    if (state.step === "source") {
      void navigate({ to: "/", search: {} });
      return;
    }
    pushState({ ...state, step: previousStep(state) });
  }

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden px-8 pt-6 pr-12 pb-8">
      <div className="flex min-h-0 flex-1 flex-col gap-4">
        <WizardStepper steps={path} currentIndex={Math.max(index, 0)} />

        <div className="flex min-h-0 flex-1 flex-col">
          {state.step === "source" ? (
            <StepSource
              value={state.source}
              onChange={(source) => {
                replaceState({
                  ...initialWizardState,
                  step: "source",
                  source,
                });
              }}
            />
          ) : null}
          {state.step === "platform" ? (
            <StepPlatform
              value={state.platform}
              onChange={(platform) => {
                replaceState({
                  ...state,
                  platform,
                  loaderVersion: "",
                });
              }}
            />
          ) : null}
          {state.step === "details" ? (
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
                replaceState({ ...state, name });
              }}
              onGameVersionChange={(gameVersion) => {
                replaceState({
                  ...state,
                  gameVersion,
                  loaderVersion: "",
                });
              }}
              onIncludeSnapshotsChange={(includeSnapshots) => {
                replaceState({ ...state, includeSnapshots });
              }}
              onLoaderChannelChange={(loaderChannel) => {
                replaceState({
                  ...state,
                  loaderChannel,
                  loaderVersion: "",
                });
              }}
              onLoaderVersionChange={(loaderVersion) => {
                replaceState({ ...state, loaderVersion });
              }}
            />
          ) : null}
          {state.step === "modpack" ? (
            <StepModpack
              query={state.modpackQuery}
              items={modpacks.data ?? []}
              selectedId={state.modpack?.id ?? null}
              pending={modpacks.isPending}
              error={
                modpacks.isError
                  ? friendlyError(messageOf(modpacks.error))
                  : null
              }
              onQueryChange={(modpackQuery) => {
                replaceState({ ...state, modpackQuery });
              }}
              onSelect={(nextModpack) => {
                replaceState({
                  ...state,
                  modpack: nextModpack,
                  modpackVersion: null,
                  name: nextModpack.name,
                });
              }}
            />
          ) : null}
          {state.step === "modpack-version" ? (
            <StepModpackVersion
              name={state.name}
              versions={packVersions.data ?? []}
              versionId={state.modpackVersion?.id ?? ""}
              versionsPending={packVersions.isPending}
              onNameChange={(name) => {
                replaceState({ ...state, name });
              }}
              onVersionChange={(versionId) => {
                const nextVersion =
                  packVersions.data?.find((entry) => entry.id === versionId) ??
                  null;
                replaceState({ ...state, modpackVersion: nextVersion });
              }}
            />
          ) : null}
          {state.step === "confirm" ? <StepConfirm state={state} /> : null}
        </div>

        <footer className="flex justify-between gap-3 pt-2">
          <Button type="button" variant="outline" onClick={goBack}>
            {state.step === "source" ? (
              <XIcon data-icon="inline-start" aria-hidden />
            ) : (
              <ArrowLeftIcon data-icon="inline-start" aria-hidden />
            )}
            {state.step === "source" ? "Cancel" : "Back"}
          </Button>
          <Button
            type="button"
            disabled={!canAdvance(state) || create.isPending}
            onClick={goNext}
          >
            {state.step === "confirm" ? (
              <PlusIcon data-icon="inline-start" aria-hidden />
            ) : null}
            {state.step === "confirm"
              ? create.isPending
                ? "Creating…"
                : "Create instance"
              : "Continue"}
            {state.step === "confirm" ? null : (
              <ArrowRightIcon data-icon="inline-end" aria-hidden />
            )}
          </Button>
        </footer>
      </div>
    </div>
  );
}
