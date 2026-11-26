import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { getSettings, runtimeJava, saveSettings } from "@/api/commands";
import { useToastError } from "@/features/settings/hooks/use-toast-error";
import {
  dismissRestartToast,
  promptRestart,
} from "@/features/settings/lib/restart-toast";
import {
  type Draft,
  fromSnapshot,
  optionalSize,
  toSettings,
} from "@/features/settings/model/draft";
import { DOWNLOAD_SPEED_ERROR, parseDownloadLimit } from "@/lib/download-speed";
import { friendlyError, messageOf } from "@/lib/errors";
import type { Settings } from "@/lib/generated/Settings";

const SAVE_DELAY_MS = 400;
// The value this launcher session started with. It outlives the Settings page,
// so leaving and coming back still compares against what is actually applied.
const APPLIED_HARDWARE_ACCELERATION_KEY = ["applied-hardware-acceleration"];

export function useSettingsDraft() {
  const queryClient = useQueryClient();
  const settings = useQuery({
    queryKey: ["settings"],
    queryFn: getSettings,
    retry: false,
  });
  const appliedHardwareAcceleration = useQuery({
    queryKey: APPLIED_HARDWARE_ACCELERATION_KEY,
    queryFn: async () => {
      const snapshot = await queryClient.ensureQueryData({
        queryKey: ["settings"],
        queryFn: getSettings,
      });
      return snapshot.settings.hardwareAcceleration;
    },
    staleTime: Number.POSITIVE_INFINITY,
    gcTime: Number.POSITIVE_INFINITY,
    retry: false,
  });
  const runtime = useQuery({
    queryKey: ["runtime-java"],
    queryFn: runtimeJava,
    retry: false,
  });

  const [draft, setDraft] = useState<Draft | null>(null);
  const draftRef = useRef<Draft | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const schemaRef = useRef(1);
  const persistRef = useRef<(current: Draft) => void>(() => {});

  const save = useMutation({
    mutationFn: saveSettings,
    onSuccess: async (saved: Settings) => {
      await queryClient.invalidateQueries({ queryKey: ["settings"] });
      const applied = queryClient.getQueryData<boolean>(
        APPLIED_HARDWARE_ACCELERATION_KEY,
      );
      if (applied !== undefined && saved.hardwareAcceleration !== applied) {
        promptRestart("Apply the hardware acceleration change now?");
      } else {
        dismissRestartToast();
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
      const next = fromSnapshot(settings.data, runtime.data ?? null);
      draftRef.current = next;
      setDraft(next);
    }
  }, [settings.data, runtime.isFetched, runtime.data, draft]);

  useToastError(settings.error);
  useToastError(runtime.error);

  // Leaving Settings saves the edit still waiting for its delay.
  useEffect(() => {
    return () => {
      if (timerRef.current !== null && draftRef.current !== null) {
        clearTimeout(timerRef.current);
        timerRef.current = null;
        persistRef.current(draftRef.current);
      }
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
      patch.hardwareAcceleration !== undefined &&
      appliedHardwareAcceleration.data !== undefined &&
      next.hardwareAcceleration === appliedHardwareAcceleration.data
    ) {
      dismissRestartToast();
    }

    if (timerRef.current !== null) {
      clearTimeout(timerRef.current);
    }
    timerRef.current = setTimeout(() => {
      timerRef.current = null;
      persistRef.current(next);
    }, SAVE_DELAY_MS);
  }

  return {
    draft,
    snapshot: settings.data ?? null,
    update,
  };
}
