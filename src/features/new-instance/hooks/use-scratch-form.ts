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
import { listLoaderVersions } from "@/lib/commands";

/**
 * The from-scratch form: its state, the version lists it depends on, and the
 * defaults that fill in once those lists load. `enabled` keeps the network
 * quiet until the form is actually on screen.
 */
export function useScratchForm(enabled: boolean) {
  const [state, setState] = useState<ScratchState>(initialScratchState);

  const update = useCallback((patch: Partial<ScratchState>) => {
    setState((current) => ({ ...current, ...patch }));
  }, []);

  const versions = useQuery({ ...versionsQueryOptions, enabled });

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
      enabled &&
      platformNeedsLoader(state.platform) &&
      state.gameVersion !== "",
    retry: false,
  });

  useEffect(() => {
    if (!enabled || state.gameVersion !== "") {
      return;
    }
    const newest = createVersionChoices(
      versions.data ?? [],
      state.includeSnapshots,
    )[0];
    if (newest !== undefined) {
      setState((current) => ({ ...current, gameVersion: newest.id }));
    }
  }, [enabled, state.gameVersion, state.includeSnapshots, versions.data]);

  useEffect(() => {
    if (!enabled || !platformNeedsLoader(state.platform)) {
      return;
    }
    const newest = loaderVersions.data?.[0];
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
    enabled,
    state.platform,
    state.loaderChannel,
    state.loaderVersion,
    loaderVersions.data,
  ]);

  return {
    state,
    update,
    versions,
    loaderVersions,
    loaderState: loaderStateOf(state, loaderVersions),
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
