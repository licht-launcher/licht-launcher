import { useMutation } from "@tanstack/react-query";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import {
  DEV_CLEAR_SIMULATED_UPDATE_EVENT,
  DEV_SIMULATE_UPDATE_EVENT,
} from "@/features/updater/components/update-check";
import { openStartupPreview, restartStartupPreview } from "@/lib/commands";
import type { StartupPreviewScenario } from "@/lib/generated/StartupPreviewScenario";

export function DevelopersSection() {
  const [simulateUpdate, setSimulateUpdate] = useState(false);
  const [scenario, setScenario] = useState<StartupPreviewScenario>("noUpdate");
  const scenarios: { value: StartupPreviewScenario; label: string }[] = [
    { value: "noUpdate", label: "No update" },
    { value: "update", label: "Update download and installation" },
    { value: "migration", label: "Migration with temporary fixtures" },
    { value: "updateAndMigration", label: "Update followed by migration" },
    { value: "checkFailure", label: "Update check failure / offline" },
    { value: "migrationFailure", label: "Migration conflict" },
  ];
  const preview = useMutation({
    mutationFn: (restart: boolean) =>
      restart ? restartStartupPreview(scenario) : openStartupPreview(scenario),
  });

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
      <div className="mt-6 flex flex-col gap-3 border-t border-border/60 pt-4">
        <h3 className="text-sm font-medium">Startup window tests</h3>
        <p className="text-sm text-muted-foreground">
          Use the real startup window with simulated updates and isolated
          migration fixtures. No release installer or user migration is executed
          by a test scenario.
        </p>
        <Select
          value={scenario}
          onValueChange={(value) => {
            const selected = scenarios.find((item) => item.value === value);
            if (selected) setScenario(selected.value);
          }}
        >
          <SelectTrigger aria-label="Startup test scenario">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {scenarios.map((item) => (
              <SelectItem key={item.value} value={item.value}>
                {item.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <div className="flex flex-wrap gap-2">
          <Button
            variant="outline"
            disabled={preview.isPending}
            onClick={() => preview.mutate(false)}
          >
            Open startup preview
          </Button>
          <Button
            variant="outline"
            disabled={preview.isPending}
            onClick={() => preview.mutate(true)}
          >
            Restart and test startup
          </Button>
        </div>
        {preview.error ? (
          <p role="alert" className="text-sm text-destructive">
            {preview.error instanceof Error
              ? preview.error.message
              : String(preview.error)}
          </p>
        ) : null}
      </div>
    </section>
  );
}
