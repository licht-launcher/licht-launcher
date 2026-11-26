import { useEffect, useRef } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { useStartupProgress } from "@/features/startup/hooks/use-startup-progress";
import {
  INSTALL_UPDATE_EVENT,
  showCheckToast,
  useUpdateActions,
} from "@/features/updater/hooks/use-update-actions";
import { messageOf } from "@/lib/errors";
import type { AppUpdatePhase } from "@/lib/generated/AppUpdatePhase";

import { updateCheckStyles } from "./update-check.styles";

export {
  DOWNLOAD_UPDATE_EVENT,
  INSTALL_UPDATE_EVENT,
  MANUAL_UPDATE_CHECK_EVENT,
} from "@/features/updater/hooks/use-update-actions";
export {
  DEV_CLEAR_SIMULATED_UPDATE_EVENT,
  DEV_SIMULATE_UPDATE_EVENT,
  getUpdateSimulation,
  setUpdateSimulation,
  subscribeUpdateSimulation,
} from "@/features/updater/model/simulation";
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
  if (phase === "externallyManaged" || phase === "portable" || phase === "msi")
    return "managed-externally";
  return "not-checked";
}

export function UpdateCheck() {
  const query = useUpdateActions();
  const startup = useStartupProgress(true);
  const warningShown = useRef(false);
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
