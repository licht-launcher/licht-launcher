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

type Target = {
  folder: string;
  /** Asked by the instance's settings window, which shows the error itself. */
  fromSettingsWindow?: boolean;
};

function reportError(error: unknown, target: Target) {
  if (!target.fromSettingsWindow) {
    toast.error(friendlyError(messageOf(error)));
  }
}

export function useInstanceMutations({
  beginInstall,
  endInstall,
  clearActionFor,
}: {
  beginInstall: (folder: string, versionId: string | null) => void;
  endInstall: (folder: string, versionId: string | null) => void;
  /** Closes the rename or delete dialog when it is about `folder`. */
  clearActionFor: (folder: string) => void;
}) {
  const queryClient = useQueryClient();

  const refreshInstances = async (target: Target) => {
    clearActionFor(target.folder);
    await queryClient.invalidateQueries({ queryKey: ["instances"] });
  };

  const rename = useMutation({
    mutationFn: (input: Target & { name: string }) =>
      renameInstance(input.folder, input.name),
    onSuccess: (_entry, input) => refreshInstances(input),
    onError: reportError,
  });

  const duplicate = useMutation({
    mutationFn: (input: Target & { name: string }) =>
      duplicateInstance(input.folder, input.name),
    onSuccess: (_entry, input) => refreshInstances(input),
    onError: reportError,
  });

  const remove = useMutation({
    mutationFn: (input: Target) => deleteInstance(input.folder),
    onSuccess: (_result, input) => refreshInstances(input),
    onError: reportError,
  });

  const openFolder = useMutation({
    mutationFn: (input: Target) => openInstanceFolder(input.folder),
    onError: reportError,
  });

  // The new launch version is known only once the change job installs it.
  const changeBase = useMutation({
    mutationFn: async (input: Target & { base: InstanceBaseInput }) => {
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
    onError: reportError,
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
