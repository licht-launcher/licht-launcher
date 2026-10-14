import { useState } from "react";

import { Switch } from "@/components/ui/switch";
import {
  DEV_CLEAR_SIMULATED_UPDATE_EVENT,
  DEV_SIMULATE_UPDATE_EVENT,
} from "@/features/updater/components/update-check";

export function DevelopersSection() {
  const [simulateUpdate, setSimulateUpdate] = useState(false);

  return (
    <section aria-label="Developer tools" className="flex max-w-lg flex-col">
      <div className="flex items-center justify-between gap-3">
        <div className="flex min-w-0 flex-col gap-1">
          <label htmlFor="simulate-update" className="text-sm font-medium">
            Simulate available update
          </label>
          <p
            id="simulate-update-hint"
            className="text-sm text-muted-foreground"
          >
            Show update controls without contacting the release server.
          </p>
        </div>
        <Switch
          id="simulate-update"
          className="shrink-0"
          aria-describedby="simulate-update-hint"
          checked={simulateUpdate}
          onCheckedChange={(checked) => {
            setSimulateUpdate(checked);
            window.dispatchEvent(
              new Event(
                checked
                  ? DEV_SIMULATE_UPDATE_EVENT
                  : DEV_CLEAR_SIMULATED_UPDATE_EVENT,
              ),
            );
          }}
        />
      </div>
    </section>
  );
}
