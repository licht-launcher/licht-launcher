import { ClockIcon, RefreshCwIcon, RotateCcwIcon } from "lucide-react";
import { toast } from "sonner";
import { restartApp } from "@/api/commands";
import { Button } from "@/components/ui/button";

import { restartToastStyles } from "./restart-toast.styles";

export const RESTART_TOAST_ID = "restart";

/** Keep in sync with `.toast-progress-bar` animation duration in tokens.css. */
const RESTART_TOAST_MS = 12_000;

export function dismissRestartToast() {
  toast.dismiss(RESTART_TOAST_ID);
}

export function promptRestart(message: string) {
  toast.custom(
    (id) => (
      <div className={restartToastStyles.root}>
        <div className={restartToastStyles.header}>
          <div aria-hidden className={restartToastStyles.iconWell}>
            <RotateCcwIcon
              className={restartToastStyles.icon}
              strokeWidth={2}
            />
          </div>
          <div className={restartToastStyles.text}>
            <p className={restartToastStyles.title}>Restart required</p>
            <p className={restartToastStyles.message}>{message}</p>
          </div>
        </div>
        <div className={restartToastStyles.actions}>
          <Button
            type="button"
            variant="outline"
            size="sm"
            className={restartToastStyles.action}
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
            className={restartToastStyles.action}
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
        <div aria-hidden className={restartToastStyles.progressBar} />
      </div>
    ),
    {
      id: RESTART_TOAST_ID,
      position: "bottom-center",
      duration: RESTART_TOAST_MS,
      unstyled: true,
    },
  );
}
