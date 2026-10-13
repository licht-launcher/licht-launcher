import { useQuery } from "@tanstack/react-query";

import { customTitleBarActive } from "@/lib/commands";

/**
 * Whether this process was started with Licht's custom title bar.
 * Changing the setting requires a restart; the value is fixed for the session.
 */
export function useCustomTitleBar() {
  return useQuery({
    queryKey: ["custom-title-bar"],
    queryFn: customTitleBarActive,
    staleTime: Number.POSITIVE_INFINITY,
    retry: false,
  });
}
