import {
  BoxIcon,
  CheckCircle2Icon,
  PackageIcon,
  PuzzleIcon,
  TagIcon,
} from "lucide-react";

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
          <span className="flex size-10 shrink-0 items-center justify-center rounded-md bg-instance/25 text-instance">
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
        <ul className="flex flex-col gap-2 text-sm text-muted-foreground">
          <SummaryRow icon={TagIcon} label="Name" value={state.name.trim()} />
          <SummaryRow
            icon={PackageIcon}
            label="Modpack"
            value={`${state.modpack.name} · ${formatModpackSource(state.modpack.source)}`}
          />
          {state.modpackVersion !== null ? (
            <SummaryRow
              icon={BoxIcon}
              label="Pack version"
              value={state.modpackVersion.name}
            />
          ) : null}
        </ul>
      </div>
    );
  }

  const platformLabel =
    PLATFORMS.find((entry) => entry.id === state.platform)?.label ?? "Unknown";

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-3">
        <span className="flex size-10 shrink-0 items-center justify-center rounded-md bg-instance/25 text-instance">
          <CheckCircle2Icon aria-hidden className="size-5" strokeWidth={2} />
        </span>
        <div>
          <h2 className="text-lg font-medium">Confirm</h2>
          <p className="text-sm text-muted-foreground">
            Review the instance, then create it.
          </p>
        </div>
      </div>
      <ul className="flex flex-col gap-2 text-sm text-muted-foreground">
        <SummaryRow icon={TagIcon} label="Name" value={state.name.trim()} />
        <SummaryRow icon={BoxIcon} label="Platform" value={platformLabel} />
        <SummaryRow
          icon={TagIcon}
          label="Game version"
          value={state.gameVersion}
        />
        {state.platform !== "vanilla" && state.platform !== null ? (
          <SummaryRow
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

function SummaryRow({
  icon: Icon,
  label,
  value,
}: {
  icon: typeof TagIcon;
  label: string;
  value: string;
}) {
  return (
    <li className="flex items-center gap-2 rounded-md bg-black/25 px-3 py-2">
      <Icon aria-hidden className="size-4 shrink-0 text-instance" />
      <span className="min-w-0 flex-1">
        <span className="text-muted-foreground">{label}: </span>
        <span className="text-foreground">{value || "—"}</span>
      </span>
    </li>
  );
}
