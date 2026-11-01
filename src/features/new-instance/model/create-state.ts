export type InstanceSource = "scratch" | "modpack" | "file" | "clone";

export type PlatformId = "vanilla" | "fabric" | "neoforge" | "forge" | "quilt";

export type LoaderChannel = "stable" | "latest" | "manual";

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

export type CreateState = {
  source: InstanceSource;
  platform: PlatformId;
  name: string;
  gameVersion: string;
  includeSnapshots: boolean;
  loaderChannel: LoaderChannel;
  loaderVersion: string;
  modpackQuery: string;
  modpack: ModpackListItem | null;
  modpackVersion: ModpackVersionItem | null;
  filePath: string;
  cloneFolder: string;
};

export const initialCreateState: CreateState = {
  source: "scratch",
  platform: "vanilla",
  name: "",
  gameVersion: "",
  includeSnapshots: false,
  loaderChannel: "stable",
  loaderVersion: "",
  modpackQuery: "",
  modpack: null,
  modpackVersion: null,
  filePath: "",
  cloneFolder: "",
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

export function canCreate(state: CreateState): boolean {
  if (state.name.trim() === "") {
    return false;
  }
  switch (state.source) {
    case "scratch":
      if (state.gameVersion === "") {
        return false;
      }
      if (
        platformNeedsLoader(state.platform) &&
        state.loaderChannel === "manual"
      ) {
        return state.loaderVersion !== "";
      }
      return true;
    case "modpack":
      return state.modpack !== null && state.modpackVersion !== null;
    case "file":
      return state.filePath !== "";
    case "clone":
      return state.cloneFolder !== "";
  }
}
