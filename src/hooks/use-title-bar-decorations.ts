import { useQuery } from "@tanstack/react-query";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect } from "react";

import { getSettings } from "@/lib/commands";

/** Syncs native window decorations with the disable-custom-title-bar setting. */
export function useTitleBarDecorations() {
  const settings = useQuery({
    queryKey: ["settings"],
    queryFn: getSettings,
    retry: false,
  });

  const disableCustomTitleBar =
    settings.data?.settings.disableCustomTitleBar ?? false;
  const username =
    (settings.data?.settings.username ?? "Steve").trim() || "Steve";

  useEffect(() => {
    void getCurrentWindow().setDecorations(disableCustomTitleBar);
  }, [disableCustomTitleBar]);

  return { disableCustomTitleBar, username };
}
