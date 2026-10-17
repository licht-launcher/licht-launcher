import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { VERSIONS_QUERY_KEY } from "@/features/instances/lib/versions-query";
import {
  deleteInstance,
  duplicateInstance,
  openInstanceFolder,
  prepareInstanceDownload,
  renameInstance,
  setInstanceVersion,
} from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";

export function useInstanceMutations({
  beginInstall,
  endInstall,
  setConfiguringFolder,
  clearAction,
}: {
  beginInstall: (folder: string, versionId: string) => void;
  endInstall: (folder: string, versionId: string) => void;
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

  const changeVersion = useMutation({
    mutationFn: async (input: {
      folder: string;
      versionId: string;
      installed: boolean;
    }) => {
      const entry = await setInstanceVersion(input.folder, input.versionId);
      if (!input.installed) {
        beginInstall(input.folder, input.versionId);
        try {
          const job = await prepareInstanceDownload(input.folder);
          if (job.status === "failed")
            throw new Error(job.error ?? "Download failed");
        } finally {
          endInstall(input.folder, input.versionId);
        }
      }
      return entry;
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["instances"] });
      await queryClient.invalidateQueries({ queryKey: VERSIONS_QUERY_KEY });
    },
    onError: (error: unknown) => {
      toast.error(friendlyError(messageOf(error)));
    },
  });

  const busy =
    rename.isPending ||
    duplicate.isPending ||
    remove.isPending ||
    openFolder.isPending ||
    changeVersion.isPending;

  return {
    busy,
    rename,
    duplicate,
    remove,
    openFolder,
    changeVersion,
  };
}
