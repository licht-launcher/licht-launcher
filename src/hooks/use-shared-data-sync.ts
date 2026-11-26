import { type QueryKey, useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { toast } from "sonner";
import { reportInterfaceError } from "@/lib/error-reporting";

/** Backend notifications and the cached data each one makes stale. */
const SHARED_DATA: Record<string, QueryKey[]> = {
  "settings-changed": [["settings"]],
  "accounts-changed": [["microsoft-accounts"], ["microsoft-account-active"]],
};

/**
 * Each window keeps its own query cache, so a change saved in one window is
 * refetched by the others when the backend announces it.
 */
export function useSharedDataSync() {
  const client = useQueryClient();
  useEffect(() => {
    let active = true;
    const stops: (() => void)[] = [];
    for (const [event, keys] of Object.entries(SHARED_DATA)) {
      listen(event, () => {
        for (const queryKey of keys) {
          void client.invalidateQueries({ queryKey });
        }
      })
        .then((stop) => {
          if (active) stops.push(stop);
          else stop();
        })
        .catch((error: unknown) => {
          reportInterfaceError("promise", error);
          if (active) toast.error("Could not listen for launcher changes.");
        });
    }
    return () => {
      active = false;
      for (const stop of stops) stop();
    };
  }, [client]);
}
