import {
  BoxIcon,
  CheckCircle2Icon,
  PackageIcon,
  PuzzleIcon,
  TagIcon,
} from "lucide-react";

import { Card } from "@/components/ui/card";
import { formatModpackSource } from "@/features/new-instance/assets/platform-icons";
import {
  PLATFORMS,
  type WizardState,
} from "@/features/new-instance/model/wizard";

export function StepConfirm({ state }: { state: WizardState }) {
  if (state.source === "modpack" && state.modpack !== null) {
    return (
      <div className="flex flex-col gap-4">
        <div className="flex items-center gap-3">
          <span className="flex size-10 shrink-0 items-center justify-center rounded-md bg-instance/20 text-instance">
            <CheckCircle2Icon aria-hidden className="size-5" strokeWidth={2} />
          </span>
          <div>
            <h2 className="text-lg font-medium">Confirm</h2>
            <p className="text-sm text-muted-foreground">
              Create{" "}
              <span className="text-foreground">{state.name.trim()}</span> from{" "}
              {state.modpack.name} ({formatModpackSource(state.modpack.source)}
              {state.modpackVersion !== null
                ? `, ${state.modpackVersion.name}`
                : ""}
              ).
            </p>
          </div>
        </div>
        <ul className="grid gap-3 sm:grid-cols-2">
          <SummaryCard icon={TagIcon} label="Name" value={state.name.trim()} />
          <SummaryCard
            icon={PackageIcon}
            label="Modpack"
            value={`${state.modpack.name} · ${formatModpackSource(state.modpack.source)}`}
          />
          {state.modpackVersion !== null ? (
            <SummaryCard
              icon={BoxIcon}
              label="Pack version"
              value={state.modpackVersion.name}
            />
          ) : null}
        </ul>
      </div>
    );
  }

  if (state.source === "file") {
    return (
      <div className="flex flex-col gap-4">
        <div className="flex items-center gap-3">
          <span className="flex size-10 shrink-0 items-center justify-center rounded-md bg-instance/20 text-instance">
            <CheckCircle2Icon aria-hidden className="size-5" strokeWidth={2} />
          </span>
          <div>
            <h2 className="text-lg font-medium">Confirm</h2>
            <p className="text-sm text-muted-foreground">
              Create{" "}
              <span className="text-foreground">{state.name.trim()}</span> from
              a local pack file.
            </p>
          </div>
        </div>
        <ul className="grid gap-3 sm:grid-cols-2">
          <SummaryCard icon={TagIcon} label="Name" value={state.name.trim()} />
          <SummaryCard icon={PackageIcon} label="File" value={state.filePath} />
        </ul>
      </div>
    );
  }

  const platformLabel =
    PLATFORMS.find((entry) => entry.id === state.platform)?.label ?? "Unknown";

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-3">
        <span className="flex size-10 shrink-0 items-center justify-center rounded-md bg-instance/20 text-instance">
          <CheckCircle2Icon aria-hidden className="size-5" strokeWidth={2} />
        </span>
        <div>
          <h2 className="text-lg font-medium">Confirm</h2>
          <p className="text-sm text-muted-foreground">
            Review the instance, then create it.
          </p>
        </div>
      </div>
      <ul className="grid gap-3 sm:grid-cols-2">
        <SummaryCard icon={TagIcon} label="Name" value={state.name.trim()} />
        <SummaryCard icon={BoxIcon} label="Platform" value={platformLabel} />
        <SummaryCard
          icon={TagIcon}
          label="Game version"
          value={state.gameVersion}
        />
        {state.platform !== "vanilla" && state.platform !== null ? (
          <SummaryCard
            icon={PuzzleIcon}
            label="Loader"
            value={
              state.loaderChannel === "manual"
                ? state.loaderVersion
                : state.loaderChannel
            }
          />
        ) : null}
      </ul>
    </div>
  );
}

function SummaryCard({
  icon: Icon,
  label,
  value,
}: {
  icon: typeof TagIcon;
  label: string;
  value: string;
}) {
  return (
    <li className="min-w-0">
      <Card aria-label={label} className="flex-row items-center gap-3 p-3">
        <span className="flex size-9 shrink-0 items-center justify-center rounded-md bg-instance/20 text-instance">
          <Icon aria-hidden className="size-4" />
        </span>
        <span className="flex min-w-0 flex-1 flex-col">
          <span className="text-xs text-muted-foreground">{label}</span>
          <span className="truncate text-sm font-medium text-foreground">
            {value || "—"}
          </span>
        </span>
      </Card>
    </li>
  );
}
