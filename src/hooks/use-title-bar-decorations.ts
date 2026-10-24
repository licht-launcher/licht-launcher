import { useQuery } from "@tanstack/react-query";

import { useToastError } from "@/features/settings/hooks/use-toast-error";
import {
  activeMicrosoftAccount,
  getPlayerHead,
  getSettings,
} from "@/lib/commands";

/** Public account display data shared by the launcher chrome. */
export function useTitleBarDecorations() {
  const settings = useQuery({
    queryKey: ["settings"],
    queryFn: getSettings,
    retry: false,
  });
  const active = useQuery({
    queryKey: ["microsoft-account-active"],
    queryFn: activeMicrosoftAccount,
    retry: false,
  });
  useToastError(active.error);
  const skinUrl = active.data?.skinUrl ?? null;
  const head = useQuery({
    queryKey: ["player-head", active.data?.uuid ?? null, skinUrl],
    queryFn: () => getPlayerHead(skinUrl),
    enabled: active.isFetched,
    staleTime: Number.POSITIVE_INFINITY,
    retry: false,
  });
  useToastError(head.error);

  const offlineUsername =
    (settings.data?.settings.username ?? "Steve").trim() || "Steve";
  const username = active.data?.name.trim() || offlineUsername;

  return {
    username,
    offlineUsername,
    activeUuid: active.data?.uuid ?? "",
    headPath: head.data,
  };
}
