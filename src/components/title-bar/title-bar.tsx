import { getCurrentWindow } from "@tauri-apps/api/window";
import { DownloadCloud, Minus } from "lucide-react";
import { type ReactNode, useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import {
  getUpdateCheckStatus,
  INSTALL_UPDATE_EVENT,
  UPDATE_CHECK_STATUS_EVENT,
} from "@/features/updater/components/update-check";
import { usesNativeCaptionButtons } from "@/lib/native-caption";
import { cn } from "@/lib/utils";

const controlClass =
  "h-full w-14 shrink-0 rounded-none [&_svg:not([class*='size-'])]:size-4";

const chipClass =
  "h-6 max-w-36 gap-1.5 rounded-md border border-border/60 bg-muted/40 px-2 text-xs font-medium text-muted-foreground hover:bg-muted hover:text-foreground";

function currentWindow() {
  try {
    return getCurrentWindow();
  } catch {
    return null;
  }
}

export function TitleBar({
  children,
  actions,
  windowControls = true,
}: {
  children?: ReactNode;
  actions?: ReactNode;
  windowControls?: boolean;
}) {
  const windowApi = currentWindow();
  const nativeCaption = windowControls && usesNativeCaptionButtons();
  const [updateAvailable, setUpdateAvailable] = useState(
    () => getUpdateCheckStatus() === "available",
  );
  useEffect(() => {
    const updateStatus = () => {
      setUpdateAvailable(getUpdateCheckStatus() === "available");
    };
    window.addEventListener(UPDATE_CHECK_STATUS_EVENT, updateStatus);
    updateStatus();
    return () => {
      window.removeEventListener(UPDATE_CHECK_STATUS_EVENT, updateStatus);
    };
  }, []);

  return (
    <header
      data-title-bar
      className="pointer-events-auto relative z-[60] shrink-0 text-popover-foreground"
    >
      <div
        className={cn(
          "flex h-9 items-stretch",
          nativeCaption && "pr-[var(--native-caption-inset)]",
        )}
      >
        {children}
        <div data-tauri-drag-region className="min-w-0 flex-1" aria-hidden />
        <div className="flex shrink-0 items-center gap-1.5 px-2 text-muted-foreground">
          {updateAvailable ? (
            <Button
              type="button"
              variant="ghost"
              className={cn(chipClass, "cursor-pointer")}
              aria-label="Update"
              onClick={() => {
                window.dispatchEvent(new Event(INSTALL_UPDATE_EVENT));
              }}
            >
              <DownloadCloud
                aria-hidden
                className="size-3.5 shrink-0"
                strokeWidth={2}
              />
              Update
            </Button>
          ) : null}
          {actions}
        </div>
        {!windowControls || nativeCaption ? null : (
          <div className="flex shrink-0 items-stretch text-muted-foreground">
            <Button
              type="button"
              variant="ghost"
              className={controlClass}
              aria-label="Minimize"
              onClick={() => {
                void windowApi?.minimize();
              }}
            >
              <Minus aria-hidden className="size-4" strokeWidth={1.75} />
            </Button>
            <Button
              type="button"
              variant="ghost"
              className={cn(
                controlClass,
                "hover:bg-destructive/20 hover:text-destructive",
              )}
              aria-label="Close"
              onClick={() => {
                void windowApi?.close();
              }}
            >
              <svg
                aria-hidden
                viewBox="0 0 24 24"
                className="size-5"
                fill="none"
              >
                <path
                  d="M7 7l10 10M17 7L7 17"
                  stroke="currentColor"
                  strokeWidth={1.75}
                  strokeLinecap="round"
                />
              </svg>
            </Button>
          </div>
        )}
      </div>
    </header>
  );
}
