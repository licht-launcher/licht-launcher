import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";

export const STORAGE_LOADING_PREVIEW_EVENT = "licht:preview-storage-loading";
export const STORAGE_LOADING_PREVIEW_QUERY_KEY = [
  "dev",
  "storage-loading-preview",
] as const;

export function requestStorageLoadingPreview() {
  if (import.meta.env.DEV) {
    window.dispatchEvent(new Event(STORAGE_LOADING_PREVIEW_EVENT));
  }
}

export function useStorageLoadingPreview(): boolean {
  const client = useQueryClient();
  const query = useQuery({
    queryKey: STORAGE_LOADING_PREVIEW_QUERY_KEY,
    queryFn: (): number | null => null,
    initialData: null,
    enabled: false,
  });
  const startedAt = import.meta.env.DEV ? query.data : null;
  useEffect(() => {
    if (startedAt === null) return;
    const timer = window.setTimeout(
      () => {
        client.setQueryData(STORAGE_LOADING_PREVIEW_QUERY_KEY, null);
      },
      Math.max(0, startedAt + 5000 - Date.now()),
    );
    return () => window.clearTimeout(timer);
  }, [client, startedAt]);
  return startedAt !== null && Date.now() < startedAt + 5000;
}
