import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { saveInstance } from "@/api/commands";
import {
  fromEntry,
  type InstanceDraft,
  optionalSize,
  toInstanceSettings,
} from "@/features/instance-settings/model/draft";
import { friendlyError, messageOf } from "@/lib/errors";
import type { InstanceEntry } from "@/lib/generated/InstanceEntry";

const SAVE_DELAY_MS = 400;

export function useInstanceDraft(entry: InstanceEntry | null, open: boolean) {
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState<InstanceDraft | null>(null);
  const draftRef = useRef<InstanceDraft | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const folderRef = useRef<string | null>(null);
  const persistRef = useRef<(current: InstanceDraft) => void>(() => {});

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

  persistRef.current = (current) => {
    const folder = folderRef.current;
    if (!folder) {
      return;
    }
    const width = optionalSize(current.width);
    const height = optionalSize(current.height);
    if (width === "invalid" || height === "invalid") {
      toast.error("window size must be at least 1");
      return;
    }
    save.mutate({
      folder,
      settings: toInstanceSettings(current, width, height),
    });
  };

  useEffect(() => {
    if (!open || !entry) {
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
  }, [entry, open]);

  useEffect(() => {
    return () => {
      if (timerRef.current !== null) {
        clearTimeout(timerRef.current);
      }
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

  return { draft, update, flush };
}
