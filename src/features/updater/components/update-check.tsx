import { useMutation } from "@tanstack/react-query";
import { check, type Update } from "@tauri-apps/plugin-updater";
import {
  CircleCheckIcon,
  DownloadIcon,
  OctagonXIcon,
  RefreshCwIcon,
  XIcon,
} from "lucide-react";
import { useCallback, useEffect, useRef } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { useStartupProgress } from "@/features/startup/hooks/use-startup-progress";
import { APP_VERSION, appReleaseLabel } from "@/lib/app-release";
import { openStartupPreview, requestUpdateRestart } from "@/lib/commands";

export const LATEST_VERSION_KEY = "licht-launcher.latest-version";
export const LATEST_VERSION_EVENT = "licht-launcher:latest-version";
export const MANUAL_UPDATE_CHECK_EVENT = "licht-launcher:manual-update-check";
export const INSTALL_UPDATE_EVENT = "licht-launcher:install-update";
export const DEV_SIMULATE_UPDATE_EVENT = "licht-launcher:dev-simulate-update";
export const DEV_CLEAR_SIMULATED_UPDATE_EVENT =
  "licht-launcher:dev-clear-simulated-update";
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

function showCheckToast(
  status: "checking" | "up-to-date" | "failed",
  message?: string,
) {
  const Icon =
    status === "failed"
      ? OctagonXIcon
      : status === "checking"
        ? RefreshCwIcon
        : CircleCheckIcon;
  const title =
    status === "failed"
      ? "Update check failed"
      : status === "checking"
        ? "Checking for updates…"
        : "Licht Launcher is up to date.";
  const descriptions = {
    checking: "Checking for a newer version of Licht Launcher.",
    "up-to-date": "No newer update is available for this installation.",
    failed: "Please try again.",
  };
  const description = message ?? descriptions[status];

  return toast.custom(
    (id) => (
      <div
        className="group/toast pointer-events-auto relative z-100 flex w-96 max-w-full items-start gap-3 overflow-hidden rounded-xl border border-border bg-popover px-3 py-3.5 pr-12 text-popover-foreground shadow-none"
        role={status === "failed" ? "alert" : "status"}
      >
        <Button
          type="button"
          aria-label="Close update notification"
          variant="ghost"
          size="icon-sm"
          className="absolute top-2 right-2 text-muted-foreground"
          onClick={(event) => {
            event.stopPropagation();
            toast.dismiss(id);
          }}
        >
          <XIcon aria-hidden className="size-4" />
        </Button>
        <div
          aria-hidden
          className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground"
        >
          <Icon
            className={status === "checking" ? "size-4 animate-spin" : "size-4"}
            strokeWidth={2}
          />
        </div>
        <div className="min-w-0 flex-1">
          <p className="font-brand text-sm font-medium">{title}</p>
          <p className="text-sm text-muted-foreground">{description}</p>
        </div>
      </div>
    ),
    {
      id: status === "failed" ? "update-check-error" : "manual-update-check",
      position: "bottom-right",
      duration:
        status === "checking" ? Infinity : status === "failed" ? 8000 : 4000,
      closeButton: false,
      unstyled: true,
    },
  );
}

export function UpdateCheck() {
  const startup = useStartupProgress(true);
  const manualStarted = useRef(false);
  const warningShown = useRef(false);
  const { mutateAsync: beginUpdate } = useMutation({
    mutationFn: (preview: boolean) =>
      preview
        ? openStartupPreview("updateAndMigration")
        : requestUpdateRestart(),
  });
  const showUpdateError = useCallback((message: string) => {
    showCheckToast("failed", message);
  }, []);

  const installUpdate = useCallback(
    async (preview = false) => {
      try {
        await beginUpdate(preview);
      } catch (error) {
        showUpdateError(
          `Could not start the update: ${
            error instanceof Error ? error.message : String(error)
          }`,
        );
      }
    },
    [beginUpdate, showUpdateError],
  );

  const showUpdateToast = useCallback(
    (available: Pick<Update, "version">) => {
      const toastId = toast.custom(
        (id) => (
          <div
            className="group/toast pointer-events-auto relative z-100 flex w-96 max-w-full flex-col gap-3 overflow-hidden rounded-xl border border-border bg-popover px-3 pt-3 pb-3.5 text-popover-foreground shadow-none"
            role="status"
          >
            <Button
              type="button"
              aria-label="Close update notification"
              variant="ghost"
              size="icon-sm"
              className="absolute top-2 right-2 text-muted-foreground"
              onClick={(event) => {
                event.stopPropagation();
                toast.dismiss(id);
              }}
            >
              <XIcon aria-hidden className="size-4" />
            </Button>
            <div className="flex items-start gap-3 pr-8">
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
                onClick={(event) => {
                  event.stopPropagation();
                  toast.dismiss(id);
                }}
              >
                Later
              </Button>
              <Button
                type="button"
                size="sm"
                className="min-w-0 flex-1 cursor-pointer"
                onClick={(event) => {
                  event.stopPropagation();
                  toast.dismiss(id);
                  void installUpdate();
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
          position: "bottom-right",
          duration: Infinity,
          closeButton: false,
          unstyled: true,
        },
      );
      return toastId;
    },
    [installUpdate],
  );

  useEffect(() => {
    const snapshot = startup.data;
    if (!snapshot || snapshot.preview || manualStarted.current) return;
    const status: UpdateCheckStatus =
      snapshot.phase === "checkingUpdates"
        ? "checking"
        : snapshot.updateStatus === "upToDate"
          ? "up-to-date"
          : snapshot.updateStatus === "available"
            ? "available"
            : snapshot.updateStatus === "failed"
              ? "failed"
              : "not-checked";
    setUpdateCheckStatus(status);
    if (snapshot.updateStatus === "upToDate")
      localStorage.removeItem(LATEST_VERSION_KEY);
    if (snapshot.updateStatus === "available" && snapshot.version) {
      localStorage.setItem(LATEST_VERSION_KEY, snapshot.version);
      showUpdateToast({ version: snapshot.version });
    }
    window.dispatchEvent(new Event(LATEST_VERSION_EVENT));
    if (snapshot.warning && !warningShown.current) {
      warningShown.current = true;
      toast.warning("Launcher startup warning", {
        id: "startup-warning",
        description: snapshot.warning,
      });
    }
  }, [startup.data, showUpdateToast]);

  useEffect(() => {
    let cancelled = false;
    let checking = false;
    let availableUpdate: Update | null = null;
    let simulatedUpdate = false;

    async function checkForUpdate() {
      if (checking) {
        return;
      }
      checking = true;
      let status: UpdateCheckStatus = "not-checked";
      setUpdateCheckStatus("checking");
      const toastId = showCheckToast("checking");
      try {
        const available = await check();
        if (cancelled) {
          return;
        }
        if (available === null) {
          status = "up-to-date";
          localStorage.removeItem(LATEST_VERSION_KEY);
          window.dispatchEvent(new Event(LATEST_VERSION_EVENT));
          showCheckToast("up-to-date");
          return;
        }

        status = "available";
        availableUpdate = available;
        if (toastId !== undefined) {
          toast.dismiss(toastId);
        }
        localStorage.setItem(LATEST_VERSION_KEY, available.version);
        window.dispatchEvent(new Event(LATEST_VERSION_EVENT));
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

    const onManualCheck = () => {
      manualStarted.current = true;
      void checkForUpdate();
    };
    const onInstallUpdate = () => {
      manualStarted.current = true;
      if (simulatedUpdate) {
        void installUpdate(true);
      } else if (
        availableUpdate !== null ||
        getUpdateCheckStatus() === "available"
      ) {
        void installUpdate();
      }
    };
    const onSimulateUpdate = () => {
      if (!import.meta.env.DEV) {
        return;
      }
      simulatedUpdate = true;
      manualStarted.current = true;
      const version = `${APP_VERSION}-dev`;
      localStorage.setItem(LATEST_VERSION_KEY, version);
      window.dispatchEvent(new Event(LATEST_VERSION_EVENT));
      setUpdateCheckStatus("available");
    };
    const onClearSimulatedUpdate = () => {
      if (!import.meta.env.DEV) {
        return;
      }
      simulatedUpdate = false;
      localStorage.removeItem(LATEST_VERSION_KEY);
      window.dispatchEvent(new Event(LATEST_VERSION_EVENT));
      setUpdateCheckStatus("not-checked");
    };
    window.addEventListener(MANUAL_UPDATE_CHECK_EVENT, onManualCheck);
    window.addEventListener(INSTALL_UPDATE_EVENT, onInstallUpdate);
    window.addEventListener(DEV_SIMULATE_UPDATE_EVENT, onSimulateUpdate);
    window.addEventListener(
      DEV_CLEAR_SIMULATED_UPDATE_EVENT,
      onClearSimulatedUpdate,
    );

    return () => {
      cancelled = true;
      window.removeEventListener(MANUAL_UPDATE_CHECK_EVENT, onManualCheck);
      window.removeEventListener(INSTALL_UPDATE_EVENT, onInstallUpdate);
      window.removeEventListener(DEV_SIMULATE_UPDATE_EVENT, onSimulateUpdate);
      window.removeEventListener(
        DEV_CLEAR_SIMULATED_UPDATE_EVENT,
        onClearSimulatedUpdate,
      );
    };
  }, [installUpdate, showUpdateError, showUpdateToast]);

  return null;
}
