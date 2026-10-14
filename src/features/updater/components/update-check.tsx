import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { DownloadIcon, OctagonXIcon, RefreshCwIcon, XIcon } from "lucide-react";
import { useCallback, useEffect } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { appReleaseLabel } from "@/lib/app-release";
import { getSettings } from "@/lib/commands";

const DEFERRED_UPDATE_KEY = "licht-launcher.deferred-update";
export const LATEST_VERSION_KEY = "licht-launcher.latest-version";
export const LATEST_VERSION_EVENT = "licht-launcher:latest-version";
export const MANUAL_UPDATE_CHECK_EVENT = "licht-launcher:manual-update-check";
export const UPDATE_CHECK_STATUS_EVENT = "licht-launcher:update-check-status";
const UPDATE_CHECK_STATUS_KEY = "licht-launcher.update-check-status";

export type UpdateCheckStatus =
  | "not-checked"
  | "checking"
  | "up-to-date"
  | "available"
  | "failed";

export function getUpdateCheckStatus(): UpdateCheckStatus {
  const status = sessionStorage.getItem(UPDATE_CHECK_STATUS_KEY);
  switch (status) {
    case "checking":
    case "up-to-date":
    case "available":
    case "failed":
      return status;
    default:
      return "not-checked";
  }
}

function setUpdateCheckStatus(status: UpdateCheckStatus) {
  sessionStorage.setItem(UPDATE_CHECK_STATUS_KEY, status);
  window.dispatchEvent(
    new CustomEvent(UPDATE_CHECK_STATUS_EVENT, {
      detail: { checking: status === "checking", status },
    }),
  );
}

export function UpdateCheck() {
  const showUpdateError = useCallback((message: string) => {
    const toastId = toast.custom(
      () => (
        <div
          className="pointer-events-auto relative z-100 flex w-[min(24rem,calc(100vw-2rem))] items-start gap-3 rounded-xl border border-border bg-popover px-3 py-3.5 pr-10 text-popover-foreground shadow-none"
          role="alert"
        >
          <button
            type="button"
            aria-label="Close update error"
            className="absolute top-2 right-2 inline-flex size-6 cursor-pointer items-center justify-center rounded-md p-1 text-muted-foreground opacity-70 transition-colors hover:bg-muted hover:text-foreground hover:opacity-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            onClick={() => {
              toast.dismiss(toastId);
            }}
          >
            <XIcon aria-hidden className="size-4" />
          </button>
          <div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
            <OctagonXIcon className="size-4" strokeWidth={2} />
          </div>
          <div className="min-w-0 flex-1">
            <p className="font-brand text-sm font-medium">
              Update check failed
            </p>
            <p className="text-sm text-muted-foreground">{message}</p>
          </div>
        </div>
      ),
      {
        id: "update-check-error",
        duration: 8000,
        unstyled: true,
      },
    );
  }, []);

  const installUpdate = useCallback(
    async (available: Update) => {
      try {
        await available.downloadAndInstall();
        localStorage.removeItem(DEFERRED_UPDATE_KEY);
        await relaunch();
      } catch (error) {
        showUpdateError(
          `Could not install the update: ${
            error instanceof Error ? error.message : String(error)
          }`,
        );
      }
    },
    [showUpdateError],
  );

  const showUpdateToast = useCallback(
    (available: Update) => {
      const toastId = toast.custom(
        () => (
          <div
            className="pointer-events-auto relative z-100 flex w-[min(24rem,calc(100vw-2rem))] flex-col gap-3 overflow-hidden rounded-xl border border-border bg-popover px-3 pt-3 pb-3.5 text-popover-foreground shadow-none"
            role="status"
          >
            <div className="flex items-start gap-3">
              <div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
                <RefreshCwIcon className="size-4" strokeWidth={2} />
              </div>
              <div className="min-w-0 flex-1">
                <p className="font-brand text-sm font-medium">
                  Update available
                </p>
                <p className="text-sm text-muted-foreground">
                  <span className="block">
                    Current version: Licht Launcher {appReleaseLabel}
                  </span>
                  <span className="block">
                    Available version: Licht Launcher {available.version}
                  </span>
                </p>
              </div>
            </div>
            <div className="flex items-center gap-1.5">
              <Button
                type="button"
                variant="outline"
                size="sm"
                className="min-w-0 flex-1 cursor-pointer"
                onClick={() => {
                  localStorage.setItem(DEFERRED_UPDATE_KEY, available.version);
                  toast.dismiss(toastId);
                }}
              >
                Later
              </Button>
              <Button
                type="button"
                size="sm"
                className="min-w-0 flex-1 cursor-pointer"
                onClick={() => {
                  toast.dismiss(toastId);
                  void installUpdate(available);
                }}
              >
                <DownloadIcon data-icon="inline-start" aria-hidden />
                Update now
              </Button>
            </div>
          </div>
        ),
        {
          id: `update-${available.version}`,
          duration: Infinity,
          unstyled: true,
        },
      );
      return toastId;
    },
    [installUpdate],
  );

  useEffect(() => {
    let cancelled = false;
    let checking = false;

    async function checkForUpdate(manual = false) {
      if (checking) {
        return;
      }
      checking = true;
      let status: UpdateCheckStatus = "not-checked";
      setUpdateCheckStatus("checking");
      const toastId = manual
        ? toast.loading("Checking for updates…", { id: "manual-update-check" })
        : undefined;
      try {
        const snapshot = await getSettings();
        if (!manual && !snapshot.settings.checkForUpdates) {
          return;
        }
        const available = await check();
        if (cancelled) {
          return;
        }
        if (available === null) {
          status = "up-to-date";
          localStorage.removeItem(LATEST_VERSION_KEY);
          window.dispatchEvent(new Event(LATEST_VERSION_EVENT));
          if (manual) {
            toast.success("Licht Launcher is up to date.", { id: toastId });
          }
          return;
        }

        status = "available";
        if (toastId !== undefined) {
          toast.dismiss(toastId);
        }
        localStorage.setItem(LATEST_VERSION_KEY, available.version);
        window.dispatchEvent(new Event(LATEST_VERSION_EVENT));
        const deferredVersion = localStorage.getItem(DEFERRED_UPDATE_KEY);
        if (
          deferredVersion === available.version &&
          snapshot.settings.installUpdatesOnRestart
        ) {
          await installUpdate(available);
          return;
        }

        showUpdateToast(available);
      } catch (error) {
        status = "failed";
        if (toastId !== undefined) {
          toast.dismiss(toastId);
        }
        if (!cancelled) {
          showUpdateError(
            `Could not check for updates: ${
              error instanceof Error ? error.message : String(error)
            }`,
          );
        }
      } finally {
        checking = false;
        if (cancelled && toastId !== undefined) {
          toast.dismiss(toastId);
        }
        setUpdateCheckStatus(status);
      }
    }

    void checkForUpdate();
    const onManualCheck = () => {
      void checkForUpdate(true);
    };
    window.addEventListener(MANUAL_UPDATE_CHECK_EVENT, onManualCheck);

    return () => {
      cancelled = true;
      window.removeEventListener(MANUAL_UPDATE_CHECK_EVENT, onManualCheck);
    };
  }, [installUpdate, showUpdateError, showUpdateToast]);

  return null;
}
