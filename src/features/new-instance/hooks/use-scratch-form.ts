import { useQuery } from "@tanstack/react-query";
import { useCallback, useEffect, useState } from "react";

import { versionsQueryOptions } from "@/features/instances/lib/versions-query";
import { createVersionChoices } from "@/features/instances/model/create-versions";
import {
  initialScratchState,
  type LoaderState,
  platformNeedsLoader,
  type ScratchState,
} from "@/features/new-instance/model/create-state";
import { listLoaderBuilds } from "@/lib/commands";

/**
 * The from-scratch form: its state, the version lists it depends on, and the
 * defaults that fill in once those lists load. It lives as long as the popup
 * does, so the lists start loading when the popup opens.
 */
export function useScratchForm() {
  const [state, setState] = useState<ScratchState>(initialScratchState);

  const update = useCallback((patch: Partial<ScratchState>) => {
    setState((current) => ({ ...current, ...patch }));
  }, []);

  const versions = useQuery(versionsQueryOptions);

  const loaderListed =
    platformNeedsLoader(state.platform) && state.gameVersion !== "";

  // One request lists every build with its stability; the channels only pick
  // from it, so switching between them never asks again.
  const builds = useQuery({
    queryKey: ["loader-builds", state.platform, state.gameVersion],
    queryFn: () => listLoaderBuilds(state.platform, state.gameVersion),
    enabled: loaderListed,
    retry: false,
  });
  const stableMissing =
    loaderListed &&
    builds.data !== undefined &&
    !builds.data.some((build) => build.stable);
  const loaderVersions = {
    isError: builds.isError,
    data: builds.data?.map((build) => build.version),
  };
  const newestLoader =
    state.loaderChannel === "stable"
      ? builds.data?.find((build) => build.stable)?.version
      : builds.data?.[0]?.version;

  useEffect(() => {
    if (state.gameVersion !== "") {
      return;
    }
    const newest = createVersionChoices(
      versions.data ?? [],
      state.includeSnapshots,
    )[0];
    if (newest !== undefined) {
      setState((current) => ({ ...current, gameVersion: newest.id }));
    }
  }, [state.gameVersion, state.includeSnapshots, versions.data]);

  useEffect(() => {
    if (stableMissing && state.loaderChannel === "stable") {
      setState((current) => ({
        ...current,
        loaderChannel: "latest",
        loaderVersion: "",
      }));
    }
  }, [stableMissing, state.loaderChannel]);

  useEffect(() => {
    if (!platformNeedsLoader(state.platform)) {
      return;
    }
    const newest = newestLoader;
    if (newest === undefined) {
      return;
    }
    // Stable and latest always follow the newest build; "manual" starts there
    // too, and then keeps whatever the user picks.
    const keepsChoice =
      state.loaderChannel === "manual" && state.loaderVersion !== "";
    if (!keepsChoice && state.loaderVersion !== newest) {
      setState((current) => ({ ...current, loaderVersion: newest }));
    }
  }, [
    state.platform,
    state.loaderChannel,
    state.loaderVersion,
    newestLoader,
  ]);

  return {
    state,
    update,
    versions,
    loaderVersions,
    loaderState: loaderStateOf(state, loaderVersions),
    stableMissing,
  };
}

function loaderStateOf(
  state: ScratchState,
  loaderVersions: { isError: boolean; data: string[] | undefined },
): LoaderState {
  if (!platformNeedsLoader(state.platform)) {
    return "none";
  }
  if (loaderVersions.isError) {
    return "error";
  }
  if (loaderVersions.data === undefined) {
    return "checking";
  }
  return loaderVersions.data.length > 0 ? "ready" : "empty";
}
