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

export function canCreateScratch(state: ScratchState): boolean {
  if (state.name.trim() === "" || state.gameVersion === "") {
    return false;
  }
  if (platformNeedsLoader(state.platform) && state.loaderChannel === "manual") {
    return state.loaderVersion !== "";
  }
  return true;
}

/** Default name for an instance created from a picked archive. */
export function uploadInstanceName(path: string): string {
  const name = path.split(/[\\/]/).pop() ?? "";
  const dot = name.lastIndexOf(".");
  const base = dot > 0 ? name.slice(0, dot) : name;
  return base === "" || dot === 0 ? "Imported pack" : base;
}
