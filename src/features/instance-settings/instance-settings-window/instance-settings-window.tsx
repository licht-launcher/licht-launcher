import { useQuery } from "@tanstack/react-query";
import { emitTo, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { instanceSettingsWindowReady, listInstances } from "@/api/commands";
import { TitleBar } from "@/components/title-bar/title-bar";
import { InstanceSettingsPage } from "@/features/instance-settings/instance-settings-page";
import {
  INSTANCE_SETTINGS_ACTION_EVENT,
  INSTANCE_SETTINGS_ACTION_FAILED_EVENT,
  INSTANCE_SETTINGS_STATUS_EVENT,
  INSTANCE_SETTINGS_STATUS_REQUEST_EVENT,
  type InstanceSettingsAction,
  type InstanceSettingsStatus,
  parseInstanceSettingsActionFailure,
  parseInstanceSettingsStatus,
} from "@/features/instance-settings/model/window-events";
import { InstanceActionDialog } from "@/features/instances/components/instance-action-dialog";
import { useToastError } from "@/features/settings/hooks/use-toast-error";
import { useSharedDataSync } from "@/hooks/use-shared-data-sync";
import { reportInterfaceError } from "@/lib/error-reporting";
import { friendlyError, messageOf } from "@/lib/errors";

import { instanceSettingsWindowStyles } from "./instance-settings-window.styles";

const LAUNCHER_WINDOW = "main";

const IDLE: InstanceSettingsStatus = {
  busy: false,
  versionBusy: false,
  activeFolders: [],
};

export function InstanceSettingsWindow() {
  const [folder, setFolder] = useState<string | null>(null);
  const [status, setStatus] = useState<InstanceSettingsStatus>(IDLE);
  const [confirmingDelete, setConfirmingDelete] = useState(false);
  const folderRef = useRef<string | null>(null);
  useSharedDataSync();

  const instances = useQuery({
    queryKey: ["instances"],
    queryFn: listInstances,
    retry: false,
  });
  useToastError(instances.error);
  const entry =
    instances.data?.find((instance) => instance.folder === folder) ?? null;

  // Ask for the instance only once listening, so no status or failure is missed.
  useEffect(() => {
    let active = true;
    const stops: (() => void)[] = [];
    const reveal = async () => {
      try {
        const ready = await instanceSettingsWindowReady();
        folderRef.current = ready;
        if (active) setFolder(ready);
        await emitTo(LAUNCHER_WINDOW, INSTANCE_SETTINGS_STATUS_REQUEST_EVENT);
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
    Promise.all([
      listen<unknown>(INSTANCE_SETTINGS_STATUS_EVENT, ({ payload }) => {
        const next = parseInstanceSettingsStatus(payload);
        if (active && next) setStatus(next);
      }),
      listen<unknown>(INSTANCE_SETTINGS_ACTION_FAILED_EVENT, ({ payload }) => {
        const failure = parseInstanceSettingsActionFailure(payload);
        if (active && failure && failure.folder === folderRef.current) {
          toast.error(failure.message);
        }
      }),
    ])
      .then((subscribed) => {
        if (!active) {
          for (const stop of subscribed) stop();
          return;
        }
        stops.push(...subscribed);
        return reveal();
      })
      .catch((error: unknown) => {
        reportInterfaceError("promise", error);
        if (!active) return;
        toast.error("Could not listen for launcher changes.");
        return reveal();
      })
      .catch((error: unknown) => reportInterfaceError("promise", error));
    return () => {
      active = false;
      for (const stop of stops) stop();
    };
  }, []);

  const name = entry?.name;
  useEffect(() => {
    if (name === undefined) return;
    getCurrentWindow()
      .setTitle(`${name} - Licht Launcher`)
      .catch((error: unknown) => reportInterfaceError("promise", error));
  }, [name]);

  function request(action: InstanceSettingsAction) {
    emitTo(LAUNCHER_WINDOW, INSTANCE_SETTINGS_ACTION_EVENT, action).catch(
      (error: unknown) => {
        reportInterfaceError("promise", error);
        toast.error(friendlyError(messageOf(error)));
      },
    );
  }

  return (
    <div className={instanceSettingsWindowStyles.root}>
      <InstanceSettingsPage
        entry={entry}
        busy={status.busy}
        versionBusy={status.versionBusy}
        instanceActive={
          entry !== null && status.activeFolders.includes(entry.folder)
        }
        onRename={(next) => {
          if (entry)
            request({ kind: "rename", folder: entry.folder, name: next });
        }}
        onChangeBase={(base) => {
          if (entry)
            request({ kind: "changeBase", folder: entry.folder, base });
        }}
        onOpenFolder={() => {
          if (entry) request({ kind: "openFolder", folder: entry.folder });
        }}
        onDuplicate={() => {
          if (entry) request({ kind: "duplicate", folder: entry.folder });
        }}
        onDelete={() => setConfirmingDelete(true)}
      />
      <div className={instanceSettingsWindowStyles.titleBar}>
        <TitleBar />
      </div>
      <InstanceActionDialog
        action={
          confirmingDelete && entry
            ? { kind: "delete", folder: entry.folder, name: entry.name }
            : null
        }
        draftName=""
        busy={status.busy}
        onOpenChange={(open) => {
          if (!open) setConfirmingDelete(false);
        }}
        onDraftNameChange={() => {}}
        onRename={() => {}}
        onDelete={() => {
          if (!entry) return;
          setConfirmingDelete(false);
          // The launcher closes this window once the instance is gone.
          request({ kind: "delete", folder: entry.folder });
        }}
      />
    </div>
  );
}
