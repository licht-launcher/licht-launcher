import { useQuery } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useLayoutEffect, useState } from "react";
import { toast } from "sonner";
import { useToastError } from "@/features/settings/hooks/use-toast-error";
import { getSettings } from "@/lib/commands";
import type { LauncherTheme } from "@/lib/generated/LauncherTheme";
import { initialTheme, isLauncherTheme, resolveTheme } from "@/lib/theme";

export function useLauncherTheme(startupWindow: boolean) {
  const [theme, setTheme] = useState<LauncherTheme>(initialTheme);
  const [resolved, setResolved] = useState(() =>
    resolveTheme(
      theme,
      window.matchMedia("(prefers-color-scheme: dark)").matches,
    ),
  );
  const settings = useQuery({
    queryKey: ["settings"],
    queryFn: getSettings,
    enabled: !startupWindow,
    staleTime: Number.POSITIVE_INFINITY,
    retry: false,
  });
  const savedTheme = settings.data?.settings.theme;
  useEffect(() => {
    if (savedTheme !== undefined) setTheme(savedTheme);
  }, [savedTheme]);
  useToastError(settings.error);

  useLayoutEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const next = resolveTheme(theme, media.matches);
      document.documentElement.classList.toggle("dark", next === "dark");
      setResolved(next);
    };
    apply();
    if (theme === "system") media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [theme]);

  useEffect(() => {
    let active = true;
    let unsubscribe: (() => void) | undefined;
    void listen<unknown>("launcher-theme-changed", ({ payload }) => {
      if (!active) return;
      if (isLauncherTheme(payload)) setTheme(payload);
      else toast.error("Invalid launcher theme notification.");
    })
      .then((stop) => {
        if (active) unsubscribe = stop;
        else stop();
      })
      .catch(() => {
        if (active) toast.error("Could not listen for launcher theme changes.");
      });
    return () => {
      active = false;
      unsubscribe?.();
    };
  }, []);
  return resolved;
}
