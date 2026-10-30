import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import { useToastError } from "@/features/settings/hooks/use-toast-error";
import {
  dismissTitleBarRestartToast,
  promptTitleBarRestart,
} from "@/features/settings/lib/title-bar-restart-toast";
import {
  type Draft,
  fromSnapshot,
  optionalSize,
  toSettings,
} from "@/features/settings/model/draft";
import { getSettings, runtimeJava, saveSettings } from "@/lib/commands";
import { DOWNLOAD_SPEED_ERROR, parseDownloadLimit } from "@/lib/download-speed";
import { friendlyError, messageOf } from "@/lib/errors";
import type { Settings } from "@/lib/generated/Settings";

const SAVE_DELAY_MS = 400;

export function useSettingsDraft(open: boolean) {
  const queryClient = useQueryClient();
  const settings = useQuery({
    queryKey: ["settings"],
    queryFn: getSettings,
    enabled: open,
    retry: false,
  });
  const runtime = useQuery({
    queryKey: ["runtime-java"],
    queryFn: runtimeJava,
    enabled: open,
    retry: false,
  });

  const [draft, setDraft] = useState<Draft | null>(null);
  const draftRef = useRef<Draft | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const schemaRef = useRef(1);
  const persistRef = useRef<(current: Draft) => void>(() => {});
  const appliedDisableCustomTitleBar = useRef<boolean | null>(null);
  const appliedDisableTransparency = useRef<boolean | null>(null);
  const appliedHardwareAcceleration = useRef<boolean | null>(null);

  const save = useMutation({
    mutationFn: saveSettings,
    onSuccess: async (saved: Settings) => {
      await queryClient.invalidateQueries({ queryKey: ["settings"] });
      if (
        appliedHardwareAcceleration.current !== null &&
        saved.hardwareAcceleration !== appliedHardwareAcceleration.current
      ) {
        promptTitleBarRestart("Apply the hardware acceleration change now?");
      } else if (
        appliedDisableTransparency.current !== null &&
        saved.disableTransparency !== appliedDisableTransparency.current
      ) {
        promptTitleBarRestart("Apply the appearance changes now?");
      } else if (
        appliedDisableCustomTitleBar.current !== null &&
        saved.disableCustomTitleBar !== appliedDisableCustomTitleBar.current
      ) {
        promptTitleBarRestart();
      } else {
        dismissTitleBarRestartToast();
      }
    },
    onError: (error: unknown) => {
      toast.error(friendlyError(messageOf(error)));
    },
  });

  persistRef.current = (current) => {
    const width = optionalSize(current.width);
    const height = optionalSize(current.height);
    if (width === "invalid" || height === "invalid") {
      toast.error("window size must be at least 1");
      return;
    }
    const speed = parseDownloadLimit(current.downloadSpeedLimitKbs);
    if (speed === "invalid") {
      toast.error(DOWNLOAD_SPEED_ERROR);
      return;
    }
    save.mutate(toSettings(current, schemaRef.current, width, height));
  };

  useEffect(() => {
    if (settings.data && runtime.isFetched && draft === null) {
      schemaRef.current = settings.data.settings.schema;
      appliedDisableCustomTitleBar.current =
        settings.data.settings.disableCustomTitleBar;
      appliedDisableTransparency.current =
        settings.data.settings.disableTransparency;
      appliedHardwareAcceleration.current =
        settings.data.settings.hardwareAcceleration;
      const next = fromSnapshot(settings.data, runtime.data ?? null);
      draftRef.current = next;
      setDraft(next);
    }
  }, [settings.data, runtime.isFetched, runtime.data, draft]);

  useToastError(settings.error);
  useToastError(runtime.error);

  useEffect(() => {
    return () => {
      if (timerRef.current !== null) {
        clearTimeout(timerRef.current);
      }
      dismissTitleBarRestartToast();
    };
  }, []);

  function update(patch: Partial<Draft>) {
    const current = draftRef.current;
    if (!current) {
      return;
    }
    const next = { ...current, ...patch };
    draftRef.current = next;
    setDraft(next);

    if (
      (patch.disableCustomTitleBar !== undefined ||
        patch.disableTransparency !== undefined ||
        patch.hardwareAcceleration !== undefined) &&
      appliedDisableCustomTitleBar.current !== null &&
      appliedDisableTransparency.current !== null &&
      appliedHardwareAcceleration.current !== null &&
      next.disableCustomTitleBar === appliedDisableCustomTitleBar.current &&
      next.disableTransparency === appliedDisableTransparency.current &&
      next.hardwareAcceleration === appliedHardwareAcceleration.current
    ) {
      dismissTitleBarRestartToast();
    }

    if (timerRef.current !== null) {
      clearTimeout(timerRef.current);
    }
    timerRef.current = setTimeout(() => {
      timerRef.current = null;
      persistRef.current(next);
    }, SAVE_DELAY_MS);
  }

  function flush() {
    if (timerRef.current === null || draftRef.current === null) {
      return;
    }
    clearTimeout(timerRef.current);
    timerRef.current = null;
    persistRef.current(draftRef.current);
  }

  return {
    draft,
    snapshot: settings.data ?? null,
    update,
    flush,
  };
}
