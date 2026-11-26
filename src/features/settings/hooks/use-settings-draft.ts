import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import {
  appliedHardwareAcceleration as getAppliedHardwareAcceleration,
  getSettings,
  runtimeJava,
  saveSettings,
} from "@/api/commands";
import { onWindowCloseRequested } from "@/api/window-close";
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
import { reportInterfaceError } from "@/lib/error-reporting";
import { friendlyError, messageOf } from "@/lib/errors";
import type { Settings } from "@/lib/generated/Settings";

const SAVE_DELAY_MS = 400;
// What the running launcher uses; it only changes when the launcher restarts.
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
    queryFn: getAppliedHardwareAcceleration,
    staleTime: Number.POSITIVE_INFINITY,
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
  // Resolves to whether the draft was saved; failures were already shown.
  const persistRef = useRef<(current: Draft) => Promise<boolean>>(
    async () => true,
  );
  const flushRef = useRef<() => Promise<boolean>>(async () => true);

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

  persistRef.current = async (current) => {
    const width = optionalSize(current.width);
    const height = optionalSize(current.height);
    if (width === "invalid" || height === "invalid") {
      toast.error("window size must be at least 1");
      return false;
    }
    const speed = parseDownloadLimit(current.downloadSpeedLimitKbs);
    if (speed === "invalid") {
      toast.error(DOWNLOAD_SPEED_ERROR);
      return false;
    }
    try {
      await save.mutateAsync(
        toSettings(current, schemaRef.current, width, height),
      );
      return true;
    } catch {
      // The mutation's onError has already shown why.
      return false;
    }
  };

  /** Saves the edit still waiting for its delay; resolves to whether it saved. */
  flushRef.current = () => {
    if (timerRef.current === null || draftRef.current === null) {
      return Promise.resolve(true);
    }
    clearTimeout(timerRef.current);
    timerRef.current = null;
    return persistRef.current(draftRef.current);
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

  // Closing the window ends the page without unmounting it, so the window
  // waits for the last edit, and stays open to show why it could not be saved.
  useEffect(() => {
    let active = true;
    let stop: (() => void) | undefined;
    onWindowCloseRequested(async (request) => {
      if (!(await flushRef.current())) request.preventDefault();
    })
      .then((unsubscribe) => {
        if (active) stop = unsubscribe;
        else unsubscribe();
      })
      .catch((error: unknown) => reportInterfaceError("promise", error));
    return () => {
      active = false;
      stop?.();
      // Unmounting without closing the window also keeps the last edit.
      void flushRef.current();
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
      void persistRef.current(next);
    }, SAVE_DELAY_MS);
  }

  return {
    draft,
    snapshot: settings.data ?? null,
    update,
  };
}
