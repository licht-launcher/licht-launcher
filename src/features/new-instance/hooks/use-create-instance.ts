import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { toast } from "sonner";

import { VERSIONS_QUERY_KEY } from "@/features/instances/lib/versions-query";
import {
  type ScratchState,
  uploadInstanceName,
} from "@/features/new-instance/model/create-state";
import {
  browseModpackFile,
  createInstanceFromModpackFile,
  createInstanceWithPlatform,
} from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

/**
 * The two ways to create an instance from the popup. Both end the same way:
 * refresh the lists, open the new instance in the library and close the popup.
 */
export function useCreateInstance({
  scratch,
  onCreated,
}: {
  scratch: ScratchState;
  onCreated: () => void;
}) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  async function finish(entry: InstanceEntry) {
    await queryClient.invalidateQueries({ queryKey: ["instances"] });
    await queryClient.invalidateQueries({ queryKey: VERSIONS_QUERY_KEY });
    await navigate({ to: "/", search: { prepare: entry.folder } });
    onCreated();
  }

  function reportFailure(error: unknown) {
    toast.error(friendlyError(messageOf(error)));
  }

  const fromScratch = useMutation({
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
    onError: reportFailure,
  });

  const fromUpload = useMutation({
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
    onError: reportFailure,
  });

  return {
    fromScratch,
    fromUpload,
    busy: fromScratch.isPending || fromUpload.isPending,
  };
}
