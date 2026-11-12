import { useMutation, useQueryClient } from "@tanstack/react-query";
import { CircleCheckIcon, OctagonXIcon, RefreshCwIcon } from "lucide-react";
import { useEffect, useRef } from "react";
import { toast } from "sonner";
import {
  checkAppUpdate,
  downloadAppUpdate,
  installAppUpdate,
  openStartupPreview,
} from "@/api/commands";
import { Button } from "@/components/ui/button";
import { useStartupProgress } from "@/features/startup/hooks/use-startup-progress";
import {
  APP_UPDATE_QUERY_KEY,
  useAppUpdateState,
} from "@/features/updater/hooks/use-app-update-state";
import { getUpdateSimulation } from "@/features/updater/model/simulation";
import { messageOf } from "@/lib/errors";
import type { AppUpdatePhase } from "@/lib/generated/AppUpdatePhase";

import { updateCheckStyles } from "./update-check.styles";

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
  | "failed"
  | "managed-externally";

export function getUpdateCheckStatus(): UpdateCheckStatus {
  const status = sessionStorage.getItem(STATUS_KEY);
  return status === "checking" ||
    status === "up-to-date" ||
    status === "available" ||
    status === "failed" ||
    status === "managed-externally"
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
  if (phase === "externallyManaged" || phase === "portable")
    return "managed-externally";
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
        className={updateCheckStyles.toast}
      >
        <Icon
          aria-hidden
          className={
            status === "checking"
              ? updateCheckStyles.spinner
              : updateCheckStyles.icon
          }
        />
        <div className={updateCheckStyles.toastText}>
          <p className={updateCheckStyles.toastTitle}>{title}</p>
          <p className={updateCheckStyles.toastDescription}>{description}</p>
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
        <div className={updateCheckStyles.prompt}>
          <div className={updateCheckStyles.promptText}>
            <p className={updateCheckStyles.toastTitle}>Update ready</p>
            <p className={updateCheckStyles.toastDescription}>
              Version {state.version} is ready. Restart to apply.
            </p>
          </div>
          <div className={updateCheckStyles.promptActions}>
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
