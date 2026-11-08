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

import { titleBarStyles } from "./title-bar.styles";

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
    <header data-title-bar className={titleBarStyles.root}>
      <div
        className={cn(
          titleBarStyles.bar,
          nativeCaption && titleBarStyles.barNativeCaption,
        )}
      >
        {children}
        <div
          data-tauri-drag-region
          className={titleBarStyles.dragRegion}
          aria-hidden
        />
        <div className={titleBarStyles.actions}>
          {updateAvailable ? (
            <Button
              type="button"
              variant="ghost"
              className={cn(titleBarStyles.chip, titleBarStyles.updateChip)}
              aria-label="Update"
              onClick={() => {
                window.dispatchEvent(new Event(INSTALL_UPDATE_EVENT));
              }}
            >
              <DownloadCloud
                aria-hidden
                className={titleBarStyles.updateIcon}
                strokeWidth={2}
              />
              Update
            </Button>
          ) : null}
          {actions}
        </div>
        {!windowControls || nativeCaption ? null : (
          <div data-window-controls className={titleBarStyles.windowControls}>
            <Button
              type="button"
              variant="ghost"
              className={titleBarStyles.control}
              aria-label="Minimize"
              onClick={() => {
                void windowApi?.minimize();
              }}
            >
              <Minus
                aria-hidden
                className={titleBarStyles.minimizeIcon}
                strokeWidth={1.75}
              />
            </Button>
            <Button
              type="button"
              variant="ghost"
              className={cn(
                titleBarStyles.control,
                titleBarStyles.closeControl,
              )}
              aria-label="Close"
              onClick={() => {
                void windowApi?.close();
              }}
            >
              <svg
                aria-hidden
                viewBox="0 0 24 24"
                className={titleBarStyles.closeIcon}
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
