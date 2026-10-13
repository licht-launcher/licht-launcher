import { ClockIcon, RefreshCwIcon, RotateCcwIcon } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { restartApp } from "@/lib/commands";

export const TITLE_BAR_RESTART_TOAST_ID = "title-bar-restart";

/** Keep in sync with `.toast-progress-bar` animation duration in tokens.css. */
const TITLE_BAR_RESTART_TOAST_MS = 12_000;

export function dismissTitleBarRestartToast() {
  toast.dismiss(TITLE_BAR_RESTART_TOAST_ID);
}

export function promptTitleBarRestart() {
  toast.custom(
    (id) => (
      <div className="group/toast pointer-events-auto relative z-100 flex w-[min(24rem,calc(100vw-2rem))] flex-col gap-3 overflow-hidden rounded-xl border border-border bg-popover px-3 pt-3 pb-3.5 text-popover-foreground shadow-none">
        <div className="flex items-start gap-3">
          <div
            aria-hidden
            className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground"
          >
            <RotateCcwIcon className="size-4" strokeWidth={2} />
          </div>
          <div className="min-w-0 flex-1">
            <p className="font-brand text-sm font-medium">Restart required</p>
            <p className="text-sm text-muted-foreground">
              Apply the title bar change now?
            </p>
          </div>
        </div>
        <div className="flex items-center gap-1.5">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="min-w-0 flex-1 cursor-pointer"
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
            className="min-w-0 flex-1 cursor-pointer"
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
        <div
          aria-hidden
          className="toast-progress-bar absolute inset-x-0 bottom-0 h-0.5 bg-primary"
        />
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
