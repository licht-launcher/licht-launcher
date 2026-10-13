import { useQuery } from "@tanstack/react-query";

import { getSettings } from "@/lib/commands";

/** Username (and related settings) for the custom title strip. */
export function useTitleBarDecorations() {
  const settings = useQuery({
    queryKey: ["settings"],
    queryFn: getSettings,
    retry: false,
  });

  const username =
    (settings.data?.settings.username ?? "Steve").trim() || "Steve";

  return { username };
}
