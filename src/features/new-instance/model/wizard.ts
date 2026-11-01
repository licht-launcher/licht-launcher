export type InstanceSource = "scratch" | "modpack" | "file";

export type PlatformId = "vanilla" | "fabric" | "neoforge" | "forge" | "quilt";

export type LoaderChannel = "stable" | "latest" | "manual";

export type WizardStep =
  | "source"
  | "platform"
  | "details"
  | "modpack"
  | "modpack-version"
  | "file"
  | "confirm";

export type ModpackSource = "modrinth" | "curseforge";

export type ModpackListItem = {
  id: string;
  source: ModpackSource;
  name: string;
  description: string;
  downloads: number;
  iconUrl: string | null;
  author: string;
};

export type ModpackVersionItem = {
  id: string;
  name: string;
  gameVersions: string[];
  loaders: string[];
};

export type WizardState = {
  step: WizardStep;
  source: InstanceSource | null;
  platform: PlatformId | null;
  name: string;
  gameVersion: string;
  includeSnapshots: boolean;
  loaderChannel: LoaderChannel;
  loaderVersion: string;
  modpackQuery: string;
  modpack: ModpackListItem | null;
  modpackVersion: ModpackVersionItem | null;
  filePath: string;
};

export const initialWizardState: WizardState = {
  step: "source",
  source: "scratch",
  platform: null,
  name: "",
  gameVersion: "",
  includeSnapshots: false,
  loaderChannel: "stable",
  loaderVersion: "",
  modpackQuery: "",
  modpack: null,
  modpackVersion: null,
  filePath: "",
};

export const PLATFORMS: {
  id: PlatformId;
  label: string;
  needsLoader: boolean;
}[] = [
  { id: "vanilla", label: "Vanilla", needsLoader: false },
  { id: "fabric", label: "Fabric", needsLoader: true },
  { id: "neoforge", label: "NeoForge", needsLoader: true },
  { id: "forge", label: "Forge", needsLoader: true },
  { id: "quilt", label: "Quilt", needsLoader: true },
];

export function platformNeedsLoader(platform: PlatformId | null): boolean {
  return PLATFORMS.some((entry) => entry.id === platform && entry.needsLoader);
}

export function fileBaseName(path: string): string {
  const name = path.split(/[\\/]/).pop() ?? "";
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(0, dot) : name;
}

export function stepsFor(state: WizardState): WizardStep[] {
  if (state.source === "modpack") {
    return ["source", "modpack", "modpack-version", "confirm"];
  }
  if (state.source === "scratch") {
    return ["source", "platform", "details", "confirm"];
  }
  if (state.source === "file") {
    return ["source", "file", "confirm"];
  }
  return ["source"];
}

export function stepIndex(state: WizardState): number {
  return stepsFor(state).indexOf(state.step);
}

export function canAdvance(state: WizardState): boolean {
  switch (state.step) {
    case "source":
      return state.source !== null;
    case "platform":
      return state.platform !== null;
    case "details": {
      if (state.name.trim() === "" || state.gameVersion === "") {
        return false;
      }
      if (
        platformNeedsLoader(state.platform) &&
        state.loaderChannel === "manual"
      ) {
        return state.loaderVersion !== "";
      }
      return true;
    }
    case "modpack":
      return state.modpack !== null;
    case "modpack-version":
      return state.modpackVersion !== null && state.name.trim() !== "";
    case "file":
      return state.filePath !== "" && state.name.trim() !== "";
    case "confirm":
      return true;
    default:
      return false;
  }
}

export function nextStep(state: WizardState): WizardStep {
  const steps = stepsFor(state);
  const index = steps.indexOf(state.step);
  return steps[Math.min(index + 1, steps.length - 1)] ?? state.step;
}

export function previousStep(state: WizardState): WizardStep {
  const steps = stepsFor(state);
  const index = steps.indexOf(state.step);
  return steps[Math.max(index - 1, 0)] ?? state.step;
}
