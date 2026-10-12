import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { ArrowLeftIcon, ArrowRightIcon, PlusIcon, XIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
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
  nextStep,
  platformNeedsLoader,
  previousStep,
  stepIndex,
  stepsFor,
  type WizardState,
} from "@/features/new-instance/model/wizard";
import {
  createInstanceFromModpack,
  createInstanceWithPlatform,
  listLoaderVersions,
  listModpackVersions,
  listVersions,
  searchModpacks,
} from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";

export function NewInstancePage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [state, setState] = useState<WizardState>(initialWizardState);

  const versions = useQuery({
    queryKey: ["versions"],
    queryFn: listVersions,
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

  const modpacks = useQuery({
    queryKey: ["modpacks", state.modpackQuery],
    queryFn: () => searchModpacks(state.modpackQuery),
    enabled: state.step === "modpack",
    retry: false,
  });

  const packVersions = useQuery({
    queryKey: ["modpack-versions", state.modpack?.source, state.modpack?.id],
    queryFn: () =>
      listModpackVersions(
        state.modpack?.source ?? "modrinth",
        state.modpack?.id ?? "",
      ),
    enabled: state.step === "modpack-version" && state.modpack !== null,
    retry: false,
  });

  useEffect(() => {
    if (state.step !== "details" || state.gameVersion !== "") {
      return;
    }
    const choices = createVersionChoices(
      versions.data ?? [],
      state.includeSnapshots,
    );
    if (choices[0] !== undefined) {
      setState((current) => ({ ...current, gameVersion: choices[0].id }));
    }
  }, [state.step, state.gameVersion, state.includeSnapshots, versions.data]);

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
      setState((current) => ({ ...current, loaderVersion: first }));
    }
  }, [
    state.step,
    state.platform,
    state.loaderChannel,
    state.loaderVersion,
    loaderVersions.data,
  ]);

  useEffect(() => {
    if (state.step !== "modpack-version" || state.modpackVersion !== null) {
      return;
    }
    const first = packVersions.data?.[0];
    if (first !== undefined) {
      setState((current) => ({
        ...current,
        modpackVersion: first,
        name:
          current.name.trim() === ""
            ? (current.modpack?.name ?? "")
            : current.name,
      }));
    }
  }, [state.step, state.modpackVersion, packVersions.data]);

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
      await queryClient.invalidateQueries({ queryKey: ["versions"] });
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
    setState((current) => ({ ...current, step: nextStep(current) }));
  }

  function goBack() {
    if (state.step === "source") {
      void navigate({ to: "/", search: {} });
      return;
    }
    setState((current) => ({ ...current, step: previousStep(current) }));
  }

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-muted/30 px-8 pt-6 pr-12 pb-8">
      <div className="flex min-h-0 flex-1 flex-col gap-4">
        <WizardStepper steps={path} currentIndex={Math.max(index, 0)} />

        <div className="flex min-h-0 flex-1 flex-col">
          {state.step === "source" ? (
            <StepSource
              value={state.source}
              onChange={(source) => {
                setState({
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
                setState((current) => ({
                  ...current,
                  platform,
                  loaderVersion: "",
                }));
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
                setState((current) => ({ ...current, name }));
              }}
              onGameVersionChange={(gameVersion) => {
                setState((current) => ({
                  ...current,
                  gameVersion,
                  loaderVersion: "",
                }));
              }}
              onIncludeSnapshotsChange={(includeSnapshots) => {
                setState((current) => ({ ...current, includeSnapshots }));
              }}
              onLoaderChannelChange={(loaderChannel) => {
                setState((current) => ({
                  ...current,
                  loaderChannel,
                  loaderVersion: "",
                }));
              }}
              onLoaderVersionChange={(loaderVersion) => {
                setState((current) => ({ ...current, loaderVersion }));
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
                setState((current) => ({ ...current, modpackQuery }));
              }}
              onSelect={(modpack) => {
                setState((current) => ({
                  ...current,
                  modpack,
                  modpackVersion: null,
                  name: modpack.name,
                }));
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
                setState((current) => ({ ...current, name }));
              }}
              onVersionChange={(versionId) => {
                const modpackVersion =
                  packVersions.data?.find((entry) => entry.id === versionId) ??
                  null;
                setState((current) => ({ ...current, modpackVersion }));
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
