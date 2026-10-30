import { useMutation, useQueryClient } from "@tanstack/react-query";
import { UserRound } from "lucide-react";
import { useEffect, useState, useSyncExternalStore } from "react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import {
  setStorageLoadingPreview,
  useStorageLoadingPreview,
} from "@/features/downloads/hooks/use-storage-loading-preview";
import {
  MEMORY_PREVIEW_SIZES_GIB,
  setMemoryPreview,
  useMemoryPreview,
} from "@/features/settings/hooks/use-memory-preview";
import {
  STARTUP_PREVIEW_QUERY_KEY,
  useStartupPreviewState,
} from "@/features/settings/hooks/use-startup-preview-state";
import type { Draft } from "@/features/settings/model/draft";
import {
  getUpdateSimulation,
  setUpdateSimulation,
  subscribeUpdateSimulation,
} from "@/features/updater/model/simulation";
import {
  closeStartupPreview,
  openStartupPreview,
  restartStartupPreview,
} from "@/lib/commands";
import { friendlyError, messageOf } from "@/lib/errors";
import type { StartupPreviewScenario } from "@/lib/generated/StartupPreviewScenario";
import {
  previewWindowSize,
  resetWindowBounds,
  WINDOW_SIZE_PRESETS,
  type WindowSizePreset,
} from "@/lib/window-preview";

const ACRYLIC_PREVIEW_STYLE_ID = "developer-acrylic-preview";
const acrylicControls = [
  { key: "opacity", label: "Opacity", max: 100, unit: "%" },
  { key: "lightness", label: "Lightness", max: 100, unit: "%" },
  { key: "hue", label: "Hue", max: 360, unit: "" },
] as const;

const concurrencyControls = [
  {
    key: "downloadConcurrency",
    label: "Simultaneous downloads",
    hint: "How many files download over the network at once.",
  },
  {
    key: "writeConcurrency",
    label: "Simultaneous writes",
    hint: "How many files are written to disk at once.",
  },
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

export function DevelopersSection({
  draft,
  onChange,
}: {
  draft: Draft;
  onChange: (patch: Partial<Draft>) => void;
}) {
  const [acrylic, setAcrylic] = useState(readAcrylicPreview);
  const nativeAcrylic =
    document.documentElement.dataset.windowEffect === "acrylic";
  const canAdjustAcrylic = nativeAcrylic && !draft.disableTransparency;
  const [adjustAcrylic, setAdjustAcrylic] = useState(
    () => document.getElementById(ACRYLIC_PREVIEW_STYLE_ID) !== null,
  );
  const adjustmentsActive = canAdjustAcrylic && adjustAcrylic;

  useEffect(() => {
    if (!canAdjustAcrylic) {
      document.getElementById(ACRYLIC_PREVIEW_STYLE_ID)?.remove();
      setAdjustAcrylic(false);
      setAcrylic(readAcrylicPreview());
    }
  }, [canAdjustAcrylic]);
  const queryClient = useQueryClient();
  const storageLoading = useStorageLoadingPreview();
  const memoryPreview = useMemoryPreview();
  const simulateUpdate = useSyncExternalStore(
    subscribeUpdateSimulation,
    getUpdateSimulation,
    () => false,
  );
  const [scenario, setScenario] = useState<StartupPreviewScenario>("noUpdate");
  const scenarios: {
    value: StartupPreviewScenario;
    label: string;
  }[] = [
    {
      value: "noUpdate",
      label: "No update",
    },
    {
      value: "update",
      label: "Update download and installation",
    },
    {
      value: "migration",
      label: "Migration with temporary fixtures",
    },
    {
      value: "updateAndMigration",
      label: "Update followed by migration",
    },
    {
      value: "checkFailure",
      label: "Update check failure / offline",
    },
    {
      value: "migrationFailure",
      label: "Migration conflict",
    },
  ];
  const previewState = useStartupPreviewState();
  const previewOpen = previewState.data === true;
  const preview = useMutation({
    mutationFn: (restart: boolean) =>
      restart
        ? restartStartupPreview(scenario)
        : previewOpen
          ? closeStartupPreview()
          : openStartupPreview(scenario),
    onSettled: () =>
      queryClient.invalidateQueries({ queryKey: STARTUP_PREVIEW_QUERY_KEY }),
  });
  const previewBusy =
    preview.isPending ||
    previewState.connecting ||
    previewState.isPending ||
    previewState.isFetching;
  const previewError = preview.error ?? previewState.error;
  const windowPreview = useMutation({
    mutationFn: (preset: WindowSizePreset | "reset") =>
      preset === "reset" ? resetWindowBounds() : previewWindowSize(preset),
  });

  return (
    <section aria-label="Developer Tools" className="flex flex-col gap-8">
      <Card title="Player">
        <div className="flex items-center justify-between gap-3">
          <div className="flex min-w-0 flex-col gap-0.5">
            <label htmlFor="offline-username" className="text-sm font-normal">
              Offline username
            </label>
            <p
              id="offline-username-hint"
              className="text-sm text-muted-foreground"
            >
              Used when no Microsoft account is active.
            </p>
          </div>
          <Input
            id="offline-username"
            className="w-40 shrink-0"
            startIcon={<UserRound aria-hidden />}
            value={draft.username}
            autoComplete="nickname"
            aria-describedby="offline-username-hint"
            onChange={(event) => onChange({ username: event.target.value })}
          />
        </div>
      </Card>
      <Card title="Window">
        <div className="flex items-center justify-between gap-3">
          <div className="flex min-w-0 flex-col gap-0.5">
            <label
              htmlFor="disable-custom-title-bar"
              className="text-sm font-normal"
            >
              System title bar
            </label>
            <div
              id="disable-custom-title-bar-hint"
              className="flex flex-col gap-0.5 text-sm"
            >
              <p className="text-muted-foreground">
                Use the operating system title bar instead of Licht's.
              </p>
            </div>
          </div>
          <Switch
            id="disable-custom-title-bar"
            className="shrink-0"
            aria-describedby="disable-custom-title-bar-hint"
            checked={draft.disableCustomTitleBar}
            onCheckedChange={(disableCustomTitleBar) =>
              onChange({ disableCustomTitleBar })
            }
          />
        </div>
        <div
          className="flex flex-col gap-3"
          aria-busy={windowPreview.isPending}
        >
          <div className="flex flex-col gap-0.5">
            <h4 className="text-sm font-normal">Window size presets</h4>
            <p className="text-sm text-muted-foreground">
              Temporarily resize the launcher to test layouts at its supported
              sizes.
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            {WINDOW_SIZE_PRESETS.map((preset) => (
              <Button
                key={preset.id}
                type="button"
                variant="outline"
                size="default"
                disabled={windowPreview.isPending}
                onClick={() => windowPreview.mutate(preset.id)}
              >
                {preset.label} ({preset.width} × {preset.height})
              </Button>
            ))}
          </div>
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div className="flex min-w-0 flex-1 flex-col gap-0.5">
              <h4 className="text-sm font-normal">Restore window</h4>
              <p
                id="reset-window-bounds-hint"
                className="text-sm text-muted-foreground"
              >
                Restore the default size and center the window.
              </p>
            </div>
            <Button
              type="button"
              variant="outline"
              size="default"
              aria-describedby="reset-window-bounds-hint"
              disabled={windowPreview.isPending}
              onClick={() => windowPreview.mutate("reset")}
            >
              Reset window bounds
            </Button>
          </div>
          {windowPreview.error ? (
            <p role="alert" className="text-sm text-destructive">
              {friendlyError(messageOf(windowPreview.error))}
            </p>
          ) : null}
        </div>
      </Card>
      <Card title="Acrylic — Experimental">
        <div className="flex items-center justify-between gap-3">
          <div className="flex min-w-0 flex-col gap-0.5">
            <label
              htmlFor="disable-transparency"
              className="text-sm font-normal"
            >
              Disable Acrylic
            </label>
            <div
              id="disable-transparency-hint"
              className="flex flex-col gap-0.5 text-sm"
            >
              <p className="text-muted-foreground">
                Windows only. Use an opaque window instead of the native Acrylic
                effect.
              </p>
            </div>
          </div>
          <Switch
            id="disable-transparency"
            className="shrink-0"
            aria-describedby="disable-transparency-hint"
            checked={draft.disableTransparency}
            onCheckedChange={(disableTransparency) =>
              onChange({ disableTransparency })
            }
          />
        </div>
        <div className="flex items-center justify-between gap-3">
          <div className="flex min-w-0 flex-col gap-0.5">
            <label htmlFor="adjust-acrylic" className="text-sm font-normal">
              Adjust Acrylic
            </label>
            <p
              id="acrylic-preview-hint"
              className="text-sm text-muted-foreground"
            >
              {!nativeAcrylic
                ? "Requires a compatible Windows system with native Acrylic enabled."
                : draft.disableTransparency
                  ? "Enable Acrylic to use these temporary adjustments."
                  : "Experimental window-overlay adjustments. Changes apply immediately and reset when the launcher closes."}
            </p>
          </div>
          <Switch
            id="adjust-acrylic"
            className="shrink-0"
            aria-describedby="acrylic-preview-hint"
            checked={adjustmentsActive}
            disabled={!canAdjustAcrylic}
            onCheckedChange={(enabled) => {
              setAdjustAcrylic(enabled);
              if (enabled) {
                applyAcrylicPreview(acrylic);
              } else {
                document.getElementById(ACRYLIC_PREVIEW_STYLE_ID)?.remove();
                setAcrylic(readAcrylicPreview());
              }
            }}
          />
        </div>
        {adjustmentsActive ? (
          <>
            {acrylicControls.map(({ key, label, max, unit }) => (
              <div key={key} className="flex flex-col gap-3">
                <div className="flex items-center justify-between gap-3 text-sm">
                  <span id={`acrylic-${key}-label`} className="font-normal">
                    {label}
                  </span>
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
                    if (
                      !adjustmentsActive ||
                      value === undefined ||
                      !Number.isFinite(value)
                    )
                      return;
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
              size="default"
              className="self-end"
              onClick={() => {
                document.getElementById(ACRYLIC_PREVIEW_STYLE_ID)?.remove();
                setAcrylic(readAcrylicPreview());
              }}
            >
              Reset Acrylic preview
            </Button>
          </>
        ) : null}
      </Card>
      <Card title="Transfers">
        {concurrencyControls.map((control) => (
          <div key={control.key} className="flex flex-col gap-3">
            <div className="flex flex-col gap-0.5">
              <div className="flex items-baseline justify-between gap-3 text-sm">
                <span id={`${control.key}-label`}>{control.label}</span>
                <span className="text-muted-foreground">
                  {draft[control.key]}
                </span>
              </div>
              <p className="text-sm text-muted-foreground">{control.hint}</p>
            </div>
            <Slider
              aria-labelledby={`${control.key}-label`}
              min={1}
              max={16}
              step={1}
              value={[draft[control.key]]}
              onValueChange={([value]) => {
                if (value !== undefined) onChange({ [control.key]: value });
              }}
            />
          </div>
        ))}
      </Card>
      <Card title="Testing">
        {import.meta.env.DEV && (
          <div className="flex flex-col gap-3">
            <div className="flex items-center justify-between gap-3">
              <div className="flex min-w-0 flex-col gap-0.5">
                <label
                  htmlFor="simulate-memory"
                  className="text-sm font-normal"
                >
                  Simulate memory
                </label>
                <p
                  id="simulate-memory-hint"
                  className="text-sm text-muted-foreground"
                >
                  Preview Memory Allocation in Instances with a different RAM
                  size.
                </p>
              </div>
              <Switch
                id="simulate-memory"
                className="shrink-0"
                aria-describedby="simulate-memory-hint"
                checked={memoryPreview.totalMb !== null}
                onCheckedChange={(enabled) =>
                  setMemoryPreview(queryClient, enabled ? 32 * 1024 : null)
                }
              />
            </div>
            {memoryPreview.totalMb !== null ? (
              <div className="flex items-center justify-between gap-3">
                <label htmlFor="simulated-ram" className="text-sm font-normal">
                  Simulated RAM
                </label>
                <Select
                  value={String(memoryPreview.totalMb / 1024)}
                  onValueChange={(value) => {
                    const gib = MEMORY_PREVIEW_SIZES_GIB.find(
                      (size) => String(size) === value,
                    );
                    if (gib !== undefined)
                      setMemoryPreview(queryClient, gib * 1024);
                  }}
                >
                  <SelectTrigger
                    id="simulated-ram"
                    aria-label="Simulated RAM"
                    aria-describedby="simulate-memory-hint"
                  >
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {MEMORY_PREVIEW_SIZES_GIB.map((gib) => (
                      <SelectItem key={gib} value={String(gib)}>
                        {gib} GiB
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            ) : null}
          </div>
        )}
        <div className="flex items-center justify-between gap-3">
          <div className="flex min-w-0 flex-col gap-0.5">
            <label htmlFor="simulate-update" className="text-sm font-normal">
              Simulate available update
            </label>
            <p
              id="simulate-update-hint"
              className="text-sm text-muted-foreground"
            >
              Show an update notification and action without checking for a
              release.
            </p>
          </div>
          <Switch
            id="simulate-update"
            className="shrink-0"
            aria-describedby="simulate-update-hint"
            checked={simulateUpdate}
            onCheckedChange={setUpdateSimulation}
          />
        </div>
        {import.meta.env.DEV && (
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div className="flex min-w-0 flex-1 flex-col gap-0.5">
              <label
                htmlFor="simulate-storage-loading"
                className="text-sm font-normal"
              >
                Simulate storage loading
              </label>
              <p
                id="simulate-storage-loading-hint"
                className="text-sm text-muted-foreground"
              >
                Open Downloads and keep storage loading until this is turned
                off.
              </p>
            </div>
            <Switch
              id="simulate-storage-loading"
              className="shrink-0"
              aria-describedby="simulate-storage-loading-hint"
              checked={storageLoading}
              onCheckedChange={(enabled) =>
                setStorageLoadingPreview(queryClient, enabled)
              }
            />
          </div>
        )}
        <div className="flex flex-col gap-3">
          <div className="flex flex-col gap-0.5">
            <h4 className="text-sm font-normal">Startup preview</h4>
            <p className="text-sm text-muted-foreground">
              Open a separate startup preview, or restart the launcher with the
              selected scenario. Updates and migrations use simulated data.
            </p>
          </div>
          <div className="flex flex-col gap-0.5">
            <Select
              disabled={previewBusy || previewOpen}
              value={scenario}
              onValueChange={(value) => {
                const selected = scenarios.find((item) => item.value === value);
                if (selected) setScenario(selected.value);
              }}
            >
              <SelectTrigger
                id="startup-test-scenario"
                aria-label="Startup test scenario"
                className="w-full"
              >
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
          </div>
          <div className="flex flex-wrap gap-2">
            <Button
              size="default"
              variant={previewOpen ? "outline" : "default"}
              className={
                previewOpen
                  ? "border-destructive/40 text-destructive hover:bg-destructive/10 hover:text-destructive"
                  : undefined
              }
              disabled={previewBusy}
              onClick={() => preview.mutate(false)}
            >
              {previewOpen ? "Close preview" : "Open preview"}
            </Button>
            <Button
              variant="outline"
              size="default"
              disabled={previewBusy || previewOpen}
              onClick={() => preview.mutate(true)}
            >
              Restart with preview
            </Button>
          </div>
          {previewError ? (
            <p role="alert" className="text-sm text-destructive">
              {messageOf(previewError)}
            </p>
          ) : null}
          {previewState.eventError ? (
            <p className="text-xs text-muted-foreground">
              Live preview status is unavailable. Polling window state.
            </p>
          ) : null}
        </div>
      </Card>
    </section>
  );
}
