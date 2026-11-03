import { useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { toast } from "sonner";

import { VERSIONS_QUERY_KEY } from "@/features/instances/lib/versions-query";
import { friendlyError, messageOf } from "@/lib/errors";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

/**
 * What every way of creating an instance does at the end: refresh the lists
 * and open the new instance in the library. Failures show a toast.
 */
export function useFinishCreation() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  async function finish(entry: InstanceEntry) {
    await queryClient.invalidateQueries({ queryKey: ["instances"] });
    await queryClient.invalidateQueries({ queryKey: VERSIONS_QUERY_KEY });
    await navigate({ to: "/", search: { prepare: entry.folder } });
  }

  function reportFailure(error: unknown) {
    toast.error(friendlyError(messageOf(error)));
  }

  return { finish, reportFailure };
}
