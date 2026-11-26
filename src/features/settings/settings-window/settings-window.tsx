import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { TitleBar } from "@/components/title-bar/title-bar";
import {
  parseSectionId,
  type SectionId,
} from "@/features/settings/model/sections";
import { SettingsPage } from "@/features/settings/settings-page";
import { useUpdateActions } from "@/features/updater/hooks/use-update-actions";
import { useSharedDataSync } from "@/hooks/use-shared-data-sync";
import { reportInterfaceError } from "@/lib/error-reporting";

import { settingsWindowStyles } from "./settings-window.styles";

/** Sent by the backend when another section is requested while this window is open. */
export const SETTINGS_WINDOW_SECTION_EVENT = "settings-window-section";

/** The section the backend asked for when it created this window. */
function initialSection(): SectionId {
  const value: unknown =
    "__LICHT_SETTINGS_SECTION__" in window
      ? window.__LICHT_SETTINGS_SECTION__
      : undefined;
  return parseSectionId(value);
}

export function SettingsWindow() {
  const [section, setSection] = useState<SectionId>(initialSection);
  useSharedDataSync();
  // The Updates section's buttons are answered in this window.
  useUpdateActions();

  useEffect(() => {
    let active = true;
    let unsubscribe: (() => void) | undefined;
    listen<unknown>(SETTINGS_WINDOW_SECTION_EVENT, ({ payload }) => {
      if (active) setSection(parseSectionId(payload));
    })
      .then((stop) => {
        if (active) unsubscribe = stop;
        else stop();
      })
      .catch(() => {
        if (active) toast.error("Could not listen for settings requests.");
      });
    return () => {
      active = false;
      unsubscribe?.();
    };
  }, []);

  // The window starts hidden and is shown once it has painted, like the launcher.
  const revealed = useRef(false);
  useEffect(() => {
    if (revealed.current) return;
    revealed.current = true;
    const fonts = document.fonts?.ready ?? Promise.resolve();
    fonts
      .finally(() => getCurrentWindow().show())
      .catch((error: unknown) => reportInterfaceError("promise", error));
  }, []);

  return (
    <div className={settingsWindowStyles.root}>
      <SettingsPage section={section} onSectionChange={setSection} />
      <div className={settingsWindowStyles.titleBar}>
        <TitleBar />
      </div>
    </div>
  );
}
