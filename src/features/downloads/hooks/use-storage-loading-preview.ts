import { type QueryClient, useQuery } from "@tanstack/react-query";

export const STORAGE_LOADING_PREVIEW_EVENT = "licht:preview-storage-loading";
export const STORAGE_LOADING_PREVIEW_QUERY_KEY = [
  "dev",
  "storage-loading-preview",
] as const;

export function setStorageLoadingPreview(
  client: QueryClient,
  enabled: boolean,
) {
  if (!import.meta.env.DEV) return;
  client.setQueryData(STORAGE_LOADING_PREVIEW_QUERY_KEY, enabled);
  if (enabled) window.dispatchEvent(new Event(STORAGE_LOADING_PREVIEW_EVENT));
}

export function useStorageLoadingPreview(): boolean {
  const query = useQuery({
    queryKey: STORAGE_LOADING_PREVIEW_QUERY_KEY,
    queryFn: (): boolean => false,
    initialData: false,
    enabled: false,
    gcTime: Infinity,
  });
  return import.meta.env.DEV && query.data === true;
}
