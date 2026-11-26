import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { settingsWindowReady } from "@/api/commands";
import { TitleBar } from "@/components/title-bar/title-bar";
import {
  parseSectionId,
  type SectionId,
} from "@/features/settings/model/sections";
import { SettingsPage } from "@/features/settings/settings-page";
import { useUpdateActions } from "@/features/updater/hooks/use-update-actions";
import { useSharedDataSync } from "@/hooks/use-shared-data-sync";
import { reportInterfaceError } from "@/lib/error-reporting";
import { friendlyError, messageOf } from "@/lib/errors";

import { settingsWindowStyles } from "./settings-window.styles";

export const SETTINGS_WINDOW_SECTION_EVENT = "settings-window-section";

export function SettingsWindow() {
  const [section, setSection] = useState<SectionId>("account");
  useSharedDataSync();
  useUpdateActions();

  // Ask for the section only once listening, so no request is missed.
  const revealed = useRef(false);
  useEffect(() => {
    let active = true;
    let unsubscribe: (() => void) | undefined;
    const reveal = async () => {
      if (revealed.current) return;
      revealed.current = true;
      try {
        setSection(parseSectionId(await settingsWindowReady()));
      } catch (error: unknown) {
        reportInterfaceError("promise", error);
        toast.error(friendlyError(messageOf(error)));
      }
      try {
        await document.fonts?.ready;
      } finally {
        await getCurrentWindow().show();
      }
    };
    listen<unknown>(SETTINGS_WINDOW_SECTION_EVENT, ({ payload }) => {
      if (active) setSection(parseSectionId(payload));
    })
      .then((stop) => {
        if (!active) {
          stop();
          return;
        }
        unsubscribe = stop;
        return reveal();
      })
      .catch((error: unknown) => {
        reportInterfaceError("promise", error);
        if (!active) return;
        toast.error("Could not listen for settings requests.");
        return reveal();
      })
      .catch((error: unknown) => reportInterfaceError("promise", error));
    return () => {
      active = false;
      unsubscribe?.();
    };
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
