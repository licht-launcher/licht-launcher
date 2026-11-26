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
import { updateCheckStyles } from "@/features/updater/components/update-check/update-check.styles";
import {
  APP_UPDATE_QUERY_KEY,
  useAppUpdateState,
} from "@/features/updater/hooks/use-app-update-state";
import { getUpdateSimulation } from "@/features/updater/model/simulation";
import { messageOf } from "@/lib/errors";

export const MANUAL_UPDATE_CHECK_EVENT = "licht-launcher:manual-update-check";
export const DOWNLOAD_UPDATE_EVENT = "licht-launcher:download-update";
export const INSTALL_UPDATE_EVENT = "licht-launcher:install-update";

export function showCheckToast(
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

/**
 * Runs the update actions requested in this window and reports their result
 * here, so the launcher and settings windows each answer their own buttons.
 */
export function useUpdateActions() {
  const query = useAppUpdateState();
  const client = useQueryClient();
  const manual = useRef<"check" | "download" | "install" | null>(null);
  const actions = useMutation({
    mutationFn: (action: "check" | "download" | "install") => {
      if (getUpdateSimulation())
        return action === "install"
          ? openStartupPreview("update")
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
    if (state?.phase === "ready" && state.version && !state.error) {
      manual.current = null;
      toast.dismiss("manual-update-check");
      return;
    }
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
  }, [state?.phase, state?.version, state?.error]);
  return query;
}
