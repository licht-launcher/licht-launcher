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
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import { requestStorageLoadingPreview } from "@/features/downloads/hooks/use-storage-loading-preview";
import {
  DEV_CLEAR_SIMULATED_UPDATE_EVENT,
  DEV_SIMULATE_UPDATE_EVENT,
} from "@/features/updater/components/update-check";
import { openStartupPreview, restartStartupPreview } from "@/lib/commands";
import type { StartupPreviewScenario } from "@/lib/generated/StartupPreviewScenario";

const ACRYLIC_PREVIEW_STYLE_ID = "developer-acrylic-preview";
const acrylicControls = [
  { key: "opacity", label: "Sidebar opacity", max: 100, unit: "%" },
  { key: "lightness", label: "Sidebar lightness", max: 100, unit: "%" },
  { key: "hue", label: "Sidebar hue", max: 360, unit: "" },
] as const;

type AcrylicPreview = Record<(typeof acrylicControls)[number]["key"], number>;

function readAcrylicPreview(): AcrylicPreview {
  const tokens = getComputedStyle(document.documentElement);
  const read = (key: keyof AcrylicPreview, fallback: number) => {
    const value = Number.parseFloat(
      tokens.getPropertyValue(`--acrylic-${key}`),
    );
    return Number.isFinite(value) ? value : fallback;
  };
  return {
    opacity: read("opacity", 60),
    lightness: read("lightness", 15),
    hue: read("hue", 260),
  };
}

function applyAcrylicPreview(preview: AcrylicPreview) {
  let stylesheet = document.getElementById(ACRYLIC_PREVIEW_STYLE_ID);
  if (!stylesheet) {
    stylesheet = document.createElement("style");
    stylesheet.id = ACRYLIC_PREVIEW_STYLE_ID;
    document.head.append(stylesheet);
  }
  // A separate stylesheet keeps the preview alive when this section unmounts.
  stylesheet.textContent = `:root { ${acrylicControls
    .map(({ key, unit }) => `--acrylic-${key}: ${preview[key]}${unit};`)
    .join(" ")} }`;
}

export function DevelopersSection() {
  const [acrylic, setAcrylic] = useState(readAcrylicPreview);
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
      {import.meta.env.DEV && (
        <div className="mt-6 flex flex-col gap-3 border-t border-border/60 pt-4">
          <h3 className="text-sm font-medium">Launcher Data loading test</h3>
          <p className="text-sm text-muted-foreground">
            Open Downloads with loading indicators for five seconds, then show
            real launcher data.
          </p>
          <Button
            variant="outline"
            className="self-start"
            onClick={requestStorageLoadingPreview}
          >
            Test Launcher Data loading
          </Button>
        </div>
      )}
      <div className="mt-6 flex flex-col gap-4 border-t border-border/60 pt-4">
        <h3 className="text-sm font-medium">Sidebar Acrylic preview</h3>
        <p id="acrylic-preview-hint" className="text-sm text-muted-foreground">
          Adjust the overlay on Windows with transparency enabled. Changes apply
          immediately and last until the launcher closes. This does not change
          the native blur strength.
        </p>
        {acrylicControls.map(({ key, label, max, unit }) => (
          <div key={key} className="flex flex-col gap-3">
            <div className="flex items-center justify-between gap-3 text-sm">
              <span id={`acrylic-${key}-label`}>{label}</span>
              <span className="text-muted-foreground">
                {acrylic[key]}
                {unit || "°"}
              </span>
            </div>
            <Slider
              aria-labelledby={`acrylic-${key}-label`}
              aria-describedby="acrylic-preview-hint"
              aria-valuetext={`${acrylic[key]}${unit || "°"}`}
              min={0}
              max={max}
              step={1}
              value={[acrylic[key]]}
              onValueChange={([value]) => {
                if (value === undefined || !Number.isFinite(value)) return;
                const next = {
                  ...acrylic,
                  [key]: Math.min(max, Math.max(0, value)),
                };
                setAcrylic(next);
                applyAcrylicPreview(next);
              }}
            />
          </div>
        ))}
        <Button
          variant="outline"
          className="self-start"
          onClick={() => {
            document.getElementById(ACRYLIC_PREVIEW_STYLE_ID)?.remove();
            setAcrylic(readAcrylicPreview());
          }}
        >
          Reset Acrylic preview
        </Button>
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
