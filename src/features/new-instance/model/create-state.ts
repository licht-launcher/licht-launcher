export type PlatformId = "vanilla" | "fabric" | "neoforge" | "forge" | "quilt";

export type LoaderChannel = "stable" | "latest" | "manual";

/** The popup's two steps: choosing where to start, then the from-scratch form. */
export type CreateView = "source" | "details";

/** What the user fills in when creating an instance from scratch. */
export type ScratchState = {
  name: string;
  platform: PlatformId;
  gameVersion: string;
  includeSnapshots: boolean;
  loaderChannel: LoaderChannel;
  loaderVersion: string;
};

export const initialScratchState: ScratchState = {
  name: "",
  platform: "vanilla",
  gameVersion: "",
  includeSnapshots: false,
  loaderChannel: "stable",
  loaderVersion: "",
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

function normalizeName(name: string): string {
  return name.trim().toLowerCase();
}

/** Instance names are unique, ignoring case and surrounding spaces. */
export function isNameTaken(name: string, taken: readonly string[]): boolean {
  const wanted = normalizeName(name);
  return (
    wanted !== "" && taken.some((other) => normalizeName(other) === wanted)
  );
}

/** `name` itself if free, else the first of `name (1)`, `name (2)`, … that is. */
export function uniqueName(name: string, taken: readonly string[]): string {
  const base = name.trim();
  if (!isNameTaken(base, taken)) {
    return base;
  }
  for (let number = 1; ; number += 1) {
    const candidate = `${base} (${number})`;
    if (!isNameTaken(candidate, taken)) {
      return candidate;
    }
  }
}

export function canCreateScratch(
  state: ScratchState,
  taken: readonly string[] = [],
): boolean {
  if (state.gameVersion === "" || isNameTaken(state.name, taken)) {
    return false;
  }
  if (platformNeedsLoader(state.platform) && state.loaderChannel === "manual") {
    return state.loaderVersion !== "";
  }
  return true;
}

/** The suggested name for a new instance, such as "Fabric 26.3". */
export function defaultInstanceName(
  state: ScratchState,
  taken: readonly string[] = [],
): string {
  if (state.gameVersion === "") {
    return "";
  }
  const platform = PLATFORMS.find((entry) => entry.id === state.platform);
  return uniqueName(
    `${platform?.label ?? state.platform} ${state.gameVersion}`,
    taken,
  );
}

/** What the instance is called: the typed name, or the suggestion when blank. */
export function resolveInstanceName(
  state: ScratchState,
  taken: readonly string[] = [],
): string {
  return state.name.trim() || defaultInstanceName(state, taken);
}

/** Default name for an instance created from a picked archive. */
export function uploadInstanceName(path: string): string {
  const name = path.split(/[\\/]/).pop() ?? "";
  const dot = name.lastIndexOf(".");
  const base = dot > 0 ? name.slice(0, dot) : name;
  return base === "" || dot === 0 ? "Imported pack" : base;
}
