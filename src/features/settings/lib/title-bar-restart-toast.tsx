import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { restartApp } from "@/lib/commands";

export const TITLE_BAR_RESTART_TOAST_ID = "title-bar-restart";

export function dismissTitleBarRestartToast() {
  toast.dismiss(TITLE_BAR_RESTART_TOAST_ID);
}

export function promptTitleBarRestart() {
  toast.custom(
    (id) => (
      <div className="flex w-[min(24rem,calc(100vw-2rem))] items-center gap-3 rounded-xl border border-border bg-popover px-3 py-3 text-popover-foreground shadow-none">
        <div className="min-w-0 flex-1">
          <p className="font-brand text-sm font-medium">Restart required</p>
          <p className="text-sm text-muted-foreground">
            Apply the title bar change now?
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-1.5">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => {
              toast.dismiss(id);
            }}
          >
            Later
          </Button>
          <Button
            type="button"
            size="sm"
            onClick={() => {
              toast.dismiss(id);
              void restartApp();
            }}
          >
            Restart
          </Button>
        </div>
      </div>
    ),
    {
      id: TITLE_BAR_RESTART_TOAST_ID,
      position: "bottom-center",
      duration: Number.POSITIVE_INFINITY,
      unstyled: true,
    },
  );
}
