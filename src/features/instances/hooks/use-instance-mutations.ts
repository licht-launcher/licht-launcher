import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  changeInstanceBase,
  deleteInstance,
  duplicateInstance,
  type InstanceBaseInput,
  openInstanceFolder,
  renameInstance,
} from "@/api/commands";
import { VERSIONS_QUERY_KEY } from "@/features/instances/lib/versions-query";
import { friendlyError, messageOf } from "@/lib/errors";

export function useInstanceMutations({
  beginInstall,
  endInstall,
  setConfiguringFolder,
  clearAction,
}: {
  beginInstall: (folder: string, versionId: string | null) => void;
  endInstall: (folder: string, versionId: string | null) => void;
  setConfiguringFolder: (folder: string | null) => void;
  clearAction: () => void;
}) {
  const queryClient = useQueryClient();

  const refreshInstances = async () => {
    clearAction();
    await queryClient.invalidateQueries({ queryKey: ["instances"] });
  };

  const rename = useMutation({
    mutationFn: (input: { folder: string; name: string }) =>
      renameInstance(input.folder, input.name),
    onSuccess: refreshInstances,
    onError: (error: unknown) => {
      toast.error(friendlyError(messageOf(error)));
    },
  });

  const duplicate = useMutation({
    mutationFn: (input: { folder: string; name: string }) =>
      duplicateInstance(input.folder, input.name),
    onSuccess: refreshInstances,
    onError: (error: unknown) => {
      toast.error(friendlyError(messageOf(error)));
    },
  });

  const remove = useMutation({
    mutationFn: deleteInstance,
    onSuccess: async () => {
      setConfiguringFolder(null);
      await refreshInstances();
    },
    onError: (error: unknown) => {
      toast.error(friendlyError(messageOf(error)));
    },
  });

  const openFolder = useMutation({
    mutationFn: openInstanceFolder,
    onError: (error: unknown) => {
      toast.error(friendlyError(messageOf(error)));
    },
  });

  // The new launch version is known only once the change job installs it.
  const changeBase = useMutation({
    mutationFn: async (input: { folder: string; base: InstanceBaseInput }) => {
      beginInstall(input.folder, null);
      try {
        const job = await changeInstanceBase(input.folder, input.base);
        if (job.status === "failed")
          throw new Error(job.error ?? "Download failed");
        return job;
      } finally {
        endInstall(input.folder, null);
      }
    },
    onSuccess: async (job) => {
      if (job.status === "completed") toast.success("Instance base updated.");
      await queryClient.invalidateQueries({ queryKey: VERSIONS_QUERY_KEY });
    },
    // Also after a failure: the instance waits to retry, or a cancel put the old base back.
    onSettled: () => queryClient.invalidateQueries({ queryKey: ["instances"] }),
    onError: (error: unknown) => {
      toast.error(friendlyError(messageOf(error)));
    },
  });

  const busy =
    rename.isPending ||
    duplicate.isPending ||
    remove.isPending ||
    openFolder.isPending ||
    changeBase.isPending;

  return {
    busy,
    rename,
    duplicate,
    remove,
    openFolder,
    changeBase,
  };
}
