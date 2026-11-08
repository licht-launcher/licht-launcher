import { useMutation, useQueryClient } from "@tanstack/react-query";
import { CircleCheckIcon, OctagonXIcon, RefreshCwIcon } from "lucide-react";
import { useEffect, useRef } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { useStartupProgress } from "@/features/startup/hooks/use-startup-progress";
import {
  APP_UPDATE_QUERY_KEY,
  useAppUpdateState,
} from "@/features/updater/hooks/use-app-update-state";
import { getUpdateSimulation } from "@/features/updater/model/simulation";
import {
  checkAppUpdate,
  downloadAppUpdate,
  installAppUpdate,
  openStartupPreview,
} from "@/lib/commands";
import { messageOf } from "@/lib/errors";
import type { AppUpdatePhase } from "@/lib/generated/AppUpdatePhase";

export {
  DEV_CLEAR_SIMULATED_UPDATE_EVENT,
  DEV_SIMULATE_UPDATE_EVENT,
  getUpdateSimulation,
  setUpdateSimulation,
  subscribeUpdateSimulation,
} from "@/features/updater/model/simulation";

export const MANUAL_UPDATE_CHECK_EVENT = "licht-launcher:manual-update-check";
export const DOWNLOAD_UPDATE_EVENT = "licht-launcher:download-update";
export const INSTALL_UPDATE_EVENT = "licht-launcher:install-update";
export const UPDATE_CHECK_STATUS_EVENT = "licht-launcher:update-check-status";
const STATUS_KEY = "licht-launcher.update-check-status";
export type UpdateCheckStatus =
  | "not-checked"
  | "checking"
  | "up-to-date"
  | "available"
  | "failed";

export function getUpdateCheckStatus(): UpdateCheckStatus {
  const status = sessionStorage.getItem(STATUS_KEY);
  return status === "checking" ||
    status === "up-to-date" ||
    status === "available" ||
    status === "failed"
    ? status
    : "not-checked";
}

function legacyStatus(phase: AppUpdatePhase | undefined): UpdateCheckStatus {
  if (
    !phase ||
    ["checking", "downloading", "verifying", "installing"].includes(phase)
  )
    return "checking";
  if (phase === "ready") return "available";
  if (phase === "upToDate") return "up-to-date";
  if (phase === "failed") return "failed";
  return "not-checked";
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
  const description =
    message ??
    (status === "checking"
      ? "Checking for a newer version of Licht Launcher."
      : "No newer update is available for this installation.");
  return toast.custom(
    (id) => (
      <div
        role={status === "failed" ? "alert" : "status"}
        className="pointer-events-auto flex w-full max-w-sm items-start gap-3 rounded-xl border border-border bg-popover p-3 text-popover-foreground"
      >
        <Icon
          aria-hidden
          className={
            status === "checking"
              ? "size-4 shrink-0 animate-spin"
              : "size-4 shrink-0"
          }
        />
        <div className="flex min-w-0 flex-1 flex-col gap-0.5">
          <p className="text-sm font-medium">{title}</p>
          <p className="text-sm text-muted-foreground">{description}</p>
        </div>
        <Button
          type="button"
          variant="ghost"
          size="xs"
          aria-label="Close update notification"
          onClick={(event) => {
            event.stopPropagation();
            toast.dismiss(id);
          }}
        >
          Close
        </Button>
      </div>
    ),
    {
      id: status === "failed" ? "update-check-error" : "manual-update-check",
      position: "bottom-right",
      duration:
        status === "checking" ? Infinity : status === "failed" ? 8000 : 4000,
      unstyled: true,
    },
  );
}

export function UpdateCheck() {
  const query = useAppUpdateState();
  const startup = useStartupProgress(true);
  const client = useQueryClient();
  const manual = useRef<"check" | "download" | "install" | null>(null);
  const warningShown = useRef(false);
  const actions = useMutation({
    mutationFn: (action: "check" | "download" | "install") => {
      if (getUpdateSimulation())
        return action === "install"
          ? openStartupPreview("updateAndMigration")
          : Promise.resolve();
      if (action === "check") return checkAppUpdate();
      if (action === "download") return downloadAppUpdate();
      return installAppUpdate();
    },
    onMutate: (action) => {
      if (getUpdateSimulation() && action !== "install") {
        manual.current = null;
        return;
      }
      manual.current = action;
      if (action === "check" && !getUpdateSimulation())
        showCheckToast("checking");
    },
    onSuccess: (_data, action) => {
      if (action === "install") {
        manual.current = null;
        toast.dismiss("app-update-ready");
      }
    },
    onError: (error: unknown) => {
      manual.current = null;
      toast.dismiss("manual-update-check");
      showCheckToast("failed", messageOf(error));
    },
    onSettled: () =>
      client.invalidateQueries({ queryKey: APP_UPDATE_QUERY_KEY }),
  });
  const { mutate } = actions;
  useEffect(() => {
    const check = () => mutate("check");
    const download = () => mutate("download");
    const install = () => mutate("install");
    window.addEventListener(MANUAL_UPDATE_CHECK_EVENT, check);
    window.addEventListener(DOWNLOAD_UPDATE_EVENT, download);
    window.addEventListener(INSTALL_UPDATE_EVENT, install);
    return () => {
      window.removeEventListener(MANUAL_UPDATE_CHECK_EVENT, check);
      window.removeEventListener(DOWNLOAD_UPDATE_EVENT, download);
      window.removeEventListener(INSTALL_UPDATE_EVENT, install);
    };
  }, [mutate]);
  const state = query.data;
  useEffect(() => {
    const status = legacyStatus(state?.phase);
    sessionStorage.setItem(STATUS_KEY, status);
    window.dispatchEvent(
      new CustomEvent(UPDATE_CHECK_STATUS_EVENT, {
        detail: { checking: status === "checking", status },
      }),
    );
  }, [state?.phase]);
  useEffect(() => {
    if (state?.phase !== "ready" || !state.version || state.error) {
      toast.dismiss("app-update-ready");
      return;
    }
    manual.current = null;
    toast.dismiss("manual-update-check");
    toast.custom(
      (id) => (
        <div className="pointer-events-auto flex w-full max-w-sm flex-col gap-3 rounded-xl border border-border bg-popover p-3 text-popover-foreground">
          <div className="flex flex-col gap-0.5">
            <p className="text-sm font-medium">Update ready</p>
            <p className="text-sm text-muted-foreground">
              Version {state.version} is ready. Restart to apply.
            </p>
          </div>
          <div className="flex items-center justify-end gap-2">
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => toast.dismiss(id)}
            >
              Later
            </Button>
            <Button
              type="button"
              size="sm"
              onClick={() =>
                window.dispatchEvent(new Event(INSTALL_UPDATE_EVENT))
              }
            >
              Restart to update
            </Button>
          </div>
        </div>
      ),
      {
        id: "app-update-ready",
        position: "bottom-right",
        duration: Infinity,
        unstyled: true,
      },
    );
  }, [state?.phase, state?.version, state?.error]);
  useEffect(() => {
    if (!manual.current) return;
    if (
      state?.phase === "upToDate" ||
      state?.phase === "failed" ||
      state?.error
    ) {
      manual.current = null;
      toast.dismiss("manual-update-check");
      showCheckToast(
        state.phase === "failed" || state.error ? "failed" : "up-to-date",
        state.error ?? undefined,
      );
    }
  }, [state?.phase, state?.error]);
  useEffect(() => {
    if (query.error && !query.simulated)
      showCheckToast("failed", messageOf(query.error));
  }, [query.error, query.simulated]);
  useEffect(() => {
    if (startup.data?.warning && !warningShown.current) {
      warningShown.current = true;
      toast.warning("Launcher startup warning", {
        id: "startup-warning",
        description: startup.data.warning,
      });
    }
  }, [startup.data]);
  return null;
}
