import { useQuery } from "@tanstack/react-query";
import { useCallback, useEffect, useState } from "react";

import { versionsQueryOptions } from "@/features/instances/lib/versions-query";
import { createVersionChoices } from "@/features/instances/model/create-versions";
import {
  initialScratchState,
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
      listLoaderVersions(state.platform, state.gameVersion, state.loaderChannel),
    enabled:
      enabled && platformNeedsLoader(state.platform) && state.gameVersion !== "",
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
    if (
      !enabled ||
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
    enabled,
    state.platform,
    state.loaderChannel,
    state.loaderVersion,
    loaderVersions.data,
  ]);

  return { state, update, versions, loaderVersions };
}
