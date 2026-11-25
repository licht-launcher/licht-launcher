import type { ModpackSource } from "@/lib/generated/ModpackSource";

/** Display names of the stores packs come from. */
export const SOURCE_LABELS: Record<ModpackSource, string> = {
  modrinth: "Modrinth",
  curseforge: "CurseForge",
};
