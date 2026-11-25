import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo } from "react";
import { toast } from "sonner";
import {
  removeInstanceContent,
  setInstanceContentEnabled,
} from "@/api/commands";
import { instanceContentQuery } from "@/features/instances/lib/content-query";
import { friendlyError, messageOf } from "@/lib/errors";
import type { InstanceContentEntry } from "@/lib/generated/InstanceContentEntry";

/**
 * An instance's content, split into what its modpack installed and what the
 * user added, with the actions that change it.
 */
export function useInstanceContent(folder: string | null) {
  const queryClient = useQueryClient();
  const content = useQuery({
    ...instanceContentQuery(queryClient, folder ?? ""),
    enabled: folder !== null,
  });
  const reload = () =>
    queryClient.invalidateQueries({ queryKey: ["instance-content", folder] });
  const update = useMutation({
    mutationFn: (input: { entry: InstanceContentEntry; enabled: boolean }) =>
      setInstanceContentEnabled(
        folder ?? "",
        input.entry.kind,
        input.entry.name,
        input.enabled,
      ),
    onSuccess: async () => {
      await reload();
      toast.success("Content status updated.");
    },
    onError: (error) => toast.error(friendlyError(messageOf(error))),
  });
  const remove = useMutation({
    mutationFn: (entry: InstanceContentEntry) =>
      removeInstanceContent(
        folder ?? "",
        entry.kind,
        entry.name,
        entry.enabled,
      ),
    onSuccess: async () => {
      await reload();
      toast.success("Content removed.");
    },
    onError: (error) => toast.error(friendlyError(messageOf(error))),
  });
  const modpackEntries = useMemo(
    () => (content.data ?? []).filter((entry) => entry.fromModpack),
    [content.data],
  );
  const extraEntries = useMemo(
    () => (content.data ?? []).filter((entry) => !entry.fromModpack),
    [content.data],
  );
  return {
    content,
    modpackEntries,
    extraEntries,
    /** True while a toggle or removal runs, so the controls wait for it. */
    busy: update.isPending || remove.isPending,
    toggle: (entry: InstanceContentEntry, enabled: boolean) =>
      update.mutate({ entry, enabled }),
    remove,
    refreshing: content.isFetching,
    /** Lists the folders again and rereads every file's metadata. */
    refresh: () => {
      if (folder === null) return;
      void queryClient
        .fetchQuery({
          ...instanceContentQuery(queryClient, folder, true),
          staleTime: 0,
        })
        .catch((error: unknown) =>
          toast.error(friendlyError(messageOf(error))),
        );
    },
  };
}
