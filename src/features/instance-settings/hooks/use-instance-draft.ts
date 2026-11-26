import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { saveInstance } from "@/api/commands";
import { onLauncherClosed, onWindowCloseRequested } from "@/api/window-close";
import {
  fromEntry,
  type InstanceDraft,
  optionalSize,
  toInstanceSettings,
} from "@/features/instance-settings/model/draft";
import { reportInterfaceError } from "@/lib/error-reporting";
import { friendlyError, messageOf } from "@/lib/errors";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

const SAVE_DELAY_MS = 400;

export function useInstanceDraft(entry: InstanceEntry | null) {
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState<InstanceDraft | null>(null);
  const draftRef = useRef<InstanceDraft | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const folderRef = useRef<string | null>(null);
  const persistRef = useRef<(current: InstanceDraft) => Promise<boolean>>(
    async () => true,
  );
  const flushRef = useRef<() => Promise<boolean>>(async () => true);
  const savingRef = useRef<Promise<boolean> | null>(null);

  const save = useMutation({
    mutationFn: (input: {
      folder: string;
      settings: ReturnType<typeof toInstanceSettings>;
    }) => saveInstance(input.folder, input.settings),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["instances"] });
    },
    onError: (error: unknown) => {
      toast.error(friendlyError(messageOf(error)));
    },
  });

  async function saveDraft(current: InstanceDraft): Promise<boolean> {
    const folder = folderRef.current;
    if (!folder) {
      return true;
    }
    const width = optionalSize(current.width);
    const height = optionalSize(current.height);
    if (width === "invalid" || height === "invalid") {
      toast.error("window size must be at least 1");
      return false;
    }
    try {
      await save.mutateAsync({
        folder,
        settings: toInstanceSettings(current, width, height),
      });
      return true;
    } catch {
      // The mutation's onError has already shown why.
      return false;
    }
  }

  persistRef.current = (current) => {
    const saving = saveDraft(current);
    savingRef.current = saving;
    void saving.then(() => {
      if (savingRef.current === saving) savingRef.current = null;
    });
    return saving;
  };

  flushRef.current = () => {
    if (timerRef.current === null || draftRef.current === null) {
      return savingRef.current ?? Promise.resolve(true);
    }
    clearTimeout(timerRef.current);
    timerRef.current = null;
    return persistRef.current(draftRef.current);
  };

  useEffect(() => {
    if (!entry) {
      draftRef.current = null;
      folderRef.current = null;
      setDraft(null);
      return;
    }
    if (folderRef.current !== entry.folder || draftRef.current === null) {
      const next = fromEntry(entry);
      folderRef.current = entry.folder;
      draftRef.current = next;
      setDraft(next);
    }
  }, [entry]);

  // Closing a window does not unmount React, so the cleanup below would not save.
  useEffect(() => {
    let active = true;
    const stops: (() => void)[] = [];
    const keep = (subscribing: Promise<() => void>) => {
      subscribing
        .then((stop) => {
          if (active) stops.push(stop);
          else stop();
        })
        .catch((error: unknown) => reportInterfaceError("promise", error));
    };
    keep(
      onWindowCloseRequested(async (request) => {
        if (!(await flushRef.current())) request.preventDefault();
      }),
    );
    keep(
      onLauncherClosed(async () => {
        if (!(await flushRef.current())) {
          reportInterfaceError(
            "promise",
            new Error(
              "The last instance settings edit could not be saved before the launcher closed",
            ),
          );
        }
      }),
    );
    return () => {
      active = false;
      for (const stop of stops) stop();
      void flushRef.current();
    };
  }, []);

  function update(patch: Partial<InstanceDraft>) {
    const current = draftRef.current;
    if (!current) {
      return;
    }
    const next = { ...current, ...patch };
    draftRef.current = next;
    setDraft(next);
    if (timerRef.current !== null) {
      clearTimeout(timerRef.current);
    }
    timerRef.current = setTimeout(() => {
      timerRef.current = null;
      void persistRef.current(next);
    }, SAVE_DELAY_MS);
  }

  return { draft, update };
}
