import {
  type InstanceSource,
  initialWizardState,
  type LoaderChannel,
  type ModpackListItem,
  type ModpackSource,
  type ModpackVersionItem,
  type PlatformId,
  stepsFor,
  type WizardState,
  type WizardStep,
} from "@/features/new-instance/model/wizard";

export type NewInstanceSearch = {
  step: WizardStep;
  source?: InstanceSource;
  platform?: PlatformId;
  name?: string;
  gameVersion?: string;
  includeSnapshots?: boolean;
  loaderChannel?: LoaderChannel;
  loaderVersion?: string;
  modpackQuery?: string;
  modpackId?: string;
  modpackSource?: ModpackSource;
  modpackVersionId?: string;
};

const WIZARD_STEPS: readonly WizardStep[] = [
  "source",
  "platform",
  "details",
  "modpack",
  "modpack-version",
  "confirm",
];

const PLATFORMS: readonly PlatformId[] = [
  "vanilla",
  "fabric",
  "neoforge",
  "forge",
  "quilt",
];

export function isWizardStep(value: unknown): value is WizardStep {
  return (
    typeof value === "string" &&
    (WIZARD_STEPS as readonly string[]).includes(value)
  );
}

function isInstanceSource(value: unknown): value is InstanceSource {
  return value === "scratch" || value === "modpack";
}

function isPlatformId(value: unknown): value is PlatformId {
  return (
    typeof value === "string" &&
    (PLATFORMS as readonly string[]).includes(value)
  );
}

function isLoaderChannel(value: unknown): value is LoaderChannel {
  return value === "stable" || value === "latest" || value === "manual";
}

function isModpackSource(value: unknown): value is ModpackSource {
  return value === "modrinth" || value === "curseforge";
}

function optionalString(value: unknown): string | undefined {
  return typeof value === "string" && value !== "" ? value : undefined;
}

/** Parse router search into a typed create-instance search object. */
export function parseNewInstanceSearch(
  search: Record<string, unknown>,
): NewInstanceSearch {
  const step = isWizardStep(search.step) ? search.step : "source";
  const parsed: NewInstanceSearch = { step };

  if (isInstanceSource(search.source)) {
    parsed.source = search.source;
  }
  if (isPlatformId(search.platform)) {
    parsed.platform = search.platform;
  }
  const name = optionalString(search.name);
  if (name !== undefined) {
    parsed.name = name;
  }
  const gameVersion = optionalString(search.gameVersion);
  if (gameVersion !== undefined) {
    parsed.gameVersion = gameVersion;
  }
  if (search.includeSnapshots === true || search.includeSnapshots === "true") {
    parsed.includeSnapshots = true;
  }
  if (isLoaderChannel(search.loaderChannel)) {
    parsed.loaderChannel = search.loaderChannel;
  }
  const loaderVersion = optionalString(search.loaderVersion);
  if (loaderVersion !== undefined) {
    parsed.loaderVersion = loaderVersion;
  }
  const modpackQuery = optionalString(search.modpackQuery);
  if (modpackQuery !== undefined) {
    parsed.modpackQuery = modpackQuery;
  }
  const modpackId = optionalString(search.modpackId);
  if (modpackId !== undefined) {
    parsed.modpackId = modpackId;
  }
  if (isModpackSource(search.modpackSource)) {
    parsed.modpackSource = search.modpackSource;
  }
  const modpackVersionId = optionalString(search.modpackVersionId);
  if (modpackVersionId !== undefined) {
    parsed.modpackVersionId = modpackVersionId;
  }

  return clampSearchStep(parsed);
}

function clampSearchStep(search: NewInstanceSearch): NewInstanceSearch {
  const draft = wizardStateFromSearch(search, null, null);
  const path = stepsFor(draft);
  if (path.includes(search.step)) {
    return search;
  }
  return { ...search, step: path[0] ?? "source" };
}

/** Build wizard state from URL search plus resolved modpack entities. */
export function wizardStateFromSearch(
  search: NewInstanceSearch,
  modpack: ModpackListItem | null,
  modpackVersion: ModpackVersionItem | null,
): WizardState {
  return {
    step: search.step,
    source: search.source ?? null,
    platform: search.platform ?? null,
    name: search.name ?? "",
    gameVersion: search.gameVersion ?? "",
    includeSnapshots: search.includeSnapshots === true,
    loaderChannel: search.loaderChannel ?? "stable",
    loaderVersion: search.loaderVersion ?? "",
    modpackQuery: search.modpackQuery ?? "",
    modpack,
    modpackVersion,
  };
}

/** Serialize wizard fields back to URL search (omit empty defaults). */
export function searchFromWizardState(state: WizardState): NewInstanceSearch {
  const search: NewInstanceSearch = { step: state.step };

  if (state.source !== null) {
    search.source = state.source;
  }
  if (state.platform !== null) {
    search.platform = state.platform;
  }
  if (state.name.trim() !== "") {
    search.name = state.name;
  }
  if (state.gameVersion !== "") {
    search.gameVersion = state.gameVersion;
  }
  if (state.includeSnapshots) {
    search.includeSnapshots = true;
  }
  if (state.loaderChannel !== "stable") {
    search.loaderChannel = state.loaderChannel;
  }
  if (state.loaderVersion !== "") {
    search.loaderVersion = state.loaderVersion;
  }
  if (state.modpackQuery !== "") {
    search.modpackQuery = state.modpackQuery;
  }
  if (state.modpack !== null) {
    search.modpackId = state.modpack.id;
    search.modpackSource = state.modpack.source;
  }
  if (state.modpackVersion !== null) {
    search.modpackVersionId = state.modpackVersion.id;
  }

  return search;
}

/** Patch the current search (used for replace navigations). */
export function patchSearch(
  current: NewInstanceSearch,
  patch: Partial<NewInstanceSearch>,
): NewInstanceSearch {
  return clampSearchStep({ ...current, ...patch });
}

export function defaultNewInstanceSearch(): NewInstanceSearch {
  return { step: initialWizardState.step };
}
