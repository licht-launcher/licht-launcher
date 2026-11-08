import { ClockIcon, RefreshCwIcon, RotateCcwIcon } from "lucide-react";
import { toast } from "sonner";
import { restartApp } from "@/api/commands";
import { Button } from "@/components/ui/button";

import { titleBarRestartToastStyles } from "./title-bar-restart-toast.styles";

export const TITLE_BAR_RESTART_TOAST_ID = "title-bar-restart";

/** Keep in sync with `.toast-progress-bar` animation duration in tokens.css. */
const TITLE_BAR_RESTART_TOAST_MS = 12_000;

export function dismissTitleBarRestartToast() {
  toast.dismiss(TITLE_BAR_RESTART_TOAST_ID);
}

export function promptTitleBarRestart(
  message = "Apply the title bar change now?",
) {
  toast.custom(
    (id) => (
      <div className={titleBarRestartToastStyles.root}>
        <div className={titleBarRestartToastStyles.header}>
          <div aria-hidden className={titleBarRestartToastStyles.iconWell}>
            <RotateCcwIcon
              className={titleBarRestartToastStyles.icon}
              strokeWidth={2}
            />
          </div>
          <div className={titleBarRestartToastStyles.text}>
            <p className={titleBarRestartToastStyles.title}>Restart required</p>
            <p className={titleBarRestartToastStyles.message}>{message}</p>
          </div>
        </div>
        <div className={titleBarRestartToastStyles.actions}>
          <Button
            type="button"
            variant="outline"
            size="sm"
            className={titleBarRestartToastStyles.action}
            onClick={(event) => {
              event.stopPropagation();
              toast.dismiss(id);
            }}
          >
            <ClockIcon data-icon="inline-start" aria-hidden />
            Later
          </Button>
          <Button
            type="button"
            size="sm"
            className={titleBarRestartToastStyles.action}
            onClick={(event) => {
              event.stopPropagation();
              toast.dismiss(id);
              void restartApp();
            }}
          >
            <RefreshCwIcon data-icon="inline-start" aria-hidden />
            Restart
          </Button>
        </div>
        <div aria-hidden className={titleBarRestartToastStyles.progressBar} />
      </div>
    ),
    {
      id: TITLE_BAR_RESTART_TOAST_ID,
      position: "bottom-center",
      duration: TITLE_BAR_RESTART_TOAST_MS,
      unstyled: true,
    },
  );
}
