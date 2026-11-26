import { emitTo } from "@tauri-apps/api/event";
import { useSyncExternalStore } from "react";
import { reportInterfaceError } from "@/lib/error-reporting";

export const STORAGE_LOADING_PREVIEW_EVENT = "dev-storage-loading-preview";
const CHANGE_EVENT = "licht-launcher:dev-storage-loading-preview-change";
// Shared with the settings window; the launcher clears it when it starts.
const key = "licht-launcher.dev-storage-loading-preview";

function getStorageLoadingPreview(): boolean {
  return import.meta.env.DEV && localStorage.getItem(key) === "true";
}

export function setStorageLoadingPreview(enabled: boolean) {
  if (!import.meta.env.DEV) return;
  if (enabled) localStorage.setItem(key, "true");
  else localStorage.removeItem(key);
  window.dispatchEvent(new Event(CHANGE_EVENT));
  if (enabled) {
    emitTo("main", STORAGE_LOADING_PREVIEW_EVENT).catch((error: unknown) =>
      reportInterfaceError("promise", error),
    );
  }
}

export function clearStorageLoadingPreview() {
  localStorage.removeItem(key);
}

function subscribe(listener: () => void) {
  const onStorage = (event: StorageEvent) => {
    if (event.key === key || event.key === null) listener();
  };
  window.addEventListener(CHANGE_EVENT, listener);
  window.addEventListener("storage", onStorage);
  return () => {
    window.removeEventListener(CHANGE_EVENT, listener);
    window.removeEventListener("storage", onStorage);
  };
}

export function useStorageLoadingPreview(): boolean {
  return useSyncExternalStore(subscribe, getStorageLoadingPreview, () => false);
}
