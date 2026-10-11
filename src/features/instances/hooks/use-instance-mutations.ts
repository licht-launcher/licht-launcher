import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import {
  createInstance,
  deleteInstance,
  duplicateInstance,
  installVersion,
  openInstanceFolder,
  renameInstance,
  setInstanceVersion,
} from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";
import type { DownloadProgress } from "@/lib/generated/DownloadProgress";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

export function useInstanceMutations({
  installingFolder,
  setProgress,
  setSelectedFolder,
  setConfiguringFolder,
  clearAction,
}: {
  installingFolder: string | null;
  setProgress: (progress: DownloadProgress | null) => void;
  setSelectedFolder: (folder: string | null) => void;
  setConfiguringFolder: (folder: string | null) => void;
  clearAction: () => void;
}) {
  const queryClient = useQueryClient();

  const refreshInstances = async () => {
    clearAction();
    await queryClient.invalidateQueries({ queryKey: ["instances"] });
  };

  const create = useMutation({
    mutationFn: (input: { name: string; versionId: string }) =>
      createInstance(input.name, input.versionId),
    onSuccess: async (entry: InstanceEntry) => {
      setSelectedFolder(entry.folder);
      await refreshInstances();
    },
    onError: (error: unknown) => {
      toast.error(friendlyError(messageOf(error)));
    },
  });

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
        await installVersion(input.versionId);
      }
      return setInstanceVersion(input.folder, input.versionId);
    },
    onSuccess: async () => {
      setProgress(null);
      await queryClient.invalidateQueries({ queryKey: ["instances"] });
      await queryClient.invalidateQueries({ queryKey: ["versions"] });
    },
    onError: (error: unknown) => {
      setProgress(null);
      toast.error(friendlyError(messageOf(error)));
    },
  });

  const busy =
    installingFolder !== null ||
    create.isPending ||
    rename.isPending ||
    duplicate.isPending ||
    remove.isPending ||
    openFolder.isPending ||
    changeVersion.isPending;

  return {
    busy,
    create,
    rename,
    duplicate,
    remove,
    openFolder,
    changeVersion,
  };
}
