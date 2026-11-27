import { useQuery, useQueryClient } from "@tanstack/react-query";
import { refreshLinkedModpackVersion } from "@/api/commands";
import type { ModpackOrigin } from "@/features/instances/lib/modpack-content";
import type { LinkedModpack } from "@/lib/generated/LinkedModpack";

/**
 * Store packs installed before version names were tidied saved the long form
 * ("Homestead 2.0.1"). Once per instance and app session, the store is asked
 * again; the saved name stays on screen meanwhile and if the store fails.
 */
export function useLinkedVersionRefresh(
  folder: string | null,
  origin: ModpackOrigin | null,
  linked: LinkedModpack | null,
) {
  const client = useQueryClient();
  useQuery({
    queryKey: ["linked-version-refresh", folder],
    queryFn: async () => {
      if (folder === null) return false;
      const entry = await refreshLinkedModpackVersion(folder);
      if (entry !== null) {
        await client.invalidateQueries({ queryKey: ["instances"] });
      }
      return entry !== null;
    },
    enabled: folder !== null && origin?.type === "modpack" && linked !== null,
    staleTime: Number.POSITIVE_INFINITY,
    gcTime: Number.POSITIVE_INFINITY,
    retry: false,
  });
}
