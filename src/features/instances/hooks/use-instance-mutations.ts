import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import {
  deleteInstance,
  duplicateInstance,
  installVersion,
  openInstanceFolder,
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
      if (!input.installed) {
        beginInstall(input.folder, input.versionId);
        try {
          await installVersion(input.versionId);
        } finally {
          endInstall(input.folder, input.versionId);
        }
      }
      return setInstanceVersion(input.folder, input.versionId);
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["instances"] });
      await queryClient.invalidateQueries({ queryKey: ["versions"] });
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
