import { emit, listen } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";
import {
  INSTANCE_SETTINGS_ACTION_EVENT,
  INSTANCE_SETTINGS_ACTION_FAILED_EVENT,
  INSTANCE_SETTINGS_STATUS_EVENT,
  INSTANCE_SETTINGS_STATUS_REQUEST_EVENT,
  type InstanceSettingsAction,
  type InstanceSettingsActionFailure,
  type InstanceSettingsStatus,
  parseInstanceSettingsAction,
} from "@/features/instance-settings/model/window-events";
import { reportInterfaceError } from "@/lib/error-reporting";
import { friendlyError, messageOf } from "@/lib/errors";

/**
 * Runs the actions instance settings windows ask for and tells them what blocks them.
 * `status` is sent only when its value changes, so it need not be memoized.
 */
export function useInstanceSettingsBridge(
  status: InstanceSettingsStatus,
  run: (action: InstanceSettingsAction) => Promise<unknown>,
) {
  const statusRef = useRef(status);
  statusRef.current = status;
  const runRef = useRef(run);
  runRef.current = run;

  // The launcher renders on every game log line; resend only what changed.
  const statusKey = JSON.stringify(status);
  const sentKeyRef = useRef<string | null>(null);
  useEffect(() => {
    if (sentKeyRef.current === statusKey) return;
    sentKeyRef.current = statusKey;
    emit(INSTANCE_SETTINGS_STATUS_EVENT, statusRef.current).catch(
      (error: unknown) => reportInterfaceError("promise", error),
    );
  }, [statusKey]);

  useEffect(() => {
    let active = true;
    const stops: (() => void)[] = [];
    const keep = (subscribing: Promise<() => void>) => {
      subscribing
        .then((stop) => {
          if (active) stops.push(stop);
          else stop();
        })
        .catch((error: unknown) => reportInterfaceError("promise", error));
    };
    keep(
      listen<unknown>(INSTANCE_SETTINGS_ACTION_EVENT, ({ payload }) => {
        const action = parseInstanceSettingsAction(payload);
        if (!action) {
          reportInterfaceError(
            "promise",
            new Error("Ignored an invalid instance settings action"),
          );
          return;
        }
        runRef.current(action).catch((error: unknown) => {
          const failure: InstanceSettingsActionFailure = {
            folder: action.folder,
            message: friendlyError(messageOf(error)),
          };
          emit(INSTANCE_SETTINGS_ACTION_FAILED_EVENT, failure).catch(
            (reportError: unknown) =>
              reportInterfaceError("promise", reportError),
          );
        });
      }),
    );
    keep(
      listen(INSTANCE_SETTINGS_STATUS_REQUEST_EVENT, () => {
        emit(INSTANCE_SETTINGS_STATUS_EVENT, statusRef.current).catch(
          (error: unknown) => reportInterfaceError("promise", error),
        );
      }),
    );
    return () => {
      active = false;
      for (const stop of stops) stop();
    };
  }, []);
}
